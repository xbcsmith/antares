# Implementations

## Phase 1: Remove Dead Weight (Complete)

### Overview

Executed Phase 1 of the game codebase cleanup plan: deleted all backup files,
removed dead code behind `#[allow(dead_code)]` suppressions, completed the
deprecated `food` field migration, fixed `#[allow(clippy::field_reassign_with_default)]`
suppressions in tests, and fixed the `#[allow(unused_mut)]` suppression in
`dialogue.rs`. All 3944 tests pass; all four quality gates pass with zero
errors and zero warnings.

### 1.1 — Deleted 10 `.bak` Files

All backup files checked into `src/` were deleted and `*.bak` was added to
`.gitignore`:

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

### 1.2 — Removed Dead Code Behind `#[allow(dead_code)]`

- **`src/sdk/cache.rs`**: Removed `CacheEntry<T>` struct and its two methods
  (`new`, `is_expired`), the `compute_file_hash` method on `ContentCache`, and
  the `preload_common_content` public helper function. Removed associated tests
  (`test_cache_entry_expiration`, `test_compute_file_hash`). Also removed the
  now-unused `serde::{Deserialize, Serialize}` and `std::fs` imports.

- **`src/domain/campaign_loader.rs`**: Removed the `content_cache:
HashMap<String, String>` field from `CampaignLoader`, its initialization in
  `CampaignLoader::new()`, and the `load_with_override<T>()` method. Removed
  the now-unused `HashMap` and `DeserializeOwned` imports.

- **`src/domain/world/types.rs`**: Removed the
  `DEFAULT_RECRUITMENT_DIALOGUE_ID` constant.

- **`src/game/systems/procedural_meshes.rs`**: Removed 15 truly dead
  dimension/color constants (`THRONE_HEIGHT`, `SHRUB_STEM_COLOR`,
  `SHRUB_FOLIAGE_COLOR`, `GRASS_BLADE_COLOR`, `COLUMN_SHAFT_RADIUS`,
  `COLUMN_CAPITAL_RADIUS`, `ARCH_OUTER_RADIUS`, `WALL_THICKNESS`,
  `RAILING_POST_RADIUS`, `STRUCTURE_IRON_COLOR`, `STRUCTURE_GOLD_COLOR`) and
  their `let _ = CONSTANT` test stubs. Restored the remaining 7 constants that
  ARE genuinely referenced in production or test code
  (`ARCH_SUPPORT_WIDTH/HEIGHT`, `DOOR_FRAME_THICKNESS`, `DOOR_FRAME_BORDER`,
  `ITEM_PARCHMENT_COLOR`, `ITEM_GOLD_COLOR`) without `#[allow(dead_code)]`;
  test-only constants were annotated `#[cfg(test)]` to prevent dead_code
  warnings in non-test builds.

- **`src/game/systems/hud.rs`**: The `colors_approx_equal` test helper was
  confirmed to be used by 10 test assertions. Removed `#[allow(dead_code)]`
  from it and added `#[cfg(test)]` to the enclosing `mod tests` block so the
  helper (and all its callers) only compile in test mode, eliminating the
  spurious `unused_import` warning on `use super::*`.

### 1.3 — Completed the Deprecated `food` Field Migration

The `#[deprecated]` `food: u8` field on `Character` and `food: u32` field on
`Party` were fully removed:

- Deleted both `#[deprecated(...)]` field declarations from
  `src/domain/character.rs`.
- Removed `#[allow(deprecated)]` and `food: 0` from `Character::new()` and
  `Party::new()`.
- Removed the `food` assertion from `test_character_default_values`.
- Removed `#[allow(deprecated)]` and `food: 0` from
  `CharacterDefinition::instantiate()` in `src/domain/character_definition.rs`.
- Removed stale `food` assertions from two tests in `character_definition.rs`.
- Removed `food: 0` and `#[allow(deprecated)]` from
  `test_good_character_cannot_equip_evil_item` in
  `src/domain/items/equipment_validation.rs`.
- Removed all 17 `#[allow(deprecated)]` from `src/sdk/templates.rs` (stale
  since `mesh_id` was un-deprecated).
- Removed 4 `#[allow(deprecated)]` from `src/domain/items/types.rs` tests.
- Removed 8 `#[allow(deprecated)]` from `src/bin/item_editor.rs`.
- Removed 5 `#[allow(deprecated)]` and stale food comments from
  `src/application/mod.rs`.
- Removed stale food comments from `src/application/save_game.rs`.
- Fixed 3 integration tests that still accessed `party.food`:
  `tests/innkeeper_party_management_integration_test.rs`,
  `tests/campaign_integration_test.rs`, `tests/game_flow_integration.rs`.
- Removed 7 stale `#[allow(deprecated)]` from `tests/cli_editor_tests.rs`.

Serde's default behavior (ignore unknown fields) provides automatic backward
compatibility for legacy save files that still contain the `food` field.

### 1.4 — Fixed `#[allow(clippy::field_reassign_with_default)]` in Tests

All 11 suppressions in `src/domain/world/types.rs` were eliminated by
converting the default-then-reassign anti-pattern to struct update syntax
(`TileVisualMetadata { field: value, ..TileVisualMetadata::default() }`).
Multi-field tests (`test_foliage_density_bounds`, `test_snow_coverage_bounds`,
`test_has_terrain_overrides_detects_all_fields`) were refactored to construct
a fresh struct literal per assertion.

### 1.5 — Fixed `#[allow(unused_mut)]` in `dialogue.rs`

Removed the `#[allow(unused_mut)]` suppression from `execute_action` in
`src/game/systems/dialogue.rs`. Replaced all `if let Some(ref mut log) =
game_log` patterns with `if let Some(log) = game_log.as_mut()` (14
occurrences), and all `if let Some(ref mut writer) = game_log_writer` with
`if let Some(writer) = game_log_writer.as_mut()` (4 occurrences). The `mut`
keyword on the `game_log` and `game_log_writer` parameter bindings was
retained because it is required for the `&mut game_log` borrows passed to
`execute_recruit_to_party`.

### Deliverables Checklist

- [x] 10 `.bak` files deleted
- [x] `*.bak` added to `.gitignore`
- [x] Dead `CacheEntry<T>` subsystem removed from `sdk/cache.rs`
- [x] Dead `content_cache` / `load_with_override` removed from `campaign_loader.rs`
- [x] Dead `DEFAULT_RECRUITMENT_DIALOGUE_ID` removed from `world/types.rs`
- [x] 15 dead constants removed from `procedural_meshes.rs` (7 restored without suppressions; remaining dead_code handled via `#[cfg(test)]`)
- [x] Dead `colors_approx_equal` suppression removed from `hud.rs` (function retained, `mod tests` made `#[cfg(test)]`)
- [x] `food` field fully removed from `Character` and `Party`
- [x] All `#[allow(deprecated)]` suppressions eliminated
- [x] 11 `#[allow(clippy::field_reassign_with_default)]` eliminated in `world/types.rs` tests
- [x] 1 `#[allow(unused_mut)]` eliminated in `dialogue.rs`
- [x] `cargo fmt --all` — clean
- [x] `cargo check --all-targets --all-features` — 0 errors, 0 warnings
- [x] `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
- [x] `cargo nextest run --all-features` — 3944 passed, 0 failed

## Scripts and Examples Directory Cleanup (Complete)

### Overview

Swept through the `scripts/` and `examples/` directories to remove deprecated
one-time migration scripts, stale copies, and orphaned examples. Moved
reusable asset generators into `scripts/`, relocated OBJ test fixtures to
`data/test_fixtures/` per Implementation Rule 5, and deleted the `examples/`
directory entirely.

### What Was Removed

**scripts/ — 17 items deleted:**

| File                                     | Reason                                                  |
| ---------------------------------------- | ------------------------------------------------------- |
| `__pycache__/`                           | Python bytecode cache — should never be committed       |
| `build_merged.py`                        | One-time mesh generator assembler                       |
| `builder.py`                             | Duplicate of `build_merged.py`                          |
| `clean_map_metadata.py`                  | One-time map data cleanup, already applied              |
| `discover_csv_combobox.sh`               | CSV migration discovery — migration complete            |
| `fix_build.py`                           | Meta-fixer for `build_merged.py` (also deleted)         |
| `fix_foliage_density.py`                 | One-time foliage data fix (v2 of 3 variants)            |
| `fix_foliage_simple.py`                  | One-time foliage data fix (v3 of 3 variants)            |
| `id_extractor.py`                        | Support script for deleted mesh generators              |
| `output.txt`                             | Stale agent working notes                               |
| `shift_ids.py`                           | One-time ID migration with hardcoded absolute paths     |
| `update_tutorial_maps.py`                | Replaced by `src/bin/update_tutorial_maps.rs`           |
| `update_tutorial_maps.rs`                | Stale copy — canonical version is in `src/bin/`         |
| `update_tutorial_maps.sh`                | sed/perl variant, also replaced by `src/bin/`           |
| `validate_csv_migration.sh`              | One-time migration validation — migration complete      |
| `validate_tutorial_maps.sh`              | Hardcoded stale map names; `validate_map` binary exists |
| `validate_creature_editor_doc_parity.sh` | Brittle string matching; better as a cargo test         |

**examples/ — entire directory deleted (11 items):**

| File                                | Reason                                                |
| ----------------------------------- | ----------------------------------------------------- |
| `generate_starter_maps.rs`          | Self-declares as DEPRECATED in its own doc comment    |
| `npc_blocking_README.md`            | Phase 1 doc, naming violation, coverage in main tests |
| `npc_blocking_example.rs`           | Phase 1 demo, blocking behavior tested in domain      |
| `obj_to_ron_universal.py`           | Functionality ported to Rust SDK (`mesh_obj_io.rs`)   |
| `name_generator_example.rs`         | Not in Cargo.toml `[[example]]`; better as doctest    |
| `npc_blueprints/README.md`          | Misplaced docs; covered by implementation archives    |
| `npc_blueprints/town_with_npcs.ron` | Redundant with actual campaign/test data              |

### What Was Moved / Kept

- **`examples/generate_all_meshes.py`** → `scripts/generate_all_meshes.py`
  (active creature mesh asset generator)
- **`examples/generate_item_meshes.py`** → `scripts/generate_item_meshes.py`
  (active item mesh asset generator)
- **`examples/female_1.obj`** → `data/test_fixtures/female_1.obj`
  (test fixture used by 2 SDK tests — Rule 5 compliance)
- **`examples/skeleton.obj`** → `data/test_fixtures/skeleton.obj`
  (test fixture used by 2 SDK tests — Rule 5 compliance)
- Updated `fixture_path()` calls in `sdk/campaign_builder/src/mesh_obj_io.rs`
  and `sdk/campaign_builder/src/obj_importer_ui.rs` to reference
  `data/test_fixtures/` instead of `examples/`.
- Added `__pycache__/` to `.gitignore`.

### Final `scripts/` Contents (6 files)

| File                              | Purpose                                         |
| --------------------------------- | ----------------------------------------------- |
| `generate_all_meshes.py`          | Regenerates all creature mesh RON assets        |
| `generate_icons.sh`               | macOS icon pipeline from source PNG             |
| `generate_item_meshes.py`         | Regenerates item mesh RON assets                |
| `generate_placeholder_sprites.py` | Placeholder sprite sheet generator              |
| `test-changed.sh`                 | Incremental test runner (changed packages only) |
| `test-full.sh`                    | Full workspace test suite runner                |

### Quality Gates

```text
cargo fmt         → no output (clean)
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings
cargo nextest run → 3946 passed; 0 failed; 8 skipped
```

---

## Codebase-Wide `#[allow(...)]` Audit and Plan Updates (Complete)

### Overview

Performed a comprehensive audit of every `#[allow(...)]` suppression across the
entire Antares codebase (game engine `src/` and SDK `sdk/`) to identify
eliminable suppressions beyond what was already captured in the cleanup plans.
Updated the Game Codebase Cleanup Plan with newly-discovered items and accurate
counts.

### What Was Found

Full inventory of 254 `#[allow(...)]` suppressions across the codebase:

| Suppression                           | Game Engine      | SDK | Total | Eliminable?                     |
| ------------------------------------- | ---------------- | --- | ----- | ------------------------------- |
| `#![allow(...)]` crate-level          | 0                | 9   | 9     | Yes (SDK Plan Phase 1.1)        |
| `deprecated`                          | 37 (+21 in .bak) | 21  | 79    | Yes, after food field removal   |
| `dead_code`                           | 34               | 5   | 39    | ~35 yes, ~4 review              |
| `clippy::too_many_arguments`          | 78               | 28  | 106   | Refactor (both plans Phase 5/6) |
| `clippy::too_many_lines`              | 10               | 0   | 10    | Refactor (Game Plan Phase 5.2)  |
| `clippy::type_complexity`             | 14               | 0   | 14    | Refactor (Game Plan Phase 5.3)  |
| `clippy::field_reassign_with_default` | 11               | 0   | 11    | Yes — builder patterns          |
| `clippy::only_used_in_recursion`      | 2                | 1   | 3     | Yes — free functions            |
| `unused_mut`                          | 1                | 0   | 1     | Yes — adjust patterns           |
| `clippy::map_clone`                   | 0                | 1   | 1     | Yes — use `.cloned()`           |
| `clippy::ptr_arg`                     | 0                | 2   | 2     | Yes — `&Path` not `&PathBuf`    |

### What Was Updated

Updated `docs/explanation/game_codebase_cleanup_plan.md` with four newly-
identified suppression categories not previously captured:

1. **Phase 1.4 (new section)**: 11 `#[allow(clippy::field_reassign_with_default)]`
   in `src/domain/world/types.rs` tests — fix via builder methods or struct
   literals on `TileVisualMetadata`.
2. **Phase 1.5 (new section)**: 1 `#[allow(unused_mut)]` on `dialogue.rs`
   `execute_action` — fix by adjusting reborrow patterns.
3. **Phase 4.8 (expanded)**: Now covers both `only_used_in_recursion`
   suppressions (game engine `evaluate_conditions` + SDK `show_file_node`).
4. **Phase 5.3 (expanded)**: Now explicitly lists all 14 `type_complexity`
   suppressions by file with specific fix approaches (was previously "8").

Also updated: Overview stats, Identified Issues section (accurate counts for
all suppression types), Deliverables, Success Criteria, and added a new
**Appendix B: Suppression Elimination Summary** table mapping all 208 game
engine suppressions to their resolution phase.

### Outcome

Both cleanup plans now have complete, audited suppression inventories with
zero gaps. The target across both plans is elimination of all 254 suppressions
(208 game engine + 46 SDK after deducting the 21 `.bak` duplicates that are
deleted in Phase 1.1).

## SDK Codebase Cleanup Plan (Plan Written)

### Overview

Authored a comprehensive 6-phase cleanup plan for the Antares SDK Campaign
Builder codebase (`sdk/campaign_builder/`). The plan addresses technical debt
accumulated across 107,880 lines of SDK source code spanning 62 files.

### What Was Analyzed

Ran parallel automated analyses across the SDK codebase to identify:

- **Dead code and suppressions**: 5 genuinely dead `#[allow(dead_code)]` items,
  9 blanket crate-level `#![allow(...)]` directives hiding real issues, 28
  `#[allow(clippy::too_many_arguments)]` suppressions, 2 `#[ignore]`d skeleton
  tests, ~21 `#[allow(deprecated)]` suppressions from upstream `Item` struct.
- **Duplicate code**: ~4,300 lines of duplicated patterns across 7 categories
  (toolbar handling in 8 editors, list/action dispatch in 6 editors, 3
  undo/redo managers, 2 mesh editor history implementations, dual validation
  type hierarchies, 13 near-identical autocomplete selectors, 7 RON load/save
  method pairs in `lib.rs`).
- **Error handling inconsistency**: ~30 public functions returning
  `Result<(), String>` instead of typed errors, ~30 `eprintln!` calls in
  production code bypassing the SDK's own `Logger`, ~15 `let _ =` patterns
  silently dropping `Result` values from user-facing save operations, duplicate
  `ValidationSeverity`/`ValidationResult` types between `validation.rs` and
  `advanced_validation.rs`.
- **Phase references**: ~130 phase references in source comments, module docs,
  test section headers, and `README.md`.
- **Structural issues**: `lib.rs` at 12,312 lines with `CampaignBuilderApp`
  holding ~140 fields (god object), `ui_helpers.rs` at 7,734 lines as a
  catch-all, ~5,700 lines of inline tests in `lib.rs`, 2
  `campaigns/tutorial` violations.

### Plan Structure

The plan is organized into 6 phases ordered by risk (lowest first) and impact
(highest first), with explicit upstream dependencies on the Game Codebase
Cleanup Plan and Game Feature Completion Plan:

1. **Phase 1: Remove Dead Code and Fix Lint Suppressions** — Remove 9 blanket
   `#![allow(...)]` directives, delete 5 dead code items, fix trivial clippy
   suppressions, remove `#[allow(deprecated)]` after upstream food field
   removal, fix `campaigns/tutorial` violations.
2. **Phase 2: Strip Phase References** — Remove ~130 phase references from
   source comments, rewrite SDK `README.md`, clean up stale comments.
3. **Phase 3: Unify Validation Types and Fix Error Handling** — Unify
   duplicate `ValidationSeverity`/`ValidationResult` types, migrate ~30
   functions from `Result<(), String>` to typed `thiserror` errors, replace
   `eprintln!` with SDK Logger, fix silent `Result` drops.
4. **Phase 4: Consolidate Duplicate Code** — Extract generic autocomplete
   selectors (~800 lines saved), generic toolbar handler (~700 lines saved),
   generic list/action dispatch (~500 lines saved), generic undo/redo stack
   (~200 lines saved), generic RON load/save (~500 lines saved).
5. **Phase 5: Structural Refactoring** — Split `ui_helpers.rs` into
   sub-modules, extract campaign I/O from `lib.rs`, decompose
   `CampaignBuilderApp` into focused state structs, move ~5,700 lines of
   inline tests to dedicated test files. Target: `lib.rs` ≤ 3,000 lines.
6. **Phase 6: Reduce `too_many_arguments` Suppressions** — Introduce
   `EditorContext` parameter struct adopted by all editor `show()` methods,
   eliminating all 28 suppressions.

### Outcome

Plan written to `docs/explanation/sdk_codebase_cleanup_plan.md` and
`docs/explanation/next_plans.md` updated to reference it. No code changes
were made — this is a planning artifact only.

## Phase 2: Strip Phase References (Complete)

### Overview

Removed all development-phase language (`Phase 1:`, `Phase 2:`, etc.) from
source code, tests, data files, benchmarks, and root documentation. This was
a mechanical find-and-replace effort with **zero behavioral changes**. The
algorithmic `Phase A:` / `Phase B:` comments in `item_usage.rs` and the
`lobe_phase` math variable in `generate_terrain_textures.rs` were correctly
preserved.

### 2.1 — Renamed Test Data IDs and Test Functions

| File                                 | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/game/systems/facing.rs`         | `test_set_facing_non_instant_snaps_in_phase3_without_proximity` → `test_set_facing_non_instant_snaps_without_proximity`                                                                                                                                                                                                                                                                                                                                 |
| `src/application/save_game.rs`       | `phase5_buy_test` → `buy_sell_test`, `phase5_container_test` → `container_test`, `merchant_phase6` → `merchant_restock`, `phase6_restock_roundtrip` → `restock_roundtrip`                                                                                                                                                                                                                                                                               |
| `src/domain/character_definition.rs` | `test_phase3_weapon` → `test_starting_weapon`, `Phase3 Knight` → `Starting Equipment Knight`, `test_phase3_unequip` → `test_starting_unequip`, `test_phase3_ac` → `test_starting_armor_ac`, `test_phase3_no_eq` → `test_no_starting_equipment`, `test_phase3_invalid_eq` → `test_invalid_starting_equipment`, `test_phase5_helmet` → `test_helmet_equip`, `test_phase5_boots` → `test_boots_equip` (plus corresponding `name` and `description` fields) |

### 2.2 — Stripped Phase Prefixes from Production Comments

~200+ inline comments across 60+ source files had `Phase N:` prefixes removed
while preserving the descriptive text. Examples:

- `// Phase 2: select handicap based on combat event type.` → `// Select handicap based on combat event type.`
- `// Phase 3: set Animating before the domain call` → `// Set Animating before the domain call`
- `/// See ... Phase 5 for dialogue specifications.` → `/// See ... for dialogue specifications.`
- `// Phase 4: Boss monsters never flee` → `// Boss monsters never flee`

Key files with many changes: `combat.rs` (~67 refs), `map.rs` (~28 refs),
`item_mesh.rs` (~20 refs), `application/mod.rs` (~13 refs).

### 2.3 — Stripped Phase Prefixes from Test Section Headers

~40 `// ===== Phase N: ... =====` section headers in test modules were
replaced with descriptive topic-only headers. Examples:

- `// ===== Phase 2: Normal and Ambush Combat Tests =====` → `// ===== Normal and Ambush Combat Tests =====`
- `// ===== Phase 3: Player Action System Tests =====` → `// ===== Player Action System Tests =====`
- `// ===== Phase 5: Performance & Polish Tests =====` → `// ===== Performance & Polish Tests =====`

### 2.4 — Cleaned Data Files and Root Documentation

| File                                              | Change                                                                                          |
| ------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `data/classes.ron`                                | Removed `Phase 1` spec reference                                                                |
| `data/examples/character_definition_formats.ron`  | Removed `(Phases 1 & 2)`                                                                        |
| `data/npc_stock_templates.ron`                    | Removed `Phase 2 of the food system migration`                                                  |
| `data/test_campaign/data/npc_stock_templates.ron` | Removed all Phase 3/6 references (~10 comments)                                                 |
| `README.md`                                       | Replaced phase-based roadmap with feature-based list; removed `(Phase 6 - Latest)` from heading |
| `assets/sprites/README.md`                        | Removed `Phase 4` reference                                                                     |
| `benches/grass_instancing.rs`                     | Removed `(Phase 4)`                                                                             |
| `benches/grass_rendering.rs`                      | Removed `(Phase 2)`                                                                             |
| `benches/sprite_rendering.rs`                     | Removed `(Phase 3)`                                                                             |

### Deliverables Checklist

- [x] ~20 test data IDs/names/descriptions renamed
- [x] 1 test function name renamed
- [x] ~200+ production comments cleaned across 60+ files
- [x] ~40 test section headers cleaned
- [x] Data files and root docs cleaned
- [x] Benchmark module docs cleaned

### Success Criteria

- `grep -rn "Phase [0-9]" src/ benches/ data/` returns **zero hits** (excluding
  `item_usage.rs` algorithmic `Phase A`/`Phase B`).
- `grep -rn "phase[0-9]" src/` returns **zero hits**.
- All quality gates pass:
  - `cargo fmt --all` — ✅ no output
  - `cargo check --all-targets --all-features` — ✅ Finished, 0 errors
  - `cargo clippy --all-targets --all-features -- -D warnings` — ✅ 0 warnings
  - `cargo nextest run --all-features` — ✅ 3,944 passed, 0 failed, 8 skipped
