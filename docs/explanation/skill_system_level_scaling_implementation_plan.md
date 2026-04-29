<!-- SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Skill System Level Scaling Implementation Plan

## Overview

This plan introduces level-scaled character skills for Antares in a phased,
engine-first approach. The first milestone is an **Auto Skills** system: skills
are defined in campaign RON data and each character receives effective skill
ranks derived from class, race, character level, and optional character-specific
bonuses. Later phases add an NPC **Train Skills** flow that mirrors the existing
NPC level-up trainer route: dialogue action, `GameMode`, paid service, UI, SDK
authoring, and validation.

The plan deliberately separates **proficiencies** from **skills**:

- **Proficiencies** remain binary item-use permissions such as
  `simple_weapon`, `light_armor`, or `arcane_item`.
- **Skills** become numeric capabilities that can scale with level, such as
  disarm traps, identify item, lore, perception, swimming, climbing, diplomacy,
  tracking, or arcane knowledge.

The implementation should preserve the current item proficiency system and add
skills as a new domain concept rather than overloading `ProficiencyDefinition`.

---

## Current State Analysis

### Existing Infrastructure

| Area | Existing File(s) | Current Capability | Relevance |
| --- | --- | --- | --- |
| Runtime character state | `src/domain/character.rs` | `Character` stores level, class, race, attributes, inventory, equipment, spells, XP | Skill ranks must attach to or derive from `Character` without breaking saves |
| Proficiency definitions | `src/domain/proficiency.rs` | Binary item-use definitions loaded from `proficiencies.ron` | Keep separate from numeric skills |
| Class definitions | `src/domain/classes.rs` | Classes grant `Vec<ProficiencyId>` and carry class metadata | Add class skill progression grants here or in separate skill progression data |
| Race definitions | `src/domain/races.rs` | Races grant proficiencies, stat modifiers, resistances, item tag restrictions | Add race skill bonuses here or in separate skill progression data |
| Campaign data | `src/domain/campaign.rs`, SDK loader/database modules | Campaigns load RON data for classes, races, items, spells, NPCs, proficiencies, levels, etc. | Add `skills.ron` and skill progression data to campaign content |
| Level scaling precedent | `src/domain/levels.rs`, `src/domain/progression.rs` | Level thresholds, config-driven leveling, max level enforcement | Follow the same data-driven pattern for skill rank scaling |
| NPC training precedent | `src/application/resources.rs`, `src/application/mod.rs`, `src/game/systems/training_ui.rs`, `src/game/systems/dialogue.rs` | NPC trainer level-up flow with `GameMode::Training`, gold fees, dialogue action, UI | Mirror this route for future NPC skill training |
| NPC definition | `src/domain/world/npc.rs` | NPCs support service flags and training fee overrides | Add skill trainer metadata in a later phase |
| Campaign Builder editor pattern | `sdk/campaign_builder/src/proficiencies_editor.rs` and other editors | Two-column list/detail editors, `EditorToolbar`, validation, import/export | Add a Skills Editor using existing editor rules |
| Test fixtures | `data/test_campaign/data/` | Stable RON fixtures for tests | Add skill fixture data here, never under `campaigns/tutorial` for tests |

### Identified Issues

1. There is no first-class `SkillDefinition` domain model.
2. Characters currently have no numeric skill ranks.
3. Existing `ProficiencyDefinition` is binary and item-focused, so it should not
   be reused as a numeric skill system.
4. No campaign data file exists for skills or skill progression.
5. Class and race data cannot currently express level-scaled skill growth.
6. There is no helper for resolving a character's effective skill rank from
   base grants, level scaling, race bonuses, class bonuses, and temporary effects.
7. No game systems can query skills for mechanics such as traps, discovery,
   dialogue checks, or item identification.
8. No SDK Skills Editor exists.
9. No NPC skill-training flow exists.
10. No validation exists for skill IDs referenced by classes, races, NPCs,
    dialogue checks, maps, or future mechanics.

---

## Implementation Phases

---

### Phase 1: Domain Foundation — Skill Definitions

Create the core data structures and loader for campaign-authored skills. This
phase does not modify gameplay yet; it establishes `skills.ron` as a validated,
data-driven content file.

#### 1.1 Foundation Work

Create `src/domain/skills.rs`.

Public type aliases:

| Alias | Type | Purpose |
| --- | --- | --- |
| `SkillId` | `String` | Stable campaign-authored skill identifier |
| `SkillRank` | `u16` | Numeric skill rank value |
| `SkillTier` | `u8` | Optional display tier or unlock tier |

Public enum: `SkillCategory`.

Initial variants:

| Variant | Meaning |
| --- | --- |
| `Combat` | Combat-adjacent tactical skills |
| `Exploration` | World traversal, traps, perception, discovery |
| `Knowledge` | Lore, identification, arcane/divine knowledge |
| `Social` | Diplomacy, intimidation, bargaining |
| `Utility` | General non-combat utility |

Public enum: `SkillScalingMode`.

Initial variants:

| Variant | Required Fields | Meaning |
| --- | --- | --- |
| `Flat` | none | Rank never increases automatically |
| `Linear` | `base`, `per_level` | `base + per_level * (level - 1)` |
| `Step` | `base`, `per_levels`, `amount` | Increase by `amount` every `per_levels` levels |
| `Table` | `thresholds` or `ranks_by_level` | Explicit rank by level, clamped to last entry |

Public struct: `SkillDefinition`.

Required fields:

| Field | Type | Notes |
| --- | --- | --- |
| `id` | `SkillId` | Unique identifier, lowercase snake_case |
| `name` | `String` | UI display name |
| `category` | `SkillCategory` | UI grouping and filtering |
| `description` | `String` | Tooltip and documentation |
| `scaling` | `SkillScalingMode` | Default auto-scaling behavior |
| `max_rank` | `SkillRank` | Hard cap for effective rank |
| `is_trainable` | `bool` | Whether NPC training can improve it later |

Public struct: `SkillDatabase`.

Required behavior:

| Function | Purpose |
| --- | --- |
| `new()` | Create empty database |
| `load_from_file(path)` | Load RON skill definitions |
| `load_from_string(text)` | Test-friendly RON loader |
| `get(id)` | Return `Option<&SkillDefinition>` |
| `has(id)` | Return bool |
| `all()` | Return all definitions |
| `all_ids()` | Iterate all IDs |
| `by_category(category)` | Filter definitions by category |
| `validate()` | Validate IDs, names, scaling rules, caps, duplicates |

Public error enum: `SkillError`.

Required variants:

| Variant | Meaning |
| --- | --- |
| `SkillNotFound(String)` | Missing skill lookup |
| `LoadError(String)` | File read failure |
| `ParseError(String)` | RON parse failure |
| `ValidationError(String)` | Invalid skill data |
| `DuplicateId(String)` | Duplicate skill ID |

#### 1.2 Add Foundation Functionality

Implement pure scaling helpers in `src/domain/skills.rs`.

Required functions:

| Function | Behavior |
| --- | --- |
| `rank_for_level(definition, level)` | Computes rank from `SkillDefinition::scaling`, clamps to `max_rank` |
| `rank_for_level_with_bonus(definition, level, bonus)` | Adds signed bonus then clamps to `0..=max_rank` |
| `validate_skill_id(id)` | Enforces non-empty lowercase snake_case ID |
| `validate_skill_rank(rank, max_rank)` | Ensures rank is within allowed bounds |

Validation rules:

| Rule | Error Condition |
| --- | --- |
| Skill ID cannot be empty | `ValidationError` |
| Skill ID must be lowercase snake_case | `ValidationError` |
| Skill name cannot be empty | `ValidationError` |
| `max_rank` must be greater than 0 | `ValidationError` |
| `Linear.per_level` must be greater than or equal to 0 | `ValidationError` |
| `Step.per_levels` must be greater than 0 | `ValidationError` |
| `Table.ranks_by_level` must not be empty | `ValidationError` |
| `Table` ranks must not exceed `max_rank` | `ValidationError` |

#### 1.3 Integrate Foundation Work

Update module wiring:

| File | Required Change |
| --- | --- |
| `src/domain/mod.rs` | Add `pub mod skills;` and re-export key skill types |
| `src/sdk/database.rs` or current content database module | Add `skills: SkillDatabase` |
| Campaign loader module | Load `data/skills.ron` when present |
| Campaign metadata/config structs | Add default `skills_file: "data/skills.ron"` if the campaign metadata uses explicit data file names |
| `data/test_campaign/data/skills.ron` | Add stable test fixture |
| `data/skills.ron` | Add base game skill definitions |
| `campaigns/tutorial/data/skills.ron` | Add live campaign skill definitions only after fixtures are stable |

Initial skill fixture examples:

| Skill ID | Category | Scaling | Trainable |
| --- | --- | --- | --- |
| `perception` | `Exploration` | `Linear(base: 0, per_level: 1)` | true |
| `disarm_traps` | `Exploration` | `Step(base: 0, per_levels: 2, amount: 1)` | true |
| `item_lore` | `Knowledge` | `Linear(base: 0, per_level: 1)` | true |
| `diplomacy` | `Social` | `Flat` | true |
| `athletics` | `Utility` | `Step(base: 1, per_levels: 3, amount: 1)` | true |

#### 1.4 Testing Requirements

Add unit tests in `src/domain/skills.rs`.

Required tests:

| Test Name | Assertion |
| --- | --- |
| `test_skill_definition_validate_rejects_empty_id` | Empty ID fails |
| `test_skill_definition_validate_rejects_non_snake_case_id` | Invalid ID fails |
| `test_skill_database_rejects_duplicate_id` | Duplicate IDs fail |
| `test_rank_for_level_flat_returns_base_rank` | Flat scaling is stable |
| `test_rank_for_level_linear_scales_by_level` | Linear scaling uses level |
| `test_rank_for_level_step_scales_at_interval` | Step scaling increments at intervals |
| `test_rank_for_level_table_clamps_after_last_entry` | Table scaling clamps to last rank |
| `test_rank_for_level_clamps_to_max_rank` | Any mode respects max rank |
| `test_skill_database_loads_test_campaign_fixture` | Loads `data/test_campaign/data/skills.ron` |

#### 1.5 Deliverables

- [ ] `src/domain/skills.rs` created with SPDX header and doc comments.
- [ ] `SkillId`, `SkillRank`, `SkillCategory`, `SkillScalingMode`,
      `SkillDefinition`, `SkillDatabase`, and `SkillError` implemented.
- [ ] Skill rank scaling helpers implemented as pure functions.
- [ ] `src/domain/mod.rs` exports skills module.
- [ ] Base and test skill RON files added.
- [ ] Campaign/content loader includes skill database.
- [ ] Unit tests cover scaling and validation.

#### 1.6 Success Criteria

- `SkillDatabase::load_from_file("data/test_campaign/data/skills.ron")` succeeds.
- `rank_for_level` produces deterministic values for all scaling modes.
- Invalid skill IDs fail validation.
- Skills are loaded into campaign content without modifying proficiency behavior.
- No tests reference `campaigns/tutorial`.

---

### Phase 2: Auto Skills — Character Effective Skill Ranks

Implement the initial gameplay-facing Auto Skills system. Characters should have
effective skill ranks derived from class, race, level, and optional
character-specific modifiers.

#### 2.1 Feature Work

Add skill grant structures.

Preferred approach:

| Type | Location | Purpose |
| --- | --- | --- |
| `SkillGrant` | `src/domain/skills.rs` | Data-driven bonus to a skill |
| `SkillGrantSource` | `src/domain/skills.rs` | Optional enum for `Class`, `Race`, `Character`, `Training`, `Temporary` |
| `CharacterSkillRanks` | `src/domain/skills.rs` or `src/domain/character.rs` | Persistent trained/manual ranks keyed by `SkillId` |

`SkillGrant` fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `skill_id` | `SkillId` | Referenced skill |
| `flat_bonus` | `i16` | Always added |
| `per_level_bonus` | `i16` | Added per character level, optional via default 0 |
| `minimum_rank` | `Option<SkillRank>` | Floor for effective rank |
| `maximum_rank_override` | `Option<SkillRank>` | Optional grant-specific cap |

Update class and race definitions:

| File | Field |
| --- | --- |
| `src/domain/classes.rs` | `#[serde(default)] pub skill_grants: Vec<SkillGrant>` |
| `src/domain/races.rs` | `#[serde(default)] pub skill_grants: Vec<SkillGrant>` |

Update character runtime state:

| File | Field |
| --- | --- |
| `src/domain/character.rs` | `#[serde(default)] pub skill_ranks: HashMap<SkillId, SkillRank>` or wrapper type |

The persistent character field should represent explicit character-owned skill
ranks, not the full derived value. Auto-derived ranks should be computed on
demand.

#### 2.2 Integrate Feature

Implement `SkillResolver`.

Preferred location: `src/domain/skills.rs`.

Required API:

| Function | Purpose |
| --- | --- |
| `effective_skill_rank(character, skill_id, skills, classes, races)` | Returns final rank |
| `effective_skill_breakdown(character, skill_id, skills, classes, races)` | Returns rank plus source breakdown for UI/debugging |
| `all_effective_skill_ranks(character, skills, classes, races)` | Returns map of all known skill IDs to ranks |
| `character_has_skill_rank(character, skill_id, minimum, skills, classes, races)` | Predicate for checks |

Effective rank formula:

1. Start with skill definition auto rank for `character.level`.
2. Add class skill grants for `character.class_id`.
3. Add race skill grants for `character.race_id`.
4. Add persistent character `skill_ranks[skill_id]`.
5. Add future temporary modifiers if present.
6. Clamp to `0..=SkillDefinition::max_rank`.
7. Apply any grant `minimum_rank` floors after additive bonuses.
8. Apply any grant `maximum_rank_override` caps before final skill max cap.

If class or race lookup fails, return a recoverable error rather than panicking.

Add error variants:

| Variant | Meaning |
| --- | --- |
| `ClassNotFound(String)` | Character class ID missing |
| `RaceNotFound(String)` | Character race ID missing |
| `InvalidSkillReference(String)` | Skill grant references missing skill |

#### 2.3 Configuration Updates

Update RON fixtures:

| File | Required Updates |
| --- | --- |
| `data/test_campaign/data/classes.ron` | Add `skill_grants` to at least two classes |
| `data/test_campaign/data/races.ron` | Add `skill_grants` to at least two races |
| `data/classes.ron` | Add base skill grants |
| `data/races.ron` | Add base skill grants |
| `campaigns/tutorial/data/classes.ron` | Add live campaign grants after fixtures pass |
| `campaigns/tutorial/data/races.ron` | Add live campaign grants after fixtures pass |

Example class grants:

| Class | Suggested Grants |
| --- | --- |
| `knight` | `athletics`, `leadership` if defined |
| `robber` | `disarm_traps`, `stealth`, `perception` |
| `sorcerer` | `arcane_lore`, `item_lore` |
| `cleric` | `divine_lore`, `diplomacy` |
| `archer` | `perception`, `tracking` |

Example race grants:

| Race | Suggested Grants |
| --- | --- |
| `elf` | `perception`, `arcane_lore` |
| `dwarf` | `stonecunning`, `disarm_traps` |
| `gnome` | `item_lore`, `disarm_traps` |
| `human` | Broad low flat bonus or no bonus |

#### 2.4 Testing Requirements

Required tests:

| Test Name | Assertion |
| --- | --- |
| `test_effective_skill_rank_uses_auto_level_scaling` | Level increases rank with no grants |
| `test_effective_skill_rank_adds_class_grant` | Class bonus applies |
| `test_effective_skill_rank_adds_race_grant` | Race bonus applies |
| `test_effective_skill_rank_adds_character_rank` | Persistent character rank applies |
| `test_effective_skill_rank_clamps_to_skill_max` | Final rank capped |
| `test_effective_skill_rank_missing_skill_returns_error` | Missing skill is recoverable |
| `test_effective_skill_rank_missing_class_returns_error` | Missing class is recoverable |
| `test_effective_skill_rank_missing_race_returns_error` | Missing race is recoverable |
| `test_all_effective_skill_ranks_contains_all_database_skills` | Resolver returns all known skills |
| `test_skill_grants_deserialize_from_test_campaign_classes` | Fixture class grants load |

#### 2.5 Deliverables

- [ ] `SkillGrant` and optional breakdown types implemented.
- [ ] `ClassDefinition.skill_grants` added with serde default.
- [ ] `RaceDefinition.skill_grants` added with serde default.
- [ ] `Character.skill_ranks` or wrapper added with serde default.
- [ ] Effective skill resolver implemented.
- [ ] Class/race fixture data updated.
- [ ] Tests cover auto scaling and grants.

#### 2.6 Success Criteria

- A level 1 and level 10 character of the same class resolve different ranks
  for linearly scaled skills.
- A robber has a higher `disarm_traps` rank than a knight at equal level when
  fixture grants specify that behavior.
- Race grants combine with class grants.
- Existing proficiency-based item usage remains unchanged.
- Existing saves/RON without `skill_grants` or `skill_ranks` still deserialize.

---

### Phase 3: Engine Integration — Skill Checks

Expose skill ranks to game mechanics through a small, deterministic skill-check
API. This phase should avoid a large gameplay rewrite; it should add reusable
checks and integrate one or two low-risk mechanics.

#### 3.1 Feature Work

Create skill check primitives in `src/domain/skills.rs` or
`src/domain/skill_checks.rs`.

Public types:

| Type | Purpose |
| --- | --- |
| `SkillCheckDifficulty` | Static or data-driven difficulty value |
| `SkillCheckRequest` | Skill ID, difficulty, optional modifiers |
| `SkillCheckResult` | Success/failure, rank, roll, margin |
| `SkillCheckError` | Missing skill/class/race or invalid difficulty |

Recommended deterministic API:

| Function | Purpose |
| --- | --- |
| `evaluate_skill_check_without_roll(rank, difficulty)` | Pure rank-vs-difficulty check |
| `roll_skill_check(rank, difficulty, rng)` | Optional randomized check |
| `skill_check_for_character(character, request, dbs, rng)` | Resolve rank then evaluate |

Use deterministic checks first unless a mechanic already expects randomness.

#### 3.2 Integrate Feature

Initial low-risk integrations:

| Mechanic | File Area | Integration |
| --- | --- | --- |
| Trap discovery or disarm preview | Map event / container / trap systems if present | Use `perception` or `disarm_traps` |
| Item identification or lore display | Inventory/item UI if present | Use `item_lore` |
| Dialogue condition checks | Dialogue condition evaluation | Add optional skill threshold condition |

Implement only one integration first if the code path is complex. The goal is
to prove the API and data model.

Dialogue condition extension candidate:

| Field | Type |
| --- | --- |
| `required_skill` | `Option<SkillRequirement>` |

`SkillRequirement` fields:

| Field | Type |
| --- | --- |
| `skill_id` | `SkillId` |
| `minimum_rank` | `SkillRank` |
| `party_scope` | enum: `AnyMember`, `ActiveSpeaker`, `PartyAverage`, `PartyTotal` |

#### 3.3 Configuration Updates

Update fixture data to include at least one skill-gated example.

Acceptable fixture options:

| Fixture | Purpose |
| --- | --- |
| `data/test_campaign/data/dialogues.ron` | Skill-gated dialogue branch |
| `data/test_campaign/data/maps/map_*.ron` | Skill-gated map event |
| Dedicated unit-test RON string | If production fixture updates are too broad |

Do not reference `campaigns/tutorial` in tests.

#### 3.4 Testing Requirements

Required tests:

| Test Name | Assertion |
| --- | --- |
| `test_evaluate_skill_check_without_roll_success_at_threshold` | Rank equal to difficulty succeeds |
| `test_evaluate_skill_check_without_roll_fails_below_threshold` | Rank below difficulty fails |
| `test_roll_skill_check_is_deterministic_with_seeded_rng` | Seeded RNG produces stable result |
| `test_skill_requirement_any_member_uses_best_matching_member` | Party scope works |
| `test_skill_requirement_party_total_sums_members` | Party aggregate works |
| `test_skill_gated_dialogue_condition_allows_qualified_party` | Qualified party passes |
| `test_skill_gated_dialogue_condition_blocks_unqualified_party` | Unqualified party fails |

#### 3.5 Deliverables

- [ ] Skill check request/result types implemented.
- [ ] Deterministic skill check helper implemented.
- [ ] Optional randomized check implemented only if needed.
- [ ] One engine mechanic integrated with skills.
- [ ] Tests cover success/failure and party scope.

#### 3.6 Success Criteria

- Game systems can query skill ranks through one public resolver/check API.
- At least one real game mechanic uses a skill threshold.
- Skill checks are deterministic where possible and unit-testable.
- No UI-specific code is required to perform a skill check.

---

### Phase 4: Auto Skill UI and Character Display

Show effective skill ranks to players and campaign authors. This phase should
make Auto Skills visible before adding paid training.

#### 4.1 Feature Work

Add skill display to character-facing UI.

Candidate files:

| File | Change |
| --- | --- |
| `src/game/systems/character_sheet_ui.rs` or current character sheet module | Add Skills section |
| `src/application/character_sheet_state.rs` if present | Add selected skills category/filter state if needed |
| `src/game/systems/hud.rs` only if character sheet entry points require changes | No skill logic here unless required |

Displayed fields:

| UI Field | Source |
| --- | --- |
| Skill name | `SkillDefinition.name` |
| Category | `SkillDefinition.category` |
| Effective rank | `effective_skill_rank` |
| Breakdown tooltip | `effective_skill_breakdown` |
| Trainable flag | `SkillDefinition.is_trainable` |

The skill display should be read-only in this phase.

#### 4.2 Integrate Feature

Follow existing egui layout rules:

| Requirement | Reason |
| --- | --- |
| Use existing character sheet layout conventions | Avoid new layout regressions |
| If adding columns, use explicit column allocation pattern required by project rules | Prevent clipped scroll areas/buttons |
| Use `id_salt` for every scroll area | egui ID correctness |
| Use `push_id` for every loop row | egui ID correctness |
| Keep hints in title/header rows where applicable | Avoid bottom-bar height bugs |

#### 4.3 Configuration Updates

No new RON required beyond Phase 2 fixtures.

#### 4.4 Testing Requirements

Required tests:

| Test Name | Assertion |
| --- | --- |
| `test_character_sheet_skill_section_renders_without_panic` | Basic render safety |
| `test_skill_display_uses_effective_rank_not_raw_character_rank` | Shows derived rank |
| `test_skill_breakdown_includes_class_and_race_sources` | Breakdown includes grant sources |
| `test_skill_display_handles_missing_skill_database_gracefully` | No panic on absent data |

#### 4.5 Deliverables

- [ ] Character sheet displays skills.
- [ ] Skill ranks grouped or sortable by category.
- [ ] Effective rank breakdown available as tooltip or detail text.
- [ ] Render tests cover missing/empty skill data.

#### 4.6 Success Criteria

- Players can see level-scaled skill ranks.
- Leveling a character changes displayed auto-scaled ranks.
- The UI does not mutate skill state.
- Empty skill database renders a clear placeholder.

---

### Phase 5: SDK Skills Editor

Add Campaign Builder support for authoring `skills.ron` and skill grants. This
phase should follow the existing Proficiencies Editor and SDK rules.

#### 5.1 Feature Work

Create `sdk/campaign_builder/src/skills_editor.rs`.

Editor state:

| Field | Purpose |
| --- | --- |
| `mode` | List/Add/Edit |
| `search_query` | Filter skills |
| `selected_skill` | Current list selection |
| `edit_buffer` | Editable `SkillDefinition` |
| `filter_category` | Category filter |
| `show_import_dialog` | RON import/export |
| `import_export_buffer` | RON text |
| `usage_cache` | Where skills are referenced |

Editor UI requirements:

| Requirement | Notes |
| --- | --- |
| Use `EditorToolbar` | Match other editors |
| Use `TwoColumnLayout` for list/detail | SDK Rule 9 |
| Use `show_standard_list_item` for rows | SDK Rule 15 |
| Use `push_id` in row loops | SDK ID rule |
| Use `ComboBox::from_id_salt` | SDK ID rule |
| Use `horizontal_wrapped` for action rows | Avoid clipping |
| Bottom action row must be `Back to List`, `Save`, `Cancel` | SDK Rule 16 |

Supported editing fields:

| Field | Widget |
| --- | --- |
| `id` | Text field with generated suggestion |
| `name` | Text field |
| `category` | ComboBox |
| `description` | Multiline text |
| `max_rank` | DragValue |
| `is_trainable` | Checkbox |
| `scaling` | Mode ComboBox plus mode-specific fields |

#### 5.2 Integrate Feature

Required integration points:

| File | Change |
| --- | --- |
| `sdk/campaign_builder/src/lib.rs` | Add `EditorTab::Skills` and render route |
| `sdk/campaign_builder/src/editor_state.rs` | Add `skills_editor_state` and `campaign_data.skills` if applicable |
| `sdk/campaign_builder/src/campaign_io.rs` | Load/save `skills.ron` |
| `sdk/campaign_builder/src/asset_manager.rs` | Track `data/skills.ron` |
| `sdk/campaign_builder/src/campaign_editor.rs` | Add Skills file path if metadata uses data file config |
| `sdk/campaign_builder/src/validation.rs` or validation area | Add skill reference validation |
| `sdk/campaign_builder/src/classes_editor.rs` | Add skill grant editor section |
| `sdk/campaign_builder/src/races_editor.rs` | Add skill grant editor section |

Use autocomplete selectors for skill ID references in class/race grant editors.
Do not use raw `TextEdit` for skill ID fields.

#### 5.3 Configuration Updates

Add files:

| File | Purpose |
| --- | --- |
| `data/test_campaign/data/skills.ron` | Stable fixture |
| `campaigns/tutorial/data/skills.ron` | Live campaign content |
| `data/skills.ron` | Base content if base game data is separate |

Update campaign metadata templates if they enumerate data file paths.

#### 5.4 Testing Requirements

Required tests:

| Test Name | Assertion |
| --- | --- |
| `test_skills_editor_state_default` | Defaults to list mode |
| `test_skills_editor_default_skill_validates` | Default buffer is valid |
| `test_skill_category_filter_matches_expected_categories` | Filter works |
| `test_skill_scaling_editor_round_trips_linear` | Linear scaling edits persist |
| `test_skill_scaling_editor_round_trips_step` | Step scaling edits persist |
| `test_skill_usage_cache_tracks_class_references` | Class grants tracked |
| `test_skill_usage_cache_tracks_race_references` | Race grants tracked |
| `test_skill_validation_rejects_unknown_class_skill_reference` | Unknown class skill fails |
| `test_skill_validation_rejects_unknown_race_skill_reference` | Unknown race skill fails |

#### 5.5 Deliverables

- [ ] `skills_editor.rs` created.
- [ ] Skills tab added to Campaign Builder.
- [ ] Load/save support for `skills.ron`.
- [ ] Class and race editors can edit skill grants.
- [ ] Skill validation integrated with campaign validation.
- [ ] SDK tests cover editor and validation behavior.

#### 5.6 Success Criteria

- Campaign authors can create and edit skills in the SDK.
- Campaign authors can assign skill grants to classes and races.
- Unknown skill references are reported by validation.
- SDK egui ID audit passes for all touched UI.
- No tests load skill fixtures from `campaigns/tutorial`.

---

### Phase 6: NPC Train Skills Domain and Application Flow

Add paid NPC skill training after Auto Skills works. This should mirror the
existing NPC level-up training route without merging the two flows.

#### 6.1 Feature Work

Extend NPC definition.

Candidate fields in `src/domain/world/npc.rs`:

| Field | Type | Meaning |
| --- | --- | --- |
| `is_skill_trainer` | `bool` | NPC offers skill training |
| `trainable_skill_ids` | `Vec<SkillId>` | Skills this NPC can train; empty means none |
| `skill_training_fee_base` | `Option<u32>` | Per-NPC fee override |
| `skill_training_fee_multiplier` | `Option<f32>` | Per-rank or per-level multiplier |
| `skill_training_max_rank` | `Option<SkillRank>` | Per-NPC rank cap |

Add campaign config defaults if needed:

| Field | Type | Default |
| --- | --- | --- |
| `skill_training_fee_base` | `u32` | 100 |
| `skill_training_fee_multiplier` | `f32` | 1.0 |
| `skill_training_max_rank` | `SkillRank` | Skill definition max rank |

Create `SkillTrainingError` in `src/application/resources.rs` or a new
`src/application/skill_training.rs`.

Required errors:

| Error | Meaning |
| --- | --- |
| `NotASkillTrainer(String)` | NPC lacks skill trainer flag |
| `SkillNotOffered(String)` | NPC does not train requested skill |
| `SkillNotTrainable(String)` | Skill definition is not trainable |
| `CharacterNotFound(usize)` | Invalid party index |
| `SkillRankAtMaximum` | Character cannot train higher |
| `InsufficientGold { need, have }` | Party cannot pay |
| `SkillResolutionFailed(String)` | Resolver/database error |

Implement `perform_skill_training_service`.

Required arguments:

| Argument | Type |
| --- | --- |
| `game_state` | `&mut GameState` |
| `npc_id` | `&str` |
| `party_index` | `usize` |
| `skill_id` | `&str` |
| `db` | content database reference |

Required behavior:

1. Look up NPC.
2. Verify `is_skill_trainer`.
3. Verify target party member exists and is alive.
4. Verify skill exists.
5. Verify skill is trainable.
6. Verify NPC offers skill.
7. Resolve current effective rank.
8. Check rank cap.
9. Compute fee.
10. Check party gold.
11. Deduct gold.
12. Increment persistent character skill rank by 1 or configured amount.
13. Return new effective rank and fee paid.

#### 6.2 Integrate Feature

Add application mode state.

Preferred new mode:

| Type | Location |
| --- | --- |
| `GameMode::SkillTraining(SkillTrainingState)` | `src/application/mod.rs` |
| `SkillTrainingState` | `src/application/mod.rs` or dedicated file |

`SkillTrainingState` fields:

| Field | Type |
| --- | --- |
| `npc_id` | `String` |
| `eligible_member_indices` | `Vec<usize>` |
| `available_skill_ids` | `Vec<SkillId>` |
| `selected_member_index` | `Option<usize>` |
| `selected_skill_index` | `Option<usize>` |
| `status_message` | `Option<String>` |

Dialogue action:

| File | Change |
| --- | --- |
| `src/domain/dialogue.rs` | Add `DialogueAction::OpenSkillTraining { npc_id: String }` |
| `src/game/systems/dialogue.rs` | Execute action by entering `GameMode::SkillTraining` |
| Dialogue SDK helpers | Add standard branch/template similar to trainer flow if applicable |

#### 6.3 Configuration Updates

Update NPC fixtures:

| File | Change |
| --- | --- |
| `data/test_campaign/data/npcs.ron` | Add one skill trainer NPC |
| `data/test_campaign/data/dialogues.ron` | Add branch/action to open skill training |
| `campaigns/tutorial/data/npcs.ron` | Add live skill trainer later |
| `campaigns/tutorial/data/dialogues.ron` | Add live dialogue branch later |

#### 6.4 Testing Requirements

Required tests:

| Test Name | Assertion |
| --- | --- |
| `test_perform_skill_training_rejects_non_trainer` | NPC flag required |
| `test_perform_skill_training_rejects_unoffered_skill` | Skill offer list enforced |
| `test_perform_skill_training_rejects_untrainable_skill` | `is_trainable` enforced |
| `test_perform_skill_training_rejects_insufficient_gold` | Gold precondition enforced |
| `test_perform_skill_training_increments_character_skill_rank` | Persistent rank increases |
| `test_perform_skill_training_deducts_gold` | Fee deducted |
| `test_perform_skill_training_rejects_max_rank` | Cap enforced |
| `test_open_skill_training_dialogue_enters_skill_training_mode` | Dialogue action works |

#### 6.5 Deliverables

- [ ] NPC skill trainer fields added with serde defaults.
- [ ] Skill training service implemented.
- [ ] `GameMode::SkillTraining` and state added.
- [ ] Dialogue action added.
- [ ] Test campaign includes skill trainer fixture.
- [ ] Unit tests cover training success/failure cases.

#### 6.6 Success Criteria

- Skill training is atomic: failed training does not deduct gold or change ranks.
- Successful training increases only persistent character skill rank.
- Auto skill scaling still applies independently of trained ranks.
- Dialogue can open skill training only for valid skill trainer NPCs.

---

### Phase 7: NPC Train Skills UI

Create the player-facing skill training interface. This phase should mirror
`training_ui.rs` while remaining separate from level-up training.

#### 7.1 Feature Work

Create `src/game/systems/skill_training_ui.rs`.

Core elements:

| UI Element | Purpose |
| --- | --- |
| Header | NPC name and leave hint |
| Party gold display | Shows current shared gold |
| Eligible member list | Select character |
| Skill list | Select trainable skill |
| Effective rank preview | Current rank → next rank |
| Fee display | Cost for selected character/skill |
| Train button | Sends training event |
| Leave button | Returns to exploration |
| Status message | Shows success/failure |

Events/messages:

| Event | Purpose |
| --- | --- |
| `TrainSkill` | Request training for party index + skill ID |
| `SelectSkillTrainingMember` | Select party member |
| `SelectSkillTrainingSkill` | Select skill |
| `ExitSkillTraining` | Leave mode |

Navigation:

| Input | Behavior |
| --- | --- |
| `Esc` | Leave skill training |
| Arrow keys | Move within active list |
| `Tab` | Switch member/skill focus |
| `Enter` | Train selected combination when valid |

#### 7.2 Integrate Feature

Required integration points:

| File | Change |
| --- | --- |
| Game systems plugin registration | Add `SkillTrainingPlugin` |
| Cleanup systems | Despawn skill training UI on mode exit |
| Input gating | Block conflicting global toggles in `GameMode::SkillTraining` |
| HUD portrait click predicate | Block portrait click while skill training |
| Save/load if mode is persisted | Ensure modal mode closes safely |

Layout requirements:

| Requirement |
| --- |
| Use egui `allocate_ui` with explicit column rects for multi-column layout |
| Scroll areas inside columns use `.auto_shrink([true, false])` |
| Navigation hints appear in title/header row |
| No separate bottom hint bar |
| Every loop row uses `push_id` |
| Every scroll area has unique `id_salt` |

#### 7.3 Configuration Updates

No new data beyond Phase 6 fixtures.

#### 7.4 Testing Requirements

Required tests:

| Test Name | Assertion |
| --- | --- |
| `test_skill_training_state_default_selection` | Initial state valid |
| `test_skill_training_eligible_members_filters_dead_members` | Dead members excluded |
| `test_skill_training_available_skills_filters_unoffered_skills` | NPC list enforced |
| `test_skill_training_input_escape_exits_mode` | Escape works |
| `test_skill_training_action_success_updates_status` | Success status appears |
| `test_skill_training_action_failure_updates_status` | Failure status appears |
| `test_portrait_click_ignored_in_skill_training` | HUD click blocked |
| `test_global_toggles_ignored_in_skill_training` | Mode conflicts blocked |

#### 7.5 Deliverables

- [ ] `skill_training_ui.rs` created.
- [ ] `SkillTrainingPlugin` registered.
- [ ] Skill training input, selection, action, cleanup systems implemented.
- [ ] UI follows egui multi-column layout rule.
- [ ] Tests cover mode gating and actions.

#### 7.6 Success Criteria

- Player can select a party member and skill, pay gold, and train.
- UI clearly shows rank changes before training.
- Invalid selections disable training or show recoverable errors.
- Leaving skill training returns to exploration.

---

### Phase 8: SDK Skill Trainer Authoring

Add Campaign Builder support for NPC skill trainers and skill-training dialogue.

#### 8.1 Feature Work

Update NPC editor.

Required UI fields:

| Field | Widget |
| --- | --- |
| `is_skill_trainer` | Checkbox |
| `trainable_skill_ids` | Autocomplete multi-selector |
| `skill_training_fee_base` | Optional DragValue |
| `skill_training_fee_multiplier` | Optional DragValue |
| `skill_training_max_rank` | Optional DragValue |
| Create skill-training dialogue branch | Button similar to existing trainer/merchant helpers |

Use an autocomplete selector for `SkillId` references. Do not use a bare
text field for skill IDs.

#### 8.2 Integrate Feature

Required integration points:

| File | Change |
| --- | --- |
| `sdk/campaign_builder/src/npc_editor/mod.rs` | Add skill trainer controls |
| SDK dialogue helper area | Add `OpenSkillTraining` template branch |
| SDK validation | Validate NPC skill trainer references |
| Campaign metadata/editor state | Ensure `skills.ron` loaded before validation |
| Asset manager | Track skills file |

Validation rules:

| Rule | Severity |
| --- | --- |
| `is_skill_trainer` true and no `trainable_skill_ids` | Warning or error; choose error for explicitness |
| Unknown skill ID in `trainable_skill_ids` | Error |
| Skill ID references non-trainable skill | Warning or error; choose error |
| Negative or zero fee multiplier if present | Error |
| Skill training max rank above skill `max_rank` | Error |
| Dialogue action references NPC that is not a skill trainer | Error |

#### 8.3 Configuration Updates

Update SDK templates and sample campaign data as needed.

Do not create JSON/YAML game data. Use RON only.

#### 8.4 Testing Requirements

Required tests:

| Test Name | Assertion |
| --- | --- |
| `test_npc_skill_trainer_validation_rejects_unknown_skill` | Unknown skill error |
| `test_npc_skill_trainer_validation_rejects_non_trainable_skill` | Non-trainable skill error |
| `test_npc_skill_trainer_dialogue_template_creates_open_skill_training_action` | Template action correct |
| `test_npc_editor_skill_trainer_defaults_are_safe` | Default NPC not trainer |
| `test_skill_id_autocomplete_extracts_candidates` | Skill selector candidate extraction works |

#### 8.5 Deliverables

- [ ] NPC editor supports skill trainer fields.
- [ ] Skill ID autocomplete selector implemented.
- [ ] Skill-training dialogue template implemented.
- [ ] Validation covers NPC and dialogue references.
- [ ] Tests cover SDK authoring behavior.

#### 8.6 Success Criteria

- Campaign authors can create skill trainer NPCs without hand-editing RON.
- SDK validation catches unknown or invalid skill trainer data.
- Generated dialogue can open skill training mode in-game.

---

### Phase 9: Balance, Documentation, and Migration

Finalize skill system rollout with balanced content, migration notes, and
architecture documentation updates.

#### 9.1 Feature Work

Update documentation:

| File | Required Update |
| --- | --- |
| `docs/reference/architecture.md` | Add skills to core data structures and campaign data |
| `docs/reference/campaign_content_format.md` if present | Document `skills.ron` format |
| `docs/explanation/implementations.md` | Summarize implementation when completed |
| SDK user docs if present | Explain Skills Editor and NPC Skill Trainers |

Add migration guidance:

| Old Concept | New Guidance |
| --- | --- |
| Item use permission | Keep using proficiencies |
| Level-scaled capability | Use skills |
| Race/class expertise | Use `skill_grants` |
| Paid level-up | Use existing NPC trainer |
| Paid skill improvement | Use NPC skill trainer |

#### 9.2 Integrate Feature

Balance initial content.

Recommended baseline:

| Class | Strong Skills |
| --- | --- |
| Knight | athletics, leadership |
| Paladin | diplomacy, divine_lore, athletics |
| Archer | perception, tracking |
| Cleric | divine_lore, diplomacy |
| Sorcerer | arcane_lore, item_lore |
| Robber | disarm_traps, stealth, perception |

Recommended rank scale:

| Rank Range | Meaning |
| --- | --- |
| 0 | Untrained |
| 1-5 | Novice |
| 6-15 | Skilled |
| 16-30 | Expert |
| 31-50 | Master |
| 51+ | Legendary or campaign-specific |

#### 9.3 Configuration Updates

Ensure all relevant content files exist in:

| Location | Requirement |
| --- | --- |
| `data/` | Base fixtures load |
| `data/test_campaign/data/` | Self-contained test fixtures |
| `campaigns/tutorial/data/` | Live campaign content |

#### 9.4 Testing Requirements

Regression and integration tests:

| Test Name | Assertion |
| --- | --- |
| `test_test_campaign_loads_with_skills` | Full test campaign loads |
| `test_campaign_validation_includes_skill_references` | Validation covers references |
| `test_existing_proficiency_item_usage_unchanged` | Skill system does not alter item permissions |
| `test_existing_level_training_flow_unchanged` | Skill training does not break level training |
| `test_tutorial_campaign_skill_data_validates` | Live campaign skill data validates, if live campaign tests exist |

#### 9.5 Deliverables

- [ ] Architecture docs updated.
- [ ] Content format docs updated.
- [ ] Base and tutorial skill data balanced.
- [ ] Migration notes written.
- [ ] Final regression tests pass.

#### 9.6 Success Criteria

- Skills are documented as distinct from proficiencies.
- Campaign authors can understand when to use skills vs proficiencies.
- Full campaign validation includes skill data.
- Existing item restrictions and level-up training still behave as before.

---

## Recommended Implementation Order

| Order | Phase | Reason |
| --- | --- | --- |
| 1 | Phase 1: Domain Foundation | Establishes stable data model and loader |
| 2 | Phase 2: Auto Skills | Delivers level scaling with minimal UI risk |
| 3 | Phase 3: Engine Skill Checks | Makes skills useful in gameplay |
| 4 | Phase 4: Auto Skill UI | Makes the system visible to players |
| 5 | Phase 5: SDK Skills Editor | Lets authors manage skill data safely |
| 6 | Phase 6: NPC Train Skills Domain | Adds paid training after auto skills are proven |
| 7 | Phase 7: NPC Train Skills UI | Adds player-facing training workflow |
| 8 | Phase 8: SDK Skill Trainer Authoring | Lets authors create trainer NPCs and dialogue |
| 9 | Phase 9: Balance and Docs | Finalizes rollout and migration guidance |

---

## Cross-Phase Architecture Rules

1. Do not modify or repurpose `ProficiencyDefinition` for numeric skills.
2. Use RON for every new game data file.
3. Add SPDX copyright and license headers to every new Rust source file.
4. Use `SkillId` instead of raw `String` in public skill APIs.
5. Keep derived auto skill ranks computed on demand.
6. Store only persistent trained/manual ranks on `Character`.
7. Do not hardcode test paths under `campaigns/tutorial`.
8. Use `data/test_campaign` for all test campaign fixtures.
9. Use recoverable `Result<T, E>` errors for missing skill/class/race/NPC data.
10. Follow SDK egui ID rules for every Campaign Builder UI change.
11. Keep NPC skill training separate from NPC level training.
12. Update `docs/explanation/implementations.md` after each implemented phase.

## Quality Gates For Each Implementation Phase

Run these commands after each implementation phase:

1. `cargo fmt --all`
2. `cargo check --all-targets --all-features`
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `cargo nextest run --all-features`

Each phase is complete only when all four gates pass with zero errors and zero
warnings.
