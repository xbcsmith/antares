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
