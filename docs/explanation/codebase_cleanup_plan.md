# Codebase Cleanup Implementation Plan

## Overview

This plan consolidates a six-axis analysis of `src/` (208 Rust files, ~250K
lines) into a prioritized, phased cleanup program. The analysis covered:
duplicate code, dead code / suppressed lints, error-handling consistency,
unfinished-work markers, stale "Phase N" dev-plan references, and security.

The headline result: **the codebase is healthier than raw grep counts
suggest.** The domain layer is exemplary (strong `thiserror` adoption,
saturating economy math, no `unsafe`, no shell execution, no secrets). The real
debt is concentrated in five places: (1) a small set of campaign/asset
path-traversal sinks and one dice-roll panic, (2) a handful of error-swallowing
`let _ =` discards at the Bevy↔domain boundary, (3) ~45 dead items and 7 stale
suppression attributes, (4) 71 "Phase N" dev-plan comments plus one plan-named
test, and (5) recurring egui layout / RON-loader / inventory-UI duplication.

This plan is ordered **safety and correctness first, mechanical cleanup next,
structural refactors after, and completing the deferred features last.** Per
project policy, backwards compatibility is a non-goal — deprecated paths are
removed, not kept.

> This is a **planning document**. It describes work for a later implementation
> pass. No code is changed by this document. Each phase ends with the four
> mandatory quality gates (`cargo fmt`, `cargo check`, `cargo clippy -D
> warnings`, `cargo nextest run`).

## Current State Analysis

### Existing Infrastructure

- **Error handling**: `thiserror` is the standard — 53 `#[derive(Error)]`, 44
  `#[from]` conversions across 48 files. The domain layer returns typed
  `Result`s idiomatically.
- **RON loading**: `impl_ron_database!` macro exists in
  `src/domain/database_common.rs` (used by `classes.rs`, `races.rs`) but is
  bypassed by ~6 hand-written loaders in `src/sdk/database.rs`.
- **UI layout**: `src/game/systems/spellbook_ui.rs` is the canonical Rule 6
  multi-column pattern; `src/game/systems/ui_helpers.rs` and
  `src/game/systems/inventory_ui_common.rs` exist as natural homes for shared
  helpers but are underused.
- **Path validation**: `FontConfig::validate` (`src/sdk/game_config.rs`) is a
  model traversal guard (rejects absolute + `..`, enforces prefix/suffix,
  unit-tested) — but the pattern is not reused at other asset sinks.
- **Safety baseline**: 0 `unsafe`, 0 `Command`/shell execution, 0 hardcoded
  secrets, saturating economy arithmetic, widen-then-clamp combat math.

### Identified Issues

| # | Area | Severity | Summary |
|---|------|----------|---------|
| A | Security — dice panic | P0 | `DiceRoll::roll` panics on `sides == 0`; `sides: u8` is deserialized straight from campaign RON with no validation (`src/domain/types.rs:398`). Crashes combat. |
| B | Security — path traversal | P0/P1 | `campaign_id` (packager install, `src/sdk/campaign_packager.rs:380`) and registry `filepath` values (`src/domain/visual/creature_database.rs:374`, `src/domain/world/object_mesh.rs:380`) are joined without sanitization → arbitrary write/read from a downloaded campaign. |
| C | Security — decompression bomb | P1 | `.tar.gz` campaign unpack has no size cap and runs before validation (`src/sdk/campaign_packager.rs:335-349`). |
| D | Error handling — swallowed Results | P1 | ~6 `let _ =` discards drop domain `Result`s in combat/inventory (`inventory_ui.rs:2770/2878`, `monster_spells.rs:302`, `spell_casting.rs:538`, `bin/antares.rs:326`). |
| E | Error handling — fragile system unwraps | P2 | Guarded-`Option` unwraps in Bevy systems (`events.rs:714-716`, `dialogue.rs:385`, `map_builder.rs:254`). |
| F | Error handling — taxonomy | P2/P3 | No central error root; 4 colliding error-enum names (`CampaignError`, `ConfigError`, `ValidationError`, `GeneratorError`); 5 manual `Display`/`Error` impls. |
| G | Dead code | P2 | 45 dead items (38 `pub fn`, 7 `pub struct`/`enum`) + 7 stale `#[allow(deprecated)]` (no `#[deprecated]` exists in `src`). |
| G2 | SDK search dead/duplicate code | P2 | SDK search already works in all ~18 campaign-builder editors (substring filter over editor-local `Vec`s). The 12 `src/sdk/database.rs` query methods it does **not** use are dead duplicates of the domain-layer query API. `campaign_editor.rs::search_filter` is a stale `(future)` stub on a single-item editor. |
| G3 | SDK unwired "future" code | P2 | `CampaignBuilderApp` has a `// Future / unused fields` block (`_export_wizard`, `_test_play_session`, `_test_play_config`, `_show_export_dialog`, `_show_test_play_panel`) never read outside their own default init. Behind them, two whole modules — `packager.rs` (`ExportWizard`, ~350 lines) and `test_play.rs` (`TestPlaySession`, ~350 lines) — are exercised **only** by their own tests. `EditorRegistry._quests_*` (4 fields) and `_stock_templates_file` are dead duplicates of `QuestEditorState` state and `CampaignMetadata.stock_templates_file`. `FileNode._children` is write-only (assigned, never read). |
| H | Suppressed lints | P3 | 92 `#[allow(...)]`; 80 are `too_many_arguments` (~17 real UI/combat refactor candidates, rest idiomatic-Bevy false positives). |
| I | Stale Phase refs | P2 | 71 `Phase N` references — **all comments**, none are production identifiers; plus one plan-named test fn. |
| J | Duplicate code | P3 | egui column-layout scaffolding, title-bar hints, UI color constants, SDK DB loaders, campaign RON loaders, combat cleanup systems, merchant/container inventory UIs. |
| K | Unfinished features | P2–P5 | Reputation/faction system (3 subsystems, log-only), Jump spell charges SP with no effect, quest `SetFlag` not persisted, `CameraMode::Tactical`/`Isometric` fall back silently, skill-rank temp modifiers no-op. |
| L | Determinism | P3 | 90 non-test `rand::rng()` sites; production RNG is entropy-seeded and non-reproducible, violating the deterministic-gameplay architecture goal. |

## Implementation Phases

### Phase 1: Security & Correctness Hardening

Close the crash/exploit surface first. Highest severity, smallest diffs.

#### 1.1 Foundation Work

- Extract `FontConfig::validate` (`src/sdk/game_config.rs:1245-1274`) traversal
  logic into a shared `validate_campaign_relative_path(base, candidate) ->
  Result<PathBuf, _>` helper (reject absolute paths, reject `Component::ParentDir`,
  canonicalize, assert `starts_with(base)`).

#### 1.2 Add Foundation Functionality

- **Item A (P0)**: Guard `DiceRoll::roll` (`src/domain/types.rs:398`) — return
  `self.bonus.max(0)` when `sides == 0` — **and** enforce `sides >= 1` in
  campaign/data validation.
- **Item B (P0)**: Allowlist-validate `manifest.campaign_id`
  (`^[A-Za-z0-9_-]+$`, non-empty) before `join` in
  `src/sdk/campaign_packager.rs:380`; assert result stays under `campaigns_dir`.
- **Item B (P1)**: Apply the §1.1 helper at `creature_database.rs:374` and
  `object_mesh.rs:380` registry `filepath` joins.
- **Item C (P1)**: Cap cumulative uncompressed size during tar/gzip `unpack`
  (`campaign_packager.rs:335-349`).
- **Items B/§2.4/§2.5 (P2)**: Sanitize `SaveGameManager::save_path`
  (`save_game.rs:479`), add `..` rejection to `landscape.rs:1164`, apply id
  allowlist to `CampaignLoader::load_campaign` (`sdk/campaign_loader.rs:852`).

#### 1.3 Integrate Foundation Work

- **Item D (P1)**: Replace error-swallowing `let _ =` with logged handling
  (`if let Err(e) = … { warn!(?e) }` routed to `GameLogEvent`/`tracing`) at
  `inventory_ui.rs:2770/2878`, `monster_spells.rs:302`, `spell_casting.rs:538`;
  propagate/log `create_dir_all` in `bin/antares.rs:326`.
- **Item E (P2)**: Convert guarded-`Option` unwraps to `let … else` / `if let`
  at `events.rs:714-716`, `dialogue.rs:385`, `map_builder.rs:254`; make the
  startup saves-dir `expect` (`menu.rs:40-41`) a graceful error.

#### 1.4 Testing Requirements

- Unit test `DiceRoll::roll` with `sides == 0` (no panic) and validation
  rejecting `sides == 0`.
- Path-traversal tests: campaign with `filepath: "../../etc/passwd"`,
  `campaign_id: "../evil"`, and absolute paths are all rejected. Use
  `data/test_campaign` fixtures (never `campaigns/tutorial`).
- Decompression-bomb test with a fixture archive exceeding the cap.
- All new fixtures live under `data/test_campaign/`.

#### 1.5 Deliverables

- [x] Shared `validate_campaign_relative_path` helper
- [x] `DiceRoll::roll` guard + `sides >= 1` validation
- [x] `campaign_id` allowlist + containment assertion
- [x] Registry `filepath` sanitization (creature + object mesh)
- [x] Decompression size cap
- [x] Save name / landscape texture / CLI id sanitization
- [x] Error-swallowing `let _ =` discards logged (2 genuine sites; the other 4
      flagged sites discard `Option`/struct values, not `Result`, and were
      correctly left unchanged)
- [x] Guarded-Option unwraps converted (4 sites) + graceful saves-dir error

#### 1.6 Success Criteria

- No panic reachable from malformed campaign/save data in the touched paths.
- No path-traversal write/read escapes `campaigns_dir` / `saves_dir` in tests.
- Four quality gates pass with zero warnings.

### Phase 2: Dead Code & Suppressed-Lint Removal

Low-risk deletions. Use the compiler as the authority.

#### 2.1 Foundation Work

- Detection: temporarily flip any crate-level `#![allow(dead_code)]` to warn and
  run `RUSTFLAGS="-W dead_code -W unused_imports" cargo check --all-targets
  --all-features`; run `cargo clippy … -W clippy::pedantic`. Confirm the
  token-frequency candidate list below against compiler output.

#### 2.2 Add Foundation Functionality (removals)

- **Item G**: Delete 7 stale `#[allow(deprecated)]` in
  `src/sdk/cli/item_editor.rs` (209, 1380, 1609, 1655, 1688, 1719, 1747) — no
  `#[deprecated]` exists in `src`.
- Delete 7 dead `pub struct`/`enum`: `ActiveActionHighlight`
  (`combat.rs:1214`), `HpText` (`hud.rs:281`), `ItemUseAction`
  (`item_usage.rs:53`), `RecruitmentDialogState` (`recruitment_dialog.rs:41`),
  `SpellCastAction` / `SpellCastResult` (`spell_casting.rs:36/49`), `TempleUiRoot`
  (`temple_ui.rs:64`).
- Delete 5 unregistered Bevy systems / spawn helpers: `creature_spawning_system`,
  `spawn_shrub`, `spawn_custom_furniture_mesh_with_rendering`,
  `get_or_create_tree_mesh`, `tree_mesh_cache_key` (verify no asset-pipeline refs).
- Delete ~16 dead domain query/accessor fns
  (`domain/items/database.rs`, `domain/character.rs`, `domain/skill_resolver.rs`,
  and misc listed in the audit).
- **Decision (resolved — delete)**: The 12 unused `sdk/database.rs` query
  methods (`get_*_by_name`, `spells_by_school`/`_by_level`, `undead_monsters`,
  `monsters_by_experience_range`, `main_quests`, `repeatable_quests`,
  `quests_for_level`, `repeatable_dialogues`, `dialogues_for_quest`) are **not
  wired to any search path** and are partial duplicates of the domain-layer
  query API. Verified consumers:
  - **SDK search already works** in all ~18 `sdk/campaign_builder` content
    editors via per-editor `search_filter`/`search_query` + inline substring
    matching over editor-local `Vec`s (`spells_editor`, `monsters_editor`,
    `items_editor`, `quest_editor`, `dialogue_editor`, `characters_editor`,
    `classes_editor`, `races_editor`, `skills_editor`, `proficiencies_editor`,
    `conditions_editor`, `levels_editor`, `stock_templates_editor`,
    `landscape_editor`, `objects_editor`, `furniture_editor`, `creatures_editor`,
    `map_editor`). **None** of them call these DB query methods.
  - `sdk::database::ContentDatabase` *is* used for campaign loading, but not
    these sub-database lookup/filter methods.
  - The tested, doc-tested equivalents live in the domain layer under different
    names/types (`domain::magic::database::SpellDatabase::get_spells_by_school`
    / `get_spells_by_level`, `domain::combat::database::MonsterDatabase::get_undead_monsters`).
  Because SDK search does **not** need them, **delete the 12 `sdk/database.rs`
  methods** as dead duplicates. Keep the domain-layer query API (already
  exercised by tests). If the campaign builder ever needs DB-backed search, add
  it against the domain API, not this surface.

#### 2.3a SDK Search Hygiene (campaign_builder crate)

> Scope note: this work is in the separate `sdk/campaign_builder` crate, outside
> the original `src/`-only analysis scope, but is included because it is the
> "SDK search + duplicate/dead code" concern.

- **Confirm SDK search works everywhere**: SDK search is already functional in
  all ~18 editors (verified above). Add a `filtered_*`/search unit test for any
  editor still missing one so search behavior is contract-tested, then leave the
  working search paths untouched. No new DB-query wiring is required.
- **Delete dead duplicate quest-search state** in
  `sdk/campaign_builder/src/editor_state.rs`: remove
  `EditorRegistry._quests_search_filter`, `_quests_show_preview`,
  `_quests_import_buffer`, `_quests_show_import_dialog` (superseded by
  `QuestEditorState`'s own `search_filter`/preview/import), and
  `_stock_templates_file` (dead duplicate of `CampaignMetadata.stock_templates_file`).
  Update/remove the self-referential tests in `tests/editor_state_tests.rs`
  (`test_quest_preview_toggle`, `test_quest_import_buffer`, and the
  `_quests_show_preview` assertion in `test_quest_editor_state_initialization`).
- **Resolve the stale `campaign_editor.rs` `(future)` search stub**: a single
  campaign has nothing to filter — remove the `search_filter` field + its
  `.with_search(...)` toolbar wiring (or, if kept for layout parity, drop the
  `(future)` comment and document why). Prefer removal.
- **Remove `FileNode._children`** (`lib.rs`): it is assigned in
  `update_file_tree`/`read_directory` but never read. Drop the field and the
  now-pointless recursion in `read_directory`.

#### 2.3b SDK Unwired "Future" Modules

`CampaignBuilderApp` (`lib.rs`) carries a `// Future / unused fields` block whose
five fields (`_export_wizard`, `_test_play_session`, `_test_play_config`,
`_show_export_dialog`, `_show_test_play_panel`) are never read. Behind them sit
two entire modules exercised **only** by their own unit tests.

- **Test Play — REMOVE (decided).** Delete
  `sdk/campaign_builder/src/test_play.rs` (`TestPlaySession`, ~350 lines, spawns
  the game as a child process), the `_test_play_session` / `_test_play_config` /
  `_show_test_play_panel` fields (+ their `Default` init), the `mod test_play`
  declaration, and the module's tests. Not planned for wiring.
- **Export Wizard — IMPLEMENT (decided).**
  `sdk/campaign_builder/src/packager.rs` (`ExportWizard`, ~350 lines) is a
  guided multi-step campaign **export/packaging** dialog
  (Validation → FileSelection → Metadata → Settings → Exporting → Complete).
  Finish and wire it into the app: add an "Export / Package Campaign" entry that
  opens the dialog, connect the `_export_wizard` / `_show_export_dialog` fields
  (renamed to drop the `_` prefix once live), and drive the actual packaging
  through the existing `src/sdk/campaign_packager.rs` (main crate, hardened in
  Phase 1) rather than duplicating pack logic. Add integration tests that run a
  full wizard flow end-to-end against a `data/test_campaign` fixture and assert a
  valid package is produced. Effort L.
  (Scope note: this exports a *campaign* into a distributable package — not a
  Wizard-class character.)

#### 2.3 Integrate

- **Item H**: Fix the two non-argument lint suppressions and remove their
  `#[allow]`: `only_used_in_recursion` (`dialogue.rs:569`),
  `needless_pass_by_value` (`exploration_interact.rs:670`).
- Leave the 3 test-only `#[allow(dead_code)]` and ~63 Bevy-system
  `too_many_arguments` allows as-is (idiomatic / false positives). The ~17 real
  UI/combat `too_many_arguments` offenders move to Phase 5.
- Add a scoped Clippy gate (`unwrap_used`, `expect_used`,
  `let_underscore_must_use` as `warn`, allowed in `#[cfg(test)]`) to prevent
  regression.

#### 2.4 Testing Requirements

- `cargo nextest run --all-features` still green after each deletion batch.
- Confirm no `#[test]` referenced a deleted item.

#### 2.5 Deliverables

- [ ] 7 stale `#[allow(deprecated)]` removed
- [ ] 7 dead structs/enums removed
- [ ] 5 dead systems/helpers removed
- [ ] 16 dead domain fns removed
- [ ] 12 dead `sdk/database.rs` query methods removed (domain-layer API kept)
- [ ] SDK search verified/contract-tested across all editors (no DB-query wiring)
- [ ] Dead `EditorRegistry._quests_*` + `_stock_templates_file` state removed + tests updated
- [ ] Stale `campaign_editor.rs` `(future)` search stub removed
- [ ] `FileNode._children` write-only field removed
- [ ] `CampaignBuilderApp` `// Future / unused fields` block resolved
- [ ] Test Play removed: `test_play.rs` + `_test_play_*` fields + tests deleted
- [ ] Export Wizard finished + wired: "Export / Package Campaign" dialog live, driven by `campaign_packager`, end-to-end test added
- [ ] 2 lint fixes + `#[allow]` removed
- [ ] Regression Clippy gate added

#### 2.6 Success Criteria

- `cargo check`/`clippy` report zero unused-item warnings in `src` **and** in
  the `sdk/campaign_builder` crate.
- SDK content search works in every editor with no dead or duplicate search
  state remaining (no `_quests_*` fields, no DB-query duplicates).
- Net line reduction with no behavior change; quality gates pass.

### Phase 3: Stale "Phase N" & Comment Cleanup

Mechanical, near-zero risk. Removes shipping-codebase dev-plan cruft.

#### 3.1 Foundation Work

- Re-run `grep -rIn -E '\bPhase[ _-]?[0-9]' src` to confirm the 71-item list;
  confirm none are identifiers except the one test fn.

#### 3.2 Add Foundation Functionality

- **Item I**: Reword all 71 `Phase N` comments/doc-comments to describe behavior
  rather than plan phases (full file:line inventory in the analysis; spans ~25
  files including `world/types.rs`, `advanced_grass.rs`, `advanced_trees.rs`,
  `combat.rs`, `spell_casting.rs`, `item_usage.rs`, `hud.rs`, `mod.rs`).
- Rename `test_test_campaign_phase1_landscape_mesh_fixture_integrity`
  (`domain/world/landscape.rs:1809`) → drop the `phase1` token.
- Keep legitimate `Phase`-named identifiers: `NavigationPhase` enum, Bevy
  `RenderPhase`/`PhaseItem`/`BinnedRenderPhaseType` (framework API).
- Clarify (not delete) misleading placeholder comments where the value is
  actually populated downstream (e.g. `inventory_ui.rs` `slot_index: 0`).

#### 3.3 Configuration Updates

- None. Comment-only changes.

#### 3.4 Testing Requirements

- `cargo nextest run` green (the one renamed test still runs).

#### 3.5 Deliverables

- [ ] 71 `Phase N` comments reworded/removed
- [ ] Plan-named test fn renamed
- [ ] Misleading placeholder comments clarified

#### 3.6 Success Criteria

- `grep -rIn -E '\bPhase[ _-]?[0-9]' src` returns only legitimate identifiers.

### Phase 4: Error-Handling Consistency & Determinism

Structural improvements at the domain↔Bevy boundary.

#### 4.1 Feature Work

- **Item F**: Rename the 4 colliding error enums to layer-qualified names
  (`domain::CampaignError` vs `sdk::CampaignError`, the two `ConfigError`, two
  `ValidationError`, two `GeneratorError`).
- Introduce a central `GameError` root (new `src/error.rs` or
  `src/domain/error.rs`) aggregating the ~50 module errors via `#[from]`; use it
  as the boundary type where Bevy systems consume domain results.
- Add a `report_err!` / logging helper so systems (which cannot return `Result`)
  route domain errors into `GameLogEvent` + `tracing` uniformly.
- Migrate the 5 manual `Display`/`Error` impls to `thiserror`
  (`domain/types.rs`, `domain/character.rs`, `domain/combat/monster.rs`,
  `domain/items/types.rs`, `game/systems/ui.rs`).
- Give `name_generator.rs` static-slice unwraps a `.expect("static name table
  is non-empty")` justification.

#### 4.2 Integrate Feature

- **Item L (in scope this pass)**: Introduce a seeded `GameRng` resource
  (`StdRng`/`SmallRng` via `SeedableRng`), persist the seed in the save file, and
  thread it through the 90 production `rand::rng()` sites (combat, exploration
  spells). This restores the deterministic-gameplay architecture guarantee and
  makes save/load replay reproducible. It is the largest sub-effort in this
  phase and should be tackled as a dedicated work item, but it is **included in
  this cleanup program, not deferred**.

#### 4.3 Configuration Updates

- Save-file schema gains a `rng_seed` field (no backwards-compat concern).

#### 4.4 Testing Requirements

- Modifier/error-conversion tests for the new `GameError` `#[from]` paths.
- Determinism test: same seed + same inputs → identical combat outcomes.

#### 4.5 Deliverables

- [ ] Colliding error enums renamed
- [ ] Central `GameError` + `report_err!` helper
- [ ] 5 manual impls migrated to `thiserror`
- [ ] `name_generator` unwrap justification
- [ ] Seeded `GameRng` + persisted seed (dedicated work item, in scope)

#### 4.6 Success Criteria

- One boundary error type consumed by systems; no cross-layer name ambiguity.
- Seeded runs reproducible; quality gates pass.

### Phase 5: Duplicate-Code Consolidation

Highest line savings, highest refactor risk — do after helpers exist.

#### 5.1 Feature Work

- **Item J (quick wins first)**:
  - Promote shared UI palette (`TITLE_COLOR`, `HINT_COLOR`, `HEADER_COLOR`) into
    `ui_helpers.rs`; replace per-file duplicates in `character_sheet_ui.rs`,
    `spellbook_ui.rs`, `skill_training_ui.rs`.
  - Add `title_bar_with_hints(ui, title, &[&str])` to `ui_helpers.rs`; adopt in
    ~7 screens.
  - Promote `format_gold` (`merchant_inventory_ui.rs:90`) to `ui_helpers.rs`.
- Extract a `three_column`/`columns_layout` Rule-6 helper into `ui_helpers.rs`
  (computes `col_h`, separator math, per-column `allocate_ui` +
  `auto_shrink([true,false])`); adopt in `spellbook_ui`, `skill_training_ui`,
  `character_sheet_ui`. This retires the most fragile UI failure mode.
- Route the ~6 hand-written `load_from_file` methods in `sdk/database.rs` through
  the existing `impl_ron_database!` macro (`database_common.rs`).
- Add `load_optional_ron<T>(rel) -> Result<Option<T>, _>` to
  `campaign_loader.rs`; collapse ~9 near-identical loaders.

#### 5.2 Integrate Feature

- Add `despawn_all<T: Component>` + `reset_on_combat_exit` helpers; collapse the
  5–7 combat cleanup systems (`combat.rs` cleanup fns).
- Consider a `GameMode` run-condition (`run_if(in_game_mode(...))`) to remove the
  ~30 `if !matches!(mode, …) { return; }` prologues.
- Move the merchant/container inventory shared scaffolding (character strip,
  split-panel, item-slot grid) into `inventory_ui_common.rs`; parameterize
  `push_id` salts. **Largest, riskiest — do last with per-flow keyboard tests.**
- Refactor the ~17 real `too_many_arguments` UI/combat helpers into
  params/context structs and drop their `#[allow]`.

> Note: extracting closures over `&mut Ui` + event writers may hit borrow-checker
> friction; expect some `FnOnce(&mut Ui) -> R` plumbing.

#### 5.3 Configuration Updates

- None (internal refactor).

#### 5.4 Testing Requirements

- egui screens follow AGENTS.md Rule 6 audit (allocate_ui per column,
  `auto_shrink([true,false])`, hints in title bar, no `auto_shrink([false,false])`).
- Keyboard-navigation tests for merchant/container/inventory flows before and
  after convergence.

#### 5.5 Deliverables

- [ ] Shared UI palette + `title_bar_with_hints` + `format_gold`
- [ ] Column-layout helper adopted in 3+ screens
- [ ] `sdk/database.rs` routed through `impl_ron_database!`
- [ ] `campaign_loader` `load_optional_ron` collapse
- [ ] Combat cleanup helpers (`despawn_all`, reset)
- [ ] Inventory UI convergence
- [ ] `too_many_arguments` UI/combat helpers refactored

#### 5.6 Success Criteria

- Net line reduction; no visual/interaction regressions; quality gates pass.

### Phase 6: Unfinished-Feature Implementation

Complete the deferred features rather than removing their surface. Each item
replaces a log-only / silent-degrade path with working behavior. This is real
feature work — the largest and most content-dependent phase — and each item
should consult the relevant `docs/reference/architecture.md` sections before
implementation, using the exact data structures and type aliases defined there.

#### 6.1 Feature Work

- **Item K (P2 — largest)**: **Implement the Reputation/faction system.**
  Currently log-only across `dialogue.rs:659` (`ReputationThreshold` condition
  always returns `false`), `dialogue.rs:1139` (`ChangeReputation` action no-op),
  and `quests.rs:345` (`QuestReward::Reputation` no-op). Add reputation state to
  the party/`GameState` model, evaluate the threshold condition against it, apply
  the change action, and honor the quest reward. This threads through three
  subsystems, so land the state model first, then the three call sites.
- **Item K (P2)**: **Implement Jump spell targeting.** Build the
  target-selection UI/logic so `TeleportDestination::Jump`
  (`exploration_spells.rs:637-641`) actually moves the party; SP is only charged
  on a successful cast.
- **Item K (P3)**: **Implement quest `SetFlag` persistence** (`quests.rs:336`) —
  wire flags into `GameState` so flag-gated quest logic works.
- **Item K (P3)**: **Implement `CameraMode::Tactical` and `::Isometric`**
  (`camera.rs:98-107`) so selecting them changes the camera instead of falling
  back to first-person.
- **Item K (P4)**: **Implement skill-rank temporary modifiers** — the "Step 5"
  no-op in `skill_resolver.rs:294` — so buffs/debuffs affect effective ranks.
- **Item K (P4)**: **Implement the recruitment confirmation UI**
  (`events.rs:619`) so `RecruitableCharacter` events without a `dialogue_id`
  have a working confirm/recruit path.
- **Item K (P5)**: **Implement real monster visuals + animation** — replace the
  `spawn_fallback_visual` colored cube (`monster_rendering.rs`) and flesh out the
  empty `CreatureAnimationState` (`creature.rs:748`) with keyframe support.

#### 6.2 Integrate Feature

- Land each feature's state/model changes first, then its call sites and UI.
- Extend `data/test_campaign` fixtures with reputation data, a Jump-spell
  scenario, quest flags, and a recruitable-without-dialogue NPC to exercise the
  new paths.

#### 6.3 Configuration Updates

- Save-file / `GameState` schema gains reputation state and quest flags.
- No RON schema fields are removed — the previously-inert reward/condition/camera
  variants become functional.

#### 6.4 Testing Requirements

- Game-mechanic tests per feature: reputation threshold gating + change +
  quest-reward application; Jump spell moves party and only charges SP on
  success; `SetFlag` persists and gates quest logic; each camera mode renders
  distinctly; skill-rank modifiers change effective ranks; recruitment confirm
  path recruits. All fixtures under `data/test_campaign` only.

#### 6.5 Deliverables

- [ ] Reputation/faction system implemented (state + 3 call sites)
- [ ] Jump spell targeting implemented (SP charged only on success)
- [ ] Quest `SetFlag` persistence implemented
- [ ] `CameraMode::Tactical` + `::Isometric` implemented
- [ ] Skill-rank temporary modifiers implemented
- [ ] Recruitment confirmation UI implemented
- [ ] Monster visuals + `CreatureAnimationState` implemented

#### 6.6 Success Criteria

- No shipped code path silently degrades or charges resources for no effect;
  every previously-inert feature now has working, tested behavior.
- Quality gates pass; `docs/explanation/implementations.md` updated.

## Recommended Sequencing Summary

| Phase | Theme | Risk | Value | Gate |
|-------|-------|------|-------|------|
| 1 | Security & correctness | Low (small diffs) | Critical | Prevents crashes/exploits |
| 2 | Dead code & lints | Low | Medium | ~45 items + 7 attrs removed |
| 3 | Phase-ref cleanup | Very low | Medium | 71 comments + 1 test |
| 4 | Error taxonomy & RNG | Medium | High | Boundary consistency, determinism |
| 5 | Duplicate consolidation | Med–High | High | Retires Rule-6 failure mode |
| 6 | Feature implementation | Med–High (per item) | High | Reputation, Jump, flags, cameras, skills, recruit, monster visuals |

Phases 1–3 are safe, high-confidence, and can proceed immediately. The three
largest chunks — Phase 4's seeded-RNG migration (~90 sites), Phase 5's inventory
UI convergence, and Phase 6's reputation/faction system — each warrant a
dedicated pass. Phase 6 is now full feature implementation (not removal), so it
is the most effort-heavy phase and depends on `docs/reference/architecture.md`
for exact data structures.
