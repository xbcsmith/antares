# Dependency Upgrade Implementation Plan

## Overview

Antares' workspace (`antares` root crate + `sdk/campaign_builder`) has
accumulated version drift across nearly every direct dependency. The
headline item is **Bevy 0.17 → 0.19** (skipping 0.18 entirely — two
migration guides' worth of breaking changes), which is also what forces
`bevy_egui` 0.38 → 0.41. Several other direct dependencies have pending
semver-major releases (`rand`, `rustyline`, `sha2`, `ordered-float`, and the
`egui`/`eframe`/`rfd`/`tray-icon` stack used only by `campaign_builder`).
This plan sequences the upgrade from lowest to highest risk so that
compounding failures stay isolated and bisectable, and lands the Bevy jump
— the one with real gameplay/visual risk — with the codebase otherwise
already on current dependencies.

## Current State Analysis

### Existing Infrastructure

- Root crate (`Cargo.toml`) — game binaries (`antares`, `antares-sdk`,
  texture/normal-map generators), pins `bevy = "0.17"`, `bevy_egui = "0.38"`,
  `rand = "0.9"`, plus ~18 other direct dependencies at various patch levels
  behind latest.
- `sdk/campaign_builder` (workspace member) — a separate `eframe`/`egui`
  0.33 desktop GUI binary that depends on `antares` as a **library only**
  (domain types), not on `bevy` or `bevy_egui`. Its upgrade path is fully
  decoupled from the Bevy jump.
- `docs/explanation/combat_improvements_implementation_plan.md` (recent,
  complete) established the repo's phase/deliverable/success-criteria
  documentation convention this plan follows.
- Prior research (this session) confirmed: Bevy 0.19's Metal backend
  (`wgpu-hal` 29 → `objc2`/`block2`) is what resolves the `block v0.1.6`
  future-incompatibility warning that motivated this plan.

### Identified Issues

Version audit (`cargo info <crate>` run outside the workspace to bypass
local semver pins, so figures reflect true crates.io latest, captured
2026-07-25):

| Crate | Current | Latest | Bump type | Exposure in codebase |
| --- | --- | --- | --- | --- |
| `bevy` | 0.17.2 | 0.19.0 | major ×2 | workspace-wide |
| `bevy_egui` | 0.38.1 | 0.41.1 | major, coupled to bevy | egui-panel UI systems |
| `rand` | 0.9.5 | 0.10.2 | major (`Rng`/`RngCore` rename) | 26 files, 114 call sites |
| `rustyline` | 17.0.2 | 18.0.1 | major | 1 file (`src/sdk/cli/map_builder.rs`) |
| `sha2` | 0.10.9 | 0.11.0 | major | 1 file (`src/sdk/campaign_packager.rs`) |
| `ordered-float` | 4.6.0 | 5.3.0 | major | 1 file |
| `eframe` / `egui` | 0.33 | 0.35.0 | major ×2 | 45 files in `campaign_builder` |
| `rfd` | 0.15 | 0.17.2 | major ×2 | 12 files in `campaign_builder` |
| `tray-icon` | 0.19 | 0.24.1 | major ×5 (macOS only) | 2 files in `campaign_builder` |
| `egui_autocomplete` | 12.0.0 | 12.0.0 | current, but must track `egui` compat | 2 files |
| `serde`, `serde_json`, `ron`, `thiserror`, `clap`, `flate2`, `tar`, `chrono`, `tracing`, `tracing-subscriber`, `image`, `bytemuck`, `regex`, `arboard`, `gltf` | — | patch/minor only | compatible | workspace-wide, no code changes expected |
| `dirs`, `wayland-client`, `wayland-sys`, `noise`, `tempfile` | — | already latest | none | no action |

Bevy 0.19 breaking-change surface confirmed against this codebase's actual
usage (not the full migration guide — only what applies here):

- **Text/font system** (Cosmic Text → Parley): `TextFont::font_size` becomes
  `FontSize::Px(f32)` instead of bare `f32` — **77 `font_size:` sites**
  across `src/game/systems/combat.rs`, `src/game/systems/hud.rs`,
  `src/game/systems/ui_helpers.rs` (including the `UI_FONT_SIZE_*`
  constants and `text_style()` helper from the recent font-consistency
  work). `TextFont::font` becomes `FontSource` instead of `Handle<Font>` —
  affects the custom-font system (`src/game/resources/font_handles.rs`,
  `dialogue_visuals.rs`, `hud.rs`, `menu.rs`).
- **`AmbientLight` resource split**: `src/game/systems/time.rs:145` uses
  `ResMut<AmbientLight>` for the day/night cycle; 0.18 splits this into an
  `AmbientLight` component + `GlobalAmbientLight` resource.
- **Resources-as-components (ECS)**: only 1 broad `Query<Entity>`/`Query<()>`
  pattern found — low exposure, still needs a manual check post-upgrade
  since it can fail silently rather than as a compile error.
- **Not exposed**: no `bevy_scene`/`DynamicScene`/`SceneRoot` usage, no
  `Gizmos::cuboid`, no `InputFocus` field access, no `ExecutorKind`, no
  custom `rodio::Decodable` impls, audio feature already enabled via
  `default-features = true`.
- `rand` 0.10 is also what `bevy_internal`'s dev-dependencies pin for
  0.19.0, so sequencing the standalone `rand` bump before the Bevy bump
  avoids resolving two unrelated breaking changes in the same diff.

## Implementation Phases

### Phase 1: Low-Risk Patch/Minor Sweep

#### 1.1 Update compatible dependencies

Run `cargo update` (no `Cargo.toml` edits needed — all are same-major-version
bumps) for: `serde`, `serde_json`, `ron`, `thiserror`, `clap`, `flate2`,
`tar`, `chrono`, `tracing`, `tracing-subscriber`, `image`, `bytemuck` in the
root [Cargo.toml](Cargo.toml), and `regex`, `arboard`, `gltf` in
[sdk/campaign_builder/Cargo.toml](sdk/campaign_builder/Cargo.toml).

#### 1.2 Skip already-current dependencies

Confirm `dirs`, `wayland-client`, `wayland-sys`, `noise`, `tempfile` need no
action (already at latest per the audit table).

#### 1.3 Verify

Full workspace build after the update; no source changes are expected since
none of these cross a semver-major boundary.

#### 1.4 Testing Requirements

- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (full doctest run excluded per project
  convention)

#### 1.5 Deliverables

- [x] `Cargo.lock` updated for all compatible-bump crates
- [x] No source changes required (or documented if an incidental fix was
      needed)

#### 1.6 Success Criteria

- Workspace builds clean with no new warnings; clippy and test suite pass
  unchanged from pre-upgrade baseline.

### Phase 2: Isolated Major-Version Bumps (Pre-Bevy)

#### 2.1 `rand` 0.9 → 0.10

Update `rand = "0.10"` in [Cargo.toml](Cargo.toml). Apply the `RngCore` →
`Rng`, `Rng` → `RngExt` trait rename across the 13 files importing
`rand::Rng` and the ~30+ `StdRng`/`SeedableRng` combat-RNG seeding sites in
`src/game/systems/combat.rs`. Doing this before Phase 3 means the Bevy bump
doesn't also introduce an unrelated `rand` migration in the same diff.

#### 2.2 `rustyline` 17 → 18

Update in [Cargo.toml](Cargo.toml); fix the single call site in
[src/sdk/cli/map_builder.rs](src/sdk/cli/map_builder.rs) against the 18.x
API.

#### 2.3 `sha2` 0.10 → 0.11

Update in [Cargo.toml](Cargo.toml); fix the single call site in
[src/sdk/campaign_packager.rs](src/sdk/campaign_packager.rs).

#### 2.4 `ordered-float` 4 → 5

Update in [Cargo.toml](Cargo.toml); fix the single affected call site.

#### 2.5 Testing Requirements

- Per-crate: build and run the directly affected module's tests after each
  bump before moving to the next (keeps failures attributable to one crate).
- Full `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` after all four bumps land.

#### 2.6 Deliverables

- [x] `rand` 0.10 migration (trait rename; 12 files actually required changes,
      not all 22 files importing `rand::Rng` called a renamed method)
- [x] `rustyline` 18 migration (no source changes required)
- [x] `sha2` 0.11 migration (`Sha256` digest output type dropped `LowerHex`;
      switched to manual hex formatting in `calculate_checksum`)
- [x] `ordered-float` 5 migration (no source changes required)

#### 2.7 Success Criteria

- All four crates on latest major version; workspace clippy/test suite
  green; no behavior change in combat RNG determinism (existing seeded-RNG
  tests continue to pass unmodified in assertions).

### Phase 3: Bevy 0.17 → 0.19 Core Engine Upgrade

#### 3.1 Bump pins and catalog compile errors

Update `bevy = "0.19"` and `bevy_egui = "0.41"` together in
[Cargo.toml](Cargo.toml) (they're version-coupled — no intermediate
`bevy_egui` release supports mixed versions). Run `cargo check --workspace`
and triage the resulting error list by subsystem before fixing anything, so
the remaining sub-phases can be worked in isolation.

#### 3.2 Text/font system migration

Wrap all 77 `font_size:` sites (and the `text_style()` helper and
`UI_FONT_SIZE_*` constant call sites) in
[src/game/systems/combat.rs](src/game/systems/combat.rs),
[src/game/systems/hud.rs](src/game/systems/hud.rs), and
[src/game/systems/ui_helpers.rs](src/game/systems/ui_helpers.rs) with
`FontSize::Px(...)`. Update `TextFont::font` assignments in the custom-font
system (`src/game/resources/font_handles.rs`, `dialogue_visuals.rs`,
`hud.rs`, `menu.rs`) from `Handle<Font>` to `FontSource::Handle(...)`.

#### 3.3 `AmbientLight` → `GlobalAmbientLight`

Update `src/game/systems/time.rs:145`'s day/night cycle system from
`ResMut<AmbientLight>` to `ResMut<GlobalAmbientLight>`, per the 0.18
component/resource split.

#### 3.4 Resources-as-components audit

Manually review the single broad `Query<Entity>`/`Query<()>` pattern found
in the audit for unintended matches against the new resource-backing
entities; add a `Without<...>`-style filter if needed.

#### 3.5 Remaining compile-error sweep

Work through whatever else `cargo check --workspace` surfaces after 3.2–3.4
(expected candidates from the migration guides, to confirm applicability
against this codebase as they're hit): `System::type_id` →
`System::system_type`, `DefaultErrorHandler` → `FallbackErrorHandler`,
picking-backend feature renames, and any `bevy_egui` 0.38→0.41 API drift not
already covered by 3.1–3.4.

#### 3.6 Testing Requirements

- `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` after each sub-phase (3.2–3.5), not just at the
  end, to keep failures attributable.
- Manual run: launch the tutorial campaign, verify HUD/combat text renders
  at correct sizes with no mixed fonts, verify day/night ambient lighting
  transitions still work, verify combat still resolves normally (RNG,
  targeting, spells/items) after the Phase 2 `rand` migration lands under
  Bevy 0.19.

#### 3.7 Deliverables

- [x] `bevy` 0.19 + `bevy_egui` 0.41 pinned and building
- [x] Text/font system migration (`FontSize::Px`, `FontSource`)
- [x] `AmbientLight`/`GlobalAmbientLight` split applied
- [x] Resources-as-components query audit (the one `Query<Entity>` in
      `dialogue_visuals.rs` is a single-entity `.get()` lookup, not a full
      iteration, so the new resource-backing entities have no effect)
- [x] Remaining compile-error sweep resolved (also required: `bevy_egui`
      0.41/egui 0.35's `Context`-based top-level panel API removal, a new
      `AssetMut` wrapper on `Assets::get_mut`, fallible `SystemState::get_mut`
      in tests, and a `grass_instancing.rs` custom render-pipeline rewrite —
      see implementation notes below)
- [x] Automated tests listed in 3.6 (clippy, `cargo nextest run
      --all-features`)
- [ ] Manual verification listed in 3.6 (launch tutorial campaign, visually
      confirm HUD/combat text, day/night lighting, combat flow) — **not
      performed**; this agent has no display/GPU to run the game binary.
      Needs a human pass before merging, especially for the
      `grass_instancing.rs` GPU pipeline rewrite (compiles and passes its
      unit tests, but wasn't visually verified).

#### 3.8 Success Criteria

- Full workspace builds and passes `cargo clippy --workspace --all-targets
  -- -D warnings` and `cargo test --workspace` on Bevy 0.19.
- The `block v0.1.6` future-incompatibility warning is gone
  (`cargo report future-incompatibilities` shows nothing for this crate).
- Manual verification in 3.6 passes with no visual/behavioral regressions.

### Phase 4: `campaign_builder` egui/eframe Stack Upgrade

#### 4.1 Bump `eframe`/`egui`

Update `eframe = "0.35"` and `egui = "0.35"` in
[sdk/campaign_builder/Cargo.toml](sdk/campaign_builder/Cargo.toml); this is
fully decoupled from Phase 3 (campaign_builder never depends on
`bevy_egui`) and can be scheduled independently, but is sequenced after
Bevy since it's the SDK tool rather than the game itself.

#### 4.2 Bump `rfd` and `tray-icon`

Update `rfd = "0.17"` (12 affected files) and, for the macOS-only target,
`tray-icon = "0.24"` (2 affected files:
[sdk/campaign_builder/src/lib.rs](sdk/campaign_builder/src/lib.rs),
[sdk/campaign_builder/src/tray.rs](sdk/campaign_builder/src/tray.rs)).

#### 4.3 Verify `egui_autocomplete` compatibility

Confirm the pinned `egui_autocomplete = "12.0"` still targets `egui 0.35`;
bump if a newer release is required for compatibility.

#### 4.4 Fix the 45-file `eframe`/`egui` API surface

Work through compile errors across `campaign_builder`'s editor modules
(`map_editor.rs`, `items_editor.rs`, `spells_editor.rs`,
`campaign_editor.rs`, etc. — full list is the 45 files identified in the
audit) against the two-minor-version `egui` API drift.

#### 4.5 Testing Requirements

- `cargo clippy -p campaign_builder --all-targets -- -D warnings` and
  `cargo test -p campaign_builder`.
- Manual run: launch `campaign-builder`, open each editor tab once, confirm
  file-dialog (`rfd`) and macOS tray-icon behavior still work.

#### 4.6 Deliverables

- [ ] `eframe`/`egui` 0.35 migration across 45 files
- [ ] `rfd` 0.17 migration
- [ ] `tray-icon` 0.24 migration (macOS)
- [ ] `egui_autocomplete` compatibility confirmed/bumped
- [ ] Tests listed in 4.5

#### 4.7 Success Criteria

- `campaign_builder` builds clean on the new `egui` stack; all editor tabs
  and file dialogs function in manual verification.

### Phase 5: Workspace-Wide Verification and Documentation

#### 5.1 Full verification pass

Run `cargo clippy --workspace --all-targets -- -D warnings` and
`cargo test --workspace` once more across the fully upgraded workspace
(catches any cross-phase interaction missed by per-phase checks).

#### 5.2 Confirm the original trigger is resolved

Re-check `cargo report future-incompatibilities` for the `block v0.1.6`
lint that motivated this plan.

#### 5.3 Update documentation

Add an entry to [docs/explanation/implementations.md](docs/explanation/implementations.md)
summarizing the upgrade, following the phase-per-entry convention used for
the combat-improvements work.

#### 5.4 Testing Requirements

- Full workspace `clippy`/`test` run (5.1) plus one final manual smoke test
  of both binaries (`antares` game and `campaign-builder` SDK tool).

#### 5.5 Deliverables

- [ ] Full workspace clippy/test pass on final dependency set
- [ ] `block v0.1.6` future-incompatibility warning confirmed resolved
- [ ] `implementations.md` entry added

#### 5.6 Success Criteria

- Every direct dependency in the workspace is on its latest stable version;
  `cargo report future-incompatibilities` is clean; both binaries run
  correctly end to end.

## Copyright

SPDX-License-Identifier: Apache-2.0

This document follows the [SPDX Spec](https://spdx.github.io/spdx-spec/) for
copyright and licensing information.
