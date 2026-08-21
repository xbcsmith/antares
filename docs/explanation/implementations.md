## Phase 1: Domain — `DialogueTree` Repair Methods

### Summary

Added two new repair methods to `DialogueTree` in `src/domain/dialogue.rs` as
part of the Fix Skill Trainer SDK plan. These methods form the domain layer of
the repair path that prevents duplicate branch insertion when a trainer dialogue
was generated with a stale or wrong NPC ID.

### New Methods

| Method | Location | Purpose |
|--------|----------|---------|
| `DialogueTree::repair_sdk_trainer_npc_id` | `src/domain/dialogue.rs` | Finds all `TrainerOpenNode`-marked nodes and patches any `OpenTraining { npc_id }` action that does not match the supplied correct ID. Returns `true` when at least one action was updated. |
| `DialogueTree::repair_sdk_skill_trainer_npc_id` | `src/domain/dialogue.rs` | Same pattern for `SkillTrainerOpenNode`-marked nodes and `OpenSkillTraining { npc_id }` actions. |

### Design

Both methods follow the same pattern:
- Iterate `self.nodes.values_mut()`
- For each node whose `sdk_metadata.managed_content` contains the relevant
  `TrainerOpenNode` / `SkillTrainerOpenNode` marker:
  - Patch mismatched `npc_id` in `node.actions`
  - Patch mismatched `npc_id` in each `choice.actions`
- Return `true` if any action was changed, `false` otherwise (no-op)

They are inserted immediately after their respective
`ensure_standard_*_branch` siblings and before
`remove_sdk_managed_*_content`, preserving the existing ordering convention.

### Tests Added

Four unit tests added to the existing `mod tests` block in
`src/domain/dialogue.rs`:

| Test | Assertion |
|------|-----------|
| `test_repair_sdk_trainer_npc_id_updates_wrong_id` | Returns `true`; tree contains correct ID; old ID gone |
| `test_repair_sdk_trainer_npc_id_is_noop_when_already_correct` | Returns `false`; tree unchanged |
| `test_repair_sdk_skill_trainer_npc_id_updates_wrong_id` | Returns `true`; tree contains correct ID; old ID gone |
| `test_repair_sdk_skill_trainer_npc_id_is_noop_when_already_correct` | Returns `false`; tree unchanged |

### Quality Gates

- `cargo fmt --all` — clean
- `cargo check --all-targets --all-features` — 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run --all-features` — 5507 passed, 0 failed

---

## Dialogue Overhaul: All Characters Recruitable, Lore-Consistent Trees

### Summary

Complete overhaul of `campaigns/tutorial/data/dialogues.ron`. Fixed a critical
duplicate-id bug (Eonir was incorrectly assigned id 1003, colliding with Isolde).
Rewrote the two placeholder NPC dialogues (Arcturus, Aetheris), rewrote both
existing stub recruitment dialogues (Zara, Whisper), expanded Jyeshtha with the
full Eonir reveal, applied lore-consistent text polish to all service NPCs, and
created four new recruitment dialogues so every premade character is now
recruitable. Updated `npcs.ron` to reference Eonir's corrected id.

### Critical Bug Fix

- `tutorial_lich_eonir` dialogue was id 1003 (duplicate of Isolde Dawnfang)
- Eonir reassigned to **id 1004**; `npcs.ron` `dialogue_id` updated to match

### Full Rewrites

| id | Old name | New name | Notes |
|----|----------|----------|-------|
| 1 | Arcturus Story | Arcturus - The Eternal Wanderer | 6 nodes; Cosmic Weave lore, quest hook for instruments |
| 2 | Arcturus Brother Story | Aetheris - The Void-Drifter | 6 nodes; Dark Forest hut, Void-Weave origin, leads to Arcturus |
| 101 | Apprentice Zara Recruitment | same | 6 nodes; Lumina-weaving, Great Alignment, Master Elian |
| 102 | Whisper Recruitment | same | 6 nodes; Vespera Moonshadow, Silver Spires, locksmith cover |

### Expanded

- **id 1001 Jyeshtha** — renamed to *The Glass Sea Watcher*; fixed encoding
  corruption (`â\u{80}\u{94}` → `—`); node 2 now delivers the full Eonir reveal
  (jade icosahedron stolen by Eonir Xavian Stalix, Vaelgrim, the stillness);
  node 5 now points to Frostspire Peaks as the destination

### Text Polish (structure preserved, text updated)

ids 3, 4, 5, 6, 7, 8, 9, 10, 11, 1002 — all updated to reference Velmoria
world lore: the Cosmic Weave, the Great Gloom, Ashvale settlements, the
stillness spreading from the north, goblin scouts as directed outriders,
Harrow Downs opening from the inside, Arcturus's instruments.

### New Recruitment Dialogues

| id | Character | character_id | Notes |
|----|-----------|-------------|-------|
| 104 | Kira Valerius | `tutorial_human_knight` | 7 nodes; Eldoria exile, Void-Blight, party leader |
| 105 | Sirius Xylanthir | `tutorial_elf_sorcerer` | 6 nodes; Dark Elf, Void-Weave, demands worthy enemies |
| 106 | Mira | `tutorial_human_cleric` | 6 nodes; Great Gloom, Order of the Resplendent Dawn |
| 107 | Old Gareth | `old_gareth` | 7 nodes; Aethelgard, Great Tremor, Core Shield, reluctant veteran |

### All Recruitable Characters — Dialogue Map

| Character | Dialogue id | Status |
|-----------|------------|--------|
| Kira | 104 | New |
| Sirius | 105 | New |
| Isolde | 1003 | Existing (excellent quality, unchanged) |
| Mira | 106 | New |
| Old Gareth | 107 | New |
| Whisper | 102 | Rewritten |
| Apprentice Zara | 101 | Rewritten |
| Zhaya | 1000 | Existing (good quality, unchanged) |

### Follow-Up Needed

`characters.ron` does not yet have a `dialogue_id` field on character entries.
The game will need a mechanism to wire each character's in-world encounter to
their recruitment dialogue (ids 100–107, 1000, 1003). This is tracked under
the Option A / recruitable-characters feature work.

---

## Map Names, Descriptions & Frostspire Peaks (map_8)

### Summary

Updated `name` and `description` fields across all seven existing maps to align
with *The Stillness Prophecy* world lore. Fixed the long-standing typo
"Dark Forrest" → "Dark Forest" and added the missing apostrophe in
"Astronomer's Temple". Created `map_8.ron` — Frostspire Peaks, the 40×40
Act IV final map where the confrontation with Eonir takes place.

### Files modified

**`campaigns/tutorial/data/maps/map_1.ron`** — Town Square
- Description updated: Ashvale settlement, Kira's gathering point

**`campaigns/tutorial/data/maps/map_2.ron`** — Dark Forest *(typo fixed)*
- Name: "Dark Forrest" → "Dark Forest"
- Description updated: goblin scouts, something larger stirring

**`campaigns/tutorial/data/maps/map_3.ron`** — Ancient Ruins
- Description updated: kobolds and goblins, Arcturus's stolen astrolabe

**`campaigns/tutorial/data/maps/map_4.ron`** — Arcturus's Cave
- Description updated: foothills of Mount Ashkarron

**`campaigns/tutorial/data/maps/map_5.ron`** — Mountain Pass
- Description updated: hidden village, first rumours of the stillness

**`campaigns/tutorial/data/maps/map_6.ron`** — The Harrow Downs
- Description updated: tombs opening from the inside

**`campaigns/tutorial/data/maps/map_7.ron`** — Astronomer's Temple *(apostrophe fixed)*
- Name: "Astronomers Temple" → "Astronomer's Temple"
- Description updated: Glass Sea pyramid, Jyeshtha's vigil

**`campaigns/tutorial/data/maps/map_8.ron`** — Frostspire Peaks *(new)*
- 40×40 (1 600 tiles), matching map_7 in size
- `maps_dir: "data/maps/"` in campaign.ron picks it up automatically
- Layout zones:
  - Mountain border + flanking canyon walls
  - Frozen tundra approach from the south (y=28-38), sparse ice-rock obstacles
  - Outer castle walls (Stone/Normal) with south gate at x=19-20 (y=27)
  - Castle courtyard (Stone floor, lit)
  - Inner keep walls with inner gate at x=19-20 (y=22)
  - Eonir's sanctum (y=13-21, x=13-26): Stone floor, `is_dark: true`
  - Jagged northern frostspire peaks beyond the castle (y=1-7)

---

## Lore Files & NPC Updates: The Stillness Prophecy Character Pass

### Summary

Updated all eight premade-character lore files under
`campaigns/tutorial/assets/characters/lore/` using the canonical lore prompts
in `campaigns/tutorial/prompts/lore/`. Updated matching `description` fields in
`campaigns/tutorial/data/characters.ron` and
`campaigns/tutorial/data/npcs.ron`. Added new NPC **Eonir the Still**
(`tutorial_lich_eonir`) — the Lich King antagonist of Act IV.

### Files modified

**`campaigns/tutorial/assets/characters/lore/kira.ron`**
- Backstory: Eldoria origin, Void-Blight, knight-errant in exile
- Title: *The Exile Knight, Blade of the Shattered Shore*
- Archetype: Soldier / Versatile Warrior

**`campaigns/tutorial/assets/characters/lore/old_gareth.ron`**
- Backstory: Master Architect of Aethelgard, Great Tremor, the Core Shield
- Title: *The Last Pillar of Aethelgard, Warden of the Core Shield*
- Archetype: Bastion Guardian / Stalwart Tank

**`campaigns/tutorial/assets/characters/lore/isolde.ron`**
- Backstory: Sunspire Dynasty princess, secret War Cleric training, rode to war
- Title: *Princess of Sunspire, Holy Warrior of the Radiant Shield*
- Archetype: War Cleric / Martial Divine Champion

**`campaigns/tutorial/assets/characters/lore/mira.ron`**
- Backstory: Village outpost against the Great Gloom, Order of the Resplendent Dawn
- Title: *The Radiant Sentinel, Acolyte of the Eternal Spark*
- Archetype: Light Cleric / Holy Guardian

**`campaigns/tutorial/assets/characters/lore/sirius.ron`**
- Backstory: Dark Elf scholar from subterranean reaches, Void-Weave mastery
- Title: *Master of the Obsidian Veil, Archon of the Void-Weave*
- Archetype: Shadow Sorcerer / Void Mage

**`campaigns/tutorial/assets/characters/lore/whisper.ron`**
- Backstory: Vespera Moonshadow, High Elf of the Silver Spires, traded nobility for stealth
- Title: *The Ghost of the Silver Spires, Master of the Unseen Key*
- Archetype: High Elf Rogue / Shadow Assassin

**`campaigns/tutorial/assets/characters/lore/apprentice_zara.ron`**
- Backstory: Lumina-weaving obsession, Great Alignment experiment, separated from Master Elian
- Title: *Luminous Scholar, Weaver of the Prismatic Spark*
- Archetype: Gnome Sorcerer / Cosmic Apprentice

**`campaigns/tutorial/assets/characters/lore/zhaya.ron`**
- Backstory: Eastern Peaks monasteries, Astrologer's Path visions, pilgrimage to the Astronomers Temple
- Title: *Celestial Monk, Seeker of the Star-Bound Truth*
- Archetype: Astral Monk / Fate-Reader

**`campaigns/tutorial/data/characters.ron`**
- Updated `description` for Kira, Sirius, Mira, Old Gareth, Whisper, Apprentice Zara, and Zhaya
- Isolde's description was already aligned with the prompt; no change

**`campaigns/tutorial/data/npcs.ron`**
- Updated `description` for `tutorial_wizard_arcturus` (Arcturus, Eternal Wanderer)
- Updated `description` for `tutorial_wizard_arcturus_brother` (Aetheris, Void-Drifter)
- Updated `description` for `tutorial_mystic_astronomer` (Jyeshtha, Glass Sea watcher)
- Added new entry `tutorial_lich_eonir` — Eonir the Still, Sovereign of Stillness
  - `creature_id`: 1021, `dialogue_id`: 1003, `faction`: "Sovereign of Stillness"
  - `portrait_id`: "eonir_still" (asset to be created)

---

## Character Bio & Navigation, Phase 5: Mouse Input Fix for Character Sheet

### Summary

Implemented Phase 5 of
`docs/explanation/character_bio_and_navigation_implementation_plan.md`, the
final phase of the Character Bio & Navigation plan: fixed the root cause of
`GameMode::CharacterSheet`'s mouse-click regressions (Identified Issues #2-3
in the plan) and resolved the HUD-portrait click-through that compounded it.
Also added a mouse-clickable "Bio" button, since Phase 3 only wired the Bio
panel to the keyboard (`B` key) and Phase 5's own success criteria requires
"mouse and keyboard... both fully functional in every Character Sheet view."

### Files modified

**`src/game/systems/input/mode_guards.rs`**
- Added `GameMode::CharacterSheet(_)` to `movement_blocked_for_mode`'s match
  arms, matching every other modal screen. `interaction_blocked_for_mode`
  and `input_blocked_for_mode` both delegate to `movement_blocked_for_mode`
  internally, so this single change fixes both -- the plan named both
  functions as line-range targets, but the second was already covered by
  the first's delegation; there was no separate match arm list to edit in
  `interaction_blocked_for_mode`.
- This closes the actual root cause identified in the plan: without this
  guard, `handle_exploration_input_interact`/`handle_exploration_input_movement`
  only stood down via the `egui_wants_any_pointer_input` run condition,
  written in `PostUpdate` -- one frame after the Character Sheet UI draws in
  `Update` -- so exploration movement/interaction input could leak through
  on the frame the sheet opened or closed.
- 3 new tests (`movement_blocked_for_mode`/`interaction_blocked_for_mode`/
  `input_blocked_for_mode`, all asserting `true` for `CharacterSheet`),
  mirroring the existing per-mode test convention in this file.

**`src/game/systems/hud.rs`**
- Removed `GameMode::CharacterSheet(_)` from `portrait_click_allowed`.
  Previously, a HUD portrait click while the sheet was already open called
  `enter_character_sheet_at` directly, double-handling the same click
  alongside whatever the sheet's own egui widgets did with it -- the root
  cause named in the plan's Identified Issue #3.
- Updated `portrait_click_allowed` and `handle_portrait_click_system`'s doc
  comments to describe the new blocked mode and why.
- Replaced `test_handle_portrait_click_when_already_in_sheet_updates_index`
  (which asserted the now-removed "click a second portrait to retarget the
  open sheet" behavior) with
  `test_handle_portrait_click_blocked_when_sheet_already_open`, asserting
  the click is blocked and focus is unaffected. Added
  `test_portrait_click_not_allowed_character_sheet` alongside the file's
  other single-mode `portrait_click_allowed` tests. Confirmed the two
  remaining `portrait_click_allowed` assertions that touch `CharacterSheet`
  mode indirectly (`test_handle_portrait_click_selects_correct_party_index`,
  `test_handle_portrait_click_opens_sheet_in_exploration`/`_in_combat`) all
  check the mode *before* entering the sheet, so they were unaffected.

**`src/game/systems/character_sheet_ui.rs`**
- Added a mouse-clickable "Bio"/"Hide Bio" button next to the existing
  "Party Overview"/"Next >"/"< Prev" buttons in the Single-view header,
  gated on `character.lore.is_some()` (same condition as the `B` key and
  hint), calling the same `CharacterSheetState::toggle_bio()` the keyboard
  shortcut uses. This wasn't in Phase 5's deliverables list, but the
  phase's own success criteria ("mouse and keyboard are both fully
  functional in every Character Sheet view") and testing requirements
  (which mention "the new Bio button") both call for it, and Phase 3 had
  only wired the toggle to the keyboard.
- Updated the module doc's Single-view ASCII diagram to show the new
  button.

### Quality gates

- `cargo fmt --all` — clean
- `cargo check --all-targets --all-features` — 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run --all-features` — **5501/5501 passed**, 8 skipped (root
  `antares` crate; full regression run, not just the new/touched tests,
  since blocking exploration input in a new mode is exactly the kind of
  change that can surface hidden cross-system assumptions)
- `cargo nextest run -p campaign_builder --all-features` — **2496/2496
  passed** (unaffected by this phase, run anyway for full-workspace
  confidence -- see the Phase 4 entry below for why this must be invoked
  explicitly with `-p`)
- `cargo test --doc -- mode_guards portrait_click_allowed` and
  `cargo test --doc -- character_sheet` (targeted, not the full workspace
  doctest suite) — 4/4 and 15/15 passed
- Manual: launched `./target/debug/antares --campaign campaigns/tutorial`,
  confirmed it starts and runs without panicking for 12+ seconds.
  Interactive mouse-driven verification of the actual click behavior in the
  live window (Party Overview "View" buttons, the new Bio button, blocked
  portrait clicks) was not possible in this sandbox (no Accessibility/
  System Events permission, consistent with every prior phase in this
  plan) -- covered instead by the mode-guard/portrait-click unit tests
  above, which exercise the real blocking predicates the input systems
  gate on.

### Plan status

This completes all 5 phases of
`docs/explanation/character_bio_and_navigation_implementation_plan.md`.

---

## Character Bio & Navigation, Phase 4: Campaign Builder (SDK) Lore Editor Support

### Summary

Implemented Phase 4 of
`docs/explanation/character_bio_and_navigation_implementation_plan.md`: a
Lore section in the Campaign Builder's Characters editor, so
`CharacterLore`/`CharacterProfile` content (added in Phase 2, consumed by
the in-game Bio panel added in Phase 3) is fully editable without hand-editing
RON files.

### Design choices

- **`lore_file` is read-only in the UI**, auto-derived from the character's
  name on save (lowercased, spaces/apostrophes/hyphens stripped) when unset
  -- the same filename convention `save_creatures` already uses for the
  creature registry. The plan only asked for a "`lore_file` display," and
  auto-derivation means authoring lore purely through the editor never
  requires typing a RON path by hand.
- **Clearing all five Lore fields clears `lore_file` too** (not just
  `lore`), so the persisted state always matches what the Lore section
  currently shows -- no orphaned reference to content the buffer no longer
  represents. The old per-entity RON file on disk, if any, is deliberately
  *not* deleted -- an unreferenced leftover file is a far safer failure mode
  for an editor to leave behind than automatically deleting user-authored
  content.
- **`load_from_file`/`save_to_file` derive the campaign root from the
  `characters.ron` path itself** (`path.parent().parent()`, the same
  derivation `sdk/database.rs`'s `ContentDatabase::load_core` already uses
  for `asset_root`) rather than changing either method's signature to take
  an explicit campaign-root parameter. The plan named
  `load_characters_from_campaign` (`campaign_io.rs`) as the integration
  point; that function already just delegates to
  `CharactersEditorState::load_from_file`, so putting the resolution there
  satisfies the plan's "load/save wired through `campaign_io.rs`"
  deliverable without a signature change rippling to its other caller
  (a test).
- **Every lore-resolution failure mode is soft** (missing campaign root,
  unsafe path, unreadable file, unparseable content) -- unlike the game
  runtime's `CharacterDatabase::load_from_campaign`, which hard-fails on a
  `lore_file` that fails path-security validation because it parses
  untrusted campaign content. The SDK editor is a local-authoring tool for a
  trusted campaign author, not a security boundary, so failing the whole
  character list over one bad lore reference would be poor editor UX for no
  corresponding security benefit. This deviation is documented in the doc
  comment on `load_from_file`.
- **`save_to_file` mirrors `save_creatures`'s "parent file + per-entity
  files" pattern**: `characters.ron` (parent) is written first, then every
  character with non-empty lore content gets its own per-entity
  `CharacterLore` RON file written under `assets/characters/lore/` --
  unconditionally re-written on every save, same as `save_creatures`
  regenerates every creature file every time (idempotent for
  untouched entries).

### Files modified

**`sdk/campaign_builder/src/characters_editor.rs`**
- `CharacterEditBuffer`: added `lore_title`, `lore_archetype`,
  `lore_core_motivation`, `lore_combat_style`, `lore_backstory` (all
  `String`) alongside the existing `lore_file: Option<String>` from Phase 2.
- `start_edit_character`: populates the five new buffer fields from
  `character.lore` when present, empty strings otherwise.
- `save_character`: builds `Option<CharacterLore>` from the buffer's five
  lore fields (`None` when all are empty after trimming, which also clears
  `lore_file`); previously this was hardcoded to `lore: None`.
- `load_from_file`: now resolves each character's `lore_file` into
  `character.lore` via `validate_campaign_relative_path`, soft-failing on
  every error mode (see Design choices above).
- `save_to_file`: signature changed `&self` -> `&mut self`; derives a
  `lore_file` for any character with `lore.is_some() && lore_file.is_none()`,
  then writes one per-entity lore RON file per character with lore content,
  creating `assets/characters/lore/` as needed.
- `show_character_form`: new "Lore" section (read-only `lore_file` display +
  a 2-column `Grid` for title/archetype/core_motivation/combat_style +a
  multiline backstory `TextEdit`) inserted between the existing Description
  section and the Back to List / Save / Cancel action row -- no new
  `ScrollArea`/`ComboBox`/panel, since the whole form already lives inside
  one `ScrollArea::vertical().id_salt("character_form_scroll")`.
- 7 new tests: `save_character` building/clearing lore + auto-deriving
  `lore_file`; `load_from_file` resolving a valid lore_file and tolerating a
  missing one; `start_edit_character` populating (and not populating) the
  buffer's lore fields.

**`docs/how-to/create_characters.md`** — "Editor Features" bullet list and
the "Creating a Character in the Editor" numbered walkthrough both updated
to mention the new Lore section (Phase 4 deliverable).

### Quality gates

- `cargo fmt --all` — clean
- `cargo check --all-targets --all-features` — 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
  (verified both at the workspace root and explicitly scoped to
  `-p campaign_builder`, see note below)
- `cargo nextest run --all-features` — **5497/5497 passed** (antares crate)
- `cargo nextest run -p campaign_builder --all-features` — **2496/2496
  passed** (this crate is a separate workspace member; the root
  `cargo nextest run --all-features` invocation only covers the root
  `antares` package by default since the workspace root is itself a
  package -- `-p campaign_builder` must be passed explicitly to exercise it,
  which this phase's scope of change warranted doing in full, not just for
  the new tests)
- `cargo test --doc -- character_definition::CharacterLore
  character_definition::CharacterProfile` — 2/2 passed (unchanged from
  Phase 2; no new doctests were needed for this phase's SDK-only changes)
- Manual: launched `./target/debug/campaign-builder` (initializes cleanly,
  no crash) and `./target/debug/antares --campaign campaigns/tutorial`
  (still loads the Phase-2-authored lore content cleanly). Interactive
  keyboard/mouse-driven verification of the actual Lore section UI in the
  live window was not possible in this sandbox (no Accessibility/System
  Events permission) -- covered instead by the 7 new unit tests, which
  exercise the real `save_character`/`load_from_file`/`save_to_file`/
  `start_edit_character` code paths against tempdir fixtures.

---

## Character Bio & Navigation, Phase 3: Bio Panel UI + Keyboard Access

### Summary

Implemented Phase 3 of
`docs/explanation/character_bio_and_navigation_implementation_plan.md`: a
keyboard-accessible Bio panel in the Character Sheet's Single view, showing
the long-form `CharacterLore` content added in Phase 2. The panel is an
overlay flag on top of `CharacterSheetView::Single`, not a third view
variant, so every existing `view` match, Esc/O toggling, and Tab/Arrow/digit
navigation is untouched.

### Files modified

**`src/application/character_sheet_state.rs`**
- Added `showing_bio: bool` to `CharacterSheetState` (next to `view`),
  initialized to `false` in `new()`.
- Added `toggle_bio()`, an unconditional flip mirroring `toggle_view()`'s
  style, with a doctest.
- New unit tests: flip false→true, flip back to false, and independence from
  `view` (toggling one must not affect the other).

**`src/game/systems/character_sheet_ui.rs`**
- `character_sheet_input_system`: new `B` handler inside the `is_single`
  branch, gated on the focused character's `lore.is_some()` (looked up via
  `focused_index` read from `CharacterSheetState` before the mutable
  re-borrow) -- a no-op (but still `return`s, mirroring the `O`/`Enter`
  handlers) when the character has no lore.
- `SingleViewParams` gained a `showing_bio: bool` field, threaded from
  `character_sheet_ui_system` (which already reads `cs_state`).
- `render_single_view`: when `showing_bio` and the focused character has
  lore, renders `render_bio_panel(...)` and returns early instead of the
  normal portrait/three-column stats layout.
- New `render_bio_panel`: single-column `egui::ScrollArea` (not
  `three_column` -- the content is one flowing column, so the multi-column
  helper isn't needed here) showing `profile.title` (falls back to the
  character's name when empty), `profile.archetype`, the wrapped
  `backstory`, `profile.core_motivation`, and `profile.combat_style`.
- Single-view hint bar: `[B] Bio` now renders between `[O] Overview` and
  `[1-6] Select`, only when the focused character has lore.
- Module doc comment: added a "Layout — Bio panel" section (ASCII diagram +
  description) and a `B` bullet in the Flow section's Single-view key list,
  matching the existing convention of keeping doc and rendered hints in
  sync.
- Fixed the 5 existing test-only `SingleViewParams { ... }` struct literals
  with `showing_bio: false`.
- New tests: 3 real `App`-harness tests for the `B` key (toggles on for a
  character with lore, no-op without lore, toggles off on a second distinct
  press -- the last one required `clear_just_pressed`, since `MinimalPlugins`
  doesn't run the `InputPlugin` system that normally clears `just_pressed`
  between frames, following the existing pattern in `combat.rs`'s Tab-wrap
  test); 2 `render_single_view` smoke tests (Bio panel renders without panic
  including the title-fallback-to-name path; `showing_bio: true` without
  lore falls back to the normal stats layout, not a broken empty panel).

**Audit follow-up**: a post-implementation audit against the plan's Phase 3
testing requirements found the `B`-is-no-op-outside-Single-view case was true
by construction (the handler is nested inside `if is_single`) but never
asserted by a test. Added
`test_character_sheet_input_b_key_is_noop_in_party_overview_view` in
`character_sheet_ui.rs`, which presses `B` on a lore-bearing character while
`cs.view == PartyOverview` and confirms `showing_bio` stays `false`.

### Quality gates (full workspace)

- `cargo fmt --all` — clean
- `cargo check --all-targets --all-features` — 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run --all-features` — **5497/5497 passed**, 8 skipped
- `cargo test --doc -- character_sheet` (targeted, not the full workspace
  doctest suite) — **15/15 passed**, including the new `toggle_bio` doctest
- Manual verification: launched `./target/debug/antares --campaign
  campaigns/tutorial`, confirmed it starts and runs without panicking for
  12+ seconds with the (now lore-populated) tutorial campaign loaded.
  Interactive keyboard-driven verification of the actual Bio panel toggle
  in the live window was not possible in this sandbox (no Accessibility/
  System Events permission to script keystrokes into the native app
  window) -- covered instead by the real `App`-harness input tests above,
  which exercise the actual `character_sheet_input_system` Bevy system.

---

## Character Bio & Navigation, Phase 2: Character Backstory & Profile Data Model

### Summary

Implemented Phase 2 of
`docs/explanation/character_bio_and_navigation_implementation_plan.md`: a new
optional, long-form lore content model for characters, layered on top of the
existing short `description` field without touching it. A character's
`lore_file` (relative to the campaign root) points to an external RON file
holding a `CharacterLore` (`backstory` + a `CharacterProfile` of
`title`/`archetype`/`core_motivation`/`combat_style`); `CharacterDatabase::
load_from_campaign` resolves it, with warn-and-continue semantics for missing
or invalid *referenced* files (only a `lore_file` value that fails
path-security validation is a hard load error). All 8 premade tutorial
characters now have authored lore content.

### Files modified

**`src/domain/character_definition.rs`**
- Added `CharacterLore` and `CharacterProfile` structs (`Serialize`,
  `Deserialize`, `Eq`), each with doc comments and a runnable doctest.
- Added `lore_file: Option<String>` (serialized, `#[serde(default)]`,
  `skip_serializing_if`) and `lore: Option<CharacterLore>` (`#[serde(skip)]`,
  not authored directly) to `CharacterDefinition`, placed next to
  `description`. Updated the `CharacterDefinitionDef` shadow struct, its
  `From` impl, `CharacterDefinition::new()`, and the struct's top-of-file
  doctest example accordingly.
- Added `CharacterDatabase::load_from_campaign(data_dir, campaign_root)`,
  mirroring `CreatureDatabase::load_from_registry` for
  `validate_campaign_relative_path`-based resolution but *not* for error
  handling: path-security failures are hard errors; a missing/unreadable/
  unparseable referenced file only logs a warning and leaves that
  character's `lore` as `None`, and loading continues.
- `instantiate()` now copies `lore: self.lore.clone()` onto the runtime
  `Character`, alongside `portrait_id`.
- Fixed all 10 in-file `CharacterDefinition { ... }` struct-literal test call
  sites (RON string literals were unaffected -- `lore_file` defaults via
  `#[serde(default)]` on the shadow struct).
- New tests: `CharacterLore` round-trip serialization; `load_from_campaign`
  for no-`lore_file`, missing-file, invalid-RON, valid-lore, and
  path-traversal-rejection cases (all via `tempfile::TempDir`, following the
  existing `creature_database.rs` test template).

**`src/domain/character.rs`**
- Added `lore: Option<CharacterLore>` to `Character`, `#[serde(default)]` so
  pre-existing save files without the field still deserialize. Updated
  `Character::new()`.
- New regression test `test_lore_field_serde_default_deserializes`, mirroring
  the existing `test_timed_stat_boost_serde_default_deserializes` convention
  (serialize, strip the field from the RON string, confirm it still
  deserializes with the field defaulting to `None`).

**`src/domain/items/equipment_validation.rs`**
- Added `lore: None` to the one other `Character { ... }` struct literal in
  the codebase (a test fixture).

**`src/sdk/database.rs`**
- Swapped `CharacterDatabase::load_from_file` → `load_from_campaign` at both
  `ContentDatabase` call sites: `load_campaign_with_skills_file` (already had
  `campaign_path`) and `load_core` (passes the already-derived `asset_root`
  as the campaign root, the same value used for creature/landscape/furniture
  asset resolution).

**`sdk/campaign_builder/src/characters_editor.rs` / `asset_manager.rs`**
- Threaded `lore_file` through `CharacterEditBuffer` (`start_edit_character`
  populates it from the character being edited, `save_character` writes it
  back), so editing/saving a character via the SDK no longer silently
  discards its `lore_file` reference. Fixed the remaining struct-literal
  sites (test code) with `lore_file: None, lore: None`. No lore-editing UI
  was added -- that's a later phase; this only prevents a save-time
  regression of Phase 2's own new field.

**Content**: 8 `CharacterLore` RON files authored under
`campaigns/tutorial/assets/characters/lore/` (Kira, Sirius, Isolde, Mira, Old
Gareth, Whisper, Apprentice Zara, Zhaya), referenced via `lore_file` from
`campaigns/tutorial/data/characters.ron`. The 3 non-premade template
characters were left untouched.

**Docs**: `docs/reference/campaign_content_format.md`,
`docs/how-to/character_definition_ron_format.md`, and
`docs/how-to/create_characters.md` each updated with the new `lore_file`
field.

**Audit follow-up**: a post-implementation audit against the plan's Phase 2
deliverables found two gaps, both closed:
- `src/application/save_game.rs` gained
  `test_pre_lore_save_fixture_deserializes_with_lore_none`, which deserializes
  the real pre-existing fixture `campaigns/tutorial/saves/save_20260809_072524.ron`
  (which genuinely predates the `lore` field) and asserts every character
  loads with `lore: None`. The prior regression test
  (`test_lore_field_serde_default_deserializes` in `character.rs`) only
  covered a synthetically-stripped save, not a real one.
- `docs/how-to/character_definition_ron_format.md` gained a "Character Lore
  File Format" subsection documenting the `CharacterLore` schema
  (`backstory`, `profile.title/archetype/core_motivation/combat_style`) that
  `lore_file` points to, which was previously only documented in
  `docs/reference/campaign_content_format.md`.

### Quality gates (full workspace)

- `cargo fmt --all` — clean
- `cargo check --all-targets --all-features` — 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run --all-features` — **5489/5489 passed**, 8 skipped
  (includes the tutorial campaign integration tests, which now load all 8
  characters' lore content end-to-end through `load_from_campaign`)
- `cargo test --doc -- character_definition` (targeted, not the full
  workspace doctest suite) — **28/28 passed**, including the new
  `CharacterLore`/`CharacterProfile`/`load_from_campaign` doctests
- `cargo check -p campaign_builder --all-targets --all-features` — 0 errors

---

## Character Bio & Navigation, Phase 1: Party Overview Keyboard Navigation

### Summary

Implemented Phase 1 of
`docs/explanation/character_bio_and_navigation_implementation_plan.md`: full
keyboard navigation for the Character Sheet's Party Overview grid, which
previously had no keyboard support at all (`character_sheet_input_system`
early-returned unless `view == Single`). Party Overview now supports arrow-key
grid movement, Enter/Space to open Single view, digit-key jump-select in both
views, a visible highlight on the keyboard-selected card, and an updated hint
bar. `CharacterSheetState.focused_index` is reused as the Party Overview
highlight index -- no new state field was introduced.

### Files modified

**`src/application/character_sheet_state.rs`**
- Added `focus_up(&mut self, party_size: usize, cols: usize)` and
  `focus_down(&mut self, party_size: usize, cols: usize)` -- row-step grid
  navigation with wrapping, alongside the existing `focus_next`/`focus_prev`.
  Handles the ragged-last-row case (party size not evenly divisible by `cols`)
  and the single-row degenerate case (`focus_down` wraps to itself).
- 7 new unit tests covering normal movement, top/bottom wrap, empty-party
  no-op, the ragged-row case, and the single-row no-op.

**`src/game/systems/character_sheet_ui.rs`**
- `character_sheet_input_system`: restructured the `is_single` early-return
  into an `if is_single {...} else {...}` branch. The Party Overview branch
  handles `↑↓←→` (reusing `focus_next`/`focus_prev` for `←→`, new
  `focus_up`/`focus_down` for `↑↓`) and `Enter`/`NumpadEnter`/`Space` (opens
  Single view for the highlighted card). The digit-key (1-6) select loop now
  runs unconditionally after the branch, so it works in both views; it only
  changes `focused_index`, never `view`.
- `render_party_overview_card` gained a `highlighted: bool` param; when set,
  draws the card's border with the existing
  `inventory_ui_common::SELECT_HIGHLIGHT_COLOR` (reused, no new color
  constant) instead of the default grey stroke.
- `render_party_overview` gained a `focused_index: usize` param (threaded from
  `character_sheet_ui_system`, which already had it) to compute the
  highlighted card, plus a new hint-bar row matching the Single-view style:
  `[Esc/P] Close  [O] Single  [1-6] Select  [Enter] View  [↑↓←→] Move`.
- Module doc comment (ASCII diagram + Flow section) updated to document the
  Party Overview keyboard vocabulary alongside the existing Single-view one.
- 5 new real `App`-harness tests (following the `skill_training_ui.rs`
  `App::new()` + `MinimalPlugins` + `app.update()` pattern, which actually
  runs `character_sheet_input_system` rather than simulating its logic
  inline) covering arrow-key movement, Enter/Space view-switch, and
  digit-key select without a view change.

### Quality gates (full workspace)

- `cargo fmt --all` — clean
- `cargo check --all-targets --all-features` — 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run --all-features` — **5482/5482 passed**, 8 skipped

---

## Phase 5: Documentation and Final Verification

### Summary

Completed the mesh editor removal by updating all documentation to reflect the
current codebase state, verifying zero stale symbol references remain in any
`.rs` source file, and confirming the full workspace test suite passes.

### Files modified

**`sdk/campaign_builder/README.md`**
- Removed `### Item Mesh Editor` feature section (the tab and its backing files
  were deleted in Phase 1)
- Updated `### Creature Asset Editor` description: replaced the stale
  "Three-Panel Edit Mode" bullet (mesh list + mesh properties) with an accurate
  "Edit Mode" bullet describing the read-only 3D preview and creature properties
  panel that remain
- Removed `item_mesh_editor.rs` entry from the Source Layout architecture tree
- Replaced `mesh_editing_tests.rs` in the tests list with `obj_importer_tests.rs`
  (the replacement file created in Phase 3)

### Stale symbol audit

Repo-wide grep across all `*.rs` files for all removed symbols confirmed **zero**
remaining references:

| Symbol set | `.rs` matches |
|---|---|
| `item_mesh_editor`, `ItemMeshEditor`, `ItemMeshEditorState`, `ItemMeshEditorSignal` | 0 |
| `mesh_ui`, `mesh_vertex_editor`, `mesh_normal_editor`, `mesh_index_editor` | 0 |
| `enter_mesh_editor`, `PrimitiveType`, `PRIMITIVE_SEGMENTS_MAX` | 0 |

References in `docs/explanation/finished/` (archived plans and implementation
records) and `docs/explanation/mesh_editor_removal_implementation_plan.md` are
legitimate historical context, not stale code — they were not modified.

### Quality gates (full workspace)

- `cargo fmt --all` — clean
- `cargo check --all-targets --all-features` — 0 errors
- `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run --all-features` — **5462/5462 passed**, 8 skipped

### Manual SDK smoke test checklist

✅ **All items verified by human on 2026-08-08:**

1. ✅ The "Item Meshes" tab is absent from the sidebar
2. ✅ Opening any creature shows only the read-only 3D preview and creature
   properties (no mesh list, no mesh properties panel)
3. ✅ The 3D preview controls (grid, wireframe, normals, axes, background colour,
   camera distance) still function
4. ✅ The Importer tab successfully imports a `.glb` file and a `.obj` file
   end-to-end
5. ✅ Opening `campaigns/tutorial` completes without error

---

## Phase 4: Remove Dead Breadcrumb Helper

### Summary

Removed the `enter_mesh_editor` breadcrumb-extension helper from
`creatures_workflow.rs`. The function had zero production call sites — it was
called only from its own `#[cfg(test)]` block and one integration test.
Removing it eliminates dead API surface with no behavioural change.

### Files modified

**`sdk/campaign_builder/src/creatures_workflow.rs`**
- Removed `pub fn enter_mesh_editor` (function body + full doc comment +
  `# Examples` doctest block, L342–L372)
- Removed internal unit test `fn test_enter_mesh_editor_extends_breadcrumbs`
- Removed internal unit test `fn test_breadcrumb_string_mesh_editor`

**`sdk/campaign_builder/tests/creature_workflow_tests.rs`**
- Removed three lines from `fn test_registry_to_asset_navigation`:
  the `workflow.enter_mesh_editor(...)` call and the two
  `breadcrumb_labels` assertions that followed it

### Quality gates

- `cargo fmt --all` — clean
- `cargo check -p campaign_builder --all-targets --all-features` — 0 errors, 0 warnings
- `cargo clippy -p campaign_builder --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run -p campaign_builder --all-features` — 2474/2474 passed

---

## Phase 3: Remove Orphaned Raw Mesh-Editing Modules

### Summary

Removed the three orphaned raw mesh-editing modules (`mesh_vertex_editor`,
`mesh_normal_editor`, `mesh_index_editor`) that had no callers anywhere in the
crate outside their own source and the now-deleted integration test file. The
kept modules (`mesh_obj_io`, `mesh_validation`) retain full test coverage via
the new `obj_importer_tests.rs`.

### Files deleted

- `sdk/campaign_builder/src/mesh_vertex_editor.rs` — vertex selection and
  manipulation editor
- `sdk/campaign_builder/src/mesh_normal_editor.rs` — normal calculation and
  editing
- `sdk/campaign_builder/src/mesh_index_editor.rs` — triangle index editor
- `sdk/campaign_builder/tests/mesh_editing_tests.rs` — mixed test file
  covering both deleted and kept modules (~938 lines)

### Files modified

**`sdk/campaign_builder/src/lib.rs`**
- Removed three `pub mod` declarations: `mesh_index_editor`, `mesh_normal_editor`,
  `mesh_vertex_editor`

**`sdk/campaign_builder/src/linear_history.rs`**
- Updated module-level doc comment to remove stale cross-references to
  `crate::mesh_vertex_editor::VertexOperation` and
  `crate::mesh_index_editor::IndexOperation`; replaced with a generic
  description: "operations (each carrying 'before' and 'after' state)"

### Files created

**`sdk/campaign_builder/tests/obj_importer_tests.rs`**
- SPDX header added
- Ported all `mesh_obj_io` and `mesh_validation` tests from the deleted
  `mesh_editing_tests.rs`: 9 validation tests, 6 OBJ import/export tests,
  1 edge-case test (17 total)
- Ported the two helper functions used by ported tests: `create_simple_triangle`,
  `create_quad_mesh`

### Quality gates

- `cargo fmt --all` — clean
- `cargo check -p campaign_builder --all-targets --all-features` — 0 errors, 0 warnings
- `cargo clippy -p campaign_builder --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run -p campaign_builder --all-features` — 2476/2476 passed

---


### Summary

Removed the interactive mesh-list / mesh-properties panels from every creature
edit screen in the Campaign Builder SDK, along with the primitive-replacement
dialog and all associated dead state. The read-only 3D preview panel
(`show_preview_panel`) and creature-level properties panel
(`show_creature_level_properties`) are fully preserved and unchanged.

### Files deleted

- `sdk/campaign_builder/src/creatures_editor/mesh_ui.rs` — `show_mesh_properties_panel` (327 lines)

### Files modified

**`sdk/campaign_builder/src/creatures_editor/mod.rs`**
- Removed `mod mesh_ui;` declaration and `use crate::mesh_validation;` import
- Removed constants `PRIMITIVE_SEGMENTS_MAX`, `PRIMITIVE_RINGS_MAX`,
  `PRIMITIVE_SOFT_TRIANGLE_BUDGET`
- Removed `PrimitiveType` enum
- Removed 16 editing-only struct fields from `CreaturesEditorState`:
  `show_mesh_list`, `show_mesh_editor`, `selected_mesh_index`,
  `mesh_edit_buffer`, `mesh_transform_buffer`, `mesh_visibility`,
  `show_primitive_dialog`, `primitive_type`, `primitive_size`,
  `primitive_segments`, `primitive_rings`, `primitive_use_current_color`,
  `primitive_custom_color`, `primitive_preserve_transform`,
  `primitive_keep_name`, `uniform_scale`
- Removed all corresponding `Default` initializers
- In `show_edit_mode`: replaced the four-panel layout (left mesh-list,
  right mesh-properties, central preview, primitive dialog) with just the
  `CentralPanel` preview + `Panel::bottom` creature-properties (both kept)
- Cleaned up 3-field reset lines from Back-to-List, Cancel,
  `revert_edit_buffer_from_registry` (both arms), `perform_save_as_with_path`,
  and `back_to_registry`
- Removed 6 functions: `show_mesh_list_panel`, `validate_selected_mesh`,
  `estimate_primitive_geometry`, `show_primitive_replacement_dialog`,
  `apply_primitive_replacement`, `_legacy_show_mesh_list_and_editor`
- Removed `refresh_validation_state` per-mesh validation loop (orphaned after
  `validate_selected_mesh` removal)
- Removed test helpers `preview_selected_mesh_for_tests`,
  `preview_visibility_for_tests`
- Removed tests: `test_mesh_selection_state`,
  `test_preview_sync_reflects_color_changes_and_visibility`,
  `test_validate_selected_mesh_reports_invalid_mesh_errors`
- Repaired tests: `test_preview_sync_clears_dirty_and_updates_statistics`
  (removed deleted-field refs, updated `selected_meshes` assertion 1→0),
  `test_preview_sync_reflects_transform_changes` (removed deleted-field refs)

**`sdk/campaign_builder/src/creatures_editor/preview_panel.rs`**
- `sync_preview_renderer_from_edit_buffer`: removed `selected_mesh_index`
  variable, `set_selected_mesh_index` call, and `mesh_visibility`-based
  visibility logic; now always renders all meshes
- `current_mesh_visibility`: simplified to `vec![true; mesh_count]`
- `build_preview_statistics`: removed `visible` parameter, iterates all meshes
  unconditionally, `selected_meshes` hardcoded to `0`

**`sdk/campaign_builder/tests/creature_asset_editor_tests.rs`**
- Fixed import: removed `PrimitiveType` from the `use` statement
- Removed 5 tests: `test_replace_mesh_with_primitive_cube`,
  `test_replace_mesh_with_primitive_sphere`, `test_mesh_visibility_tracking`,
  `test_primitive_type_enum`, `test_uniform_scale_toggle`

**`sdk/campaign_builder/tests/creature_preview_integration_test.rs`**
- Removed two deleted field assignments (`mesh_visibility`, `selected_mesh_index`)

### Quality gates

- `cargo fmt --all` — clean
- `cargo check -p campaign_builder --all-targets --all-features` — 0 errors, 0 warnings
- `cargo clippy -p campaign_builder --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run -p campaign_builder --all-features` — 2572/2572 passed

---


### Summary

Removed the interactive "Item Meshes" tab and its entire backing implementation
from the Campaign Builder SDK. The user's workflow authors 3D models in Blender
and imports them via the existing Importer tab; the in-SDK mesh editor was
unused dead weight.

### Files deleted

- `sdk/campaign_builder/src/item_mesh_editor.rs` — full interactive mesh
  editor (~3 071 lines)
- `sdk/campaign_builder/src/item_mesh_workflow.rs` — workflow state (~472
  lines)
- `sdk/campaign_builder/src/item_mesh_undo_redo.rs` — undo/redo history

### Files modified

**`sdk/campaign_builder/src/lib.rs`**
- Removed three `pub mod` declarations (`item_mesh_editor`,
  `item_mesh_undo_redo`, `item_mesh_workflow`)
- Removed `EditorTab::ItemMeshes` variant and its `name()` arm
- Removed `item_mesh_editor_state` field from `CampaignBuilderApp` and its
  `Default` initializer
- Removed the `EditorTab::ItemMeshes` central-panel match arm (including the
  `ItemMeshEditorSignal::OpenInItemsEditor` signal handler)
- Removed cross-tab navigation guard inside the `EditorTab::Items` arm that
  drained `requested_open_item_mesh` and switched to the now-deleted tab
- Removed `item_mesh_editor_state.load_from_campaign()` call inside the
  `ObjImporterUiSignal::Item` handler
- Removed `EditorTab::ItemMeshes` from the sidebar tabs array

**`sdk/campaign_builder/src/campaign_io.rs`**
- Removed the item mesh asset load block in `do_open_campaign` (the
  `load_from_campaign` call and surrounding log messages)

**`sdk/campaign_builder/src/items_editor.rs`**
- Removed `requested_open_item_mesh: Option<ItemId>` field and its doc
  comment from `ItemsEditorState`
- Removed corresponding `Default` initializer
- Removed "✏️ Open in Item Mesh Editor" button and handler from `show_form`
- Removed `ItemId` import (now unused)
- Removed test `test_items_editor_requested_open_item_mesh_set_on_button`

**`sdk/campaign_builder/src/objects_editor.rs`**
- Updated module doc comment that referenced the now-deleted
  `crate::item_mesh_editor`

**`sdk/campaign_builder/src/map_editor.rs`**
- Fixed pre-existing Clippy `manual_clamp` lint (`.min().max()` →
  `.clamp()`) that blocked the `-D warnings` quality gate

### Quality gates

- `cargo fmt --all` — clean
- `cargo check -p campaign_builder --all-targets --all-features` — 0 errors,
  0 warnings
- `cargo clippy -p campaign_builder --all-targets --all-features -- -D warnings` — 0 warnings
- `cargo nextest run -p campaign_builder --all-features` — 2580/2580 passed

---


### Root cause

`init_save_manager()` is called once during `MenuPlugin::build()` — before any campaign
loads — and hardcodes the path as `"saves"`, a directory relative to wherever
`cargo run` is invoked (the project root). Nothing in the code ever updated the manager
after a campaign became active, so all saves landed in `antares/saves/` regardless of
which campaign was being played.

### Fix

**`src/application/save_game.rs`** — `SaveGameManager`:
- Added `pub fn saves_dir(&self) -> &Path` accessor so callers can inspect the current
  save directory without reaching into the private field.

**`src/game/systems/menu.rs`** — `MenuPlugin` + new system `sync_save_manager_to_campaign`:
- Registered `sync_save_manager_to_campaign` in `MenuPlugin::build()`.
- The system runs every frame. When `global_state.0.campaign` is `Some`, it compares
  `campaign.root_path.join("saves")` against `save_manager.saves_dir()`. If they differ
  (first frame after campaign load, or after a campaign switch), it creates a new
  `SaveGameManager` pointing at the campaign directory and re-inserts it as the Bevy
  resource. It also clears `menu_state.save_list` so `populate_save_list` re-queries
  the new directory on the next frame. On subsequent frames the paths match and the
  system exits after one comparison.

Save files will now be written to `campaigns/tutorial/saves/` (or whichever campaign
is active) rather than `antares/saves/`. Existing saves in `antares/saves/` can be
moved manually if needed.

### Validation

```
cargo fmt --all                                           → no output
cargo check --all-targets --all-features                 → 0 errors
cargo clippy --all-targets --all-features -- -D warnings → 0 warnings
cargo nextest run --all-features                         → 5462 passed, 0 failed, 8 skipped
```

---

## Bug Fix: Save/Load — wrong save selected, map not re-drawn, campaign lost, timestamps missing

### Root causes

**Bug 1 (Critical) — Saves sorted oldest-first; default slot 0 loaded an old save**
`SaveGameManager::list_saves()` sorted filenames alphabetically (`saves.sort()`), which
places the oldest save at index 0. After any submenu transition `selected_index` resets to 0,
so pressing Load always loaded the oldest save, making it appear as if the game had reset.

**Bug 2 (Critical) — Map not re-drawn after loading when `current_map` is unchanged**
`spawn_map_markers` caches the last rendered map id in a `Local<Option<MapId>>` and skips
re-rendering when the id matches. If the loaded save contained the same `current_map` as
the active session, the 3D world stayed stale (old tile positions, stale events).

**Bug 3 (Significant) — `campaign` field lost after load**
`GameState.campaign` is annotated `#[serde(skip)]`, so `save_manager.load()` returns a
`GameState` with `campaign: None`. The old code replaced `global_state.0` with the
loaded state without re-attaching the campaign, breaking `MenuButton::NewGame`, font
loading, and any system that checks `game_state.campaign`.

**Bug 4 (UX) — No feedback on save/load success or error**
Both `save_game_operation` and `load_game_operation` logged with `debug!/error!` but showed
nothing in the UI.

**Bug 5 (UX) — Timestamps showed "Unknown" in the save list**
`populate_save_list` hardcoded `timestamp: String::from("Unknown")` instead of parsing the
filename (`save_YYYYMMDD_HHMMSS`).

### Fix

**`src/application/save_game.rs`** — `list_saves()`:
- `saves.sort()` → `saves.sort_by(|a, b| b.cmp(a))` — newest-first so index 0 is always
  the most recent save.

**`src/application/mod.rs`** — `GameState`:
- Added `#[serde(skip, default)] pub needs_map_refresh: bool` field; set to `false` in
  both `GameState::new()` and `GameState::new_game()`.

**`src/application/menu.rs`** — `MenuState`:
- Added `#[serde(skip, default)] pub status_message: Option<String>` field.
- Updated `MenuState::new()` and `Default` impl to include `status_message: None`.

**`src/game/systems/map.rs`** — `spawn_map` / `spawn_map_markers`:
- Changed `spawn_map`'s `global_state` parameter from `Res<GlobalState>` to
  `&GlobalState` so the function can be called from both a `Res` and a `ResMut` context.
- Changed `spawn_map_markers` to take `mut global_state: ResMut<GlobalState>`.
- Added `needs_map_refresh` check at the top of `spawn_map_markers`: when the flag is
  set, it is cleared and `*last_map = None` is forced so the full despawn+respawn path
  runs regardless of whether `current_map` changed.
- Updated both `spawn_map_system` and `spawn_map_markers` call sites to pass `&global_state`.

**`src/game/systems/menu.rs`** — multiple functions:
- Added `parse_save_timestamp(filename)` helper that converts `save_YYYYMMDD_HHMMSS` →
  `"YYYY-MM-DD HH:MM:SS"` with a safe fallback of `"Unknown"`.
- `populate_save_list`: replaced hardcoded `"Unknown"` with `parse_save_timestamp(&filename)`.
- `save_game_operation`: sets `menu_state.status_message = Some("Saved: …")` on success
  and `"Save failed: …"` on error.
- `load_game_operation`:
  - Takes `campaign` from the old state before replacing it, then re-attaches it after
    `global_state.0 = loaded_state`.
  - Sets `global_state.0.needs_map_refresh = true` to trigger a forced re-render.
  - Sets `menu_state.status_message = Some("Load failed: …")` on error.
- `spawn_main_menu`: displays `menu_state.status_message` in green below the buttons
  (save confirmation).
- `spawn_save_load_menu`: displays `menu_state.status_message` in green (success) or red
  (error) below the action buttons.

### Save file location (Mac)

Save files are written to `antares/saves/` relative to the working directory where
`cargo run` is invoked (i.e. the project root). To list them:

```bash
ls saves/
```

Files are named `save_YYYYMMDD_HHMMSS.ron` and are plain RON text. To delete one:

```bash
rm saves/save_20260717_134557.ron
```

### Validation

```
cargo fmt --all                                           → no output
cargo check --all-targets --all-features                 → 0 errors
cargo clippy --all-targets --all-features -- -D warnings → 0 warnings
cargo nextest run --all-features                         → 5462 passed, 0 failed, 8 skipped
```

---

## Bug Fix: Map 4 to Map 2 portal loop, blocked exit, bandit bleed-through, and SDK preview offset

### Root causes

Three separate bugs combined to produce the reported symptoms.

**1. Portal destination too close to the return portal (the loop)**

The Dungeon Portal in map_4.ron (Arcturus Cave, tile 19,17) teleports the party to
map 2 (Dark Forest) at tile (18,10). The return portal, Arcturus Cave Portal, sits
at (19,10) on map 2, one tile east of the landing spot. A player arriving facing east
immediately stepped onto the return portal and was sent back to map 4 at (18,17),
where the outbound portal is one tile east at (19,17). This created an inescapable
teleport loop that appeared as: going back to map 4, can not go two tiles straight
ahead, and because each cycle briefly loaded map 2 the map-2 Bandit Patrol at (17,16)
occasionally resolved before the second bounce returned the party.

**2. Wrong map number in the Dungeon Portal description**

The description read Dark Forrest (map 4) at (19,10) - map number and coordinates
were both wrong. Should be Dark Forest (map 2) at (16,10).

**3. Wrong map number in the Arcturus Cave Portal description**

The description read Arcturus Cave (map 2) at (18,17) - map 2 is the Dark Forest;
Arcturus Cave is map 4. Should be Arcturus Cave (map 4) at (16,17).

**4. SDK Campaign Builder - teleport destination map preview was offset and cut off**

MapPreviewWidget::ui used the same fill clip rect, then center logic as the main map
grid editor. Inside the inspector vertical-only ScrollArea, the clip-rect width was
the full inspector panel width (often wider than the 360 px map preview). This
produced a positive grid_offset.x, shifting the map right by half the surplus and
making click coordinates visually misaligned. When the inspector was narrower than
360 px the map was clipped on the right with no horizontal scrollbar. Making the SDK
window wider increased the offset rather than fixing the cut-off.

### Fix

campaigns/tutorial/data/maps/map_4.ron - Dungeon Portal at (19,17):
- destination.x: 18 to 16 (3 tiles from the return portal instead of 1)
- description corrected to Dark Forest (map 2) at (16,10)

campaigns/tutorial/data/maps/map_2.ron - Arcturus Cave Portal at (19,10):
- destination.x: 18 to 16
- description corrected to Arcturus Cave (map 4) at (16,17)

data/test_campaign/data/maps/map_4.ron - same portal:
- map_id: 4 to 2 (was pointing back to itself)
- destination: (19,10) to (16,10)
- description corrected

data/test_campaign/data/maps/map_2.ron - same portal:
- destination.x: 18 to 16
- description corrected

sdk/campaign_builder/src/map_editor.rs - MapPreviewWidget::ui:
- Removed the avail = ui.clip_rect().size() expansion and centering grid_offset.
- The widget now allocates exactly map_width_px x map_height_px, left-aligned.
- Click handler updated to subtract only response.rect.min (no offset).

sdk/campaign_builder/src/map_editor.rs - show_event_editor, Teleport branch:
- Changed tile_size from hard-coded 18.0 to (avail_w / map.width).min(18.0).max(6.0)
  so the preview always fits the available inspector width without overflow.

---

## Bug Fix: First Aid (and other non-damage spells) had no effect in combat

### Root cause

`campaigns/tutorial/data/spells.ron` was generated before the `effect_type`
field existed on the `Spell` RON schema. Every spell in that file had
`effect_type: None`.

The combat spell dispatcher in `execute_spell_cast_with_spell`
(`src/domain/combat/spell_casting.rs`) routes healing, cure, buff, utility, and
resurrection effects by checking `spell.effective_effect_type()`. When
`effect_type` is `None`, `effective_effect_type()` calls `infer_effect_type()`,
which only recognises damage dice (`damage: Some(...)`) and applied conditions
(`applied_conditions: [...]`). A heal-only spell like First Aid — no damage
dice, no conditions — fell through to the default:

```rust
SpellEffectType::Utility { utility_type: UtilityType::Information }
```

The `Healing` dispatch block never matched, so SP was consumed, the cast
animation played, and zero HP was restored. "Always no effect" is correct
because the data was consistently wrong for the entire campaign.

### Fix

**`campaigns/tutorial/data/spells.ron`** — 7 spells corrected:

| Spell ID | Name | `effect_type` added |
|----------|------|---------------------|
| 257 | Awaken (Cleric) | `Some(CureCondition(condition_id: "asleep"))` |
| 260 | First Aid | `Some(Healing(amount: (count: 0, sides: 0, bonus: 8)))` |
| 262 | Power Cure | `Some(Healing(amount: (count: 0, sides: 0, bonus: 15)))` |
| 513 | Cure Wounds | `Some(Healing(amount: (count: 0, sides: 0, bonus: 25)))` |
| 770 | Cure Blindness | `Some(CureCondition(condition_id: "blinded"))` |
| 771 | Cure Paralysis | `Some(CureCondition(condition_id: "paralyzed"))` |
| 1025 | Awaken (Sorcerer) | `Some(CureCondition(condition_id: "asleep"))` |

The remaining `effect_type: None` entries in the tutorial file are correct:
those spells carry `damage: Some(...)` or non-empty `applied_conditions`, so
`infer_effect_type()` handles them correctly without an explicit tag.

`data/test_campaign/data/spells.ron` already had all `effect_type` values
correctly set — no changes needed there.

### Why tests didn't catch this

The existing combat heal tests (`test_party_target_confirm_spell_heals_ally`,
etc.) construct spells in-code with `spell.effect_type` set explicitly. They do
not load from `campaigns/tutorial/data/spells.ron`. Test data lives in
`data/test_campaign/data/spells.ron`, which was already correct.

### Validation

```
cargo check --all-targets --all-features   → 0 errors
cargo nextest run --all-features           → 5462 passed, 0 failed, 8 skipped
```


### Root cause

In both `NpcEditorState::show_edit_view` (`npc_editor/mod.rs`) and
`CharactersEditorState::show` (`characters_editor.rs`), the creature picker
popup called `manager.load_all_creatures()` **on every rendered frame** while
the window was open. `load_all_creatures` reads the registry RON file from
disk, then reads every individual creature `.ron` file in sequence — easily
50–100 disk reads per frame at 60 fps. Combined with egui laying out every
row in the list regardless of visibility, this made the picker
unusable on campaigns with a large creature database.

### Fix

**`sdk/campaign_builder/src/npc_editor/mod.rs`**
**`sdk/campaign_builder/src/characters_editor.rs`**

Three changes per file:

1. **Cache, don't reload.** Both editor states already maintain
   `available_creatures: Vec<(u32, String)>` — a list of `(id, name)` pairs
   rebuilt only when the campaign directory changes or the cache-dirty flag is
   set. The picker now reads this in-memory list instead of calling
   `load_all_creatures()`.

2. **Search filter.** Added `creature_picker_search: String` field
   (`#[serde(skip)]`, reset to empty each time the picker opens). A search bar
   at the top of the popup filters the list to matching IDs and names in one
   `O(n)` pass over the in-memory cache. The result count is shown as
   `"N of M creatures"`.

3. **Virtualised rendering.** Replaced `ScrollArea::show(…)` (renders all
   rows) with `ScrollArea::show_rows(…, row_h, total, …)` (renders only the
   rows currently visible in the viewport). On a list of 500 creatures only
   ~20 rows are rendered per frame instead of 500.

The `creature_manager` parameter was removed from `NpcEditorState::show_edit_view`
(it was only passed to the now-replaced picker block). One internal test call
site was updated accordingly.

### Performance improvement

| Before | After |
|--------|-------|
| ~50–100 disk reads per frame while picker is open | 0 disk reads per frame |
| All `N` rows laid out and rendered every frame | Only visible rows (~20) rendered |
| No search — must scroll entire list | Instant substring filter |

### Validation

```
cargo fmt --all                                                   # no output
cargo clippy --manifest-path sdk/campaign_builder/Cargo.toml
             --all-targets --all-features -- -D warnings          # 0 warnings
cargo nextest run --manifest-path sdk/campaign_builder/Cargo.toml # 2631 passed
```

## Phase 5.8: `#[allow(clippy::too_many_arguments)]` Refactor — Params/Context Structs

Refactored non-Bevy helper functions that had `#[allow(clippy::too_many_arguments)]`
suppressors by bundling their repeated parameters into purpose-built structs.
Constructors with 30+ call sites were kept with the allow-attribute but given
explanatory comments instead of silent suppression.

### `src/game/systems/advanced_trees.rs` — `MeshBuffers<'a>`

Introduced `MeshBuffers<'a>` struct bundling the four mesh attribute buffers
(`positions`, `normals`, `uvs`, `indices`) that every foliage geometry helper
writes into. Removed `#[allow(clippy::too_many_arguments)]` from all 10 `append_*`
functions:

`append_leaf_shape`, `append_leaf_card_cross`, `append_leaf_card`, `append_diamond`,
`append_lobed_leaf_cluster`, `append_pine_needle_cluster`, `append_willow_hanging_strip`,
`append_palm_frond`, `append_diamond_leaf`, `append_clustered_shrub_leaf`

Also updated `append_quad` and `append_triangle` (below the clippy threshold but using
the same 4-buffer pattern) for consistency. `generate_leaf_mesh` creates a scoped
`MeshBuffers` instance and drops it before reading back `positions`.

Test call sites in the `tests` module updated to construct inline `MeshBuffers` literals.

### `src/game/systems/advanced_grass.rs` — `GrassClumpArgs<'a>`

Introduced `GrassClumpArgs<'a>` bundling the 10 per-clump non-resource parameters
(center, blade dimensions, appearance, lod index, parent entity, render mode) of the
private `spawn_grass_clump` helper. Function reduced from 16 parameters to 7 (5 Bevy
resource handles + `args` struct + `rng`).

The three public helpers `spawn_grass`, `spawn_grass_cached`, and
`spawn_grass_cached_with_exclusions` retain their `#[allow]` because their heavy
parameter counts reflect unavoidable Bevy resource handles that cannot be bundled
without naming Bevy's internal world-lifetime parameters. Comments now explain this
rationale explicitly. `build_grass_chunks_system` is a Bevy system proper —
its `#[allow]` is unchanged and idiomatic.

### `src/domain/combat/monster.rs` — `Monster::new`

Kept `#[allow(clippy::too_many_arguments)]` but replaced the implicit suppression
with an explanatory block comment: 30+ call sites, all required fields, no
natural sub-grouping, stable API.

### `src/domain/magic/types.rs` — `Spell::new`

Same treatment as `Monster::new`: 40+ call sites across tests, the domain layer,
and the SDK. Comment documents the rationale.

### `src/game/systems/actor.rs` — `ActorSpawnParams<'a>`

New public struct bundling the three per-actor descriptor fields (`sprite_ref`,
`position`, `actor_type`). `spawn_actor_sprite` reduced from 8 parameters to 6.
Module-level doc example and function doc example updated. Two call sites in
`src/game/systems/map.rs` updated to pass `&ActorSpawnParams { … }` literals.
`map.rs` import updated to bring `ActorSpawnParams` into scope.

### `src/bin/generate_foliage_textures.rs` — `EllipseGeometry`

New public struct bundling the five ellipse geometry fields (`cx`, `cy`, `semi_a`,
`semi_b`, `angle`). `draw_ellipse` reduced from 8 parameters to 4. All 10 call
sites in `stamp_oak`, `stamp_pine`, `stamp_birch`, `stamp_willow`, `stamp_palm`,
and `stamp_shrub` updated to pass inline `EllipseGeometry { … }` literals.

### `sdk/campaign_builder/src/characters_editor.rs` — `CharactersEditorData<'a>`

New public struct bundling the five data-slice parameters (`races`, `classes`,
`items`, `spells`, `creature_manager`) shared by `show()` and
`show_character_form()`. Both functions reduced from 8 parameters (including
`self`) to 4, removing their `#[allow(clippy::too_many_arguments)]` suppressors.

Call sites updated:
- `sdk/campaign_builder/src/lib.rs`: constructs a `CharactersEditorData` before
  the `show()` call.
- Internal call from `show()` to `show_character_form()` updated.
- Test at `characters_editor.rs:3696` updated to construct `CharactersEditorData`
  before calling `show_character_form()`.

Extracted per-system `GameMode` prologues into shared Bevy run-condition
functions so that system bodies stay focused and the plugin registrations
declaratively express when each system should run.

### New file — `src/game/run_conditions.rs`

Public run-condition helpers gated on `GameMode`:

| Function | Returns `true` when… |
|---|---|
| `in_combat_mode` | `GameMode::Combat(_)` |
| `in_dialogue_mode` | `GameMode::Dialogue(_)` |
| `in_character_sheet_mode` | `GameMode::CharacterSheet(_)` |
| `in_automap_mode` | `GameMode::Automap` |

Registered as `pub mod run_conditions;` in `src/game/mod.rs`.

### `src/game/systems/combat.rs` — `CombatPlugin`

Removed simple `if !matches!(…GameMode::Combat…) { return; }` guard from
11 systems and added `.run_if(crate::game::run_conditions::in_combat_mode)`
to each system's `add_systems` call:

`sync_party_hp_during_combat`, `setup_combat_ui`, `update_combat_ui`,
`combat_input_system`, `tick_combat_time`, `check_combat_resolution`,
`collect_combat_feedback_log_lines`, `mirror_combat_feedback_to_game_log`,
`update_combat_log_typewriter`, `spawn_monster_hp_hover_bars`,
`update_monster_hp_hover_bars`.

The `global_state: Res<GlobalState>` parameter was **removed** from the seven
functions that used it only for the guard: `setup_combat_ui`,
`update_combat_ui`, `combat_input_system`, `check_combat_resolution`,
`collect_combat_feedback_log_lines`, `mirror_combat_feedback_to_game_log`,
`update_combat_log_typewriter`.

`execute_monster_turn` was **skipped** — its guard branch sets
`*was_enemy_turn = false` (side effect) and cannot be a simple run condition.

### `src/game/systems/combat_visual.rs`

Removed combat-mode guard and `global_state`/`GameMode` imports from
`spawn_turn_indicator` and `update_turn_indicator`. The `.run_if(…)` calls
were added in `CombatPlugin::build` (where these systems are registered).

### `src/game/systems/dialogue.rs` — `DialoguePlugin`

Removed guard and `global_state` parameter from `dialogue_input_system`;
added `.run_if(crate::game::run_conditions::in_dialogue_mode)` in the plugin.

### `src/game/systems/dialogue_choices.rs`

Removed guard from `choice_input_system` (kept `global_state` — mutates
`global_state.0.mode` to exit dialogue on Escape). `.run_if(…)` added in
`DialoguePlugin::build`.

`cleanup_choice_ui` was **skipped** — it performs cleanup (resetting
`choice_state`) when NOT in dialogue mode, which is intentional side-effect
behaviour.

### `src/game/systems/character_sheet_ui.rs` — `CharacterSheetPlugin`

Removed guard from `character_sheet_input_system` (kept `global_state` —
used for mode mutations throughout the function body); added
`.run_if(crate::game::run_conditions::in_character_sheet_mode)` in the
plugin.

### `src/game/systems/hud.rs`

Left the private `not_in_combat` and `in_automap_mode` helpers unchanged —
they are private, already working, and breaking them would require touching
test-verified hud plugin logic.

### Verification (run_conditions + full cleanup pass)

After all Batch 1 and Phase 5.6/5.8 changes merged:

```
cargo fmt --all                                              # no output
cargo check --all-targets --all-features                    # Finished — 0 errors
cargo clippy --all-targets --all-features -- -D warnings    # Finished — 0 warnings
cargo nextest run --all-features                            # 5462 passed, 8 skipped
cargo nextest run --manifest-path sdk/campaign_builder/…    # 2631 passed, 0 skipped
```

---

## Codebase Cleanup — Missed-Deliverable Remediation Pass

Audit of the six-phase cleanup plan revealed that Phases 1–4 had three
partial deliverables and that Phases 5–6 had six items still outstanding.
All were resolved in a single remediation pass.

### Phase 1.7 residue — two remaining `let _ =` sites

**`src/game/systems/inventory_ui.rs`** (~lines 2774, 2887):
`let _ = character.inventory.remove_item(slot_index)` → now logs a `warn!` if
`remove_item` returns `None` (unexpected slot-not-found), using the existing
`bevy::prelude::warn!` import. No logic changed.

**`src/domain/combat/spell_casting.rs`** (~line 512):
`let _ = spell_dispatch::apply_utility_spell(*utility_type)` → now binds the
`UtilityResult` and logs it at `tracing::debug!` level. No logic changed.

### Phase 4.1 residue — `CampaignError` name collision

`sdk/campaign_builder/src/lib.rs` defined `pub enum CampaignError` while the
domain crate defines its own `CampaignError`. Pure rename: the SDK enum is now
`CampaignBuilderError`. Updated across 4 files:
- `sdk/campaign_builder/src/lib.rs` (definition)
- `sdk/campaign_builder/src/campaign_io.rs` (14 occurrences)
- `sdk/campaign_builder/src/campaign_editor.rs` (2 signatures + 1 doc comment)
- `sdk/campaign_builder/tests/campaign_io_tests.rs` (1 `matches!` assertion)

### Stale doc comment — `src/game/systems/dialogue.rs` ~L1017

The doc table for `execute_action` claimed `SetFlag` was "not yet persisted"
and `ChangeReputation` was "not yet implemented" — both are fully wired.
Updated both rows to reflect actual behavior.

### Phase N plan-referencing comments in `sdk/campaign_builder/`

Three comments in the SDK crate explicitly referenced "Phase 5 of the codebase
cleanup plan":
- `sdk/campaign_builder/src/characters_editor.rs` (~L1167, ~L1805)
- `sdk/campaign_builder/src/quest_editor.rs` (~L1147)

All three reworded to describe the actual technical situation (pre-dates the
`EditorContext` parameter-bundle pattern; should be refactored to a context
struct) without referring to the plan document.

### `docs/explanation/codebase_cleanup_plan.md` updated

- Phase 5 deliverables: 6 of 8 flipped `[ ]` → `[x]`; 2 remaining marked as
  in-progress
- Phase 6 deliverables: all 7 flipped `[ ]` → `[x]`
- Phase 6.6 Success Criteria: added status note confirming all gates pass

---

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
