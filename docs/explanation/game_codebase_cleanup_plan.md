# Game Codebase Cleanup Plan

## Overview

This plan addresses technical debt, dead code, inconsistent patterns, and
development-phase artifacts accumulated across the Antares game engine codebase
(`src/`). The analysis identified **~130+ phase references**, **10 backup
files**, **34 dead-code suppressions**, **78 too-many-arguments suppressions**,
**10 too-many-lines suppressions**, **14 type-complexity suppressions**, **11
field-reassign-with-default suppressions**, **~58 `#[allow(deprecated)]`
suppressions** (37 in live code + 21 in `.bak` files), **1 `unused_mut`
suppression**, **2 `only_used_in_recursion` suppressions**, **~50 silent error
drops in production code**, **24 TODO/placeholder stubs**, and **~1,200 lines
of duplicate code** that can be consolidated. The cleanup is organized into six
phases, ordered by risk (lowest first) and impact (highest first). Phase 6
collects residual deliverables from Phases 1–4 that were missed or left
incomplete during their original implementation.

We do not care about backwards compatibility.

## Current State Analysis

### Existing Infrastructure

- **Error handling**: 34 modules use `thiserror` consistently; zero `anyhow`
  usage; zero `todo!()`/`unimplemented!()` macros remaining.
- **Testing**: Extensive test suites exist across all modules; no `#[ignore]`
  tests found.
- **Code quality gates**: `cargo fmt`, `cargo check`, `cargo clippy`, and
  `cargo nextest run` are enforced per `AGENTS.md`.

### Identified Issues

1. **Backup files**: 10 `.bak` files checked into `src/` serve no purpose.
2. **Dead code**: 33 `#[allow(dead_code)]` suppressions, including 22 unused
   constants in `procedural_meshes.rs` and an entirely unused `CacheEntry<T>`
   subsystem in `sdk/cache.rs`.
3. **Duplicate code**: ~1,200+ reducible lines across database loaders (16
   copies), CLI editors (3 copies), inventory UI constants (3 copies), and test
   character factories (12+ copies).
4. **Silent error drops**: ~50 `let _ =` calls in production code discard
   `Result` values from game-state-modifying operations (quest rewards, combat
   loot, spell conditions, damage application).
5. **Phase references**: 130+ references to development phases in comments,
   test data IDs, test function names, deprecation attributes, and data files.
6. **Suppressed clippy warnings**: 78 `too_many_arguments`, 10
   `too_many_lines`, 14 `type_complexity`, 11
   `field_reassign_with_default` (all in `world/types.rs` tests), 2
   `only_used_in_recursion` (including `dialogue.rs` `evaluate_conditions`),
   and 1 `unused_mut` (on `dialogue.rs` `execute_action`) — indicating
   functions and tests that need structural refactoring or pattern fixes.
7. **Incomplete deprecation migration**: The `food` field on `Character` and
   `Party` is deprecated but still present, causing a ripple of ~58
   `#[allow(deprecated)]` suppressions (37 in live code + 21 in `.bak`
   files that will be deleted in Phase 1.1).
8. **Placeholder stubs**: 24 TODO/FIXME items including non-functional traps,
   non-functional treasure loot, stub recruitment dialogue actions, and a
   logging-only audio system.
9. **Inconsistent error types**: ~20 functions return `Result<(), String>`
   instead of typed `thiserror` errors.

## Implementation Phases

### Phase 1: Remove Dead Weight (Low Risk, High Visibility)

Delete files and code that serve no purpose. No behavioral changes; all
removals are provably unreachable.

#### 1.1 Delete Backup Files

Remove all 10 `.bak` files and add `*.bak` to `.gitignore`:

| File                          | Location             |
| ----------------------------- | -------------------- |
| `transactions.rs.bak`         | `src/domain/`        |
| `item_usage.rs.bak`           | `src/domain/combat/` |
| `database.rs.bak`             | `src/domain/items/`  |
| `equipment_validation.rs.bak` | `src/domain/items/`  |
| `types.rs.bak`                | `src/domain/items/`  |
| `combat.rs.bak`               | `src/game/systems/`  |
| `creature_meshes.rs.bak`      | `src/game/systems/`  |
| `dialogue.rs.bak`             | `src/game/systems/`  |
| `creature_validation.rs.bak`  | `src/sdk/`           |
| `templates.rs.bak`            | `src/sdk/`           |

#### 1.2 Remove Dead Code Behind `#[allow(dead_code)]`

1. **`src/sdk/cache.rs`** — Remove `CacheEntry<T>` struct and its 3 methods
   (L93–120), `compute_file_hash` (L228), and `preload_common_content` (L341–
   348). The entire caching subsystem is scaffolding that was never wired up.
2. **`src/domain/campaign_loader.rs`** — Remove `content_cache` field (L180)
   and `load_with_override()` method (L399). Planned feature never completed.
3. **`src/domain/world/types.rs`** — Remove `DEFAULT_RECRUITMENT_DIALOGUE_ID`
   constant (L2352). Defined but never referenced.
4. **`src/game/systems/procedural_meshes.rs`** — Remove 22 unused
   dimension/color constants (L770–902, L5319–5322). The comment at L832
   confirms these were inlined into spawn functions; the originals are
   vestigial.
5. **`src/game/systems/hud.rs`** — Remove `colors_approx_equal` test helper
   (L2553). Defined but never called by any test.

#### 1.3 Complete the Deprecated `food` Field Migration

The `food` field on `Character` (L1181) and `Party` (L1547) is deprecated but
still present, causing ~58 `#[allow(deprecated)]` suppressions across
constructors, templates, tests, and `.bak` files (21 of which are eliminated
by deleting the `.bak` files in Phase 1.1).

1. Remove the `food` field from `Character` and `Party` structs entirely.
2. Remove the `#[deprecated]` attribute and all `#[allow(deprecated)]` blocks
   in `src/domain/character.rs`, `src/domain/character_definition.rs`,
   `src/application/mod.rs`, `src/bin/item_editor.rs`,
   `src/domain/items/types.rs`, and `src/domain/items/equipment_validation.rs`.
3. Update `src/sdk/templates.rs` — remove all 16 `#[allow(deprecated)]`
   suppressions and the `food` field from template constructors.
4. Update save game deserialization (`src/application/save_game.rs`) to handle
   legacy saves that contain the `food` field (deserialize and discard).
5. Update all RON data files that reference `food` as a character/party field.

#### 1.4 Fix `#[allow(clippy::field_reassign_with_default)]` in Tests

**11 suppressions in `src/domain/world/types.rs` tests.**

All 11 use the same anti-pattern: `TileVisualMetadata::default()` followed by
field reassignment. Fix by constructing the struct literal directly or adding
`with_*` builder methods to `TileVisualMetadata`:

| Test function                                      | Line  |
| -------------------------------------------------- | ----- |
| `test_tile_visual_metadata_with_sprite_layers`     | L5237 |
| `test_tile_visual_metadata_with_sprite_rule`       | L5269 |
| `test_grass_density_serialization`                 | L5344 |
| `test_has_terrain_overrides_returns_true_when_set` | L5377 |
| `test_foliage_density_clamps_in_valid_range`       | L5385 |
| `test_metadata_with_grass_density_serializes`      | L5461 |
| `test_has_terrain_overrides_detects_grass_density` | L5501 |
| `test_has_terrain_overrides_detects_tree_type`     | L5511 |
| `test_has_terrain_overrides_detects_all_fields`    | L5519 |
| `test_foliage_density_bounds`                      | L5540 |
| `test_snow_coverage_bounds`                        | L5558 |

Option A (preferred): Add a builder method like
`TileVisualMetadata::default().with_grass_density(0.8)` so tests are
expressive without triggering the lint.

Option B: Construct the full struct literal in each test instead of
default-then-reassign.

#### 1.5 Fix `#[allow(unused_mut)]` on `dialogue.rs` `execute_action`

**1 suppression in `src/game/systems/dialogue.rs` L838.**

The `mut` keyword on `game_log` and `game_log_writer` parameters is flagged
because clippy sees it is not strictly needed. The current code uses
`if let Some(ref mut log) = game_log` to reborrow without moving. Fix by
using `game_log.as_mut()` or `if let Some(log) = &mut game_log` patterns
instead of `ref mut`, then remove the `mut` from the parameter bindings and
the `#[allow(unused_mut)]`.

#### 1.6 Testing Requirements

- All existing tests must continue to pass after deletions.
- Verify no compilation errors from removed items (unused imports, missing
  fields).
- Run full quality gate: `cargo fmt`, `cargo check`, `cargo clippy -D
warnings`, `cargo nextest run`.

#### 1.7 Deliverables

- [ ] 10 `.bak` files deleted
- [ ] `*.bak` added to `.gitignore`
- [ ] Dead `CacheEntry<T>` subsystem removed from `sdk/cache.rs`
- [ ] Dead `content_cache` / `load_with_override` removed from
      `campaign_loader.rs`
- [ ] Dead `DEFAULT_RECRUITMENT_DIALOGUE_ID` removed from `world/types.rs`
- [ ] 22 dead constants removed from `procedural_meshes.rs`
- [ ] Dead `colors_approx_equal` removed from `hud.rs`
- [ ] `food` field fully removed from `Character` and `Party`
- [ ] All ~58 `#[allow(deprecated)]` suppressions eliminated
- [ ] 11 `#[allow(clippy::field_reassign_with_default)]` eliminated in
      `world/types.rs` tests
- [ ] 1 `#[allow(unused_mut)]` eliminated in `dialogue.rs`

#### 1.8 Success Criteria

- Zero `#[allow(dead_code)]` suppressions remain in `sdk/cache.rs`,
  `campaign_loader.rs`, `world/types.rs`, `procedural_meshes.rs` (constants
  section), and `hud.rs` (test helper).
- Zero `#[allow(deprecated)]` suppressions remain in the codebase.
- Zero `#[allow(clippy::field_reassign_with_default)]` suppressions remain.
- Zero `#[allow(unused_mut)]` suppressions remain in production code.
- Zero `.bak` files exist under `src/`.
- All quality gates pass.

---

### Phase 2: Strip Phase References (Low Risk, Medium Effort)

Remove all development-phase language from source code, tests, data files, and
root documentation. This is a mechanical find-and-replace. Use the file_edit tool to ensure consistency and go file by file CAREFULLY, renaming and avoiding missing any references.

#### 2.1 Rename Test Data IDs and Test Functions

**HIGH priority** — these leak into test output and assertion messages.

1. **`src/domain/character_definition.rs`** — Rename ~20 test data IDs:
   `test_phase3_weapon` → `test_starting_weapon`, `Phase3 Knight` →
   `Starting Equipment Knight`, `test_phase5_helmet` → `test_helmet_equip`,
   etc. (L3283–3516, L4199–4277).
2. **`src/application/save_game.rs`** — Rename test save slots:
   `phase5_buy_test` → `buy_sell_test`, `phase5_container_test` →
   `container_test`, `merchant_phase6` → `merchant_restock`,
   `phase6_restock_roundtrip` → `restock_roundtrip` (L1362–1551).
3. **`src/game/systems/facing.rs`** — Rename test function
   `test_set_facing_non_instant_snaps_in_phase3_without_proximity` →
   `test_set_facing_non_instant_snaps_without_proximity` (L588).

#### 2.2 Strip Phase Prefixes from Production Comments

Remove `Phase N:` prefixes from all production code comments while preserving
the descriptive text. ~50 instances across:

- `src/application/mod.rs` (L829, L850, L877, L1337, L1367, L1755)
- `src/bin/antares.rs` (L272, L275, L278, L281, L284)
- `src/bin/update_tutorial_maps.rs` (L4–5, L222 — also the printed string)
- `src/domain/campaign_loader.rs` (L144)
- `src/domain/character.rs` (L1181, L1191–1192, L1547, L1554–1555)
- `src/domain/character_definition.rs` (L26–27)
- `src/domain/classes.rs` (L13)
- `src/domain/combat/item_usage.rs` (L9, L29, L33)
- `src/domain/combat/types.rs` (L436)
- `src/domain/dialogue.rs` (L12, L764–765)
- `src/domain/items/equipment_validation.rs` (L13)
- `src/domain/items/types.rs` (multiple test section headers)
- `src/domain/proficiency.rs` (L1263)
- `src/domain/quest.rs` (L12)
- `src/domain/races.rs` (L13)
- `src/domain/resources.rs` (L9–12)
- `src/domain/transactions.rs` (L24)
- `src/domain/visual/item_mesh.rs` (L29–41, L145–190, L356–1761)
- `src/domain/world/` — `creature_binding.rs`, `furniture.rs`, `lock.rs`,
  `mod.rs`, `npc.rs`, `npc_runtime.rs`, `sprite_selection.rs`, `types.rs`,
  `events.rs`
- `src/game/components/` — `creature.rs`, `inventory.rs`
- `src/game/resources/` — `mod.rs`, `sprite_assets.rs`
- `src/game/systems/` — `advanced_grass.rs`, `advanced_trees.rs`

**Important**: Do NOT modify `Phase A:` / `Phase B:` comments in
`domain/combat/item_usage.rs` (L297–314) — these describe algorithmic borrow-
splitting phases, not development phases. Do NOT modify `lobe_phase` in
`bin/generate_terrain_textures.rs` — this is a math variable.

#### 2.3 Strip Phase Prefixes from Test Section Headers

Remove `Phase N:` from ~35 `// ===== Phase N: ... =====` section headers in
test modules. Replace with descriptive topic-only headers.

Example: `// ===== Phase 2: TimedStatBoost wiring tests =====` becomes
`// ===== TimedStatBoost wiring tests =====`.

Files: `src/application/mod.rs`, `src/application/save_game.rs`,
`src/domain/character.rs`, `src/domain/character_definition.rs`,
`src/domain/combat/database.rs`, `src/domain/combat/engine.rs`,
`src/domain/combat/item_usage.rs`, `src/domain/combat/types.rs`,
`src/domain/items/consumable_usage.rs`,
`src/domain/items/equipment_validation.rs`, `src/domain/items/types.rs`,
`src/domain/transactions.rs`, `src/domain/world/npc.rs`,
`src/domain/world/npc_runtime.rs`, `src/game/systems/advanced_trees.rs`.

#### 2.4 Clean Up Data Files and Root Documentation

1. **`data/classes.ron`** (L4) — Remove `Phase 1` reference.
2. **`data/examples/character_definition_formats.ron`** (L4) — Remove
   `(Phases 1 & 2)`.
3. **`data/npc_stock_templates.ron`** (L54) — Reword to remove phase ref.
4. **`data/test_campaign/data/npc_stock_templates.ron`** (L10–130) — Remove
   all Phase 3/6 references.
5. **`README.md`** (L58, L233–234, L267–277) — Replace phase-based roadmap
   section with a feature-based list.
6. **`assets/sprites/README.md`** (L36) — Remove `Phase 4` reference.
7. **`benches/grass_instancing.rs`** (L4), **`benches/grass_rendering.rs`**
   (L4), **`benches/sprite_rendering.rs`** (L4) — Remove phase references.

#### 2.5 Testing Requirements

- All tests pass with renamed IDs and function names.
- `grep -rn "Phase [0-9]" src/` returns zero results (excluding algorithmic
  `Phase A`/`Phase B` in `item_usage.rs`).

#### 2.6 Deliverables

- [ ] ~30 test data IDs/names/descriptions renamed
- [ ] 1 test function name renamed
- [ ] ~50 production comments cleaned
- [ ] ~35 test section headers cleaned
- [ ] Data files and root docs cleaned
- [ ] Benchmark module docs cleaned

#### 2.7 Success Criteria

- `grep -rn "Phase [0-9]" src/ benches/ data/` returns zero hits (excluding
  `item_usage.rs` algorithmic phases).
- `grep -rn "phase[0-9]" src/` returns zero hits.
- All quality gates pass.

---

### Phase 3: Consolidate Duplicate Code (Medium Risk, High Impact)

Extract shared patterns into reusable abstractions. This phase yields the
largest line-count reduction (~1,200+ lines) but requires careful refactoring.

#### 3.1 Create `RonDatabase` Trait or Macro

**Estimated reduction: ~350 lines across 16 implementations.**

The `load_from_file` / `load_from_string` pattern is identically repeated in:

- `src/domain/`: `items/database.rs`, `combat/database.rs`,
  `magic/database.rs`, `classes.rs`, `races.rs`, `proficiency.rs`,
  `character_definition.rs`, `visual/creature_database.rs`,
  `world/furniture.rs`, `world/npc_runtime.rs`
- `src/sdk/database.rs`: 6 more instances (SpellDatabase, MonsterDatabase,
  QuestDatabase, ConditionDatabase, DialogueDatabase, NpcDatabase)

Create a trait or declarative macro in a shared module (e.g.,
`src/domain/database_common.rs`) that provides blanket `load_from_file` and
`load_from_string` implementations given the entity type, ID accessor, error
variant, and HashMap field name.

#### 3.2 Extract Generic CLI Editor Base

**Estimated reduction: ~300 lines across 3 binaries.**

`src/bin/item_editor.rs`, `src/bin/class_editor.rs`, and
`src/bin/race_editor.rs` share identical:

- Editor struct layout (`items: Vec<T>`, `file_path: PathBuf`, `modified:
bool`)
- `run()` method with same match arms
- `confirm_exit()`, `read_input()`, `input_multistring_values()`, `save()`
  methods
- `truncate()` free function (duplicated in `class_editor.rs` L627 and
  `race_editor.rs` L771)
- `filter_valid_proficiencies()` (duplicated in `class_editor.rs` L663 and
  `race_editor.rs` L784)
- `filter_valid_tags()` (duplicated in `item_editor.rs` L1355 and
  `race_editor.rs` L793)

Extract a `CliEditor<T>` trait or base module in `src/bin/editor_common.rs`
with the shared scaffolding. Move `truncate`, `filter_valid_proficiencies`,
and `filter_valid_tags` into the shared module.

Also move duplicated constants:

- `STANDARD_PROFICIENCY_IDS` (duplicated in `class_editor.rs` L37 and
  `race_editor.rs` L37)
- `STANDARD_ITEM_TAGS` (duplicated in `item_editor.rs` L45 and
  `race_editor.rs` L52)

#### 3.3 Create Shared Inventory UI Module

**Estimated reduction: ~70 lines across 3 files.**

`src/game/systems/inventory_ui.rs`, `merchant_inventory_ui.rs`, and
`container_inventory_ui.rs` share:

- 7 identical UI constants (`PANEL_HEADER_H`, `SLOT_COLS`, `GRID_LINE_COLOR`,
  etc.)
- Nearly identical `NavigationState` structs with identical `reset()` methods

Create `src/game/systems/inventory_ui_common.rs` containing:

1. Shared constants
2. A generic `PanelNavState` struct with the shared fields and `reset()`
3. The `NavigationPhase` enum (currently defined in `inventory_ui.rs` and
   re-imported)

#### 3.4 Create Shared Test Character Factory

**Estimated reduction: ~100 lines across 12+ test modules.**

Create `src/test_helpers.rs` (gated behind `#[cfg(test)]`) with:

- `test_character(name: &str) -> Character`
- `test_character_with_class(name: &str, class_id: &str) -> Character`
- `test_dead_character(name: &str) -> Character`
- `test_character_with_weapon(name: &str) -> Character`

Replace the 12+ duplicated factory functions in: `application/save_game.rs`,
`application/resources.rs`, `domain/character.rs`, `domain/combat/engine.rs`,
`domain/items/consumable_usage.rs`, `domain/magic/casting.rs`,
`domain/party_manager.rs`, `domain/progression.rs`, `domain/transactions.rs`,
`domain/world/lock.rs`, `game/systems/lock_ui.rs`,
`game/systems/temple_ui.rs`.

#### 3.5 Extract UI Helper Functions

**Estimated reduction: ~200 lines.**

1. **Fullscreen overlay node** — Extract `fn fullscreen_centered_overlay() ->
Node` to replace 9+ identical 8-line `Node` blocks across `combat.rs`,
   `menu.rs`, `rest.rs`, `hud.rs`.
2. **SFX/Music helpers** — Extract `fn play_sfx(writer, sfx_id)` and
   `fn play_music(writer, track_id, looped)` to replace 11 identical guard
   patterns in `combat.rs`.
3. **Button spawn helper** — Extract `fn spawn_ui_button(parent, label,
marker, ...)` to replace repeated button spawn blocks in `menu.rs` and
   `combat.rs`.
4. **Text style constants** — Define `const BODY_TEXT_FONT: TextFont` and
   `const BODY_TEXT_COLOR: TextColor` to replace 14+ identical font/color
   combos.
5. **Image initialization** — Extract `fn create_blank_image(images, size) ->
Handle<Image>` to deduplicate `initialize_mini_map_image` and
   `initialize_automap_image` in `hud.rs`.

#### 3.6 Replace Trivial `Default` Implementations with `#[derive(Default)]`

**Estimated reduction: ~60 lines across 15+ types.**

All `impl Default for X { fn default() -> Self { Self::new() } }` blocks
where `new()` only sets `HashMap::new()` or other default values can be
replaced with `#[derive(Default)]`.

Affected types: `Resistances`, `Condition`, `Inventory`, `Equipment`,
`SpellBook`, `QuestFlags`, `Party`, `Roster`, `MonsterDatabase`,
`Monster`, `ItemDatabase`, `SpellDatabase`, `InventoryState`,
`ContainerInventoryState`, `CampaignLoader`, `CreatureDatabase`,
`ActiveSpells`, `QuestLog`, `GameState`.

#### 3.7 Testing Requirements

- All existing tests pass without modification (or with updated imports).
- New shared modules have their own unit tests.
- Test coverage does not decrease.

#### 3.8 Deliverables

- [ ] `RonDatabase` trait/macro created; 16 implementations migrated
- [ ] `editor_common.rs` created; 3 CLI editors refactored
- [ ] `inventory_ui_common.rs` created; 3 inventory UIs refactored
- [ ] `test_helpers.rs` created; 12+ test factories consolidated
- [ ] UI helper functions extracted
- [ ] Trivial `Default` impls replaced with `#[derive(Default)]`

#### 3.9 Success Criteria

- Zero duplicated `load_from_file` / `load_from_string` boilerplate.
- Zero duplicated CLI editor scaffolding.
- Zero duplicated inventory UI constants.
- Net line-count reduction ≥ 800 lines.
- All quality gates pass.

---

### Phase 4: Fix Error Handling (Medium Risk, High Impact)

Address silent error drops and inconsistent error patterns in production code.
These are correctness issues — lost quest rewards, unlogged combat failures,
and silent damage drops. Use the file_edit tool to ensure consistency and go file
by file CAREFULLY, renaming and avoiding missing any references.

#### 4.1 Audit and Fix Silent Error Drops in Combat

**~20 `let _ =` calls in `src/game/systems/combat.rs`.**

| Location                          | Operation                          | Fix                              |
| --------------------------------- | ---------------------------------- | -------------------------------- |
| L3220                             | `perform_attack_action_with_rng`   | Log error to combat log          |
| L3333                             | `perform_use_item_action_with_rng` | Log error to combat log          |
| L3637                             | `perform_cast_action_with_rng`     | Log error to combat log          |
| L3753, L4064, L4150, L4238, L4304 | `advance_turn` (×5)                | Log warning via `tracing::warn!` |
| L3783                             | `perform_defend_action`            | Log error to combat log          |
| L3911                             | `perform_flee_action`              | Log error to combat log          |
| L4548                             | `award_experience`                 | Log to combat log                |
| L4603, L4614                      | `add_item` (loot)                  | Log warning, notify player       |
| L2733, L2927, L4089               | `apply_damage`                     | Preserve return value or log     |

Strategy: Create a helper `fn log_combat_result<T, E: Display>(result:
Result<T, E>, combat_log: &mut CombatLog, context: &str)` to replace `let _
=` with consistent error logging.

#### 4.2 Fix Silent Error Drops in Spell Casting

**7 `let _ =` calls in `src/domain/combat/spell_casting.rs` (L259–359).**

`take_damage` and `apply_condition_to_monster_by_id` /
`apply_condition_to_character_by_id` results are silently discarded. At
minimum, aggregate errors and return them to the caller so combat feedback is
accurate.

#### 4.3 Fix Silent Error Drops in Quest and Dialogue Rewards

| File                          | Line | Operation                    | Fix                                                          |
| ----------------------------- | ---- | ---------------------------- | ------------------------------------------------------------ |
| `application/quests.rs`       | L305 | `add_item` (quest reward)    | Check result; if inventory full, queue overflow notification |
| `game/systems/dialogue.rs`    | L873 | `add_item` (dialogue reward) | Same as above                                                |
| `domain/combat/item_usage.rs` | L351 | `remove_item` (consume)      | Log error; consumable duplication bug potential              |

#### 4.4 Replace `panic!` with Graceful Error Handling

**`src/bin/antares.rs` L240** — Replace `panic!("Starting map {} not found")`
with `eprintln!` + `std::process::exit(1)` or propagate as `Result`.

#### 4.5 Harden Production `unwrap()` Calls

| File                     | Line         | Pattern                                  | Fix                                   |
| ------------------------ | ------------ | ---------------------------------------- | ------------------------------------- |
| `game/systems/combat.rs` | L2640        | `.unwrap()` after `is_none()` guard      | Refactor to `if let Some(attacker)`   |
| `game/systems/combat.rs` | L3948, L3956 | `.unwrap()` on `min_by_key`/`max_by_key` | Add `.expect("candidates non-empty")` |

#### 4.6 Migrate `Result<(), String>` to Typed Errors

Create a `ValidationError` enum with `thiserror` in `src/domain/validation.rs`
to replace ~20 functions returning `Result<_, String>`:

- `domain/dialogue.rs` — `DialogueTree::validate`
- `domain/races.rs` — `Resistances::validate`
- `domain/visual/animation.rs` — `AnimationDefinition::validate`
- `domain/visual/animation_state_machine.rs` — `AnimationStateMachine::validate`
- `domain/visual/blend_tree.rs` — `BlendNode::validate`
- `domain/visual/creature_variations.rs` — `CreatureVariation::validate`
- `domain/visual/mesh_validation.rs` — `validate_mesh_definition` + helpers
- `domain/visual/mod.rs` — `CreatureDefinition::validate`
- `domain/visual/skeletal_animation.rs` — `SkeletalAnimation::validate`
- `domain/visual/skeleton.rs` — `Skeleton::validate`
- `application/quests.rs` — `start_quest`
- `application/resources.rs` — `check_permadeath_allows_resurrection`,
  `perform_resurrection_service`

#### 4.7 Replace `println!` Placeholders with `tracing`

- `src/application/quests.rs` L316, L319 — Replace `println!` with
  `tracing::warn!` for unimplemented flag/reputation rewards.
- `src/game/systems/events.rs` L355, L363 — Replace `println!` with
  `tracing::warn!` for trap/treasure events.

#### 4.8 Investigate `clippy::only_used_in_recursion` Suppressions

**2 suppressions in `src/game/systems/dialogue.rs`:**

1. **`evaluate_conditions()` L541** — The `db` parameter
   (`&ContentDatabase`) is passed to every recursive call but never read
   within the function body. This may mask a logic bug where conditions
   like `HasItem` should be consulting the database for item data but
   aren't. Investigate and either:

   - Wire `db` into condition evaluation branches that need it, or
   - Remove the parameter entirely if it's genuinely unused, or
   - Document why the suppression is correct (e.g., future condition
     types will need it).

2. **`show_file_node()` in the SDK** (`sdk/campaign_builder/src/lib.rs`
   L5776) — The `&self` receiver is only used in the recursive call, not
   to access any struct fields. Convert to a free function or associated
   function taking `&FileNode` directly. (Tracked in SDK Cleanup Plan
   Phase 1.3.)

#### 4.9 Testing Requirements

- Write tests for each fixed error path (e.g., quest reward with full
  inventory now returns an error or queues overflow).
- Existing tests continue to pass.
- No new `let _ =` calls introduced.

#### 4.10 Deliverables

- [x] ~20 combat `let _ =` calls replaced with error logging
- [x] 7 spell casting `let _ =` calls fixed
- [x] 3 reward distribution `let _ =` calls fixed
- [x] `panic!` in `antares.rs` replaced with graceful exit
- [x] 3 production `unwrap()` calls hardened
- [x] `ValidationError` enum created; ~20 functions migrated
- [x] `println!` placeholders replaced with `tracing`
- [x] 2 `only_used_in_recursion` suppressions investigated and resolved

#### 4.11 Success Criteria

- `grep -rn "let _ =" src/ --include="*.rs" | grep -v "#\[cfg(test)\]"` count
  reduced to only intentionally-ignored non-critical results (history, dir
  creation).
- Zero `panic!` in production code outside of tests.
- Zero `Result<(), String>` in public function signatures.
- All quality gates pass.

---

### Phase 5: Structural Refactoring (Higher Risk, Long-Term Maintainability)

Address the 78 `too_many_arguments`, 10 `too_many_lines`, and 14
`type_complexity` clippy suppressions by introducing parameter structs,
extracting sub-functions, and defining type aliases for complex Bevy queries.

#### 5.1 Introduce `MeshSpawnContext` Parameter Struct

**Eliminates ~30 `too_many_arguments` suppressions in
`procedural_meshes.rs`.**

Create a struct bundling the common parameters passed to every `spawn_*`
function:

```
MeshSpawnContext {
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    map_id: MapId,
    config: &MeshConfig,
    cache: &mut MeshCache,
}
```

Refactor all ~30 `spawn_*` functions to accept `&mut MeshSpawnContext` instead
of 7+ individual parameters.

#### 5.2 Extract Sub-Renderers from Large UI Systems

**Addresses 9 `too_many_lines` suppressions.**

For each oversized system function, extract named sub-functions:

| File                        | Function                                   | Extract                                                                  |
| --------------------------- | ------------------------------------------ | ------------------------------------------------------------------------ |
| `inventory_ui.rs`           | `inventory_ui_system` (L960)               | `render_equipment_panel`, `render_item_grid`, `render_action_bar`        |
| `inventory_ui.rs`           | `inventory_input_system` (L531)            | `handle_grid_navigation`, `handle_action_selection`, `handle_equip_flow` |
| `merchant_inventory_ui.rs`  | `merchant_inventory_ui_system` (L511)      | `render_merchant_panel`, `render_player_panel`, `render_transaction_bar` |
| `container_inventory_ui.rs` | `container_inventory_ui_system` (L589)     | `render_container_panel`, `render_player_panel`                          |
| `inn_ui.rs`                 | `inn_ui_system` (L123)                     | `render_roster_list`, `render_party_slots`, `render_actions`             |
| `temple_ui.rs`              | `temple_ui_system` / `temple_input_system` | Extract condition-specific handlers                                      |

#### 5.3 Introduce Bevy SystemParam Structs and Type Aliases for Complex Queries

**Addresses 14 `#[allow(clippy::type_complexity)]` suppressions.**

| File                        | Occurrences | Approach                                    |
| --------------------------- | ----------- | ------------------------------------------- |
| `combat.rs`                 | 5           | `#[derive(SystemParam)]` for combat queries |
| `combat_visual.rs`          | 2           | Type alias for turn-indicator query         |
| `hud.rs`                    | 2           | Type alias for HUD entity queries           |
| `creature_meshes.rs`        | 1           | Type alias for texture-loading query        |
| `menu.rs`                   | 1           | Type alias for slider interaction query     |
| `sprite_uv_update.rs`       | 1           | Type alias for animated sprite query        |
| `inventory_ui.rs`           | 1           | Included in sub-renderer extraction (5.2)   |
| `container_inventory_ui.rs` | 1           | Included in sub-renderer extraction (5.2)   |

For `combat.rs`, define a `CombatQueries` `SystemParam` struct bundling the
commonly-shared query parameters. For simpler cases, define a type alias:
`type HudTextQuery = Query<&mut Text, With<ClockTimeText>>;` etc.

#### 5.4 Testing Requirements

- All existing tests pass.
- New parameter structs have constructor tests.
- Extracted sub-functions have targeted unit tests.

#### 5.5 Deliverables

- [ ] `MeshSpawnContext` struct created; ~30 functions refactored
- [ ] 9 oversized UI functions split into sub-renderers
- [ ] `SystemParam` structs and type aliases introduced for 14
      `type_complexity` suppressions
- [ ] All `too_many_arguments` suppressions in `procedural_meshes.rs`
      eliminated

#### 5.6 Success Criteria

- `grep -rn "too_many_arguments" src/game/systems/procedural_meshes.rs`
  returns zero hits.
- `grep -rn "too_many_lines" src/game/systems/` count reduced by ≥ 50%.
- Zero `#[allow(clippy::type_complexity)]` suppressions remain.
- All quality gates pass.

---

### Phase 6: Finish the Plan (Low–Medium Risk, Completeness)

Phases 1–5 left a trail of residual deliverables — items marked done that
weren't fully complete, items never started, and edge cases that slipped
through success-criteria grep patterns. This phase collects every known gap
into a single sweep so the cleanup plan can be closed out.

#### 6.1 Eliminate Remaining `#[allow(dead_code)]` on `ProceduralMeshCache` Fields

**Residual from Phase 1.2 — 3 suppressions remain.**

Three struct fields in `src/game/systems/procedural_meshes.rs` still carry
`#[allow(dead_code)]`:

| Field                    | Line  | Action                                              |
| ------------------------ | ----- | --------------------------------------------------- |
| `structure_wall`         | ~L100 | Wire into `spawn_wall` or remove if truly unused    |
| `structure_railing_post` | ~L108 | Wire into `spawn_railing` or remove if truly unused |
| `structure_railing_bar`  | ~L110 | Wire into `spawn_railing` or remove if truly unused |

For each field: if a `spawn_*` function exists that should use the cached
mesh, update it to read from the cache. If no consumer exists or is planned,
delete the field entirely. Do **not** leave `#[allow(dead_code)]`.

#### 6.2 Eliminate Remaining `#[allow(deprecated)]` in SDK

**Residual from Phase 1.3 — ~21 suppressions remain in `sdk/campaign_builder/`.**

These stem from `Item` struct construction that still references deprecated
fields. Files affected:

- `sdk/campaign_builder/src/advanced_validation.rs`
- `sdk/campaign_builder/src/asset_manager.rs`
- `sdk/campaign_builder/src/items_editor.rs` (9 instances)
- `sdk/campaign_builder/src/lib.rs` (6 instances)
- `sdk/campaign_builder/src/templates.rs` (2 instances)
- `sdk/campaign_builder/src/ui_helpers.rs` (1 instance)

Update all SDK `Item` construction sites to use the current field layout.
Remove every `#[allow(deprecated)]` suppression.

#### 6.3 Remove Hyphenated `Phase-N` References from Production Comments

**Residual from Phase 2.2 — 4 references survive the space-separated grep.**

| File                                       | Line  | Current Text                 | Action                         |
| ------------------------------------------ | ----- | ---------------------------- | ------------------------------ |
| `src/game/systems/dropped_item_visuals.rs` | L314  | `"Phase-3.2 addition"`       | Reword to describe the feature |
| `src/domain/world/npc_runtime.rs`          | L77   | `"Phase-6 fields"`           | Reword to describe the fields  |
| `src/domain/world/npc_runtime.rs`          | L246  | `"Phase-6 restock tracking"` | Reword to `"restock tracking"` |
| `src/domain/world/npc_runtime.rs`          | L1797 | `"Phase-6 defaults applied"` | Reword to `"defaults applied"` |

These pass the existing `grep -rn "Phase [0-9]"` criteria (space-separated)
but are still development-phase language that should not survive in a cleaned
codebase.

#### 6.4 Create `RonDatabase` Trait or Macro

**Unstarted from Phase 3.1 — 0 of 16 implementations migrated.**

The `load_from_file` / `load_from_string` pattern is identically repeated in
16 database types across `src/domain/` and `src/sdk/database.rs` (see Phase
3.1 for the full list). Create a trait or declarative macro in
`src/domain/database_common.rs` that provides blanket implementations given
the entity type, ID accessor, error variant, and `HashMap` field name.

**Target**: Zero duplicated `load_from_file` / `load_from_string` boilerplate.
Estimated reduction: ~350 lines.

#### 6.5 Expand `test_helpers.rs` to 12+ Factories

**Residual from Phase 3.4 — only 4 factories exist with 3 consumers.**

The current `src/test_helpers.rs` provides `test_character`,
`test_character_with_class`, `test_character_with_race_class`, and
`test_dead_character`. The original deliverable called for 12+ factories
covering the remaining inline test constructors.

Add the following factories and migrate their consumers:

| Factory                                  | Consumer Modules                                          |
| ---------------------------------------- | --------------------------------------------------------- |
| `test_character_with_weapon(name)`       | `combat/engine.rs`, `items/consumable_usage.rs`           |
| `test_character_with_spell(name, spell)` | `magic/casting.rs`                                        |
| `test_character_with_inventory(name)`    | `transactions.rs`, `items/consumable_usage.rs`            |
| `test_party()`                           | `save_game.rs`, `resources.rs`, `party_manager.rs`        |
| `test_party_with_members(n)`             | `combat/engine.rs`, `progression.rs`                      |
| `test_item(name)`                        | `transactions.rs`, `items/consumable_usage.rs`, `lock.rs` |
| `test_weapon(name)`                      | `combat/engine.rs`                                        |
| `test_spell(name)`                       | `magic/casting.rs`                                        |

Migrate the remaining ~9 test modules that still construct characters inline
to use the shared factories.

#### 6.6 Replace Trivial `Default` Implementations with `#[derive(Default)]`

**Unstarted from Phase 3.6 — 80+ manual impls remain.**

Audit all `impl Default for X` blocks. For each one where `default()` only
delegates to `Self::new()` **and** `new()` produces all-default values (empty
collections, zeros, `false`, `None`), replace with `#[derive(Default)]` and
remove the manual impl block.

Known candidates (at least 14 in `src/`):

- `GameState` (`application/mod.rs`)
- `Party` (`domain/character.rs`)
- `MonsterResistances` (`domain/combat/monster.rs`)
- `MerchantStock`, `ServiceCatalog` (`domain/inventory.rs`)
- `MeshTransform` (`domain/visual/mod.rs`)
- `World` (`domain/world/types.rs`)
- `BranchGraph` (`game/systems/advanced_trees.rs`)
- `CombatResource` (`game/systems/combat.rs`)
- `SpriteAssets` (`game/resources/sprite_assets.rs`)

Plus ~10 in `sdk/` (`ContextMenuManager`, `CreatureIdManager`,
`ShortcutManager`, `UndoRedoManager`, `PreviewRenderer`, etc.)

For types where `new()` sets non-default values (string literals, specific
numbers, colors), the manual impl is correct — leave those alone.

Estimated reduction: ~60 lines.

#### 6.7 Harden Remaining Production `unwrap()` Calls

**Residual from Phase 4.5 — 2 `unwrap()` calls remain in `performance.rs`.**

| File                            | Line  | Pattern                                | Fix                                                  |
| ------------------------------- | ----- | -------------------------------------- | ---------------------------------------------------- |
| `game/resources/performance.rs` | ~L117 | `.partial_cmp(b).unwrap()` in `min_by` | Replace with `f32::total_cmp` or `.unwrap_or(Equal)` |
| `game/resources/performance.rs` | ~L127 | `.partial_cmp(b).unwrap()` in `max_by` | Replace with `f32::total_cmp` or `.unwrap_or(Equal)` |

These will panic on NaN `f32` values. Use `f32::total_cmp` for a safe,
allocation-free comparison that handles NaN correctly.

#### 6.8 Eliminate Remaining Production `panic!` Calls

**Residual from Phase 4.4 — 4 production `panic!` calls remain.**

| File                                | Line  | Context                          | Fix                                                 |
| ----------------------------------- | ----- | -------------------------------- | --------------------------------------------------- |
| `game/systems/menu.rs`              | ~L39  | `SaveGameManager` initialization | Return `Result` or use `expect()` with message      |
| `game/systems/procedural_meshes.rs` | ~L423 | Unknown component match arm      | Log `tracing::error!` + return early or use default |
| `game/systems/procedural_meshes.rs` | ~L462 | Unknown component match arm      | Log `tracing::error!` + return early or use default |
| `game/systems/procedural_meshes.rs` | ~L563 | Unknown component match arm      | Log `tracing::error!` + return early or use default |

The `procedural_meshes.rs` panics are in match arms for enum variants that
should never appear — replace with `tracing::error!` + a safe no-op fallback
rather than crashing the game. For `menu.rs`, propagate the error or use
`expect("SaveGameManager state directory must be writable")` with a
descriptive message.

#### 6.9 Migrate `dialogue_validation.rs` to `ValidationError`

**Residual from Phase 4.6 — `Result<(), String>` remains in public API.**

`src/game/systems/dialogue_validation.rs` L20 defines:

```
pub type ValidationResult = Result<(), String>;
```

Migrate all validation functions in this file to return
`Result<(), ValidationError>` using the existing `ValidationError` enum from
`src/domain/validation.rs`. Update all callers.

#### 6.10 Replace Remaining Production `eprintln!` with `tracing`

**Residual from Phase 4.7 — 4 `eprintln!("Warning: ...")` calls remain.**

| File                    | Line   | Current Message                        | Replacement      |
| ----------------------- | ------ | -------------------------------------- | ---------------- |
| `sdk/database.rs`       | ~L389  | `"Warning: failed to read map file…"`  | `tracing::warn!` |
| `sdk/database.rs`       | ~L408  | `"Warning: failed to parse map file…"` | `tracing::warn!` |
| `sdk/game_config.rs`    | ~L184  | `"Warning: Config file not found…"`    | `tracing::warn!` |
| `domain/world/types.rs` | ~L3057 | `"Warning: NPC '{}' not found…"`       | `tracing::warn!` |

Note: `sdk/error_formatter.rs` intentionally uses `println!`/`eprintln!` as a
console output formatter — that is correct by design and should not be
changed.

#### 6.11 Testing Requirements

- All existing tests pass without modification (or with updated imports).
- New `RonDatabase` trait/macro has its own unit tests.
- New test factories have their own tests verifying default state.
- Each hardened `unwrap()` / replaced `panic!` path has a test exercising
  the new error branch.
- `dialogue_validation.rs` callers updated and tested with `ValidationError`.
- Test coverage does not decrease.

#### 6.12 Deliverables

- [ ] 3 `#[allow(dead_code)]` eliminated from `ProceduralMeshCache` fields
- [ ] ~21 `#[allow(deprecated)]` eliminated from `sdk/campaign_builder/`
- [ ] 4 hyphenated `Phase-N` comment references removed
- [ ] `RonDatabase` trait/macro created; 16 implementations migrated
- [ ] `test_helpers.rs` expanded to 12+ factories; ~9 remaining consumers
      migrated
- [ ] Trivial `Default` impls replaced with `#[derive(Default)]` (~14+ types)
- [ ] 2 production `unwrap()` calls in `performance.rs` hardened
- [ ] 4 production `panic!` calls replaced with graceful error handling
- [ ] `dialogue_validation.rs` migrated from `Result<(), String>` to
      `ValidationError`
- [ ] 4 production `eprintln!` calls replaced with `tracing::warn!`

#### 6.13 Success Criteria

- Zero `#[allow(dead_code)]` suppressions remain in `procedural_meshes.rs`.
- Zero `#[allow(deprecated)]` suppressions remain project-wide (including
  `sdk/`).
- `grep -rn "Phase-[0-9]" src/` returns zero hits.
- Zero duplicated `load_from_file` / `load_from_string` boilerplate.
- `test_helpers.rs` provides ≥ 12 factory functions with ≥ 10 consumer
  modules.
- Net reduction of ≥ 60 lines from `Default` impl cleanup.
- Zero `partial_cmp().unwrap()` in production code.
- Zero `panic!` in production code outside of tests.
- Zero `Result<(), String>` in public function signatures.
- Zero `eprintln!("Warning: ...")` calls in production code (excluding
  `error_formatter.rs`).
- All quality gates pass.

---

## Appendix A: Placeholder Stubs and Missing Features

The 24 placeholder stubs discovered during the codebase audit, plus 4
additional user-reported issues (game log placement, time advancement,
recruited character mesh persistence, lock UI input handling), are tracked in a
separate plan:

**[Game Feature Completion Plan](./game_feature_completion_plan.md)**

These items require new feature implementation, not code cleanup, and are
intentionally kept in their own plan to prevent scope creep.

## Appendix B: Suppression Elimination Summary

| Suppression                                     | Count   | Phase   | Action                                 |
| ----------------------------------------------- | ------- | ------- | -------------------------------------- |
| `#[allow(deprecated)]`                          | 58      | 1.3/6.2 | Remove `food` field entirely; fix SDK  |
| `#[allow(dead_code)]`                           | 34      | 1.2/6.1 | Delete dead code; fix mesh cache       |
| `#[allow(clippy::field_reassign_with_default)]` | 11      | 1.4     | Builder methods or struct literals     |
| `#[allow(unused_mut)]`                          | 1       | 1.5     | Remove `mut` + adjust patterns         |
| `#[allow(clippy::only_used_in_recursion)]`      | 2       | 4.8     | Investigate and resolve                |
| `#[allow(clippy::too_many_arguments)]`          | 78      | 5.1     | `MeshSpawnContext` + parameter structs |
| `#[allow(clippy::too_many_lines)]`              | 10      | 5.2     | Extract sub-renderers                  |
| `#[allow(clippy::type_complexity)]`             | 14      | 5.3     | `SystemParam` structs + type aliases   |
| **Total**                                       | **208** |         | **Target: 0**                          |

## Appendix C: Files Changed Per Phase

| Phase              | Estimated Files Touched   | Estimated Net Line Change |
| ------------------ | ------------------------- | ------------------------- |
| 1: Dead Weight     | ~27 files + 10 deletions  | −300 lines                |
| 2: Phase Refs      | ~60 files                 | −0 lines (rewording only) |
| 3: Dedup           | ~30 files + 4 new modules | −800 lines                |
| 4: Error Handling  | ~15 files + 1 new module  | +200 lines (logging)      |
| 5: Structural      | ~12 files + 3 new structs | −100 lines                |
| 6: Finish the Plan | ~45 files + 1 new module  | −450 lines                |
| **Total**          | ~105 unique files         | **−1,450 lines net**      |
