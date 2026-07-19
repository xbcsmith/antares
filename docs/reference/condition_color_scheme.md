# Condition Color Scheme Reference

This document is the canonical reference for the condition→colour mapping
used to tint combat and HUD cards, and for the precedence rule that governs
when a card shows the active-turn highlight versus a condition tint versus
its default background. Future features that add a new card, panel, or list
row representing a combatant (party member or monster) should reuse this
scheme rather than introducing an ad-hoc colour.

## Transparency Rule

Every tint in this scheme is a **translucent overlay**, never an opaque,
high-saturation colour: all alpha values are `< 1.0`. This keeps the card's
portrait, name, HP bar, and other content readable underneath the tint, and
distinguishes a background *tint* (a wash of colour behind existing content)
from a foreground *label* colour (condition name text, which may be more
saturated/opaque for legibility).

## Precedence Order

A card's background colour is resolved in this fixed order, evaluated
top-to-bottom — the first rule that applies wins:

1. **Active-turn highlight** — the card belongs to the combatant whose turn
   it currently is (HUD party cards only; see
   [Active-Turn Highlight](#active-turn-highlight) below).
2. **Condition tint** — the card's owner has a non-`Normal` condition.
3. **Default background** — neither of the above applies.

This is encoded once as a pure function,
[`resolve_card_background`](#source-location), so every card kind (HUD party
cards, combat enemy cards, and any future card) applies the same precedence
without re-implementing the logic.

## Condition → Colour Table

| Category       | Colour name                     | `srgba` value                | Alpha | Applies to                                             |
| --------------- | -------------------------------- | ----------------------------- | ----- | ------------------------------------------------------- |
| Fatal           | `CONDITION_FATAL_COLOR`          | `(0.85, 0.2, 0.2, 0.85)`      | 0.85  | Dead (players and monsters)                              |
| Poisoned        | `CONDITION_POISON_TINT_COLOR`    | `(0.2, 0.7, 0.2, 0.75)`       | 0.75  | Poisoned, Diseased (players only — monsters have neither) |
| Unconscious     | `CONDITION_UNCONSCIOUS_TINT_COLOR` | `(0.5, 0.5, 0.5, 0.75)`     | 0.75  | Unconscious (players only — monsters have no unconscious state) |
| Status          | `CONDITION_STATUS_COLOR`         | `(0.9, 0.85, 0.3, 0.85)`      | 0.85  | Generic fallback for every other non-fatal condition: Paralyzed, Asleep, Blinded, Silenced, Held, Webbed, Mindless, Afraid |
| None (no tint)  | — (keeps the card's default background) | —                       | —     | `Normal` condition, or a character with only *positive* active conditions (buffed) |

A monster's richer `MonsterCondition` enum and a character's `Condition`
bitflags each reduce down to one of these categories before the colour is
looked up — see [Source Location](#source-location) for the two mapping
functions.

Note: being merely **buffed** (positive active conditions only, no bad status
flag) does **not** tint the card — a buff is not a warning state, so it is
mapped to `None` (no tint) rather than `Status`.

## Active-Turn Highlight

| Constant                | `srgba` value             | Alpha | Applies to                                            |
| ------------------------ | -------------------------- | ----- | ------------------------------------------------------- |
| `ACTIVE_TURN_CARD_COLOR` | `(0.15, 0.45, 0.15, 0.7)`  | 0.7   | The HUD party card of the member whose turn it currently is, during `CombatTurnState::PlayerTurn` |

The active-turn highlight is presently only wired up for HUD party cards
(4.1); enemy combat cards do not yet have an equivalent "this monster is
acting now" highlight; they currently render condition tint vs. default only.

## Default Backgrounds (non-tinted state)

| Constant               | `srgba` value            | Applies to        |
| ----------------------- | -------------------------- | ------------------ |
| `DEFAULT_CARD_COLOR`    | `(0.2, 0.2, 0.2, 0.7)`     | HUD party cards    |
| `ENEMY_CARD_DEFAULT_COLOR` | `(0.2, 0.15, 0.15, 0.9)` | Combat enemy cards |

Party and enemy cards intentionally use slightly different default shades (a
neutral grey vs. a warm dark red-brown) — this is a pre-existing visual
distinction between "your side" and "the enemy side," unrelated to condition
state, and is preserved rather than unified.

## Source Location

The colour constants, the `CardConditionTint` category enum, and the
`resolve_card_background` precedence function are defined once in
`src/game/systems/ui_helpers.rs` and re-exported / consumed from there:

- `CONDITION_FATAL_COLOR`, `CONDITION_STATUS_COLOR`,
  `CONDITION_POISON_TINT_COLOR`, `CONDITION_UNCONSCIOUS_TINT_COLOR` —
  `src/game/systems/ui_helpers.rs`
- `CardConditionTint` (the shared category enum) — `src/game/systems/ui_helpers.rs`
- `resolve_card_background` (the precedence function) — `src/game/systems/ui_helpers.rs`
- `ACTIVE_TURN_CARD_COLOR`, `DEFAULT_CARD_COLOR` — `src/game/systems/hud.rs`
  (HUD-specific; passed into `resolve_card_background` as a parameter rather
  than hardcoded, since only HUD cards currently define an active-turn state)
- `ENEMY_CARD_DEFAULT_COLOR` — `src/game/systems/combat.rs`
- `condition_tint_category` (maps `Condition` bitflags → `CardConditionTint`) —
  `src/game/systems/hud.rs`, co-located with `get_priority_condition`
- `monster_condition_tint` (maps `MonsterCondition` → `CardConditionTint`) —
  `src/game/systems/combat.rs`, used by `enter_target_selection`

`src/game/systems/combat.rs` re-exports `CONDITION_FATAL_COLOR` and
`CONDITION_STATUS_COLOR` from `ui_helpers` (`pub use ...`) so existing
call sites in that module are unaffected by the constants' canonical home
living in `ui_helpers`.

## See Also

- [Combat Bug Fixes Implementation Plan](../explanation/combat_bug_fixes_implementation_plan.md) — Phase 4 (Combat UX Polish), which introduced this scheme

## Copyright

SPDX-License-Identifier: Apache-2.0

This document follows the [SPDX Spec](https://spdx.github.io/spdx-spec/) for
copyright and licensing information.
