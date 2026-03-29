# SDK Codebase Cleanup Plan

## Overview

This plan addresses technical debt, dead code, duplicate patterns, inconsistent
error handling, and development-phase artifacts accumulated across the Antares
SDK Campaign Builder codebase (`sdk/campaign_builder/`). The analysis identified
**~130 phase references in source code**, **9 blanket crate-level lint
suppressions**, **28 `#[allow(clippy::too_many_arguments)]` suppressions**,
**~30 `eprintln!` calls in production code**, **~30 public functions returning
`Result<(), String>`**, **duplicate validation type hierarchies**, **~4,300
lines of duplicate code** across 7 major patterns, and a **12,312-line god
object** in `lib.rs`. The cleanup is organized into six phases, ordered by risk
(lowest first) and impact (highest first).

We do not care about backwards compatibility.

**Upstream dependency**: The Game Codebase Cleanup Plan and Game Feature
Completion Plan will run before this plan. Key upstream changes that affect the
SDK:

- **`food` field removal** (Game Cleanup Phase 1.3) — eliminates the ~21
  `#[allow(deprecated)]` suppressions on `Item` construction in the SDK.
- **`RonDatabase` trait/macro** (Game Cleanup Phase 3.1) — the SDK's RON
  load/save boilerplate in `lib.rs` should adopt whatever shared abstraction
  the game engine creates.
- **Typed error migration** (Game Cleanup Phase 4.6) — the game engine will
  move from `Result<(), String>` to `thiserror` enums; the SDK should follow
  the same patterns.
- **Phase reference stripping** (Game Cleanup Phase 2) — the game engine will
  strip all phase references; the SDK must do the same.
- **SDK validation stubs** (Game Feature Phase 4.3–4.5) — `validate_references`
  and `validate_connectivity` will be implemented, affecting
  `advanced_validation.rs` and `validation.rs`.

Phases in this plan are designed to be executed after upstream changes land, so
that the SDK cleanup is not blocked by or conflicting with game engine work.

## Current State Analysis

### Existing Infrastructure

- **Codebase size**: 107,880 lines across 62 source files in
  `sdk/campaign_builder/src/`, plus 8,840 lines in 17 integration test files
  under `tests/`.
- **Test coverage**: Every non-trivial source file has an inline `#[cfg(test)]`
  module. `lib.rs` alone contains 430 test functions (~5,700 lines of inline
  tests).
- **Error handling**: 10 modules define proper `thiserror` error enums
  (`CampaignError`, `AutoSaveError`, `CreatureAssetError`, `EditorError`,
  `IdError`, `ObjError`, `MeshError`, `PaletteError`, `ObjImporterError`,
  `ObjImporterExportError`, `CsvParseError`). Zero `anyhow` usage. Zero
  `todo!()`/`unimplemented!()` macros.
- **UI framework**: Consistent use of `egui`/`eframe` 0.33 with `EditorToolbar`,
  `TwoColumnLayout`, `AutocompleteInput`, and `ActionButtons` helpers in
  `ui_helpers.rs`.
- **Dependency direction**: The SDK is a path-dependent consumer of the main
  `antares` crate (`antares = { path = "../.." }`). It imports domain types but
  does not contribute back.

### Identified Issues

1. **God object**: `CampaignBuilderApp` in `lib.rs` has ~140 fields, ~100
   methods, and the file is 12,312 lines — 12× the recommended maximum.
2. **Blanket lint suppression**: 9 `#![allow(...)]` directives at crate root
   (`lib.rs` L14–22) suppress `dead_code`, `unused_variables`,
   `unused_imports`, and 6 clippy lints across the entire crate, hiding real
   issues.
3. **Duplicate code**: ~4,300 lines of duplicated patterns across 7 categories
   (toolbar handling, list/action dispatch, undo/redo managers, mesh editor
   history, validation types, autocomplete selectors, RON load/save).
4. **Error handling inconsistency**: ~30 public functions return
   `Result<(), String>` instead of typed errors. ~30 `eprintln!` calls in
   production code bypass the SDK's own `Logger`. ~15 `let _ =` calls silently
   drop `Result` values from user-facing save operations.
5. **Phase references**: ~130 phase references in source comments, module docs,
   test section headers, and `README.md`.
6. **Dead code**: 5 `#[allow(dead_code)]` items (all genuinely dead), 2
   `#[ignore]`d skeleton tests, 3 dead test helper functions, 1 deprecated
   panic-only method.
7. **`#[allow(clippy::too_many_arguments)]`**: 28 suppressions across 15 files,
   all on `show()`-family methods that pass 7–10+ mutable state slices.
8. **Duplicate validation types**: `validation.rs` and
   `advanced_validation.rs` define incompatible `ValidationSeverity` and
   `ValidationResult` types with different variant sets and field layouts.
9. **`campaigns/tutorial` violations**: 2 test/source references to
   `campaigns/tutorial` violate Implementation Rule 5.
10. **Stale module documentation**: `lib.rs` module doc (L4–12) claims
    "Phase 2: Foundation UI" and "Placeholder list views" — both factually
    incorrect for the current state of the codebase.
11. **`#[allow(deprecated)]` proliferation**: ~21 suppressions for `Item`
    struct construction due to upstream deprecated `food` field (resolved by
    Game Cleanup Phase 1.3; SDK must update after).

## Implementation Phases

### Phase 1: Remove Dead Code and Fix Lint Suppressions (Low Risk, High Visibility)

Delete provably-dead code, fix trivial clippy suppressions, and remove the
blanket crate-level `#![allow(...)]` directives. No behavioral changes.

**Prerequisite**: Game Cleanup Phase 1.3 (food field removal) must land first
so the `#[allow(deprecated)]` suppressions on `Item` construction can be
removed simultaneously.

#### 1.1 Remove Blanket Crate-Level `#![allow(...)]` Directives

Remove all 9 blanket lint suppressions from `lib.rs` L14–22:

| Suppression | Action |
| --- | --- |
| `#![allow(dead_code)]` | Remove; fix any newly-surfaced dead code warnings |
| `#![allow(unused_variables)]` | Remove; prefix unused params with `_` |
| `#![allow(unused_imports)]` | Remove; delete unused imports |
| `#![allow(clippy::collapsible_if)]` | Remove; collapse or keep per-site `#[allow]` |
| `#![allow(clippy::single_char_add_str)]` | Remove; use `push` instead of `push_str` for single chars |
| `#![allow(clippy::derivable_impls)]` | Remove; replace trivial `Default` impls with `#[derive(Default)]` |
| `#![allow(clippy::for_kv_map)]` | Remove; use `.values()` or `.values_mut()` |
| `#![allow(clippy::vec_init_then_push)]` | Remove; use `vec![...]` literal syntax |
| `#![allow(clippy::useless_conversion)]` | Remove; delete `.into()` / `.from()` on same types |

After removal, run `cargo clippy --all-targets --all-features -- -D warnings`
and fix every newly-surfaced warning. This will likely surface 50–100+ issues
that were previously hidden. Fix them file-by-file.

#### 1.2 Delete Dead Code

| Item | File | Line | Action |
| --- | --- | --- | --- |
| `show_list_mode()` deprecated panic stub | `creatures_editor.rs` | L1331 | Delete method |
| `FileNode.path` field (stored, never read) | `lib.rs` | L821 | Delete field + population code |
| 3 dead test helpers (`create_test_campaign_dir`, `create_test_item_ron`, `create_test_monster_ron`) | `tests/bug_verification.rs` | L303–340 | Delete `mod helpers` |
| 2 `#[ignore]`d skeleton tests | `tests/bug_verification.rs` | L9, L34 | Delete or implement properly |
| Legacy `show_configuration_editor` stub | `lib.rs` | L5559–5565 | Delete if no callers remain |

#### 1.3 Fix Trivial Clippy Suppressions

| Suppression | File | Line | Fix |
| --- | --- | --- | --- |
| `clippy::map_clone` | `ui_helpers.rs` | L160 | Replace `.map(\|s\| s.clone())` with `.cloned()` |
| `clippy::ptr_arg` (3 instances) | `races_editor.rs` L828, L1187; `map_editor.rs` L3563 | Change `Option<&PathBuf>` to `Option<&Path>` |
| `clippy::only_used_in_recursion` | `lib.rs` | L5776 | Convert `show_file_node` to a free function or associated function |

#### 1.4 Remove `#[allow(deprecated)]` After Upstream Food Field Removal

Once Game Cleanup Phase 1.3 removes the `food` field from `Character`/`Party`
and the `Item` struct's deprecated fields, remove all ~21
`#[allow(deprecated)]` suppressions in the SDK:

| File | Approx. occurrences |
| --- | --- |
| `src/items_editor.rs` | 9 |
| `src/lib.rs` | 6 |
| `src/templates.rs` | 2 |
| `src/advanced_validation.rs` | 1 |
| `src/asset_manager.rs` | 1 |
| `src/undo_redo.rs` | 1 |
| `src/ui_helpers.rs` | 1 |

Update all `Item` struct literal construction to match the new field layout.

#### 1.5 Fix `campaigns/tutorial` Violations

| File | Line | Fix |
| --- | --- | --- |
| `src/asset_manager.rs` | L3163 | Change `PathBuf::from("campaigns/tutorial")` to use `data/test_campaign` |
| `tests/map_data_validation.rs` | L6 | Rewrite test to reference `data/test_campaign/data/maps` |

#### 1.6 Testing Requirements

- All existing tests pass after dead code removal.
- `cargo clippy --all-targets --all-features -- -D warnings` produces zero
  warnings.
- No `#![allow(...)]` directives remain at crate root.
- No `campaigns/tutorial` references remain in test or source code.

#### 1.7 Deliverables

- [ ] 9 blanket `#![allow(...)]` directives removed from `lib.rs`
- [ ] All surfaced clippy/compiler warnings fixed
- [ ] 5 dead code items deleted
- [ ] 2 `#[ignore]`d tests resolved (deleted or implemented)
- [ ] 3 trivial clippy suppressions fixed
- [ ] ~21 `#[allow(deprecated)]` suppressions removed (after upstream)
- [ ] 2 `campaigns/tutorial` violations fixed

#### 1.8 Success Criteria

- Zero blanket `#![allow(...)]` at crate root.
- Zero `#[allow(dead_code)]` in SDK source (except upstream `antares` crate).
- Zero `#[allow(deprecated)]` in SDK source.
- Zero `campaigns/tutorial` references in SDK tests or source.
- All quality gates pass.

---

### Phase 2: Strip Phase References (Low Risk, Medium Effort)

Mechanically remove all development-phase references from source comments,
module docs, test section headers, and documentation files. This is a parallel
effort to Game Cleanup Phase 2 but scoped to `sdk/`.

#### 2.1 Strip Phase Prefixes from Module-Level Doc Comments

Replace `//! ... Phase N ...` with descriptive content that explains what the
module does, not when it was built.

| File | Line | Before | After |
| --- | --- | --- | --- |
| `lib.rs` | L4 | `//! Campaign Builder - Phase 2: Foundation UI` | `//! Campaign Builder for Antares SDK` |
| `lib.rs` | L6–12 | Phase 2 feature list | Current feature list (metadata editor, file I/O, validation, etc.) |
| `advanced_validation.rs` | L4 | `//! Advanced Validation Features - Phase 15.4` | `//! Advanced Validation Features` |
| `auto_save.rs` | L4 | `//! Auto-Save and Recovery System - Phase 5.6` | `//! Auto-Save and Recovery System` |
| `campaign_editor.rs` | L8 | `//! Phase 5 - Docs, Cleanup & Handoff:` | Remove line |
| `context_menu.rs` | L4 | `//! Context Menu System - Phase 5.4` | `//! Context Menu System` |
| `creature_undo_redo.rs` | L4 | `//! ... - Phase 5.5` | `//! Creature Editing Undo/Redo Commands` |
| `creatures_manager.rs` | L4 | `//! Creatures Manager for Phase 6` | `//! Creatures Manager` |
| `creatures_workflow.rs` | L4,7 | `//! ... - Phase 5.1` | `//! Creature Editor Unified Workflow` |
| `item_mesh_editor.rs` | L4 | `//! Item Mesh Editor … (Phase 5)` | `//! Item Mesh Editor` |
| `keyboard_shortcuts.rs` | L4 | `//! Keyboard Shortcuts System - Phase 5.3` | `//! Keyboard Shortcuts System` |
| `preview_features.rs` | L4 | `//! Preview Features - Phase 5.2` | `//! Preview Features` |
| `templates.rs` | L4 | `//! Template System - Phase 15.2` | `//! Template System` |
| `undo_redo.rs` | L4 | `//! Undo/Redo System - Phase 15.1` | `//! Undo/Redo System` |
| `ui_helpers.rs` | L31,42,56 | `//! ## Autocomplete System (Phase 1-3)` | `//! ## Autocomplete System` |

#### 2.2 Strip Phase Prefixes from Inline Code Comments

Replace `// Phase N:` section headers with descriptive labels:

| Pattern | Example replacement |
| --- | --- |
| `// Phase 1: Registry Management UI` | `// Registry Management UI` |
| `// Phase 6 trees` | `// Tree variants` |
| `// Phase 5 — ...` | Remove `Phase 5 —` prefix |
| `// Note: For Phase 1 we keep the UI minimal…` | Remove comment or update |

Key files with high phase-reference density:

| File | Approx. references | Notes |
| --- | --- | --- |
| `creatures_editor.rs` | ~15 | Struct field comments, section headers |
| `map_editor.rs` | ~20 | `VisualPreset` names, terrain variant comments |
| `lib.rs` | ~15 | `Default::default()` section comments, `update()` |
| `dialogue_editor.rs` | ~10 | `// Phase 3:` node enhancement comments |
| `conditions_editor.rs` | ~5 | `// Phase 1 additions` comments |

#### 2.3 Strip Phase Prefixes from Test Section Headers

Replace `// Phase N: ...` test section headings with descriptive labels:

| File | Lines | Before | After |
| --- | --- | --- | --- |
| `lib.rs` | L8254+ | `// Phase 3: ...` | `// RON Load/Save Tests` (etc.) |
| `map_editor.rs` | L8107+ | `// Phase 1–7: ...` | `// Event Editor Tests` (etc.) |
| `config_editor.rs` | L1497+ | `// Phase 2/3: ...` | `// Inline Validation Tests` (etc.) |
| `characters_editor.rs` | L3192 | `// Phase 5: Polish and Edge Cases Tests` | `// Polish and Edge Cases Tests` |
| `items_editor.rs` | L1990 | `// Phase 5: Duration-Aware Consumable Tests` | `// Duration-Aware Consumable Tests` |
| `npc_editor.rs` | L4121 | `// Phase 7: stock_template field tests` | `// Stock Template Field Tests` |
| `tray.rs` | L286+ | `// Phase 2/3 tests:` | Remove prefix |

#### 2.4 Strip Phase References from Test Files

| File | Line | Before | After |
| --- | --- | --- | --- |
| `tests/creature_asset_editor_tests.rs` | L4 | `//! Unit tests for Phase 2: ...` | `//! Unit tests for Creature Asset Editor UI` |
| `tests/furniture_customization_tests.rs` | L4 | `//! ... Phase 9:` | Remove phase reference |
| `tests/furniture_editor_tests.rs` | L6 | `//! ... Phase 7:` | Remove phase reference |
| `tests/furniture_properties_tests.rs` | L4 | `//! ... Phase 8:` | Remove phase reference |
| `tests/gui_integration_test.rs` | L7 | `//! ... in Phase 4.` | Remove phase reference |
| `tests/mesh_editing_tests.rs` | L4 | `//! Phase 4: ...` | `//! Advanced Mesh Editing Tools` |
| `tests/template_system_integration_tests.rs` | L4 | `//! ... Phase 3:` | Remove phase reference |
| `tests/ui_improvements_test.rs` | L6 | `//! ... Phase 8 ...` | Remove phase reference |

#### 2.5 Rewrite SDK README.md

The current `README.md` title is "Phase 2: Foundation" and the structure is
phase-centric. Rewrite it as a current-state description of the Campaign
Builder:

- Title: "Antares Campaign Builder"
- Sections: Features, Getting Started, Architecture, Building, Testing
- Remove all phase roadmap content

Also fix `QUICKSTART.md` L74: strip "(NEW in Phase 7.1!)" from the test quest
editing heading.

#### 2.6 Remove Stale Comments

| File | Line | Comment | Action |
| --- | --- | --- | --- |
| `preview_renderer.rs` | L562 | "Phase 5 will use proper 3D rendering" | Remove or update to current state |
| `map_editor.rs` | L2728 | "Removed temporary debug red border" | Delete stale cleanup note |
| `map_editor.rs` | L3800–3802 | "Removed temporary UI debug label/print" | Delete stale cleanup notes |
| `campaign_editor.rs` | L486 | "For Phase 1 we keep the UI minimal…" | Delete — UI is not minimal anymore |
| `lib.rs` | L11 | "Placeholder list views" | Update — editors are fully implemented |

#### 2.7 Testing Requirements

- All tests pass without modification (comment-only changes).
- `cargo fmt`, `cargo check`, `cargo clippy`, `cargo nextest run` all pass.

#### 2.8 Deliverables

- [ ] ~130 phase references stripped from source comments
- [ ] ~8 phase references stripped from test file module docs
- [ ] `README.md` rewritten as current-state documentation
- [ ] `QUICKSTART.md` phase reference removed
- [ ] Stale "temporary"/"placeholder" comments cleaned up

#### 2.9 Success Criteria

- `grep -rn "Phase [0-9]" sdk/campaign_builder/src/` returns zero results.
- `grep -rn "Phase [0-9]" sdk/campaign_builder/tests/` returns zero results.
- `README.md` contains no phase references.
- All quality gates pass.

---

### Phase 3: Unify Validation Types and Fix Error Handling (Medium Risk, High Impact)

Address the most impactful error handling problems: duplicate validation type
hierarchies, `Result<(), String>` returns, production `eprintln!` calls, and
silent `Result` drops.

**Prerequisite**: Game Cleanup Phase 4.6 (typed error migration) should land
first to establish the pattern. Game Feature Phase 4.3–4.5 (validation stubs)
should land first so `advanced_validation.rs` changes don't conflict.

#### 3.1 Unify `ValidationSeverity` and `ValidationResult`

`validation.rs` and `advanced_validation.rs` define incompatible parallel type
hierarchies:

| Type | `validation.rs` | `advanced_validation.rs` |
| --- | --- | --- |
| `ValidationSeverity` | `Error, Warning, Info, Passed` | `Info, Warning, Error, Critical` |
| `ValidationResult` | `severity, category (enum), message` | `severity, category (String), message, details, suggestion` |

**Action**:

1. Create a unified `ValidationSeverity` in `validation.rs` with 5 variants:
   `Critical, Error, Warning, Info, Passed`.
2. Extend `ValidationResult` in `validation.rs` with optional `details:
   Option<String>` and `suggestion: Option<String>` fields.
3. Change `advanced_validation.rs` `category` from `String` to
   `ValidationCategory` enum.
4. Delete the duplicate type definitions from `advanced_validation.rs` and
   re-export from `validation.rs`.
5. Update all call sites in both modules.

#### 3.2 Migrate `Result<(), String>` to Typed Errors

Create domain-specific `thiserror` error enums for editor operations. The ~30
affected functions span 8 modules:

| Module | Functions affected | Suggested error type |
| --- | --- | --- |
| `characters_editor.rs` | `save_character`, `load_from_file`, `save_to_file` | `CharacterEditorError` |
| `classes_editor.rs` | `save_class`, `load_from_file`, `save_to_file` | `ClassEditorError` |
| `conditions_editor.rs` | `apply_condition_edits`, `validate_effect_edit_buffer`, `delete_effect_from_condition`, `duplicate_effect_in_condition`, `move_effect_in_condition`, `update_effect_in_condition` | `ConditionEditorError` |
| `config_editor.rs` | `save_config`, `validate_key_binding`, `validate_config` | `ConfigEditorError` |
| `creature_undo_redo.rs` | `execute()`, `undo()` on 6 command impls (12 fns) | `CreatureCommandError` |
| `creatures_editor.rs` | `sync_preview_renderer_from_edit_buffer`, `write_creature_asset_file` | `CreatureEditorError` |
| `dialogue_editor.rs` | `edit_node`, `save_node`, `delete_node`, `edit_choice`, `save_choice`, `delete_choice`, `save_dialogue`, `add_choice`, `load_from_file`, `save_to_file` | `DialogueEditorError` |
| `item_mesh_editor.rs` | `perform_save_as_with_path`, `execute_register_asset` | `ItemMeshEditorError` |

Each error enum should follow the existing `thiserror` pattern used by
`AutoSaveError`, `CreatureAssetError`, etc. Use `#[error("...")]` with
descriptive messages and `#[from]` where appropriate for `std::io::Error` and
`ron::Error`.

#### 3.3 Replace `eprintln!` with SDK Logger

Replace ~30 `eprintln!` calls in production code with the SDK's existing
`Logger` (from `logging.rs`). Key files:

| File | Approx. occurrences | Context |
| --- | --- | --- |
| `lib.rs` | ~25 | Load function errors, startup info messages |
| `characters_editor.rs` | ~3 | Portrait load and persist errors |
| `npc_editor.rs` | ~5 | Portrait and persist errors |
| `classes_editor.rs` | ~1 | Save error |
| `auto_save.rs` | ~1 | Backup removal warning |

Pattern: Replace `eprintln!("[ERROR] Failed to ...: {}", e)` with
`self.logger.error(format!("Failed to ...: {}", e))` or equivalent logger
method.

#### 3.4 Fix Silent `Result` Drops on User-Facing Operations

These `let _ =` patterns silently discard errors from operations where the user
expects feedback:

| File | Line | Pattern | Fix |
| --- | --- | --- | --- |
| `lib.rs` | L5470 | `let _ = self.save_campaign()` | Log error + set `status_message` |
| `item_mesh_editor.rs` | L1989 | `let _ = self.perform_save_as_with_path(...)` | Log error + set `status_message` |
| `quest_editor.rs` | L1075 | `let _ = std::fs::create_dir_all(parent)` | Propagate error with `?` or log |
| `lib.rs` | L3298 | `let _ = db.npcs.add_npc(npc.clone())` | Log if error |

Non-critical `let _ =` patterns (file cleanup, intentional drops) may remain
with an explanatory comment.

#### 3.5 Fix Production `panic!`

| File | Line | Context | Fix |
| --- | --- | --- | --- |
| `creatures_editor.rs` | L1339 | `panic!("Deprecated show_list_mode()...")` | Delete the entire dead method (covered by Phase 1.2) |

#### 3.6 Harden Production `unwrap()` Calls

| File | Line | Context | Fix |
| --- | --- | --- | --- |
| `advanced_validation.rs` | L546–547 | `.min().unwrap()` / `.max().unwrap()` on guarded iterator | Use `if let` or add safety comment |
| `characters_editor.rs` | L742 | `.get(id).unwrap().is_some()` | Use `if let Some(entry) = ...` |
| `characters_editor.rs` | L893–895 | Double unwrap on portrait texture | Use `if let Some(Some(texture)) = ...` |

#### 3.7 Add `thiserror::Error` Derive to `MeshError`

`mesh_validation.rs` defines `MeshError` as a plain enum without
`thiserror::Error` derivation — the only such exception in the SDK. Add the
derive and `#[error("...")]` attributes.

#### 3.8 Testing Requirements

- All existing tests pass with updated error types (`String` → typed enums).
- New error enums have unit tests for `Display` formatting.
- Validation type unification does not change validation behavior.

#### 3.9 Deliverables

- [ ] `ValidationSeverity` and `ValidationResult` unified into single types
- [ ] ~30 functions migrated from `Result<(), String>` to typed errors
- [ ] ~30 `eprintln!` calls replaced with SDK `Logger`
- [ ] ~4 silent `Result` drops fixed with logging/error display
- [ ] `MeshError` derives `thiserror::Error`
- [ ] Production `unwrap()` calls hardened

#### 3.10 Success Criteria

- Zero `Result<(), String>` return types in SDK public functions.
- Zero `eprintln!` calls in production code (CLI binary excepted).
- Zero duplicate `ValidationSeverity` or `ValidationResult` definitions.
- All quality gates pass.

---

### Phase 4: Consolidate Duplicate Code (Medium Risk, Highest Line-Count Impact)

Extract shared patterns into reusable abstractions. This phase yields the
largest line-count reduction (~2,860 estimated lines saved) but requires
careful refactoring to ensure behavioral equivalence.

**Prerequisite**: Game Cleanup Phase 3.1 (RonDatabase trait) should land first
so the SDK can adopt the same abstraction for its RON load/save boilerplate.

#### 4.1 Generic Autocomplete Selectors

**Estimated savings: ~800–1,000 lines from `ui_helpers.rs`.**

There are 13 nearly-identical `autocomplete_*_selector` functions in
`ui_helpers.rs` (L2637–L3789) that follow the exact same template. Extract two
generic functions:

1. `autocomplete_entity_selector<T, Id>` — for single-select (replacing 9
   entity-specific functions: item, quest, monster, creature, condition, map,
   npc, race, class).
2. `autocomplete_list_selector<T, Id>` — for multi-select (replacing 4
   list-specific functions: item_list, proficiency_list, tag_list,
   ability_list).

Each current function becomes a 3–5 line thin wrapper calling the generic
with entity-specific name/ID accessor closures.

#### 4.2 Generic Toolbar Action Handler

**Estimated savings: ~600–800 lines across 8 editors.**

`ui_helpers.rs` already defines `handle_file_load`, `handle_file_save`, and
`handle_reload` — but only 3 of 8 editors use them. The other 5 inline the
same logic. Create a generic `handle_toolbar_action<T>()` function that
handles `Save`, `Load`, `Export`, `Reload`, and `None` arms for any type
`T: Serialize + DeserializeOwned` with an ID field. Each editor only supplies
the `New` and `Import` arms.

Affected editors:

| File | Current pattern |
| --- | --- |
| `classes_editor.rs` | Inline toolbar handling (~110 lines) |
| `races_editor.rs` | Inline toolbar handling (~120 lines) |
| `conditions_editor.rs` | Inline toolbar handling (~200 lines) |
| `proficiencies_editor.rs` | Inline toolbar handling (~150 lines) |
| `characters_editor.rs` | Inline toolbar handling (~240 lines) |

Already using shared helpers (verify and keep):

| File | Notes |
| --- | --- |
| `items_editor.rs` | Uses `handle_reload` |
| `spells_editor.rs` | Uses `handle_reload` |
| `monsters_editor.rs` | Uses `handle_reload` |

#### 4.3 Generic List/Action Dispatch

**Estimated savings: ~500 lines across 6 editors.**

Every editor's `show_list` method repeats the same skeleton: filter → sort →
`TwoColumnLayout` → capture selection → dispatch `ItemAction` (Edit, Delete,
Duplicate, Export, None). Extract a generic
`dispatch_list_action<T: Clone + Serialize + HasId>()` function.

Affected files: `spells_editor.rs`, `monsters_editor.rs`, `items_editor.rs`,
`conditions_editor.rs`, `proficiencies_editor.rs`, `dialogue_editor.rs`.

#### 4.4 Generic Undo/Redo Stack

**Estimated savings: ~200 lines across 3 files.**

Three undo/redo managers (`undo_redo.rs`, `creature_undo_redo.rs`,
`item_mesh_undo_redo.rs`) duplicate the same stack-based logic. Create a
generic `UndoRedoStack<C>` struct parameterized on the command type:

- `push(cmd: C)`, `pop_undo() -> Option<C>`, `pop_redo() -> Option<C>`
- `can_undo()`, `can_redo()`, `clear()`
- Configurable `max_history` (default 50)

Each domain-specific manager becomes a thin wrapper that calls `execute` on
the command and delegates stack operations to `UndoRedoStack`.

Similarly, extract a `LinearHistory<Op>` struct for the cursor-based history
pattern shared between `mesh_vertex_editor.rs` (L717–793) and
`mesh_index_editor.rs` (L499–563), saving ~80 additional lines.

#### 4.5 Generic RON Load/Save in `lib.rs`

**Estimated savings: ~500 lines from `lib.rs`.**

The `load_X` / `save_X` method pairs in `lib.rs` (items, spells, conditions,
proficiencies, monsters, furniture, creatures — 7 pairs totaling ~800 lines)
follow an identical pattern. After the upstream `RonDatabase` trait lands (Game
Cleanup Phase 3.1), create generic `load_ron_data<T>()` and
`save_ron_data<T>()` methods on `CampaignBuilderApp`:

- `load_ron_data<T: DeserializeOwned>(file_field, data_target, validator_fn)`
- `save_ron_data<T: Serialize + Ord>(file_field, data_source)`

Each current `load_X`/`save_X` method becomes a 2–3 line call to the generic.

#### 4.6 Testing Requirements

- All existing tests pass after consolidation (or with updated imports).
- New generic functions have their own unit tests.
- Behavioral equivalence verified for every consolidated pattern.

#### 4.7 Deliverables

- [ ] 2 generic autocomplete selector functions created; 13 wrappers refactored
- [ ] Generic toolbar action handler created; 5 editors migrated
- [ ] Generic list/action dispatch created; 6 editors migrated
- [ ] `UndoRedoStack<C>` created; 3 managers refactored
- [ ] `LinearHistory<Op>` created; 2 mesh editors refactored
- [ ] Generic RON load/save methods created; 7 load/save pairs consolidated

#### 4.8 Success Criteria

- Net line-count reduction ≥ 2,000 lines.
- Zero duplicated toolbar action handling across editors.
- Zero duplicated undo/redo stack logic.
- All quality gates pass.

---

### Phase 5: Structural Refactoring — Break Up the God Object (Higher Risk, Long-Term Maintainability)

Address the structural root cause of most SDK maintainability problems:
`lib.rs` at 12,312 lines with `CampaignBuilderApp` holding ~140 fields. This
is the highest-risk phase because it touches the application's central nervous
system.

#### 5.1 Split `ui_helpers.rs` into Sub-Modules

**Current size: 7,734 lines.** Split into focused modules:

| New module | Content | Approx. lines |
| --- | --- | --- |
| `ui/layout.rs` | `EditorToolbar`, `TwoColumnLayout`, `ActionButtons`, `MetadataBadge`, `StandardListItemConfig`, `show_standard_list_item` | ~800 |
| `ui/autocomplete.rs` | `AutocompleteInput`, `AutocompleteCandidateCache`, `make_autocomplete_id`, `load_autocomplete_buffer`, `store_autocomplete_buffer`, `remove_autocomplete_buffer`, all `autocomplete_*_selector` wrappers, all `extract_*_candidates` functions | ~2,500 |
| `ui/file_io.rs` | `ImportExportDialog`, `ImportExportDialogState`, `ImportExportResult`, `load_ron_file`, `save_ron_file`, `handle_file_load`, `handle_file_save`, `handle_reload`, `CsvParseError` | ~600 |
| `ui/attribute.rs` | `AttributePairInput`, `AttributePair16Input` | ~300 |
| `ui/mod.rs` | Re-exports | ~20 |

The remaining validation-display helpers and editor-tab-specific rendering stay
in a slimmed-down `ui_helpers.rs` or move into `ui/widgets.rs`.

#### 5.2 Extract Campaign I/O from `lib.rs`

Move campaign load/save/validation orchestration into a dedicated
`campaign_io.rs` module:

- `load_campaign()`, `save_campaign()`, `save_campaign_as()`
- All `load_X()` / `save_X()` methods (or their generic replacements from
  Phase 4.5)
- `validate_campaign()`, `validate_item_ids()`, `validate_npc_ids()`, etc.
  (~700 lines of validation that duplicates `validation.rs` purpose)
- `load_campaign_directory()`, `create_new_campaign()`
- File tree building (`read_directory`, `update_file_tree`, `show_file_node`)

Estimated extraction: ~2,000–3,000 lines from `lib.rs`.

#### 5.3 Extract Editor State from `CampaignBuilderApp`

Group the ~140 fields of `CampaignBuilderApp` into focused state structs:

| State struct | Fields to move | Current home |
| --- | --- | --- |
| `CampaignData` | All loaded data vectors (`items`, `spells`, `monsters`, `conditions`, `classes`, `races`, `proficiencies`, `characters`, `quests`, `dialogues`, `npcs`, `maps`, `furniture`, `creatures`) | ~30 fields in `CampaignBuilderApp` |
| `EditorUiState` | `current_tab`, `search_query`, `file_load_merge_mode`, `show_preferences`, `show_keyboard_shortcuts`, `status_message`, `file_tree` | ~15 fields |
| `EditorRegistry` | All sub-editor instances (`items_editor`, `spells_editor`, `monsters_editor`, etc.) | ~15 fields |
| `ValidationState` | `validation_results`, `validation_summary`, `advanced_results`, filter state | ~10 fields |

`CampaignBuilderApp` becomes a thin coordinator holding these state structs
and delegating to them. This reduces cognitive load and makes the `update()`
method a dispatch table rather than a monolith.

#### 5.4 Extract Inline Tests from `lib.rs`

Move the ~5,700 lines of inline tests (L6,642–L12,312) from `lib.rs` into
dedicated test files under `tests/`:

| Target file | Test category |
| --- | --- |
| `tests/campaign_io_tests.rs` | Load/save/validation integration tests |
| `tests/editor_state_tests.rs` | Editor state management, tab switching |
| `tests/ron_serialization_tests.rs` | RON serialization round-trip tests |

This immediately cuts `lib.rs` nearly in half.

#### 5.5 Resolve Undo/Redo Parallel State

The current `UndoRedoState` in `undo_redo.rs` duplicates data vectors that
also live in `CampaignBuilderApp`, requiring manual sync via
`sync_state_from_undo_redo` / `sync_state_to_undo_redo`. After Phase 5.3
introduces `CampaignData`, the undo/redo system should operate directly on
`CampaignData` snapshots rather than maintaining a parallel copy.

#### 5.6 Testing Requirements

- All existing tests pass after structural refactoring.
- Module extraction is purely organizational — no behavioral changes.
- Integration tests verify the `update()` dispatch still works correctly.

#### 5.7 Deliverables

- [ ] `ui_helpers.rs` split into `ui/` sub-module directory
- [ ] Campaign I/O extracted from `lib.rs` into `campaign_io.rs`
- [ ] `CampaignBuilderApp` fields grouped into focused state structs
- [ ] ~5,700 lines of inline tests moved to `tests/` files
- [ ] Undo/redo parallel state resolved

#### 5.8 Success Criteria

- `lib.rs` reduced to ≤ 3,000 lines.
- `ui_helpers.rs` eliminated or reduced to ≤ 500 lines (re-export hub).
- `CampaignBuilderApp` has ≤ 30 direct fields (state structs count as 1 each).
- No file in the SDK exceeds 4,000 lines.
- All quality gates pass.

---

### Phase 6: Reduce `too_many_arguments` Suppressions (Medium Risk, Medium Effort)

Address the 28 `#[allow(clippy::too_many_arguments)]` suppressions by
introducing context/parameter structs for editor `show()` methods. This phase
benefits from Phase 5's state struct extraction — once `CampaignData` and
`EditorUiState` exist, they can be passed as single references instead of
8–10 individual `&mut` parameters.

#### 6.1 Introduce `EditorContext` Parameter Struct

Create a shared context struct that bundles the commonly-passed mutable
state:

```
pub struct EditorContext<'a> {
    pub campaign_dir: Option<&'a Path>,
    pub unsaved_changes: &'a mut bool,
    pub status_message: &'a mut String,
    pub file_load_merge_mode: &'a mut bool,
    pub logger: &'a Logger,
    pub asset_manager: &'a mut AssetManager,
}
```

Each editor's `show()` method changes from 8–10 individual parameters to
`(ui, ctx: &mut EditorContext, data: &mut CampaignData)`.

#### 6.2 Introduce `SearchableSelectorContext` for UI Helper Functions

The `searchable_selector_single` and `searchable_selector_multi` functions in
`ui_helpers.rs` take 8+ parameters. Bundle the non-data parameters into a
config struct.

#### 6.3 Prioritized Refactoring Order

Start with the most-suppressed files and work outward:

| Priority | File | Suppressions | Approach |
| --- | --- | --- | --- |
| 1 | `conditions_editor.rs` | 4 | `EditorContext` + extract sub-renderers |
| 2 | `furniture_editor.rs` | 4 | `EditorContext` + extract sub-renderers |
| 3 | `items_editor.rs` | 3 | `EditorContext` |
| 4 | `quest_editor.rs` | 2 | `EditorContext` |
| 5 | `spells_editor.rs` | 2 | `EditorContext` |
| 6 | `ui_helpers.rs` | 2 | `SearchableSelectorContext` |
| 7 | `campaign_editor.rs` | 2 | `EditorContext` |
| 8 | `characters_editor.rs` | 1 | `EditorContext` |
| 9 | `classes_editor.rs` | 1 | `EditorContext` |
| 10 | `dialogue_editor.rs` | 1 | `EditorContext` |
| 11 | `map_editor.rs` | 2 | `EditorContext` + `MapEditorContext` |
| 12 | `monsters_editor.rs` | 1 | `EditorContext` |
| 13 | `proficiencies_editor.rs` | 1 | `EditorContext` |
| 14 | `races_editor.rs` | 1 | `EditorContext` |
| 15 | `asset_manager.rs` | 2 | Parameter struct for `init_data_files` |

#### 6.4 Testing Requirements

- All existing tests pass with updated function signatures.
- `EditorContext` has documentation tests.

#### 6.5 Deliverables

- [ ] `EditorContext` struct created and adopted by all editor `show()` methods
- [ ] `SearchableSelectorContext` created for UI helper functions
- [ ] All 28 `#[allow(clippy::too_many_arguments)]` suppressions eliminated

#### 6.6 Success Criteria

- Zero `#[allow(clippy::too_many_arguments)]` suppressions in SDK source.
- All quality gates pass.
- No editor `show()` method takes more than 5 parameters.

---

## Appendix A: Genuine TODOs and Future Enhancement Gaps

These items are genuine future work identified during the audit. They are NOT
cleanup — they should be tracked separately as feature requests:

| Item | File | Line | Description |
| --- | --- | --- | --- |
| Mesh table editing UI | `creatures_editor.rs` | L2081 | "TODO: Add View/Edit Table buttons for vertices/indices/normals" |
| Monster loot validation | `advanced_validation.rs` | L432 | "Placeholder for future enhancement" — loot table cross-referencing |
| `RewardEditBuffer` quantity | `quest_editor.rs` | L320–327 | Missing `quantity` field; uses `..Default::default()` workaround |
| `AssetReference::Class` variant | `asset_manager.rs` | L1369 | `AssetReference::Item` used as stand-in for Class references |
| Incomplete undo/redo coverage | `undo_redo.rs` | — | No commands for quest edit/delete, conditions, dialogues, NPCs, or maps |

## Appendix B: Files Changed Per Phase

| Phase | Files touched (approx.) |
| --- | --- |
| Phase 1: Dead code + lint fixes | ~30 files (blanket allow removal ripple) |
| Phase 2: Phase references | ~40 files + README.md + QUICKSTART.md |
| Phase 3: Error handling | ~15 files (8 editor modules + validation + lib + logging) |
| Phase 4: Consolidate duplicates | ~20 files (ui_helpers + 8 editors + 3 undo + 2 mesh + lib) |
| Phase 5: Structural refactoring | ~10 files (lib.rs split + ui_helpers split + test moves) |
| Phase 6: too_many_arguments | ~18 files (15 editors + ui_helpers + new context structs) |

## Appendix C: Metrics Summary

| Metric | Current | Target after all phases |
| --- | --- | --- |
| Total SDK source lines | 107,880 | ~100,000 (8% reduction from dedup) |
| Largest file (`lib.rs`) | 12,312 lines | ≤ 3,000 lines |
| Second largest (`map_editor.rs`) | 9,897 lines | ~9,000 (minimal change) |
| Third largest (`ui_helpers.rs`) | 7,734 lines | ≤ 500 (re-export hub) |
| Blanket `#![allow(...)]` | 9 | 0 |
| `#[allow(dead_code)]` | 5 | 0 |
| `#[allow(deprecated)]` | 21 | 0 |
| `#[allow(clippy::too_many_arguments)]` | 28 | 0 |
| Phase references in source | ~130 | 0 |
| `Result<(), String>` returns | ~30 | 0 |
| `eprintln!` in production | ~30 | 0 |
| Duplicate `ValidationSeverity` defs | 2 | 1 |
| `campaigns/tutorial` violations | 2 | 0 |
| `CampaignBuilderApp` fields | ~140 | ≤ 30 (via state structs) |
