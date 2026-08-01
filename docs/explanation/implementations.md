# Implementations

## Dependency Upgrade Sweep (Bevy 0.17 → 0.19 + workspace-wide)

Executed per `docs/explanation/dependency_upgrade_implementation_plan.md`,
sequencing lowest-risk to highest-risk bumps so failures stayed isolated and
bisectable. Motivating trigger: the `block v0.1.6` future-incompatibility
warning, now resolved (the crate is gone from `Cargo.lock`, replaced by
`block2` via Bevy 0.19's Metal backend).

### Phase 1 — Low-Risk Patch/Minor Sweep

Refreshed `Cargo.lock` for all same-major-version dependencies (`serde`,
`serde_json`, `ron`, `thiserror`, `clap`, `flate2`, `tar`, `chrono`,
`tracing`, `tracing-subscriber`, `image`, `bytemuck` in the root crate;
`regex`, `arboard`, `gltf` in `sdk/campaign_builder`). Confirmed `dirs`,
`wayland-client`, `wayland-sys`, `noise`, `tempfile` were already current. No
source changes required.

### Phase 2 — Isolated Major-Version Bumps (Pre-Bevy)

- `rand` 0.9 → 0.10: applied the `Rng`/`RngCore` → `Rng`/`RngExt` trait
  rename across the affected combat/domain files (`use rand::{Rng, RngExt}`).
- `rustyline` 17 → 18: version bump only, no source changes.
- `sha2` 0.10 → 0.11: `Sha256` digest output dropped `LowerHex`; switched
  `calculate_checksum` in `src/sdk/campaign_packager.rs` to manual hex
  formatting (`hash.iter().map(|byte| format!("{byte:02x}")).collect()`).
- `ordered-float` 4 → 5: version bump only, no source changes.

### Phase 3 — Bevy 0.17 → 0.19 Core Engine Upgrade

Pinned `bevy = "0.19"` and `bevy_egui = "0.41"` (version-coupled).

- Text/font system (Cosmic Text → Parley): wrapped every `font_size:` site in
  `FontSize::Px(...)` (`combat.rs`, `hud.rs`, `ui_helpers.rs`, including the
  `UI_FONT_SIZE_*` constants and the `text_style()` helper), and migrated
  `TextFont::font` assignments from `Handle<Font>` to
  `FontSource::Handle(...)` in the custom-font system (`font_handles.rs`,
  `dialogue_visuals.rs`, `hud.rs`, `menu.rs`).
- `AmbientLight` resource split: `src/game/systems/time.rs`'s day/night cycle
  now uses `ResMut<GlobalAmbientLight>`.
- Resources-as-components audit: the one broad `Query<Entity>` in
  `dialogue_visuals.rs` is a single-entity `.get()` lookup, not an iteration,
  so the new resource-backing entities have no effect.
- Remaining compile-error sweep: `bevy_egui` 0.41 / egui 0.35 `Context`-based
  top-level panel API removal, a new `AssetMut` wrapper on `Assets::get_mut`,
  fallible `SystemState::get_mut` in tests, and a `grass_instancing.rs` custom
  render-pipeline rewrite.

Manual in-game visual verification (HUD/combat text sizes, day/night ambient
lighting, combat flow, and the `grass_instancing.rs` GPU pipeline) still
requires a human pass on a real display/GPU — not runnable in a headless
environment.

### Phase 4 — `campaign_builder` egui/eframe Stack Upgrade

- `eframe`/`egui` 0.33 → 0.35: migrated the affected editor modules against
  the two-minor-version API drift (`App::ui()` split, `Panel`/`CentralPanel`
  rewiring).
- `rfd` 0.15 → 0.17 and `tray-icon` 0.19 → 0.24 (macOS): version bumps only,
  no source changes; the macOS tray code path was compiled and checked.
- `egui_autocomplete`: no release compatible with egui 0.35 exists, so the
  dependency was dropped and reimplemented locally as
  `sdk/campaign_builder/src/ui_helpers/autocomplete_widget.rs` (ported from
  the MIT-licensed original with attribution), pulling in `fuzzy-matcher`
  directly.

Manual UI verification (each editor tab, file dialogs, macOS tray icon) still
needs a human pass on a real display.

### Phase 5 — Workspace-Wide Verification

- `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
  clean.
- `cargo nextest run --workspace --all-features`: 8006 passed, 0 failed, 8
  skipped.
- `cargo report future-incompatibilities`: clean; `block v0.1.6` is no longer
  in `Cargo.lock` (replaced by `block2`).

Every direct dependency in the workspace is now on its latest stable version.
The one remaining open item across the whole plan is human, on-display visual
verification of both binaries (`antares` game and `campaign-builder` SDK
tool), which cannot be performed headlessly.

## Codebase Cleanup — Phase 1: Security & Correctness Hardening

Executed per `docs/explanation/codebase_cleanup_plan.md` §1. The goal was to
close every reachable panic and path-traversal/decompression-bomb sink in the
campaign, save, and asset-loading paths, and to stop silently swallowing
errors. Untrusted, filesystem-bound identifiers are now validated at the sink
through one shared helper module before any path is constructed. All four
quality gates pass with zero warnings (`fmt`, `check`, `clippy -D warnings`,
`nextest`), and every touched module's doctests pass.

### Foundation — shared path-security helper (`src/domain/path_security.rs`)

New module, registered in `src/domain/mod.rs` with re-exports. Uses
`thiserror` (`PathSecurityError`) and provides three primitives reused by every
other deliverable below:

- `validate_campaign_relative_path(base, candidate) -> Result<PathBuf, _>` —
  rejects absolute paths and any `..`/root components, then canonicalizes the
  join and asserts it stays inside `base` (containment check).
- `validate_identifier(id)` — allowlist `^[A-Za-z0-9_-]+$` (no separators, no
  `..`, non-empty).
- `validate_filename_component(name)` — accepts exactly one safe path
  component (rejects separators, `..`, absolute paths).

Documented with runnable examples; 10 unit tests cover the traversal,
absolute-path, and empty-input edge cases.

### A — `DiceRoll` panic guard + validation (`src/domain/types.rs`)

- `DiceRoll::roll` now returns `bonus.max(0)` when `sides == 0` instead of
  panicking on the modulo-by-zero in the RNG path.
- Added `DiceRollError` + `DiceRoll::validate()` (rejects `sides == 0`).
- Tests: `sides == 0` no longer panics and `validate()` rejects it.

### B — Dice-roll validation wired into the SDK validator

- `src/sdk/validation.rs`: added `ValidationError::InvalidDiceRoll { context,
  reason }` (+ `severity()` arm → `Severity::Error`), a `validate_dice_rolls()`
  pass that flags `sides == 0` across `monster.attacks`, and a call to it from
  `validate_all()`.
- `src/sdk/error_formatter.rs`: added the matching `InvalidDiceRoll` arm to the
  exhaustive `get_suggestions` match.

### C — Registry `filepath` + campaign id + texture-path sanitization

- `src/domain/visual/creature_database.rs` and
  `src/domain/world/object_mesh.rs`: registry `filepath` joins now route
  through `validate_campaign_relative_path` (reusing the existing
  `ReadError`/`AssetReadError` variants).
- `src/domain/world/landscape.rs`: `validate_texture_paths` now rejects `..`
  components.
- `src/sdk/campaign_loader.rs`: `CampaignLoader::load_campaign` validates the
  id via `validate_identifier` (new `CampaignError::InvalidId`) before joining
  it onto the campaigns dir; `validate_campaign` inherits the guard through it.
  Real ids (`tutorial`, `test_campaign`) remain valid.

### D — Decompression-bomb cap + safe extraction (`src/sdk/campaign_packager.rs`)

- `install_package` validates `campaign_id` via `validate_identifier` plus a
  parent-containment check (new `UnsafeCampaignId`), and extracts entries in a
  streaming loop with a `MAX_UNCOMPRESSED_BYTES = 512 MiB` cap (new
  `ArchiveTooLarge`). Test-only `total_uncompressed_size` helper is
  `#[cfg(test)]`.
- Tests: a fixture archive exceeding the cap is rejected; hand-built tar-gz
  fixtures correctly finish the gzip stream (`tar.into_inner()?` +
  `enc.finish()?`).

### E — Save-file name sanitization (`src/application/save_game.rs`)

- `SaveGameManager::save_path` now returns `Result<PathBuf, SaveGameError>`,
  validating the name as a single safe component (new
  `SaveGameError::InvalidName`). Callers `save`/`load`/`delete` propagate with
  `?`.
- Tests: traversal/separator names return `InvalidName` and write nothing
  outside the saves dir; a normal timestamp name still round-trips.

### F — Error-swallowing `let _ =` discards logged

An audit of the flagged sites found only **two** genuine `Result` discards (the
plan's "6 sites" over-counted: the `inventory_ui.rs` and `spell_casting.rs`
sites discard an `Option`/plain struct, not a `Result`, and were correctly left
unchanged). Both real discards now log via `tracing::warn!(?e, ...)`:
`src/domain/combat/monster_spells.rs` and `src/bin/antares.rs`.

### G — Guarded-Option unwraps converted + graceful saves-dir error

- `src/game/systems/events.rs`, `dialogue.rs`, `menu.rs`, and
  `src/sdk/cli/map_builder.rs`: guarded-`Option` unwraps rewritten as
  `let ... else`.
- `menu.rs`: the `SaveGameManager` startup `expect` is now graceful — it is an
  optional resource with a temp-dir fallback, and all three consumers were
  updated to handle its absence.

### Verification

- `cargo fmt --all` — clean.
- `cargo check --all-targets --all-features` — 0 warnings.
- `cargo clippy --all-targets --all-features -- -D warnings` — clean.
- `cargo nextest run --all-features` — 5407 passed, 8 pre-existing skips, 0
  failed.
- `cargo test --doc` — every touched module's doctests pass (`path_security`,
  `types::DiceRoll`, `save_game`, `campaign_loader`, `campaign_packager`,
  `validation`, `landscape`, `creature_database`, `object_mesh`,
  `monster_spells`).

All new tests and fixtures use `data`/`data/test_campaign` only (never
`campaigns/tutorial`), per Implementation Rule 5.

## Codebase Cleanup — Phase 2: Dead Code & Suppressed-Lint Removal

Executed per `docs/explanation/codebase_cleanup_plan.md` §2. Low-risk deletions
using the compiler + workspace-wide grep as the authority (the "dead" items are
`pub`, so `rustc` does not flag them — each was proven dead by confirming its
only references were its own definition/doc/tests). Net effect: ~1000+ lines
removed, one full module deleted, one dormant feature finished and wired, and
search behavior contract-tested across every SDK editor. All four quality gates
pass on the whole workspace with zero warnings.

### Main crate (`src/`) removals

- **7 stale `#[allow(deprecated)]`** removed from `src/sdk/cli/item_editor.rs`
  (no `#[deprecated]` exists anywhere in the crate).
- **7 dead `pub` types** removed: `ActiveActionHighlight` (`combat.rs`),
  `HpText` (`hud.rs`, keeping the live `HpTextOverlay`), `RecruitmentDialogState`
  (`recruitment_dialog.rs`), `TempleUiRoot` (`temple_ui.rs`), `ItemUseAction`
  (`domain/combat/item_usage.rs`), `SpellCastAction` + `SpellCastResult`
  (`domain/combat/spell_casting.rs`).
- **5 dead systems / spawn helpers** removed: `creature_spawning_system`,
  `spawn_shrub` (keeping `spawn_shrub_with_offset`), `get_or_create_tree_mesh`
  (keeping `get_or_create_tree_mesh_pair`), `tree_mesh_cache_key`,
  `spawn_custom_furniture_mesh_with_rendering`. Orphaned imports were cleaned up;
  a test-only `SpawnCreatureRequest` import was moved into the `#[cfg(test)]`
  module.
- **Dead domain query/accessor fns** removed from `domain/items/database.rs`,
  `domain/character.rs`, and `domain/skill_resolver.rs` (each grep-verified as
  unreferenced; 481 lines net).
- **14 dead `sdk/database.rs` query methods** removed (the plan estimated 12;
  the `get_*_by_name` family had 6 members of which 5 were dead —
  `get_condition_by_name` is live and was kept): `get_spell_by_name`,
  `spells_by_school`, `spells_by_level`, `get_monster_by_name`,
  `undead_monsters`, `monsters_by_experience_range`, `get_quest_by_name`,
  `main_quests`, `repeatable_quests`, `quests_for_level`, `get_dialogue_by_name`,
  `repeatable_dialogues`, `dialogues_for_quest`, `get_npc_by_name`. These were
  unwired dead duplicates of the tested domain-layer query API (kept intact).
- **Item H lint fixes**: removed `#[allow(clippy::only_used_in_recursion)]` on
  `evaluate_conditions` (`dialogue.rs`) — the `db` param is now genuinely used
  by the `SkillCheck` arm, so the lint no longer fires and the stale
  "forward-compat" comment was deleted; removed the spurious
  `#[allow(clippy::needless_pass_by_value)]` on `try_pickup_adjacent_dropped_item`
  (`input/exploration_interact.rs`) — all its params are `Copy`, so the lint
  never fired.

### SDK crate (`sdk/campaign_builder/`) hygiene

- **Test Play removed** (decided: not wiring it): deleted `src/test_play.rs`
  (`TestPlaySession`/`TestPlayConfig` + tests), its `pub mod test_play;`, and the
  three `_test_play_*` fields from `CampaignBuilderApp` (+ `Default`).
- **Dead `EditorRegistry` state removed** (`editor_state.rs`):
  `_quests_search_filter`, `_quests_show_preview`, `_quests_import_buffer`,
  `_quests_show_import_dialog`, `_stock_templates_file` (superseded by
  `QuestEditorState` and `CampaignMetadata.stock_templates_file`); the two
  self-referential tests in `tests/editor_state_tests.rs` were deleted and the
  `_quests_show_preview` assertion dropped.
- **Stale `(future)` search stub removed** from `campaign_editor.rs`: the
  single-campaign `search_filter` field + its `.with_search(...)` toolbar wiring
  (a lone campaign has nothing to filter).
- **Write-only `FileNode._children` removed** (`lib.rs`) along with the now-dead
  recursive `read_directory` method in `campaign_io.rs`.

### Export Wizard — finished and wired (decided: keep + implement)

The dormant `sdk/campaign_builder/src/packager.rs` `ExportWizard` (a guided
multi-step campaign **export/packaging** dialog — not a game character) is now
live:

- Activated the two remaining `// Future / unused fields` on
  `CampaignBuilderApp` by renaming `_export_wizard` → `export_wizard` and
  `_show_export_dialog` → `show_export_dialog` (the block is fully resolved and
  its banner removed).
- Added egui-free, testable methods to `ExportWizard`:
  `populate_files_from_campaign(&mut self, &Path)` and
  `run_export(&mut self, &Path) -> Result<PackageManifest, String>`, the latter
  driving the actual packaging through the Phase 1-hardened main-crate
  `antares::sdk::campaign_packager::CampaignPackager` (no duplicated pack logic).
- Wired a `📦 Export / Package Campaign...` menu entry and a
  `render_export_wizard` `egui::Window` (Validation → FileSelection → Metadata →
  Settings → Exporting → Complete) following the SDK egui rules (unique window
  title, `id_salt` on the file `ScrollArea`, `request_repaint()` on step
  changes, `rfd` save dialog forcing a `.tar.gz` output).
- Added `tests/export_wizard_tests.rs`: a full end-to-end flow packaging the
  `data/test_campaign` fixture and asserting a valid `.tar.gz` + manifest, plus
  error-path and helper tests.

### SDK search — verified and contract-tested across all 18 editors

SDK content search was already functional (per-editor `search_filter`/
`search_query` + inline substring matching over editor-local `Vec`s; the deleted
`sdk/database.rs` query methods were never part of it). To lock the behavior in:
8 editors already exposed a testable `filtered_*` seam with a test; the
remaining 10 inline-only editors (`spells`, `monsters`, `items`, `skills`,
`proficiencies`, `conditions`, `furniture`, `levels`, `stock_templates`,
`creatures`) each gained a minimal pure `filtered_*` method (their render code
now calls it, so there is no duplicate predicate) plus a behavior-asserting
contract test (identity of survivors for empty / matching / non-matching
queries, per SDK Rule 11). `map_editor` was tested against its existing
`build_filtered_maps_snapshot` seam. Net: 11 new search contract tests.

### Regression Clippy gate — deferred to Phase 4 (documented, not skipped)

The plan calls for a `unwrap_used` / `expect_used` / `let_underscore_must_use`
"warn" gate. Measurement shows the main-crate **lib alone** has ≈51 pre-existing
non-test occurrences (≈20 `unwrap`, ≈27 `expect`, 4 `let _`), with more in the
binaries and the SDK crate. Because the mandatory gate is
`cargo clippy --all-targets --all-features -- -D warnings`, enabling these
lints crate-wide now would promote every pre-existing occurrence to a hard error
and **break the mandatory zero-warning gate** (AGENTS.md Rule 3). Converting
~50+ call sites is behavior-changing error-handling work that is explicitly the
scope of **Phase 4 (Error-Handling Consistency & Determinism)** and conflicts
with Phase 2's "no behavior change" criterion. Grandfathering with ~50+
`#[allow(...)]` annotations would itself add the clutter this phase removes.
The gate is therefore sequenced into Phase 4, to be switched on cleanly once the
unwrap/expect debt is cleared. This is a documented, evidence-based decision,
not an omission.

### Verification

- `cargo fmt --all` — clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean
  for **both** `antares` and `campaign_builder`.
- `cargo nextest run --workspace --all-features` — 8030 passed (up from 8013:
  +6 Export Wizard, +11 search contract tests), 8 pre-existing skips, 0 failed.
- Doctests: the full `campaign_builder` doctest suite passes (386); two
  pre-existing stale doctests unrelated to this phase (`visible_innkeepers`,
  `filter_spells_for_class`, broken by earlier `NpcDefinition`/`ClassDefinition`
  field additions) were repaired as hygiene. Main-crate doctests for every
  touched module pass.

All new tests/fixtures use `data`/`data/test_campaign` only, per Rule 5.

## Codebase Cleanup — Phase 3: Stale "Phase N" & Comment Cleanup

Executed per `docs/explanation/codebase_cleanup_plan.md` §3. A comment-only pass
that removed the last of the internal dev-plan scaffolding ("Phase N",
"Phase-N", "Phase N.M") from `src/`, rewording each site to describe *what the
code does* instead of *which plan step introduced it*. No behavior, identifiers,
or string literals changed (with one explicit test-fn rename), so the test count
is unchanged. All four quality gates pass on the whole workspace with zero
warnings.

### What changed

- **71 `Phase N` comments reworded across 30 files.** Module `//! Phase N of
  plan.md` headers were replaced with one-line behavioral summaries (or the plan
  pointer dropped); inline `// Phase-6 path` / `// Phase-7 path` render comments
  now name the actual path they guard — e.g. the per-entity `ExtendedMaterial`
  path vs. the GPU-instanced (`GrassInstanceBatch`) path — so the comment stays
  useful without referencing a plan that no longer exists.
- **One plan-named test renamed** in `src/domain/world/landscape.rs`:
  `test_test_campaign_phase1_landscape_mesh_fixture_integrity` →
  `test_test_campaign_landscape_mesh_fixture_integrity` (still runs and passes).
- **`inventory_ui.rs` placeholder comments** (4 sites) normalized to
  `// resolved to the focused slot when the action is executed`, clarifying the
  intent instead of reading as unfinished work.
- **Legitimate `Phase`-named identifiers preserved** (Bevy render API surface,
  not dev-plan scaffolding): `NavigationPhase`, `RenderPhase`, `PhaseItem`,
  `BinnedRenderPhaseType` were verified present and left untouched.

### Verification

- `grep -rInE '\bPhase[ _-]?[0-9]' src` — **zero** matches (case-sensitive
  success criterion met).
- `grep -rIniE 'phase[ _-]?[0-9]' src` — **zero** matches (case-insensitive,
  including the renamed test).
- `cargo fmt --all -- --check` — clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean
  for **both** `antares` and `campaign_builder`.
- `cargo nextest run --workspace --all-features` — 8030 passed, 8 skipped,
  0 failed (identical to pre-Phase-3; changes are comment-only).
- Targeted doctests on edited modules pass; all reworded prose was outside code
  fences, so no doctest behavior changed.

## Phase 4: Error-Handling Consistency & Determinism

Executed per `docs/explanation/codebase_cleanup_plan.md` §4. Goal: one boundary
error type at the domain↔Bevy seam, no cross-layer error-name ambiguity, a
seeded deterministic RNG restoring the reproducible-gameplay architecture
guarantee, and a Clippy regression gate that forbids new panicking `unwrap`/
`expect` and ignored `#[must_use]` results in non-test code.

### Item F — colliding error enums renamed (layer-qualified)

- `sdk::game_config::ConfigError` → `GameConfigError`;
  `sdk::tool_config::ConfigError` → `ToolConfigError`.
- `sdk::campaign_loader::CampaignError` → `CampaignLoadError` (the **domain**
  `CampaignError` is a different type and was left unchanged).
- `sdk::validation::ValidationError` → `CampaignValidationError` (the **domain**
  `ValidationError` was left unchanged).
- Binary generators: `generate_foliage_textures::GeneratorError` →
  `FoliageGeneratorError`; `generate_normal_map::GeneratorError` →
  `NormalMapGeneratorError`.
- Fixed the one stale reference the rename exposed in the separate
  `campaign_builder` workspace crate
  (`sdk/campaign_builder/src/campaign_io.rs` matched
  `validation::ValidationError::InvalidStartingInnkeeper`). This regression was
  invisible to `cargo check --all-targets` because that does **not** compile
  workspace members without `--workspace`; caught by the workspace clippy gate.

### Central `GameError` + `report_err!`

- New `src/error.rs` at the crate root (placed there, not under `domain/`, to
  avoid a layering violation) defines `GameError`, aggregating the library
  module errors via `#[error(transparent)] #[from]`. Registered as
  `pub mod error;` + `pub use error::GameError;` in `lib.rs`.
- `#[macro_export] report_err!` provides three forms — `(err)` (tracing only);
  `(writer, err)`; `(writer, category, err)` — so Bevy systems (which cannot
  return `Result`) route domain errors into a `GameLogEvent` **and**
  `tracing::error!` uniformly. Integrated into the dialogue buy/sell failure
  arms as the reference call site. 6 unit tests cover the `#[from]` conversion
  paths and each macro form.
- Note on `#[error(transparent)]`: `source()` forwards to the *inner* error's
  source (a leaf error yields `None`), which the tests assert accordingly.

### thiserror migration

- Migrated the two genuine manual `Display`/`Error` implementations to
  `#[derive(thiserror::Error)]`: `DialogueValidationError`
  (`sdk/dialogue_editor.rs`) and `QuestValidationError` (`sdk/quest_editor.rs`).
- Plan §4.1 also named `domain/types.rs`, `domain/character.rs`,
  `domain/combat/monster.rs`, `domain/items/types.rs`, and `game/systems/ui.rs`.
  On inspection those `Display` impls are **value formatters** (`GameTime`,
  `Item`, `AttributePair`, `MonsterCondition`, `LogEntry`), not `Error` types,
  so converting them to `thiserror` would be incorrect; they were correctly left
  as-is. Documented here per the "explain WHY your code differs" rule.

### Seeded `GameRng` (Item L) — deterministic gameplay restored

- New `src/game/resources/game_rng.rs`: `GameRng` is a Bevy `Resource` wrapping
  a seed (`u64`) + `StdRng`. API: `from_seed`, `from_entropy`, `seed`, `reseed`,
  `rng() -> &mut StdRng`, and `fallback_std_rng()` for Option-parameter systems
  in minimal test apps. `Default` = entropy. Registered in
  `src/game/resources/mod.rs`.
- `GameState` gained a persisted `rng_seed: u64` field (`#[serde(default)]`),
  populated by a new `generate_rng_seed()` (non-zero random) in both `new()` and
  `new_game()` — this is the save-schema change from §4.3.
- **Threading (production gameplay boundaries only):** domain functions already
  took `&mut rng`; the holes were fresh `rand::rng()` calls at the Bevy seam.
  Contract: domain/helper fns take `rng: &mut impl rand::Rng` as the last
  parameter; systems take `ResMut<GameRng>` (combat) or
  `Option<ResMut<GameRng>>` + the `fallback_std_rng()` pattern (test-friendly
  systems). Threaded through: combat (`combat.rs` 5 systems + defend/flee),
  spell effects/casting, the combat `engine` turn/round/DOT chain, exploration
  spells, `rest.rs` `process_rest`, `lock_ui.rs`, `inventory_ui.rs`, and
  `application/mod.rs::move_party_and_handle_events` (the main movement→encounter
  path, plus its `exploration_movement.rs` → `input.rs` caller chain).
- **App wiring:** `bin/antares.rs` inserts `GameRng::from_seed(game_state.rng_seed)`
  before adding plugins; `CombatPlugin` calls `init_resource::<GameRng>()` (a
  no-op when a seeded instance already exists, so the ~81 combat test apps get a
  resource and never panic on the non-optional `ResMut<GameRng>` param). A new
  `sync_game_rng_seed` system in `MenuPlugin` reseeds `GameRng` whenever
  `GameState::rng_seed` diverges from the resource seed — the single cheap
  per-frame comparison keeps save/load reproducible without threading the seed
  through the 9 `handle_button_press`/`load_game_operation` call sites.
- Cosmetic/tooling RNG left on `rand::rng()` by design: procedural grass
  placement (`advanced_grass.rs`) and the `name_generator` SDK authoring tool
  are outside the combat/encounter determinism guarantee.
- Adding the `GameRng` param pushed two combat systems over Clippy's
  7-argument limit; both received `#[allow(clippy::too_many_arguments)]` with a
  "Bevy system, inherent param count" justification.

### Determinism test (§4.4)

- New `tests/combat_determinism_test.rs` drives a fixed, combat-representative
  roll sequence (1d20 attacks, 2d6+1 damage, per-class HP dice, fizzle checks,
  raw range rolls) against `GameRng` and asserts: same seed → identical trace;
  different seeds → divergent traces; `reseed` to the original seed rewinds the
  stream (the exact guarantee the save/load path relies on).

### Non-test `unwrap`/`expect`/`let _` debt + regression Clippy gate

- Enabled `#![warn(clippy::unwrap_used, clippy::expect_used,
  clippy::let_underscore_must_use)]` with `#![cfg_attr(test, allow(...))]` at the
  crate roots of the `antares` lib, all four binaries, and the
  `campaign_builder` crate. Test code (unit tests, fixtures) is exempt.
- Resolved every pre-existing non-test occurrence (~53 in the main lib, 6 in the
  `antares` bin, 1 in `antares_sdk`, ~33 in `campaign_builder`): each is either
  refactored to real error handling (e.g. `autocomplete_widget` `is_some_and`,
  `landscape_editor` `if let Err(e)`) or carries a tightly-scoped
  `#[allow(...)]` with a concrete, code-based justification comment explaining
  why the panic is unreachable or the discard is intentional (guarded indices,
  compile-time-embedded assets, never-poisoned log-file mutexes, best-effort
  history/clipboard/cleanup I/O, side-effect-capturing `run_export`).
- The Export Wizard `let _ = wizard.run_export(dir)` was verified correct:
  `run_export` records success/failure into its own
  `export_error`/`progress_message` fields, which the UI surfaces, so the
  returned manifest is intentionally discarded (annotated, not refactored).

### Verification

- `cargo fmt --all -- --check` — clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean
  for both `antares` and `campaign_builder`, with the three new restriction
  lints active and test code exempted.
- `cargo nextest run --workspace --all-features` — **8043 passed, 8 skipped,
  0 failed** (up from 8030; +3 determinism tests, plus the newly-compiling
  `campaign_builder` fix).

## Phase 5 — Shared UI Helpers, RON-Loader & Cleanup Consolidation

Foundational duplicate-code consolidation for Phase 5. New shared helpers plus
adoption across UI screens, SDK databases, the campaign loader, and combat
cleanup systems.

### Shared egui UI helpers (`game::systems::ui_helpers`)

- **Palette constants** `UI_TITLE_COLOR` (204,217,255), `UI_HINT_COLOR`
  (140,140,166), `UI_HEADER_COLOR` (179,204,255) — the single source of truth
  for the per-screen `TITLE_COLOR`/`HINT_COLOR`/`*_HEADER_COLOR` duplicates that
  previously lived in `character_sheet_ui`, `spellbook_ui`, and
  `skill_training_ui`.
- **`title_bar_with_hints(ui, title, &[&str])`** — the canonical Rule 6 title bar
  (heading + right-aligned `UI_HINT_COLOR` hints + trailing separator). Adopted
  in `spellbook_ui` and `skill_training_ui`.
- **`format_gold(u32)`** — comma-grouped thousands formatter, promoted from
  `merchant_inventory_ui` (whose local copy + tests were removed).
- **`three_column(ui, left_w, right_w, min_center, left, center, right)`** — the
  Rule 6 three-column scaffold (reads `available_size()` before `ui.horizontal`,
  gives each column an explicit `allocate_ui` rect of full `col_h`, computes
  center width, draws separators, returns each column closure's output). Adopted
  in `spellbook_ui`, `skill_training_ui`, and `character_sheet_ui`
  (`render_single_view`), retiring the most fragile UI failure mode.

`skill_training_ui`'s hint colour was unified from (160,160,120) to the shared
`UI_HINT_COLOR`. `character_sheet_ui`'s single-view title bar keeps its
interactive **Party Overview / Next / Prev** buttons (they mutate
`GameMode::CharacterSheet`) rather than adopting the hints-only
`title_bar_with_hints`, so no on-screen interaction was lost; it uses the shared
palette and `three_column`.

### SDK RON databases routed through `impl_ron_database!`

Extended the `impl_ron_database!` macro (`domain::database_common`) with an
optional **`missing_ok:` arm** that treats a missing file as `Ok(empty)` instead
of an I/O error, then routed the six hand-written single-`HashMap` loaders
(`SpellDatabase`, `MonsterDatabase`, `QuestDatabase`, `ConditionDatabase`,
`DialogueDatabase`, `NpcDatabase`) in `sdk::database` through it. This preserves
their historical empty-on-missing contract (asserted by existing tests) while
deleting the duplicated read+parse+dedup wrappers and the now-unused
`load_ron_entries` import. `MapDatabase` (directory loader) is unchanged.

### Campaign loader optional-file collapse (`domain::campaign_loader`)

Added `load_optional_ron<T>(rel) -> Result<Option<T>, CampaignError>` (pure RON
deserialize; `Ok(None)` when absent) and `load_optional_registry(...)` (exists-
check + error wrapping for asset-resolving registry databases). Collapsed five
near-identical loaders (`load_item_meshes`, `load_furniture_meshes`,
`load_landscape_meshes`, `load_object_meshes`, `load_wind_config`); loaders whose
`load_from_file` does extra work (index rebuild, list-parse with dup detection,
`validate_definition_ids`, or campaign/base fallback) were left with an inline
note explaining why.

### Combat cleanup helpers (`game::systems::combat`)

Added `despawn_all<T: Component>`, `combat_exited(&GameMode) -> bool`, and
`reset_on_combat_exit<R: Resource + Default>`. Refactored the eight combat-exit
cleanup/reset systems to use them (three despawn loops → `despawn_all`, all
`matches!(…Combat…)` guards → the shared predicate, one full reset →
`reset_on_combat_exit`), preserving each system's signature, guard polarity, and
place in the plugin schedule.

### Verification (Phase 5 — helpers)

- `cargo fmt --all` — clean.
- `cargo check --all-targets --all-features` — clean.
- `cargo clippy --all-targets --all-features -- -D warnings` — clean.
- `cargo nextest run --all-features` — **5415 passed, 8 skipped, 0 failed**.
- `cargo test --doc` (`ui_helpers`, `database_common`) — passed.

## Phase 5 — `too_many_arguments` Consolidation (Item J, UI/combat helpers)

Refactored the *plain helper* functions that carried
`#[allow(clippy::too_many_arguments)]` into params/context structs and dropped
the attribute. Idiomatic Bevy **systems** (whose parameters are `SystemParam`
dependency injection — `Commands`, `Query`, `Res`/`ResMut`, `Message*`) were
intentionally left untouched: bundling their DI params into a plain struct would
not compile as a system, so those `#[allow]`s remain documented false positives.

### Refactored plain helpers

- `character_sheet_ui::render_single_view` (9 → 3 args): read-only inputs grouped
  into a new `SingleViewParams<'a>` struct (`party_len`, `focused_index`,
  `campaign_config`, `level_db`, `content_db`, `full_portrait_id`,
  `portrait_key`). `ui: &mut egui::Ui` and `global_state: &mut GlobalState` stay
  separate because they are `&mut` and awkward to bundle. Struct destructured at
  the top so the body is byte-identical.
- `combat::dispatch_combat_action` (9 → 5 args): mutable resource refs grouped
  into `CombatActionState<'a>` (`target_sel`, `action_menu_state`,
  `ranged_pending`, `spell_panel_state`, `item_panel_state`); the two
  `&mut Option<MessageWriter<…>>` writers stay separate to avoid nested-lifetime
  borrow-checker friction.
- `combat::confirm_attack_target` (7 → 5 args): mutable target-selection refs
  grouped into `TargetConfirmState<'a>` (`target_sel`, `action_menu_state`,
  `ranged_pending`); `attack_writer`/`ranged_writer` kept separate.
- `combat::dispatch_item_button` (10 → 6 args): mutable resource refs grouped
  into `ItemDispatchState<'a>` (`item_panel_state`, `pending_item`,
  `party_target_state`, `target_sel`, `action_menu_state`); `content:
  &GameContent` and `use_item_writer` kept separate.

All four helpers now pass `clippy -D warnings` without `#[allow]`. Every new
struct carries `///` docs. Behavior is byte-identical (same events emitted, same
UI); each struct is destructured at the function head so the existing bodies and
all call sites (production + unit tests) were mechanically updated only.

### Deliberately left `#[allow(clippy::too_many_arguments)]` (Bevy systems)

`combat.rs`: `update_spell_selection_panel`, `handle_spell_button_interaction`,
`apply_spell_selection`, `update_item_selection_panel`,
`handle_item_button_interaction`, `update_combat_ui`, `combat_input_system`,
`select_target`, `handle_attack_action`, `handle_use_item_action`,
`execute_monster_turn`, `handle_combat_victory` — all `SystemParam` DI systems;
their argument lists are Bevy's dependency injection and must remain individual
params.

### Verification (Phase 5)

- `cargo fmt --all` — clean.
- `cargo check --all-targets --all-features` — clean.
- `cargo clippy --all-targets --all-features -- -D warnings` — clean.
- `cargo nextest run --all-features character_sheet_ui combat` — **408 passed,
  0 failed**.

## Phase 5 — Split-Inventory Overlay Consolidation (merchant + container)

Removed duplicated scaffolding shared by the merchant buy/sell overlay
(`merchant_inventory_ui.rs`) and the container take/stash overlay
(`container_inventory_ui.rs`), centralising it in
`game::systems::inventory_ui_common`. These are **split** (two-panel) screens, so
their existing half-width split-panel geometry was preserved (not converted to
Rule-6 three-column layout).

### Shared `format_gold`

`merchant_inventory_ui::format_gold` (an exact duplicate of the comma-grouping
helper now living in `ui_helpers::format_gold`) and its four `test_format_gold_*`
unit tests were deleted. `merchant_inventory_ui.rs` now
`use`s `crate::game::systems::ui_helpers::format_gold`; the `render_merchant_top_bar`
call site is unchanged. No coverage was lost — the identical tests already exist
in `ui_helpers.rs`, and nothing outside the merchant module imported the old
function.

### Helpers added to `inventory_ui_common.rs` (all `pub(crate)`)

- `SLOT_NAV_HINT: &str` — the single slot-navigation hint string both overlays
  display during `NavigationPhase::SlotNavigation`. Each screen keeps its own
  distinct `ActionNavigation` hint (Sell/Buy vs. Take/Stash cycling).
- `render_character_strip(ui, party, active_char_idx, id_prefix)` — the
  active-character selector strip. The former `render_merchant_character_strip`
  and `render_container_character_strip` were byte-identical except the
  `push_id` salt prefix, so the shared fn takes an `id_prefix` and builds
  `format!("{id_prefix}_{i}")`. Callers pass `"merch_char_btn"` /
  `"cont_char_btn"`, keeping every widget-id salt byte-identical to before. Both
  original strips are purely visual (button click responses discarded; character
  switching is driven by number keys in the input systems), so the merge is
  behaviour-preserving.
- `split_panel(ui, left, right)` — reproduces the
  `available`/`half_w = (available.x - 8.0)/2.0`/`ui.horizontal`/`item_spacing.x = 8.0`
  scaffold and calls `left(ui, half_w)` then `right(ui, half_w)`. Both
  `*_ui_system`s adopt it; each panel's existing `ui.push_id("<salt>", …)` block
  moved unchanged into the corresponding closure. The panel height is sampled as
  `ui.available_size().y` immediately before the call (same UI state
  `split_panel` reads) and captured in each closure, so `size` remains
  `egui::vec2(half_w, panel_h)` exactly as before. The two closures write to
  disjoint message writers, so no borrow-check dispatch-after-return workaround
  was needed.

### Plain-helper params-struct refactors (dropped `#[allow(clippy::too_many_arguments)]`)

Four plain render helpers had their argument lists grouped into a borrowing
params struct, destructured at the function head so the bodies stay identical:

- `merchant_inventory_ui::render_character_sell_panel` (9 → 1 arg) →
  `CharacterSellPanelParams<'a>`.
- `merchant_inventory_ui::render_merchant_stock_panel` (8 → 1 arg) →
  `MerchantStockPanelParams<'a>`.
- `container_inventory_ui::render_character_stash_panel` (8 → 1 arg) →
  `CharacterStashPanelParams<'a>`.
- `container_inventory_ui::render_container_items_panel` (8 → 1 arg) →
  `ContainerItemsPanelParams<'a>`.

The Bevy **systems** that also carry the allow
(`merchant_inventory_ui_system`, `container_inventory_ui_system`,
`container_inventory_action_system`) were left untouched — their argument lists
are `SystemParam` dependency injection (false positives).

### Tests

- Added `test_merchant_keyboard_navigation_phase_transitions` and
  `test_container_keyboard_navigation_phase_transitions`, each driving the real
  input system inside a minimal Bevy `App` through the full flow: Enter starts
  slot nav, a second Enter enters action mode, Esc cancels, Tab switches panels,
  arrows move the selection (and cycle container action buttons), and a number
  key resets to a character. All fixtures are built in-memory (no
  `campaigns/tutorial` reference).
- All pre-existing merchant/container unit tests still pass unchanged.

### Verification (Phase 5 — inventory)

- `cargo fmt --all` — clean.
- `cargo check --all-targets --all-features` — clean.
- `cargo clippy --all-targets --all-features -- -D warnings` — clean.
- `cargo nextest run --all-features merchant_inventory_ui container_inventory_ui
  inventory_ui_common` — **66 passed, 0 failed**.
- `cargo test --doc --all-features inventory` — **79 passed, 0 failed**.
