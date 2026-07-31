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
