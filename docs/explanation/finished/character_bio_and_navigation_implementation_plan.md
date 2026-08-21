# Character Bio & Navigation Implementation Plan

## Overview

Two problems, one screen. `src/game/systems/character_sheet_ui.rs`
(`GameMode::CharacterSheet`, the "Party Overview" + single view character
screen) currently has no in-game way to show a character's
backstory/character-profile content, and separately its mouse input is
effectively dead in Party Overview view — confirmed root cause below. Rather
than block the new Bio feature on a mouse fix, this plan sequences keyboard
navigation first (which every other screen in the game already uses as its
primary or backup input method), then layers the Bio content and its own
keyboard access on top, then fixes the mouse issue last as an independently
valuable, low-risk change once it's fully understood.

## Current State Analysis

### Existing Infrastructure

- `CharacterSheetState` (`src/application/character_sheet_state.rs:88-110`)
  tracks `focused_index`, `view: CharacterSheetView` (`Single` |
  `PartyOverview`), and `previous_mode`, with `focus_next`/`focus_prev`
  (wrapping) and `toggle_view` helpers already unit-tested.
- `character_sheet_input_system` (`character_sheet_ui.rs:151-221`) already
  implements a full keyboard vocabulary for **Single** view: `Tab`/`Shift+Tab`,
  `←`/`→` (focus_next/prev), digit keys 1-6 via the remappable
  `GameAction::SelectCharacter(i)` (`src/game/systems/input/keymap.rs`), `O`
  (toggle view), and the global `Esc`/`P` close. This is the established
  key-vocabulary convention used consistently across every other screen
  (inventory, spellbook, dialogue, shops, training) — see line 174-176: all of
  this is gated behind `is_single`, i.e. **none of it currently runs while in
  Party Overview view**.
- `render_party_overview` / `render_party_overview_card`
  (`character_sheet_ui.rs:1026-1195`) lay the party out as a `(cols, rows)`
  grid (`party_overview_grid_dimensions`, `:1015-1023`) and compute
  `idx = row * cols + col` per card — this indexing is exactly what a
  keyboard grid-navigation scheme needs, it's just not wired to input yet.
  Clicking a card's "View" button today sets `cs.focused_index` and
  `cs.view = Single` (`:1189-1194`) — this is the exact effect the new keyboard
  path needs to reproduce.
- The screen's on-screen hint bar (`character_sheet_ui.rs:483-496`) follows a
  house style also used via `title_bar_with_hints`
  (`src/game/systems/ui_helpers.rs:543-556`): `[Key] Label` entries,
  right-aligned, separated, in `UI_HINT_COLOR`. (`character_sheet_ui.rs` does
  not itself call `title_bar_with_hints` — it has a hand-rolled hint bar that
  follows the same visual convention; other screens like `spellbook_ui.rs`
  and `skill_training_ui.rs` do call the shared helper directly.)
- `CharacterDefinition` (`src/domain/character_definition.rs:369-481`) has a
  `description: String` field already (doc comment: "Character
  backstory/biography", `:448-449`; also documented in
  `docs/how-to/create_characters.md`'s Field Reference table as "Character
  backstory/bio") but no *structured*, long-form profile fields, and no
  in-game screen currently renders it. The new `CharacterLore.backstory` is
  **not** a duplicate of this field — see Phase 2.1 for the intended
  distinction between the two.
- Precedent for "definition references an external per-entity content file":
  `CreatureReference` + `CreatureDatabase::load_from_registry`
  (`src/domain/visual/creature_database.rs:356-410`), which resolves paths via
  `validate_campaign_relative_path` (`src/domain/path_security.rs:95`).
  `CharacterDefinition::instantiate()` already threads small definition fields
  (e.g. `portrait_id`, `:890`) onto the runtime `Character` — the same
  mechanism will carry lore content.

### Identified Issues

1. **No keyboard path through Party Overview at all.** A player who reaches
   Party Overview (via `O` from Single view) cannot select a card, view a
   character, or do anything except toggle back to Single or close the sheet
   — confirmed at `character_sheet_ui.rs:174-176`, the early-return applies to
   Tab, arrows, and digit-select alike.
2. **Mouse input is broken specifically in `GameMode::CharacterSheet`, root
   cause confirmed:** `movement_blocked_for_mode` /
   `interaction_blocked_for_mode`
   (`src/game/systems/input/mode_guards.rs:47-62`, `90-92`) list every other
   modal screen (`Menu`, `Inventory`, `Automap`, `Combat`, `Resting`,
   `RestMenu`, `GameLog`, `SpellCasting`, `TrapNotification`, `SkillTraining`,
   `GameOver`) but omit `CharacterSheet`. Those guards are what makes
   `handle_exploration_input_interact` (`src/game/systems/input.rs:204`) /
   `handle_exploration_input_movement` (`src/game/systems/input.rs:266`)
   stand down *synchronously, every frame*. Without it, CharacterSheet depends solely on bevy_egui's
   `egui_wants_any_pointer_input` run condition
   (`src/game/systems/input.rs:132-146`), which is backed by
   `EguiWantsInput`, written in `PostUpdate`
   (`bevy_egui-0.41.1/src/lib.rs:1240-1249`) — one frame *after* this frame's
   `character_sheet_ui_system` draws in `Update`. The guard the screen relies
   on is structurally stale.
   NOTE: this codebase has fixed Character Sheet mouse-click regressions at
   least four times before via different root causes (label-before-button
   clip-rect bugs, nested fixed-width `allocate_ui` clipping, the
   horizontal-scroll-strip party overview layout, and the
   `egui_wants_any_pointer_input` gate itself in commit `2ab2fae`). This
   specific fix (adding `CharacterSheet` to `mode_guards.rs`) has never been
   tried before, per full git history of that file, but Phase 5 should start
   with hands-on reproduction, not an assumption that this is the only
   remaining cause.
3. **Compounding click-through:** `handle_portrait_click_system`
   (`src/game/systems/hud.rs:470`) has no `run_if` gate at all, and
   `portrait_click_allowed` (`hud.rs:863-874`) explicitly includes
   `GameMode::CharacterSheet(_)`. The HUD portrait strip is pinned across the
   bottom of the screen and visually overlaps the Party Overview card grid, so
   a click there can simultaneously fire the HUD's own
   `enter_character_sheet_at(...)` handler, independent of anything egui does
   with the same click.
4. No cursor-grab/FPS mouse-look system exists anywhere in the codebase
   (ruled out as a cause) — this is a pure input-routing/mode-guard gap, not a
   window/cursor-mode issue.

## Implementation Phases

### Phase 1: Party Overview Keyboard Navigation

The immediate unblock: make Party Overview fully usable with only a keyboard,
using the exact key vocabulary already established for Single view, so the
Bio feature (Phase 3) has something to attach to and the screen isn't
mouse-dependent while Phase 5 is pending.

#### 1.1 Foundation Work

Add a highlighted-card index to `CharacterSheetState`
(`src/application/character_sheet_state.rs`) — reuse `focused_index` itself
as the highlight in Party Overview (it already exists, already clamps to
party size, no new field required) rather than introducing a second index to
keep in sync.

#### 1.2 Add Foundation Functionality

In `character_sheet_input_system` (`character_sheet_ui.rs:151-221`), add a
Party-Overview-scoped branch (mirrors the existing `is_single` branch,
inverse condition) supporting, using the existing `(cols, rows)` grid math
from `party_overview_grid_dimensions`:
- `↑`/`↓`/`←`/`→` — move the highlighted index by one row/column within the
  grid, wrapping at party bounds (reuse `focus_next`/`focus_prev` for
  `←`/`→`; add row-step arithmetic for `↑`/`↓` using the same `cols` value
  `render_party_overview` already computes).
- `Enter`/`Space` — equivalent to clicking a card's "View" button: set
  `cs.view = Single` at the current `focused_index` (same effect as
  `character_sheet_ui.rs:1189-1194`).
- Digit keys 1-6 — currently gated to Single view only; drop that gate so
  they also jump-select in Party Overview (harmless, and directly restores a
  no-mouse path).

#### 1.3 Integrate Foundation Work

`render_party_overview_card` (`character_sheet_ui.rs:1026-1109`) needs a
`highlighted: bool` parameter to draw a visible border/background on the
keyboard-selected card (there is currently zero visual feedback for a
non-mouse user); `render_party_overview` passes
`idx == global_state's cs.focused_index`.

#### 1.4 Testing Requirements

- Unit tests in `character_sheet_state.rs` for the new grid-step behavior
  (row/col wrap at grid edges, no-op on empty party — same style as existing
  `focus_next`/`focus_prev` tests).
- Automated `App`-harness tests of `character_sheet_input_system` itself
  (mirroring the existing `App::new()`/`app.update()` pattern used by
  `test_character_sheet_input_system_escape_closes` and
  `test_character_sheet_input_configured_digit_key_switches_focused_index`
  in this same file), covering: arrow-key grid nav actually moving
  `focused_index` through the real input system while in `PartyOverview`
  view, and Enter/Space actually switching `cs.view` to `Single`. Manual
  verification alone is a weaker bar than this file's existing test
  convention.
- Manual verification via the `run` skill: open Party Overview, confirm full
  keyboard traversal + Enter to open Single view, with zero mouse input.

#### 1.5 Deliverables

- [x] Arrow-key grid navigation in Party Overview
- [x] Enter/Space opens Single view for the highlighted card
- [x] Digit keys 1-6 work in both views
- [x] Visible highlight on the keyboard-selected card
- [x] Updated hint bar for Party Overview (`[↑↓←→] Move  [Enter] View  [1-6] Select  [O] Single  [Esc/P] Close`)
- [x] Module doc comment (`character_sheet_ui.rs:21-51`) updated to match

#### 1.6 Success Criteria

A player can fully operate the Character Sheet screen — browse the party
grid, open any member's Single view, close the screen — using only the
keyboard, with no regression to existing Single-view shortcuts.

---

### Phase 2: Character Backstory & Profile Data Model

Introduces the content model this was all for.

#### 2.1 Foundation Work

Add `CharacterLore` / `CharacterProfile` structs to
`src/domain/character_definition.rs` (`backstory: String`, and
`profile: { title, archetype, core_motivation, combat_style }`), plus
`lore_file: Option<String>` and a non-serialized `lore: Option<CharacterLore>`
field on `CharacterDefinition` (near `description`, `:447`). Update the
`CharacterDefinitionDef` shadow struct (`:493-572`) accordingly.

**`description` vs. `CharacterLore.backstory`:** these are deliberately
distinct, not redundant. `description` (doc comment: "Character
backstory/biography", `:448-449`) stays a short, single-line blurb — it is
asserted non-empty for every premade character by an existing test
(`character_definition.rs:2634`) and must keep that contract unchanged.
`CharacterLore.backstory` is new long-form, scrollable, multi-paragraph
content sourced from an external per-character file and shown only in the
new Bio panel (Phase 3). Do not repurpose or migrate `description` — add
`CharacterLore` alongside it.

#### 2.2 Add Foundation Functionality

Add `CharacterDatabase::load_from_campaign(data_dir, campaign_root)`
(`src/domain/character_definition.rs`) that loads `characters.ron` as today,
then resolves each `lore_file` via `validate_campaign_relative_path`
(`src/domain/path_security.rs:95`) and parses it into `CharacterLore` —
mirroring `CreatureDatabase::load_from_registry`
(`src/domain/visual/creature_database.rs:356-410`) for path resolution, but
**not** for error handling: `load_from_registry` is fail-fast (any single
missing/unreadable/invalid referenced file aborts the whole database load),
which is correct for creature data but wrong here since `lore_file` is
optional, decorative, per-character content. `load_from_campaign` must
instead: on a missing `lore_file` path, leave `lore: None`; on an
unreadable/invalid *referenced* file, log a warning, leave `lore: None` for
that character, and continue loading the rest of the campaign. Only path-
traversal attempts (rejected by `validate_campaign_relative_path`) should be
treated as hard errors.

`instantiate()` (`:795-962`) copies `lore` onto the runtime `Character`
(`src/domain/character.rs:1121+`), exactly like `portrait_id` at `:890`. The
runtime `Character` struct derives `Serialize, Deserialize` and is persisted
via `src/application/save_game.rs`; the new `lore` field **must** be marked
`#[serde(default)]` (the established pattern in this file — see the existing
`#[serde(default)]` fields and the comment at `character.rs:2022` explaining
why old save files without a since-added field must still deserialize),
otherwise loading a save created before this feature will fail.

#### 2.3 Integrate Foundation Work

Swap `CharacterDatabase::load_from_file` → `load_from_campaign` at both call
sites in `src/sdk/database.rs`:
- `load_campaign_with_skills_file` (fn at `:835`) already binds
  `campaign_path` (`:839`) — pass it directly.
- `load_core` (fn at `:1074`) does **not** have a `campaign_path` variable —
  it binds `data_path` (`:1075`) and derives `asset_root` (`:1076`) instead.
  Derive the campaign root here the same way `asset_root` is already
  derived (e.g. `data_path.parent()`), then pass that.

#### 2.4 Testing Requirements

- Round-trip tests for `CharacterLore` (de)serialization.
- `load_from_campaign` tests for: no `lore_file` set (→ `lore: None`,
  campaign still loads), missing referenced file (→ warning, `lore: None`,
  campaign still loads), invalid/unparseable referenced file (→ warning,
  `lore: None`, campaign still loads), and path-traversal rejection (→ hard
  error, mirroring the existing creature-registry path-traversal test as a
  template).
- Regression test: deserialize a pre-existing save-game fixture that
  predates the `lore` field and confirm it still loads successfully with
  `lore: None` (mirrors the convention at `character.rs:2022`).

#### 2.5 Deliverables

- [x] `CharacterLore`/`CharacterProfile` types + fields on `CharacterDefinition`/`Character`, with `Character.lore` marked `#[serde(default)]`
- [x] `CharacterDatabase::load_from_campaign` with lore resolution, warn-and-continue error handling
- [x] Both `ContentDatabase` loaders updated (including the `load_core` campaign-root derivation)
- [x] Old-save-fixture deserialization regression test
- [x] Lore RON files authored for tutorial characters under `campaigns/tutorial/assets/characters/lore/`, referenced from `campaigns/tutorial/data/characters.ron`. The full premade roster is 8 characters: Kira, Sirius, Isolde, Mira, Old Gareth, Whisper, Apprentice Zara, and Zhaya — author lore for all 8, or state an explicit inclusion criterion here if scoping down.
- [x] `docs/reference/campaign_content_format.md` — add `lore_file` to the characters.ron "Optional Fields" table (currently `:221-266`)
- [x] `docs/how-to/character_definition_ron_format.md` — document the new field(s) in the RON format walkthrough
- [x] `docs/how-to/create_characters.md` — add `lore_file` to the "Field Reference" table

#### 2.6 Success Criteria

`cargo test` passes; loading `campaigns/tutorial` populates `lore` on every
runtime `Character` that has a valid `lore_file`, with no effect on
characters that don't; a campaign with one character's broken/missing
`lore_file` still loads successfully with a warning; a save file created
before this feature still deserializes correctly; the three documentation
files above accurately describe the new field.

---

### Phase 3: Bio Panel UI + Keyboard Access

#### 3.1 Feature Work

Add a `showing_bio: bool` overlay flag to `CharacterSheetState` alongside
`view` (not a third `CharacterSheetView` variant — Bio is an overlay on
Single view, not a peer of Party Overview, so this avoids touching every
existing match on `CharacterSheetView` and keeps Esc/O/navigation working
underneath it unchanged).

#### 3.2 Integrate Feature

New key in `character_sheet_input_system`: `B` toggles the Bio panel,
available only when `is_single` and `character.lore.is_some()` (mirrors the
existing `O` handling at `:161-166`). Bio panel renders `profile.title` (or
falls back to name), `archetype`, `backstory` (scrollable wrapped label),
`core_motivation`, `combat_style` — reuse existing layout helpers
(`three_column`, or a simple `egui::ScrollArea`) rather than new primitives.

#### 3.3 Configuration Updates

Update the hint bar (`:483-495`) to add `[B] Bio` only when the focused
character has lore content; update the module doc diagram (`:21-51`) to
match, per the existing convention of keeping doc and rendered hints in sync.

#### 3.4 Testing Requirements

- Unit test the new toggle state transition on `CharacterSheetState`.
- Automated `App`-harness test of `character_sheet_input_system` (same
  pattern as Phase 1.4) confirming the `B` key actually toggles
  `showing_bio` when `is_single && character.lore.is_some()`, and is a no-op
  otherwise.
- Manual verification via `run` skill for a character with lore (Whisper)
  and one without, confirming the button/hint only appears when appropriate.

#### 3.5 Deliverables

- [x] Bio panel state + toggle in `CharacterSheetState`
- [x] `B` keyboard shortcut, gated on lore presence
- [x] Bio panel rendering (title/archetype/backstory/motivation/combat style)
- [x] Hint bar + doc comment updated

#### 3.6 Success Criteria

From Single view, pressing `B` on a character with lore shows their
backstory/profile; the key and hint are absent for characters without lore;
Esc/O/navigation still work correctly with the Bio panel open or closed.

---

### Phase 4: Campaign Builder (SDK) Lore Editor Support

#### 4.1 Feature Work

`sdk/campaign_builder/src/characters_editor.rs`: add a "Lore" section next to
the existing `description` editor (`:1861-1867`, `:2233`) — `lore_file`
display plus editable backstory/profile fields in `CharacterEditBuffer`.

#### 4.2 Integrate Feature

`sdk/campaign_builder/src/campaign_io.rs`: `load_characters_from_campaign`
(`:3652-3696`) resolves+loads each `lore_file` into the edit buffer; saving
writes one lore file per character with non-empty content, following
`save_creatures` (`:2269-2358`) as the template for "parent file + per-entity
files."

#### 4.3 Configuration Updates

None beyond the editor UI itself.

#### 4.4 Testing Requirements

Manual verification: edit lore for a character in `campaign_builder`, save,
reload the campaign in-game, confirm the Bio panel (Phase 3) reflects the
edit.

#### 4.5 Deliverables

- [x] Lore section in the character editor
- [x] `docs/how-to/create_characters.md` — "Using the Campaign Builder →
      Editor Features" section updated to mention the new Lore section
- [x] Load/save wired through `campaign_io.rs`

#### 4.6 Success Criteria

Lore content is fully editable and round-trips through the SDK without
hand-editing RON files.

---

### Phase 5: Mouse Input Fix for Character Sheet

Root cause is understood as far as static analysis can tell (see Identified
Issues #2-3), but given this exact screen's history of recurring mouse-click
regressions from other causes (see the note under Issue #2), Phase 5 must
start with hands-on reproduction against the current build before assuming
the mode-guards fix is sufficient on its own.

#### 5.1 Feature Work

Add `GameMode::CharacterSheet(_)` to both `movement_blocked_for_mode` and
`interaction_blocked_for_mode` in
`src/game/systems/input/mode_guards.rs:47-62,90-92`, matching every other
modal screen.

#### 5.2 Integrate Feature

Remove `GameMode::CharacterSheet(_)` from `portrait_click_allowed`
(`hud.rs:863-874`): once the sheet is open, its own keyboard/mouse
navigation (Phase 1 grid nav, Single-view Tab/arrows/digits) is the only
way to change focus — HUD portrait clicks are redundant and are what caused
the double-handling bug in the first place.

#### 5.3 Testing Requirements

- Unit tests on `movement_blocked_for_mode`/`interaction_blocked_for_mode`
  asserting `GameMode::CharacterSheet(_)` now returns blocked, alongside
  existing coverage for the other modes (mirror however the current modes
  are already tested, if at all — add coverage if none exists).
- Manual verification via `run` skill: confirm mouse clicks on Party
  Overview cards' "View" buttons and the new Bio button work; confirm
  exploration movement/interact does not leak through while the sheet is
  open; confirm existing screens (`Inventory`, etc.) are unaffected.

#### 5.4 Deliverables

- [x] `CharacterSheet` added to mode guards
- [x] Portrait click-through resolved

#### 5.5 Success Criteria

Mouse and keyboard are both fully functional in every Character Sheet view,
with no regressions to other modal screens' input handling.
