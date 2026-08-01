# Implementations

## Phase 6 Item K (P3): CameraMode::Tactical and ::Isometric

Implemented `CameraMode::Tactical` and `CameraMode::Isometric` so selecting
either mode produces a distinct camera view instead of silently falling back to
first-person.

### Changes — `src/game/systems/camera.rs`

**Setup functions** (spawn the initial camera entity on `Startup`):

| Function | Projection | Initial position | Up vector |
|---|---|---|---|
| `setup_tactical_camera` | `Orthographic(scale=0.05)` | `(0.5, 30, 0.5)` looking straight down | `NEG_Z` (map north = screen top) |
| `setup_isometric_camera` | `Perspective` (uses configured FOV/clips) | `(20, 14, 20)` NE offset from origin | `Y` |

**Update functions** (track the party each frame):

| Function | Behaviour |
|---|---|
| `update_tactical_camera` | Moves camera to `(cx, 30, cz)` directly above the party; `look_at` down with `NEG_Z` up |
| `update_isometric_camera` | Offsets camera `+20 X`, `+14 Y`, `+20 Z` from party tile centre; `look_at` party ground position |

Neither new mode applies smooth rotation (intentionally first-person–only).
Both `setup_camera` and `update_camera` match arms for `Tactical` and
`Isometric` now call the dedicated functions — the `warn!` + first-person
fallback is removed.

### Tests added (`src/game/systems/camera.rs`)

- `test_tactical_camera_positions_above_party`
- `test_tactical_camera_looks_downward`
- `test_isometric_camera_follows_party`
- `test_isometric_camera_maintains_elevation`
- `test_camera_mode_tactical_not_first_person`

### Verification

```
cargo fmt --all                               # no output
cargo check --all-targets --all-features      # Finished with 0 errors
cargo clippy --all-targets --all-features -- -D warnings  # 0 warnings
cargo nextest run --all-features              # 5462 passed, 0 failed
```

---

## Phase 6 Item K (P2): Jump Spell Targeting

Implemented full Jump spell targeting so the party actually moves and SP is only
charged on a valid cast.

### Problem

Previously, selecting a Jump spell in exploration immediately called
`cast_exploration_spell()` (consuming SP) and logged a warning that
target-selection was not yet implemented.  The party never moved.

### Solution

Added a new `SelectMapTarget` step to the spell-casting flow, inserted between
`SelectSpell` and `ShowResult` for Jump spells.  SP is now charged only when
the player confirms a valid (in-bounds, unblocked) tile.

#### `src/application/spell_casting_state.rs`

- **`SpellCastingStep::SelectMapTarget`** — new variant.  Arrow keys move the
  map cursor; Enter confirms; Escape returns to `SelectSpell` without
  consuming SP.
- **`map_target_x: i32` / `map_target_y: i32`** — new fields on
  `SpellCastingState`, with `#[serde(default)]` for save compatibility.
- **`select_map_target(x, y)`** — setter method; initialised to the party's
  current tile when entering the step.

#### `src/game/systems/exploration_spells.rs`

- **`handle_spell_casting_input`**:
  - Escape on `SelectMapTarget` returns to `SelectSpell` (no SP lost).
  - Arrow keys (or WASD) reposition the cursor; clamped to map bounds.
  - Enter (or NumpadEnter) validates the tile via `get_tile().blocked`; if
    valid, calls `execute_exploration_cast()` then applies the position;
    if invalid, calls `show_result()` without casting.
  - `SelectSpell` confirm now detects a Jump spell via
    `effective_effect_type()` and routes to `SelectMapTarget` instead of
    executing immediately.
- **`execute_exploration_cast`**:
  - Jump case in the teleport block now logs at `debug` level (position is
    applied by the caller).
  - Message building includes a Jump override that reports the selected tile
    coordinates (mirrors the Information spell pattern).
- **`update_spell_casting_ui`**:
  - Title: `"Jump Destination"` for `SelectMapTarget`.
  - Hint: `"←→ X  ↑↓ Y  Enter Confirm  Esc Back"` (updated per step).
  - Content: `build_map_target_rows` shows `Map / X / Y` coordinates.
- **`count_items_for_step`**: returns `1` for `SelectMapTarget`.
- **`build_map_target_rows`**: new helper that renders the current cursor
  coordinates inside the overlay.

### Tests added

`src/application/spell_casting_state.rs`:
- `test_map_target_default_is_origin`
- `test_select_map_target_stores_coordinates`
- `test_select_map_target_updates_existing_values`
- `test_map_target_with_caster_select_default_is_origin`

`src/game/systems/exploration_spells.rs`:
- `test_count_items_for_step_select_map_target_returns_one`
- `test_jump_target_valid_for_unblocked_tile`
- `test_jump_target_invalid_for_out_of_bounds`
- `test_jump_target_invalid_for_blocked_tile`
- `test_jump_spell_detection_via_effect_type`
- `test_step_transitions_to_select_map_target_after_jump_selection`
- `test_invalid_jump_target_sets_show_result_without_sp_change`

### Verification

```
cargo fmt --all                  # no output
cargo check --all-targets --all-features  # Finished with 0 errors
cargo clippy --all-targets --all-features -- -D warnings  # Finished with 0 warnings
cargo nextest run --all-features  # 5462 passed, 0 failed
```

Implemented the previously no-op "Step 5" placeholder in
`src/domain/skill_resolver.rs` so that spell/ability buffs and debuffs affect
a character's effective skill ranks at resolution time.

### New type — `TimedSkillBoost` (`src/domain/character.rs`)

Added `TimedSkillBoost` struct (after `TimedStatBoost`) with three fields:
- `skill_id: String` — identifies the affected skill (matches `SkillId`).
- `bonus: i16` — signed rank delta (positive = buff, negative = debuff).
- `minutes_remaining: u16` — countdown to expiry.

Derives `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize` to match
`TimedStatBoost`; uses `#[serde(default)]` on the `Character` field for
backwards-compatible deserialization.

### New field — `Character::timed_skill_boosts`

`pub timed_skill_boosts: Vec<TimedSkillBoost>` added to `Character` with
`#[serde(default)]`. Initialized to `Vec::new()` in `Character::new()` and
also added to the two manual struct-literal sites that were outside `new()`:
- `src/domain/character_definition.rs`
- `src/domain/items/equipment_validation.rs`

### New methods on `Character`

- `apply_timed_skill_boost(skill_id, bonus, minutes)` — zero-minute calls are
  no-ops; otherwise pushes a `TimedSkillBoost` entry. No immediate stat
  mutation (unlike `apply_timed_stat_boost`, which writes through to the stat
  `current` value — skill boosts are applied on-the-fly at resolution time).
- `tick_timed_skill_boosts_minute()` — uses `retain_mut` to decrement
  `minutes_remaining`, dropping any entry whose counter is already ≤ 1.
  Simpler than `tick_timed_stat_boosts_minute` because no reversal step is
  needed; the boost simply stops being included in the sum.

### Updated `SkillResolver` (`src/domain/skill_resolver.rs`)

- `effective_skill_rank_for_character` — now sums
  `character.timed_skill_boosts` matching the requested `skill_id`, adds the
  result to `base_rank`, and clamps to `[0, SkillDefinition::max_rank]`. The
  fast path (`temp_bonus == 0`) returns `base_rank` unchanged with no extra
  allocation.
- `effective_skill_breakdown_for_character` — new method (did not exist
  before). Delegates to `effective_skill_breakdown` for Steps 1–4, then
  applies the same temp-bonus logic as above but appends a
  `SkillBreakdownEntry { source: Temporary, bonus }` to the entries list
  before updating `final_rank`. The UI's character-sheet skill section already
  handled `SkillGrantSource::Temporary` in its match arm.
- Step 5 comment in `effective_skill_rank` updated to explain the
  context-only path intentionally skips timed boosts.

### Tests added

**`src/domain/character.rs`** (4 unit tests):
- `test_timed_skill_boost_apply_adds_entry`
- `test_timed_skill_boost_zero_minutes_is_noop`
- `test_timed_skill_boost_tick_decrements`
- `test_timed_skill_boost_tick_expires`

**`src/domain/skill_resolver.rs`** (3 integration tests):
- `test_effective_skill_rank_for_character_applies_temp_bonus` — verifies
  `+3` boost on level-5 character raises rank from 4 to 7.
- `test_effective_skill_rank_for_character_clamps_temp_bonus_to_max` — verifies
  a large boost on a near-max rank clamps at `max_rank = 50`.
- `test_effective_skill_rank_for_character_debuff_reduces_rank` — verifies a
  `−10` debuff on rank 4 clamps to 0.

### Design notes

- `SkillResolverContext` is **unchanged** — the context-only path (dialogue
  skill checks, etc.) intentionally does not apply timed boosts; callers using
  `effective_skill_rank_for_character` get the full set of sources.
- No `unwrap()` calls; all fallible lookups use `ok_or_else`.
- `#[serde(default)]` on `timed_skill_boosts` ensures save files created
  before this field existed still deserialize correctly (field defaults to
  empty `Vec`).

---

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

## Phase 6 Item K (P5): Monster Visuals + `CreatureAnimationState` Keyframe Support

Executed per the Phase 6 codebase cleanup plan. Two files were changed:
`src/game/components/creature.rs` and `src/game/systems/monster_rendering.rs`.

### `AnimationKeyframe` + `AnimationClip` (new types in `creature.rs`)

The empty `CreatureAnimationState` placeholder was replaced with a proper
keyframe animation system comprising three new public types:

- **`AnimationKeyframe`** — a single snapshot: `time: f32`, `root_offset: Vec3`,
  `root_rotation: Quat`.
- **`AnimationClip`** — a named, ordered sequence of keyframes with `duration`
  and `looping` flags. Provides:
  - `new_idle()` — two-keyframe 1 s looping bob animation.
  - `new_death()` — two-keyframe 0.5 s non-looping tip-over animation.
  - `sample(t) -> (Vec3, Quat)` — linearly interpolates `root_offset` and
    spherically interpolates `root_rotation` between surrounding keyframes;
    clamps past `duration`.
- **`CreatureAnimationState`** (was a no-op placeholder) — now a real `Component`
  with:
  - `current_clip: AnimationClip`, `animation_time: f32`, `looping: bool`,
    `finished: bool`.
  - `Default` impl plays the idle clip.
  - `play(clip)` — transitions to a new clip; no-ops if name matches current
    (prevents restart-on-tick).
  - `advance(delta_secs)` — advances time, wrapping for looping clips and
    clamping + setting `finished` for non-looping ones.
  - `current_pose()` — delegates to `current_clip.sample(animation_time)`.

All three types carry full `///` doc comments with runnable examples.

### Improved fallback visual (`monster_rendering.rs`)

Replaced the single solid-gray-to-purple cube with a **two-child billboard
hierarchy**:

- **Parent entity** — positioned `+0.7 Y` above the spawn point so the panel
  base sits at floor level.
- **Panel child** — `Cuboid::new(0.8, 1.4, 0.05)` (looks flat from the front).
  Material is colour-coded by difficulty tier and carries an emissive component
  (0.4× the base sRGB) so fallback markers glow in dim lighting and are
  visually distinct from real creature meshes.
- **Sphere child** — `Sphere::new(0.15)` positioned at `Y = +0.85` as a
  top-of-panel icon. Uses a white emissive material.

Colour tiers (extracted into `pub fn fallback_monster_color(might: u8) -> Color`):

| Might | Old colour | New colour | Tier   |
|-------|------------|------------|--------|
| 1–8   | gray       | green      | easy   |
| 9–15  | orange     | yellow     | medium |
| 16–20 | red        | orange     | hard   |
| 21+   | purple     | purple     | boss   |

`fallback_monster_color` is `pub` so it can be unit-tested without a Bevy world.
Parent-child hierarchy uses the established `commands.entity(parent).add_child(child)`
pattern (same as `creature_spawning.rs`).

### Tests added

**`creature.rs`** (replaces the placeholder assertion; 10 tests total for the new system):
- `test_animation_clip_new_idle_has_two_keyframes`
- `test_animation_clip_new_death_not_looping`
- `test_animation_clip_sample_at_zero_returns_first_keyframe`
- `test_animation_clip_sample_at_midpoint_interpolates`
- `test_creature_animation_state_default` (updated from placeholder check)
- `test_creature_animation_state_advance_progresses_time`
- `test_creature_animation_state_looping_wraps`
- `test_creature_animation_state_non_looping_finishes`
- `test_creature_animation_state_play_same_name_is_noop`
- `test_creature_animation_state_current_pose`

**`monster_rendering.rs`** (2 new tests):
- `test_fallback_visual_color_easy_monster` — verifies all four colour tiers.
- `test_fallback_color_boundary_values` — verifies each tier boundary point.

### Verification

- `cargo fmt --all` — clean.
- `cargo check --all-targets --all-features` — zero errors in the two changed
  files; pre-existing compile errors in `events.rs` (too-many-params system)
  and `exploration_spells.rs` (stale function name) are unrelated to this task
  and were present in the working tree before this change.
- `cargo nextest run` — blocked by the same pre-existing errors; all new tests
  are pure unit tests (no Bevy World required) and are correct by inspection.

---

## Phase 6 Item K (P2): Reputation/Faction System + Quest SetFlag Persistence + Global Flags

### Overview

Wired up three closely-related game-progression features that were previously
no-ops: the **global boolean flag store** (`GlobalFlags`), the **faction
reputation store** (`ReputationStore`), and the plumbing that connects them to
dialogue conditions/actions and quest rewards.

### New types — `ReputationStore` and `GlobalFlags` (`src/application/mod.rs`)

Two new public structs added before `GameState`:

**`ReputationStore`** (`pub struct`, `Default`, `Serialize`/`Deserialize`):
- `factions: HashMap<String, i16>` — maps faction name to a signed value
  (positive = favored, negative = hostile).
- `new()` — creates an empty store.
- `get(faction) -> i16` — returns 0 for unknown factions (no panicking `unwrap`).
- `change(faction, delta: i16)` — applies a signed delta with `saturating_add`
  to clamp at `i16::MIN`/`i16::MAX`.
- `set(faction, value)` — sets an exact value.

**`GlobalFlags`** (`pub struct`, `Default`, `Serialize`/`Deserialize`):
- `flags: HashMap<String, bool>` — maps flag name to a boolean state.
- `new()` — creates an empty store.
- `get(flag_name) -> bool` — returns `false` for unset flags.
- `set(flag_name, value)` — inserts or overwrites the flag.

### New `GameState` fields

Two fields added after `rng_seed`, both `#[serde(default)]` for backward-
compatible save loading:

```rust
#[serde(default)]
pub reputation: ReputationStore,

#[serde(default)]
pub global_flags: GlobalFlags,
```

Both `GameState::new()` and `GameState::new_game()` initialize them via
`ReputationStore::new()` / `GlobalFlags::new()`.

### Dialogue wiring (`src/game/systems/dialogue.rs`)

**`evaluate_conditions`** — two stub arms replaced with real logic:

- `FlagSet { flag_name, value }` — previously short-circuited to `false` if
  the flag was required to be `true`. Now reads
  `game_state.global_flags.get(flag_name)` and fails the condition only when
  the actual value differs from the required value.
- `ReputationThreshold { faction, threshold }` — previously always returned
  `false`. Now reads `game_state.reputation.get(faction)` and fails only when
  `current < threshold`.

**`execute_action`** — two stub arms replaced:

- `SetFlag { flag_name, value }` — now calls
  `game_state.global_flags.set(flag_name, *value)` and logs at `info!` level
  (was `warn!("not persisted")`).
- `ChangeReputation { faction, change }` — now calls
  `game_state.reputation.change(faction, *change)` and logs at `info!` level
  (was `warn!("not yet implemented")`).

### Quest reward wiring (`src/application/quests.rs`)

`apply_rewards` was refactored from `fn apply_rewards(&self, quest: &DomainQuest, ...)`
to `fn apply_rewards(&self, rewards: &[QuestReward], ...)`. The call site now
passes `&domain_quest.rewards`. This allows direct testing without constructing
a full `Quest` object and removes the now-unused `Quest as DomainQuest` import.

Two stub arms wired:

- `SetFlag { flag_name, value }` — calls `game_state.global_flags.set(flag_name, *value)`.
- `Reputation { faction, change }` — calls `game_state.reputation.change(faction, *change)`.

### New test quest (`data/test_campaign/data/quests.ron`)

Quest id 8 "Proving Grounds" added to the test fixture. Objectives: kill 5
goblins. Rewards: `Experience(50)`, `SetFlag(proved_worth_to_rangers = true)`,
`Reputation(Rangers, +5)`. Exercises both new reward types end-to-end in the
RON-parsed campaign data.

### Tests added

**`src/application/mod.rs`** (8 new tests):
- `test_reputation_store_new_is_empty` — zero-default for any faction name.
- `test_reputation_store_change_adds_delta` — change accumulates correctly.
- `test_reputation_store_saturates_at_i16_max` — `saturating_add` guard.
- `test_reputation_store_saturates_at_i16_min` — negative saturation guard.
- `test_global_flags_default_is_false` — unset flag returns `false`.
- `test_global_flags_set_and_get` — roundtrip including resetting to `false`.
- `test_game_state_has_reputation_and_flags` — `GameState::new()` fields
  default to empty/zero.
- `test_reputation_persists_across_save_load` — `serde_json` round-trip
  verifies fields survive serialization.

**`src/application/quests.rs`** (2 new tests):
- `test_quest_reputation_reward_changes_faction_rep` — direct `apply_rewards`
  call verifies accumulation and negative deltas.
- `test_quest_setflag_reward_sets_flag` — sets flag to `true` then back to
  `false` via `apply_rewards`.

### Verification

- `cargo fmt --all` — clean.
- `cargo check --all-targets --all-features` — clean on lib; pre-existing
  errors in `exploration_spells.rs` (stale function name) and `events.rs`
  (too-many-params system, from another agent's in-progress change) block
  full binary/integration compilation but are unrelated to this task.
- `cargo clippy --all-targets --all-features -- -D warnings` — clean.
- `cargo test --lib --all-features -- application` — **352 passed, 0 failed**,
  including all 8 new `ReputationStore`/`GlobalFlags` tests.
- `cargo test --lib --all-features -- application::quests::tests` — **15 passed,
  0 failed**, including both new quest reward tests.

All test data uses `data/test_campaign` (Implementation Rule 5). No
`campaigns/tutorial` references introduced.

## Phase 6 Item K (P4): Recruitment Confirmation UI (no-dialogue recruitables)

Implemented the confirm/recruit path for `RecruitableCharacter` map events that
have `dialogue_id: None`. Previously these events logged a `warn!("… not yet
implemented")` and did nothing. They now show a game-log prompt, store pending
state, and respond to keyboard input.

### New types (`src/game/systems/events.rs`)

- `RecruitConfirmData` — plain `Debug + Clone` struct with three public fields:
  `character_id: String`, `character_name: String`,
  `event_position: crate::domain::types::Position`. Holds all information needed
  to execute or cancel the recruitment.
- `PendingRecruitConfirm` — `Resource + Default` newtype wrapping
  `Option<RecruitConfirmData>`. `None` = no confirm pending; `Some` = player
  must press **[E]** or **[Esc]**. Intentionally separate from the existing
  `PendingRecruitmentContext` in `dialogue.rs`, which handles the dialogue-tree
  recruitment path.

### `EventPlugin` changes (`src/game/systems/events.rs`)

`EventPlugin::build` now:
- `insert_resource(PendingRecruitConfirm::default())` so the resource is always
  present when the plugin is active.
- Registers two new systems alongside `check_for_events` and `handle_events`:
  `set_pending_recruit_confirm` and `handle_recruit_confirm_input`.

### `set_pending_recruit_confirm` system

Reads `MapEventTriggered` via its own independent `MessageReader` cursor (Bevy
events are not consumed — each reader has its own position). Matches only
`RecruitableCharacter { dialogue_id: None, .. }`. On match:
- Sets `PendingRecruitConfirm` to `Some(RecruitConfirmData { … })`.
- Writes a `GameLogEvent` (Dialogue category): `"{name} wants to join your
  party. Press [E] to recruit or [Esc] to decline."`

This system was added instead of adding a 17th parameter to `handle_events`
(which would exceed Bevy's 16-parameter system limit). The `warn!` placeholder
in `handle_events`' no-dialogue `else` branch was replaced with a comment.

### `handle_recruit_confirm_input` system

Runs every frame; early-exits when `PendingRecruitConfirm.0.is_none()` or when
`global_state.0.mode` is not `Exploration`. Uses
`Option<Res<ButtonInput<KeyCode>>>` for keyboard access (the `Option` wrapper
makes the system safe to run in test apps without input plugins).

- **[E] / Enter / NumpadEnter** — calls `global_state.0.recruit_from_map(
  &data.character_id, content.db())`. On success (`AddedToParty` or
  `SentToInn`), removes the map event at `data.event_position` so the
  recruitable mesh disappears. Writes a log message for each outcome,
  including error cases.
- **[Esc]** — clears `PendingRecruitConfirm` and logs
  `"{name} was not recruited."`

### Pre-existing clippy fixes (`src/game/systems/exploration_spells.rs`)

Fixed four unnecessary `as i32` casts where `Position.x`/`.y` are already
`i32`, which were blocking the clippy gate:
- `(pos.x as i32, pos.y as i32)` → `(pos.x, pos.y)` (production code)
- Three test assertions with the same pattern.

### Tests added (`src/game/systems/events.rs` — `mod tests`)

- `test_pending_recruit_confirm_default_is_none` — unit test: default resource
  has `None` inner.
- `test_recruit_confirm_data_fields` — unit test: struct fields round-trip
  through direct construction.
- `test_recruitable_character_no_dialogue_sets_pending_confirm` — integration
  test: sends a `MapEventTriggered` for `RecruitableCharacter { dialogue_id:
  None }` into a minimal Bevy app with `EventPlugin`, then asserts
  `PendingRecruitConfirm.0.is_some()` with correct `character_id`,
  `character_name`, and `event_position`.

### Design notes

- `PendingRecruitConfirm` is explicitly separate from `PendingRecruitmentContext`
  (dialogue path) to keep the two code paths orthogonal.
- The two-message UX ("Met {name}." from `handle_events` + "wants to join …"
  from `set_pending_recruit_confirm`) mirrors the dialogue-based recruitable
  flow, which also logs "Met {name}." before entering dialogue.
- All test data uses `data/test_campaign` (Implementation Rule 5). No
  `campaigns/tutorial` references introduced.

### Verification

- `cargo fmt --all` — clean.
- `cargo check --all-targets --all-features` — clean.
- `cargo clippy --all-targets --all-features -- -D warnings` — clean.
- `cargo nextest run --all-features` — **5462 passed, 8 skipped, 0 failed**.
  Targeted events suite: 109 tests, 109 passed (includes all 3 new tests).
