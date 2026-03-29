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

## CLI Editor Shared Module Extraction (Complete)

### Overview

Extracted duplicated constants and helper functions from three CLI editor
binaries (`item_editor.rs`, `class_editor.rs`, `race_editor.rs`) into a new
shared module `src/bin/editor_common.rs`. This eliminates code duplication
while preserving identical behavior and full test coverage.

### What Was Extracted

The following items were duplicated across two or three editor binaries:

| Item                                      | Previously In                       | Now In             |
| ----------------------------------------- | ----------------------------------- | ------------------ |
| `STANDARD_PROFICIENCY_IDS` (constant)     | `class_editor.rs`, `race_editor.rs` | `editor_common.rs` |
| `STANDARD_ITEM_TAGS` (constant)           | `item_editor.rs`, `race_editor.rs`  | `editor_common.rs` |
| `truncate()` (function)                   | `class_editor.rs`, `race_editor.rs` | `editor_common.rs` |
| `filter_valid_proficiencies()` (function) | `class_editor.rs`, `race_editor.rs` | `editor_common.rs` |
| `filter_valid_tags()` (function)          | `item_editor.rs`, `race_editor.rs`  | `editor_common.rs` |

### How Sharing Works

Since each file in `src/bin/` compiles as its own independent crate, standard
`mod` imports don't work. Instead, each binary includes the shared module via
the `#[path]` attribute:

```rust
#[path = "editor_common.rs"]
mod editor_common;
use editor_common::{filter_valid_proficiencies, truncate};
```

A module-level `#![allow(dead_code)]` in `editor_common.rs` suppresses warnings
for items that a particular binary doesn't import (each binary uses a different
subset of the shared module).

### What Each Binary Imports

- **`class_editor.rs`**: `filter_valid_proficiencies`, `truncate`
- **`race_editor.rs`**: `STANDARD_PROFICIENCY_IDS`, `STANDARD_ITEM_TAGS`,
  `truncate`, `filter_valid_proficiencies`, `filter_valid_tags`
- **`item_editor.rs`**: `STANDARD_ITEM_TAGS`, `filter_valid_tags`

### New File

- `src/bin/editor_common.rs` — shared module with SPDX header, `///` doc
  comments on all public items, and its own `#[cfg(test)]` test suite
  (9 tests covering all functions and constants).

### Quality Gates

- `cargo fmt --all` — ✅ no output
- `cargo check --bin class_editor --bin race_editor --bin item_editor` — ✅ 0 errors, 0 warnings
- `cargo clippy --bin class_editor --bin race_editor --bin item_editor -- -D warnings` — ✅ 0 warnings
- `cargo nextest run --bin class_editor --bin race_editor --bin item_editor` — ✅ 57 passed, 0 failed, 0 skipped

## Inventory UI Shared Module Extraction (Complete)

### Overview

Extracted duplicated constants and the `NavigationPhase` enum from three
inventory UI files into a single shared module, eliminating copy-paste
duplication and ensuring visual consistency across all inventory-related
screens.

**Problem**: The following three files contained identical definitions of 10
layout/colour constants and shared the same `NavigationPhase` enum (defined in
`inventory_ui.rs`, re-imported by the other two):

- `src/game/systems/inventory_ui.rs`
- `src/game/systems/merchant_inventory_ui.rs`
- `src/game/systems/container_inventory_ui.rs`

### What Was Extracted

New file: `src/game/systems/inventory_ui_common.rs`

**10 shared constants** (all `pub(crate)`):

| Constant                 | Type            | Value                             |
| ------------------------ | --------------- | --------------------------------- |
| `PANEL_HEADER_H`         | `f32`           | `36.0`                            |
| `PANEL_ACTION_H`         | `f32`           | `48.0`                            |
| `SLOT_COLS`              | `usize`         | `8`                               |
| `GRID_LINE_COLOR`        | `egui::Color32` | `(60, 60, 60, 255)` premultiplied |
| `PANEL_BG_COLOR`         | `egui::Color32` | `(18, 18, 18, 255)` premultiplied |
| `HEADER_BG_COLOR`        | `egui::Color32` | `(35, 35, 35, 255)` premultiplied |
| `SELECT_HIGHLIGHT_COLOR` | `egui::Color32` | `YELLOW`                          |
| `FOCUSED_BORDER_COLOR`   | `egui::Color32` | `YELLOW`                          |
| `UNFOCUSED_BORDER_COLOR` | `egui::Color32` | `(80, 80, 80, 255)` premultiplied |
| `ACTION_FOCUSED_COLOR`   | `egui::Color32` | `YELLOW`                          |

**1 shared enum**: `NavigationPhase` (`SlotNavigation`, `ActionNavigation`)

### What Stayed File-Local

Each file retains constants unique to its screen:

- **`inventory_ui.rs`**: `EQUIP_STRIP_H`, `ITEM_SILHOUETTE_COLOR`
- **`merchant_inventory_ui.rs`**: `STOCK_ROW_H`, `STOCK_ITEM_COLOR`, `STOCK_EMPTY_COLOR`, `BUY_COLOR`, `SELL_COLOR`
- **`container_inventory_ui.rs`**: `CONTAINER_ROW_H`, `CONTAINER_ITEM_COLOR`, `TAKE_COLOR`, `STASH_COLOR`

### How Sharing Works

- `inventory_ui_common.rs` is registered as `pub mod inventory_ui_common` in
  `src/game/systems/mod.rs`.
- Each consumer imports the shared constants and `NavigationPhase` via
  `use super::inventory_ui_common::{ ... }` (or the equivalent `crate::` path).
- `inventory_ui.rs` adds `pub use super::inventory_ui_common::NavigationPhase`
  so that existing external imports
  (`use antares::game::systems::inventory_ui::NavigationPhase`) continue to
  resolve without changes — preserving backward compatibility for integration
  tests and doc-tests.
- Doc-test import paths on `MerchantNavState` and `ContainerNavState` were
  updated to point at `inventory_ui_common::NavigationPhase`.

### Quality Gates

- `cargo fmt --all` — ✅ no output
- `cargo check --lib --all-features` — ✅ 0 errors
- `cargo clippy --lib --all-features -- -D warnings` — ✅ 0 warnings
- `cargo nextest run --lib --all-features` (inventory/merchant/container tests) — ✅ 168 passed, 0 failed
- `cargo test --doc --all-features` (NavigationPhase, MerchantNavState, ContainerNavState, InventoryNavigationState) — ✅ 4 passed, 0 failed

## Shared Test Character Factory Module (Complete)

### Overview

Consolidated duplicate `create_test_character()` helper functions that were
copy-pasted across 9+ test modules into a single shared module at
`src/test_helpers.rs`. This eliminates ~100 lines of duplicated code and
establishes a single source of truth for test character construction.

### Problem

Many test modules defined their own nearly-identical factory functions for
creating `Character` instances. These included:

- `src/application/save_game.rs` — `fn create_test_character(name: &str)`
- `src/domain/combat/engine.rs` — `fn create_test_character(name: &str, speed: u8)`
- `src/domain/magic/casting.rs` — `fn create_test_character(class_id: &str, level: u32, sp: u16, gems: u32)`
- `src/domain/party_manager.rs` — `fn create_test_character(name: &str, race_id: &str, class_id: &str)`
- `src/domain/progression.rs` — `fn create_test_character(class_id: &str)`
- `tests/combat_integration.rs`, `tests/innkeeper_party_management_integration_test.rs`, `tests/recruitment_integration_test.rs`

All followed the same pattern: call `Character::new(...)` with `Sex::Male`,
`Alignment::Good`, and usually `"human"` race / `"knight"` class defaults.

### What Was Created

**New file**: `src/test_helpers.rs`

A `#[cfg(test)]`-gated module containing a `factories` submodule with four
public factory functions:

| Function                         | Signature                                                  | Purpose                                    |
| -------------------------------- | ---------------------------------------------------------- | ------------------------------------------ |
| `test_character`                 | `(name: &str) -> Character`                                | Basic character with human/knight defaults |
| `test_character_with_class`      | `(name: &str, class_id: &str) -> Character`                | Character with a specific class            |
| `test_character_with_race_class` | `(name: &str, race_id: &str, class_id: &str) -> Character` | Character with specific race and class     |
| `test_dead_character`            | `(name: &str) -> Character`                                | Character with `hp.current = 0`            |

All functions include full `///` doc comments with argument descriptions and
usage examples.

### What Was Updated

**Modules that fully adopted shared factories** (local factory removed):

| File                           | Old factory                                      | Replaced with                                             |
| ------------------------------ | ------------------------------------------------ | --------------------------------------------------------- |
| `src/application/save_game.rs` | `create_test_character(name)`                    | `test_helpers::factories::test_character`                 |
| `src/domain/party_manager.rs`  | `create_test_character(name, race_id, class_id)` | `test_helpers::factories::test_character_with_race_class` |

**Modules that delegate to shared factories** (local wrapper kept):

| File                        | Old factory                       | Now delegates to                              |
| --------------------------- | --------------------------------- | --------------------------------------------- |
| `src/domain/progression.rs` | `create_test_character(class_id)` | `test_character_with_class("Test", class_id)` |

The local wrapper was kept because the original factory hardcoded the name
`"Test"` and accepted only `class_id`, so all existing call sites
(`create_test_character("knight")`) continue to work without modification.

**Modules left unchanged** (specialized factories with extra setup):

| File                                                   | Reason                                                    |
| ------------------------------------------------------ | --------------------------------------------------------- |
| `src/domain/combat/engine.rs`                          | Sets `stats.speed.current` after construction             |
| `src/domain/magic/casting.rs`                          | Sets `level`, `sp.current`, and `gems` after construction |
| `tests/combat_integration.rs`                          | Sets `hp.current` and `hp.base` after construction        |
| `tests/innkeeper_party_management_integration_test.rs` | Integration test, not in `src/`                           |
| `tests/recruitment_integration_test.rs`                | Integration test, not in `src/`                           |

These specialized factories could adopt delegation in a future pass.

**Module registration**: Added `#[cfg(test)] pub mod test_helpers;` to
`src/lib.rs`.

**Unused import cleanup**: Removed the now-unused `Character` import from
`save_game.rs` tests, and removed `Alignment`/`Sex` imports from
`party_manager.rs` and `progression.rs` tests (now encapsulated in the shared
factories).

### Quality Gates

- `cargo fmt --all` — ✅ no output
- `cargo check --all-targets --all-features` — ✅ 0 errors, 0 warnings
- `cargo clippy --all-targets --all-features -- -D warnings` — ✅ 0 warnings
- `cargo nextest run --all-features` — ✅ 3979 passed, 0 failed, 8 skipped

## UI Helpers Shared Module Extraction (Complete)

### Overview

Created `src/game/systems/ui_helpers.rs` to consolidate duplicated Bevy UI
text-styling and image-creation patterns found across combat, HUD, menu, and
game-log systems. This extraction follows Phase 3, Section 3.5 of the cleanup
plan.

### Problem

Two categories of boilerplate were repeated heavily across multiple system files:

1. **Text style tuples** — The exact pattern
   `TextFont { font_size: X, ..default() }, TextColor(Color::WHITE)` appeared
   23+ times across four files, with two dominant combinations:

   - `font_size: 16.0` + `Color::WHITE` — **13 occurrences** (combat 3,
     menu 9, hud 1)
   - `font_size: 14.0` + `Color::WHITE` — **10 occurrences** (combat 3,
     hud 6, ui 1)

2. **Blank RGBA image creation** — `initialize_mini_map_image` and
   `initialize_automap_image` in `hud.rs` contained identical 10-line
   `Image::new_fill(…)` blocks differing only in the size parameter and
   resource type.

### What Was Extracted

**New file: `src/game/systems/ui_helpers.rs`**

| Item                            | Kind                         | Purpose                                                                     |
| ------------------------------- | ---------------------------- | --------------------------------------------------------------------------- |
| `BODY_FONT_SIZE`                | `const f32 = 16.0`           | Semantic name for the most common body-text size                            |
| `LABEL_FONT_SIZE`               | `const f32 = 14.0`           | Semantic name for label / legend text size                                  |
| `text_style(font_size, color)`  | `fn → (TextFont, TextColor)` | Returns a bundle pair that Bevy accepts as a nested tuple inside `spawn(…)` |
| `create_blank_rgba_image(size)` | `fn → Image`                 | Creates a square transparent RGBA8 texture for map backing images           |

Seven unit tests cover value correctness, image dimensions, data length, and
all-zeros initialization.

### What Was Updated

| File                         | Changes                                                                                                                                                                                                                                                                                                                                                          |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/game/systems/mod.rs`    | Added `pub mod ui_helpers;`                                                                                                                                                                                                                                                                                                                                      |
| `src/game/systems/hud.rs`    | Replaced 2 `Image::new_fill` blocks in `initialize_mini_map_image` / `initialize_automap_image` with `create_blank_rgba_image`; replaced 7 text-style tuples with `text_style(…)` calls; replaced 3 identical image-creation blocks in test setup functions; removed unused `RenderAssetUsages`, `TextureDimension`, `TextureFormat` imports from non-test scope |
| `src/game/systems/combat.rs` | Replaced 6 text-style tuples (3× `LABEL_FONT_SIZE`, 3× `BODY_FONT_SIZE`)                                                                                                                                                                                                                                                                                         |
| `src/game/systems/menu.rs`   | Replaced 9 text-style tuples (all `BODY_FONT_SIZE` + `Color::WHITE`)                                                                                                                                                                                                                                                                                             |
| `src/game/systems/ui.rs`     | Replaced 1 text-style tuple (game-log header)                                                                                                                                                                                                                                                                                                                    |

### Patterns Investigated But Not Extracted

- **`font_size: 10.0` + `Color::WHITE`** — only 4 occurrences (under the 5+
  threshold)
- **`font_size: 12.0` + `Color::srgb(0.9, 0.9, 0.9)`** — only 3 occurrences,
  all within `menu.rs`
- **`font_size: 18.0` + `Color::WHITE`** — only 2 occurrences in `combat.rs`;
  menu uses a different constant (`BUTTON_TEXT_COLOR`)
- **Rest UI text styles** — every occurrence in `rest.rs` uses unique `srgba`
  colors (gold, green, grey tints); no duplicates met the 5+ threshold

### Quality Gates

- `cargo fmt --all` — ✅ no output
- `cargo check --all-targets --all-features` — ✅ 0 errors, 0 warnings
- `cargo clippy --all-targets --all-features -- -D warnings` — ✅ 0 warnings
- `cargo nextest run --all-features` — ✅ 3987 passed, 0 failed, 8 skipped

## RonDatabase Helper (`database_common.rs`) (Complete)

### Overview

Created `src/domain/database_common.rs` — a shared module containing generic
helpers that encapsulate the "parse RON → iterate → check duplicates → insert
into HashMap" pattern repeated across 16 database implementations.

### Problem

Every database type (`ItemDatabase`, `MonsterDatabase`, `SpellDatabase`,
`ClassDatabase`, `RaceDatabase`, `ProficiencyDatabase`, `CharacterDatabase`,
`CreatureDatabase`, `FurnitureDatabase`, `MerchantStockTemplateDatabase`, and
6 SDK databases) contained nearly identical `load_from_file` /
`load_from_string` methods with the same parse-iterate-dedup-insert loop.

### What Was Created

`src/domain/database_common.rs` exposes two public functions:

| Function                                                   | Purpose                                                                                       |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `load_ron_entries(ron_data, id_of, dup_err, parse_err)`    | Deserializes a RON string into `Vec<T>`, inserts into `HashMap<K, T>` with duplicate checking |
| `load_ron_file(path, id_of, dup_err, read_err, parse_err)` | Reads a file then delegates to `load_ron_entries`                                             |

Both are fully generic over entity type `T`, key type `K`, and error type `E`.
Callers pass closures for ID extraction and error construction, keeping each
database's error type untouched.

### What Was Updated

**Domain databases** (9 files updated):

- `items/database.rs` — `ItemDatabase`: both methods → `load_ron_file` / `load_ron_entries`
- `combat/database.rs` — `MonsterDatabase`: both methods
- `magic/database.rs` — `SpellDatabase`: both methods
- `classes.rs` — `ClassDatabase`: `load_from_string` only (preserves `validate()`)
- `races.rs` — `RaceDatabase`: `load_from_string` only (preserves `validate()`)
- `proficiency.rs` — `ProficiencyDatabase`: `load_from_string` only
- `visual/creature_database.rs` — `CreatureDatabase`: `load_from_string` only
- `world/furniture.rs` — `FurnitureDatabase`: `load_from_string` only
- `world/npc_runtime.rs` — `MerchantStockTemplateDatabase`: `load_from_string` only

**SDK databases** (6 types in `sdk/database.rs`):

- `SpellDatabase`, `MonsterDatabase`, `QuestDatabase`, `ConditionDatabase`,
  `DialogueDatabase`, `NpcDatabase` — all `load_from_file` methods refactored

**Skipped**: `CharacterDatabase` — has per-entity `definition.validate()?`
that does not fit the generic helper pattern.

### Behavioral Improvement

SDK databases now **reject duplicate IDs** at load time (returning an error)
instead of silently overwriting. This catches data bugs earlier.

### Quality Gates

- `cargo fmt --all` — ✅ no output
- `cargo check --all-targets --all-features` — ✅ 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — ✅ 0 warnings
- `cargo nextest run --all-features` — ✅ 3987 passed, 0 failed, 8 skipped

## Trivial `Default` Implementations Replaced with `#[derive(Default)]` (Complete)

### Overview

Replaced 17 manual `impl Default for X { fn default() -> Self { Self::new() } }`
blocks with `#[derive(Default)]` on the struct definitions. Each `new()` method
was verified to produce the same result as the derived `Default` (all fields
set to their type's default: empty collections, 0, None, etc.).

### What Was Changed

**`src/domain/character.rs`** (9 types):

| Type              | Fields                      | Why Safe                                     |
| ----------------- | --------------------------- | -------------------------------------------- |
| `AttributePair`   | `base: u8`, `current: u8`   | `new(0)` ≡ `{ 0, 0 }` ≡ Default              |
| `AttributePair16` | `base: u16`, `current: u16` | Same reasoning                               |
| `Condition`       | tuple struct `(u8)`         | `FINE = 0`, `u8::default() = 0`              |
| `Resistances`     | 8 × `AttributePair`         | All `AttributePair::new(0)` ≡ Default        |
| `Inventory`       | `items: Vec<InventorySlot>` | `Vec::new()` ≡ Default                       |
| `Equipment`       | 7 × `Option<ItemId>`        | All `None` ≡ Default                         |
| `SpellBook`       | 2 × `HashMap`               | Already used `Default::default()` in `new()` |
| `QuestFlags`      | `flags: Vec<bool>`          | `Vec::new()` ≡ Default                       |
| `Roster`          | 2 × `Vec`                   | `Vec::new()` ≡ Default                       |

**Other domain files** (4 types):

| File                          | Type               | Reason           |
| ----------------------------- | ------------------ | ---------------- |
| `items/database.rs`           | `ItemDatabase`     | `HashMap::new()` |
| `combat/database.rs`          | `MonsterDatabase`  | `HashMap::new()` |
| `magic/database.rs`           | `SpellDatabase`    | `HashMap::new()` |
| `visual/creature_database.rs` | `CreatureDatabase` | `HashMap::new()` |

**Application layer** (`application/mod.rs`, 2 types):

| Type           | Reason                  |
| -------------- | ----------------------- |
| `ActiveSpells` | All 18 `u32` fields = 0 |
| `QuestLog`     | 2 × `Vec::new()`        |

**SDK and campaign loader** (2 types):

| File                 | Type          | Reason                        |
| -------------------- | ------------- | ----------------------------- |
| `sdk/database.rs`    | `NpcDatabase` | `HashMap::new()`              |
| `campaign_loader.rs` | `GameData`    | All fields now derive Default |

### NOT Changed (Intentionally Skipped)

- **`Party`** — `position_index: [true, true, true, false, false, false]` ≠ `[false; 6]`
- **`GameState`** — `time: GameTime::new(1, 6, 0)` differs from Default

All `new()` methods were preserved as named constructors.

### Quality Gates

- `cargo fmt --all` — ✅ no output
- `cargo check --all-targets --all-features` — ✅ 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — ✅ 0 warnings
- `cargo nextest run --all-features` — ✅ 3987 passed, 0 failed, 8 skipped

## Phase 3: Consolidate Duplicate Code — Summary (Complete)

All six sub-tasks from the cleanup plan have been completed:

| Sub-task                   | Deliverable                                                           | Status |
| -------------------------- | --------------------------------------------------------------------- | ------ |
| 3.1 RonDatabase helper     | `src/domain/database_common.rs`; 15 database implementations migrated | ✅     |
| 3.2 CLI editor base        | `src/bin/editor_common.rs`; 3 editors refactored                      | ✅     |
| 3.3 Inventory UI common    | `src/game/systems/inventory_ui_common.rs`; 3 UIs refactored           | ✅     |
| 3.4 Test character factory | `src/test_helpers.rs`; 3 test modules consolidated                    | ✅     |
| 3.5 UI helper functions    | `src/game/systems/ui_helpers.rs`; 25 call sites updated               | ✅     |
| 3.6 Trivial Default impls  | 17 types switched to `#[derive(Default)]`                             | ✅     |

### Final Quality Gates

- `cargo fmt --all` — ✅ no output (all files formatted)
- `cargo check --all-targets --all-features` — ✅ 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — ✅ 0 warnings
- `cargo nextest run --all-features` — ✅ 3987 passed, 0 failed, 8 skipped

### Architecture Compliance

- [x] No architectural deviations from `architecture.md`
- [x] Module placement follows Section 3.2 (domain, application, game, sdk)
- [x] Type aliases used consistently
- [x] Constants extracted, not hardcoded
- [x] RON format used for data files
- [x] All new modules have SPDX headers
- [x] All public items documented with `///` doc comments
- [x] No test references `campaigns/tutorial`
