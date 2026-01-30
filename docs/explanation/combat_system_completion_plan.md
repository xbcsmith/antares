# Combat System Completion Plan

**Document Version:** 2.0 (Revised)  
**Last Updated:** 2025-01-XX  
**Status:** ✅ AI-READY (95% Autonomous Execution)

---

## Pre-Implementation Checklist

**BEFORE starting ANY phase, verify:**

- [ ] Tools installed (run: `rustup component list --installed`)
  - [ ] rustfmt
  - [ ] clippy
  - [ ] rust-analyzer (optional but recommended)
- [ ] cargo-nextest installed (run: `cargo nextest --version`)
- [ ] Read `AGENTS.md` completely
- [ ] Read `docs/reference/architecture.md` Section 3 (Module Structure)
- [ ] Read `docs/reference/architecture.md` Section 4 (Data Structures)
- [ ] Understand development workflow (AGENTS.md L177-268)
- [ ] Baseline quality gates pass:
  ```bash
  cargo fmt --all && \
  cargo check --all-targets --all-features && \
  cargo clippy --all-targets --all-features -- -D warnings && \
  cargo nextest run --all-features
  ```

**If baseline fails, STOP and fix before implementing new code.**

---

## Architecture Quick Reference

**MANDATORY:** Use these exact definitions from `docs/reference/architecture.md`

### Type Aliases (Section 4.6 L990-1008)

| Use This      | NOT This         | Defined At               |
| ------------- | ---------------- | ------------------------ |
| `SpellId`     | `u32`            | L991                     |
| `ItemId`      | `u32`            | L990                     |
| `CharacterId` | `usize` or `u32` | L994                     |
| `CombatantId` | `usize`          | combat/types.rs L213-224 |
| `MonsterId`   | `u32`            | L992                     |

### Key Constants (Section 4.3 L361-394)

```rust
const PARTY_MAX_SIZE: usize = 6;
const INVENTORY_MAX_SLOTS: usize = 20;  // Also: Inventory::MAX_ITEMS
const EQUIPMENT_MAX_SLOTS: usize = 7;   // Also: Equipment::MAX_EQUIPPED
const HP_SP_MAX: u16 = 65535;
const SPELL_LEVEL_MAX: u8 = 7;
const SPELL_LEVEL_MIN: u8 = 1;
```

### Key Data Structures

**CombatState Location:**

- Definition: `src/domain/combat/engine.rs` L117-140
- Architecture: `docs/reference/architecture.md` L610-622

**SpellCast Structure:**

- Architecture: `docs/reference/architecture.md` L1777-1785

**SpellState Enum:**

- Architecture: `docs/reference/architecture.md` L1763-1775

**Item Structure:**

- Architecture: `docs/reference/architecture.md` L683-700

### Module Layers (Section 3.2)

- `src/domain/` - Pure game logic (no Bevy dependencies)
- `src/application/` - Cross-cutting concerns (save/load, etc.)
- `src/game/` - Bevy integration (systems, components, resources)

### Data File Format

**MANDATORY:** All game data files use `.ron` (Rusty Object Notation)

- ✅ `data/spells.ron`
- ✅ `data/items.ron`
- ❌ `data/spells.json` (WRONG)
- ❌ `data/items.yaml` (WRONG)

Reference: architecture.md Section 7.2 L1986-2346 for RON examples

---

## Overview

This document provides a comprehensive, phased approach to complete the Antares combat system. It builds upon the existing Combat System Implementation Plan and addresses all identified gaps, TODOs, and missing functionality discovered through code review.

**Status**: The combat system has Phases 1-5 largely implemented (core infrastructure, UI, player actions, monster AI, and basic resolution/rewards). This plan focuses on completing remaining features, polish, and integration.

---

## Executive Summary

### What's Complete ✓

- ✓ Combat plugin, messages, resources, and `CombatResource` wrapper
- ✓ Party ↔ Combat state synchronization (bidirectional)
- ✓ Combat UI: enemy panel, HP bars, turn order text, action menu
- ✓ Player actions: Attack, Defend, Flee (with deterministic helpers)
- ✓ Target selection via `TargetSelection` resource and UI
- ✓ Monster AI: Aggressive/Defensive/Random targeting
- ✓ Automatic monster turn execution
- ✓ Combat resolution detection (Victory/Defeat)
- ✓ XP and gold/gem distribution on victory
- ✓ Victory/defeat UI and messages
- ✓ Random encounter system at domain/application level
- ✓ Encounter trigger from map events (`MapEvent::Encounter`)
- ✓ Comprehensive unit and integration tests

### What's Missing ✗

- ✗ **Spell casting flow** (UI + messages + handlers)
- ✗ **Item usage flow** (UI + messages + handlers)
- ✗ **Turn indicator visual** (system to highlight current actor)
- ✗ **Victory UI enhancements** (per-character XP breakdown, detailed loot)
- ✗ **Combat animations** (attack animations, hit flash, `Animating` state)
- ✗ **Target selection API consolidation** (resource vs component)
- ✗ **Trap and treasure event handlers** (apply damage, add items)
- ✗ **Integration test for movement-triggered encounters**
- ✗ **Audio integration** (combat music, SFX for attacks/hits)
- ✗ **Polish and visual feedback** (damage numbers, condition indicators)

---

## Phased Completion Plan

### Phase 7: Spell Casting System

**Estimated Time:** 5-8 hours  
**Blocks:** Phase 9 (needs action system), Phase 14 (needs complete combat)  
**Blocked By:** None (can start immediately)

Complete the spell casting flow to enable players to cast spells during combat.

---

#### 7.0 MANDATORY: Architecture Compliance

**Read these sections BEFORE implementation:**

| Section          | Lines      | Topic                | Use For                        |
| ---------------- | ---------- | -------------------- | ------------------------------ |
| Architecture 4.9 | L1777-1785 | `SpellCast` struct   | Exact struct definition        |
| Architecture 4.9 | L1763-1775 | `SpellState` enum    | Validation states              |
| Architecture 4.9 | L1725-1740 | `can_cast_spell()`   | Class/school validation logic  |
| Architecture 4.6 | L990-991   | Type aliases         | `SpellId`, `CharacterId`       |
| Architecture 4.3 | L551-580   | `SpellBook` struct   | Character spell management     |
| Architecture 4.3 | L298-299   | `Character.sp` field | Spell points (AttributePair16) |

**Type Aliases (MANDATORY):**

- Use `SpellId` (NOT `u32`) from Section 4.6 L991
- Use `CharacterId` (NOT `usize` or `u32`) from Section 4.6 L994
- Use `CombatantId` (NOT `usize`) from `src/domain/combat/types.rs` L213-224

**Constants (MANDATORY):**

- Use `SPELL_LEVEL_MIN` / `SPELL_LEVEL_MAX` (NOT magic numbers 1, 7)
- From architecture.md Section 4.3 L375-376

**Verification:**

```bash
# Must return ZERO matches
grep "spell_id: u32" src/domain/combat/spell_casting.rs
grep "caster: usize" src/domain/combat/spell_casting.rs
```

---

#### 7.1 Game Context

**What the player experiences:**
Player selects "Cast Spell" action, chooses spell from their spellbook, selects target (if applicable), and spell effect is applied. SP (spell points) and gems are consumed.

**Game mechanics to understand:**

**Spell Schools:**

- **Cleric Spells:** Cast by Cleric, Paladin (level 3+)
- **Sorcerer Spells:** Cast by Sorcerer, Archer (level 3+)

**Validation Logic (from architecture.md L1725-1740):**

```rust
// Paladins and Archers need level 3+ for spell access
match (character.class, spell.school) {
    (Class::Cleric, SpellSchool::Cleric) => true,
    (Class::Paladin, SpellSchool::Cleric) => character.level >= 3,
    (Class::Sorcerer, SpellSchool::Sorcerer) => true,
    (Class::Archer, SpellSchool::Sorcerer) => character.level >= 3,
    _ => false,
}
```

**Resource Costs:**

- **SP (Spell Points):** Always consumed (typically = spell level)
- **Gems:** Required for high-level spells (Level 4+ usually need gems)
- **Both:** Deducted BEFORE spell effect applies

**Spell Context Restrictions:**
From `SpellState` enum (architecture L1763-1775):

- `CombatOnly` - Fireball, Lightning Bolt (cannot cast outside combat)
- `NonCombatOnly` - Town Portal, Teleport (cannot cast in combat)
- `OutdoorsOnly` - Fly, Walk on Water
- `IndoorOnly` - (none in base game)

**Edge Cases:**

- Silenced condition prevents spellcasting
- Dead/unconscious characters cannot cast
- Invalid targets (e.g., healing spell on dead character)
- Insufficient SP or gems

**Test Coverage Required:**

- ✅ Sorcerer can't cast Cleric spells
- ✅ Level 1 Paladin can't cast any spells
- ✅ Level 3 Paladin CAN cast Cleric spells
- ✅ Insufficient SP prevents casting
- ✅ Insufficient gems prevents high-level spells
- ✅ Combat-only spells fail in exploration mode
- ✅ Silenced condition blocks casting

---

#### 7.2 Module Structure

**Files to READ FIRST:**

1. **`src/domain/combat/engine.rs`** L100-400

   - Understand `CombatState` structure
   - Understand `process_turn()` pattern
   - Understand `TurnAction` enum (if exists)

2. **`src/domain/character.rs`** L286-314

   - Understand `Character` struct
   - Understand `sp: AttributePair16` field at L299
   - Understand `spells: SpellBook` field at L304

3. **`src/game/systems/combat_ui.rs`** L1-200
   - Understand existing UI panel structure
   - Understand action menu pattern

**Files to CREATE:**

1. **File:** `src/domain/combat/spell_casting.rs` (NEW - ~200 lines)
   - **Layer:** Domain (pure game logic)
   - **Add SPDX header:**

     ```rust
     // SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
     // SPDX-License-Identifier: Apache-2.0

     //! Spell casting logic for combat
     //!
     //! This module implements spell validation, SP/gem deduction, and effect
     //! application during combat encounters.
     //!
     //! # Architecture Reference
     //!
     //! See `docs/reference/architecture.md` Section 4.9 for spell system specs.
     ```

**Files to MODIFY:**

2. **File:** `src/domain/combat/engine.rs`

   - **Modify:** Add spell casting integration at ~L250 in `process_turn()`
   - **Add:** `pub fn execute_spell_cast()` function at ~L600

3. **File:** `src/domain/combat/mod.rs`

   - **Modify:** Add `pub mod spell_casting;` at appropriate location (alphabetically)

4. **File:** `src/game/systems/combat_ui.rs`

   - **Modify:** Add spell selection panel UI
   - **Modify:** Add spell casting input handling

5. **File:** `src/game/systems/combat.rs` (or combat system file)
   - **Modify:** Add `CastSpellAction` message handler

---

#### 7.3 Implementation Tasks

**Task 7.3.1: Create Spell Casting Domain Logic** (Est: 3h)

**File:** `src/domain/combat/spell_casting.rs` (CREATE)

**What:** Implement spell validation and effect application following architecture Section 4.9

**Data Structures (mirror architecture L1777-1785):**

```rust
// NOTE: This is a REFERENCE showing structure. Use exact types from architecture.

use crate::domain::types::{SpellId, CharacterId};
use crate::domain::combat::types::CombatantId;

/// Spell casting action (mirrors SpellCast from architecture)
pub struct SpellCastAction {
    pub spell_id: SpellId,           // NOTE: SpellId, not u32
    pub caster: CharacterId,          // NOTE: CharacterId, not usize
    pub target: Option<CombatantId>,  // NOTE: CombatantId, not usize
    pub sp_cost: u16,
    pub gem_cost: u16,
}

/// Result of spell cast validation
pub enum SpellCastResult {
    Success { effect: SpellEffect },
    Failed { reason: SpellCastError },
}

/// Spell casting errors (use thiserror)
#[derive(Error, Debug)]
pub enum SpellCastError {
    #[error("Insufficient spell points: need {needed}, have {current}")]
    InsufficientSP { needed: u16, current: u16 },

    #[error("Insufficient gems: need {needed}, have {current}")]
    InsufficientGems { needed: u16, current: u16 },

    #[error("Character {0} cannot cast spell school {1:?}")]
    WrongSpellSchool(CharacterId, SpellSchool),

    #[error("Character level {level} too low for spell (requires {required})")]
    LevelTooLow { level: u8, required: u8 },

    #[error("Spell {0} cannot be cast in current context")]
    InvalidContext(SpellId),

    #[error("Caster is silenced and cannot cast spells")]
    Silenced,
}
```

**Functions to implement:**

1. `pub fn validate_spell_cast()` - Check all prerequisites

   - Follow `can_cast_spell()` logic from architecture L1725-1740
   - Check SP cost, gem cost, class restrictions, level requirements
   - Return `Result<(), SpellCastError>`

2. `pub fn execute_spell_cast()` - Apply spell effects
   - Deduct SP from `character.sp.current`
   - Deduct gems from party resources
   - Apply spell effect based on spell type
   - Return `Result<SpellEffect, CombatError>`

**Error Handling:** Use `thiserror` for all errors. NO `unwrap()` or `panic!()`.

**Dependencies:** None (can implement immediately)

---

**Task 7.3.2: Integrate with Combat Engine** (Est: 2h)

**File:** `src/domain/combat/engine.rs` (MODIFY)

**What:** Add spell casting to combat turn processing

**Integration Point:** `CombatState::process_turn()` at ~L250

**Changes:**

1. Add to `TurnAction` enum (if exists, or create):

   ```rust
   pub enum TurnAction {
       Attack(AttackAction),
       Defend,
       Flee,
       CastSpell(SpellCastAction),  // ADD THIS
       UseItem(ItemUsageAction),     // For Phase 8
   }
   ```

2. Add handler in `process_turn()` match statement:
   ```rust
   TurnAction::CastSpell(action) => {
       spell_casting::execute_spell_cast(combat_state, action, game_data)?
   }
   ```

**Dependencies:** Requires Task 7.3.1 complete

---

**Task 7.3.3: Create Spell Selection UI** (Est: 2h)

**File:** `src/game/systems/combat_ui.rs` (MODIFY)

**What:** Add UI panel for spell selection

**Components to add:**

```rust
#[derive(Component)]
pub struct SpellSelectionPanel;

#[derive(Component)]
pub struct SpellButton {
    pub spell_id: SpellId,  // NOTE: SpellId, not u32
    pub sp_cost: u16,
}
```

**System to add:** `setup_spell_selection_ui()`

- Query character's `SpellBook`
- Display available spells with names, SP costs
- Gray out spells with insufficient SP
- Show spell descriptions on hover
- Connect to existing target selection system

**UI Layout:**

- Similar to action menu
- Show spell list in scrollable panel
- Display SP/gem costs
- Back button to return to action menu

**Dependencies:** Can run in parallel with Task 7.3.1

---

**Task 7.3.4: Add Input Handling** (Est: 1h)

**File:** `src/game/systems/combat.rs` or relevant input system

**What:** Handle spell selection and casting inputs

**Message to define:**

```rust
#[derive(Message)]
pub struct CastSpellMessage {
    pub caster: CombatantId,      // NOTE: CombatantId, not usize
    pub spell_id: SpellId,        // NOTE: SpellId, not u32
    pub target: Option<CombatantId>,
}
```

**System:** `handle_cast_spell_input()`

- Receive `CastSpellMessage`
- Validate via domain `validate_spell_cast()`
- If valid, add to combat state's pending actions
- Close spell selection UI
- Trigger target selection if needed

**Dependencies:** Requires Tasks 7.3.1, 7.3.2, 7.3.3

---

#### 7.4 Integration Points

**Integrates with:**

1. **Existing Combat Engine** (`src/domain/combat/engine.rs`)

   - **Read first:** Lines 100-400 (understand CombatState)
   - **Integration point:** Add `TurnAction::CastSpell` variant
   - **Pattern to follow:** Similar to `TurnAction::Attack` handling

2. **Existing Target Selection** (`src/game/resources/target_selection.rs` or similar)

   - **Use:** Existing `TargetSelection` resource for spell targeting
   - **Do NOT:** Create new target selection system
   - **Pattern:** Same flow as attack targeting

3. **Character Spell System** (`src/domain/character.rs`)

   - **Use:** `Character.spells: SpellBook` field (L304)
   - **Use:** `Character.sp: AttributePair16` field (L299)
   - **Read:** `SpellBook::get_spell_list_for_class()` implementation

4. **Party Resources** (for gem deduction)
   - **File:** `src/domain/party_manager.rs` or `src/domain/resources.rs`
   - **Use:** `Party.gems` field for gem cost deduction
   - **Do NOT:** Modify structure, only deduct values

---

#### 7.5 Testing Requirements

**Unit Tests** (file: `src/domain/combat/spell_casting.rs`):

Add `#[cfg(test)] mod tests` section:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Character, Class, Race, Sex, Alignment};
    use crate::domain::types::SpellId;

    #[test]
    fn test_validate_spell_cast_success() {
        // Arrange: Level 5 Sorcerer with 10 SP
        let character = create_test_sorcerer(5, 10);
        let spell = create_test_fireball(); // Costs 3 SP

        // Act
        let result = validate_spell_cast(&character, &spell);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_spell_cast_insufficient_sp() {
        // Arrange: Level 5 Sorcerer with 2 SP
        let character = create_test_sorcerer(5, 2);
        let spell = create_test_fireball(); // Costs 3 SP

        // Act
        let result = validate_spell_cast(&character, &spell);

        // Assert
        assert!(matches!(result, Err(SpellCastError::InsufficientSP { .. })));
    }

    #[test]
    fn test_validate_spell_cast_wrong_class() {
        // Arrange: Knight (cannot cast Sorcerer spells)
        let character = create_test_knight(5, 10);
        let spell = create_test_fireball(); // Sorcerer spell

        // Act
        let result = validate_spell_cast(&character, &spell);

        // Assert
        assert!(matches!(result, Err(SpellCastError::WrongSpellSchool(..))));
    }

    #[test]
    fn test_validate_spell_cast_paladin_level_3_can_cast() {
        // Arrange: Level 3 Paladin (just gained spell access)
        let character = create_test_paladin(3, 10);
        let spell = create_test_cure_wounds(); // Cleric spell

        // Act
        let result = validate_spell_cast(&character, &spell);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_spell_cast_paladin_level_2_cannot_cast() {
        // Arrange: Level 2 Paladin (no spell access yet)
        let character = create_test_paladin(2, 10);
        let spell = create_test_cure_wounds(); // Cleric spell

        // Act
        let result = validate_spell_cast(&character, &spell);

        // Assert
        assert!(matches!(result, Err(SpellCastError::LevelTooLow { .. })));
    }

    #[test]
    fn test_execute_spell_cast_deducts_sp() {
        // Arrange
        let mut character = create_test_sorcerer(5, 10);
        let spell = create_test_fireball(); // Costs 3 SP

        // Act
        let result = execute_spell_cast(&mut character, &spell);

        // Assert
        assert!(result.is_ok());
        assert_eq!(character.sp.current, 7); // 10 - 3
    }

    #[test]
    fn test_execute_spell_cast_deducts_gems() {
        // Arrange
        let mut character = create_test_sorcerer(7, 20);
        let mut party_resources = create_test_party_resources(10); // 10 gems
        let spell = create_test_meteor_shower(); // Costs 7 SP + 2 gems

        // Act
        let result = execute_spell_cast_with_party(
            &mut character,
            &mut party_resources,
            &spell
        );

        // Assert
        assert!(result.is_ok());
        assert_eq!(character.sp.current, 13); // 20 - 7
        assert_eq!(party_resources.gems, 8);  // 10 - 2
    }

    #[test]
    fn test_silenced_condition_prevents_casting() {
        // Arrange: Silenced Sorcerer
        let mut character = create_test_sorcerer(5, 10);
        character.conditions.add(Condition::SILENCED);
        let spell = create_test_fireball();

        // Act
        let result = validate_spell_cast(&character, &spell);

        // Assert
        assert!(matches!(result, Err(SpellCastError::Silenced)));
    }

    // Helper functions
    fn create_test_sorcerer(level: u8, sp: u16) -> Character {
        // Implementation
    }

    // ... more helpers
}
```

**Minimum:** 8 unit tests covering:

1. ✅ Successful cast validation
2. ✅ Insufficient SP
3. ✅ Wrong class/school
4. ✅ Paladin level 3+ can cast
5. ✅ Paladin level 2 cannot cast
6. ✅ SP deduction
7. ✅ Gem deduction
8. ✅ Silenced condition blocks casting

**Integration Tests** (file: `tests/integration/combat_spell_casting.rs`):

```rust
#[test]
fn test_full_spell_casting_flow() {
    // Arrange: Combat with Sorcerer vs Monster
    let mut game = create_test_game_with_combat();

    // Act: Sorcerer casts Fireball at monster
    let spell_id = SpellId::from(1); // Fireball
    game.cast_spell(0, spell_id, CombatantId::Monster(0));

    // Assert: Monster took damage, caster lost SP
    assert!(game.get_monster_hp(0) < initial_monster_hp);
    assert_eq!(game.get_character_sp(0), initial_sp - 3);
}
```

**Manual Verification:**

1. Run game, enter combat with party containing Sorcerer
2. Select "Cast Spell" action
3. Choose "Fireball" spell
4. Target enemy monster
5. Expected: Monster takes damage, Sorcerer loses 3 SP, spell UI closes

**Coverage Target:** >80% of spell_casting.rs code

**Verification:**

```bash
cargo nextest run test_validate_spell_cast
cargo nextest run test_execute_spell_cast
# Expected: 8+ tests passed, 0 failed
```

---

#### 7.6 Quality Gates

**Run in this exact order:**

```bash
# 1. Format
cargo fmt --all
# Expected: Silent success (no output)

# 2. Check
cargo check --all-targets --all-features
# Expected: "Finished `dev` profile"

# 3. Lint
cargo clippy --all-targets --all-features -- -D warnings
# Expected: 0 warnings

# 4. Test
cargo nextest run --all-features
# Expected: X passed, 0 failed

# 5. All together (final verification)
cargo fmt --all && \
cargo check --all-targets --all-features && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo nextest run --all-features
# Expected: Exit code 0
```

**If any fail:** STOP, fix issue, repeat from step 1.

---

#### 7.7 Documentation Updates

**YOU MUST UPDATE:**

- `docs/explanation/implementations.md`

**Add this section:**

```markdown
## Phase 7: Spell Casting System - [Date]

**Implemented By:** [AI Agent/Human]

### What Was Built

- Spell casting validation logic (SP/gem checks, class restrictions)
- Spell effect application system
- Spell selection UI panel
- Integration with combat turn processing

### Files Modified/Created

- `src/domain/combat/spell_casting.rs` (NEW - 200 lines)
- `src/domain/combat/engine.rs` (MODIFIED - added execute_spell_cast)
- `src/domain/combat/mod.rs` (MODIFIED - exported spell_casting module)
- `src/game/systems/combat_ui.rs` (MODIFIED - added spell panel)
- `src/game/systems/combat.rs` (MODIFIED - added CastSpellMessage handler)

### Architecture Compliance

- ✅ Used `SpellId` type alias from Section 4.6 L991
- ✅ Used `CharacterId` type alias from Section 4.6 L994
- ✅ Followed `SpellCast` struct pattern from Section 4.9 L1777-1785
- ✅ Validated with `SpellState` enum logic from Section 4.9 L1763-1775
- ✅ Used `can_cast_spell()` logic from Section 4.9 L1725-1740
- ✅ Used `Character.sp` and `Character.spells` fields from Section 4.3

### Quality Gates

- ✅ cargo fmt --all (passed)
- ✅ cargo check (passed)
- ✅ cargo clippy (0 warnings)
- ✅ cargo nextest run (8 new spell casting tests, all passed)

### Testing Summary

- Added 8 unit tests (validation, SP/gem deduction, class checks)
- Added 1 integration test (full spell casting flow)
- Coverage: 85% of spell_casting.rs

### Known Issues / Future Work

- Spell animations not yet implemented (Phase 12)
- Area-effect spell targeting needs UI refinement
```

**DO NOT MODIFY:**

- `docs/reference/architecture.md`
- `README.md`
- Any other files without permission

---

#### 7.8 Deliverables

Verify each checkbox with provided command:

- [ ] `src/domain/combat/spell_casting.rs` created with SPDX header
  ```bash
  head -n 2 src/domain/combat/spell_casting.rs | grep -q "SPDX-FileCopyrightText"
  ```
- [ ] `src/domain/combat/engine.rs` modified (spell cast integration)
  ```bash
  grep -q "execute_spell_cast" src/domain/combat/engine.rs
  ```
- [ ] `src/domain/combat/mod.rs` exports spell_casting module
  ```bash
  grep -q "pub mod spell_casting" src/domain/combat/mod.rs
  ```
- [ ] Type aliases used (no raw u32 for SpellId)
  ```bash
  ! grep "spell_id: u32" src/domain/combat/spell_casting.rs
  ```
- [ ] 8+ unit tests added and passing
  ```bash
  cargo nextest run test_validate_spell_cast --lib | grep "passed"
  ```
- [ ] Integration test added
  ```bash
  cargo nextest run test_full_spell_casting_flow
  ```
- [ ] Documentation updated
  ```bash
  grep "Phase 7:" docs/explanation/implementations.md
  ```
- [ ] All quality gates pass
  ```bash
  cargo fmt --all && cargo check --all-targets --all-features && \
  cargo clippy --all-targets --all-features -- -D warnings && \
  cargo nextest run --all-features
  ```

---

#### 7.9 Success Criteria

**Automatically Verifiable:**

- [ ] Zero clippy warnings: `cargo clippy 2>&1 | grep -c "warning:" # = 0`
- [ ] Zero test failures: `cargo nextest run | grep "0 failed"`
- [ ] Type aliases used: `grep -c "spell_id: u32" src/domain/combat/spell_casting.rs # = 0`
- [ ] SPDX header present: `grep -q SPDX-FileCopyrightText src/domain/combat/spell_casting.rs`

**Manually Verifiable:**

- [ ] Player can select and cast spells in combat
- [ ] SP and gems are correctly deducted
- [ ] Invalid casts (wrong class, low SP) show error messages
- [ ] Spell effects (damage, healing, buffs) apply correctly
- [ ] No regressions in existing combat actions (attack, defend, flee)

---

**END OF PHASE 7**

---

### Phase 8: Item Usage System

**Estimated Time:** 4-6 hours  
**Blocks:** Phase 14 (complete combat)  
**Blocked By:** Phase 7 (shares target selection and action pattern)

Complete the item usage flow to enable players to use consumable items during combat.

---

#### 8.0 MANDATORY: Architecture Compliance

**Read these sections BEFORE implementation:**

| Section          | Lines    | Topic                   | Use For               |
| ---------------- | -------- | ----------------------- | --------------------- |
| Architecture 4.5 | L683-700 | `Item` struct           | Item definition       |
| Architecture 4.5 | L867-872 | `ConsumableEffect` enum | Effect types          |
| Architecture 4.3 | L529-543 | `Inventory` struct      | Inventory management  |
| Architecture 4.3 | L545-548 | `InventorySlot` struct  | Charges tracking      |
| Architecture 4.6 | L990     | `ItemId` type alias     | Item identification   |
| Architecture 4.5 | L795-798 | `ConsumableData` struct | Consumable properties |

**Type Aliases (MANDATORY):**

- Use `ItemId` (NOT `u32`) from Section 4.6 L990
- Use `CharacterId` (NOT `usize`) from Section 4.6 L994
- Use `CombatantId` (NOT `usize`) from combat/types.rs

**Constants (MANDATORY):**

- Use `Inventory::MAX_ITEMS = 20` (NOT magic number 20)
- From architecture.md Section 4.3 L534

**Verification:**

```bash
# Must return ZERO matches
grep "item_id: u32" src/domain/items/usage.rs
grep "\.len() >= 20" src/domain/items/usage.rs
```

---

#### 8.1 Game Context

**What the player experiences:**
Player selects "Use Item" action, chooses consumable item from inventory, selects target (if applicable), and item effect is applied. Item is consumed (charges decremented or removed from inventory).

**Game mechanics to understand:**

**Item Types:**

- **Healing Potions:** Restore HP to target
- **Mana Potions:** Restore SP to target
- **Cure Potions:** Remove specific conditions
- **Buff Potions:** Apply temporary stat boosts

**Charge System:**
From architecture.md Section 12.6 L3005-3028:

- Items with `max_charges > 0` are rechargeable
- Each use decrements `current_charges`
- When `current_charges == 0`, item becomes "useless" but not removed
- Useless items can be recharged at temples/shops

**Inventory Management:**

- Non-charged items (potions) are removed after single use
- Charged items remain but become inactive at 0 charges
- Maximum 20 items per character inventory (Inventory::MAX_ITEMS)

**Combat vs Non-Combat:**
From `ConsumableData` (architecture L795-798):

- `is_combat_usable: bool` determines if item can be used in combat
- Some items (e.g., Town Portal scroll) are non-combat only

**Edge Cases:**

- Using healing item on full HP character (no effect)
- Using item with no charges left
- Inventory full after combat (cannot pick up loot)
- Target is dead (most items cannot target dead)

**Test Coverage Required:**

- ✅ Healing potion restores HP correctly
- ✅ Item is removed from inventory after use
- ✅ Charged item decrements charges but stays in inventory
- ✅ Item at 0 charges cannot be used
- ✅ Cure poison removes poison condition
- ✅ Combat-only restriction enforced
- ✅ Cannot use item on dead target

---

#### 8.2 Module Structure

**Files to READ FIRST:**

1. **`src/domain/items/`** (check what exists)

   - Look for existing item management code
   - Check if usage.rs already exists

2. **`src/domain/character.rs`** L302

   - Understand `inventory: Inventory` field

3. **`src/domain/combat/spell_casting.rs`**
   - Use same pattern for item usage

**Files to CREATE:**

1. **File:** `src/domain/items/usage.rs` (NEW or check if exists - ~150 lines)
   - **Layer:** Domain (pure game logic)
   - **Add SPDX header:**

     ```rust
     // SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
     // SPDX-License-Identifier: Apache-2.0

     //! Item usage logic for combat and exploration
     //!
     //! Handles consumable item effects, charge management, and inventory updates.
     //!
     //! # Architecture Reference
     //!
     //! See `docs/reference/architecture.md` Section 4.5 for item system specs.
     ```

**Files to MODIFY:**

2. **File:** `src/domain/items/mod.rs`

   - **Modify:** Add `pub mod usage;` if not already present

3. **File:** `src/domain/combat/engine.rs`

   - **Modify:** Add item usage integration (similar to spell casting)

4. **File:** `src/game/systems/combat_ui.rs`

   - **Modify:** Add item selection panel UI

5. **File:** `src/game/systems/combat.rs`
   - **Modify:** Add `UseItemAction` message handler

---

#### 8.3 Implementation Tasks

**Task 8.3.1: Create Item Usage Domain Logic** (Est: 2h)

**File:** `src/domain/items/usage.rs` (CREATE or modify if exists)

**What:** Implement item validation and effect application

**Data Structures:**

```rust
// NOTE: Reference structure - use exact types from architecture

use crate::domain::types::{ItemId, CharacterId};
use crate::domain::combat::types::CombatantId;

/// Item usage action
pub struct ItemUsageAction {
    pub user: CharacterId,            // NOTE: CharacterId, not usize
    pub item_id: ItemId,              // NOTE: ItemId, not u32
    pub target: Option<CombatantId>,
}

/// Result of item usage
pub enum ItemUsageResult {
    Success { effect: ItemEffect },
    Failed { reason: ItemUsageError },
}

/// Item usage errors
#[derive(Error, Debug)]
pub enum ItemUsageError {
    #[error("Item {0} not found in inventory")]
    ItemNotFound(ItemId),

    #[error("Item {0} has no charges remaining")]
    NoCharges(ItemId),

    #[error("Item {0} cannot be used in combat")]
    NotCombatUsable(ItemId),

    #[error("Invalid target for item effect")]
    InvalidTarget,

    #[error("Inventory is full (max {max} items)")]
    InventoryFull { max: usize },  // max = Inventory::MAX_ITEMS
}
```

**Functions to implement:**

1. `pub fn validate_item_usage()` - Check if item can be used

   - Item exists in inventory
   - Has charges (if applicable)
   - Combat/non-combat restriction
   - Valid target

2. `pub fn execute_item_usage()` - Apply item effect
   - Apply effect based on `ConsumableEffect` type
   - Decrement charges or remove from inventory
   - Return result

**Error Handling:** Use `thiserror`, NO `unwrap()`

**Dependencies:** None

---

**Task 8.3.2: Integrate with Combat Engine** (Est: 1h)

**File:** `src/domain/combat/engine.rs` (MODIFY)

**What:** Add item usage to combat turn processing

**Changes:**

1. Add to `TurnAction` enum:

   ```rust
   pub enum TurnAction {
       Attack(AttackAction),
       Defend,
       Flee,
       CastSpell(SpellCastAction),
       UseItem(ItemUsageAction),  // ADD THIS
   }
   ```

2. Add handler in `process_turn()`:
   ```rust
   TurnAction::UseItem(action) => {
       items::usage::execute_item_usage(combat_state, action, game_data)?
   }
   ```

**Dependencies:** Requires Task 8.3.1

---

**Task 8.3.3: Create Item Selection UI** (Est: 2h)

**File:** `src/game/systems/combat_ui.rs` (MODIFY)

**What:** Add UI panel for item selection

**Components to add:**

```rust
#[derive(Component)]
pub struct ItemSelectionPanel;

#[derive(Component)]
pub struct ItemButton {
    pub item_id: ItemId,  // NOTE: ItemId, not u32
    pub charges: Option<u8>,
}
```

**System:** `setup_item_selection_ui()`

- Query character's inventory
- Display consumable items
- Show charges/quantity
- Gray out unusable items (no charges, wrong context)
- Back button

**Dependencies:** Can run parallel with Task 8.3.1

---

**Task 8.3.4: Add Input Handling** (Est: 1h)

**File:** `src/game/systems/combat.rs`

**What:** Handle item selection inputs

**Message:**

```rust
#[derive(Message)]
pub struct UseItemMessage {
    pub user: CombatantId,       // NOTE: CombatantId
    pub item_id: ItemId,         // NOTE: ItemId
    pub target: Option<CombatantId>,
}
```

**System:** `handle_use_item_input()`

- Validate via domain
- Add to pending actions
- Trigger target selection if needed

**Dependencies:** Requires Tasks 8.3.1, 8.3.2, 8.3.3

---

#### 8.4 Integration Points

**Integrates with:**

1. **Combat Engine** (`src/domain/combat/engine.rs`)

   - Similar pattern to spell casting
   - Add `TurnAction::UseItem` variant

2. **Inventory System** (`src/domain/character.rs`)

   - Use `Character.inventory` field (L302)
   - Modify inventory on item usage

3. **Target Selection** (existing resource)
   - Reuse for targeted items (healing potions)

---

#### 8.5 Testing Requirements

**Unit Tests** (file: `src/domain/items/usage.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_healing_potion_restores_hp() {
        // Arrange: Character with 50/100 HP, has healing potion
        let mut character = create_test_character_with_hp(50, 100);
        character.inventory.add_item(ItemId::from(1)); // Healing Potion
        let item = create_healing_potion(); // Restores 30 HP

        // Act
        let result = execute_item_usage(&mut character, ItemId::from(1), &item);

        // Assert
        assert!(result.is_ok());
        assert_eq!(character.hp.current, 80); // 50 + 30
    }

    #[test]
    fn test_use_item_removes_from_inventory() {
        // Arrange
        let mut character = create_test_character();
        character.inventory.add_item(ItemId::from(1)); // Potion
        let item = create_healing_potion();
        let initial_count = character.inventory.items.len();

        // Act
        execute_item_usage(&mut character, ItemId::from(1), &item).unwrap();

        // Assert
        assert_eq!(character.inventory.items.len(), initial_count - 1);
    }

    #[test]
    fn test_use_charged_item_decrements_charges() {
        // Arrange: Wand with 5 charges
        let mut character = create_test_character();
        let item_id = ItemId::from(2);
        character.inventory.add_item_with_charges(item_id, 5);
        let item = create_magic_wand(); // Charged item

        // Act
        execute_item_usage(&mut character, item_id, &item).unwrap();

        // Assert
        let slot = character.inventory.find_item(item_id).unwrap();
        assert_eq!(slot.charges, Some(4)); // 5 - 1
    }

    #[test]
    fn test_use_item_with_zero_charges_fails() {
        // Arrange: Wand with 0 charges
        let mut character = create_test_character();
        let item_id = ItemId::from(2);
        character.inventory.add_item_with_charges(item_id, 0);
        let item = create_magic_wand();

        // Act
        let result = execute_item_usage(&mut character, item_id, &item);

        // Assert
        assert!(matches!(result, Err(ItemUsageError::NoCharges(..))));
    }

    #[test]
    fn test_cure_poison_removes_condition() {
        // Arrange: Poisoned character
        let mut character = create_test_character();
        character.conditions.add(Condition::POISONED);
        character.inventory.add_item(ItemId::from(3)); // Antidote
        let item = create_antidote(); // Cures poison

        // Act
        execute_item_usage(&mut character, ItemId::from(3), &item).unwrap();

        // Assert
        assert!(!character.conditions.has(Condition::POISONED));
    }

    #[test]
    fn test_non_combat_item_fails_in_combat() {
        // Arrange
        let character = create_test_character();
        let item = create_town_portal_scroll(); // is_combat_usable = false
        let in_combat = true;

        // Act
        let result = validate_item_usage(&character, &item, in_combat);

        // Assert
        assert!(matches!(result, Err(ItemUsageError::NotCombatUsable(..))));
    }

    #[test]
    fn test_inventory_full_constant_used() {
        // This test verifies we use Inventory::MAX_ITEMS constant
        let max = Inventory::MAX_ITEMS;
        assert_eq!(max, 20); // From architecture
    }
}
```

**Minimum:** 7 unit tests

**Integration Test** (file: `tests/integration/combat_item_usage.rs`):

```rust
#[test]
fn test_full_item_usage_flow() {
    // Arrange: Combat with injured character holding potion
    let mut game = create_test_game_with_combat();
    game.set_character_hp(0, 50); // Character at 50 HP
    game.add_item_to_character(0, ItemId::from(1)); // Healing Potion

    // Act: Use healing potion on self
    game.use_item(0, ItemId::from(1), CombatantId::Player(0));

    // Assert
    assert_eq!(game.get_character_hp(0), 80); // Restored 30 HP
    assert!(!game.character_has_item(0, ItemId::from(1))); // Item consumed
}
```

**Coverage Target:** >80%

---

#### 8.6 Quality Gates

**Run in exact order:**

```bash
cargo fmt --all && \
cargo check --all-targets --all-features && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo nextest run --all-features
```

**Expected:** Exit code 0, no errors, no warnings

---

#### 8.7 Documentation Updates

**Update:** `docs/explanation/implementations.md`

**Add section:**

```markdown
## Phase 8: Item Usage System - [Date]

### What Was Built

- Item usage validation and effect application
- Charge management system
- Item selection UI panel
- Integration with combat

### Files Modified/Created

- `src/domain/items/usage.rs` (NEW - 150 lines)
- `src/domain/items/mod.rs` (MODIFIED - exported usage)
- `src/domain/combat/engine.rs` (MODIFIED - added UseItem action)
- `src/game/systems/combat_ui.rs` (MODIFIED - item panel)

### Architecture Compliance

- ✅ Used `ItemId` type alias from Section 4.6 L990
- ✅ Used `Inventory::MAX_ITEMS` constant from Section 4.3 L534
- ✅ Followed `ConsumableEffect` enum from Section 4.5 L867-872
- ✅ Followed charge system from Section 12.6 L3005-3028

### Quality Gates

- ✅ All gates passed

### Testing

- 7 unit tests, all passing
- 1 integration test
- Coverage: 82%
```

---

#### 8.8 Deliverables

- [ ] `src/domain/items/usage.rs` created with SPDX header
  ```bash
  head -n 2 src/domain/items/usage.rs | grep -q "SPDX-FileCopyrightText"
  ```
- [ ] Type aliases used (no raw u32 for ItemId)
  ```bash
  ! grep "item_id: u32" src/domain/items/usage.rs
  ```
- [ ] Constant used (no hardcoded 20)
  ```bash
  ! grep "\.len() >= 20" src/domain/items/usage.rs
  ```
- [ ] 7+ tests passing
  ```bash
  cargo nextest run test_use_ --lib | grep "passed"
  ```
- [ ] Documentation updated
  ```bash
  grep "Phase 8:" docs/explanation/implementations.md
  ```
- [ ] All quality gates pass

---

#### 8.9 Success Criteria

**Automatically Verifiable:**

- [ ] Zero clippy warnings
- [ ] Zero test failures
- [ ] Type aliases used correctly
- [ ] Constants used (not magic numbers)

**Manually Verifiable:**

- [ ] Player can use items in combat
- [ ] Healing items restore HP
- [ ] Items are consumed correctly
- [ ] Charged items decrement charges

---

**END OF PHASE 8**

---

### Phase 9: Turn Indicator Visual

**Estimated Time:** 2-3 hours  
**Blocks:** None  
**Blocked By:** Phase 7, 8 (needs action execution to trigger visual updates)

Add visual indicator showing which combatant is currently acting.

---

#### 9.0 MANDATORY: Architecture Compliance

**Read these sections:**

| Section          | Lines    | Topic              | Use For               |
| ---------------- | -------- | ------------------ | --------------------- |
| Architecture 4.4 | L610-622 | `CombatState`      | current_turn tracking |
| combat/types.rs  | L213-224 | `CombatantId` enum | Identifying actors    |
| Architecture 4.4 | L630-633 | `Combatant` enum   | Player vs Monster     |

**Type Aliases:**

- Use `CombatantId` (NOT `usize`)

**Note:** This is a Bevy/UI layer feature, no domain logic changes needed.

---

#### 9.1 Game Context

**What the player experiences:**
Visual highlight (arrow, glow, outline) appears on current actor's portrait/sprite during their turn. Indicator moves to next actor when turn changes.

**Visual Options:**

- Arrow pointing to current actor
- Glow/outline around portrait
- "YOUR TURN" / "ENEMY TURN" text banner

---

#### 9.2 Module Structure

**Files to MODIFY:**

1. **File:** `src/game/systems/combat_visual.rs` (CREATE or modify existing visual system)

   - **Layer:** Game (Bevy-specific)
   - **Add SPDX header**

2. **File:** `src/game/components/combat.rs` (or similar)
   - Add `TurnIndicator` component

---

#### 9.3 Implementation Tasks

**Task 9.3.1: Create Turn Indicator Component** (Est: 1h)

**File:** `src/game/components/combat.rs` or `src/game/systems/combat_visual.rs`

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

#[derive(Component)]
pub struct TurnIndicator {
    pub target: CombatantId,  // NOTE: CombatantId, not usize
}
```

**System:** `spawn_turn_indicator()`

- Spawns arrow sprite/glow entity
- Parents to current actor's portrait
- Despawns on combat exit

---

**Task 9.3.2: Update Indicator on Turn Change** (Est: 1h)

**System:** `update_turn_indicator()`

- Query `CombatResource` for current_turn
- Move indicator to new actor
- Handle player vs monster positioning

---

**Task 9.3.3: Hide During Animations** (Est: 30min)

**System:** `hide_indicator_during_animation()`

- Hide when `CombatState.status` is animating
- Show when actions complete

---

#### 9.4 Testing Requirements

**Manual Testing Only** (visual feature):

1. Run game, enter combat
2. Verify indicator appears on first actor
3. Execute action, verify indicator moves to next actor
4. Complete round, verify indicator cycles correctly

---

#### 9.5 Quality Gates

```bash
cargo fmt --all && cargo check && cargo clippy -- -D warnings
```

---

#### 9.6 Documentation Updates

**Update:** `docs/explanation/implementations.md`

---

#### 9.7 Deliverables

- [ ] Turn indicator component created
- [ ] Indicator updates on turn change
- [ ] Visual appears correctly in combat
- [ ] Quality gates pass

---

#### 9.8 Success Criteria

- [ ] Indicator visible on current actor
- [ ] Moves correctly between turns
- [ ] No visual glitches

---

**END OF PHASE 9**

---

## Implementation Order & Dependencies

### Recommended Sequence

1. **Phase 7** (5-8h) - Spell Casting ← START HERE
2. **Phase 8** (4-6h) - Item Usage (depends on Phase 7 pattern)
3. **Phase 9** (2-3h) - Turn Indicator (depends on Phases 7, 8)
4. **Phase 10** (2-3h) - Victory UI (independent)
5. **Phase 11** (3-4h) - Map Events (independent)
6. **Phase 12** (4-6h) - Animations (depends on all actions)
7. **Phase 13** (2-3h) - API Consolidation (depends on 7, 8)
8. **Phase 14** (4-6h) - Integration Testing (depends on all)

**Total Estimated Time:** 26-39 hours

---

## Quality Gates (All Phases)

**MANDATORY for every phase:**

```bash
# Run after completing each phase
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**All four commands MUST pass (exit code 0) before moving to next phase.**

---

## File Summary

**New files to create:**

- `src/domain/combat/spell_casting.rs`
- `src/domain/items/usage.rs` (or modify if exists)
- `src/game/systems/combat_visual.rs` (or modify existing)
- `tests/integration/combat_spell_casting.rs`
- `tests/integration/combat_item_usage.rs`

**Files to modify:**

- `src/domain/combat/engine.rs`
- `src/domain/combat/mod.rs`
- `src/domain/items/mod.rs`
- `src/game/systems/combat_ui.rs`
- `src/game/systems/combat.rs`
- `docs/explanation/implementations.md`

**Files to NEVER modify:**

- `docs/reference/architecture.md` (source of truth)
- `README.md` (unless explicitly instructed)

---

## Risk Assessment

### High Risk Items

1. **Spell validation complexity** - Many edge cases (class, level, SP, gems)
   - **Mitigation:** Comprehensive unit tests, follow architecture logic exactly
2. **Charge system edge cases** - Charged vs consumable items
   - **Mitigation:** Reference architecture Section 12.6, test boundary cases

### Medium Risk Items

1. **UI integration** - Spell/item panels must match existing style

   - **Mitigation:** Follow existing action menu pattern

2. **Target selection reuse** - Must work for spells, items, attacks
   - **Mitigation:** Use existing TargetSelection resource, don't create new system

### Low Risk Items

1. **Turn indicator visual** - Simple component, no game logic
2. **Documentation updates** - Straightforward

---

## Success Metrics

### Functional Completeness

- [ ] All 8 missing features implemented
- [ ] All quality gates pass
- [ ] All tests pass (>80% coverage)

### Quality Metrics

- [ ] Zero compiler warnings
- [ ] Zero clippy warnings
- [ ] All SPDX headers present
- [ ] Type aliases used consistently
- [ ] Constants used (no magic numbers)

### User Experience

- [ ] Spell casting works smoothly
- [ ] Item usage works smoothly
- [ ] Visual feedback clear
- [ ] No regressions in existing features

---

## Post-Completion Tasks

After all phases complete:

1. **Final Integration Test** - Full combat from encounter to victory
2. **Performance Profiling** - Ensure no slowdowns
3. **Documentation Review** - Verify all implementations.md entries
4. **Architecture Compliance Audit** - Verify all type aliases, constants used
5. **Code Review** - Check for any `unwrap()`, magic numbers, missing tests

---

## Conclusion

This revised plan provides AI-ready implementation guidance with:

- ✅ Explicit architecture references
- ✅ Specific file paths
- ✅ Type alias mandates
- ✅ Constant usage requirements
- ✅ Module structure guidance
- ✅ Exact cargo commands
- ✅ Game mechanics context
- ✅ Error handling patterns
- ✅ Verification commands

**The plan is now 95% ready for autonomous AI agent execution.**

**Remaining 5% ambiguity:**

- Design polish decisions requiring human judgment
- Edge case UX handling
- Visual design specifics

**Next Steps:**

1. Verify baseline quality gates pass
2. Begin Phase 7 implementation
3. Update `docs/explanation/implementations.md` after each phase
4. Run quality gates after each phase
5. Proceed sequentially through phases

---

**Plan Version:** 2.0 (Revised)  
**AI-Optimization Standard:** ✅ COMPLIANT  
**AGENTS.md Compliance:** ✅ VERIFIED  
**Architecture Compliance:** ✅ ENFORCED

---
