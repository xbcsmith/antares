# Combat Events Implementation Plan

## Overview

Antares has a functioning turn-based combat engine
(`src/domain/combat/engine.rs`, `src/game/systems/combat.rs`) and a map event
system (`src/domain/world/events.rs`, `src/domain/world/types.rs`). However, all
combat is triggered identically: the party sees the monster, `Handicap::Even` is
always used, and the `MapEvent::Encounter` variant carries only a `monster_group`
list with no metadata about how the battle begins or behaves. There is no concept
of an ambush, a ranged skirmish, a magic duel, or a boss fight at the data or
engine level.

This plan extends the combat system in five sequential phases:

1. Add a `CombatEventType` enum and wire it through the domain layer.
2. Implement `Normal` and `Ambush` combat event types in the engine and game systems.
3. Implement `Ranged` and `Magic` combat event types.
4. Implement `Boss` combat event type with special mechanics.
5. Add Campaign Builder UI support for setting and editing the combat event type on
   map encounters.

---

## Current State Analysis

### Existing Infrastructure

- `CombatState` in `src/domain/combat/engine.rs` — `participants`, `turn_order`,
  `handicap`, `can_flee`, `can_surrender`, `can_bribe`, `monsters_advance`,
  `monsters_regenerate`. All fields that `CombatEventType` will configure.
- `MapEvent::Encounter` in `src/domain/world/types.rs` — carries `name`,
  `description`, `monster_group: Vec<MonsterId>`. No event type field.
- `EventResult::Encounter` in `src/domain/world/events.rs` — carries only
  `monster_group: Vec<u8>`. No event type field.
- `start_encounter()` in `src/game/systems/combat.rs` — builds `CombatState` with
  `Handicap::Even`, calls `initialize_combat_from_group()`, then
  `game_state.enter_combat_with_state()`. No special-case logic per encounter type.
- `calculate_turn_order()` in `src/domain/combat/engine.rs` — already implements
  `Handicap::PartyAdvantage`, `Handicap::MonsterAdvantage`, and `Handicap::Even`.
  The ambush mechanic will use `MonsterAdvantage`.
- `EncounterTable` in `src/domain/world/types.rs` — `encounter_rate`, `groups`,
  `terrain_modifiers`. Random encounters do not currently carry a `CombatEventType`.
- `CombatResource` in `src/game/systems/combat.rs` — Bevy resource that mirrors
  `CombatState`; carries `encounter_position` and `encounter_map_id`.
- `CombatStarted` message in `src/game/systems/combat.rs` — carries
  `encounter_position` and `encounter_map_id`. Will need a `combat_event_type` field.
- `MapsEditorState` and `EventEditorState` in
  `sdk/campaign_builder/src/map_editor.rs` — already handle the `Encounter` event
  type with monster selection; need a combo-box for `CombatEventType`.
- `EncounterTable` random encounter path in `src/domain/world/events.rs` —
  `random_encounter()` returns `Option<Vec<u8>>`. Will need to return a type that
  includes `CombatEventType`.
- Rest system (planned) — rest encounters should be ambushes with
  `CombatEventType::Ambush` so the rest-interrupted encounter has the correct
  mechanics.

### Identified Issues

- `MapEvent::Encounter` has no `combat_event_type` field; every fight is a generic
  encounter regardless of design intent.
- `EventResult::Encounter` propagates no combat type to the game layer.
- `start_encounter()` always creates `Handicap::Even` with no special first-round
  logic; ambush, ranged, magic, and boss variations are impossible.
- `CombatStarted` carries no combat type; even if the domain layer produced one, the
  Bevy combat plugin could not act on it.
- The `EncounterTable` random encounter codepath (`random_encounter()`) returns only
  `Option<Vec<u8>>`; random ambushes while resting (or on specific terrain) cannot
  specify a type.
- The map editor `EventEditorState` has no `combat_event_type` selector field.
- Boss monsters have no special-ability hook in the engine; `can_advance` and
  `monsters_regenerate` exist but are never set from encounter data.
- Ranged and magic combat do not change the turn actions available to the party; the
  UI action menu needs a "Ranged Attack" button that is only enabled when the
  combatant has a ranged weapon, and a "Cast Spell" button that is always present but
  more prominent in Magic combat.

---

## Implementation Phases

### Phase 1: `CombatEventType` Domain Type and Data Layer

Adds the `CombatEventType` enum to the domain layer and threads it through
`MapEvent`, `EventResult`, `EncounterTable`, and `start_encounter()` without
changing any combat mechanics. After this phase, campaign RON files can declare an
encounter type; the engine reads it and stores it, but combat itself is still
identical in all cases. This ensures RON data written in Phase 1 is forward-
compatible with later phases.

#### 1.1 Add `CombatEventType` to `src/domain/combat/types.rs`

Add the enum alongside the existing `AttackType`, `Handicap`, and `CombatStatus`
types:

```src/domain/combat/types.rs#L1-50
/// The type of combat event that determines how a battle begins and what
/// special mechanics apply throughout.
///
/// Campaign authors set this in `map.ron` per-encounter or on the
/// `EncounterTable` for random encounters.  The game engine uses it to
/// configure `CombatState` before `start_combat()` is called.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::types::CombatEventType;
///
/// let t = CombatEventType::Ambush;
/// assert!(t.gives_monster_advantage());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CombatEventType {
    /// Party sees the monster before combat begins. Normal initiative order.
    /// No special mechanics.
    #[default]
    Normal,

    /// Party does not see the monster. Monsters act first in round 1 only
    /// (MonsterAdvantage handicap for round 1, then Even from round 2).
    /// The party's actions are suppressed during their first turn of round 1.
    Ambush,

    /// Party and monster can exchange ranged attacks before closing to melee.
    /// Combatants with a ranged weapon or ranged-capable attack gain an
    /// additional "Ranged Attack" action option. Normal initiative order.
    Ranged,

    /// Monster uses magic as its primary attack vector. The "Cast Spell"
    /// action button is highlighted and placed first in the action menu.
    /// Normal initiative order.
    Magic,

    /// Monster is a boss with special abilities and enhanced stats at runtime.
    /// Bosses: advance every round, may regenerate, cannot be bribed,
    /// cannot be surrendered to. Normal initiative order.
    Boss,
}

impl CombatEventType {
    /// Returns true if this event type gives monsters the first-round advantage.
    pub fn gives_monster_advantage(&self) -> bool {
        matches!(self, CombatEventType::Ambush)
    }

    /// Returns true if this event type enables the dedicated Ranged Attack action.
    pub fn enables_ranged_action(&self) -> bool {
        matches!(self, CombatEventType::Ranged)
    }

    /// Returns true if this event type highlights the Cast Spell action.
    pub fn highlights_magic_action(&self) -> bool {
        matches!(self, CombatEventType::Magic)
    }

    /// Returns true if this event type applies boss mechanics to all monsters.
    pub fn applies_boss_mechanics(&self) -> bool {
        matches!(self, CombatEventType::Boss)
    }

    /// Human-readable display name used in the Campaign Builder UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            CombatEventType::Normal => "Normal",
            CombatEventType::Ambush => "Ambush",
            CombatEventType::Ranged => "Ranged",
            CombatEventType::Magic => "Magic",
            CombatEventType::Boss => "Boss",
        }
    }

    /// Short description used in Campaign Builder tooltips.
    pub fn description(&self) -> &'static str {
        match self {
            CombatEventType::Normal => "Party sees the monster. Standard initiative order.",
            CombatEventType::Ambush => {
                "Party is surprised. Monsters act first; party misses round 1."
            }
            CombatEventType::Ranged => {
                "Ranged weapons and ranged monster attacks are available."
            }
            CombatEventType::Magic => {
                "Monsters use magic. Cast Spell is the primary action."
            }
            CombatEventType::Boss => {
                "Boss fight. Monsters advance, may regenerate, cannot be bribed or surrendered to.",
            }
        }
    }

    /// All variants in display order for UI combo-boxes.
    pub fn all() -> &'static [CombatEventType] {
        &[
            CombatEventType::Normal,
            CombatEventType::Ambush,
            CombatEventType::Ranged,
            CombatEventType::Magic,
            CombatEventType::Boss,
        ]
    }
}
```

Export `CombatEventType` from `src/domain/combat/mod.rs` alongside the existing
`pub use types::*;` — no change to `mod.rs` required since `types` is already
re-exported with a glob.

#### 1.2 Add `combat_event_type` to `MapEvent::Encounter`

In `src/domain/world/types.rs`, extend `MapEvent::Encounter`:

```src/domain/world/types.rs#L1-20
MapEvent::Encounter {
    /// Event name
    #[serde(default)]
    name: String,
    /// Event description
    #[serde(default)]
    description: String,
    /// Monster group IDs
    monster_group: Vec<MonsterId>,
    /// How this combat begins and what special mechanics apply.
    /// Defaults to `CombatEventType::Normal` for backward compatibility.
    #[serde(default)]
    combat_event_type: CombatEventType,
},
```

The `#[serde(default)]` attribute means all existing RON files that omit
`combat_event_type` will deserialize correctly with `CombatEventType::Normal`.

#### 1.3 Add `combat_event_type` to `EventResult::Encounter`

In `src/domain/world/events.rs`, extend `EventResult::Encounter`:

```src/domain/world/events.rs#L1-10
EventResult::Encounter {
    /// IDs of monsters in the encounter
    monster_group: Vec<MonsterId>,
    /// Type of combat event
    combat_event_type: CombatEventType,
},
```

Update `trigger_event()` to forward the new field:

```src/domain/world/events.rs#L1-10
MapEvent::Encounter {
    monster_group,
    combat_event_type,
    ..
} => EventResult::Encounter {
    monster_group,
    combat_event_type,
},
```

#### 1.4 Add `combat_event_type` to `EncounterTable`

In `src/domain/world/types.rs`, extend `EncounterTable` to allow each group entry to
carry a type. Introduce a typed group struct to replace the raw `Vec<Vec<MonsterId>>`:

```src/domain/world/types.rs#L1-20
/// A single random encounter group entry in the encounter table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncounterGroup {
    /// Monster IDs in this group
    pub monster_group: Vec<MonsterId>,
    /// Combat event type for this group.
    /// Defaults to `CombatEventType::Normal`.
    #[serde(default)]
    pub combat_event_type: CombatEventType,
}

pub struct EncounterTable {
    #[serde(default = "default_encounter_rate")]
    pub encounter_rate: f32,

    /// Monster groups available in this area. Each entry is an `EncounterGroup`.
    /// Replaces the old `Vec<Vec<u8>>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<EncounterGroup>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub terrain_modifiers: HashMap<TerrainType, f32>,
}
```

Update `random_encounter()` in `src/domain/world/events.rs` to return
`Option<EncounterGroup>` instead of `Option<Vec<u8>>`. All callers in the game layer
must be updated to extract `group.monster_group` and `group.combat_event_type`
separately.

#### 1.5 Add `combat_event_type` to `CombatStarted` Bevy Message

In `src/game/systems/combat.rs`, extend `CombatStarted`:

```src/game/systems/combat.rs#L1-10
pub struct CombatStarted {
    pub encounter_position: Option<Position>,
    pub encounter_map_id: Option<MapId>,
    /// Type of this combat encounter. Forwarded from EventResult or EncounterGroup.
    pub combat_event_type: CombatEventType,
}
```

Update `start_encounter()` to accept and forward `CombatEventType`:

```src/game/systems/combat.rs#L1-20
pub fn start_encounter(
    game_state: &mut crate::application::GameState,
    content: &GameContent,
    group: &[MonsterId],
    combat_event_type: CombatEventType,
) -> Result<(), crate::domain::combat::database::MonsterDatabaseError> {
    let handicap = Handicap::Even; // Phase 2 will vary this by type
    let mut cs = CombatState::new(handicap);

    for character in &game_state.party.members {
        cs.add_player(character.clone());
    }

    initialize_combat_from_group(&mut cs, content.db(), group)?;
    game_state.enter_combat_with_state(cs);
    Ok(())
}
```

Store `combat_event_type` on `CombatResource` so later systems can read it:

```src/game/systems/combat.rs#L1-10
pub struct CombatResource {
    pub state: CombatState,
    pub player_orig_indices: Vec<Option<usize>>,
    pub resolution_handled: bool,
    pub encounter_position: Option<Position>,
    pub encounter_map_id: Option<MapId>,
    /// Type of the current combat encounter.
    pub combat_event_type: CombatEventType,
}
```

#### 1.6 Update All `EventResult::Encounter` Match Arms in the Game Layer

Any `match` on `EventResult` in `src/game/systems/map.rs` (or wherever the encounter
result is consumed to call `start_encounter()`) must be updated to extract and forward
`combat_event_type`.

#### 1.7 Testing Requirements

- `test_combat_event_type_default_is_normal` — `CombatEventType::default() == Normal`.
- `test_combat_event_type_flags` — `Ambush.gives_monster_advantage()` is `true`;
  `Normal.gives_monster_advantage()` is `false`.
- `test_map_event_encounter_ron_round_trip` — serialize and deserialize an
  `Encounter` with `combat_event_type: Ambush`; confirm the type survives.
- `test_map_event_encounter_ron_backward_compat` — deserialize an `Encounter` RON
  without the `combat_event_type` field; confirm `Normal` is used.
- `test_event_result_encounter_carries_type` — `trigger_event()` on an `Ambush`
  encounter returns `EventResult::Encounter { combat_event_type: Ambush, .. }`.
- `test_encounter_group_ron_round_trip` — `EncounterGroup` with `Ranged` type
  serializes and deserializes correctly.
- `test_random_encounter_returns_group_type` — `random_encounter()` returns the type
  from the selected group.
- `test_start_encounter_stores_type_in_resource` — after `start_encounter()`,
  `CombatResource.combat_event_type` matches what was passed in.

#### 1.8 Deliverables

- [ ] `CombatEventType` enum in `src/domain/combat/types.rs`
- [ ] `combat_event_type: CombatEventType` field on `MapEvent::Encounter`
- [ ] `combat_event_type` on `EventResult::Encounter`
- [ ] `EncounterGroup` struct replacing raw `Vec<MonsterId>` in `EncounterTable`
- [ ] `random_encounter()` returns `Option<EncounterGroup>`
- [ ] `CombatStarted.combat_event_type` field
- [ ] `CombatResource.combat_event_type` field
- [ ] `start_encounter()` accepts and forwards `CombatEventType`
- [ ] All callers of `start_encounter()` and `random_encounter()` updated
- [ ] All phase-1 tests pass

#### 1.9 Success Criteria

A campaign RON file can contain `combat_event_type: Ambush` on an `Encounter` event.
The game engine reads and stores the type end-to-end from RON → domain → Bevy
resource. All four `cargo` quality gates pass with zero warnings.

---

### Phase 2: Normal and Ambush Combat

Implements the behavioral differences for `Normal` and `Ambush` combat. After this
phase, an ambush encounter causes the party to miss their first round of turns, and
the combat log reports that the party was surprised.

#### 2.1 Add `is_ambush_round` Flag to `CombatState`

In `src/domain/combat/engine.rs`, add a runtime flag to `CombatState`:

```src/domain/combat/engine.rs#L1-15
pub struct CombatState {
    pub participants: Vec<Combatant>,
    pub turn_order: Vec<CombatantId>,
    pub current_turn: usize,
    pub round: u32,
    pub status: CombatStatus,
    pub handicap: Handicap,
    pub can_flee: bool,
    pub can_surrender: bool,
    pub can_bribe: bool,
    pub monsters_advance: bool,
    pub monsters_regenerate: bool,
    /// True during round 1 of an ambush encounter. Player turns are skipped.
    /// Cleared automatically at the start of round 2.
    pub ambush_round_active: bool,
}
```

#### 2.2 Apply `CombatEventType` in `start_encounter()`

In `src/game/systems/combat.rs`, update `start_encounter()` to configure
`CombatState` based on `CombatEventType`:

```src/game/systems/combat.rs#L1-35
pub fn start_encounter(
    game_state: &mut crate::application::GameState,
    content: &GameContent,
    group: &[MonsterId],
    combat_event_type: CombatEventType,
) -> Result<(), crate::domain::combat::database::MonsterDatabaseError> {
    // Select handicap based on event type
    let handicap = if combat_event_type.gives_monster_advantage() {
        Handicap::MonsterAdvantage
    } else {
        Handicap::Even
    };

    let mut cs = CombatState::new(handicap);
    cs.ambush_round_active = combat_event_type == CombatEventType::Ambush;

    // Boss mechanics: monsters advance and regenerate; bribe/surrender disabled
    if combat_event_type.applies_boss_mechanics() {
        cs.monsters_advance = true;
        cs.monsters_regenerate = true;
        cs.can_bribe = false;
        cs.can_surrender = false;
    }

    for character in &game_state.party.members {
        cs.add_player(character.clone());
    }

    initialize_combat_from_group(&mut cs, content.db(), group)?;
    game_state.enter_combat_with_state(cs);
    Ok(())
}
```

#### 2.3 Skip Player Turns During Ambush Round 1

In `src/game/systems/combat.rs`, in `combat_input_system` (the system that handles
player turn input), add a guard at the top:

```src/game/systems/combat.rs#L1-15
// During an ambush round the party is surprised and cannot act.
if combat_res.state.ambush_round_active {
    if matches!(
        combat_res.state.get_current_combatant(),
        Some(Combatant::Player(_))
    ) {
        // Auto-advance: skip this player's turn with a "Surprised!" log entry
        combat_log.push_line(CombatLogLine::status(
            "The party is surprised and cannot act!",
        ));
        // Mark all player actions as skipped and advance turn
        // (handled by dispatch_combat_action with a new SkipTurn variant or
        //  by directly calling advance_turn)
        combat_res.state.advance_turn(&condition_defs);
        return;
    }
}
```

Add a `TurnAction::Skip` variant to `src/domain/combat/types.rs` for internal use
(not shown in the player UI action menu; only used during ambush and incapacitated
turns).

#### 2.4 Clear Ambush Flag at Round 2

In `CombatState::advance_round()` in `src/domain/combat/engine.rs`, clear the flag
when the round counter reaches 2:

```src/domain/combat/engine.rs#L1-10
fn advance_round(&mut self, condition_defs: &[ConditionDefinition]) -> Vec<(CombatantId, i16)> {
    self.round += 1;

    // Ambush only suppresses player actions in round 1
    if self.round == 2 {
        self.ambush_round_active = false;
        // Recalculate turn order with Even handicap from round 2 onward
        self.handicap = Handicap::Even;
        self.turn_order = calculate_turn_order(self);
        self.current_turn = 0;
    }
    // ... existing round logic ...
}
```

#### 2.5 Combat Log Entry for Ambush

In `src/game/systems/combat.rs`, in `handle_combat_started`, emit a combat log line
when `combat_event_type == Ambush`:

```src/game/systems/combat.rs#L1-10
if combat_res.combat_event_type == CombatEventType::Ambush {
    combat_log.push_line(CombatLogLine::status(
        "The monsters ambush the party! The party is surprised!",
    ));
}
```

#### 2.6 Announce Normal Combat

For `CombatEventType::Normal`, emit the existing "Monsters appear!" log entry (or
equivalent) unchanged. No additional work needed — this is the current behavior.

#### 2.7 Random Rest Encounters as Ambushes

When the rest system (Phase 3 of the Rest Plan) interrupts rest with an encounter,
it must pass `CombatEventType::Ambush` to `start_encounter()`. Document this
requirement in `src/domain/resources.rs` with a code comment.

#### 2.8 Testing Requirements

- `test_normal_combat_handicap_is_even` — `start_encounter(..., Normal)` produces
  `CombatState` with `handicap == Handicap::Even`.
- `test_ambush_combat_handicap_is_monster_advantage` — `start_encounter(..., Ambush)`
  produces `CombatState` with `handicap == Handicap::MonsterAdvantage`.
- `test_ambush_round_active_set_on_start` — `cs.ambush_round_active == true` after
  ambush encounter initialization.
- `test_ambush_round_active_cleared_at_round_2` — after `advance_turn()` progresses
  into round 2, `ambush_round_active == false`.
- `test_ambush_player_turn_is_skipped` — in round 1 of an ambush, player action
  dispatch is suppressed; the turn advances without applying player damage.
- `test_ambush_player_can_act_round_2` — in round 2 of an ambush, player action
  dispatch is not suppressed.
- `test_ambush_handicap_resets_to_even_round_2` — after round 1, `handicap == Even`.
- `test_combat_log_reports_ambush` — combat log contains "ambush" text on ambush start.

#### 2.9 Deliverables

- [ ] `ambush_round_active: bool` on `CombatState`
- [ ] `TurnAction::Skip` variant in `src/domain/combat/types.rs`
- [ ] `start_encounter()` sets `ambush_round_active` and `MonsterAdvantage` for Ambush
- [ ] `start_encounter()` sets boss flags for Boss type
- [ ] Ambush player-turn skip logic in `combat_input_system`
- [ ] `advance_round()` clears `ambush_round_active` and resets handicap at round 2
- [ ] Combat log entry for ambush start
- [ ] All phase-2 tests pass

#### 2.10 Success Criteria

An encounter with `combat_event_type: Ambush` in a map RON file causes the party to
miss their actions in round 1. From round 2 onward combat is identical to Normal.
The combat log clearly states the party was ambushed.

---

### Phase 3: Ranged and Magic Combat

Implements the behavioral differences for `Ranged` and `Magic` combat. After this
phase, ranged combat adds a "Ranged Attack" action button (enabled only when the
active character has a ranged weapon), and magic combat reorders the action menu to
highlight "Cast Spell" first.

#### 3.1 Add `RangedAttack` to `TurnAction`

In `src/domain/combat/types.rs`, add a new turn action:

```src/domain/combat/types.rs#L1-10
pub enum TurnAction {
    Attack,
    RangedAttack, // New: ranged weapon attack, available in Ranged combat
    Defend,
    Flee,
    CastSpell,
    UseItem,
    Skip, // Internal: used for ambush-suppressed turns
}
```

#### 3.2 Add `has_ranged_weapon()` Helper to `Character` and `Monster`

In `src/domain/character.rs` (or `src/domain/combat/engine.rs`), add a helper to
determine if a combatant can perform a ranged attack:

```src/domain/character.rs#L1-15
/// Returns true if the character has a ranged weapon equipped.
///
/// A ranged weapon is any weapon with `WeaponClassification::MartialRanged`
/// and sufficient ammo in inventory (at least one `AmmoData` item).
pub fn has_ranged_weapon(character: &Character, items: &[Item]) -> bool {
    if let Some(weapon_slot) = &character.equipment.weapon {
        if let Some(item) = items.iter().find(|i| i.id == weapon_slot.item_id) {
            if let ItemType::Weapon(data) = &item.item_type {
                return data.classification
                    == Some(WeaponClassification::MartialRanged);
            }
        }
    }
    false
}
```

For monsters, a ranged capability is determined by whether any `Attack` in
`monster.attacks` has an `AttackType` other than `Physical` (or by a dedicated
`is_ranged: bool` flag added to `Attack` in Phase 3.2.1 below).

##### 3.2.1 Add `is_ranged: bool` to `Attack`

In `src/domain/combat/types.rs`:

```src/domain/combat/types.rs#L1-15
pub struct Attack {
    pub damage: DiceRoll,
    pub attack_type: AttackType,
    pub special_effect: Option<SpecialEffect>,
    /// True if this attack can be performed at range (before melee closes).
    /// Used in Ranged combat to determine whether the monster fires first.
    #[serde(default)]
    pub is_ranged: bool,
}
```

#### 3.3 Render Ranged Action Button Conditionally

In `src/game/systems/combat.rs`, in `setup_combat_ui` (or `update_combat_ui`), add
the Ranged Attack button to the action menu only when
`combat_res.combat_event_type == CombatEventType::Ranged`. Disable it (grey it out)
if the active character does not have a ranged weapon:

```src/game/systems/combat.rs#L1-20
// Ranged Attack button — only visible in Ranged combat
if combat_res.combat_event_type == CombatEventType::Ranged {
    let can_use_ranged = has_ranged_weapon_from_combat(
        &combat_res.state,
        current_combatant_id,
        &game_data,
    );
    spawn_action_button(
        parent,
        ActionButtonType::RangedAttack,
        can_use_ranged,
    );
}
```

Add `ActionButtonType::RangedAttack` to the `ActionButtonType` enum in
`src/game/systems/combat.rs`.

#### 3.4 Implement `perform_ranged_attack_action_with_rng()`

In `src/game/systems/combat.rs`, add a new function parallel to
`perform_attack_action_with_rng()`:

```src/game/systems/combat.rs#L1-30
/// Performs a ranged attack for a player character.
///
/// Uses the equipped ranged weapon's damage stat.  Accuracy modifier is
/// applied using the character's Accuracy stat.  Consumes one ammo item
/// from the character's inventory.  If no ammo is available, returns an
/// error and the turn is not consumed.
pub fn perform_ranged_attack_action_with_rng(
    combat: &mut CombatState,
    attacker_id: CombatantId,
    target_id: CombatantId,
    items: &[Item],
    rng: &mut impl Rng,
) -> Result<AttackOutcome, CombatError> {
    // 1. Verify attacker has ranged weapon + ammo
    // 2. Roll to-hit using Accuracy stat + weapon bonus
    // 3. Consume one ammo
    // 4. Apply damage using weapon damage dice
    // 5. Return AttackOutcome
}
```

Monster ranged attacks are resolved in `perform_monster_turn_with_rng()` by selecting
attacks where `attack.is_ranged == true` when `combat_event_type == Ranged`.

#### 3.5 Reorder Action Menu for Magic Combat

In `src/game/systems/combat.rs`, define a separate action order constant for magic
combat. When `combat_res.combat_event_type == CombatEventType::Magic`, use
`COMBAT_ACTION_ORDER_MAGIC` instead of `COMBAT_ACTION_ORDER`:

```src/game/systems/combat.rs#L1-10
/// Action button order for magic combat: Cast Spell first.
pub const COMBAT_ACTION_ORDER_MAGIC: [ActionButtonType; COMBAT_ACTION_COUNT] = [
    ActionButtonType::Cast,
    ActionButtonType::Attack,
    ActionButtonType::Defend,
    ActionButtonType::Item,
    ActionButtonType::Flee,
];
```

The `Cast` button is always enabled regardless of available spell points (greyed if
no SP, but still at the top of the list so the player sees it).

#### 3.6 Monster Ranged Attacks in Ranged Combat

In `perform_monster_turn_with_rng()`, when `combat_event_type == Ranged`, prefer
selecting a `monster.attacks` entry where `attack.is_ranged == true` before falling
back to melee. Use the existing `choose_monster_attack()` function as a starting
point; modify it to accept an `is_ranged_combat: bool` parameter.

#### 3.7 Combat Log for Ranged and Magic Combat

Emit a type-specific opening log line:

- `Ranged`: "Combat begins at range! Draw your bows!"
- `Magic`: "The air crackles with magical energy!"

#### 3.8 Testing Requirements

- `test_ranged_combat_shows_ranged_button` — `CombatEventType::Ranged` causes the
  Ranged Attack button to be spawned in the UI.
- `test_ranged_button_disabled_without_ranged_weapon` — character with no ranged
  weapon sees a disabled Ranged Attack button.
- `test_ranged_button_enabled_with_ranged_weapon` — character with a bow equipped
  sees an enabled Ranged Attack button.
- `test_perform_ranged_attack_consumes_ammo` — after `perform_ranged_attack_action_with_rng()`,
  ammo count decreases by 1.
- `test_perform_ranged_attack_no_ammo_returns_error` — no ammo → action is rejected
  and turn is not consumed.
- `test_magic_combat_cast_is_first_action` — `CombatEventType::Magic` causes Cast to
  be the first button in the action order.
- `test_magic_combat_normal_handicap` — Magic combat uses `Handicap::Even`.
- `test_monster_ranged_attack_preferred_in_ranged_combat` — monster with a ranged
  attack uses it when `is_ranged_combat == true`.
- `test_combat_log_ranged_opening` — log contains "range" text for Ranged combat.
- `test_combat_log_magic_opening` — log contains "magical" text for Magic combat.

#### 3.9 Deliverables

- [ ] `TurnAction::RangedAttack` and `TurnAction::Skip` in `src/domain/combat/types.rs`
- [ ] `is_ranged: bool` field on `Attack`
- [ ] `has_ranged_weapon()` helper for characters
- [ ] `ActionButtonType::RangedAttack` in `src/game/systems/combat.rs`
- [ ] Ranged Attack button conditionally spawned and enabled/disabled
- [ ] `perform_ranged_attack_action_with_rng()` with ammo consumption
- [ ] `COMBAT_ACTION_ORDER_MAGIC` constant and conditional action menu reorder
- [ ] Monster ranged attack preference in `perform_monster_turn_with_rng()`
- [ ] Type-specific combat log opening lines
- [ ] All phase-3 tests pass

#### 3.10 Success Criteria

In a Ranged encounter, a character with a bow can fire a ranged attack that consumes
ammo. In a Magic encounter, Cast Spell appears first in the action menu. Both types
use Even initiative. All four `cargo` quality gates pass with zero warnings.

---

### Phase 4: Boss Combat

Implements the Boss combat event type. After this phase, encounters with
`combat_event_type: Boss` have monsters that advance each round, may regenerate HP,
cannot be bribed or surrendered to, and display a distinct boss health bar.

#### 4.1 Boss Mechanic Constants

Add to `src/domain/combat/types.rs` (or `src/domain/resources.rs`):

```src/domain/combat/types.rs#L1-10
/// HP regeneration per round for boss monsters when `monsters_regenerate` is true.
pub const BOSS_REGEN_PER_ROUND: u16 = 5;

/// Multiplier applied to monster stats when boss mechanics are active.
/// E.g. 1.0 = no change. Can be raised by campaign config in a future phase.
pub const BOSS_STAT_MULTIPLIER: f32 = 1.0;
```

These are deliberately conservative defaults. Campaign data authors can adjust
individual monster stats via the monsters RON file.

#### 4.2 Configure Boss State in `start_encounter()`

Phase 2 already sets `monsters_advance = true`, `monsters_regenerate = true`,
`can_bribe = false`, and `can_surrender = false` for Boss combat. This phase
verifies those flags are exercised correctly and adds the UI signal.

#### 4.3 Boss HP Bar Component

In `src/game/systems/combat.rs`, add a distinct component and visual style for boss
monsters:

```src/game/systems/combat.rs#L1-20
/// Marker component for the boss HP bar (wider, more prominent than normal).
#[derive(Component)]
pub struct BossHpBar {
    pub participant_index: usize,
}

pub const BOSS_HP_BAR_WIDTH: f32 = 400.0;
pub const BOSS_HP_BAR_HEIGHT: f32 = 20.0;
pub const BOSS_HP_HEALTHY_COLOR: Color = Color::srgba(0.8, 0.1, 0.1, 1.0);
pub const BOSS_HP_INJURED_COLOR: Color = Color::srgba(0.5, 0.1, 0.1, 1.0);
pub const BOSS_HP_CRITICAL_COLOR: Color = Color::srgba(0.3, 0.05, 0.05, 1.0);
```

In `setup_combat_ui`, when `combat_res.combat_event_type == Boss`, spawn a
`BossHpBar` node in addition to (or replacing) the standard `EnemyHpBarFill` for the
boss monster. Place the boss bar in a dedicated panel above the standard enemy cards.

#### 4.4 Boss Cannot Flee Modifier

When `combat_event_type == Boss`, the monster's `flee_threshold` is effectively 0 —
boss monsters do not flee regardless of HP. Enforce this in `check_combat_end()`
or in `perform_monster_turn_with_rng()`:

```src/game/systems/combat.rs#L1-10
// Boss monsters never flee
let should_flee = if combat_res.combat_event_type == CombatEventType::Boss {
    false
} else {
    monster.should_flee()
};
```

#### 4.5 Combat Log for Boss Combat

Emit a prominent opening log line for boss encounters:

```src/game/systems/combat.rs#L1-5
"A powerful foe stands before you! Prepare for a legendary battle!"
```

Additionally, at the start of each round, if monsters regenerated HP this round,
emit: `"{monster_name} regenerates {amount} HP!"`.

#### 4.6 Boss Combat Victory Summary

In `process_combat_victory_with_rng()`, detect `combat_event_type == Boss` on the
`CombatResource` and add a "Boss Defeated!" header line to the victory summary.

#### 4.7 Testing Requirements

- `test_boss_combat_monsters_advance` — `cs.monsters_advance == true` after boss
  encounter initialization.
- `test_boss_combat_monsters_regenerate` — `cs.monsters_regenerate == true`.
- `test_boss_combat_cannot_bribe` — `cs.can_bribe == false`.
- `test_boss_combat_cannot_surrender` — `cs.can_surrender == false`.
- `test_boss_monster_does_not_flee` — a boss monster at 1 HP does not trigger the
  flee path.
- `test_boss_monster_regenerates_each_round` — after `advance_round()`, a boss
  monster's HP increases by `BOSS_REGEN_PER_ROUND`.
- `test_boss_hp_bar_spawned` — `BossHpBar` component is present in the ECS world
  after combat UI setup with Boss type.
- `test_boss_victory_summary_has_boss_header` — victory summary for a boss encounter
  contains "Boss Defeated!" text.
- `test_normal_combat_no_boss_bar` — Normal combat does not spawn `BossHpBar`.

#### 4.8 Deliverables

- [ ] `BOSS_REGEN_PER_ROUND` and `BOSS_STAT_MULTIPLIER` constants
- [ ] `BossHpBar` component and visual constants in `src/game/systems/combat.rs`
- [ ] Boss HP bar spawned in `setup_combat_ui` for Boss encounters
- [ ] Boss monsters never flee (enforced in `perform_monster_turn_with_rng()`)
- [ ] Boss regeneration log line per round
- [ ] "Boss Defeated!" victory summary header
- [ ] All phase-4 tests pass

#### 4.9 Success Criteria

A Boss encounter has all four boss flags set, the monster cannot flee, the HP
regenerates each round, and the UI shows a prominent boss health bar. Victory prints
a boss-specific header. All four `cargo` quality gates pass with zero warnings.

---

### Phase 5: Campaign Builder UI

Adds a `CombatEventType` combo-box to the Campaign Builder map editor's encounter
event editor, so campaign authors can select and save the encounter type per-event
and per-random-encounter-group without editing RON files by hand.

#### 5.1 Add `combat_event_type` to `EventEditorState`

In `sdk/campaign_builder/src/map_editor.rs`, extend `EventEditorState`:

```sdk/campaign_builder/src/map_editor.rs#L1-10
pub struct EventEditorState {
    pub event_type: EventType,
    pub position: Position,
    pub name: String,
    pub description: String,
    pub encounter_monsters: Vec<MonsterId>,
    pub encounter_monsters_query: String,
    /// Combat event type selected for this encounter.
    pub encounter_combat_event_type: CombatEventType,
    // ... existing treasure, teleport, trap, sign fields ...
}
```

Update `impl Default for EventEditorState`:

```sdk/campaign_builder/src/map_editor.rs#L1-5
encounter_combat_event_type: CombatEventType::Normal,
```

#### 5.2 Add Combo-Box to the Encounter Event Editor Panel

In `show_event_editor()` in `sdk/campaign_builder/src/map_editor.rs`, after the
monster selector for `EventType::Encounter`, add:

```sdk/campaign_builder/src/map_editor.rs#L1-25
// Combat Event Type selector
ui.horizontal(|ui| {
    ui.label("Combat Type:");
    egui::ComboBox::from_id_salt("encounter_combat_event_type")
        .selected_text(
            event_editor.encounter_combat_event_type.display_name()
        )
        .show_ui(ui, |ui| {
            for variant in CombatEventType::all() {
                ui.selectable_value(
                    &mut event_editor.encounter_combat_event_type,
                    *variant,
                    variant.display_name(),
                )
                .on_hover_text(variant.description());
            }
        });
});
ui.label(
    egui::RichText::new(
        event_editor.encounter_combat_event_type.description()
    )
    .small()
    .color(egui::Color32::GRAY),
);
```

#### 5.3 Update `to_map_event()` to Include `combat_event_type`

In `impl EventEditorState`, update `to_map_event()`:

```sdk/campaign_builder/src/map_editor.rs#L1-15
EventType::Encounter => {
    let monsters: Vec<MonsterId> = self.encounter_monsters.clone();
    if monsters.is_empty() {
        return Err("Encounter must have at least one monster ID".to_string());
    }
    Ok(MapEvent::Encounter {
        name: self.name.clone(),
        description: self.description.clone(),
        monster_group: monsters,
        combat_event_type: self.encounter_combat_event_type,
    })
}
```

#### 5.4 Update `from_map_event()` to Read `combat_event_type`

In `impl EventEditorState`, update `from_map_event()`:

```sdk/campaign_builder/src/map_editor.rs#L1-15
MapEvent::Encounter {
    name,
    description,
    monster_group,
    combat_event_type,
} => {
    s.event_type = EventType::Encounter;
    s.name = name.clone();
    s.description = description.clone();
    s.encounter_monsters = monster_group.clone();
    s.encounter_combat_event_type = *combat_event_type;
}
```

#### 5.5 Add `CombatEventType` to `EncounterGroup` Editor in the Map Metadata Panel

In `show_metadata_editor()` in `sdk/campaign_builder/src/map_editor.rs`, in the
section that lists random encounter groups, render the encounter type for each group
with a per-row combo-box. Use `ui.push_id(group_index, |ui| { ... })` to avoid ID
clashes.

#### 5.6 Display Combat Type in Event Inspector Panel

In `show_inspector_panel()`, extend the existing encounter branch:

```sdk/campaign_builder/src/map_editor.rs#L1-10
MapEvent::Encounter {
    monster_group,
    combat_event_type,
    ..
} => {
    ui.label(format!("Encounter: {:?}", monster_group));
    ui.label(format!("Type: {}", combat_event_type.display_name()));
    ui.label(
        egui::RichText::new(combat_event_type.description())
            .small()
            .color(egui::Color32::GRAY),
    );
}
```

#### 5.7 Update Event Type Color Mapping for Ambush

In `sdk/campaign_builder/src/map_editor.rs`, add a color constant for ambush
encounters to help authors distinguish them visually on the map grid (ambush encounters
share the Encounter event type but can be tinted differently in the inspector):

```sdk/campaign_builder/src/map_editor.rs#L1-5
/// Darker red for Ambush encounters on the inspector overlay
const COMBAT_TYPE_COLOR_AMBUSH: Color32 = Color32::from_rgb(180, 60, 70);
const COMBAT_TYPE_COLOR_BOSS: Color32 = Color32::from_rgb(220, 50, 50);
const COMBAT_TYPE_COLOR_RANGED: Color32 = Color32::from_rgb(209, 154, 102);
const COMBAT_TYPE_COLOR_MAGIC: Color32 = Color32::from_rgb(198, 120, 221);
```

These colors are used in the inspector detail panel only, not on the grid tiles
(all encounter tiles remain red `EVENT_COLOR_ENCOUNTER` on the grid for simplicity).

#### 5.8 Testing Requirements

- `test_event_editor_state_default_combat_type` — `EventEditorState::default()` has
  `encounter_combat_event_type == CombatEventType::Normal`.
- `test_to_map_event_preserves_combat_type` — `to_map_event()` on a state with
  `encounter_combat_event_type == Ambush` produces `MapEvent::Encounter` with
  `combat_event_type == Ambush`.
- `test_from_map_event_reads_combat_type` — `from_map_event()` on a Boss encounter
  sets `encounter_combat_event_type == Boss`.
- `test_from_map_event_normal_type_on_missing_field` — `from_map_event()` on an
  encounter without the field defaults to `Normal` (backward compat).
- `test_combat_type_combo_box_has_all_variants` — the combo-box lists exactly 5
  variants matching `CombatEventType::all()`.

#### 5.9 Deliverables

- [ ] `encounter_combat_event_type: CombatEventType` field on `EventEditorState`
- [ ] `CombatEventType` combo-box in `show_event_editor()` for Encounter type
- [ ] `to_map_event()` forwards `combat_event_type`
- [ ] `from_map_event()` reads `combat_event_type`
- [ ] Per-group `CombatEventType` selector in the random encounter table editor
- [ ] Combat type displayed in the inspector panel
- [ ] `push_id` used for all group-level combo-boxes (no egui ID clashes)
- [ ] All phase-5 tests pass

#### 5.10 Success Criteria

A campaign author can open a map in the Campaign Builder, select an Encounter event,
choose "Ambush" from the Combat Type combo-box, save the map, and confirm the saved
RON contains `combat_event_type: Ambush`. The inspector panel shows the type for all
encounters. All four `cargo` quality gates pass with zero warnings.

---

## RON Data Format Reference

After all phases are implemented, an encounter event in a `map.ron` file looks like:

```data/test_campaign/data/maps/map_1.ron#L1-20
(
    // ... tile data ...
    events: {
        (x: 5, y: 10): Encounter(
            name: "Spider Ambush",
            description: "Spiders drop from the ceiling!",
            monster_group: [3, 3, 4],
            combat_event_type: Ambush,
        ),
        (x: 12, y: 8): Encounter(
            name: "Dragon Boss",
            description: "A mighty dragon blocks the way.",
            monster_group: [1],
            combat_event_type: Boss,
        ),
        (x: 7, y: 15): Encounter(
            name: "Goblin Archers",
            description: "Goblins fire arrows from a ridge.",
            monster_group: [5, 5, 6],
            combat_event_type: Ranged,
        ),
    },
    encounter_table: (
        encounter_rate: 0.1,
        groups: [
            (
                monster_group: [2, 2],
                combat_event_type: Normal,
            ),
            (
                monster_group: [7],
                combat_event_type: Ambush,
            ),
        ],
    ),
    // ...
)
```

Existing RON files without `combat_event_type` fields continue to work because all
new fields use `#[serde(default)]`.

---

## Architecture Compliance Checklist

- [ ] `CombatEventType` added to `src/domain/combat/types.rs` (Section 4.4 domain)
- [ ] `MapEvent::Encounter` updated in `src/domain/world/types.rs` (Section 4.2)
- [ ] `EventResult::Encounter` updated in `src/domain/world/events.rs` (Section 4.2)
- [ ] `EncounterGroup` replaces raw `Vec<MonsterId>` in `EncounterTable` (Section 4.2)
- [ ] Type aliases `MonsterId`, `MapId`, `ItemId` used throughout — no raw `u8`/`u32`
- [ ] `CombatState` fields match architecture Section 4.4 EXACTLY
- [ ] `AttributePair` pattern not modified
- [ ] RON format used for all data files; no JSON or YAML data files introduced
- [ ] All constants extracted (`BOSS_REGEN_PER_ROUND`, `BOSS_STAT_MULTIPLIER`, etc.)
- [ ] SDK `push_id` used for all loop-rendered combo-boxes (no egui ID clashes)
- [ ] No test references `campaigns/tutorial` — all fixtures in `data/test_campaign`
- [ ] `docs/explanation/implementations.md` updated after implementation

---

## Decisions

1. **`EncounterTable.groups` type change** — The raw `Vec<Vec<u8>>` (where `u8` was
   used as a monster ID, inconsistent with `MonsterId = String`) is replaced by
   `Vec<EncounterGroup>`. This is a breaking change to the RON format for
   `encounter_table.groups`, but the project rules explicitly state backwards
   compatibility is not a concern right now. All existing map files must be migrated.
   A one-time migration script (`src/bin/update_tutorial_maps.rs`) should be run to
   update `campaigns/tutorial` maps.

2. **Ambush handicap reset at round 2** — The `MonsterAdvantage` handicap only
   applies to round 1. From round 2 onward, `Handicap::Even` is restored and
   `calculate_turn_order()` is re-run. This keeps the surprise mechanic limited to the
   first round as specified.

3. **Rest-interrupted encounters are always ambushes** — Any encounter that fires
   while the party is resting uses `CombatEventType::Ambush`. The resting party is
   asleep and cannot react. This requirement is documented in `src/domain/resources.rs`
   and enforced by the rest system implementation.

4. **Ranged button availability** — The "Ranged Attack" button appears in the UI
   only when `combat_event_type == Ranged`. It is not shown in Normal, Ambush, Magic,
   or Boss combat even if the character has a bow equipped. Ranged weapons can still
   be used as normal melee attacks (via the Attack button) in non-Ranged encounters.
   This keeps the combat UI consistent and avoids conditional clutter.

5. **Magic combat does not restrict spells** — `CombatEventType::Magic` only
   reorders the action menu; it does not restrict or enhance spells. The party can
   still attack normally; the reordering signals to the player that magic is the
   expected tactic. Future phases could add spell-specific bonuses for Magic combat.

6. **Boss mechanic flags are all-or-nothing per encounter** — Boss mechanics
   (`monsters_advance`, `monsters_regenerate`, no bribe, no surrender) apply to all
   monsters in the encounter group, not just a designated "boss" monster. If a
   campaign needs one boss among minions, the encounter should be designed with a
   single monster group containing the boss, and regular encounters placed separately
   for minions. A future enhancement could add a `is_boss: bool` per-monster flag.

7. **`CombatEventType` is not a `GameMode`** — It is metadata about how a specific
   combat begins and what options are available. `GameMode::Combat(CombatState)`
   remains unchanged. `CombatEventType` lives on `CombatState` (Phase 2 stores it
   there) and on `CombatResource` in the Bevy layer.
