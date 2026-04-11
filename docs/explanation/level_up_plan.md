<!-- SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Character Leveling System Implementation Plan

## Overview

Implement a fully configurable character leveling system for Antares. The
system introduces two XP-curve strategies (formula-based and explicit
per-class tables via `levels.ron`), two level-up trigger modes (`Auto` and
`NpcTrainer`), the complete Might & Magic-style trainer-NPC flow (dialogue,
gold fee, `GameMode::Training`), and a new SDK **Levels Editor** tab. The
existing `domain/progression.rs` domain logic is already correct; this plan
wires it up end-to-end and extends it where needed.

---

## Current State Analysis

### Existing Infrastructure

| Layer            | File(s)                                               | What Exists                                                                                                                                                                          |
| ---------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| XP formula       | `src/domain/progression.rs`                           | `experience_for_level(level)` — hardcoded `BASE_XP=1000`, `XP_MULTIPLIER=1.5`; `award_experience`, `check_level_up`, `level_up_and_grant_spells` all implemented and tested          |
| XP award         | `src/game/systems/combat.rs`                          | `process_combat_victory_with_rng` splits monster XP among living recipients; `award_experience` called correctly                                                                     |
| XP award         | `src/application/quests.rs`, `src/domain/dialogue.rs` | `QuestReward::Experience` and `DialogueAction::GrantExperience` exist                                                                                                                |
| Campaign config  | `src/domain/campaign.rs`                              | `CampaignConfig` has `experience_rate: f32` (unused in awards), `max_party_level: Option<u32>` (unenforced)                                                                          |
| NPC flags        | `src/domain/world/npc.rs`                             | `is_merchant`, `is_innkeeper`, `is_priest` established pattern to follow                                                                                                             |
| Dialogue actions | `src/domain/dialogue.rs`                              | `DialogueAction::OpenMerchant`, `DialogueSdkManagedContent` merchant variants, `standard_merchant_template`, `ensure_standard_merchant_branch` — full pattern to mirror for trainers |
| Game modes       | `src/application/mod.rs`                              | `TempleService(TempleServiceState)`, `InnManagement(InnManagementState)` — established pattern for NPC-service game modes                                                            |
| SDK NPC editor   | `sdk/campaign_builder/src/npc_editor/mod.rs`          | `is_merchant` checkbox, `create_merchant_dialog` button, `MerchantDialogueValidationState` — full pattern to follow                                                                  |
| SDK tabs         | `sdk/campaign_builder/src/lib.rs`                     | `EditorTab` enum; `StockTemplates` is the closest structural sibling                                                                                                                 |
| Campaign data    | `sdk/campaign_builder/src/editor_state.rs`            | `CampaignData` struct holds per-tab data vectors                                                                                                                                     |
| Config editor    | `sdk/campaign_builder/src/config_editor.rs`           | Key-binding rows pattern; settings sections pattern                                                                                                                                  |

### Identified Issues

1. `check_level_up` and `level_up_and_grant_spells` are **never called** from any game system — characters accumulate XP forever without advancing.
2. `experience_rate` from `CampaignConfig` is **not applied** when awarding XP in combat or quests.
3. `max_party_level` from `CampaignConfig` is **not enforced** by the level-up functions.
4. `BASE_XP` and `XP_MULTIPLIER` are **private constants** with no per-campaign override path.
5. No per-class XP curve — all classes share the same threshold formula.
6. No `is_trainer` flag, no trainer NPC flow, no `GameMode::Training`.
7. No `levels.ron` data file, no `LevelDatabase` domain type, no loader.
8. SDK has no Levels Editor tab; NPC editor has no trainer support.

---

## Implementation Phases

---

### Phase 1: Domain — `LevelDatabase` and `levels.ron`

Introduce the data model for explicit per-class XP tables and the fallback
formula configuration. This is the foundation everything else builds on.

#### 1.1 New Domain File — `src/domain/levels.rs`

Create `src/domain/levels.rs` with the following public types:

**`ClassLevelThresholds`** — XP thresholds for one class:

- `class_id: ClassId` — matches an id in `classes.ron`
- `thresholds: Vec<u64>` — indexed by `(level - 1)`; `thresholds[0]` is
  always `0` (XP required to be level 1); `thresholds[1]` is XP required to
  reach level 2; up to 200 entries. If a character is at a level beyond the
  end of the vector the last delta is repeated (cap behaviour).

**`LevelDatabase`** — top-level container loaded from `levels.ron`:

- `entries: Vec<ClassLevelThresholds>` — serialised list
- Internal `HashMap<ClassId, ClassLevelThresholds>` built on construction
- `pub fn get(&self, class_id: &str) -> Option<&ClassLevelThresholds>`
- `pub fn threshold_for_class(&self, class_id: &str, level: u32) -> Option<u64>`
  — returns `Some(xp)` when the class has an explicit table, `None` to
  signal fallback to formula
- `pub fn load_from_file(path: &Path) -> Result<Self, LevelError>` — RON
  deserialisation
- Derive `Serialize`, `Deserialize`, `Debug`, `Clone`

**`LevelError`** — `thiserror` enum with `LoadError(String)`,
`ParseError(String)`, `ClassNotFound(String)`.

Add `pub mod levels;` to `src/domain/mod.rs` and re-export `LevelDatabase`,
`ClassLevelThresholds`, `LevelError`.

#### 1.2 Update `src/domain/progression.rs`

Add two new public functions alongside the existing ones:

**`experience_for_level_class(level, class_id, db: Option<&LevelDatabase>) -> u64`**
— if `db` is `Some` and the class has an explicit table, use it; otherwise
fall through to the existing formula. The existing `experience_for_level`
keeps its current signature for backward compatibility.

**`check_level_up_with_db(character, db: Option<&LevelDatabase>) -> bool`**
— replaces the body of `check_level_up` but accepts an optional database.
Keep `check_level_up` as a thin wrapper calling
`check_level_up_with_db(character, None)`.

Update `level_up_from_db` and `level_up_and_grant_spells` to accept
`level_db: Option<&LevelDatabase>` (default `None` callers are unaffected via
a new `*_with_level_db` variant or optional parameter pattern — choose
whichever avoids breaking existing call sites).

Enforce `max_party_level` from `CampaignConfig` inside `level_up_from_db`:
if `character.level >= campaign_config.max_party_level.unwrap_or(MAX_LEVEL)`
return `Err(ProgressionError::MaxLevelReached)`.

#### 1.3 Test Fixture — `data/test_campaign/data/levels.ron`

Create a small but complete fixture covering:

- One class with explicit thresholds (e.g., `"knight"` with 10 levels
  defined, then a delta for remaining levels)
- One class using the flat/step model
- Verify it round-trips through RON without loss

#### 1.4 Update `src/domain/campaign_loader.rs`

Extend `CampaignLoader::load_game_data()` to optionally load `levels.ron`
from `campaign/data/levels.ron`. If the file is absent, `GameData::levels`
is `None` (the formula fallback is used). Add `pub levels:
Option<LevelDatabase>` to `GameData`.

#### 1.5 Testing Requirements

- Unit tests for `ClassLevelThresholds` covering boundary conditions (level
  beyond end of vector, level 1, level 200).
- Tests for `threshold_for_class` with and without a matching entry.
- Tests for `experience_for_level_class` verifying formula fallback when no
  database entry exists.
- Tests for `check_level_up_with_db` against both formula and explicit table.
- Round-trip serialisation test for `LevelDatabase` via RON.
- `data/test_campaign/data/levels.ron` must be loadable by the existing
  campaign loader test suite without errors.

#### 1.6 Deliverables

- [ ] `src/domain/levels.rs` — `LevelDatabase`, `ClassLevelThresholds`, `LevelError`
- [ ] `src/domain/mod.rs` — `pub mod levels` + re-exports
- [ ] `src/domain/progression.rs` — `experience_for_level_class`,
      `check_level_up_with_db`, `max_party_level` enforcement
- [ ] `data/test_campaign/data/levels.ron` — test fixture
- [ ] `src/domain/campaign_loader.rs` — optional `levels.ron` loading, `GameData::levels`
- [ ] All quality gates pass: `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo nextest run`

#### 1.6 Success Criteria

- `experience_for_level_class("knight", 2, Some(&db))` returns the value from
  the fixture, not the formula value.
- `experience_for_level_class("unknown_class", 2, Some(&db))` falls back to the
  formula without panicking.
- `check_level_up_with_db` returns `true` exactly when XP ≥ threshold.
- `level_up_from_db` returns `MaxLevelReached` when `level >= max_party_level`.
- All existing `progression.rs` tests continue to pass unchanged.

---

### Phase 2: Campaign Config — XP Curve and Level-Up Mode

Wire the per-campaign formula overrides and the `LevelUpMode` switch into
`CampaignConfig` and `config.ron`.

#### 2.1 Extend `CampaignConfig` in `src/domain/campaign.rs`

Add the following fields (all with `#[serde(default)]` for backward
compatibility with existing saves and `config.ron` files):

```
pub base_xp: u64,                       // default 1000  (replaces private BASE_XP)
pub xp_multiplier: f64,                 // default 1.5   (replaces private XP_MULTIPLIER)
pub level_up_mode: LevelUpMode,         // default LevelUpMode::Auto
pub training_fee_base: u32,             // default 500   (gold per level for NpcTrainer mode)
pub training_fee_multiplier: f32,       // default 1.0   (scaled by level number)
```

**`LevelUpMode`** — new `pub enum` in `src/domain/campaign.rs`:

```
pub enum LevelUpMode {
    /// Characters level up automatically the moment XP threshold is reached.
    Auto,
    /// Characters must visit a trainer NPC and pay a gold fee to level up.
    NpcTrainer,
}
impl Default for LevelUpMode { fn default() -> Self { Self::Auto } }
```

Derive `Serialize`, `Deserialize`, `Debug`, `Clone`, `PartialEq`, `Eq`.

#### 2.2 Update `experience_for_level_class` Signature

Change the function to accept `base_xp: u64` and `xp_multiplier: f64`
parameters in addition to the `LevelDatabase` option. Callers that have a
`CampaignConfig` pass `config.base_xp` and `config.xp_multiplier`;
callers that have neither pass the constants as defaults.

Convenience wrapper: `experience_for_level_with_config(level, class_id,
campaign_config: &CampaignConfig, level_db: Option<&LevelDatabase>) -> u64`.

#### 2.3 Apply `experience_rate` in Award Sites

In `process_combat_victory_with_rng` (`src/game/systems/combat.rs`), multiply
the XP share by `global_state.0.campaign_config.experience_rate` before
calling `award_experience`. Do the same in `src/application/quests.rs`
`apply_rewards` for `QuestReward::Experience`.

#### 2.4 Update `data/test_campaign/config.ron`

Add the new `CampaignConfig` fields with their default values so the test
campaign config remains valid and explicitly opts in to defaults.

#### 2.5 Testing Requirements

- Test that `experience_rate = 2.0` doubles the XP awarded in a mock combat
  victory.
- Test that `base_xp = 500` + `xp_multiplier = 2.0` produces the expected
  threshold for level 5.
- Test `LevelUpMode` serialisation round-trip.
- Test that `training_fee_base` and `training_fee_multiplier` serialise and
  deserialise correctly.

#### 2.6 Deliverables

- [ ] `src/domain/campaign.rs` — `LevelUpMode` enum, new `CampaignConfig` fields
- [ ] `src/domain/progression.rs` — `experience_for_level_with_config` wrapper
- [ ] `src/game/systems/combat.rs` — `experience_rate` applied in XP award loop
- [ ] `src/application/quests.rs` — `experience_rate` applied in `apply_rewards`
- [ ] `data/test_campaign/config.ron` — updated with new fields at defaults

#### 2.7 Success Criteria

- `experience_rate = 0.5` halves post-combat XP; `experience_rate = 2.0`
  doubles it.
- `LevelUpMode::Auto` is the default when the field is absent in `config.ron`.
- All prior save-game tests continue to pass (new fields use `serde(default)`).

---

### Phase 3: Auto Level-Up Game System

When `LevelUpMode::Auto` is configured, characters that cross an XP threshold
level up immediately without visiting a trainer.

#### 3.1 New File — `src/game/systems/progression.rs`

Create a Bevy system `auto_level_up_system` with parameters:

- `mut global_state: ResMut<GlobalState>`
- `content: Option<Res<GameContent>>`
- `mut game_log: Option<ResMut<GameLog>>`

System logic:

1. Return early if `global_state.0.campaign_config.level_up_mode !=
LevelUpMode::Auto`.
2. Skip if `global_state.0.mode` is `Combat(_)` (level-up deferred to after
   battle).
3. For each living party member, call
   `check_level_up_with_db(&member, level_db)`.
4. If eligible and `Auto`: call `level_up_and_grant_spells` with the class and
   spell databases from `GameContent`.
5. Write a `GameLog` entry per level-up:
   `"{name} advanced to level {n}! (+{hp} HP{spell_msg})"`.
6. If a member can level up multiple times in one pass (large XP award), loop
   until `check_level_up_with_db` returns `false`.

Add a `ProgressionPlugin` that registers the system in `Update` after
`consume_game_log_events`.

#### 3.2 Register Plugin in `src/bin/antares.rs`

Add `app.add_plugins(antares::game::systems::progression::ProgressionPlugin)`.

#### 3.3 Testing Requirements

- Unit test: mock a party member with sufficient XP; call
  `auto_level_up_system` via a minimal Bevy `App`; assert `level` incremented
  and log entry was written.
- Test that the system is a no-op in `NpcTrainer` mode.
- Test that a character is not levelled during `GameMode::Combat`.
- Test multi-level-up in one pass (character with XP for three levels at once).
- Test that dead characters are skipped.

#### 3.4 Deliverables

- [ ] `src/game/systems/progression.rs` — `auto_level_up_system`, `ProgressionPlugin`
- [ ] `src/game/systems/mod.rs` — `pub mod progression`
- [ ] `src/bin/antares.rs` — `ProgressionPlugin` registered

#### 3.5 Success Criteria

- A character that earns enough XP in combat levels up before the next
  exploration frame completes.
- Level-up message appears in the game log.
- Auto-level does not fire during combat or when mode is `NpcTrainer`.
- `max_party_level` cap is respected.

---

### Phase 4: NPC Trainer — Domain Layer

Extend `NpcDefinition`, the dialogue system, and application state to support
trainer NPCs.

#### 4.1 Extend `NpcDefinition` in `src/domain/world/npc.rs`

Add to the struct (all `#[serde(default)]`):

```
pub is_trainer: bool,
pub training_fee_base: Option<u32>,      // overrides CampaignConfig::training_fee_base
pub training_fee_multiplier: Option<f32>, // overrides CampaignConfig::training_fee_multiplier
```

Add a constructor helper `NpcDefinition::trainer(id, name, portrait_id,
fee_base) -> Self` mirroring the existing `NpcDefinition::merchant` helper.

Add `pub fn training_fee_for_level(&self, level: u32, campaign_config:
&CampaignConfig) -> u32` — computes gold cost using the NPC's override values
if present, otherwise `campaign_config.training_fee_base` ×
`training_fee_multiplier` × `level`.

#### 4.2 Extend `src/domain/dialogue.rs`

**`DialogueAction`** — add new variant:

```
OpenTraining { npc_id: String },
```

And `description()` arm: `"Open training session for '{npc_id}'"`.

**`DialogueSdkManagedContent`** — add four variants mirroring the merchant
set:

```
TrainerTemplateTree,
TrainerBranchInsertion,
TrainerChoice,
TrainerOpenNode,
```

**`DialogueTree`** — add methods mirroring the merchant equivalents:

- `has_sdk_managed_trainer_content() -> bool`
- `standard_trainer_template(npc_id, npc_name) -> DialogueTree` — root node
  greets the player; choice "I seek training" routes to a node that fires
  `OpenTraining { npc_id }`; choice "Farewell" ends dialogue.
- `ensure_standard_trainer_branch(npc_id, npc_name) -> bool` — inserts the
  branch if absent, returns whether a change was made.
- `remove_sdk_managed_trainer_content() -> bool`
- `is_sdk_managed_trainer_choice(&DialogueChoice) -> bool`

**`DialogueSdkMetadata`** — add `has_trainer_content() -> bool`.

#### 4.3 New `GameMode::Training` in `src/application/mod.rs`

Add variant:

```
Training(TrainingState),
```

New `pub struct TrainingState`:

```
pub npc_id: String,
pub eligible_member_indices: Vec<usize>,  // indices into party.members
pub selected_member_index: Option<usize>,
pub status_message: Option<String>,
```

Add `impl TrainingState` with `pub fn new(npc_id: String) -> Self`.

#### 4.4 Training Service in `src/application/resources.rs`

Add `pub fn perform_training_service(game_state: &mut GameState, npc_id:
&str, party_index: usize, class_db: &ClassDatabase, spell_db: &SpellDatabase,
level_db: Option<&LevelDatabase>, rng: &mut impl Rng) -> Result<(u16,
Vec<SpellId>), TrainingError>`.

Logic:

1. Validate: NPC `is_trainer`, character is alive and eligible for level-up.
2. Compute gold fee via `npc.training_fee_for_level(char.level, campaign_config)`.
3. Check `party.gold >= fee`; return `Err(TrainingError::InsufficientGold)` if not.
4. Deduct gold, call `level_up_and_grant_spells`.
5. Return `Ok((hp_gained, spells_granted))`.

New `TrainingError` thiserror enum: `NotATrainer`, `CharacterNotEligible`,
`InsufficientGold { need: u32, have: u32 }`, `LevelUpFailed(ProgressionError)`.

#### 4.5 Update Dialogue Execution in `src/game/systems/dialogue.rs`

Handle `DialogueAction::OpenTraining { npc_id }` in `execute_action`:

1. Find the NPC definition by `npc_id`.
2. Build `TrainingState` with `eligible_member_indices` = indices of living
   members where `check_level_up_with_db` returns `true`.
3. Set `global_state.0.mode = GameMode::Training(state)`.
4. If `NpcTrainer` mode is not configured in `campaign_config`, log a warning
   and do nothing (guard against misconfiguration).

#### 4.6 Testing Requirements

- Test `NpcDefinition::training_fee_for_level` with and without override values.
- Test `training_fee_for_level` at level 1, 5, 10.
- Test `DialogueTree::standard_trainer_template` produces a valid tree with
  `OpenTraining` action.
- Test `ensure_standard_trainer_branch` is a no-op when a branch already exists.
- Test `perform_training_service` succeeds with correct fee deduction and
  level-up.
- Test `perform_training_service` returns `InsufficientGold` correctly.
- Test `perform_training_service` returns `CharacterNotEligible` when XP is
  insufficient.
- All test NPC definitions in `data/test_campaign` must add `is_trainer:
false` at default — verify fixture files compile cleanly.

#### 4.7 Deliverables

- [ ] `src/domain/world/npc.rs` — `is_trainer`, fee fields, `trainer()` constructor, `training_fee_for_level()`
- [ ] `src/domain/dialogue.rs` — `OpenTraining` action, trainer `DialogueSdkManagedContent` variants, `DialogueTree` trainer methods
- [ ] `src/application/mod.rs` — `GameMode::Training(TrainingState)`, `TrainingState`
- [ ] `src/application/resources.rs` — `perform_training_service`, `TrainingError`
- [ ] `src/game/systems/dialogue.rs` — `OpenTraining` handler in `execute_action`

#### 4.8 Success Criteria

- `perform_training_service` correctly levels up the character, deducts gold,
  and grants spells.
- `GameMode::Training` is entered when a dialogue node fires `OpenTraining`.
- All existing dialogue tests pass unchanged.
- `cargo nextest run` green.

---

### Phase 5: NPC Trainer — Game UI System

Render the `GameMode::Training` screen and handle player input within it.

#### 5.1 New File — `src/game/systems/training_ui.rs`

`TrainingPlugin` registers the following systems in `Update`:

**`training_ui_system`** — renders the training screen using Bevy UI:

- Header: `"Training Grounds — {npc_name}"`.
- For each `eligible_member_indices` entry: character name, current level,
  XP progress, gold fee for next level, a selectable row.
- "Train" button (disabled if selected member has insufficient gold or none
  selected).
- "Leave" button to exit back to Exploration.
- Status bar showing `state.status_message`.

**`training_input_system`** — keyboard/mouse handling:

- Arrow Up / Arrow Down: cycle `selected_member_index`.
- Enter / Space: trigger training for selected member; call
  `perform_training_service`; update `status_message` with result.
- Escape: exit to Exploration.

**`training_cleanup_system`** — despawns training UI entities when
`GameMode` is no longer `Training`.

#### 5.2 Register Plugin in `src/bin/antares.rs`

Add `app.add_plugins(antares::game::systems::training_ui::TrainingPlugin)`.

#### 5.3 Testing Requirements

- Test that entering `GameMode::Training` with no eligible members shows an
  empty list without panicking.
- Test that pressing Escape transitions back to `GameMode::Exploration`.
- Test that a successful training call updates `status_message` with the
  level-up result.
- Test that the system is a complete no-op when mode is not `Training`.

#### 5.4 Deliverables

- [ ] `src/game/systems/training_ui.rs` — `TrainingPlugin`, UI and input systems
- [ ] `src/game/systems/mod.rs` — `pub mod training_ui`
- [ ] `src/bin/antares.rs` — `TrainingPlugin` registered

#### 5.5 Success Criteria

- Training screen renders correctly for all eligible party members.
- Gold cost per member is accurate.
- Character advances on "Train", gold is deducted, updated level visible.
- Ineligible characters (wrong XP or dead) are not listed.

---

### Phase 6: SDK — Levels Editor Tab

New SDK editor tab for creating and editing `levels.ron` files.

#### 6.1 New File — `sdk/campaign_builder/src/levels_editor.rs`

Implement `LevelsEditorState` following the two-column layout pattern used
by `StockTemplatesEditorState`:

**Left panel — class list:**

- Scrollable list of class IDs that have entries in `campaign_data.levels`.
- "+ Add Class" button opens an Add form with class-ID autocomplete drawn
  from the campaign's loaded `ClassDefinition` list.
- Selected row highlights the active entry for editing in the right panel.
- Each row shows: class ID badge + first few threshold values as a preview.

**Right panel — threshold editor:**

- Header: `"Level Thresholds — {class_id}"`.
- `class_id` text field with autocomplete against `campaign_data.classes`
  IDs (must be unique across all entries).
- Scrollable table with columns: `Level`, `XP Required`, `Delta from prev`.
  - Pre-populated with 200 rows when a new entry is created (calculated
    from the default formula so the user starts from a sensible baseline).
  - Rows are editable `u64` fields. Editing a row updates all `Delta`
    display values downstream automatically.
- **"Fill Formula"** button: overwrites all thresholds using the
  `base_xp` + `xp_multiplier` from the current `CampaignConfig`.
- **"Fill Flat"** button: opens a small modal asking for a delta value;
  fills thresholds as `level * delta` (cumulative totals).
- **"Fill Step"** button: opens a modal asking for a base value, a step
  value, and a breakpoint level; fills thresholds as a step-wise linear
  table.
- Save / Cancel buttons.

**Auto-populate on tab open:** if `campaign_metadata.levels_file` is set and
the file exists on disk, load it automatically; display an empty left panel
otherwise.

#### 6.2 Extend `CampaignData` in `sdk/campaign_builder/src/editor_state.rs`

Add:

```
pub levels: Vec<ClassLevelThresholds>,
```

Add `levels_editor_state: levels_editor::LevelsEditorState` to
`EditorRegistry`.

#### 6.3 Add `EditorTab::Levels` to `sdk/campaign_builder/src/lib.rs`

Insert `Levels` in `EditorTab` enum (between `Classes` and `Races` fits the
game-data grouping). Add `"Levels"` arm to `EditorTab::name()`. Add to the
`tabs` array in `CampaignBuilderApp::update`.

Add the match arm in the central-panel `match self.ui_state.active_tab`
block to render `self.editor_registry.levels_editor_state.show(...)`.

#### 6.4 Extend `CampaignMetadata` Levels File Field

In `sdk/campaign_builder/src/campaign_editor.rs`, add
`levels_file: String` to `CampaignMetadataEditBuffer` and wire it to the
Files grid alongside the other data-file paths.

In `campaign_io.rs`, add loading of `levels.ron` into `CampaignData::levels`
when opening a campaign.

#### 6.5 Save `levels.ron` on Campaign Save

In `campaign_io.rs`, serialise `campaign_data.levels` to the path specified
by `campaign_metadata.levels_file` (defaulting to `data/levels.ron`) when
the user saves the campaign.

#### 6.6 Testing Requirements

- Test `LevelsEditorState::default()` produces an empty list with no panics.
- Test adding a new class entry auto-populates 200 rows from the formula.
- Test "Fill Flat" with delta 5000 produces cumulative totals 0, 5000,
  10000, 15000 at levels 1–4.
- Test "Fill Step" with base 1000, step 500, breakpoint 10 produces the
  correct step-wise thresholds.
- Test editing threshold row `n` recomputes `Delta` column for rows `n` and
  `n+1`.
- Test that campaign save writes a parseable `levels.ron` file and reload
  round-trips without loss.
- All SDK `cargo nextest run` tests pass.

#### 6.7 Deliverables

- [ ] `sdk/campaign_builder/src/levels_editor.rs` — `LevelsEditorState`, full two-column UI
- [ ] `sdk/campaign_builder/src/editor_state.rs` — `CampaignData::levels`, `EditorRegistry::levels_editor_state`
- [ ] `sdk/campaign_builder/src/lib.rs` — `EditorTab::Levels`, `name()`, tab render arm
- [ ] `sdk/campaign_builder/src/campaign_editor.rs` — `levels_file` in buffer
- [ ] `sdk/campaign_builder/src/campaign_io.rs` — load and save `levels.ron`

#### 6.8 Success Criteria

- Opening a campaign with an existing `levels.ron` populates the left panel
  automatically.
- Opening a campaign without `levels.ron` shows an empty panel without errors.
- Class-ID autocomplete lists all classes from `classes.ron`.
- Fill Formula / Fill Flat / Fill Step produce correct threshold arrays.
- Saving the campaign writes a RON file that passes `cargo check` parsing.

---

### Phase 7: SDK — NPC Editor Trainer Support

Mirror the full `is_merchant` + `create_merchant_dialog` pattern for trainers
in `sdk/campaign_builder/src/npc_editor/mod.rs`.

#### 7.1 Extend `NpcEditBuffer`

Add fields:

```
pub is_trainer: bool,
pub training_fee_base: String,        // text buffer; empty = use campaign default
pub training_fee_multiplier: String,  // text buffer; empty = use campaign default
```

#### 7.2 Trainer Dialogue Validation

Add `TrainerDialogueValidationState` enum mirroring
`MerchantDialogueValidationState`:

```
NotTrainer,
Valid,
Missing,
AssignedDialogueMissing,
StaleTrainerContent,
```

Add `trainer_dialogue_validation_for_definition` private method mirroring
`merchant_dialogue_validation_for_definition`.

#### 7.3 List View Trainer Badge

In `show_list_view`, display a `🎓` badge on NPCs with `is_trainer: true`,
similar to the `🏪` badge for merchants.

#### 7.4 Edit View Trainer Controls

In `show_edit_view`, inside the NPC-role checkboxes section:

- `ui.checkbox(&mut self.edit_buffer.is_trainer, "🎓 Is Trainer")`.
- When checked: auto-call `auto_apply_trainer_dialogue_to_edit_buffer()`
  (mirrors `auto_apply_merchant_dialogue_to_edit_buffer`).
- When unchecked: call `remove_trainer_dialogue_from_edit_buffer()`.
- When `is_trainer` is `true`, show:
  - `training_fee_base` text field (label: "Training Fee Base (gold per level)").
  - `training_fee_multiplier` text field.
  - `"Create trainer dialogue"` button → calls
    `create_or_repair_trainer_dialogue_for_buffer()`.
  - `"Repair trainer dialogue"` button.

#### 7.5 Trainer Dialogue Logic Methods

Add private methods mirroring the merchant equivalents:

- `auto_apply_trainer_dialogue_to_edit_buffer() -> Result<String, String>`
- `remove_trainer_dialogue_from_edit_buffer() -> Result<String, String>`
- `create_or_repair_trainer_dialogue_for_buffer() -> Result<String, String>`
  — calls `DialogueTree::standard_trainer_template` or
  `ensure_standard_trainer_branch` as appropriate.
- `trainer_dialogue_status_for_buffer() -> TrainerDialogueValidationState`

#### 7.6 Build NPC From Buffer

In `build_npc_from_edit_buffer` and `save_npc`, populate `is_trainer`,
`training_fee_base`, `training_fee_multiplier` from the edit buffer.

#### 7.7 Filters

Add `filter_trainers: bool` to `NpcEditorState` (mirrors `filter_merchants`).
Wire to a `🎓 Trainers` filter chip in the search bar.

#### 7.8 Testing Requirements

- Test that checking `is_trainer` auto-applies a training dialogue template.
- Test that `create_or_repair_trainer_dialogue_for_buffer` returns a guidance
  string (not empty) when `is_trainer == false`.
- Test that `create_or_repair_trainer_dialogue_for_buffer` creates a tree with
  `OpenTraining` action when `is_trainer == true`.
- Test `build_npc_from_edit_buffer` round-trips `is_trainer`, fee fields.
- Test the `filter_trainers` flag hides non-trainer NPCs.
- All existing NPC editor tests pass unchanged.

#### 7.9 Deliverables

- [ ] `sdk/campaign_builder/src/npc_editor/mod.rs` — `is_trainer` + fee fields in buffer, edit view UI, all trainer dialogue logic methods, list badge, filter

#### 7.10 Success Criteria

- Checking `🎓 Is Trainer` immediately creates or auto-applies a training
  dialogue tree.
- "Create trainer dialogue" produces a tree whose dialogue preview shows the
  "I seek training" choice.
- Unchecking `is_trainer` removes SDK-managed trainer content without touching
  unrelated dialogue nodes.
- `is_merchant` and `is_trainer` are fully independent; an NPC may be both.

---

### Phase 8: SDK — Config Editor `LevelUpMode` and XP Formula Settings

Expose the new `CampaignConfig` fields in the SDK Config editor tab.

#### 8.1 Extend `ConfigEditorState` in `sdk/campaign_builder/src/config_editor.rs`

Add buffer fields:

```
pub level_up_mode_is_npc: bool,    // false = Auto, true = NpcTrainer
pub base_xp_buffer: String,
pub xp_multiplier_buffer: String,
pub training_fee_base_buffer: String,
pub training_fee_multiplier_buffer: String,
```

#### 8.2 Wire Buffer ↔ Config Conversions

In `update_edit_buffers`: populate all five fields from
`game_config.campaign.level_up_mode`, `base_xp`, etc.

In `update_config_from_buffers`: parse and write back, clamping to valid
ranges (`base_xp > 0`, `xp_multiplier > 1.0`,
`training_fee_base >= 0`, `training_fee_multiplier > 0.0`).

#### 8.3 Render a New "Leveling" Section in the Config UI

Inside the config editor's main panel, add a `"Leveling"` collapsible section
(after the existing "Game Log" section) containing:

- `"Level-Up Mode"` row: radio buttons `Auto` / `NPC Trainer`.
- `"Base XP"` row: numeric text field (tooltip: "XP required to reach level
  2; subsequent levels scale from this").
- `"XP Multiplier"` row: float field (tooltip: "Exponential curve exponent;
  1.5 = default, 1.0 = flat, 2.0 = steep").
- `"Training Fee Base"` row: numeric field (visible only when NPC Trainer
  selected; tooltip: "Gold per level for NPC trainer").
- `"Training Fee Multiplier"` row: float field (visible only when NPC Trainer
  selected).

#### 8.4 Testing Requirements

- Test that `update_edit_buffers` populates all five buffers correctly.
- Test that `update_config_from_buffers` parses and clamps all five fields.
- Test buffer defaults match `CampaignConfig::default()` values.
- Test that Training Fee rows are only rendered when `level_up_mode_is_npc ==
true`.

#### 8.5 Deliverables

- [ ] `sdk/campaign_builder/src/config_editor.rs` — new buffer fields, buffer ↔ config wiring, Leveling UI section

#### 8.6 Success Criteria

- Changing mode to "NPC Trainer" and saving the campaign writes
  `level_up_mode: NpcTrainer` to `config.ron`.
- Changing `base_xp` and reloading the campaign uses the updated value in
  `experience_for_level_with_config`.
- All config editor existing tests pass.

---

### Phase 9: Character Sheet Screen

A read-only, out-of-combat character viewer accessible from the exploration
HUD. Mirrors the `GameMode::Inventory` pattern: a single-character detail
view and a party-overview mode, toggled with Tab / Shift-Tab. No stat edits
are permitted — this is a display-only screen.

#### 9.1 New Application State — `src/application/character_sheet_state.rs`

Create a new file following the exact pattern of
`src/application/inventory_state.rs`.

**`CharacterSheetView`** enum (Serialize, Deserialize, Debug, Clone, PartialEq):

```
pub enum CharacterSheetView {
    /// Detailed single-character panel.
    Single,
    /// Compact overview cards for every party member.
    PartyOverview,
}
impl Default for CharacterSheetView { fn default() -> Self { Self::Single } }
```

**`CharacterSheetState`** struct (Serialize, Deserialize, Debug, Clone,
PartialEq):

| Field           | Type                 | Purpose                                                                  |
| --------------- | -------------------- | ------------------------------------------------------------------------ |
| `previous_mode` | `Box<GameMode>`      | Mode to restore on close (same box-wrapping pattern as `InventoryState`) |
| `focused_index` | `usize`              | Party index of the character currently shown in `Single` view            |
| `view`          | `CharacterSheetView` | Whether the player is in single-detail or party-overview mode            |

**`impl CharacterSheetState`**:

- `pub fn new(previous_mode: GameMode) -> Self` — `focused_index = 0`,
  `view = CharacterSheetView::Single`.
- `pub fn get_resume_mode(&self) -> GameMode` — clones `previous_mode`.
- `pub fn focus_next(&mut self, party_size: usize)` — advances
  `focused_index`, wrapping around. No-op when `party_size == 0`.
- `pub fn focus_prev(&mut self, party_size: usize)` — decrements
  `focused_index`, wrapping.
- `pub fn toggle_view(&mut self)` — flips between `Single` and
  `PartyOverview`.

Add `pub mod character_sheet_state;` to `src/application/mod.rs` alongside
the other `pub mod` declarations.

#### 9.2 Add `GameMode::CharacterSheet` in `src/application/mod.rs`

Insert the new variant into the `GameMode` enum:

```
/// Read-only character stats viewer (out of combat).
///
/// Entered by pressing the character sheet key (default `P`) in Exploration
/// mode. Tab / Shift-Tab cycles through party members. ESC returns to the
/// previous [`GameMode`].
CharacterSheet(crate::application::character_sheet_state::CharacterSheetState),
```

Add `pub fn enter_character_sheet(&mut self)` to `impl GameState`:

- Stores current mode as `previous_mode`.
- Sets `self.mode = GameMode::CharacterSheet(CharacterSheetState::new(prev))`.
- Guard: if already in `CharacterSheet`, do nothing (idempotent).

#### 9.3 Add `GameAction::CharacterSheet` in `src/game/systems/input/keymap.rs`

Add the variant to `GameAction`:

```
/// Open or close the character sheet viewer.
CharacterSheet,
```

In `KeyMap::from_controls_config`, add:

```
insert_action_bindings(&mut bindings, &config.character_sheet, GameAction::CharacterSheet);
```

#### 9.4 Add `character_sheet` Key Binding in `src/sdk/game_config.rs`

Add to `ControlsConfig`:

```
/// Keys for opening the character sheet viewer.
///
/// Default: `["P"]`
#[serde(default = "default_character_sheet_keys")]
pub character_sheet: Vec<String>,
```

Add the private helper:

```
fn default_character_sheet_keys() -> Vec<String> {
    vec!["P".to_string()]
}
```

Wire into `ControlsConfig::default()`:

```
character_sheet: default_character_sheet_keys(),
```

Wire into `ControlsConfig::validate()` (alongside the existing `spell_book`
validation: check that the list is non-empty).

#### 9.5 Wire the Toggle in `src/game/systems/input/global_toggles.rs`

In `handle_global_mode_toggles`, add handling for
`GameAction::CharacterSheet`:

- **Open**: when in `Exploration` (or other non-blocking modes), call
  `game_state.enter_character_sheet()`.
- **Close**: when already in `GameMode::CharacterSheet(state)`, call
  `game_state.mode = state.get_resume_mode()`.
- **Blocked**: no-op when in `Combat(_)`, `Dialogue(_)`,
  `MerchantInventory(_)`, or `Training(_)`.

Follow the exact same `consumed: bool` return pattern used by
`GameAction::Inventory` and `GameAction::OpenSpellBook`.

#### 9.6 New UI System — `src/game/systems/character_sheet_ui.rs`

Create a new file with a `CharacterSheetPlugin` that registers three systems
in `Update`, ordered `setup → ui → input → cleanup`:

**`character_sheet_ui_system`** (egui, runs when
`GameMode::CharacterSheet(_)`) renders two layouts:

_Single-character view_ — full-width egui `Window` titled
`"{name} — Level {level} {race} {class}"`:

| Section           | Content                                                                                                                                                                                                                                                           |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Header**        | Name, sex, age, alignment; `"[< Prev]"` / `"[Next >]"` buttons to cycle characters; `"[Party Overview]"` toggle button                                                                                                                                            |
| **Core Stats**    | Two-column table: Might, Endurance, Intellect, Personality, Speed, Accuracy, Luck; each row shows `base / current` (highlighted amber when `current != base`)                                                                                                     |
| **Combat Stats**  | HP `current/max`, SP `current/max` (hidden for non-casters), AC, Spell Level                                                                                                                                                                                      |
| **Experience**    | Current XP, XP to next level (computed via `experience_for_level_with_config` using the loaded `LevelDatabase` when available; label reads `"Ready to level up!"` in green when eligible in `Auto` mode, or `"Visit a trainer"` in yellow when `NpcTrainer` mode) |
| **Conditions**    | Badge list of active conditions (coloured per condition severity); `"None"` when all clear                                                                                                                                                                        |
| **Equipment**     | 7 equipment slots: slot name + item name (or `"—"` when empty)                                                                                                                                                                                                    |
| **Proficiencies** | Comma-separated list of proficiency IDs granted by class and race                                                                                                                                                                                                 |

_Party overview_ — horizontal scrollable row of compact cards, one per party
member. Each card shows: portrait placeholder (coloured box keyed to
character index), name, class, level, HP bar `current/max`, SP bar
(casters only), conditions badge count, and a `"[View]"` button that
switches to Single view for that member.

**`character_sheet_input_system`** — keyboard handling:

| Key          | Action                                          |
| ------------ | ----------------------------------------------- |
| Escape       | `game_state.mode = state.get_resume_mode()`     |
| Tab          | `state.focus_next(party_size)` (in Single view) |
| Shift-Tab    | `state.focus_prev(party_size)` (in Single view) |
| O            | `state.toggle_view()` (toggle overview)         |
| Left / Right | `focus_prev` / `focus_next` in Single view      |

**`character_sheet_cleanup_system`** — despawns any Bevy-world entities
spawned by the UI when `GameMode` is no longer `CharacterSheet`. (If the
implementation is pure egui with no spawned entities, this system is a
documented no-op stub retained for structural consistency with the other UI
plugins.)

Register the plugin in `src/game/systems/mod.rs` as
`pub mod character_sheet_ui` and in `src/bin/antares.rs`:

```
app.add_plugins(antares::game::systems::character_sheet_ui::CharacterSheetPlugin);
```

#### 9.7 Add `character_sheet` to `data/test_campaign/config.ron`

Append `character_sheet: ["P"]` inside the `ControlsConfig(...)` block so the
test campaign config validates cleanly.

#### 9.8 Extend SDK Config Editor — Character Sheet Key Row

In `src/sdk/campaign_builder/src/config_editor.rs`, following the same
pattern used for the `spell_book` binding row added in Phase 6 of the SDK
Fixes plan:

- Add `controls_character_sheet_buffer: String` to `ConfigEditorState`.
- Populate in `update_edit_buffers` from
  `game_config.controls.character_sheet`.
- Write back in `update_config_from_buffers`.
- Handle key-capture for `controls_character_sheet_buffer` in the capture
  dispatch.
- Render a `"Character Sheet [P]"` row in the Key Bindings section of the
  config UI, placed after the Spell Book row.

#### 9.9 Testing Requirements

- `CharacterSheetState::new` stores `previous_mode` and defaults to
  `focused_index = 0`, `view = Single`.
- `get_resume_mode` returns a clone of the stored mode.
- `focus_next` wraps correctly at `party_size - 1`.
- `focus_prev` wraps correctly at `0`.
- `toggle_view` alternates between `Single` and `PartyOverview`.
- `enter_character_sheet` transitions to `GameMode::CharacterSheet`.
- `enter_character_sheet` is idempotent when already in `CharacterSheet`.
- `handle_global_mode_toggles` opens the sheet from `Exploration`.
- `handle_global_mode_toggles` closes the sheet back to `Exploration`.
- `handle_global_mode_toggles` is blocked in `Combat(_)`.
- `CharacterSheetPlugin` builds without panicking in a minimal Bevy `App`.
- `ControlsConfig::default()` includes `character_sheet: ["P"]`.
- Config editor: buffer round-trips through `update_edit_buffers` →
  `update_config_from_buffers` without loss.

#### 9.10 Deliverables

- [ ] `src/application/character_sheet_state.rs` — `CharacterSheetState`, `CharacterSheetView`
- [ ] `src/application/mod.rs` — `pub mod character_sheet_state`, `GameMode::CharacterSheet`, `enter_character_sheet()`
- [ ] `src/game/systems/input/keymap.rs` — `GameAction::CharacterSheet`, binding wired
- [ ] `src/sdk/game_config.rs` — `ControlsConfig::character_sheet`, default `["P"]`, validation
- [ ] `src/game/systems/input/global_toggles.rs` — open/close/block logic
- [ ] `src/game/systems/character_sheet_ui.rs` — `CharacterSheetPlugin`, UI, input, cleanup systems
- [ ] `src/game/systems/mod.rs` — `pub mod character_sheet_ui`
- [ ] `src/bin/antares.rs` — `CharacterSheetPlugin` registered
- [ ] `data/test_campaign/config.ron` — `character_sheet: ["P"]` added
- [ ] `sdk/campaign_builder/src/config_editor.rs` — Character Sheet key-binding row

#### 9.11 Success Criteria

- Pressing `P` in Exploration opens the character sheet; pressing `P` or
  Escape closes it and restores the prior mode.
- Tab / Shift-Tab cycles through all party members without panicking on a
  single-member party.
- `O` toggles between Single and Party Overview; all members are visible in
  the overview.
- XP section shows `"Ready to level up!"` exactly when
  `check_level_up_with_db` returns `true` and mode is `Auto`; shows
  `"Visit a trainer"` when mode is `NpcTrainer`.
- Conditions section shows active condition names; `"None"` when the
  character has no active conditions.
- The sheet cannot be opened during `Combat(_)` or `Training(_)`.
- All quality gates pass: zero errors, zero warnings.

---

## Data File Reference

### `data/levels.ron` — RON Schema

```
(
    entries: [
        (
            class_id: "knight",
            thresholds: [
                0,      // Level 1: 0 XP
                1000,   // Level 2: 1000 XP total
                2500,   // Level 3: 2500 XP total
                5000,   // Level 4
                7500,   // Level 5
                10000,  // Level 6
                // ...up to 200 entries; extra entries are capped silently
            ],
        ),
        (
            class_id: "sorcerer",
            thresholds: [
                0,
                800,
                2000,
                4000,
                6500,
                9500,
            ],
        ),
    ],
)
```

If a class_id is absent from the file, the formula
`base_xp * (level - 1)^xp_multiplier` (using `CampaignConfig` values) is
used as fallback.

### `config.ron` — New `CampaignConfig` Fields

```
// Inside the existing CampaignConfig(...) block:
level_up_mode: Auto,         // or: NpcTrainer
base_xp: 1000,
xp_multiplier: 1.5,
training_fee_base: 500,
training_fee_multiplier: 1.0,
```

---

## Cross-Cutting Concerns

### Backward Compatibility

All new struct fields use `#[serde(default)]`. Existing saves, `config.ron`
files, and `campaign.ron` files load without migration. The existing
`experience_for_level` function signature is preserved; new callers use the
`_with_config` or `_class` variants.

### AGENTS.md Rule Compliance

- All implementation files in `src/**/*.rs` carry the SPDX header.
- All new `.ron` data files go under `data/test_campaign/data/` for tests;
  never under `campaigns/tutorial/`.
- No test references `campaigns/tutorial`.
- `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo nextest run`
  must all pass at zero errors/warnings after each phase.
- `docs/explanation/implementations.md` updated after each phase completes.

### Implementation Order Rationale

Phases 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 is the recommended delivery
order, but several phases are parallelisable once their dependencies are
stable:

- **Phase 1** (LevelDatabase) must come first — every downstream phase
  reads `experience_for_level_with_config` or `check_level_up_with_db`.
- **Phase 2** (CampaignConfig wiring) depends on Phase 1.
- **Phase 3** (Auto Level-Up) depends on Phases 1 and 2.
- **Phase 4** (Trainer NPC domain) depends on Phases 1 and 2 for
  `perform_training_service`; its dialogue types are independent.
- **Phase 5** (Trainer UI) depends on Phase 4.
- **Phases 6, 7, 8** (SDK — Levels Editor, NPC Editor trainer support,
  Config Editor leveling) are independent of each other and may be
  parallelised once Phase 1–2 types are stable.
- **Phase 9** (Character Sheet) depends only on Phases 1 and 2 for the
  XP-progress display. The state machine, key binding, and UI skeleton can
  be built in parallel with Phases 3–8; the XP readiness indicator is
  wired last once `check_level_up_with_db` and `LevelUpMode` are available.
