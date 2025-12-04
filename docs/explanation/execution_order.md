<!-- SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Implementation Plan Execution Order

This document defines the execution order for the data-driven architecture
migration plans. Following this order minimizes rework, avoids conversion
layers, and builds each system on a clean foundation.

## Overview

Four implementation plans must be executed in a specific sequence:

1. **Hard-coded Removal Plan** (Phases 1-4)
2. **Character Definition Plan** (All Phases)
3. **Hard-coded Removal Plan** (Phases 5-7)
4. **Proficiency Migration Plan** (All Phases)

## Execution Sequence

```
┌─────────────────────────────────────────────────────────────┐
│  HARD-CODED REMOVAL PLAN - Phases 1-4                       │
│                                                             │
│  Phase 1: Add class_id/race_id to Character (keep enums)    │
│  Phase 2: Migrate class logic to ClassDatabase lookups      │
│  Phase 3: Make Disablement system dynamic                   │
│  Phase 4: Complete Race System (merges race plan)           │
│           - Creates RaceDefinition with proficiency fields  │
│           - SDK Races Editor                                │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  CHARACTER DEFINITION PLAN - All Phases                     │
│                                                             │
│  Phase 1-6: CharacterDefinition, characters.ron, SDK editor │
│  - Uses clean class_id/race_id infrastructure               │
│  - Instantiation uses RaceDatabase for modifiers            │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  HARD-CODED REMOVAL PLAN - Phases 5-7                       │
│                                                             │
│  Phase 5: Remove Class and Race enums entirely              │
│  Phase 6: Update SDK editors to be fully dynamic            │
│  Phase 7: Documentation and cleanup                         │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  PROFICIENCY MIGRATION PLAN - All Phases                    │
│                                                             │
│  Phase 1: Create proficiency types and data                 │
│  Phase 2: Populate class/race proficiencies                 │
│  Phase 3: Update items with classification                  │
│  Phase 4-6: Replace Disablement with Proficiency system     │
│                                                             │
│  NOTE: Race/Class definitions already have proficiency      │
│        fields from Hard-coded Removal Phase 4               │
└─────────────────────────────────────────────────────────────┘
```

## Rationale

### Why Hard-coded Removal First (Phases 1-4)?

The `Character` struct is the central data structure. Migrating from enum-based
fields (`race: Race`, `class: Class`) to ID-based fields (`race_id: RaceId`,
`class_id: ClassId`) is a foundational change that affects all other systems.

**Benefits of doing this first:**

- Creates clean infrastructure for Character Definition Plan
- Completes the Race System (merged into Phase 4)
- Adds forward-compatible proficiency fields to RaceDefinition
- No conversion layers needed in subsequent plans

### Why Character Definition Second?

The Character Definition Plan creates `CharacterDefinition` templates that
reference races and classes by ID. Executing this after Hard-coded Phases 1-4
means:

- `CharacterDefinition.class_id` maps directly to `Character.class_id`
- `CharacterDefinition.race_id` maps directly to `Character.race_id`
- Race modifiers can be applied during instantiation using `RaceDatabase`
- No enum-to-ID conversion code required

### Why Hard-coded Removal Phases 5-7 Third?

With the Character Definition system complete, it's safe to remove the legacy
Class and Race enums entirely:

- All new code uses ID-based references
- Character creation uses `CharacterDefinition.instantiate()`
- SDK editors work with databases, not enums

### Why Proficiency Migration Last?

The Proficiency system replaces the Disablement bitmask for item restrictions.
Executing this last provides:

- **Stability**: Disablement remains functional (made dynamic in Phase 3) until
  replacement is ready
- **Forward compatibility**: Phase 4 adds `proficiencies` and
  `incompatible_item_tags` fields to `RaceDefinition` with `#[serde(default)]`,
  avoiding rework
- **Clean foundation**: All class/race infrastructure is complete before adding
  proficiency logic
- **Lower priority**: Proficiency is a UX improvement; other plans address
  architectural debt

## Plan Documents

| Plan                  | Document                                      | Status                                                  |
| --------------------- | --------------------------------------------- | ------------------------------------------------------- |
| Hard-coded Removal    | `hardcoded_removal_implementation_plan.md`    | Active                                                  |
| Character Definition  | `character_definition_implementation_plan.md` | Active                                                  |
| Proficiency Migration | `proficiency_migration_plan.md`               | Active                                                  |
| Race System           | `race_system_implementation_plan.md`          | **Superseded** (merged into Hard-coded Removal Phase 4) |

## Phase Summary

### Hard-coded Removal Plan

| Phase | Description                | Key Deliverables                                      |
| ----- | -------------------------- | ----------------------------------------------------- |
| 1     | Character Struct Migration | `class_id`/`race_id` fields added                     |
| 2     | Class Logic Migration      | `progression.rs`, `casting.rs` use `ClassDatabase`    |
| 3     | Disablement Migration      | Dynamic `can_use_class_id()` methods                  |
| 4     | Race System                | Complete `RaceDefinition`, `RaceDatabase`, SDK editor |
| 5     | Enum Removal               | Remove `Class` and `Race` enums                       |
| 6     | SDK Updates                | Dynamic editors using databases                       |
| 7     | Documentation              | Architecture docs, how-to guides                      |

### Character Definition Plan

| Phase | Description     | Key Deliverables                           |
| ----- | --------------- | ------------------------------------------ |
| 1     | Domain Types    | `CharacterDefinition`, `CharacterDatabase` |
| 2     | Data Files      | `characters.ron` for core and campaigns    |
| 3     | SDK Integration | `ContentDatabase` updates, validation      |
| 4     | SDK Editor      | `characters_editor.rs`                     |
| 5     | Instantiation   | `CharacterDefinition::instantiate()`       |
| 6     | Documentation   | Architecture docs, how-to guides           |

### Proficiency Migration Plan

| Phase | Description          | Key Deliverables                              |
| ----- | -------------------- | --------------------------------------------- |
| 1     | Core Types           | `ProficiencyDefinition`, classification enums |
| 2     | Class/Race Migration | Add `proficiencies` data to RON files         |
| 3     | Item Migration       | Add `classification`, `tags` to items         |
| 4     | SDK Editors          | Update editors for proficiency UI             |
| 5     | CLI Editors          | Update CLI tools                              |
| 6     | Cleanup              | Remove deprecated Disablement system          |

## Dependencies Diagram

```
                    ┌──────────────────┐
                    │  ClassDatabase   │
                    │  (exists)        │
                    └────────┬─────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
┌─────────────────┐  ┌──────────────────┐  ┌─────────────────┐
│ Character       │  │ RaceDatabase     │  │ Disablement     │
│ class_id field  │  │ (Phase 4)        │  │ dynamic methods │
│ (Phase 1)       │  │                  │  │ (Phase 3)       │
└────────┬────────┘  └────────┬─────────┘  └────────┬────────┘
         │                    │                     │
         └────────────────────┼─────────────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │ CharacterDef     │
                    │ (Char Def Plan)  │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │ Enum Removal     │
                    │ (Phase 5-7)      │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │ Proficiency      │
                    │ System           │
                    └──────────────────┘
```

## Checkpoints

Use these checkpoints to verify progress before moving to the next stage:

### Checkpoint 1: After Hard-coded Phases 1-4

- [ ] `Character` struct has `class_id` and `race_id` fields
- [ ] `roll_hp_gain()` uses `ClassDatabase` lookup
- [ ] `calculate_spell_points()` uses `ClassDefinition.spell_stat`
- [ ] `Disablement::can_use_class_id()` works with dynamic bit lookup
- [ ] `RaceDefinition` and `RaceDatabase` are complete in domain
- [ ] `races.ron` has full race data (modifiers, resistances, abilities)
- [ ] SDK Races Editor exists and functions
- [ ] All tests pass

### Checkpoint 2: After Character Definition Plan

- [ ] `CharacterDefinition` and `CharacterDatabase` exist
- [ ] `characters.ron` exists in `data/` and `campaigns/tutorial/data/`
- [ ] SDK Character Editor exists and functions
- [ ] `CharacterDefinition::instantiate()` creates valid `Character`
- [ ] Race modifiers apply during instantiation
- [ ] All tests pass

### Checkpoint 3: After Hard-coded Phases 5-7

- [ ] `Class` enum removed from `src/domain/character.rs`
- [ ] `Race` enum removed from `src/domain/character.rs`
- [ ] All code uses `class_id`/`race_id` string references
- [ ] SDK editors populate dropdowns from databases
- [ ] Documentation updated
- [ ] All tests pass

### Checkpoint 4: After Proficiency Migration

- [ ] `ProficiencyDefinition` and `ProficiencyDatabase` exist
- [ ] Classes have `proficiencies` populated in RON
- [ ] Races have `proficiencies` and `incompatible_item_tags` populated
- [ ] Items have `classification` and `tags` fields
- [ ] `Disablement` system removed
- [ ] All item restriction checks use proficiency logic
- [ ] All tests pass

## Risk Mitigation

1. **Incremental commits**: Each phase has deliverables; commit at boundaries
2. **Dual support during transition**: Phases 1-4 maintain enum compatibility
3. **Test coverage**: Extensive tests at each phase catch regressions
4. **Rollback capability**: Git history enables reverting to any checkpoint
5. **Forward compatibility**: Proficiency fields added early with defaults
