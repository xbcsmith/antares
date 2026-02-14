# Combat System - PENDING IMPLEMENTATION TASKS

## Summary

This section lists the remaining tasks discovered after Phase 1–5 of the Combat System Implementation Plan. Each task includes scope, suggested files to modify, tests to add, rough estimate, priority, and acceptance criteria to make it actionable for planning and assignment.

## Pending Tasks

### 1) Spell casting & item usage flow

- Scope: Implement `CastSpellAction` and `UseItemAction` message types, corresponding handler systems, validation (MP, item availability), UI flows for spell/item selection, deterministic helpers for testing, and integration with the domain (spell effects, item effects).
- Files:
  - `src/game/systems/combat.rs` (add messages, handlers, helper functions, UI hooks)
  - `src/game/components/combat.rs` (add any UI marker components if needed)
  - Possibly small domain helpers if spells/items need new domain APIs
- Tests to add:
  - `test_cast_spell_applies_effect_and_consumes_mp`
  - `test_cast_spell_invalid_mp_no_effect`
  - `test_use_item_applies_effect_and_consumes_item`
- Est. size: Medium
- Priority: High
- Acceptance: Player can cast spells and use items during `PlayerTurn`; MP and inventory counts update correctly; effects apply to targets and are testable deterministically.

### 2) Turn indicator visual (Follow current actor)

- Scope: Spawn/update a `TurnIndicator` UI entity that visually highlights the current actor (enemy card or party card); update its position when `current_turn` changes and when UI layout reflows.
- Files:
  - `src/game/components/combat.rs` (component exists: `TurnIndicator`)
  - `src/game/systems/combat.rs` (add `update_turn_indicator` system; spawn indicator in `setup_combat_ui`)
- Tests to add:
  - `test_turn_indicator_spawns_on_enter`
  - `test_turn_indicator_moves_with_current_actor`
- Est. size: Small
- Priority: Medium
- Acceptance: Single indicator highlights current actor and moves correctly when turns advance.

### 3) Victory summary: per-character breakdown

- Scope: Extend `handle_combat_victory` UI to list per-character XP (`VictorySummary.xp_awarded`) and show collected items individually; improve victory UI clarity.
- Files:
  - `src/game/systems/combat.rs` (update UI construction)
- Tests to add:
  - `test_victory_ui_shows_per_character_xp`
  - `test_victory_ui_shows_items`
- Est. size: Small
- Priority: Medium
- Acceptance: Victory UI displays per-character XP and full loot details; unit testable and covered.

### 4) Combat animations & sequencing (attack animations, hit flash)

- Scope: Integrate sprite/animation infrastructure so attacks trigger animations and hit flashes. Use `CombatTurnState::Animating` to pause logic until animation completion; ensure SFX and animation events are coordinated.
- Files:
  - `src/game/systems/combat.rs` (action handlers should trigger animations)
  - `src/game/systems/animation.rs` (or reuse existing sprite animation systems)
- Tests to add:
  - `test_attack_triggers_animating_state`
  - `test_animation_completion_resumes_combat`
- Est. size: Large
- Priority: Low–Medium (polish, but important for UX)
- Acceptance: Actions trigger animations; combat state waits for animations to finish before advancing.

### 5) Consolidate target selection API (resource vs component)

- Scope: Decide and implement a single consistent target-selection API. Either:
  - Use `TargetSelection` resource (current implementation), or
  - Use `TargetSelector` component (declared in components) driven by UI entities.
  - Remove duplication and add tests for the chosen approach.
- Files:
  - `src/game/components/combat.rs`
  - `src/game/systems/combat.rs`
- Tests to add:
  - `test_target_selector_component_flow` (if component route chosen)
- Est. size: Small
- Priority: Low–Medium
- Acceptance: One coherent API used by UI and systems; tests cover expected flows.

### 6) Integration test: random encounter from movement

- Scope: Add integration test to assert `GameState::move_party_and_handle_events` triggers combat when the map's `encounter_table` ensures an encounter (100% encounter rate).
- Files:
  - `tests/combat_integration.rs` or similar
- Tests to add:
  - `test_move_party_triggers_random_encounter`
- Est. size: Small
- Priority: Medium
- Acceptance: Movement on a map configured to guarantee encounters causes `GameMode` to switch to `Combat`.

### 7) Events: implement trap & treasure handlers

- Scope: Implement `MapEvent::Trap` to apply damage/effects to the party and `MapEvent::Treasure` to add items (respecting inventory capacity/rules) and log messages.
- Files:
  - `src/game/systems/events.rs`
  - Domain inventory APIs as needed
- Tests to add:
  - `test_trap_applies_damage_to_party`
  - `test_treasure_adds_items_to_inventory_or_stashes`
- Est. size: Small
- Priority: Medium
- Acceptance: Trap damage affects party HP/conditions; treasure deposits items to party inventories (or stashes if full).

---
