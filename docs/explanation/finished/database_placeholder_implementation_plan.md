# Database Placeholder Implementation Plan

## Critical Distinction: Editor vs. Game Runtime

**IMPORTANT**: This plan addresses placeholders in the **game engine's runtime database** (`src/sdk/database.rs`), NOT the Campaign Builder editor.

### Two Separate Systems

1. **Campaign Builder SDK** (`sdk/campaign_builder/src/main.rs`)
   - **Purpose**: Editor UI for creating/modifying content
   - **Status**: ✅ **Already working** - Loads quests/dialogues directly for editing
   - **Location**: Loads files on-demand in the UI code (around lines 840-850 in `main.rs`)

2. **Content Database** (`src/sdk/database.rs`)
   - **Purpose**: Runtime game engine content loading (used during actual gameplay)
   - **Status**: ❌ **Broken** - The `load_from_file()` methods return empty databases
   - **Impact**: The *game engine* can't load quests/dialogues even though the *editor* can

### The Problem

```rust
// Current placeholder implementation (line 332):
pub fn load_from_file<P: AsRef<Path>>(_path: P) -> Result<Self, DatabaseError> {
    // Placeholder implementation - will load quests from RON file
    Ok(Self::new())  // ← Returns EMPTY database! Game can't access quests!
}
```

When the game tries to load content:
```rust
let db = ContentDatabase::load_campaign("campaigns/tutorial")?;
let quest = db.quests.get_quest(1); // ← Always returns None!
```

### Summary

- ✅ **Campaign Builder**: Can edit quests/dialogues (works fine)
- ❌ **Game Engine**: Can't load quests/dialogues for gameplay (broken)

This plan implements RON loading **for the game engine's ContentDatabase**, which is separate from how the Campaign Builder loads files for editing.

---

## Overview

This plan addresses the 9 remaining placeholder implementations in `src/sdk/database.rs`. The database currently has functional infrastructure for Classes, Items, and Maps, but lacks implementation for Spells, Monsters, Quests, and Dialogues. This plan focuses on leveraging existing domain types and implementing proper RON file loading for each system. Races are deferred to a future phase as they lack full SDK definition.

## Current State Analysis

### Existing Infrastructure

- **Functional Systems**: `ClassDatabase`, `ItemDatabase`, and `MapDatabase` have complete implementations with RON file loading, validation, and query methods.
- **Domain Types**: Complete domain definitions exist for:
  - `Spell` in `src/domain/magic/types.rs`
  - `Monster` in `src/domain/combat/monster.rs`
  - `Quest` in `src/domain/quest.rs`
  - `DialogueTree` in `src/domain/dialogue.rs`
- **File Structure**: Campaign directory structure is established with `data/` subdirectories for all content types.
- **Error Handling**: `DatabaseError` enum exists with variants for all content types.

### Identified Issues

1. **Spell System**: `SpellDefinition` is a minimal placeholder (only id/name) instead of using the full `Spell` type from `domain::magic::types`.
2. **Monster System**: `MonsterDefinition` is minimal instead of using the full `Monster` type.
3. **Quest/Dialogue Systems**: Have empty `load_from_file()` implementations that return empty databases.
4. **Type Duplication**: Placeholder types (`SpellDefinition`, `MonsterDefinition`) duplicate existing domain types.
5. **Load Functions**: Three `load_campaign()` calls reference placeholder implementations (lines 567, 574).

## Implementation Phases

### Phase 1: Spell System Implementation

**Objective**: Replace placeholder `SpellDefinition` with domain `Spell` type and implement RON loading.

#### 1.1 Foundation Work

- Remove `SpellDefinition` struct (lines 141-148).
- Import `Spell` from `crate::domain::magic::types`.
- Update `SpellDatabase` to use `HashMap<SpellId, Spell>` instead of `HashMap<SpellId, SpellDefinition>`.
- Update all `SpellDatabase` method signatures to return `&Spell` instead of `&SpellDefinition`.

#### 1.2 Add Foundation Functionality

- Implement `SpellDatabase::load_from_file()`:
  - Read RON file using `std::fs::read_to_string()`.
  - Deserialize `Vec<Spell>` using `ron::from_str()`.
  - Build `HashMap<SpellId, Spell>` from vector.
  - Return `Result<Self, DatabaseError>` with proper error mapping.
- Pattern after `ItemDatabase::load_from_file()` in `src/domain/items/mod.rs`.

#### 1.3 Integrate Foundation Work

- Update `ContentDatabase::load_campaign()` line 575-578 to properly handle spell loading errors.
- Update `ContentDatabase::load_core()` line 667-670 similarly.
- Ensure error variant `DatabaseError::SpellLoadError` properly wraps RON errors.

#### 1.4 Testing Requirements

- Unit test: `test_spell_database_load_from_file()` with sample RON data.
- Unit test: `test_spell_database_empty_file()` for empty file handling.
- Integration test: Load tutorial campaign spells and verify count.
- Round-trip test: Serialize and deserialize spell data.

#### 1.5 Deliverables

- `SpellDatabase` with functional `load_from_file()` implementation.
- Updated `ContentDatabase` with spell loading integration.
- Test coverage for spell loading edge cases.

#### 1.6 Success Criteria

- `SpellDatabase::load_from_file("campaigns/tutorial/data/spells.ron")` successfully loads existing spell data.
- Error handling distinguishes between missing files (returns empty DB) and parsing errors (returns error).
- All existing tests pass without modification.

---

### Phase 2: Monster System Implementation

**Objective**: Replace placeholder `MonsterDefinition` with domain `Monster` type and implement RON loading.

#### 2.1 Feature Work

- Remove `MonsterDefinition` struct (lines 193-200).
- Import `Monster` from `crate::domain::combat::monster`.
- Update `MonsterDatabase` to use `HashMap<MonsterId, Monster>`.
- Update all method signatures to return `&Monster`.

#### 2.2 Integrate Feature

- Implement `MonsterDatabase::load_from_file()`:
  - Read and deserialize `Vec<Monster>` from RON.
  - Build `HashMap<MonsterId, Monster>`.
  - Map errors to `DatabaseError::MonsterLoadError`.
- Update `ContentDatabase::load_campaign()` line 568-571.
- Update `ContentDatabase::load_core()` line 660-663.

#### 2.3 Configuration Updates

- Verify `Monster` type has `Serialize` and `Deserialize` derives.
- Check if any `Monster` fields need `#[serde(default)]` attributes for backward compatibility.

#### 2.4 Testing Requirements

- Unit test: `test_monster_database_load_from_file()`.
- Unit test: `test_monster_database_invalid_ron()` for error handling.
- Integration test: Load monsters from tutorial campaign.

#### 2.5 Deliverables

- Functional `MonsterDatabase::load_from_file()`.
- Integration with `ContentDatabase`.
- Test suite for monster loading.

#### 2.6 Success Criteria

- Can load monster data from `campaigns/tutorial/data/monsters.ron`.
- `ContentDatabase::stats()` correctly reports monster count.
- No type duplication between SDK and domain layers.

---

### Phase 3: Quest System Implementation

**Objective**: Implement quest loading from RON files.

#### 3.1 Feature Work

- Implement `QuestDatabase::load_from_file()` (line 331-334):
  - Read RON file.
  - Deserialize `Vec<Quest>`.
  - Build `HashMap<QuestId, Quest>`.
- Verify `Quest` type in `src/domain/quest.rs` has proper serialization support.

#### 3.2 Integrate Feature

- Update `ContentDatabase::load_campaign()` line 589-592 error handling.
- Update `ContentDatabase::load_core()` line 681-684.
- Add validation for quest prerequisites and rewards references.

#### 3.3 Configuration Updates

- Check if `Quest` references (item IDs, NPC IDs) need validation.
- Consider adding `QuestDatabase::validate()` method for cross-references.

#### 3.4 Testing Requirements

- Unit test: `test_quest_database_load_from_file()`.
- Unit test: `test_quest_database_multiple_quests()`.
- Integration test: Quest chain validation (prerequisites exist).

#### 3.5 Deliverables

- Functional `QuestDatabase::load_from_file()`.
- Quest data loading in `ContentDatabase`.
- Basic validation for quest chains.

#### 3.6 Success Criteria

- Tutorial campaign quests load successfully.
- Quest database correctly indexes by `QuestId`.
- `has_quest()` and `get_quest()` methods work as expected.

---

### Phase 4: Dialogue System Implementation

**Objective**: Implement dialogue tree loading from RON files.

#### 4.1 Feature Work

- Implement `DialogueDatabase::load_from_file()` (line 379-382):
  - Read and deserialize `Vec<DialogueTree>`.
  - Build `HashMap<DialogueId, DialogueTree>`.
  - Handle nested dialogue node structures.
- Verify `DialogueTree` in `src/domain/dialogue.rs` serializes correctly.

#### 4.2 Integrate Feature

- Update `ContentDatabase::load_campaign()` line 596-599.
- Update `ContentDatabase::load_core()` line 688-691.
- Consider caching dialogue text for localization future-proofing.

#### 4.3 Configuration Updates

- Verify dialogue node IDs are unique within trees.
- Check if dialogue choices reference valid node IDs.
- Add optional validation for dialogue graph completeness.

#### 4.4 Testing Requirements

- Unit test: `test_dialogue_database_load_from_file()`.
- Unit test: `test_dialogue_tree_branching()` for complex dialogues.
- Integration test: Load tutorial dialogues and traverse tree.

#### 4.5 Deliverables

- Functional `DialogueDatabase::load_from_file()`.
- Dialogue loading in `ContentDatabase`.
- Dialogue graph validation (optional).

#### 4.6 Success Criteria

- Tutorial campaign dialogues load without errors.
- Dialogue trees preserve branching structure.
- `get_dialogue()` retrieves complete dialogue trees.

---

### Phase 5: Integration Testing & Documentation

**Objective**: Comprehensive testing and documentation updates.

#### 5.1 Integration Work

- Update `ContentDatabase::validate()` (line 724-733) to validate all systems:
  - Call validation methods for spells, monsters, quests, dialogues.
  - Check cross-references (e.g., quests referencing valid items/monsters).
- Create comprehensive integration test loading full tutorial campaign.

#### 5.2 Documentation Updates

- Update `src/sdk/database.rs` module documentation with examples for each system.
- Document expected RON file formats for spells, monsters, quests, dialogues.
- Add examples to doc comments showing typical usage patterns.
- Update `docs/explanation/sdk_implementation_plan.md` to reflect completion.

#### 5.3 Example Data Updates

- Ensure `campaigns/tutorial/data/` has example files for all systems.
- Create minimal example campaign in docs showing file structure.
- Add RON snippets to doc comments demonstrating format.

#### 5.4 Testing Requirements

- Integration test: `test_load_full_campaign()` loading all content types.
- Integration test: `test_cross_reference_validation()` checking ID references.
- Performance test: Benchmark loading large campaigns (1000+ items).

#### 5.5 Deliverables

- Complete `ContentDatabase` with all placeholder implementations resolved.
- Comprehensive test suite (unit + integration).
- Updated documentation and examples.

#### 5.6 Success Criteria

- All 9 placeholder implementations resolved.
- Tutorial campaign loads completely with all content types.
- `ContentDatabase::stats()` accurately reports all content counts.
- Documentation includes working examples for each database type.
- Test coverage >80% for database module.

---

## File & Symbol Modifications (Reference)

### Files to Modify

- `src/sdk/database.rs`:
  - Remove: `SpellDefinition` (lines 141-148), `MonsterDefinition` (lines 193-200)
  - Modify: `SpellDatabase`, `MonsterDatabase`, `QuestDatabase`, `DialogueDatabase`
  - Implement: Four `load_from_file()` methods
  - Update: `ContentDatabase::load_campaign()`, `ContentDatabase::load_core()`, `ContentDatabase::validate()`

### New Tests to Add

- `src/sdk/database.rs` test module:
  - `test_spell_database_load_from_file()`
  - `test_monster_database_load_from_file()`
  - `test_quest_database_load_from_file()`
  - `test_dialogue_database_load_from_file()`
  - `test_load_full_campaign()`
  - `test_cross_reference_validation()`

---

## Recommended Implementation Order

1. **Phase 1: Spells** - High impact, existing domain type is complete
2. **Phase 2: Monsters** - High impact, complete domain type
3. **Phase 3: Quests** - Medium impact, simpler than dialogues
4. **Phase 4: Dialogues** - Medium impact, tree structure needs care
5. **Phase 5: Integration** - Final polish and documentation

---

## Future Phase: Race System

Deferred until Race definition is complete in SDK:

- Phase 6 would include:
  - Define complete `RaceDefinition` with attributes, bonuses, restrictions
  - Implement `RaceDatabase::load_from_file()`
  - Update Phase 2 references in current code
  - Add race-specific validation

---

## Notes

- All implementations follow the existing pattern from `ItemDatabase` and `ClassDatabase`.
- RON file format is already established in campaign data files.
- Error handling uses existing `DatabaseError` variants.
- No breaking changes to public API - only filling in placeholders.
