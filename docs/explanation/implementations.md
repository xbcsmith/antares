# Implementations

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
