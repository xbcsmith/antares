<!-- SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Spell Book egui Conversion Implementation Plan

## Overview

`src/game/systems/spellbook_ui.rs` is the only exploration-mode management
screen in the game that uses Bevy's native entity/component UI instead of
`bevy_egui`.  Every other management screen (inventory, merchant, container,
inn, temple, lock) uses egui.  The Spell Book was implemented against the wrong
reference pattern (`exploration_spells.rs`, a gameplay overlay) rather than
the correct one (`inn_ui.rs`, a management screen).  This plan replaces the
four-system entity-lifecycle approach with a single egui render system and a
single input system, eliminating seven marker components and all
spawn/despawn bookkeeping.  `SpellBookState`, `GameMode::SpellBook`, and all
call sites outside `spellbook_ui.rs` are **not touched**.

## Current State Analysis

### Existing Infrastructure

| Layer | File | What Exists |
|---|---|---|
| **Domain state** | `src/application/spell_book_state.rs` | `SpellBookState` with `character_index`, `selected_row`, `selected_spell_id`, `previous_mode`; full navigation helpers |
| **Game mode** | `src/application/mod.rs` | `GameMode::SpellBook(SpellBookState)`, `enter_spellbook()`, `enter_spellbook_with_caster_select()`, `exit_spellbook()` |
| **Toggle guard** | `src/game/systems/input/global_toggles.rs` | `spell_book_toggle` hard-gated to `GameMode::Exploration`; explicitly tested for combat rejection |
| **Current UI** | `src/game/systems/spellbook_ui.rs` | `SpellBookPlugin` (4 chained systems), 7 marker components, 3 entity-builder helpers, 1 public helper, 10 `bevy::Color` constants |
| **egui dependency** | `Cargo.toml` + `src/bin/antares.rs` | `bevy_egui = "0.38"` already in workspace; `EguiPlugin` already registered |
| **egui pattern** | `src/game/systems/inn_ui.rs` | `InnUiPlugin` — `(inn_input_system, inn_ui_system).chain()`, `egui::CentralPanel`, `ScrollArea`, `push_id` per card |

### Identified Issues

1. **Wrong rendering paradigm** — `setup_spellbook_ui`, `update_spellbook_ui`,
   and `cleanup_spellbook_ui` manage Bevy entity lifecycle (spawn on enter,
   rebuild every frame, despawn on exit).  The inn, inventory, temple, and lock
   screens have none of this overhead because egui redraws from scratch each
   frame automatically.

2. **Seven unnecessary marker components** — `SpellBookOverlay`,
   `SpellBookContent`, `SpellBookCharTab`, `SpellBookSpellRow`,
   `SpellBookCharList`, `SpellBookSpellList`, `SpellBookDetailPane` exist
   solely to support the spawn/despawn lifecycle.  They have no equivalent in
   the egui screens and will all be deleted.

3. **Per-frame entity tree rebuild** — `update_spellbook_ui` calls
   `despawn_children` on three container entities and re-spawns all children
   every frame.  In egui this is the default behavior and requires no explicit
   management.

4. **Four chained Bevy systems** where two suffice — the egui equivalent is
   `(spellbook_input_system, spellbook_ui_system).chain()`, matching the inn
   pattern exactly.

5. **Color constants are wrong type** — the ten `pub const SPELLBOOK_*` values
   are `bevy::prelude::Color`.  After conversion they become
   `egui::Color32`, consistent with every other egui screen's color constants.

6. **Four Bevy App integration tests become obsolete** — the tests for
   `setup_spellbook_ui` spawning/despawning `SpellBookOverlay` entities test
   infrastructure that will no longer exist.  The twenty-odd pure-logic tests
   (state transitions, `collect_spell_ids_from_state`, SP affordability, Tab
   navigation) are renderer-agnostic and survive unchanged.

---

## Implementation Phases

### Phase 1: Port Rendering Helpers to egui

**Goal**: Replace the three Bevy entity-builder helpers with three egui render
helpers.  At the end of this phase `spellbook_ui.rs` contains **both** the old
entity-based code and the new egui helpers; nothing is wired up or deleted yet.
This lets `cargo check` confirm the egui logic compiles before any cutover.

#### 1.1 Add egui Imports

At the top of `src/game/systems/spellbook_ui.rs`, add:

```
use bevy_egui::{egui, EguiContexts};
```

alongside the existing `bevy::prelude::*` import.  No other files change.

#### 1.2 Add egui Color Constants

Add ten new `pub const` color constants of type `egui::Color32` immediately
below the existing Bevy `Color` constants.  Use the same names suffixed with
`_EG` temporarily to avoid a name collision during the transition period (they
will be renamed in Phase 3 once the old constants are deleted).

| Constant | egui::Color32 value |
|---|---|
| `SPELLBOOK_OVERLAY_BG_EG` | `from_rgba_premultiplied(0, 0, 26, 224)` |
| `SPELLBOOK_PANEL_BG_EG` | `from_rgba_premultiplied(15, 15, 46, 247)` |
| `SPELLBOOK_SELECTED_ROW_BG_EG` | `from_rgba_premultiplied(51, 51, 13, 230)` |
| `SPELLBOOK_NORMAL_ROW_COLOR_EG` | `WHITE` |
| `SPELLBOOK_DISABLED_SPELL_COLOR_EG` | `from_rgb(115, 115, 115)` |
| `SPELLBOOK_LEVEL_HEADER_COLOR_EG` | `from_rgb(179, 204, 255)` |
| `SPELLBOOK_CHAR_TAB_ACTIVE_COLOR_EG` | `from_rgb(255, 230, 51)` |
| `SPELLBOOK_CHAR_TAB_INACTIVE_COLOR_EG` | `from_rgb(153, 153, 179)` |
| `SPELLBOOK_HINT_COLOR_EG` | `from_rgb(140, 140, 166)` |
| `SPELLBOOK_TITLE_COLOR_EG` | `from_rgb(204, 217, 255)` |

#### 1.3 Add `render_char_tabs()`

Add a new private function:

```
fn render_char_tabs(
    ui: &mut egui::Ui,
    sb: &SpellBookState,
    global_state: &GlobalState,
)
```

The function body is a direct translation of `build_char_tabs` with
`ChildSpawnerCommands` calls replaced by egui equivalents:

- Column header via `ui.label(egui::RichText::new("Characters").color(...))`.
- Empty-party guard via `ui.label(...)`.
- Per-member loop: `ui.push_id(i, |ui| { ... })` wrapping each tab row
  (required by `sdk/AGENTS.md` egui ID rules, applied here for consistency).
- Active tab background via `egui::Frame::none().fill(...).show(ui, |ui| { ... })`.
- Active/inactive text color logic identical to `build_char_tabs`.

#### 1.4 Add `render_spell_list()`

Add a new private function:

```
fn render_spell_list(
    ui: &mut egui::Ui,
    sb: &SpellBookState,
    global_state: &GlobalState,
    content: Option<&GameContent>,
    spell_ids: &[SpellId],
)
```

The function body is a direct translation of `build_spell_list`:

- Header and guard labels via `ui.label(...)`.
- Level headers via `ui.label(egui::RichText::new("-- Level N --").color(...))`.
- Per-spell rows: `ui.push_id(spell_id, |ui| { ... })` wrapping each row.
  Selected rows use `egui::Frame::none().fill(SPELLBOOK_SELECTED_ROW_BG_EG)`.
- SP affordability, gem cost, context tag, and label formatting logic are
  identical to `build_spell_list` — only the output call changes.
- Learnable Scrolls section follows immediately, same logic as `build_spell_list`.

#### 1.5 Add `render_detail_panel()`

Add a new private function:

```
fn render_detail_panel(
    ui: &mut egui::Ui,
    sb: &SpellBookState,
    content: Option<&GameContent>,
)
```

Translation of `build_detail_panel`:

- Header via `ui.label(...)`.
- `None`-selected guard via `ui.label("Select a spell to view details.")`.
- Spell fields (name, school, level, SP cost, gem cost, context, description)
  via `ui.label(egui::RichText::new(...).color(...))` calls.
- Spell name rendered larger via `.size(BODY_FONT_SIZE + 2.0)`.

#### 1.6 Testing Requirements

No new tests are written in this phase.  Run quality gates after each helper
to confirm the code compiles and lints clean:

```
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

The full test suite is not expected to change at this point — the new helpers
are not yet called by anything.

#### 1.7 Deliverables

- [ ] `use bevy_egui::{egui, EguiContexts};` added to `spellbook_ui.rs`
- [ ] Ten `SPELLBOOK_*_EG` `egui::Color32` constants added
- [ ] `render_char_tabs()` added (compiles, lints clean)
- [ ] `render_spell_list()` added (compiles, lints clean)
- [ ] `render_detail_panel()` added (compiles, lints clean)

#### 1.8 Success Criteria

- `cargo check --all-targets --all-features` reports zero errors.
- `cargo clippy --all-targets --all-features -- -D warnings` reports zero
  warnings.
- No existing tests fail.

---

### Phase 2: Add the egui System and Simplify the Plugin

**Goal**: Write the primary `spellbook_ui_system`, rename
`handle_spellbook_input` to `spellbook_input_system`, and update
`SpellBookPlugin` to use the new two-system chain.  At the end of this phase
the egui rendering is live; the old Bevy systems are still present but no
longer registered.

#### 2.1 Rename `handle_spellbook_input` to `spellbook_input_system`

Rename the function in place.  Its signature, body, and logic are **unchanged**
— the function already directly mutates `GlobalState` via `ResMut<GlobalState>`
which is idiomatic.  Only the name changes:

```
pub fn spellbook_input_system(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut global_state: ResMut<GlobalState>,
    content: Option<Res<GameContent>>,
)
```

#### 2.2 Add `spellbook_ui_system`

Add the egui render system following the exact structural pattern of
`inn_ui_system` in `src/game/systems/inn_ui.rs`:

```
fn spellbook_ui_system(
    mut contexts: EguiContexts,
    global_state: Res<GlobalState>,
    content: Option<Res<GameContent>>,
)
```

Structure of the body:

1. Guard on `GameMode::SpellBook(sb)` — return early otherwise.
2. `let ctx = match contexts.ctx_mut() { Ok(ctx) => ctx, Err(_) => return };`
3. `let spell_ids = collect_spell_ids_from_state(&global_state.0, content.as_deref());`
4. `egui::CentralPanel::default().show(ctx, |ui| { ... })` containing:
   - Title bar: `ui.horizontal(|ui| { ui.heading("📚 Spell Book"); ui.with_layout(..., |ui| { ui.label("[ESC] Close") }) })`.
   - `ui.separator()`
   - Three-column body via `ui.horizontal(|ui| { ... })`:
     - Left column (fixed width ~150 px): `ui.vertical(|ui| { ui.set_min_width(140.0); ui.set_max_width(160.0); render_char_tabs(ui, sb, &global_state); })`.
     - `ui.separator()`
     - Center column (fills remaining space): `ui.vertical(|ui| { ui.set_min_width(200.0); egui::ScrollArea::vertical().id_salt("spellbook_spell_list").show(ui, |ui| { render_spell_list(ui, sb, &global_state, content.as_deref(), &spell_ids); }); })`.
     - `ui.separator()`
     - Right column (fixed width ~200 px): `ui.vertical(|ui| { ui.set_min_width(180.0); ui.set_max_width(215.0); egui::ScrollArea::vertical().id_salt("spellbook_detail_pane").show(ui, |ui| { render_detail_panel(ui, sb, content.as_deref()); }); })`.
   - `ui.separator()`
   - Bottom hint bar: `ui.horizontal_centered(|ui| { ui.label("[C] Cast Spell   [Tab] Switch Character   [↑↓] Select Spell"); })`.

The two `ScrollArea` instances must carry unique `id_salt` values
(`"spellbook_spell_list"` and `"spellbook_detail_pane"`) per the egui ID
audit rules in `sdk/AGENTS.md`.

#### 2.3 Update `SpellBookPlugin`

Replace the four-system chain with the two-system pattern matching
`InnUiPlugin`:

```
app.add_systems(
    Update,
    (spellbook_input_system, spellbook_ui_system).chain(),
);
```

The old systems (`setup_spellbook_ui`, `update_spellbook_ui`,
`cleanup_spellbook_ui`) are no longer registered.  They remain in the file
temporarily until Phase 3 removes them.

#### 2.4 Testing Requirements

- Run `cargo nextest run --all-features` and confirm that:
  - The four Bevy App integration tests that call `setup_spellbook_ui` /
    `cleanup_spellbook_ui` directly now **fail** (expected — they will be
    deleted in Phase 3).
  - All pure-logic tests continue to pass.
- Smoke-test in the running game: open exploration mode, press `B`, confirm the
  egui Spell Book overlay appears; press `Tab` to cycle characters; press `Esc`
  to close; press `C` to transition to the casting flow.

#### 2.5 Deliverables

- [ ] `handle_spellbook_input` renamed to `spellbook_input_system`
- [ ] `spellbook_ui_system` added with `EguiContexts` and three-column layout
- [ ] Both `ScrollArea` instances carry unique `id_salt` values
- [ ] `SpellBookPlugin::build()` uses `(spellbook_input_system, spellbook_ui_system).chain()`
- [ ] Old systems no longer registered (still present in file)

#### 2.6 Success Criteria

- `cargo check --all-targets --all-features` reports zero errors.
- `cargo clippy --all-targets --all-features -- -D warnings` reports zero
  warnings.
- In a running game build, the Spell Book opens, displays the three-column
  egui layout, Tab navigation cycles characters, Esc closes correctly.

---

### Phase 3: Delete All Bevy-Native Dead Code

**Goal**: Remove every symbol that existed solely to support the entity
lifecycle: four Bevy systems, seven marker components, three entity-builder
helpers, one internal helper function, and the ten old Bevy `Color` constants.
Rename the `_EG`-suffixed egui constants to their canonical names.

#### 3.1 Delete Bevy Systems

Remove the following functions entirely from `spellbook_ui.rs`:

- `pub fn setup_spellbook_ui(...)` — entity spawning on mode entry
- `pub fn update_spellbook_ui(...)` — per-frame entity tree rebuild
- `pub fn cleanup_spellbook_ui(...)` — entity despawning on mode exit
- `fn despawn_children(...)` — internal helper used only by `update_spellbook_ui`

#### 3.2 Delete Marker Components

Remove the following `#[derive(Component)]` structs entirely:

- `SpellBookOverlay`
- `SpellBookContent`
- `SpellBookCharTab`
- `SpellBookSpellRow`
- `SpellBookCharList`
- `SpellBookSpellList`
- `SpellBookDetailPane`

#### 3.3 Delete Bevy Entity-Builder Helpers

Remove the following private functions entirely:

- `fn build_char_tabs(list: &mut ChildSpawnerCommands<'_>, ...)`
- `fn build_spell_list(list: &mut ChildSpawnerCommands<'_>, ...)`
- `fn build_detail_panel(pane: &mut ChildSpawnerCommands<'_>, ...)`

These are fully superseded by `render_char_tabs`, `render_spell_list`, and
`render_detail_panel`.

#### 3.4 Delete Old Bevy Color Constants and Rename egui Constants

Remove the ten `pub const SPELLBOOK_*: Color` constants (Bevy color type).
Rename the ten `SPELLBOOK_*_EG: egui::Color32` constants to drop the `_EG`
suffix, restoring the canonical names (`SPELLBOOK_OVERLAY_BG`,
`SPELLBOOK_PANEL_BG`, etc.).  Update all references inside the file
accordingly.

#### 3.5 Remove Unused Imports

Remove the following imports that are no longer referenced after the dead code
is deleted:

- `BackgroundColor`, `BorderRadius`, `ChildSpawnerCommands`, `Children`,
  `FlexDirection`, `JustifyContent`, `Node`, `Overflow`, `PositionType`,
  `Text`, `TextColor`, `TextFont`, `UiRect`, `Val`, `Visibility` (any Bevy UI
  node types no longer used)
- `crate::game::systems::ui_helpers::{BODY_FONT_SIZE, LABEL_FONT_SIZE}` if
  they are now expressed inline as `egui::FontId::proportional(...)` constants

Retain all imports needed by `spellbook_input_system`, `collect_spell_ids_from_state`,
and the new egui render helpers.

#### 3.6 Testing Requirements

```
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

All three must pass with zero errors and zero warnings before proceeding to
Phase 4.

#### 3.7 Deliverables

- [ ] `setup_spellbook_ui`, `update_spellbook_ui`, `cleanup_spellbook_ui`, `despawn_children` deleted
- [ ] Seven marker components deleted
- [ ] `build_char_tabs`, `build_spell_list`, `build_detail_panel` deleted
- [ ] Ten old `bevy::prelude::Color` constants deleted
- [ ] Ten egui constants renamed (drop `_EG` suffix)
- [ ] Unused Bevy UI imports removed
- [ ] `cargo check` and `cargo clippy` pass with zero issues

#### 3.8 Success Criteria

- `cargo check --all-targets --all-features` — zero errors.
- `cargo clippy --all-targets --all-features -- -D warnings` — zero warnings.
- `src/game/systems/spellbook_ui.rs` no longer imports `bevy::prelude::Node`,
  `BackgroundColor`, `ChildSpawnerCommands`, or any other Bevy UI entity type.

---

### Phase 4: Test Rewrite and Documentation

**Goal**: Delete the now-invalid Bevy App integration tests, add egui render
function unit tests, verify the full pure-logic test suite passes unchanged,
and record the conversion in `docs/explanation/implementations.md`.

#### 4.1 Delete Bevy App Integration Tests

Remove the following four tests from the `mod tests` block in
`spellbook_ui.rs`.  They test entity lifecycle that no longer exists:

- `test_setup_spellbook_ui_spawns_overlay`
- `test_cleanup_spellbook_ui_despawns_overlays`
- `test_setup_spellbook_ui_is_idempotent`
- `test_setup_spellbook_ui_no_spawn_in_exploration_mode`

Also remove the `test_spell_book_overlay_is_marker_component`,
`test_spell_book_content_is_marker_component`,
`test_spell_book_char_tab_stores_party_index`, and
`test_spell_book_spell_row_stores_spell_id` tests — they test marker components
that no longer exist.

#### 4.2 Verify Pure-Logic Tests Are Unchanged

Confirm that the following test groups continue to pass **without any
modification**:

| Group | Tests |
|---|---|
| `collect_spell_ids_from_state` | `test_collect_spell_ids_not_in_spellbook_mode_returns_empty`, `test_collect_spell_ids_empty_party_returns_empty`, `test_collect_spell_ids_no_content_returns_empty` |
| Tab navigation | `test_tab_forward_increments_character_index`, `test_tab_forward_wraps_at_party_size`, `test_tab_back_decrements_character_index`, `test_tab_back_wraps_to_end_at_zero` |
| SP affordability | `test_spell_row_disabled_when_sp_insufficient`, `test_spell_row_enabled_when_sp_sufficient` |
| Mode transitions | `test_enter_and_exit_spellbook_roundtrip`, `test_exit_spellbook_noop_when_not_spellbook_mode` |
| Key simulation | `test_esc_triggers_exit_spellbook`, `test_c_key_transitions_to_spell_casting` |

These tests are renderer-agnostic and must require zero changes.  If any
fail, treat it as a regression introduced in Phases 2–3.

#### 4.3 Add egui Render Helper Unit Tests

Add three new tests that exercise the egui helper functions using
`egui::__run_test_ui`:

- `test_render_char_tabs_empty_party_no_panic` — calls `render_char_tabs` with
  an empty `party.members` vec and asserts no panic.
- `test_render_spell_list_no_spells_shows_placeholder` — calls
  `render_spell_list` with `spell_ids = &[]` and verifies the function
  completes without panic.
- `test_render_detail_panel_no_selection_no_panic` — calls
  `render_detail_panel` with `sb.selected_spell_id = None` and asserts no
  panic.

These are smoke tests for the render helpers, following the same pattern used
in `inn_ui.rs` render function tests.  They do not assert on egui widget state
(which requires a full context); they only verify the functions do not panic
given boundary inputs.

#### 4.4 Run Full Test Suite

```
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

All four gates must pass with zero errors and zero warnings.

#### 4.5 Update `docs/explanation/implementations.md`

Add a section summarising the conversion:

- State what changed: four Bevy systems → two; seven marker components deleted;
  rendering now egui via `CentralPanel` + three-column `horizontal` layout with
  `ScrollArea` for the center and detail columns.
- State what did not change: `SpellBookState`, `GameMode::SpellBook`,
  `enter_spellbook()`, `exit_spellbook()`, `collect_spell_ids_from_state`,
  `spellbook_input_system` logic, `global_toggles.rs`.
- Note line-count delta (expected net reduction of ~200–300 lines).

#### 4.6 Deliverables

- [ ] Eight obsolete tests deleted (four lifecycle, four marker-component)
- [ ] All pure-logic tests pass unchanged
- [ ] `test_render_char_tabs_empty_party_no_panic` added and passing
- [ ] `test_render_spell_list_no_spells_shows_placeholder` added and passing
- [ ] `test_render_detail_panel_no_selection_no_panic` added and passing
- [ ] All four quality gates pass with zero errors and zero warnings
- [ ] `docs/explanation/implementations.md` updated

#### 4.7 Success Criteria

- `cargo nextest run --all-features` — zero failures, zero errors.
- `src/game/systems/spellbook_ui.rs` imports no Bevy UI entity types.
- `SpellBookPlugin::build()` registers exactly two systems.
- The file contains zero `#[derive(Component)]` structs.
- `docs/explanation/implementations.md` records the conversion.

---

## Architecture Compliance Checklist

- [ ] `SpellBookState` in `src/application/spell_book_state.rs` — not modified
- [ ] `GameMode::SpellBook`, `enter_spellbook()`, `exit_spellbook()` in
      `src/application/mod.rs` — not modified
- [ ] `global_toggles.rs` spell-book-toggle guard — not modified
- [ ] `src/bin/antares.rs` `SpellBookPlugin` registration — not modified
- [ ] Both `ScrollArea` instances carry unique `id_salt` values
      (`"spellbook_spell_list"`, `"spellbook_detail_pane"`)
- [ ] Every character tab loop iteration uses `ui.push_id(i, ...)`
- [ ] Every spell row loop iteration uses `ui.push_id(spell_id, ...)`
- [ ] All `pub const SPELLBOOK_*` constants are `egui::Color32` — not
      `bevy::prelude::Color`
- [ ] `spellbook_ui_system` guards on `GameMode::SpellBook` and returns early
      for all other modes (matches the pattern of every other egui screen)
- [ ] No test references `SpellBookOverlay`, `SpellBookContent`, or any of the
      other deleted marker components
- [ ] No test references `setup_spellbook_ui`, `update_spellbook_ui`, or
      `cleanup_spellbook_ui`
- [ ] `docs/explanation/implementations.md` updated after Phase 4 completion
