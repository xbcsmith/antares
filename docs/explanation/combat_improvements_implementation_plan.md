# Combat Improvements Implementation Plan (Round 2)

## Overview

Follow-up to `combat_bug_fixes_implementation_plan.md` (complete). This round
adds classic turn-based RPG combat features that improve the play experience:
using items on party members, polished ally spell targeting, clear group-spell
targeting, floating damage numbers, enemy inspection, and font consistency.

Exploration found the **domain layer is already ahead of the UI**:

- `execute_item_use_by_slot` (`src/domain/combat/item_usage.rs:288`) accepts an
  arbitrary party-member target, but the UI (`dispatch_item_button`,
  `src/game/systems/combat.rs:3404-3412`) hardcodes `target = user` for every
  consumable — a Healing Potion or Resurrect item can never be used on the
  downed ally who needs it.
- `SingleCharacter` spells already open a `PartyTargetPanel`
  (`combat.rs:2849-2855`), so the "First Aid can't target another character"
  item (`docs/explanation/next_plans.md:216`) appears fixed but is unverified,
  unchecked, and the panel has **no eligibility filtering** (dead members
  selectable for heals; living members selectable for Raise Dead).
- Group spells (`AllMonsters`/`MonsterGroup`/`SpecificMonsters`) silently emit
  a first-alive-monster placeholder target (`combat.rs:2856-2874`) with no
  visual indication that everything will be hit.
- `apply_consumable_effect` (`src/domain/items/consumable_usage.rs:231`) is
  target-safe except one hole: `HealHp` on a **dead** character raises HP
  without reviving (only `revive_from_unconscious` is called) — a wasted
  potion and a weird state.

## Current State Analysis

### Existing Infrastructure (reuse seams)

- **Party target panel**: `PartyTargetPanelState` (`combat.rs:927`),
  `PartyTargetPanel` (`combat.rs:629`), confirm handler
  `handle_party_target_button` (`combat.rs:3099`) — the ally-selection UI
  both spells and items will share.
- **Item flow**: `ItemPanelState` (`combat.rs:951`), panel spawn
  `update_item_selection_panel` (`combat.rs:3176`) filtered by
  `validate_item_use_slot` (`item_usage.rs:167`), dispatch
  `dispatch_item_button` (`combat.rs:3366`), pending state `PendingItemUse`
  (`combat.rs:979`).
- **Domain applicators**: `apply_consumable_effect`
  (`consumable_usage.rs:231`) and `execute_item_use_by_slot`
  (`item_usage.rs:288`) — no new domain effect code needed.
- **Spell metadata**: `SpellTarget` (`src/domain/magic/types.rs:131-147`),
  `SpellEffectType` + `Spell::infer_effect_type()`
  (`types.rs:329-378`, `types.rs:634`) — drives target-eligibility rules.
- **Visual language**: `update_target_highlight` target-selection colors,
  `ui_helpers.rs` condition-color constants,
  `ACTION_BUTTON_DISABLED_COLOR`, and the precedence rules in
  `docs/reference/condition_color_scheme.md`.

## Implementation Phases

### Phase 1: Use Item on a Party Member

Beneficial consumables prompt for an ally target via the existing party
target panel instead of hardcoded self-targeting.

#### 1.1 Generalize the panel's pending payload

Replace `PartyTargetPanelState.pending_spell: Option<(CombatantId, SpellId)>`
with `pending_action: Option<PartyTargetAction>` where `PartyTargetAction` is
a small enum: `Spell { caster, spell_id }` | `Item { user, inventory_index }`.
Update `dispatch_spell_target` (`combat.rs:2849`),
`handle_party_target_button` (`combat.rs:3099` — the Item arm emits
`UseItemAction { user, inventory_index, target: Player(pidx) }`), the panel
spawn system, keyboard navigation in `combat_input_system`, and
`cleanup_party_target_on_combat_exit` (`combat.rs:3156`).

#### 1.2 Route consumables through the panel

In `dispatch_item_button` (`combat.rs:3366`): keep the
`spell_effect → SingleMonster` monster-targeting path; route all other
consumables (`HealHp`, `RestoreSp`, `CureCondition`, `BoostAttribute`,
`BoostResistance`, `Resurrect`) into the party target panel with
`focused_index` defaulting to the user's own row (self-use stays one Enter
away). Items whose `spell_effect` spell targets `SingleCharacter` (healing
scrolls/wands) also route to the panel. `CastSpell`-effect items with other
targets keep current behavior.

#### 1.3 Shared target-eligibility helper

New pure function (in `combat.rs` near the panel code, or `ui_helpers.rs`):
derive `TargetEligibility { LivingOnly, DeadOnly, Any }` from a
`ConsumableEffect` (and in Phase 2 from a `SpellEffectType`), plus
`is_eligible(&Character) -> bool` using `Condition::is_dead()`.
`Resurrect` → `DeadOnly`; heal/cure/restore/boost → `LivingOnly`.
Unit-testable without Bevy.

#### 1.4 Domain guard (defense in depth)

In `apply_consumable_effect` (`consumable_usage.rs:231`), make `HealHp` a
no-op (zeroed result) on a dead character, mirroring how `Resurrect` already
no-ops on the living. Audit `execute_item_use_by_slot` for the same.

#### 1.5 Testing Requirements

- Eligibility-helper unit tests (per `ConsumableEffect` variant).
- Integration: Healing Potion used on a wounded ally — ally HP rises, item
  consumed from the *user's* inventory, turn advances.
- Integration: Resurrect item — only dead-member rows selectable.
- Regression: existing self-target item tests updated for the panel step.

#### 1.6 Deliverables

- [ ] `PartyTargetAction` enum replacing `pending_spell`
- [ ] Consumables routed through the party target panel
- [ ] Shared `TargetEligibility` helper
- [ ] `HealHp`-on-dead domain guard
- [ ] Unit + integration tests listed in 1.5

#### 1.7 Success Criteria

- A Healing Potion (or any beneficial consumable) can be used on any eligible
  party member from combat; the item is consumed from the user's inventory
  and the user's turn is spent.
- Self-use requires only one extra confirm (user's row pre-focused).
- Healing a dead character is impossible from the UI and a no-op in the
  domain.
- `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` pass (full doctest run excluded per project
  convention).

### Phase 2: Ally Spell Targeting Polish

#### 2.1 Verify end-to-end

Integration tests: First Aid (spell id 260, `SingleCharacter`) cast on a
different party member heals that member; Raise Dead (id 2051,
`Resurrection`) revives a dead member. Confirm dead members appear in the
panel's `participant_indices`.

#### 2.2 Eligibility filtering in the panel

Apply the Phase 1 helper, driven by `spell.effect_type` falling back to
`infer_effect_type()`: `Healing`/`CureCondition`/`Buff` → `LivingOnly`;
`Resurrection` → `DeadOnly`. Ineligible rows render greyed
(`ACTION_BUTTON_DISABLED_COLOR` pattern), mouse clicks on them are ignored,
and keyboard navigation skips them. If no eligible target exists, show a
"No valid target" row and allow only cancel.

#### 2.3 Docs checkoffs

Mark `next_plans.md:216` ✅ COMPLETED; verify the prior round's condition
tinting covers `next_plans.md:214` and mark it ✅ COMPLETED.

#### 2.4 Testing Requirements

- Ineligible-row skip (keyboard) and ignore (mouse) integration tests.
- Eligibility mapping unit tests per `SpellEffectType`.

#### 2.5 Deliverables

- [ ] First Aid / Raise Dead end-to-end integration tests
- [ ] Eligibility filtering (grey rows, keyboard skip, mouse ignore,
      no-valid-target row)
- [ ] next_plans.md checkoffs (216, 214)

#### 2.6 Success Criteria

- First Aid can heal any living party member; Raise Dead can only target
  dead members; ineligible rows are visibly greyed and unselectable.
- `cargo clippy` and `cargo test --workspace` pass.

### Phase 3: Group-Spell Target Clarity

**Decision:** a confirm step, consistent with the single-target
select→confirm flow — no new animation infrastructure. Domain behavior
unchanged (still emits the existing placeholder-target `CastSpellAction` on
confirm).

#### 3.1 Pending state

In `dispatch_spell_target` (`combat.rs:2856-2884`), instead of immediately
casting `AllMonsters`/`MonsterGroup`/`SpecificMonsters`/`AllCharacters`
spells, set a new `GroupTargetPending` state (spell + caster + affected
side).

#### 3.2 Highlight + confirm

While pending, highlight **all** living monster cards (reuse the
target-selection highlight color from `update_target_highlight`) or all
party HUD cards for `AllCharacters`; show a prompt line
("Enter: cast on all enemies — Esc: cancel"). Enter emits the
`CastSpellAction` exactly as today; Esc returns to the spell panel.

#### 3.3 Precedence

Group-highlight uses the same border/tint slot as single-target highlight;
active-turn/condition tint precedence from
`docs/reference/condition_color_scheme.md` is unchanged.

#### 3.4 Testing Requirements

- Integration: selecting an `AllMonsters` spell enters pending state and
  highlights every living monster card; Enter casts (all monsters damaged),
  Esc cancels without SP loss.
- `AllCharacters` equivalent for HUD cards.

#### 3.5 Deliverables

- [ ] `GroupTargetPending` confirm step for group spells
- [ ] All-affected-cards highlight (monsters and party)
- [ ] Prompt line with Enter/Esc handling
- [ ] Integration tests listed in 3.4

#### 3.6 Success Criteria

- Casting a group spell shows exactly which combatants will be affected
  before it resolves; cancel costs nothing.
- `cargo clippy` and `cargo test --workspace` pass.

### Phase 4: Classic QoL Extras

#### 4.1 Floating damage/heal numbers

Spawn short-lived `Text` entities over monster cards / HUD cards when
attack/spell/item results resolve (the result structs already carry
`damage`/`healing` and affected indices). A small system moves them up and
fades alpha over ~1s via a `Timer`, then despawns. Colors from shared
constants: damage red, healing green, misses grey "Miss". Anchor to the
card's UI node position.

#### 4.2 Inspect enemy

While a monster card is focused during target selection (Tab), show an info
strip on the card: AC, current conditions, and special-attack summary from
the monster's data. Key off the existing `active_target_index` focus — no
new input mode.

#### 4.3 SP potions usable in combat

Data fix: `data/items.ron` id 51 "Magic Potion"
`is_combat_usable: false → true` (RestoreSp is a first-class classic combat
action; the restore-SP consumable path already works). Mirror in
`data/test_campaign/data/` if item 51 exists there.

#### 4.4 Testing Requirements

- Floating-text system unit test (spawn → fade → despawn over ticked time).
- Inspect-strip integration test (focused card shows AC/conditions).
- RON round-trip/data test for item 51.

#### 4.5 Deliverables

- [ ] Floating damage/heal/miss numbers over monster and HUD cards
- [ ] Inspect info strip on the focused monster card
- [ ] Magic Potion combat-usable data change
- [ ] Tests listed in 4.4

#### 4.6 Success Criteria

- Damage and healing are visible at the point of impact without reading the
  combat log; a focused enemy reveals AC and conditions; SP can be restored
  by potion mid-combat.
- `cargo clippy` and `cargo test --workspace` pass.

### Phase 5: Font-Size Consistency (next_plans.md:232)

#### 5.1 Audit

Audit `TextFont { font_size }` spawn sites in `src/game/systems/combat.rs`
and `src/game/systems/hud.rs` for mismatched sizes on the same visual line
(known suspects: HUD card name/HP/condition rows, combat card HP+condition
lines, action-button labels vs. hotkey hints).

#### 5.2 Shared constants

Introduce shared font-size constants (e.g. `UI_FONT_SIZE_SM/MD/LG` in
`ui_helpers.rs`, following the existing shared-constant pattern) and replace
ad-hoc literals at the audited sites.

#### 5.3 Docs checkoff

Mark `next_plans.md:232` ✅ COMPLETED.

#### 5.4 Testing Requirements

- Constant sanity test (sizes distinct, ordered).
- Visual verification in-game.

#### 5.5 Deliverables

- [ ] Same-line font-size audit of combat/HUD text
- [ ] Shared `UI_FONT_SIZE_*` constants; literals replaced
- [ ] next_plans.md:232 checkoff

#### 5.6 Success Criteria

- No visual line in combat or HUD mixes font sizes; sizes come from shared
  constants.
- `cargo clippy` and `cargo test --workspace` pass.

## Cross-Cutting Deliverables

- Update `docs/explanation/implementations.md` after each phase (established
  pattern from the bug-fix phases).
- One commit per phase, matching the existing
  `feat: … phase N complete` convention.

## Verification

Per phase and at the end:

- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (full doctest run excluded per project convention)
- Manual run (tutorial campaign): trigger combat; use a Healing Potion on a
  wounded ally and a Resurrect item/Raise Dead on a dead ally; cast First Aid
  on another member; cast an all-enemies spell and observe the group
  highlight + confirm; watch floating numbers; Tab-focus a monster to see
  the inspect strip; drink a Magic Potion in combat; eyeball same-line font
  sizes on HUD and enemy cards.

## Copyright

SPDX-License-Identifier: Apache-2.0

This document follows the [SPDX Spec](https://spdx.github.io/spdx-spec/) for
copyright and licensing information.
