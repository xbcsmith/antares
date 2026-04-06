<!-- SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Spell Management Implementation Plan

## Overview

The spell system's core mechanics are fully operational — learning, casting
(combat and exploration), quest/dialogue rewards, scroll dispatch, and the
SP bar on the HUD all work. What is missing is the **player-facing management
surface**: a dedicated in-game Spell Book UI for browsing known spells and SP
status outside the active casting flow; a `starting_spells` field on
`CharacterDefinition` so pre-made and NPC-recruitable characters can ship with
populated spell books; and a matching Starting Spells section in the SDK
character editor so campaign authors can author those values. This plan closes
all three gaps in four sequential phases ordered from lowest to highest
dependency depth.

## Current State Analysis

### Existing Infrastructure

| Layer | File(s) | What Exists |
|---|---|---|
| **SpellBook domain type** | `src/domain/character.rs` | `SpellBook` with `cleric_spells`/`sorcerer_spells` `[Vec<SpellId>; 7]` per character |
| **Spell learning domain** | `src/domain/magic/learning.rs` | `learn_spell`, `can_learn_spell`, `get_learnable_spells`, `grant_level_up_spells`, `SpellLearnError` |
| **Exploration casting** | `src/domain/magic/exploration_casting.rs` | `cast_exploration_spell`, `can_cast_exploration_spell`, `get_castable_exploration_spells` |
| **Exploration cast UI** | `src/game/systems/exploration_spells.rs` | Multi-step **cast** flow using `GameMode::SpellCasting` (caster → spell → target → result) |
| **Combat spell UI** | `src/game/systems/combat.rs` | `SpellSelectionPanel`, `SpellButton`, `handle_cast_spell_action` |
| **SP bar on HUD** | `src/game/systems/hud.rs` | `SpBarFill`, `SpBarBackground`, `SpBarTextOverlay`, `sp_bar_color()`, `format_sp_display()` |
| **Scroll dispatch** | `src/game/systems/inventory_ui.rs` | `CastSpell`/`LearnSpell` scroll effects fully wired in `handle_use_item_action_exploration` |
| **Dialogue LearnSpell** | `src/domain/dialogue.rs` + `src/game/systems/dialogue.rs` | `DialogueAction::LearnSpell` defined and executed |
| **Quest LearnSpell** | `src/application/quests.rs` | `QuestReward::LearnSpell` defined and applied |
| **SDK spell editor** | `sdk/campaign_builder/src/spells_editor.rs` | Full list/add/edit/filter editor with RON export |
| **SDK dialogue editor** | `sdk/campaign_builder/src/dialogue_editor.rs` | `ActionType::LearnSpell` with autocomplete spell picker |
| **SDK quest editor** | `sdk/campaign_builder/src/quest_editor.rs` | `RewardType::LearnSpell` with autocomplete spell picker |
| **SDK items editor** | `sdk/campaign_builder/src/items_editor.rs` | `CastSpell`/`LearnSpell` consumable effect editing |
| **SDK validation** | `sdk/campaign_builder/src/campaign_io.rs` | `validate_dialogue_learn_spell_actions`, `validate_quest_learn_spell_rewards` |

### Identified Issues

1. **No in-game Spell Book screen** — there is no `GameMode::SpellBook` and no
   dedicated UI for a player to browse their character's known spells outside
   of the active spell-casting flow. `GameMode::SpellCasting` drives the cast
   action; no passive read-only view exists. Players cannot inspect spell
   descriptions, SP costs, or gem requirements without committing to a cast
   attempt.

2. **`CharacterDefinition` has no `starting_spells` field** — pre-made and
   NPC-recruitable characters cannot be authored with pre-populated spell
   books. Every character instantiated from a `CharacterDefinition` begins
   with an empty `SpellBook` regardless of class, level, or backstory. This
   makes it impossible to ship a tutorial or premade party where the Cleric
   already knows First Aid.

3. **`CharacterDefinition::instantiate()` ignores spells** — even if
   `starting_spells` were added to RON data today, `instantiate()` receives no
   `SpellDatabase` and has no path to populate `character.spells`. The method
   must be updated to accept database references alongside the new field.

4. **SDK character editor has no spell management** — `CharacterEditBuffer` in
   `sdk/campaign_builder/src/characters_editor.rs` has no `starting_spells`
   field and `show_character_form` has no spell section. Campaign authors
   cannot define starting spells for their characters, making the `starting_spells`
   RON field inaccessible without hand-editing data files.

---

## Implementation Phases

### Phase 1: `starting_spells` in `CharacterDefinition` (Domain Layer)

**Goal**: Allow RON character templates to declare spells a character starts
with. When `instantiate()` is called those spells are correctly placed in the
`SpellBook` based on each spell's school and level in the `SpellDatabase`.

**Dependencies**: None — this is the foundational domain change all other
phases build on.

#### 1.1 Add `starting_spells` to `CharacterDefinition`

In `src/domain/character_definition.rs`:

- Add `pub starting_spells: Vec<SpellId>` to `CharacterDefinition` with
  `#[serde(default)]` and `#[serde(skip_serializing_if = "Vec::is_empty")]` so
  existing RON files deserialize without the field and newly serialized files
  only emit the field when non-empty.
- Add the same field (with `#[serde(default)]`) to the private
  `CharacterDefinitionDef` serde helper struct.
- Propagate the field in `impl From<CharacterDefinitionDef> for
  CharacterDefinition`: `starting_spells: def.starting_spells`.
- Initialize `starting_spells: Vec::new()` in `CharacterDefinition::new()`.
- Add a new error variant to `CharacterDefinitionError`:
  ```
  InvalidSpellId { character_id: CharacterDefinitionId, spell_id: SpellId }
  ```
  with a descriptive `#[error(...)]` message.

#### 1.2 Update `instantiate()` to Accept `SpellDatabase`

`CharacterDefinition::instantiate()` currently builds a `Character` with an
empty `SpellBook` and has no database access. It must receive a reference to
`SpellDatabase` so it can resolve each `SpellId` to school and level.

- Update the signature to:
  `pub fn instantiate(&self, spell_db: &SpellDatabase) -> Result<Character, CharacterDefinitionError>`
- After the existing item/equipment setup, iterate `self.starting_spells`. For
  each `spell_id`:
  - Look up the spell with `spell_db.get_spell(spell_id)`. If not found,
    return `Err(CharacterDefinitionError::InvalidSpellId { character_id:
    self.id.clone(), spell_id })`.
  - Determine the list to update: `SpellSchool::Cleric` →
    `character.spells.cleric_spells`, `SpellSchool::Sorcerer` →
    `character.spells.sorcerer_spells`.
  - Compute the zero-based level index as `spell.level.saturating_sub(1) as
    usize` clamped to `0..=6`.
  - Push `spell_id` into `character.spells.*_spells[level_index]` only if it
    is not already present (prevent duplicates).
- Do **not** enforce class restrictions here — `CharacterDefinition` is the
  authoritative source for a premade character's starting state. Authors are
  responsible for placing valid spells; the SDK validation pass (Phase 4) will
  warn on mismatches.

#### 1.3 Update All Call Sites of `instantiate()`

Locate every call to `CharacterDefinition::instantiate()` across the codebase
(primarily `src/game/systems/campaign_loading.rs` and any domain helpers).
Each call site must now supply a `&SpellDatabase` reference. Campaign loading
already holds `ContentDatabase`; pass `&content.db().spells` at each call
site.

#### 1.4 Testing Requirements

- Test that a `CharacterDefinition` with a cleric `SpellId` in
  `starting_spells` instantiates a character with that spell in
  `cleric_spells[level_index]`.
- Test that a sorcerer spell goes into `sorcerer_spells[level_index]`.
- Test that an unknown `SpellId` returns `Err(InvalidSpellId)`.
- Test that existing RON character data without `starting_spells` still
  deserializes cleanly (backward-compat serde default).
- Test that a character instantiated with no `starting_spells` has an empty
  `SpellBook`.
- Test that duplicate `SpellId` values in `starting_spells` do not produce
  duplicate entries in the `SpellBook`.
- Test `CharacterDefinitionError::InvalidSpellId` display message contains
  both the character ID and the spell ID.
- All fixtures from `data/test_campaign/`; **never** `campaigns/tutorial`.

#### 1.5 Deliverables

- [ ] `starting_spells: Vec<SpellId>` field in `CharacterDefinition` and
      `CharacterDefinitionDef`
- [ ] `CharacterDefinitionError::InvalidSpellId` variant
- [ ] `instantiate(&self, spell_db: &SpellDatabase)` updated signature with
      spell population logic
- [ ] All `instantiate()` call sites updated to pass `&SpellDatabase`
- [ ] Unit tests in `src/domain/character_definition.rs` for all new scenarios
- [ ] `data/test_campaign/data/characters.ron` updated with at least one
      fixture character that has `starting_spells` set for integration coverage

#### 1.6 Success Criteria

- `cargo check --all-targets --all-features` passes with zero errors.
- `cargo nextest run --all-features` passes including new spell-granting tests.
- Existing campaign data files without `starting_spells` continue to load.
- All four quality gates pass with zero warnings.

---

### Phase 2: In-Game Spell Book Management UI (Game Engine)

**Goal**: Add a dedicated read-only in-game Spell Book screen reachable from
exploration mode so players can browse each caster's known spells, view SP
status, read spell descriptions, and see which learnable scrolls are in the
character's inventory — entirely separate from the active spell-casting flow.

**Dependencies**: Phase 1 is strongly recommended before Phase 2 ships so that
premade characters arrive with populated spell books and the UI has non-trivial
content to display. The two phases may be developed in parallel.

#### 2.1 Add `SpellBookState` Application State

Create `src/application/spell_book_state.rs` (new file):

- `SpellBookState` struct fields:
  - `pub character_index: usize` — which party member's book is currently open.
  - `pub selected_spell_id: Option<SpellId>` — spell highlighted in the list.
  - `pub previous_mode: Box<GameMode>` — mode to restore on close. Uses the
    same `Box<GameMode>` pattern as `SpellCastingState` to break the recursive
    size dependency.
- `impl SpellBookState`:
  - `pub fn new(character_index: usize, previous_mode: GameMode) -> Self`
  - `pub fn get_resume_mode(&self) -> GameMode`
- Add `/// Spell book management screen` doc comment and a short module-level
  `//!` doc explaining the purpose and lifetime of the state.
- Add `pub mod spell_book_state;` to `src/application/mod.rs`.

#### 2.2 Add `GameMode::SpellBook` Variant

In `src/application/mod.rs`:

- Add the variant to `GameMode`:
  ```
  SpellBook(crate::application::spell_book_state::SpellBookState),
  ```
- Add convenience methods on `GameState`:
  - `pub fn enter_spellbook(&mut self, character_index: usize)` — saves current
    mode in `SpellBookState::previous_mode`, sets `self.mode` to
    `GameMode::SpellBook(...)`.
  - `pub fn enter_spellbook_with_caster_select(&mut self)` — calls
    `enter_spellbook(0)` and lets the UI handle skipping non-casters.
  - `pub fn exit_spellbook(&mut self)` — if mode is `SpellBook`, restores
    `previous_mode`; otherwise no-op.
- Add `///` doc comments with `# Examples` blocks on all three methods.

#### 2.3 Create `SpellBookPlugin` and UI System

Create `src/game/systems/spellbook_ui.rs` (new file). Follow the code structure
of `src/game/systems/exploration_spells.rs` as the reference pattern.

**Constants** (all `pub const`):

| Constant | Purpose |
|---|---|
| `SPELLBOOK_OVERLAY_BG` | Full-screen semi-transparent backdrop |
| `SPELLBOOK_PANEL_BG` | Inner panel background |
| `SPELLBOOK_SELECTED_ROW_BG` | Highlight color for selected spell row |
| `SPELLBOOK_NORMAL_ROW_COLOR` | Default spell name text color |
| `SPELLBOOK_DISABLED_SPELL_COLOR` | Spell name color when SP is insufficient |
| `SPELLBOOK_LEVEL_HEADER_COLOR` | "Level N" group header text color |
| `SPELLBOOK_CHAR_TAB_ACTIVE_COLOR` | Active character tab highlight |
| `SPELLBOOK_CHAR_TAB_INACTIVE_COLOR` | Inactive character tab color |
| `SPELLBOOK_HINT_COLOR` | Bottom hint text color |
| `SPELLBOOK_TITLE_COLOR` | "Spell Book" title text color |

**Marker components** (zero-size, `#[derive(Component)]`):

- `SpellBookOverlay` — root full-screen node.
- `SpellBookContent` — inner layout node.
- `SpellBookCharTab { pub party_index: usize }` — one per party member tab.
- `SpellBookSpellRow { pub spell_id: SpellId }` — one per spell entry in list.

**Systems registered by `SpellBookPlugin`**:

- `setup_spellbook_ui` — spawns the overlay hierarchy when
  `GameMode::SpellBook` becomes active (run condition: `in_spellbook_mode`).
- `cleanup_spellbook_ui` — despawns all `SpellBookOverlay` entities on exit.
- `update_spellbook_ui` — rebuilds the spell list when `character_index`
  changes; updates SP text; marks rows as enabled/disabled.
- `handle_spellbook_input` — handles keyboard (Tab to cycle characters, Up/Down
  to navigate spells, Enter/Space to select, C to open cast flow for the
  displayed character, Esc to close).

**Layout** (three-column design):

```
┌───────────────────────────────────────────────────────────────────┐
│  📚 Spell Book                                          [ESC Close]│
├───────────────┬────────────────────────────┬──────────────────────┤
│ Characters    │ Known Spells               │ Detail               │
│ ─────────     │ ─────────────              │ ──────               │
│ [Aria   ✓]   │ ── Level 1 ──              │ First Aid            │
│ [Korbin  ]   │  First Aid    5 SP          │ School: Cleric       │
│ [Sylva  ✓]   │  Cure Poison  8 SP  💎1    │ Level:  1            │
│ [Drek    ]   │ ── Level 2 ──              │ SP Cost: 5           │
│               │  Bless       12 SP  ⚔      │ Gem Cost: —          │
│               │                            │ Context: Any         │
│               │ ── Learnable Scrolls ──    │                      │
│               │  Scroll of Light  [✓ use   │ Restores 1d6+1 HP    │
│               │  from inventory]           │ to a single target.  │
├───────────────┴────────────────────────────┴──────────────────────┤
│  [C] Cast Spell    [Tab] Switch Character    [↑↓] Select Spell    │
└───────────────────────────────────────────────────────────────────┘
```

**Spell list construction** (in `update_spellbook_ui`):

- Call `SpellBook::get_spell_list_by_id(&character.class_id, &class_db)` to
  get the correct school's spell array.
- Iterate levels 1–7; for each level with at least one known spell, emit a
  level-header row then one row per spell.
- For each spell row: look up the `Spell` definition from `SpellDatabase` to
  get name, SP cost, gem cost (if `> 0` show 💎N), and context (combat-only ⚔,
  exploration-only 🌍, or empty for both). Mark the row disabled
  (`SPELLBOOK_DISABLED_SPELL_COLOR`) when `character.sp.current < spell cost`.

**Learnable Scrolls section** (bottom of the center column):

- Scan `character.inventory.items` for slots where the item's
  `ConsumableData.effect` is `ConsumableEffect::LearnSpell(spell_id)`.
- For each such scroll, show: scroll name, arrow, spell name. Show whether the
  character is eligible via `can_learn_spell`. Eligibility is read-only in this
  view; the player uses the Inventory screen to actually learn from the scroll.

**Detail panel** (right column):

- When `selected_spell_id` is `Some(id)`, show: name (large), school, level,
  SP cost, gem cost, context tag, and the spell's `description` string from
  `SpellDatabase`.
- When `selected_spell_id` is `None`, show "Select a spell to view details."

**C key shortcut**: pressing C while the Spell Book is open calls
`game_state.exit_spellbook()` followed by
`game_state.enter_spell_casting(character_index)` so the player is dropped
directly into the casting flow for the character they were browsing.

#### 2.4 Wire Input Binding to Open Spell Book

In `src/game/systems/input/keymap.rs` (or wherever `ControlsConfig` keybindings
are defined):

- Add `GameAction::OpenSpellBook` (or re-use an existing unbound action slot).
- Default key: choose an unoccupied key (verify no conflict in `keymap.rs`
  before assigning; `B` is the likely candidate).
- Add the matching entry to `ControlsConfig` in `data/test_campaign/config.ron`
  and `campaigns/tutorial/config.ron`.

In the exploration input system (wherever `GameAction::OpenSpellBook` is
consumed):

- On the action: call `game_state.enter_spellbook_with_caster_select()` when
  `game_state.mode` is `GameMode::Exploration`.

#### 2.5 Register `SpellBookPlugin` in the Game

In `src/game/systems/mod.rs` (or wherever game plugins are assembled):

- Add `SpellBookPlugin` to the plugin set, alongside `ExplorationSpellPlugin`
  and `CombatPlugin`.

#### 2.6 Testing Requirements

- Test `SpellBookState::new()` sets `character_index` and captures
  `previous_mode`.
- Test `SpellBookState::get_resume_mode()` returns the stored previous mode.
- Test `GameState::enter_spellbook(2)` sets `mode` to `GameMode::SpellBook`
  with `character_index == 2`.
- Test `GameState::exit_spellbook()` restores `previous_mode` from
  `SpellBookState`.
- Test `GameState::exit_spellbook()` when mode is not `SpellBook` is a no-op.
- Test `setup_spellbook_ui` spawns at least one `SpellBookOverlay` entity.
- Test `cleanup_spellbook_ui` despawns all `SpellBookOverlay` entities.
- Test that Tab navigation increments `character_index` and wraps at party size.
- Test that the spell list is populated from the active character's `SpellBook`.
- Test that a spell row with `sp_cost > character.sp.current` renders with
  `SPELLBOOK_DISABLED_SPELL_COLOR`.
- Test that pressing Esc triggers `exit_spellbook()`.
- All fixtures from `data/test_campaign/`; **never** `campaigns/tutorial`.

#### 2.7 Deliverables

- [ ] `src/application/spell_book_state.rs` — `SpellBookState` struct with
      constructors and `get_resume_mode()`
- [ ] `GameMode::SpellBook(SpellBookState)` variant in `src/application/mod.rs`
- [ ] `GameState::enter_spellbook()`, `enter_spellbook_with_caster_select()`,
      `exit_spellbook()` methods
- [ ] `src/game/systems/spellbook_ui.rs` — `SpellBookPlugin` with full layout,
      update, and input systems
- [ ] `SpellBookPlugin` registered in `src/game/systems/mod.rs`
- [ ] `GameAction::OpenSpellBook` keybinding in `keymap.rs` and `config.ron`
      files
- [ ] Unit and Bevy integration tests for state transitions and UI lifecycle

#### 2.8 Success Criteria

- Player can open the Spell Book with the bound key from exploration mode.
- All party casters' spell books are browsable with Tab navigation.
- Spell detail panel shows name, school, level, SP cost, gem cost, context, and
  description for the selected spell.
- Spells the character cannot currently afford appear visually dimmed.
- Learnable scrolls from the character's inventory appear in the scrolls
  section with eligibility indication.
- Pressing Esc returns to the mode that was active before the Spell Book was
  opened.
- Pressing C from the Spell Book transitions to the exploration spell-casting
  flow with the current character pre-selected.
- Non-caster characters' tabs are shown but greyed out (no learnable spells to
  display).
- All four quality gates pass with zero warnings.

---

### Phase 3: SDK Character Editor — Starting Spells Section

**Goal**: Campaign authors can define `starting_spells` for characters in the
SDK Campaign Builder, editing the field that Phase 1 added to
`CharacterDefinition`. The full add/remove/preview workflow should follow the
same patterns as the existing equipment and starting items editors.

**Dependencies**: Phase 1 (`starting_spells` must exist in
`CharacterDefinition` before the SDK can edit and serialize it). Phase 2 is
independent.

#### 3.1 Add `starting_spells` to `CharacterEditBuffer`

In `sdk/campaign_builder/src/characters_editor.rs`:

- Add `pub starting_spells: Vec<SpellId>` to `CharacterEditBuffer`.
- Initialize to `Vec::new()` in `CharacterEditBuffer::default()`.
- Document the field with an inline comment describing its purpose.

#### 3.2 Populate Buffer from Existing Definition

In `CharactersEditorState::start_edit_character()`:

- After loading all other fields from the existing `CharacterDefinition`, copy
  `definition.starting_spells.clone()` into `self.buffer.starting_spells`.

#### 3.3 Persist Buffer to Definition in `save_character()`

In `CharactersEditorState::save_character()`:

- Write `self.buffer.starting_spells.clone()` into the `CharacterDefinition`
  that is constructed and pushed to the characters list.
- Apply a soft validation pass (warning label in the UI, not a hard error):
  check each `SpellId` in `buffer.starting_spells` against the currently
  loaded `available_spells` list and surface a warning in
  `ctx.status_message` if any ID is unrecognized. This mirrors the pattern
  used in the dialogue editor for unknown spell IDs.

#### 3.4 Add `show_starting_spells_editor()` Helper

Add a private method
`fn show_starting_spells_editor(ui: &mut egui::Ui, buffer: &mut CharacterEditBuffer, available_spells: &[Spell])`
(or make it a method on `CharactersEditorState`) in `characters_editor.rs`.

The section appears inside `show_character_form()` below the equipment editor,
under a collapsible header `"📚 Starting Spells"`.

**Layout**:

```
▼ Starting Spells

  [Add Spell ▼ ] ← autocomplete spell selector (same widget as dialogue/items editors)

  Slot  Name             School   Level   [Remove]
  ────  ───────────────  ───────  ─────   ────────
   1    First Aid        Cleric     1      ❌
   2    Cure Poison      Cleric     1      ❌

  ⚠ "knight" cannot cast spells; starting spells are stored but have
    no in-game effect.                        ← shown only for non-casters
```

Implementation details:

- Use `egui::ScrollArea::vertical()` for the spell list, with a unique
  `id_salt` (per `sdk/AGENTS.md` egui rules).
- Wrap each row in `ui.push_id(spell_id, |ui| { ... })` (per egui ID audit
  rules).
- Use the `autocomplete_spell_selector` widget already present in the SDK
  (used in dialogue and items editors) for the Add Spell input.
- Prevent duplicate entries: before pushing a spell to
  `buffer.starting_spells`, check `!buffer.starting_spells.contains(&spell_id)`.
- Non-caster warning: check whether the character's `class_id` maps to
  `spell_school: None` by looking up the class in the available classes list
  (or simply pattern-match on known non-caster class IDs if the class list is
  not readily available). Display the warning as
  `ui.colored_label(egui::Color32::YELLOW, "⚠ ...")`.

#### 3.5 Testing Requirements

- Test `CharacterEditBuffer::default()` has `starting_spells` as an empty
  `Vec`.
- Test `start_edit_character()` loads `starting_spells` from an existing
  definition that has spells defined.
- Test `start_edit_character()` loads an empty `Vec` for a definition without
  `starting_spells`.
- Test `save_character()` persists `starting_spells` changes to the definition
  list (round-trip: set spells in buffer → save → read back from definition).
- Test adding a duplicate spell ID does not produce a duplicate in
  `buffer.starting_spells`.
- Test removing a spell removes the correct entry and does not shift remaining
  entries.
- Test the non-caster warning string is generated for a knight-class character
  with non-empty `starting_spells`.
- All fixtures from `data/test_campaign/`; **never** `campaigns/tutorial`.

#### 3.6 Deliverables

- [ ] `starting_spells: Vec<SpellId>` field in `CharacterEditBuffer`
- [ ] `start_edit_character()` updated to load spells from definition
- [ ] `save_character()` updated to persist spells to definition
- [ ] `show_starting_spells_editor()` helper with `ScrollArea`, per-row
      `push_id`, and autocomplete spell picker
- [ ] Non-caster class warning label
- [ ] Unit tests for all new editor logic in `characters_editor.rs`

#### 3.7 Success Criteria

- Campaign author can add and remove starting spells for any character
  definition in the SDK.
- Round-trip verified: define starting spells in SDK → save RON →
  load in game → character instantiates with the correct `SpellBook` contents.
- Non-caster warning appears when a knight/robber has spells defined.
- Duplicate spells cannot be added via the UI.
- All four quality gates pass with zero warnings.

---

### Phase 4: Validation Integration and Documentation

**Goal**: Close the SDK validation gap for `starting_spells` references,
ensure the test campaign fixture covers all new code paths, and update
documentation.

**Dependencies**: Phases 1 and 3 complete (the field and editor must exist
before validation rules can reference them).

#### 4.1 Add `validate_character_starting_spells` Validation Rule

In `sdk/campaign_builder/src/advanced_validation.rs`:

- Add the public function:
  ```
  pub fn validate_character_starting_spells(
      characters: &[CharacterDefinition],
      spells: &[Spell],
  ) -> Vec<ValidationError>
  ```
- For each `CharacterDefinition` with a non-empty `starting_spells` vec,
  verify each `SpellId` is present in the `spells` slice (by ID).
- Return a `ValidationError` (or `ValidationWarning` if the framework
  distinguishes severity) for each unresolvable reference, including the
  character's `id` and the unknown `spell_id` in the message.
- Optionally emit a warning (not an error) when a spell's school does not match
  the character's class — informational only, since authors may intentionally
  override the school placement for story reasons.

#### 4.2 Wire Validation in `campaign_io.rs`

In `sdk/campaign_builder/src/campaign_io.rs`, inside `validate_campaign()`,
alongside the existing calls to `validate_dialogue_learn_spell_actions` and
`validate_quest_learn_spell_rewards`:

```rust
self.validation_state.validation_errors.extend(
    validation::validate_character_starting_spells(
        &self.campaign_data.characters,
        &self.campaign_data.spells,
    ),
);
```

#### 4.3 Update `data/test_campaign/` Fixtures

- Ensure `data/test_campaign/data/characters.ron` includes at least one premade
  character definition with `starting_spells` set to one or more valid spell
  IDs present in `data/test_campaign/data/spells.ron`. This fixture drives
  integration test coverage for Phase 1's `instantiate()` path and Phase 4's
  validation path.
- If `data/test_campaign/data/spells.ron` lacks enough spells to cover the
  fixture, add the minimum required entries (at least one cleric L1 spell and
  one sorcerer L1 spell).

#### 4.4 Update `docs/explanation/implementations.md`

Add a section summarising what was built across all four phases: the
`starting_spells` domain change, the in-game Spell Book UI, the SDK character
editor spell section, and the validation rule.

#### 4.5 Testing Requirements

- Test `validate_character_starting_spells` returns a `ValidationError` when
  `starting_spells` references a `SpellId` not in the provided spells slice.
- Test `validate_character_starting_spells` returns an empty `Vec` when all
  referenced spell IDs are valid.
- Test `validate_character_starting_spells` returns an empty `Vec` for
  characters with empty `starting_spells`.
- Test that `validate_campaign()` in `campaign_io.rs` invokes the new rule
  (integration test: a campaign with an invalid spell reference in a character
  definition must surface a validation error).
- All fixtures from `data/test_campaign/`; **never** `campaigns/tutorial`.

#### 4.6 Deliverables

- [ ] `validate_character_starting_spells()` in
      `sdk/campaign_builder/src/advanced_validation.rs`
- [ ] Validation call wired in
      `sdk/campaign_builder/src/campaign_io.rs::validate_campaign()`
- [ ] `data/test_campaign/data/characters.ron` updated with a fixture entry
      containing `starting_spells`
- [ ] `data/test_campaign/data/spells.ron` updated if additional entries are
      needed for the fixture
- [ ] `docs/explanation/implementations.md` updated with a summary of all four
      phases

#### 4.7 Success Criteria

- Validation catches a character definition that references a `SpellId` not
  present in the campaign's spell database.
- Integration path verified end-to-end: character defined with `starting_spells`
  in RON → `instantiate()` → `SpellBook` populated → Spell Book UI displays
  the spell → SDK validation passes.
- All four quality gates pass with zero warnings.

---

## Architecture Compliance Checklist

- [ ] `SpellId` type alias used throughout — never raw `u16` or `u32`
- [ ] `SpellBook` fields accessed only through the struct's accessor methods
      outside of the domain layer; no raw `cleric_spells[i].push()` calls
      outside `instantiate()` and `learning.rs`
- [ ] `GameMode::SpellBook` uses `Box<GameMode>` for previous mode — same
      pattern as `GameMode::SpellCasting(SpellCastingState)`
- [ ] All `spellbook_ui.rs` colors defined as `pub const` — no inline
      `Color::rgba(...)` literals
- [ ] `#[serde(default)]` on `starting_spells` in both `CharacterDefinition`
      and `CharacterDefinitionDef` for RON backward compatibility
- [ ] `#[serde(skip_serializing_if = "Vec::is_empty")]` on `starting_spells` so
      RON files without the field remain unchanged
- [ ] All new public functions, structs, and enums carry `///` doc comments
      with `# Examples` sections and `# Errors` sections where applicable
- [ ] Every egui `ScrollArea` in `spellbook_ui.rs` and
      `show_starting_spells_editor()` carries a unique `id_salt`
- [ ] Every loop-spawned egui row in the Spell Book UI and character editor
      spell section uses `ui.push_id(...)` (per `sdk/AGENTS.md`)
- [ ] No test in any phase references `campaigns/tutorial` — all fixtures live
      under `data/test_campaign/`
- [ ] `docs/explanation/implementations.md` updated after Phase 4 completion
