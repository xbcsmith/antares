# Party Member Dialogue — Talk Button Implementation Plan

## Overview

Add a **Talk** button to the character sheet's Single view so players can speak
with any recruited party member who has a `party_dialogue_id` assigned.  The
feature requires a new optional field on `Character` and `CharacterDefinition`,
eight short dialogue trees in the tutorial campaign, a game-state helper to
transition cleanly from `CharacterSheet` mode into `Dialogue` mode and back,
and the UI button + keyboard shortcut (`[T]`) wired into the existing character
sheet input system.

When `party_dialogue_id` is `None`, the button is still shown but clicking it
displays a brief inline fallback line rather than opening the dialogue system.
This keeps the button always-visible and teaches players that talking to
characters is a feature, even when there is currently nothing to say.

---

## Current State Analysis

### Existing Infrastructure

| Symbol | Location | Notes |
|--------|----------|-------|
| `Character::lore: Option<CharacterLore>` | `src/domain/character.rs` L1196 | `#[serde(default)]` — established pattern for optional character fields |
| `CharacterDefinition::lore_file: Option<String>` | `src/domain/character_definition.rs` L532 | Resolved at load time; copied to `Character::lore` at instantiation |
| `CharacterSheetState::showing_bio: bool` | `src/application/character_sheet_state.rs` L117 | Overlay flag toggled by `[B]` key — direct template for the Talk fallback flag |
| `render_single_view` Bio button | `src/game/systems/character_sheet_ui.rs` L594 | Shows conditionally when `character.lore.is_some()`; exact pattern to mirror |
| `character_sheet_input_system` `[B]` handler | `src/game/systems/character_sheet_ui.rs` L261 | Template for adding `[T]` |
| `GameState::enter_dialogue` | `src/application/mod.rs` L2525 | Sets `mode = GameMode::Dialogue(DialogueState::new())`; does not store resume mode |
| `GameMode::Dialogue` `close_modal` | `src/application/mod.rs` L2252 | Hardcoded to restore `GameMode::Exploration` — must be updated to support resume |
| `MerchantInventoryState::previous_mode` | `src/application/merchant_inventory_state.rs` L70 | Established pattern: store the mode to restore on close |
| `DialogueState::start(tree_id, root_node, pos, speaker)` | `src/application/dialogue.rs` L146 | Creates an active dialogue state for a specific tree |
| 8 premade characters | `campaigns/tutorial/data/characters.ron` | All have `lore_file`; none yet have `party_dialogue_id` |
| Lore source files | `campaigns/tutorial/assets/characters/lore/` | One `.ron` per character — content source for dialogue writing |

### Premade Character → Dialogue ID Assignment

| Character definition ID | Name | Dialogue ID |
|------------------------|------|------------|
| `tutorial_human_knight` | Kira | 201 |
| `tutorial_elf_mage` | Sirius | 202 |
| `tutorial_human_cleric_2` | Isolde | 203 |
| `tutorial_human_cleric` | Mira | 204 |
| `tutorial_dwarf_fighter` | Old Gareth | 205 |
| `tutorial_elf_archer` | Whisper | 206 |
| `tutorial_elf_sorcerer` | Apprentice Zara | 207 |
| `tutorial_human_monk` | Zhaya | 208 |

IDs 201–208 fall within the existing `DialogueId = u16` range; verify no
collision with existing tutorial dialogue IDs before assigning.

### Identified Issues

1. **No `party_dialogue_id` field** — neither `Character` nor `CharacterDefinition`
   carries this field yet.
2. **No resume path from Dialogue → CharacterSheet** — `close_modal` for
   `GameMode::Dialogue` unconditionally restores `GameMode::Exploration`, losing
   the character sheet context.
3. **No Talk button or `[T]` binding** — the character sheet input system and
   `render_single_view` must be extended.
4. **No party dialogue trees in tutorial** — 8 dialogue trees need to be authored.

---

## Implementation Phases

### Phase 1: Domain Types — `party_dialogue_id` on `Character` and `CharacterDefinition`

#### 1.1 Add `party_dialogue_id` to `Character`

In `src/domain/character.rs`, add to `pub struct Character` immediately after
the `lore` field (L1196), following the same `#[serde(default)]` pattern:

```
/// Party-conversation dialogue tree ID, assigned from the character definition.
/// `None` for characters with no authored in-party dialogue.
#[serde(default)]
pub party_dialogue_id: Option<DialogueId>,
```

`#[serde(default)]` ensures all existing save files and `characters.ron` entries
that lack the field continue to deserialise correctly.

#### 1.2 Add `party_dialogue_id` to `CharacterDefinition`

In `src/domain/character_definition.rs`, add to `pub struct CharacterDefinition`
alongside `lore_file`:

```
/// Optional in-party dialogue tree ID for the Talk button.
/// When `Some(id)`, the character can be spoken to from the character sheet.
#[serde(default)]
#[serde(skip_serializing_if = "Option::is_none")]
pub party_dialogue_id: Option<DialogueId>,
```

Also add to `struct CharacterDefinitionDef` (the intermediate deserialisation
struct at the end of the file) so the field round-trips through the RON loader.

#### 1.3 Propagate at Instantiation

In the `Character::from_definition` (or equivalent instantiation function),
copy `definition.party_dialogue_id` to `character.party_dialogue_id`.  This
mirrors exactly how `definition.lore` is copied to `character.lore`.

#### 1.4 Testing Requirements

- `test_character_party_dialogue_id_defaults_to_none` — a `Character` built
  without the field set has `party_dialogue_id == None`.
- `test_character_definition_party_dialogue_id_defaults_to_none` — same for
  `CharacterDefinition`.
- `test_from_definition_copies_party_dialogue_id` — definition with
  `party_dialogue_id = Some(201)` produces a character with the same value.
- Update the `CharacterDefinition` RON round-trip doctest to include the field
  (with `None`) to confirm it serialises cleanly.

#### 1.5 Deliverables

- [ ] `Character::party_dialogue_id: Option<DialogueId>` added (`#[serde(default)]`)
- [ ] `CharacterDefinition::party_dialogue_id: Option<DialogueId>` added
- [ ] `CharacterDefinitionDef::party_dialogue_id` added for deserialisation
- [ ] Instantiation propagates the field
- [ ] All Phase 1 tests pass; `cargo check` clean

#### 1.6 Success Criteria

A `CharacterDefinition` with `party_dialogue_id: Some(201)` round-trips through
RON serialisation and produces a `Character` with `party_dialogue_id == Some(201)`.
All existing save files and `characters.ron` files (which lack the field) continue
to load without error.

---

### Phase 2: Campaign Data — Party Dialogue Trees

Author the 8 in-party dialogue trees and assign dialogue IDs to the premade
characters in the tutorial campaign.  All test fixtures use `data/test_campaign`,
never `campaigns/tutorial` (Implementation Rule 5).

#### 2.1 Assign IDs in `campaigns/tutorial/data/characters.ron`

For each premade character entry (`is_premade: true`) add:

```ron
party_dialogue_id: Some(201),  // or 202–208 per the table above
```

Non-premade template entries (`is_premade: false`) are left without the field
(the `#[serde(default)]` handles the absence).

#### 2.2 Author 8 Dialogue Trees

Add 8 `DialogueTree` entries to the tutorial dialogues data file (or a new
`data/party_dialogues.ron` if the main dialogues file is large and the campaign
config already supports splitting):

- Each tree has `id: 201–208`, `repeatable: true`, and 2–4 nodes.
- Content draws from the corresponding lore file in
  `campaigns/tutorial/assets/characters/lore/`.
- Each tree has a root greeting node (node id 1) and at least one
  "tell me more" / "farewell" branch.
- A sensible short comment reflects where the party is in the main story — this
  can be simple for now (see **Out of scope**: state-conditional branches).

Example structure (Kira, id 201):
- Node 1 (root): greeting + 1 choice → Node 2 (farewell)
- Node 2: short closing line, no choices (ends dialogue)

#### 2.3 Add Minimal Fixture to `data/test_campaign/`

Add one `DialogueTree` entry (id 201, 2 nodes) to
`data/test_campaign/data/dialogues.ron` so the loader and runtime tests have
a party dialogue to exercise.

Add `party_dialogue_id: Some(201)` to the first premade character entry in
`data/test_campaign/data/characters.ron` (if such an entry exists there).

#### 2.4 Testing Requirements

- `test_tutorial_party_dialogue_ids_assigned_for_all_premades` — loads
  `data/test_campaign/data/characters.ron`; asserts the fixture character with
  `is_premade: true` has `party_dialogue_id == Some(201)`.
- `test_party_dialogue_201_exists_in_test_campaign` — loads
  `data/test_campaign/data/dialogues.ron`; asserts a tree with `id == 201`
  exists and is `repeatable`.

All test assertions use `data/test_campaign` paths (Rule 5).

#### 2.5 Deliverables

- [ ] `party_dialogue_id: Some(201..208)` set on all 8 premade characters in
      `campaigns/tutorial/data/characters.ron`
- [ ] 8 dialogue trees (ids 201–208) authored in tutorial campaign data
- [ ] `data/test_campaign` updated with minimal fixture (id 201 tree + one
      character with `party_dialogue_id: Some(201)`)
- [ ] All Phase 2 tests pass

#### 2.6 Success Criteria

Loading the test campaign produces a premade `CharacterDefinition` with
`party_dialogue_id == Some(201)` and a `DialogueTree` with `id == 201` that is
`repeatable == true`.

---

### Phase 3: Game State — `enter_party_dialogue` and Resume Path

Wire the mode transition so opening a party dialogue from the character sheet
returns cleanly to the character sheet on close.

#### 3.1 Add `resume_mode` to `DialogueState`

In `src/application/dialogue.rs`, add to `pub struct DialogueState`:

```
/// Mode to restore when this dialogue ends.
/// `None` means restore to `GameMode::Exploration` (current default).
/// Set to `Some(GameMode::CharacterSheet(_))` when opened from the character sheet.
#[serde(default)]
pub resume_mode: Option<Box<GameMode>>,
```

This mirrors the `previous_mode: Box<GameMode>` pattern used by
`MerchantInventoryState`.

#### 3.2 Update `close_modal` for `GameMode::Dialogue`

In `src/application/mod.rs`, update the `GameMode::Dialogue(_)` arm of
`close_modal`:

```rust
GameMode::Dialogue(dialogue_state) => {
    self.mode = dialogue_state.resume_mode
        .map(|m| *m)
        .unwrap_or(GameMode::Exploration);
    true
}
```

This is a pure backward-compatible change: existing code that creates a
`DialogueState` without setting `resume_mode` (i.e. everything today) continues
to restore `Exploration`, because `resume_mode` defaults to `None`.

#### 3.3 Add `GameState::enter_party_dialogue`

In `src/application/mod.rs`, add a new public method:

```
pub fn enter_party_dialogue(&mut self, focused_index: usize)
```

Logic:
1. Retrieve `character = self.party.members.get(focused_index)`.
2. Match on `character.party_dialogue_id`:
   - `Some(dialogue_id)`: create a `DialogueState` via `DialogueState::start(dialogue_id, 1, None, None)`.
     Set `dialogue_state.resume_mode = Some(Box::new(self.mode.clone()))`.
     Set `self.mode = GameMode::Dialogue(dialogue_state)`.
   - `None`: do nothing — the UI layer handles the fallback message (see Phase 4).

`root_node = 1` is the convention for all dialogue trees in the project.

#### 3.4 Testing Requirements

- `test_enter_party_dialogue_with_valid_id_sets_dialogue_mode` — party member
  with `party_dialogue_id = Some(201)` → `self.mode` becomes
  `GameMode::Dialogue(_)` and the active tree id is `201`.
- `test_enter_party_dialogue_stores_character_sheet_resume_mode` — when called
  while in `GameMode::CharacterSheet`, the resulting `DialogueState::resume_mode`
  is `Some(GameMode::CharacterSheet(_))`.
- `test_enter_party_dialogue_with_none_id_is_noop` — party member with
  `party_dialogue_id = None` → mode unchanged.
- `test_close_modal_dialogue_with_resume_mode_restores_it` — a `GameState` in
  `Dialogue` mode with `resume_mode = Some(CharacterSheet(...))` becomes
  `CharacterSheet` after `close_modal`.
- `test_close_modal_dialogue_without_resume_mode_restores_exploration` —
  existing behaviour preserved.

#### 3.5 Deliverables

- [ ] `DialogueState::resume_mode: Option<Box<GameMode>>` added (`#[serde(default)]`)
- [ ] `close_modal` for `Dialogue` uses `resume_mode` with `Exploration` fallback
- [ ] `GameState::enter_party_dialogue` implemented
- [ ] All Phase 3 tests pass; no regressions in existing `close_modal` tests

#### 3.6 Success Criteria

A game state in `CharacterSheet` mode that calls `enter_party_dialogue(0)` for
a character with dialogue id 201 enters `Dialogue` mode.  Calling `close_modal`
returns to `CharacterSheet`, not `Exploration`.  The same sequence for a character
with `party_dialogue_id = None` leaves the mode unchanged.

---

### Phase 4: Character Sheet UI — Talk Button and `[T]` Shortcut

Extend `render_single_view` and `character_sheet_input_system` to surface the
Talk button and handle the fallback message.

#### 4.1 Add `showing_talk_fallback: bool` to `CharacterSheetState`

In `src/application/character_sheet_state.rs`, add to `CharacterSheetState`:

```
/// Whether to show the "nothing to say" fallback message below the hint bar.
/// Set when Talk is pressed and `party_dialogue_id` is `None`.
/// Cleared on any navigation event (focus change, view toggle, mode exit).
#[serde(default)]
pub showing_talk_fallback: bool,
```

Add `pub fn clear_talk_fallback(&mut self)` (sets to `false`) and ensure
`focus_next`, `focus_prev`, `toggle_view`, and `toggle_bio` all clear it.

#### 4.2 Add the Talk Button to `render_single_view`

In `render_single_view` (`character_sheet_ui.rs` L587), add to the right-to-left
header button group, after the Bio button:

```rust
// Always shown; behaviour differs by whether party_dialogue_id is set.
let talk_label = "Talk";
if ui.small_button(talk_label).clicked() {
    if character.party_dialogue_id.is_some() {
        global_state.0.enter_party_dialogue(safe_index);
    } else {
        if let GameMode::CharacterSheet(ref mut cs) = global_state.0.mode {
            cs.showing_talk_fallback = true;
        }
    }
}
```

Add `[T] Talk` to the keyboard hint bar (always visible, not conditional on
dialogue ID being set — it teaches players the feature exists).

Add a fallback message below the hint bar, shown when
`showing_talk_fallback == true`:

```rust
if cs_state.showing_talk_fallback {
    ui.colored_label(STAT_EMPTY_COLOR, "They have nothing to say right now.");
}
```

#### 4.3 Add `[T]` to `character_sheet_input_system`

In `character_sheet_input_system` (L261), add a `KeyCode::KeyT` handler inside
the `is_single` branch, following the `[B]` Bio handler pattern:

```rust
if kb.just_pressed(KeyCode::KeyT) {
    let focused_index = /* read from cs_state */;
    let has_dialogue = global_state.0.party.members
        .get(focused_index)
        .is_some_and(|c| c.party_dialogue_id.is_some());
    if has_dialogue {
        global_state.0.enter_party_dialogue(focused_index);
    } else {
        if let GameMode::CharacterSheet(ref mut cs) = global_state.0.mode {
            cs.showing_talk_fallback = true;
        }
    }
    return;
}
```

`[T]` is a no-op in Party Overview view (only Talk in Single view, matching how
`[B]` for Bio works).

#### 4.4 Testing Requirements

- `test_character_sheet_input_t_key_enters_dialogue_when_party_dialogue_set`
- `test_character_sheet_input_t_key_sets_fallback_when_no_party_dialogue`
- `test_character_sheet_input_t_key_is_noop_in_party_overview_view`
- `test_render_single_view_talk_button_always_visible`
- `test_render_single_view_talk_fallback_message_shown_when_flag_set`
- `test_showing_talk_fallback_cleared_on_focus_change` — `focus_next` clears
  `showing_talk_fallback`.
- `test_character_sheet_state_showing_talk_fallback_default_is_false`

#### 4.5 Deliverables

- [ ] `CharacterSheetState::showing_talk_fallback` field added
- [ ] `clear_talk_fallback` called from `focus_next`, `focus_prev`, `toggle_view`,
      `toggle_bio`
- [ ] Talk button added to `render_single_view` header button row
- [ ] `[T]` hint added to keyboard hint bar
- [ ] Fallback message rendered when `showing_talk_fallback == true`
- [ ] `character_sheet_input_system` handles `KeyCode::KeyT`
- [ ] All Phase 4 tests pass; `cargo nextest run --all-features` fully green
- [ ] `docs/explanation/implementations.md` updated

#### 4.6 Success Criteria

A player in `CharacterSheet` mode with a character that has `party_dialogue_id =
Some(201)` clicks **Talk** (or presses `[T]`) and the game transitions to
`GameMode::Dialogue` showing tree 201.  Pressing `[Esc]` ends the dialogue and
returns to the character sheet at the same focused character.  For a character
with no `party_dialogue_id`, clicking Talk shows "They have nothing to say right
now." inline without leaving the character sheet.

---

## Affected Files Summary

| File | Phase | Change |
|------|-------|--------|
| `src/domain/character.rs` | 1 | Add `party_dialogue_id: Option<DialogueId>` to `Character` |
| `src/domain/character_definition.rs` | 1 | Add `party_dialogue_id` to `CharacterDefinition` + `CharacterDefinitionDef`; propagate at instantiation |
| `src/application/dialogue.rs` | 3 | Add `resume_mode: Option<Box<GameMode>>` to `DialogueState` |
| `src/application/mod.rs` | 3 | Update `close_modal` Dialogue arm; add `enter_party_dialogue` |
| `src/application/character_sheet_state.rs` | 4 | Add `showing_talk_fallback`; `clear_talk_fallback`; clear on nav |
| `src/game/systems/character_sheet_ui.rs` | 4 | Talk button in header; `[T]` hint bar; fallback message; `[T]` key handler |
| `campaigns/tutorial/data/characters.ron` | 2 | Add `party_dialogue_id: Some(201–208)` to 8 premade entries |
| `campaigns/tutorial/data/dialogues.ron` | 2 | Add 8 dialogue trees (ids 201–208) |
| `data/test_campaign/data/characters.ron` | 2 | Add `party_dialogue_id: Some(201)` to fixture premade entry |
| `data/test_campaign/data/dialogues.ron` | 2 | Add minimal fixture dialogue tree (id 201) |
| `docs/explanation/implementations.md` | 4 | Implementation summary |

---

## Decisions Log

| Question | Decision |
|----------|----------|
| Talk button: always shown or conditional? | Always shown — teaches the feature; fallback message for `None` case |
| Return mode after Talk dialogue closes? | `DialogueState::resume_mode` — same pattern as `MerchantInventoryState::previous_mode`; `Exploration` fallback when not set |
| `party_dialogue_id` root node assumption? | `root_node = 1` — consistent with all existing dialogue trees |
| Fallback message location? | Inline below the hint bar inside Single view — no toast/popup needed |
| `[T]` key active in Party Overview? | No — mirrors `[B]` Bio; Talk only in Single view |
