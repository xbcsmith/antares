# Spell System Updates Implementation Plan

## Overview

The spell system in Antares has a solid data model and validation layer already
in place, but the **effect execution pipeline is partial** (damage + resurrection
only), the **UI layer is incomplete** (no SP bar, no exploration cast UI, combat
Cast submenu is stubbed), and **buff/heal/utility spell effects** lack a dispatch
mechanism to actually modify game state. This plan addresses every gap in a
six-phase approach, ordered so each phase can be implemented sequentially:
effect engine, HUD/combat UI, exploration casting, spell learning and
acquisition, data completion and advanced features, and SDK/content tooling.

## Current State Analysis

### Existing Infrastructure

The following spell-related code is already implemented and working:

| Layer                  | File(s)                                                                                       | What Exists                                                                                                                              |
| ---------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| **Data types**         | `src/domain/magic/types.rs`                                                                   | `Spell`, `SpellSchool`, `SpellContext`, `SpellTarget`, `SpellResult`, `SpellError`                                                       |
| **Database**           | `src/domain/magic/database.rs`                                                                | `SpellDatabase` — loads from RON, queries by school/level                                                                                |
| **SpellBook**          | `src/domain/character.rs`                                                                     | Per-character `SpellBook` with `cleric_spells` and `sorcerer_spells` arrays (7 levels each), class-routed accessors                      |
| **Casting validation** | `src/domain/magic/casting.rs`                                                                 | `can_cast_spell`, `can_class_cast_school`, `get_required_level_for_spell`, `calculate_spell_points`, `cast_spell` (consumes SP/gems)     |
| **Combat execution**   | `src/domain/combat/spell_casting.rs`                                                          | `execute_spell_cast_with_spell` — validates, consumes resources, applies damage, applies conditions, handles resurrection, advances turn |
| **Condition effects**  | `src/domain/magic/spell_effects.rs`                                                           | `apply_spell_conditions_to_character`, `apply_spell_conditions_to_monster`, `apply_condition_dot_effects`                                |
| **ActiveSpells**       | `src/application/mod.rs`                                                                      | Party-wide buff tracking with 18 `u8` duration fields and `tick()` method                                                                |
| **Bevy ECS**           | `src/game/systems/combat.rs`                                                                  | `CastSpellAction` event, `handle_cast_spell_action` system, `SpellSelectionPanel` / `SpellButton` components, `ActionButtonType::Cast`   |
| **HUD**                | `src/game/systems/hud.rs`                                                                     | `HpBarFill`, `HpTextOverlay`, `hp_bar_color()` — HP bar with color thresholds, per-character cards                                       |
| **Save/load**          | `src/application/save_game.rs`                                                                | `GameState` serializes `SpellBook`, `ActiveSpells`, `CombatState` automatically via `Serialize`/`Deserialize`                            |
| **Spell data**         | `data/spells.ron`, `data/test_campaign/data/spells.ron`, `campaigns/tutorial/data/spells.ron` | ~35 spells defined across Cleric L1–L5 and Sorcerer L1–L3                                                                                |

### Identified Issues

1. **No healing spell logic** — `execute_spell_cast_with_spell` handles damage
   dice and resurrection but has no generic "heal X HP" path for spells like
   First Aid, Cure Wounds, or Power Cure.

2. **No buff/utility spell application** — Spells like Light, Bless, Shield,
   Levitate, Walk on Water need to write durations into `ActiveSpells` fields,
   but no code maps spell IDs to `ActiveSpells` field writes.

3. **No exploration-mode casting** — There is no equivalent of
   `execute_spell_cast_with_spell` for non-combat contexts (healing between
   fights, casting Light, Town Portal, Create Food, etc.).

4. **No spell learning system** — `DialogueAction` has no `LearnSpell` variant.
   `QuestReward` has no spell-granting variant. There is no `learn_spell()`
   domain function to safely add a spell to a character's `SpellBook`.

5. **No SP bar on HUD** — The HUD displays HP bars only. SP is tracked on
   `Character.sp: AttributePair16` but is not visualized.

6. **Spell scroll / item-cast pipeline incomplete** — `Item` has
   `spell_effect: Option<SpellId>` but the pipeline to trigger a spell from an
   item (scroll use, wand charges) is not wired in combat or exploration.

7. **Combat spell selection UI partial** — `SpellSelectionPanel` and
   `SpellButton` components exist, but the full spell picker flow (browse
   spellbook by level, select target) may be incomplete or stubbed.

8. **Monster spell casting absent** — Monsters have special attacks but no
   system for them casting spells from the spell database.

9. **Spell data incomplete** — Only ~35 spells exist in RON data. Architecture
   describes Cleric L1–L7 and Sorcerer L1–L7 (potentially 70+ spells).

10. **No `ConsumableEffect::CastSpell` variant** — Spell scrolls would need
    a new consumable effect variant or a new `ItemType` variant.

## Implementation Phases

Each phase is numbered in the order it should be implemented. Every phase
builds only on work completed in prior phases — no forward dependencies.

---

### Phase 1: Spell Effect Resolution Engine

**Goal**: Build the missing spell effect dispatch layer so that every spell
category (damage, healing, buff, debuff, utility, resurrection) resolves
correctly through a single pipeline.

**Dependencies**: None — this is the foundational phase.

#### 1.1 Create Spell Effect Dispatcher

Add a new module `src/domain/magic/effect_dispatch.rs` that maps `Spell`
definitions to concrete game state mutations. This is the central routing
function that both combat and exploration casting will call.

**Key function signatures:**

- `apply_spell_effect(spell, caster, target, active_spells, party, rng) -> SpellEffectResult`
- `apply_healing_spell(spell, target_character) -> HealResult`
- `apply_buff_spell(spell, active_spells) -> BuffResult`
- `apply_utility_spell(spell, party, world) -> UtilityResult`

**Spell effect categories to dispatch:**

| Category           | Examples                                          | Target State Mutation                   |
| ------------------ | ------------------------------------------------- | --------------------------------------- |
| Direct damage      | Flame Arrow, Fireball, Lightning Bolt             | `target.hp.modify(-damage)`             |
| Direct healing     | First Aid, Cure Wounds, Power Cure                | `target.hp.modify(+healing)`            |
| Condition cure     | Cure Blindness, Cure Paralysis, Neutralize Poison | `target.remove_condition(id)`           |
| Protection buff    | Protection from Fear/Cold/Fire/Poison             | `active_spells.{field} = duration`      |
| Combat buff        | Bless, Power, Quickness, Shield, Heroism          | `active_spells.{field} = duration`      |
| Utility — light    | Light, Lasting Light                              | `active_spells.light = duration`        |
| Utility — movement | Walk on Water, Levitate, Fly                      | `active_spells.{field} = duration`      |
| Utility — info     | Location, Detect Magic, Identify Monster          | Return info result (no state change)    |
| Utility — creation | Create Food                                       | `party.food += amount`                  |
| Utility — teleport | Town Portal, Surface, Jump                        | Modify `world.party_position`           |
| Debuff             | Sleep, Silence, Blind, Feeble Mind                | Apply condition to monster(s)           |
| Resurrection       | Resurrect                                         | `revive_from_dead(target, hp)`          |
| Invisibility       | Invisibility                                      | `active_spells.invisibility = duration` |

#### 1.2 Add Spell Effect Metadata to `Spell` Struct

Extend `Spell` in `src/domain/magic/types.rs` with a new field to classify
the effect type so the dispatcher can route correctly:

- Add `pub effect_type: SpellEffectType` enum field to `Spell`
- `SpellEffectType` variants: `Damage`, `Healing { amount: DiceRoll }`,
  `CureCondition { condition_id: String }`, `Buff { buff_field: BuffField, duration: u16 }`,
  `Utility { utility_type: UtilityType }`, `Debuff`, `Resurrection`,
  `Composite(Vec<SpellEffectType>)`
- Add `#[serde(default)]` so existing RON data continues to load
- Default to inferring effect type from existing fields (damage dice presence,
  `resurrect_hp`, `applied_conditions`) via a `Spell::infer_effect_type()` method

#### 1.3 Integrate Dispatcher into Combat Spell Execution

Refactor `execute_spell_cast_with_spell` in
`src/domain/combat/spell_casting.rs` to delegate to the new effect dispatcher
for healing, buff, and utility effects instead of only handling damage and
resurrection inline. The existing damage path remains but is also routed
through the dispatcher for consistency.

#### 1.4 Testing Requirements

- Unit tests for each spell effect category in `effect_dispatch.rs`
- Test healing spells restore HP correctly (single target and party-wide)
- Test buff spells write correct durations to `ActiveSpells` fields
- Test condition cure spells remove the correct condition
- Test utility spells (Create Food adds food, Light sets `active_spells.light`)
- Test composite spells (e.g., a spell that heals and removes a condition)
- Test that existing damage and resurrection paths still work after refactor
- All test data from `data/test_campaign/`

#### 1.5 Deliverables

- [ ] `src/domain/magic/effect_dispatch.rs` — spell effect dispatcher module
- [ ] `SpellEffectType` enum in `src/domain/magic/types.rs`
- [ ] `Spell::infer_effect_type()` fallback method
- [ ] Refactored `execute_spell_cast_with_spell` using dispatcher
- [ ] Unit tests with >80% coverage for all effect categories
- [ ] Updated `src/domain/magic/mod.rs` to export new module

#### 1.6 Success Criteria

- All existing combat spell tests pass unchanged
- New healing/buff/utility spell effects produce correct state mutations
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo nextest run --all-features` passes with 0 failures

---

### Phase 2: HUD Spell Point Bar and Combat UI Completion

**Goal**: Add a visible SP bar to the HUD (blue, modeled after the HP bar) and
complete the combat spell selection UI flow.

**Dependencies**: Phase 1 (effect dispatcher used by combat UI feedback and
spell cast execution). The SP bar itself (2.1–2.2) has no dependency on
Phase 1 and can begin immediately in parallel if desired.

#### 2.1 Add SP Bar to HUD Character Cards

Extend `src/game/systems/hud.rs` following the existing `HpBarFill` /
`HpTextOverlay` pattern:

**New components:**

- `SpBarFill { party_index: usize }` — colored fill node for SP bar
- `SpBarTextOverlay { party_index: usize }` — "current/max" text overlay

**New constants:**

- `SP_BAR_HEIGHT: Val = Val::Px(8.0)` — slightly thinner than HP bar
- `SP_HEALTHY_COLOR` — blue (e.g., `Color::srgb(0.2, 0.4, 0.9)`)
- `SP_LOW_COLOR` — light blue (e.g., `Color::srgb(0.4, 0.6, 0.8)`)
- `SP_EMPTY_COLOR` — grey
- `SP_HEALTHY_THRESHOLD: f32 = 0.5`
- `SP_TEXT_COLOR` — white or contrast-aware

**Layout change:** Each character card gains a second bar below the HP bar.
The card layout becomes: Portrait → HP bar → SP bar → Condition text.

**Update `update_hud` system:** Add a loop for `sp_bar_query` that mirrors
the HP bar update logic but reads `character.sp.current` / `character.sp.base`.

**Non-caster handling:** If `character.sp.base == 0`, hide the SP bar
(set `display: Display::None`) to avoid visual clutter for Knights and Robbers.

#### 2.2 Add `sp_bar_color()` Utility Function

Mirror `hp_bar_color()` with SP-specific thresholds and colors. Add unit tests
for boundary conditions.

#### 2.3 Complete Combat Spell Selection UI

Verify and complete the combat spell picker flow in
`src/game/systems/combat.rs`:

- When `ActionButtonType::Cast` is selected, open `SpellSelectionPanel`
- Panel displays caster's known spells organized by level (1–7)
- Each `SpellButton` shows spell name, SP cost, and gem cost
- Greyed-out spells that can't be cast (insufficient SP, wrong context, etc.)
- After spell selection, prompt for target if needed (single monster, single
  character, etc.)
- On confirm, emit `CastSpellAction` event
- On cancel, return to action selection

#### 2.4 Add Spell Cast Feedback to Combat Log

Extend combat feedback in `src/game/systems/combat.rs` and
`src/game/systems/combat_visual.rs`:

- Display spell name and effect in combat log
- Show damage numbers for offensive spells
- Show healing amounts for healing spells
- Show "Spell fizzled!" for failed casts
- Show condition application messages ("Monster is now Asleep!")

#### 2.5 Sync SP During Combat

Extend `sync_party_hp_during_combat` in `src/game/systems/hud.rs` to also
sync SP values from `CombatResource.participants` back to `party.members` so
the SP bar updates in real-time during combat.

#### 2.6 Testing Requirements

- Test SP bar renders correctly for caster classes
- Test SP bar is hidden for non-casters (Knight, Robber)
- Test `sp_bar_color()` returns correct colors at thresholds
- Test SP bar updates after spell cast in combat
- Test spell selection panel displays correct spells per class
- Test greyed-out spells cannot be selected
- Test target selection flow for single-target vs. group spells
- All test data from `data/test_campaign/`

#### 2.7 Deliverables

- [ ] `SpBarFill`, `SpBarTextOverlay` components in HUD
- [ ] SP bar layout in character card setup
- [ ] `sp_bar_color()` function with unit tests
- [ ] SP bar update logic in `update_hud` system
- [ ] SP sync in `sync_party_hp_during_combat`
- [ ] Completed combat spell selection panel flow
- [ ] Spell cast feedback messages in combat log

#### 2.8 Success Criteria

- SP bar is visible for all spellcasting characters during exploration and
  combat
- SP bar updates in real-time as spells are cast
- Non-casters do not display an SP bar
- Combat spell picker allows full spell selection flow from spellbook
- All four quality gates pass

---

### Phase 3: Exploration-Mode Spell Casting

**Goal**: Allow characters to cast spells outside of combat — healing between
fights, casting Light in dungeons, Town Portal, Create Food, etc.

**Dependencies**: Phase 1 (effect dispatcher for resolving spell effects).
Phase 2 SP bar will automatically reflect SP changes from exploration casts.

#### 3.1 Create Exploration Casting Domain Module

Add `src/domain/magic/exploration_casting.rs` with functions that operate on
`GameState` directly (not `CombatState`):

- `can_cast_exploration_spell(character, spell, game_state) -> Result<(), SpellError>` —
  validates context is non-combat compatible, SP/gems sufficient, class/level OK
- `cast_exploration_spell(character, spell, target, game_state, rng) -> Result<SpellEffectResult, SpellError>` —
  consumes SP/gems, delegates to effect dispatcher, returns result
- `get_castable_exploration_spells(character, spell_db, game_state) -> Vec<&Spell>` —
  returns list of spells the character can currently cast outside combat

**Target resolution for exploration**:

| `SpellTarget`                                    | Exploration behavior                  |
| ------------------------------------------------ | ------------------------------------- |
| `Self_`                                          | Applies to caster only                |
| `SingleCharacter`                                | UI prompts for party member selection |
| `AllCharacters`                                  | Applies to all party members          |
| `SingleMonster` / `MonsterGroup` / `AllMonsters` | Returns `SpellError::CombatOnly`      |
| `SpecificMonsters`                               | Returns `SpellError::CombatOnly`      |

#### 3.2 Wire Exploration Casting into Application Layer

Add a `SpellCastingState` to `src/application/mod.rs` (or a new
`src/application/spell_casting_state.rs`) that tracks the multi-step UI flow:

1. Player opens spell menu → select caster (party member)
2. Display caster's known spells (from `SpellBook`) filtered to castable ones
3. Player selects spell → if target needed, prompt for target selection
4. Execute cast → show result feedback → return to exploration

#### 3.3 Create Exploration Spell Casting Bevy System

Add a new Bevy system (or extend existing systems) in
`src/game/systems/exploration_spells.rs`:

- `ExplorationSpellPlugin` with setup/update systems
- `GameMode::SpellCasting` variant (or reuse `GameMode::Menu` with sub-state)
- UI overlay: character selector → spell list → target selector → result
- Keyboard shortcut to open spell menu (e.g., `C` for Cast)
- Integrate with existing `ControlsConfig` for key binding

#### 3.4 Handle Utility Spell World Effects

Wire utility spells to actual world state changes:

- **Light** / **Lasting Light**: Set `active_spells.light` field, integrate
  with existing light system in `src/game/systems/` that checks
  `active_spells.light > 0`
- **Walk on Water**: Set `active_spells.walk_on_water`, integrate with
  movement validation in map traversal
- **Levitate** / **Fly**: Set `active_spells.levitate`, integrate with
  pit/chasm tile validation
- **Create Food**: Add food to `party.food`
- **Town Portal** / **Surface**: Modify `world.party_position` and
  `world.current_map` as appropriate
- **Location**: Display current coordinates in feedback message

#### 3.5 Testing Requirements

- Test exploration casting validates context correctly (rejects combat-only)
- Test SP/gem consumption in exploration mode
- Test healing spells restore party member HP outside combat
- Test buff spells update `ActiveSpells` fields with correct durations
- Test utility spells produce correct world state changes
- Test that `get_castable_exploration_spells` filters correctly by class,
  level, context, and available resources
- All test data from `data/test_campaign/`

#### 3.6 Deliverables

- [ ] `src/domain/magic/exploration_casting.rs` — exploration casting domain logic
- [ ] `src/application/spell_casting_state.rs` — multi-step UI state tracking
- [ ] `src/game/systems/exploration_spells.rs` — Bevy ECS plugin for exploration spell UI
- [ ] Utility spell world effect integrations (light, movement, food, teleport)
- [ ] Unit and integration tests for exploration casting
- [ ] Updated module exports in `src/domain/magic/mod.rs` and `src/game/systems/mod.rs`

#### 3.7 Success Criteria

- Characters can cast `Anytime` and `NonCombatOnly` spells during exploration
- `CombatOnly` spells are correctly rejected outside combat
- Healing spells restore HP, buff spells update `ActiveSpells`, utility spells
  modify world state
- All four quality gates pass

---

### Phase 4: Spell Learning and Acquisition

**Goal**: Implement the full spell acquisition pipeline — characters learn
spells via leveling, NPC dialogue, scrolls, and quest rewards.

**Dependencies**: Phase 1 (effect dispatcher for scroll-based spell casting).
Phase 3 (exploration casting pipeline for non-combat scroll use).

#### 4.1 Create Spell Learning Domain Functions

Add to `src/domain/magic/` (new file `learning.rs` or extend `casting.rs`):

- `learn_spell(character, spell_id, spell_db, class_db) -> Result<(), SpellLearnError>` —
  validates class compatibility and spell level access, adds to `SpellBook`
- `can_learn_spell(character, spell_id, spell_db, class_db) -> Result<(), SpellLearnError>` —
  checks class, level, and whether already known
- `get_learnable_spells(character, spell_db, class_db) -> Vec<SpellId>` —
  returns spells the character is eligible for but hasn't learned yet
- `grant_level_up_spells(character, new_level, spell_db, class_db) -> Vec<SpellId>` —
  returns spells newly accessible at this level (caller decides auto-learn policy)

**`SpellLearnError` variants**: `WrongClass`, `LevelTooLow`, `AlreadyKnown`,
`SpellNotFound`, `SpellBookFull` (if a cap is ever needed)

#### 4.2 Add `DialogueAction::LearnSpell`

Extend `DialogueAction` in `src/domain/dialogue.rs`:

- Add variant `LearnSpell { spell_id: SpellId, target_character_id: Option<CharacterId> }`
- If `target_character_id` is `None`, learn for first eligible party member
- Add `description()` match arm
- Wire handler in `src/game/systems/dialogue.rs` `execute_action()` to call
  `learn_spell()` from the domain layer

#### 4.3 Add `QuestReward::LearnSpell`

Extend `QuestReward` in `src/domain/quest.rs`:

- Add variant `LearnSpell { spell_id: SpellId }`
- Wire handler in `src/application/quests.rs` `apply_rewards()` to call
  `learn_spell()` for the first eligible party member (or all eligible)

#### 4.4 Add Spell Scroll Consumable

Extend the item system to support spell scrolls:

- Add `ConsumableEffect::CastSpell(SpellId)` variant in
  `src/domain/items/types.rs`
- Add `ConsumableEffect::LearnSpell(SpellId)` variant for scrolls that teach
  permanently (vs. single-use cast)
- Wire item-use handling in `src/domain/combat/item_usage.rs` for combat
  scroll use (delegates to spell casting pipeline)
- Wire exploration item use for non-combat scroll consumption

#### 4.5 Integrate Spell Learning with Level-Up

In the training/leveling system (wherever level-up is processed):

- After level-up, call `grant_level_up_spells()` to determine newly accessible
  spell levels
- Auto-learn policy: in Might and Magic 1 style, spells at the new level
  become available if the character visits a trainer or equivalent. Implement
  the simplest version first (auto-grant on level-up) with a configuration
  hook for gated learning.

#### 4.6 Testing Requirements

- Test `learn_spell` adds spell to correct school and level in `SpellBook`
- Test `learn_spell` rejects wrong class, too-low level, already-known
- Test `DialogueAction::LearnSpell` execution adds spell to character
- Test `QuestReward::LearnSpell` execution adds spell to eligible member
- Test `ConsumableEffect::LearnSpell` and `ConsumableEffect::CastSpell`
- Test level-up spell granting at key level boundaries (1, 3, 5, 7, 9, 11, 13)
- Test Paladin/Archer delayed access (no spells at level 1–2, gain at 3)
- All test data from `data/test_campaign/`

#### 4.7 Deliverables

- [ ] `src/domain/magic/learning.rs` — spell learning domain functions
- [ ] `DialogueAction::LearnSpell` variant + handler in dialogue system
- [ ] `QuestReward::LearnSpell` variant + handler in quest system
- [ ] `ConsumableEffect::CastSpell` and `ConsumableEffect::LearnSpell` variants
- [ ] Level-up spell granting integration
- [ ] Comprehensive unit tests for all learning paths
- [ ] Updated RON data in `data/test_campaign/data/` as needed for test fixtures

#### 4.8 Success Criteria

- Characters can learn spells through all four channels (level-up, dialogue,
  quest reward, scroll)
- Class and level restrictions are enforced on all channels
- Learned spells persist through save/load (automatic via `Serialize`)
- All four quality gates pass

---

### Phase 5: Complete Spell Data and Advanced Features

**Goal**: Flesh out the spell roster to cover all 7 levels for both schools,
add monster spell casting, and wire item-based spell effects.

**Dependencies**: Phases 1–4 (all core systems must be in place before
expanding content and adding advanced mechanics).

#### 5.1 Complete Spell RON Data

Expand `data/spells.ron` and `campaigns/tutorial/data/spells.ron` to include
full spell lists for both schools:

**Cleric spells to add** (Levels 4–7):

- Level 4: Cure Disease, Protection from Acid, Protection from Electricity,
  Holy Word, etc.
- Level 5: Raise Dead (already exists as Resurrect), Dispel Magic, Mass Cure
- Level 6: Stone to Flesh, Word of Recall, Restoration
- Level 7: Holy Word, Resurrection (full HP), Divine Intervention

**Sorcerer spells to add** (Levels 4–7):

- Level 4: Guard Dog, Power Shield, Slow, Web
- Level 5: Finger of Death, Shelter, Teleport, Disintegrate
- Level 6: Recharge Item, Stone to Flesh, Prismatic Spray
- Level 7: Implosion, Meteor Shower, Prismatic Sphere

Add `effect_type` field to all spell RON entries (or rely on inference from
Phase 1 initially).

Update `data/test_campaign/data/spells.ron` with representative test fixtures
for new spell levels.

#### 5.2 Wire Item Spell Effects

Complete the `spell_effect: Option<SpellId>` pipeline on `Item`:

- When a charged item is "used" in combat, look up `spell_effect` SpellId in
  `SpellDatabase`, create a `SpellCast`, and execute through the combat spell
  pipeline
- When a charged item is "used" in exploration, route through exploration
  casting pipeline
- Consume one charge per use (`ChargeState::use_charge()`)
- If charges are depleted, item becomes inert (already handled by
  `ChargeState::is_useless()`)

#### 5.3 Monster Spell Casting (Stretch Goal)

Add optional spell-casting capability to monsters:

- Add `pub spells: Vec<SpellId>` field to `Monster` struct in
  `src/domain/combat/monster.rs`
- In monster AI turn resolution, if monster has spells and is not silenced,
  choose between physical attack and spell cast based on situation
- Monster spell casts go through the same effect dispatcher but with monster
  as caster
- Monster SP is unlimited (or use a simple cooldown counter)

#### 5.4 Spell Fizzle System

Implement the fizzle mechanic described in architecture:

- Fizzle chance based on caster's primary stat (Intellect for Sorcerers,
  Personality for Clerics)
- Formula: `fizzle_chance = max(0, 50 - (primary_stat - 10) * 2)` percent
- On fizzle: SP is still consumed, but spell has no effect
- Display "Spell fizzled!" feedback
- Higher-level spells may have increased fizzle chance

#### 5.5 Dispel Magic Implementation

Implement the Dispel Magic spell that clears all `ActiveSpells`:

- Reset all `ActiveSpells` fields to 0
- Remove all active buff conditions from party members
- Special handling: if cast by enemy, affects player party; if cast by
  player, affects enemy buffs (if any)

#### 5.6 Testing Requirements

- Test all new spell definitions load correctly from RON
- Test item spell effect pipeline (combat and exploration)
- Test monster spell casting AI decision-making
- Test fizzle system probability distribution
- Test Dispel Magic clears all active spells
- All test data from `data/test_campaign/`

#### 5.7 Deliverables

- [ ] Complete spell definitions in `data/spells.ron` (L1–L7 both schools)
- [ ] Updated `data/test_campaign/data/spells.ron` with test fixtures
- [ ] Item spell effect pipeline (combat + exploration)
- [ ] Monster spell casting system (if implementing)
- [ ] Spell fizzle mechanic
- [ ] Dispel Magic implementation
- [ ] Comprehensive tests for all new features

#### 5.8 Success Criteria

- Full spell roster of 70+ spells loadable from RON data
- Items with `spell_effect` correctly cast spells when used
- Fizzle mechanic adds strategic depth to spell casting
- All four quality gates pass

---

### Phase 6: SDK and Content Tooling Updates

**Goal**: Update the Campaign Builder SDK to support creating, editing, and
validating spells, spell scrolls, and spell-granting dialogue/quest content.

**Dependencies**: Phases 1, 3, 4 (all new domain types and variants must be
finalized before the SDK exposes editors for them).

#### 6.1 Spell Editor in Campaign Builder

Add a spell editor panel to the SDK Campaign Builder UI:

- List all spells with filtering by school and level
- Create new spell definitions with all fields
- Edit existing spell definitions
- Validate spell data (unique IDs, valid school/level, positive costs)
- Export to RON format

#### 6.2 Update Item Editor for Spell Scrolls

Extend the existing item editor:

- Add `ConsumableEffect::CastSpell` and `ConsumableEffect::LearnSpell`
  options when editing consumable items
- Spell ID picker (dropdown from loaded `SpellDatabase`)
- Preview of spell details when selected

#### 6.3 Update Dialogue Editor for LearnSpell Action

Extend the dialogue editor:

- Add `DialogueAction::LearnSpell` to available actions in dialogue node/choice
  editors
- Spell ID picker for the `spell_id` field
- Optional character ID picker for `target_character_id`

#### 6.4 Update Quest Editor for Spell Rewards

Extend the quest editor:

- Add `QuestReward::LearnSpell` to available reward types
- Spell ID picker for the `spell_id` field

#### 6.5 Validation Framework Updates

Extend `src/sdk/validation/` to validate spell-related cross-references:

- Validate that `DialogueAction::LearnSpell` references valid `SpellId`s
- Validate that `QuestReward::LearnSpell` references valid `SpellId`s
- Validate that `ConsumableEffect::CastSpell` / `LearnSpell` reference valid
  `SpellId`s
- Validate that item `spell_effect` fields reference valid `SpellId`s
- Validate spell data integrity (no duplicate IDs, valid level ranges 1–7,
  non-negative costs)

#### 6.6 Testing Requirements

- Test spell editor creates valid spell definitions
- Test item editor produces valid spell scroll items
- Test dialogue editor produces valid `LearnSpell` actions
- Test quest editor produces valid spell rewards
- Test validation catches invalid cross-references
- All test data from `data/test_campaign/`

#### 6.7 Deliverables

- [ ] Spell editor panel in Campaign Builder
- [ ] Updated item editor with spell scroll support
- [ ] Updated dialogue editor with `LearnSpell` action
- [ ] Updated quest editor with `LearnSpell` reward
- [ ] Validation rules for spell cross-references
- [ ] SDK-level tests for all new editor functionality

#### 6.8 Success Criteria

- Campaign authors can create and edit spells entirely within the SDK
- All spell-related content validates correctly
- Round-trip: create spell in SDK → export RON → load in game → cast spell
- All four quality gates pass

---

### Phase 7: Remediation of Audit Gaps

**Goal**: Close the five concrete gaps identified during the Phase 1–6 audit.
Every gap is a missing integration between an already-implemented domain
function and the game or Bevy layer that should consume it. No new domain
types are introduced; only call-site wiring and integration hooks are added.

**Dependencies**: Phases 1–6 complete. All five tasks in this phase are
independent of each other and may be implemented in any order.

#### 7.1 Wire Exploration Scroll Dispatch (CastSpell / LearnSpell)

**Root cause**: `handle_use_item_action_exploration` in
`src/game/systems/inventory_ui.rs` calls `apply_consumable_effect_exploration`
and obtains a `ConsumableApplyResult`, but never checks the
`result.spell_cast_id` or `result.spell_learn_id` signal fields. The domain
layer already sets these correctly; the Bevy handler just ignores them.

**Fix**: After the existing Step 6 in `handle_use_item_action_exploration`,
add two dispatch blocks:

- **`spell_cast_id`**: If `Some(spell_id)`, look up the spell in
  `content_db.spells`. Call `cast_exploration_spell(party_index, &spell,
ExplorationTarget::Self_, game_state, &mut rng)`. Log success or the
  `SpellError` returned. Use a seeded `SmallRng` from `rand::rng()` in
  contexts where a resource RNG is unavailable, consistent with the pattern
  used in `exploration_spells.rs`.

- **`spell_learn_id`**: If `Some(spell_id)`, call
  `crate::domain::magic::learning::learn_spell(&mut character, spell_id,
spell_db, class_db)`. Log the outcome: spell name on success, the
  `SpellLearnError` variant (wrong class, already known, etc.) on failure.
  Do **not** panic on failure — scroll use is silently skipped for ineligible
  characters, matching the dialogue and quest reward handlers.

Update `build_consumable_use_log` to produce more informative messages when
`spell_cast_id` or `spell_learn_id` is present (e.g., include the resolved
spell name rather than the raw numeric ID).

#### 7.2 Wire Walk on Water to Map Traversal

**Root cause**: `BuffField::WalkOnWater` correctly writes
`active_spells.walk_on_water` when the spell is cast, but the movement
validation code in `src/game/systems/input/exploration_movement.rs` (and any
companion map-tile helper) never reads that field. Water tiles are therefore
impassable regardless of buff state.

**Fix**:

- Locate the tile-type check that blocks movement onto water tiles in
  `exploration_movement.rs` (or the relevant movement validation helper).
- Before returning the "blocked" result, read
  `global_state.0.active_spells.walk_on_water`. If `> 0`, allow traversal.
- Add a named constant `ACTIVE_SPELL_DURATION_THRESHOLD: u8 = 1` (or reuse
  an existing `> 0` guard pattern) so the intent is explicit and consistent
  with the levitate check added in 7.3.
- Do not modify `ActiveSpells` here — duration ticking is already handled by
  `ActiveSpells::tick()`.

#### 7.3 Wire Levitate / Fly to Pit and Chasm Tile Validation

**Root cause**: `BuffField::Levitate` correctly writes
`active_spells.levitate`, but the pit/chasm tile validation in
`exploration_movement.rs` (or the map tile helper) never consults it.
Characters fall into pits regardless of the Levitate buff.

**Fix**:

- Locate the tile-type check that triggers pit or chasm effects in the
  movement validation path.
- Before applying the pit effect, read `global_state.0.active_spells.levitate`.
  If `> 0`, skip the pit effect and allow traversal as if it were a normal
  floor tile.
- Follow the same guard pattern as the Walk on Water fix above for
  consistency.

#### 7.4 Implement Town Portal / Surface Teleport

**Root cause**: `apply_utility_spell` handles `UtilityType::Teleport` by
returning a `UtilityResult` with a generic message and `food_created: 0`, but
never signals _where_ to teleport and never touches `game_state.world_state`.
The Bevy layer (`exploration_spells.rs`) does not follow up on the result.

**Fix — domain layer (`src/domain/magic/types.rs`)**:

- Extend `UtilityType::Teleport` to carry destination intent:

  ```rust
  Teleport { destination: TeleportDestination },
  ```

- Add `TeleportDestination` enum in `types.rs`:

  ```rust
  pub enum TeleportDestination {
      /// Return party to the map's designated entry point
      Surface,
      /// Return party to the registered home inn / town
      TownPortal,
      /// Direct jump to explicit coordinates (Jump spell)
      Jump,
  }
  ```

- Update `apply_utility_spell` to populate a new `teleport_destination:
Option<TeleportDestination>` field on `UtilityResult` instead of only
  returning a string. Add `#[serde(default)]` to the new field.

**Fix — Bevy layer (`src/game/systems/exploration_spells.rs`)**:

- After `cast_exploration_spell` returns, inspect
  `result.utility_result.teleport_destination`:
  - `Surface` → set `game_state.world_state.party_position` to the current
    map's entry tile (the position stored when the map was entered).
  - `TownPortal` → set `party_position` and `current_map` to the party's
    registered home town (stored in `GameState` or `Party`; fall back to
    map 1 / position (0,0) if not yet set).
  - `Jump` → leave position unchanged for now (full Jump targeting requires
    a separate target-selection UI and is deferred as a stretch goal).
- Write a `GameLogEvent` describing the teleport destination.

**Fix — RON data**:

Update the `Town Portal`, `Surface`, and `Jump` spell entries in
`data/spells.ron`, `data/test_campaign/data/spells.ron`, and
`campaigns/tutorial/data/spells.ron` to use the new
`UtilityType::Teleport { destination: TownPortal }` /
`UtilityType::Teleport { destination: Surface }` form. Add `#[serde(default)]`
so existing entries that omit `destination` deserialize without error,
defaulting to `TeleportDestination::Surface`.

#### 7.5 Implement Location Spell Coordinate Display

**Root cause**: `apply_utility_spell` is a pure domain function with no access
to game state, so it cannot embed real map coordinates. The Bevy system does
not post-process the result to inject coordinates.

**Fix — Bevy layer only (`src/game/systems/exploration_spells.rs`)**:

- After `cast_exploration_spell` returns successfully, check
  `spell.effective_effect_type()`. If it resolves to
  `SpellEffectType::Utility { utility_type: UtilityType::Information }`:
  - Read `game_state.world_state.party_position` (x, y) and
    `game_state.world_state.current_map_id`.
  - Overwrite the feedback message with a formatted string such as:
    `"Location: Map {map_id}, ({x}, {y})."`.
- No domain-layer changes are required; the Bevy layer has access to the
  world state that the pure domain function intentionally does not.

#### 7.6 Testing Requirements

- **7.1 — Scroll dispatch**:

  - Test `ConsumableEffect::CastSpell` scroll use in exploration actually casts
    the spell (SP decremented on caster, effect applied to target character).
  - Test `ConsumableEffect::LearnSpell` scroll use adds the spell to the
    correct character's `SpellBook`.
  - Test that using a `LearnSpell` scroll for a wrong-class character logs a
    warning and does not panic.
  - Test that using a `LearnSpell` scroll for an already-known spell does not
    duplicate the entry.
  - All test data from `data/test_campaign/`.

- **7.2 — Walk on Water**:

  - Test that a character without the buff cannot move onto a water tile.
  - Test that a character with `active_spells.walk_on_water > 0` can move onto
    a water tile.

- **7.3 — Levitate**:

  - Test that a character without the buff triggers pit effects.
  - Test that a character with `active_spells.levitate > 0` passes over a pit
    without triggering the effect.

- **7.4 — Teleport**:

  - Test that casting a `TownPortal` spell updates `party_position` and
    `current_map` to the registered home location.
  - Test that casting a `Surface` spell updates `party_position` to the map's
    entry point.
  - Test that existing RON spell data (without explicit `destination`) still
    deserializes without error (serde default).

- **7.5 — Location**:
  - Test that the Location spell feedback message contains the current map ID
    and party coordinates rather than a generic string.
  - All test data from `data/test_campaign/`.

#### 7.7 Deliverables

- [ ] `src/game/systems/inventory_ui.rs` — dispatch `spell_cast_id` and
      `spell_learn_id` after exploration consumable use
- [ ] `src/game/systems/input/exploration_movement.rs` — check
      `active_spells.walk_on_water` before blocking water tile movement
- [ ] `src/game/systems/input/exploration_movement.rs` — check
      `active_spells.levitate` before triggering pit/chasm effects
- [ ] `src/domain/magic/types.rs` — `TeleportDestination` enum and extended
      `UtilityType::Teleport { destination }` variant
- [ ] `src/domain/magic/effect_dispatch.rs` — `UtilityResult` carries
      `teleport_destination: Option<TeleportDestination>`
- [ ] `src/game/systems/exploration_spells.rs` — apply teleport destination
      to `game_state.world_state` and format Location spell message with
      real coordinates
- [ ] Updated `data/spells.ron`, `data/test_campaign/data/spells.ron`, and
      `campaigns/tutorial/data/spells.ron` with `destination` field on teleport
      spells
- [ ] Unit and integration tests for all five gap areas
- [ ] `docs/explanation/implementations.md` updated

#### 7.8 Success Criteria

- Using a `CastSpell` scroll in exploration actually casts the spell through
  the exploration pipeline
- Using a `LearnSpell` scroll in exploration actually adds the spell to the
  character's `SpellBook`
- Characters with Walk on Water buff can traverse water tiles; without it,
  water tiles remain impassable
- Characters with Levitate buff float over pit tiles; without it, pit effects
  apply as normal
- Casting Town Portal or Surface teleports the party to the correct world
  position and updates `game_state.world_state`
- Casting the Location spell displays the party's actual map ID and
  coordinates in the feedback message
- All four quality gates pass with zero warnings

---

## Architecture Compliance Checklist

- [ ] All new types use architecture-defined type aliases (`SpellId`, `CharacterId`, etc.)
- [ ] `AttributePair` / `AttributePair16` pattern used for SP modifications
- [ ] Constants extracted (no magic numbers for durations, costs, thresholds)
- [ ] `ActiveSpells` field writes use the existing struct — no parallel tracking
- [ ] `SpellBook` class routing respects `ClassDatabase` spell school mappings
- [ ] RON format used for all spell data files
- [ ] Game mode context respected (combat vs. exploration casting logic separated)
- [ ] All new public items have `///` doc comments with examples
- [ ] No test references to `campaigns/tutorial` (use `data/test_campaign/`)
- [ ] `docs/explanation/implementations.md` updated after each phase
