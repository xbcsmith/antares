# Input Refactor Plan

## Overview

This document outlines a phased refactor plan for `src/game/systems/input.rs`.

The current file has grown into a large, mixed-responsibility module that combines:

- input decoding
- mode toggles
- mode guards
- exploration interaction
- exploration movement
- world-click fallback logic
- victory-overlay cleanup
- a large volume of in-file tests

The immediate goal is to break the file into smaller, more maintainable pieces while preserving current gameplay behavior exactly. A second goal is to reduce the size and complexity of the Bevy-facing input systems so they are easier to reason about, easier to test, and less likely to hit system-configuration problems.

This plan favors incremental extraction over a large rewrite. Each phase should keep the project compiling and keep behavior stable.

---

## Refactor Goals

1. Reduce the size and complexity of `handle_input`.
2. Separate pure helpers from Bevy-specific systems.
3. Split exploration interaction from exploration movement.
4. Isolate the Phase 6 mouse-to-interact fallback into a small, explicit unit.
5. Move tests next to the logic they validate where practical.
6. Keep keyboard and mouse behavior identical to current semantics.
7. Preserve architecture expectations around `GameMode`, `World`, and party-level state.
8. End with smaller, focused input systems registered in a clear order.

---

## Current Problems

### Mixed responsibilities

`handle_input` currently performs too many jobs in one place:

- menu toggle behavior
- automap toggle behavior
- inventory toggle behavior
- rest toggle behavior
- movement cooldown checks
- mode blocking rules
- exploration interact behavior
- exploration movement behavior
- dialogue cancellation on movement
- victory overlay cleanup after movement

This makes the function difficult to change safely.

### Hard-to-follow control flow

The function contains many early returns and mode-specific branches. That is not inherently wrong, but the combination of many concerns in one system makes the real execution order difficult to understand.

### Test sprawl

`input.rs` contains many tests that cover different conceptual areas:

- key parsing
- keymap generation
- menu state toggling
- dialogue inventory behavior
- exploration click fallback
- movement guards
- door interaction
- locked container interaction
- locked door interaction

These are valuable tests, but they make the file larger and harder to navigate.

### Large Bevy system signature

The current input system depends on many resources, queries, and messages. Even when the logic is correct, a very large system signature is harder to maintain and debug.

---

## Guiding Principles

### Preserve behavior first

Refactors should not change gameplay behavior unless explicitly intended. The keyboard path remains the canonical behavior model; mouse support should continue to route through the same semantics.

### Extract pure logic before splitting systems

Where possible, move pure or mostly pure helpers out first. That reduces risk and makes later system splitting much easier.

### Split by responsibility, not by line count

New modules should correspond to clear responsibilities:

- key mapping
- mode toggles
- mode guards
- exploration movement
- exploration interaction
- mouse world-click fallback

### Keep tests close to the code they validate

Pure-helper tests should move with helpers. High-level regression tests should stay in integration tests.

### Maintain deterministic order

When splitting one large system into several smaller systems, preserve the current order of operations explicitly.

---

## Target End State

A likely end-state structure is:

```text
src/game/systems/input/
├── mod.rs
├── keymap.rs
├── menu_toggle.rs
├── mode_guards.rs
├── world_click.rs
├── exploration_movement.rs
├── exploration_interact.rs
└── helpers.rs
```

## Concrete Implementation Checklist

This section converts the phased refactor into an execution checklist with exact
file creation order, function moves, and test-module moves.

### File creation order

Create files in this exact order:

1. `src/game/systems/input/keymap.rs`
2. `src/game/systems/input/helpers.rs`
3. `src/game/systems/input/menu_toggle.rs`
4. `src/game/systems/input/mode_guards.rs`
5. `src/game/systems/input/world_click.rs`
6. `src/game/systems/input/exploration_interact.rs`
7. `src/game/systems/input/exploration_movement.rs`
8. Convert `src/game/systems/input.rs` into `src/game/systems/input/mod.rs`
   once the extracted modules compile cleanly.

### Phase-by-phase checklist

#### Phase 1 checklist — pure helpers first

**Create files**

- [ ] `src/game/systems/input/keymap.rs`
- [ ] `src/game/systems/input/helpers.rs`
- [ ] `src/game/systems/input/menu_toggle.rs`

**Move functions and types**

- [ ] Move `GameAction` to `keymap.rs`
- [ ] Move `KeyMap` to `keymap.rs`
- [ ] Move `parse_key_code` to `keymap.rs`
- [ ] Move `toggle_menu_state` to `menu_toggle.rs`
- [ ] Move `get_adjacent_positions` to `helpers.rs`

**Update call sites**

- [ ] Update `InputPlugin` and `handle_input` imports to use the extracted modules
- [ ] Keep `handle_input` behavior unchanged

**Move test modules**

- [ ] Move `adjacent_tile_tests` with `get_adjacent_positions` to `helpers.rs`
- [ ] Move key parsing tests from `mod tests` to `keymap.rs`
- [ ] Move keymap tests from `mod tests` to `keymap.rs`
- [ ] Move menu toggle tests from `mod tests` to `menu_toggle.rs`

**Definition of done**

- [ ] `input.rs` still owns `InputPlugin`, `InputConfigResource`, and `handle_input`
- [ ] Pure helper tests live beside their helpers
- [ ] All quality gates pass

#### Phase 2 checklist — introduce decoded frame input

**Create files**

- [ ] `src/game/systems/input/mode_guards.rs`
- [ ] `src/game/systems/input/world_click.rs` (file may be mostly scaffolding in this phase)

**Add new types and helpers**

- [ ] Add `FrameInputIntent` to `helpers.rs` or `mod.rs`
- [ ] Add `decode_frame_input(...) -> FrameInputIntent`
- [ ] Add a helper for computing movement attempts from decoded intent
- [ ] Add a helper for computing interaction attempts from decoded intent

**Move functions and logic**

- [ ] Keep `handle_input` in place, but replace repeated key-map polling with `FrameInputIntent`
- [ ] Move center-screen mouse fallback calculation into `world_click.rs`
- [ ] Add `mouse_center_interact_pressed(...) -> bool` in `world_click.rs`

**Move test modules**

- [ ] Keep existing integration tests in `input.rs` temporarily
- [ ] Add unit tests for `decode_frame_input`
- [ ] Add unit tests for `mouse_center_interact_pressed`

**Definition of done**

- [ ] `handle_input` uses decoded intent rather than repeated raw key checks
- [ ] Mouse-center fallback logic is isolated
- [ ] All quality gates pass

#### Phase 3 checklist — global toggles

**Files touched**

- [ ] `src/game/systems/input/menu_toggle.rs`
- [ ] `src/game/systems/input/mod.rs` or `input.rs`

**Add helpers**

- [ ] Add `handle_global_mode_toggles(...) -> bool`
- [ ] Add narrow helpers if needed:
  - [ ] `handle_automap_toggle(...)`
  - [ ] `handle_menu_toggle(...)`
  - [ ] `handle_inventory_toggle(...)`
  - [ ] `handle_rest_toggle(...)`

**Move logic out of `handle_input`**

- [ ] Move automap toggle logic
- [ ] Move menu toggle logic
- [ ] Move inventory toggle logic
- [ ] Move rest toggle logic

**Move test modules**

- [ ] Keep `dialogue_inventory_tests` together, but move them under `menu_toggle.rs` only if the inventory-toggle logic now lives there
- [ ] Move these tests from `integration_tests` into a new `global_toggle_tests` module:
  - [ ] `test_escape_opens_and_closes_menu_via_button_input`
  - [ ] `test_escape_opens_after_movement`
  - [ ] `test_escape_opens_when_move_and_menu_pressed_simultaneously`
  - [ ] `test_handle_input_i_opens_inventory`
  - [ ] `test_gamemode_automap_toggle`
  - [ ] `test_gamemode_automap_escape_closes`
  - [ ] `test_handle_input_i_closes_inventory`
  - [ ] `test_handle_input_i_ignored_in_menu_mode`
  - [ ] `test_handle_input_r_in_exploration_fires_initiate_rest_event`
  - [ ] `test_handle_input_r_ignored_in_menu_mode`
  - [ ] `test_handle_input_r_ignored_in_inventory_mode`
  - [ ] `test_handle_input_r_ignored_in_combat_mode`
  - [ ] `test_handle_input_r_in_exploration_two_frames_two_events`

**Definition of done**

- [ ] `handle_input` delegates global toggles to one helper
- [ ] Global toggle tests are grouped together
- [ ] All quality gates pass

#### Phase 4 checklist — mode guards

**Files touched**

- [ ] `src/game/systems/input/mode_guards.rs`

**Add helpers**

- [ ] Add `input_blocked_for_mode(...)`
- [ ] Or split into:
  - [ ] `movement_blocked_for_mode(...)`
  - [ ] `interaction_blocked_for_mode(...)`

**Move logic out of `handle_input`**

- [ ] Move menu/inventory/automap blocking rules
- [ ] Move combat blocking rules
- [ ] Move rest-mode blocking rules
- [ ] Make dialogue-specific restrictions explicit

**Move test modules**

- [ ] Move `inventory_guard_tests` to `mode_guards.rs`
- [ ] Move `combat_guard_tests` to `mode_guards.rs`

**Definition of done**

- [ ] Mode-blocking rules are centralized
- [ ] `handle_input` uses explicit guard helpers
- [ ] All quality gates pass

#### Phase 5 checklist — exploration interaction extraction

**Create files**

- [ ] `src/game/systems/input/exploration_interact.rs`

**Add top-level helper**

- [ ] Add `handle_exploration_interact(...) -> bool`

**Add internal helpers**

- [ ] Add `try_interact_furniture_door(...)`
- [ ] Add `try_interact_locked_door_event(...)`
- [ ] Add `try_interact_locked_container_event(...)`
- [ ] Add `try_interact_adjacent_world_events(...)`
- [ ] Add `try_interact_npc_or_recruitable(...)`

**Move logic out of `handle_input`**

- [ ] Move furniture door interaction logic
- [ ] Move locked door map-event logic
- [ ] Move locked container map-event logic
- [ ] Move adjacent NPC dialogue logic
- [ ] Move recruitable character interaction logic
- [ ] Move sign interaction logic
- [ ] Move teleport interaction logic
- [ ] Move encounter interaction logic
- [ ] Move any tile-ahead interaction routing now owned by the helper

**Move test modules**

- [ ] Move `interaction_tests` to `exploration_interact.rs`
- [ ] Move `door_interaction_tests` to `exploration_interact.rs`
- [ ] Move `locked_container_map_event_tests` to `exploration_interact.rs`
- [ ] Move `locked_door_map_event_tests` to `exploration_interact.rs`
- [ ] Move these world-click interaction tests alongside interaction logic or to `world_click.rs` depending on final ownership:
  - [ ] `test_world_click_npc_triggers_dialogue`
  - [ ] `test_world_click_blocked_outside_exploration_mode`

**Definition of done**

- [ ] Exploration interaction no longer lives inline in `handle_input`
- [ ] Interaction tests live beside interaction code
- [ ] All quality gates pass

#### Phase 6 checklist — exploration movement extraction

**Create files**

- [ ] `src/game/systems/input/exploration_movement.rs`

**Add helpers**

- [ ] Add `is_movement_attempt(...)`
- [ ] Add `movement_blocked_by_cooldown(...)`
- [ ] Add `handle_exploration_movement(...) -> bool`
- [ ] Add `cleanup_victory_overlay_on_movement(...)` if cleanup still belongs with movement

**Move logic out of `handle_input`**

- [ ] Move forward movement logic
- [ ] Move backward movement logic
- [ ] Move turn-left logic
- [ ] Move turn-right logic
- [ ] Move movement cooldown logic
- [ ] Move last-move-time update logic
- [ ] Move movement-time dialogue cancellation logic
- [ ] Move victory overlay cleanup on movement

**Move test modules**

- [ ] Move movement/cooldown tests out of `integration_tests` if they become helper-local
- [ ] Keep or move:
  - [ ] `test_victory_overlay_dismissed_after_party_moves`
  - [ ] `test_locked_furniture_door_blocks_forward_movement`
  - [ ] `test_open_furniture_door_allows_forward_movement`

**Definition of done**

- [ ] Movement behavior is isolated from interaction behavior
- [ ] `handle_input` only orchestrates
- [ ] All quality gates pass

#### Phase 7 checklist — split the Bevy systems

**Files touched**

- [ ] `src/game/systems/input/mod.rs`

**Convert orchestration into multiple systems**

- [ ] Add `handle_global_input_toggles`
- [ ] Add `handle_exploration_input_interact`
- [ ] Add `handle_exploration_input_movement`
- [ ] Add `cleanup_victory_overlay_after_movement` if separate

**Plugin registration work**

- [ ] Register smaller systems in deterministic order
- [ ] Use chaining or explicit ordering where required
- [ ] Confirm interaction still runs before movement if that is current behavior
- [ ] Confirm cleanup still happens after movement

**Move test modules**

- [ ] Keep system-local tests in their owning modules
- [ ] Keep cross-system regression coverage in `tests/mouse_input_regression.rs`

**Definition of done**

- [ ] No monolithic `handle_input` remains as a god-system
- [ ] Bevy-facing system signatures are smaller and manageable
- [ ] All quality gates pass

#### Phase 8 checklist — final test and documentation cleanup

**Files touched**

- [ ] `src/game/systems/input/*.rs`
- [ ] `tests/mouse_input_regression.rs`
- [ ] `docs/explanation/implementations.md`

**Cleanup work**

- [ ] Remove redundant in-file tests that duplicate integration coverage
- [ ] Keep unit tests next to helper modules
- [ ] Ensure comments refer to current module structure
- [ ] Ensure module docs are present where needed
- [ ] Update `docs/explanation/implementations.md` with refactor summary

**Final verification**

- [ ] `cargo fmt --all`
- [ ] `cargo check --all-targets --all-features`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo nextest run --all-features`

### Migration table

| Phase | File created / touched                      | Functions moved                                                                         | Test modules moved                                                                                                                              |
| ----- | ------------------------------------------- | --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| 1     | `keymap.rs`, `helpers.rs`, `menu_toggle.rs` | `GameAction`, `KeyMap`, `parse_key_code`, `toggle_menu_state`, `get_adjacent_positions` | `adjacent_tile_tests`, keymap tests, parse tests, menu toggle tests                                                                             |
| 2     | `world_click.rs`, `helpers.rs` or `mod.rs`  | `FrameInputIntent`, `decode_frame_input`, mouse-center helper                           | new unit tests for decoded input and center-click logic                                                                                         |
| 3     | `menu_toggle.rs`, `mod.rs`                  | global toggle helpers                                                                   | dialogue inventory tests if colocated, plus menu/inventory/rest/automap integration tests regrouped                                             |
| 4     | `mode_guards.rs`                            | mode guard helpers                                                                      | `inventory_guard_tests`, `combat_guard_tests`                                                                                                   |
| 5     | `exploration_interact.rs`                   | all exploration interact helpers                                                        | `interaction_tests`, `door_interaction_tests`, `locked_container_map_event_tests`, `locked_door_map_event_tests`, world-click interaction tests |
| 6     | `exploration_movement.rs`                   | movement helpers and cleanup helpers                                                    | movement/cooldown and move-triggered cleanup tests                                                                                              |
| 7     | `mod.rs`                                    | split orchestration into multiple systems                                               | retain only focused module tests plus integration regressions                                                                                   |
| 8     | all input modules + docs                    | no major logic moves, cleanup only                                                      | deduplicate tests and finalize docs                                                                                                             |

A likely end-state structure is:

```text
src/game/systems/input/
├── mod.rs
├── keymap.rs
├── menu_toggle.rs
├── mode_guards.rs
├── world_click.rs
├── exploration_movement.rs
├── exploration_interact.rs
└── helpers.rs
```

### Planned ownership

#### `mod.rs`

- `InputPlugin`
- `InputConfigResource`
- orchestration and system registration
- thin façade systems only

#### `keymap.rs`

- `GameAction`
- `KeyMap`
- `parse_key_code`

#### `menu_toggle.rs`

- menu toggle helper
- automap / inventory / rest / menu global toggle helpers

#### `mode_guards.rs`

- helper rules for when movement and interaction are blocked by current mode

#### `world_click.rs`

- center-screen exploration click heuristic
- mouse fallback helper logic for Phase 6

#### `exploration_movement.rs`

- movement cooldown helpers
- turning / stepping behavior
- post-move cleanup hooks

#### `exploration_interact.rs`

- furniture door interaction
- locked door interaction
- locked container interaction
- adjacent NPC / event interaction
- exploration click-to-interact routing

#### `helpers.rs`

- small shared utility helpers such as adjacent-position calculations

---

## Phased Plan

# Phase 1: Low-risk extraction of pure helpers

## Goal

Reduce file size without changing Bevy system behavior.

## Work

Move these items out first:

- `GameAction`
- `KeyMap`
- `parse_key_code`
- `toggle_menu_state`
- `get_adjacent_positions`

## Recommended destination

- `keymap.rs`
  - `GameAction`
  - `KeyMap`
  - `parse_key_code`
- `menu_toggle.rs`
  - `toggle_menu_state`
- `helpers.rs`
  - `get_adjacent_positions`

## Tests to move in this phase

Move the tests that directly validate these helpers:

- adjacent tile tests
- parse-key-code tests
- keymap tests
- menu toggle tests

## Expected benefit

This immediately reduces the size of `input.rs` and makes the core input system easier to scan.

---

# Phase 2: Introduce an input-intent layer

## Goal

Separate raw button polling from behavior execution.

## Work

Add a small per-frame intent type, for example:

- `FrameInputIntent`

Potential fields:

- `menu_toggle`
- `inventory_toggle`
- `automap_toggle`
- `rest`
- `move_forward`
- `move_back`
- `turn_left`
- `turn_right`
- `interact`
- `mouse_center_interact`

Add a helper that computes this once per frame from the active button resources and current window state.

## Suggested helper

- `decode_frame_input(...) -> FrameInputIntent`

## Expected benefit

The Bevy system can stop repeating key-map checks inline and instead operate on a small decoded intent object. This also makes behavior order more obvious.

---

# Phase 3: Extract global toggle handling

## Goal

Separate non-exploration global mode toggles from exploration behavior.

## Work

Create a helper responsible only for:

- menu toggle
- automap toggle
- inventory toggle
- rest toggle

Suggested helper:

- `handle_global_mode_toggles(...) -> bool`

This should return whether the frame was consumed.

## Tests to group with this phase

Move or regroup tests related to:

- Escape opening/closing menu
- automap open/close
- inventory open/close
- inventory ignored in unsupported modes
- rest key behavior across modes

## Expected benefit

This removes a large amount of top-of-function branching from the main input path.

---

# Phase 4: Extract mode blocking rules

## Goal

Make input blocking explicit and reusable.

## Work

Move the current mode-guard logic into helpers such as:

- `movement_blocked_for_mode(...)`
- `interaction_blocked_for_mode(...)`
- or a combined `input_blocked_for_mode(...)`

This phase should centralize the rules for:

- menu
- inventory
- combat
- automap
- rest-related modes
- dialogue-specific restrictions

## Tests to group with this phase

Move or regroup:

- inventory guard tests
- combat guard tests
- any tests proving movement or interaction is blocked in specific modes

## Expected benefit

This makes the early-return rules easier to reason about and reduces duplication in any later split systems.

---

# Phase 5: Extract exploration world-click fallback

## Goal

Isolate the Phase 6 exploration mouse heuristic so it is small, explicit, and easy to swap later.

## Work

Move the center-screen mouse fallback logic into a helper module.

Suggested helper:

- `mouse_center_interact_pressed(...) -> bool`

This helper should own:

- left-click detection
- primary-window lookup
- cursor-position validation
- center-third heuristic

## Recommended destination

- `world_click.rs`

## Tests to group with this phase

Move or regroup:

- exploration click-to-interact tests
- world click blocked outside exploration mode

## Expected benefit

This gives the project a clean seam for upgrading to mesh picking later without reopening the entire input system.

---

# Phase 6: Extract exploration interaction

## Goal

Move all exploration `Interact` behavior into one focused module.

## Work

Create `exploration_interact.rs` and move the current exploration interaction flow there.

Suggested top-level helper:

- `handle_exploration_interact(...) -> bool`

### Internal helpers to create

- `try_interact_furniture_door(...)`
- `try_interact_locked_door_event(...)`
- `try_interact_locked_container_event(...)`
- `try_interact_adjacent_world_events(...)`
- `try_interact_npc_or_recruitable(...)`

## Behavior this module should own

- furniture doors
- tile-based locked doors
- tile-based locked containers
- adjacent NPC dialogue
- recruitable character dialogue / events
- signs
- teleports
- encounters
- any tile-ahead or adjacent interaction checks currently in `handle_input`

## Tests to move in this phase

Move or regroup:

- `interaction_tests`
- `door_interaction_tests`
- `locked_container_map_event_tests`
- `locked_door_map_event_tests`
- world-click interaction tests

## Expected benefit

This phase should remove the largest and most complex chunk of logic from the monolithic input system.

---

# Phase 7: Extract exploration movement

## Goal

Move exploration movement and turn handling into a focused module.

## Work

Create `exploration_movement.rs`.

Suggested helpers:

- `is_movement_attempt(...)`
- `movement_blocked_by_cooldown(...)`
- `handle_exploration_movement(...) -> bool`

This module should own:

- forward movement
- backward movement
- turning left/right
- cooldown gating
- movement-time updates
- move-triggered dialogue cancellation if that remains part of movement semantics
- movement-triggered victory overlay cleanup if still coupled to movement

## Tests to move in this phase

Move or regroup:

- movement/cooldown tests
- victory overlay dismissal test
- door-blocked movement tests
- open-door movement tests

## Expected benefit

Movement logic becomes independent from interaction logic, which makes both simpler.

---

# Phase 8: Split the monolithic Bevy system into multiple systems

## Goal

Replace one very large Bevy system with several smaller systems that each have focused responsibilities and smaller parameter lists.

## Proposed systems

A likely split is:

1. `handle_global_input_toggles`
2. `handle_exploration_input_interact`
3. `handle_exploration_input_movement`
4. `cleanup_victory_overlay_after_movement`

Depending on implementation details, the cleanup piece may remain part of movement if that is still the clearest representation.

## Ordering requirements

Preserve the current behavior order explicitly. Recommended order:

1. global toggles first
2. interaction before movement if that matches current semantics
3. movement after interaction
4. cleanup after movement

Use explicit scheduling order rather than relying on insertion order.

## Expected benefit

This is the phase most likely to eliminate current system-configuration pain while making the code much easier to understand.

---

# Phase 9: Restructure tests

## Goal

Move the file from “giant implementation plus giant test appendix” to a cleaner model.

## Work

### Keep module-local unit tests

Tests that validate:

- pure helper behavior
- small local transformations
- focused interaction helper logic

should live in the same module as the code.

### Keep high-level integration tests separate

Cross-mode or cross-system behavior should live in integration tests such as:

- `tests/mouse_input_regression.rs`

### Reduce duplication

Where a module-local test and an integration test assert the same thing at the same level, keep the cheaper and more focused version.

## Expected benefit

This reduces noise in the implementation file while preserving coverage.

---

# Phase 10: Final cleanup and documentation pass

## Goal

Polish structure and make future maintenance easier.

## Work

- rename helpers for clarity
- remove stale comments referring to old structure
- ensure new modules have proper `///` docs where needed
- ensure `mouse_input.rs` documentation still reflects the final canonical model
- update `docs/explanation/implementations.md` after the refactor is complete

## Expected benefit

The final structure becomes easier for future contributors to understand and extend.

---

## Recommended implementation order

### Stage A: Safe extraction

1. create `src/game/systems/input/keymap.rs`
2. move `GameAction`, `KeyMap`, `parse_key_code`
3. create `src/game/systems/input/helpers.rs`
4. move `get_adjacent_positions`
5. create `src/game/systems/input/menu_toggle.rs`
6. move `toggle_menu_state`

### Stage B: Execution seams

7. create `src/game/systems/input/mode_guards.rs`
8. create `src/game/systems/input/world_click.rs`
9. add `FrameInputIntent`
10. add `decode_frame_input`
11. add `handle_global_mode_toggles`
12. add mode guard helpers

### Stage C: Exploration decomposition

13. create `src/game/systems/input/exploration_interact.rs`
14. move exploration interaction helpers and tests
15. create `src/game/systems/input/exploration_movement.rs`
16. move exploration movement helpers and tests

### Stage D: System split

17. convert `src/game/systems/input.rs` into `src/game/systems/input/mod.rs`
18. register multiple smaller systems in a defined order
19. remove the monolithic orchestration path

### Stage E: Test reorganization

20. move remaining module-local tests beside their owning modules
21. retain integration regressions in `tests/mouse_input_regression.rs`
22. remove redundant in-file test duplication
23. update implementation documentation

---

## Risks and mitigations

### Risk: behavior drift

Small ordering changes can alter gameplay semantics.

**Mitigation**:

- preserve existing control-flow order during extraction
- run full quality gates after every phase
- keep regression tests for representative input paths

### Risk: hidden coupling between movement and interaction

Some movement and interaction rules may depend on shared state in non-obvious ways.

**Mitigation**:

- extract into helpers first while still called by one system
- only split into multiple Bevy systems after behavior has been stabilized in helper form

### Risk: test breakage during module moves

Moving tests too early can make refactor work noisy.

**Mitigation**:

- move tests in batches immediately after the code they validate is extracted
- avoid moving unrelated tests in the same step

---

## Quality-gate workflow for each phase

Each phase should end with:

1. `cargo fmt --all`
2. `cargo check --all-targets --all-features`
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `cargo nextest run --all-features`

No phase should be considered complete until the quality gates pass.

---

## Success Criteria

The refactor is complete when:

- `input.rs` is an orchestration module rather than a god file
- no single input system owns all input responsibilities
- exploration interaction and movement are separated
- mouse world-click fallback is isolated behind a clear helper
- key mapping and parsing are separated from gameplay execution
- tests are grouped by responsibility and easier to navigate
- all quality gates are green

---

## Phase Exit Checklist

Each phase should only be considered complete when all of the following are true:

- [ ] the planned file creation or file move for the phase is complete
- [ ] the listed functions for the phase have been moved
- [ ] the listed test modules for the phase have been moved or regrouped
- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo nextest run --all-features` passes

## Summary

This refactor should be done incrementally. The best path is:

- extract pure helpers first
- introduce a decoded-input seam
- separate global toggles from exploration behavior
- split exploration interaction from exploration movement
- only then split one huge Bevy system into several smaller systems

That approach minimizes risk, preserves current behavior, and produces a code layout that is much easier to test and extend.
