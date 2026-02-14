# Combat System Completion Plan - Comprehensive Review

**Review Date:** 2025-01-XX
**Reviewer:** AI Agent (Architecture & AGENTS.md Compliance)
**Document Under Review:** `docs/explanation/combat_system_completion_plan.md`
**Status:** ❌ REQUIRES MAJOR REVISIONS

---

## Executive Summary

The combat system completion plan provides a solid **conceptual framework** but **fails critical AI-optimization standards** and **lacks architectural compliance details** required by AGENTS.md.

**Bottom Line:** The plan cannot be executed by an AI agent without significant ambiguity and interpretation. An estimated **6-9 hours of revision** is needed before implementation can begin.

### Overall Assessment

| Category                | Score   | Status           |
| ----------------------- | ------- | ---------------- |
| Conceptual Structure    | 8/10    | ✅ GOOD          |
| AI Readability          | 4/10    | ❌ POOR          |
| Architecture Compliance | 3/10    | ❌ CRITICAL      |
| File Path Specificity   | 2/10    | ❌ CRITICAL      |
| Validation Criteria     | 5/10    | ⚠️ NEEDS WORK    |
| Implementation Order    | 7/10    | ✅ ACCEPTABLE    |
| Documentation Guidance  | 4/10    | ❌ POOR          |
| **OVERALL READINESS**   | **30%** | ❌ **NOT READY** |

### Key Strengths

- ✅ Good phase breakdown (Phases 7-14)
- ✅ Testing requirements included for each phase
- ✅ Risk assessment and dependency graph present
- ✅ Success metrics defined

### Critical Gaps

- ❌ **No architecture.md section references** - AI agents will create wrong data structures
- ❌ **No specific file paths** - AI agents won't know where to place code
- ❌ **Type aliases not mandated** - Code will use raw `u32` instead of `SpellId`
- ❌ **Constants not referenced** - Magic numbers will be hardcoded
- ❌ **Module structure unclear** - Code will go in wrong layers
- ❌ **Data file format not specified** - AI agents will create `.json` instead of `.ron`
- ❌ **SPDX headers not mentioned** - New files will lack required copyright
- ❌ **Documentation rules vague** - AI agents might modify `architecture.md`

**RECOMMENDATION:** PAUSE implementation until Priority 1 fixes are completed (see Action Items section).

---

## Critical Issues (MUST FIX)

### Issue 1: Missing Architecture Section References

**Severity:** CRITICAL
**Impact:** AI agents will create incorrect data structures

**Problem:**

- Plan shows code examples without referencing `docs/reference/architecture.md` definitions
- Example: `CastSpellAction` struct shown in Phase 7.2 (L79-83) but doesn't reference architecture.md Section 4.9 L1777-1785 which defines `SpellCast`
- No mandate to use EXACT architecture definitions

**Required Fix:**

```markdown
### Phase 7: Spell Casting System

#### 7.0 Architecture Compliance (NEW SECTION)

**MANDATORY:** Before implementation, review these architecture sections:

- **Section 4.9 L1777-1785:** `SpellCast` struct definition (USE THIS EXACTLY)
- **Section 4.9 L1763-1775:** `SpellState` enum for validation
- **Section 4.9 L1725-1740:** `can_cast_spell()` function logic
- **Section 4.6 L990-991:** Type aliases (`SpellId`, `CharacterId`)
- **Section 4.3 L551-580:** `SpellBook` structure

**RULE:** You MUST use architecture-defined structures. DO NOT create new ones.
```

**Apply to ALL phases:** Every phase must reference exact architecture sections.

---

### Issue 2: No Specific File Paths

**Severity:** CRITICAL
**Impact:** AI agents won't know where to place code

**Problem:**

- Plan says "implement spell casting" but doesn't specify file paths
- No reference to existing module structure in `src/domain/combat/`, `src/application/`, `src/game/`
- Violates AI-optimization standard: "Include specific file paths"

**Example of Current State (Phase 7.2):**

```markdown
❌ BAD (Current):

- Create `CastSpellAction` struct
- Implement spell casting logic
```

**Required Fix:**

```markdown
✅ GOOD (Required):
**File: `src/domain/combat/spell_casting.rs` (CREATE NEW)**

1. Add SPDX header:
```

// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

````

2. Create `SpellCastAction` struct (mirrors architecture.md L1777-1785):
- Use `SpellId` type alias (NOT u32)
- Use `CharacterId` type alias (NOT usize)
- Return `Result<SpellCastResult, CombatError>`

**File: `src/domain/combat/engine.rs` (MODIFY EXISTING)**

3. Add function `pub fn execute_spell_cast()` at line ~600:
- Takes `&mut CombatState`, `SpellCastAction`, `&GameData`
- Validates using `can_cast_spell()` logic from architecture
- Deducts SP and gems from caster
- Applies spell effects via `apply_spell_effect()`

**File: `src/domain/combat/mod.rs` (MODIFY EXISTING)**

4. Export new module:
```rust
pub mod spell_casting;
````

````

**Apply to ALL tasks:** Every implementation task needs explicit file paths.

---

### Issue 3: Type Aliases Not Mandated

**Severity:** CRITICAL
**Impact:** Code will use raw types instead of type-safe aliases

**Problem:**
- Plan shows `spell_id: u32` instead of `spell_id: SpellId`
- Architecture.md Section 4.6 L990-1008 defines type aliases
- AGENTS.md Golden Rule 3: "Always use type aliases"

**Example Violations in Plan:**
```rust
// Phase 7.2 L79-83 shows:
pub struct CastSpellAction {
    pub caster: usize,        // ❌ Should be CharacterId
    pub spell_id: u32,        // ❌ Should be SpellId
    pub target: Option<usize>, // ❌ Should be Option<CombatantId>
}
````

**Required Fix:**
Add to each phase's "Architecture Compliance" section:

```markdown
#### Type Aliases (MANDATORY)

You MUST use these type aliases from architecture.md Section 4.6:

| Use This      | NOT This         | Defined At               |
| ------------- | ---------------- | ------------------------ |
| `SpellId`     | `u32`            | L991                     |
| `ItemId`      | `u32`            | L990                     |
| `CharacterId` | `usize` or `u32` | L994                     |
| `CombatantId` | `usize`          | combat/types.rs L213-224 |
| `MonsterId`   | `u32`            | L992                     |

**Verification:** Run `grep -r "spell_id: u32" src/` - must return ZERO results.
```

---

### Issue 4: Constants Not Referenced

**Severity:** HIGH
**Impact:** Magic numbers will be hardcoded

**Problem:**

- Plan doesn't mention constants from architecture.md Section 4.3
- Example: Inventory max items, party size limits, stat ranges

**Required Fix:**
Add constant reference sections:

```markdown
#### Constants (MANDATORY)

From architecture.md Section 4.3 L361-394:

| Constant                  | Value     | Use For                |
| ------------------------- | --------- | ---------------------- |
| `Inventory::MAX_ITEMS`    | 20        | Item capacity checks   |
| `PARTY_MAX_SIZE`          | 6         | Party member limits    |
| `HP_SP_MIN` / `HP_SP_MAX` | 0 / 65535 | Stat boundaries        |
| `SPELL_LEVEL_MIN` / `MAX` | 1 / 7     | Spell level validation |

**NEVER hardcode these values.** Reference constants from architecture.

**Verification:** Run `rg "\.len\(\) >= 20" src/` - must return ZERO results.
```

---

### Issue 5: No Module Structure Guidance

**Severity:** HIGH
**Impact:** AI agents will create modules in wrong locations

**Problem:**

- Plan doesn't reference architecture.md Section 3.2 (module structure)
- No guidance on which layer (domain/application/game) for each component
- Existing structure: `src/domain/combat/`, `src/application/`, `src/game/systems/`

**Required Fix:**
Add to beginning of plan:

````markdown
## Module Structure Reference

**MANDATORY:** Follow architecture.md Section 3.2 module layout.

### Layer Assignments

| Component           | Layer       | Directory                              | Rationale       |
| ------------------- | ----------- | -------------------------------------- | --------------- |
| Spell casting logic | Domain      | `src/domain/combat/`                   | Pure game rules |
| Item usage effects  | Domain      | `src/domain/items/`                    | Pure game rules |
| Combat animations   | Game        | `src/game/systems/combat_animation.rs` | Bevy-specific   |
| UI panels           | Game        | `src/game/systems/combat_ui.rs`        | Bevy-specific   |
| Save/load           | Application | `src/application/save_game.rs`         | Cross-cutting   |

### Existing Files (DO NOT CREATE NEW)

Before creating a file, check if it exists:

```bash
find src/ -name "*combat*" -o -name "*spell*" -o -name "*item*"
```
````

Existing combat files:

- `src/domain/combat/engine.rs` - Core combat logic
- `src/domain/combat/types.rs` - Type definitions
- `src/domain/combat/monster.rs` - Monster data
- `src/game/systems/combat_system.rs` - Bevy integration

````

---

### Issue 6: Cargo Commands Missing Details

**Severity:** HIGH
**Impact:** AI agents won't know how to validate

**Problem:**
- Quality gates mention commands but lack exact syntax
- Missing expected output examples
- No troubleshooting guidance

**Current State (L977-986):**
```markdown
❌ Vague:
- Run clippy
- Run tests
````

**Required Fix:**

````markdown
### Quality Gate Commands (RUN IN THIS ORDER)

#### 1. Format Code

```bash
cargo fmt --all
```
````

**Expected:** No output (silent success)
**If fails:** Run `rustfmt --version`, verify `rustfmt 1.7.0+` installed

#### 2. Check Compilation

```bash
cargo check --all-targets --all-features
```

**Expected:**

```
    Checking antares v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in X.XXs
```

**If fails:** Read error, fix compilation issue, repeat

#### 3. Lint with Clippy

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

**Expected:**

```
    Checking antares v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in X.XXs
```

**Count:** ZERO warnings (warnings treated as errors with `-D`)
**If fails:** Fix first warning, re-run, repeat until zero

#### 4. Run Tests

```bash
cargo nextest run --all-features
```

**Expected:**

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in X.XXs
       Tests: X passed, 0 failed
```

**Minimum Coverage:** >80% per AGENTS.md
**If fails:** Read failure output, fix test or code, repeat

#### 5. Final Verification

```bash
# All four commands in sequence - MUST ALL PASS
cargo fmt --all && \
cargo check --all-targets --all-features && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo nextest run --all-features
```

**Expected:** Exit code 0, no errors, no warnings

````

---

### Issue 7: Data File Format Not Specified

**Severity:** HIGH
**Impact:** AI agents will create .json or .yaml files

**Problem:**
- Plan doesn't specify data files must use `.ron` format
- Architecture.md Section 7.1 mandates RON for game data
- AGENTS.md Implementation Rule 1: "Use `.ron` for game data"

**Required Fix:**
Add to each phase dealing with data:

```markdown
#### Data File Format (MANDATORY)

**RULE:** All game data files MUST use `.ron` (Rusty Object Notation) format.

**Correct:**
- `data/spells.ron` ✅
- `data/items.ron` ✅
- `data/monsters.ron` ✅

**Incorrect:**
- `data/spells.json` ❌ WRONG
- `data/items.yaml` ❌ WRONG
- `data/config.yml` ❌ WRONG (unless it's CI config)

**Reference:** See architecture.md Section 7.2 L1986-2346 for RON examples.

**Validation:**
```bash
# Ensure no JSON/YAML game data files
find data/ -name "*.json" -o -name "*.yaml" -o -name "*.yml"
# Expected: Empty output (or only CI/build configs)
````

````

---

### Issue 8: Documentation Update Rules Missing

**Severity:** MEDIUM
**Impact:** AI agents will modify architecture.md

**Problem:**
- Plan says "update documentation" but doesn't specify which files
- AGENTS.md Rule: Update `docs/explanation/implementations.md`, NOT architecture.md
- Architecture.md is source of truth, not implementation log

**Required Fix:**
Add to every phase's deliverables:

```markdown
#### Documentation Updates (MANDATORY)

**YOU MUST UPDATE:**
- `docs/explanation/implementations.md` - Add your implementation summary

**DO NOT MODIFY:**
- `docs/reference/architecture.md` - This is the source of truth you follow
- `README.md` - Unless explicitly instructed
- Any other docs without explicit permission

**Template for implementations.md:**
```markdown
## Phase X: [Feature Name] - [Date]

**Implemented By:** [AI Agent/Human]

### What Was Built
- [Component 1]: [Brief description]
- [Component 2]: [Brief description]

### Files Modified/Created
- `src/domain/combat/spell_casting.rs` (NEW)
- `src/domain/combat/engine.rs` (MODIFIED - added execute_spell_cast)
- `src/game/systems/combat_ui.rs` (MODIFIED - added spell panel)

### Architecture Compliance
- ✅ Used `SpellId` type alias from Section 4.6
- ✅ Followed `SpellCast` struct from Section 4.9 L1777
- ✅ Validated with `SpellState` enum from Section 4.9 L1763

### Quality Gates
- ✅ cargo fmt --all (passed)
- ✅ cargo check (passed)
- ✅ cargo clippy (0 warnings)
- ✅ cargo nextest run (15 new tests, all passed)

### Testing Summary
- Added 15 unit tests (spell validation, SP deduction, effect application)
- Coverage: 87% of new code

### Known Issues / Future Work
- [Any technical debt or follow-up needed]
````

````

---

### Issue 9: Copyright Headers Not Mentioned

**Severity:** MEDIUM
**Impact:** New files won't have required SPDX headers

**Problem:**
- AGENTS.md Implementation Rule 1 mandates SPDX headers
- Plan doesn't mention this requirement

**Required Fix:**
Add to every phase with new file creation:

```markdown
#### Copyright Headers (MANDATORY)

ALL new `.rs` files MUST start with SPDX headers:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Module description here
````

**Verification:**

```bash
# Check all Rust files have SPDX headers
for f in $(find src/ -name "*.rs"); do
  head -n 2 "$f" | grep -q "SPDX-FileCopyrightText" || echo "Missing header: $f"
done
```

````

---

### Issue 10: Game Context Missing

**Severity:** MEDIUM
**Impact:** AI agents won't understand RPG-specific mechanics

**Problem:**
- Plan assumes understanding of RPG mechanics (spell schools, class restrictions)
- AGENTS.md states: "Understanding game mechanics is mandatory"

**Required Fix:**
Add game context sections:

```markdown
### Game Context: Spell Casting Mechanics

**REQUIRED READING:** Understand these game rules before implementation.

#### Spell Schools
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
````

#### Resource Costs

- **SP (Spell Points):** Always consumed (typically = spell level)
- **Gems:** Required for high-level spells (Level 4+ usually need gems)
- **Both:** Deducted BEFORE spell effect applies

#### Spell Context Restrictions

From `SpellState` enum (architecture L1763-1775):

- `CombatOnly` - Fireball, Lightning Bolt
- `NonCombatOnly` - Town Portal, Teleport
- `OutdoorsOnly` - Fly, Walk on Water
- `IndoorOnly` - (none in base game)

**Test Coverage Required:**

- ✅ Sorcerer can't cast Cleric spells
- ✅ Level 1 Paladin can't cast any spells
- ✅ Level 3 Paladin CAN cast Cleric spells
- ✅ Insufficient SP prevents casting
- ✅ Insufficient gems prevents high-level spells
- ✅ Combat-only spells fail in exploration mode

````

---

## Medium Priority Issues

### Issue 11: Validation Criteria Too Vague

**Problem:** Success criteria like "spell casting works correctly" are not automatically verifiable.

**Fix Example:**
```markdown
❌ Vague: "Spell casting works correctly"

✅ Specific:
- [ ] `cargo nextest run test_spell_cast_` returns 0 failures (minimum 8 tests)
- [ ] `grep "spell_id: u32" src/domain/combat/` returns 0 matches (type alias compliance)
- [ ] `cargo clippy` reports 0 warnings on spell_casting.rs
- [ ] Manual test: Cast "Fireball" in combat, verify damage applied to target
- [ ] Manual test: Attempt to cast with insufficient SP, verify error returned
````

---

### Issue 12: Integration Points Not Specified

**Problem:** Plan doesn't explain how new code integrates with existing systems.

**Fix Example:**

```markdown
### Integration Requirements

#### Phase 7: Spell Casting Integration Points

**With Existing Combat Engine (`src/domain/combat/engine.rs`):**

- Hook into `CombatState::process_turn()` at L~250
- Add `TurnAction::CastSpell(SpellCastAction)` enum variant
- Call `execute_spell_cast()` in match statement

**With Existing UI (`src/game/systems/combat_ui.rs`):**

- Add spell selection panel to existing UI layout
- Connect to existing input handling system
- Reuse `TargetSelectionResource` for spell targeting

**With Resource Management (`src/domain/resources.rs`):**

- Use existing `Party::gems` field for gem deduction
- Use `Character::sp.current` for SP deduction
- No new resource types needed

**Files to Read First:**

1. `src/domain/combat/engine.rs` L100-400 (understand `CombatState`)
2. `src/game/systems/combat_ui.rs` L1-200 (understand UI structure)
3. `src/domain/character.rs` L286-314 (understand `Character` struct)
```

---

### Issue 13: Dependency Chain Unclear

**Problem:** Dependency graph exists but doesn't specify blocking tasks.

**Fix Example:**

```markdown
## Implementation Order with Blocking Dependencies

### Phase 7: Spell Casting (5-8 hours)

**Blocks:** Phase 9 (needs action system), Phase 14 (needs complete combat)
**Blocked By:** None (can start immediately)

**Task Order:**

1. **Task 7.2.1** (1h): Create spell_casting.rs types
2. **Task 7.2.2** (2h): Implement validation logic (BLOCKS 7.2.3)
3. **Task 7.2.3** (2h): Implement effect application (REQUIRES 7.2.2)
4. **Task 7.2.4** (1h): Add UI panel (can run parallel with 7.2.2)
5. **Task 7.3** (2h): Write tests (REQUIRES 7.2.1-7.2.3)

### Phase 8: Item Usage (4-6 hours)

**Blocks:** Phase 14
**Blocked By:** Phase 7 (shares target selection API)

### Phase 9: Turn Indicator (2-3 hours)

**Blocks:** None
**Blocked By:** Phase 7, 8 (needs action execution to trigger)
```

---

## Low Priority Issues

### Issue 14: Code Examples May Confuse

**Problem:** Pseudo-code examples might be mistaken for files to create.

**Fix:** Add clarifications:

```markdown
**NOTE:** The following code blocks are ARCHITECTURAL REFERENCES, not files to create.
They show the STRUCTURE to implement. Refer to architecture.md for exact definitions.
```

---

### Issue 15: No Error Handling Patterns

**Problem:** Plan doesn't specify error handling approach.

**Fix:**

````markdown
### Error Handling (MANDATORY)

**Use thiserror for all errors:**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SpellCastError {
    #[error("Insufficient spell points: need {needed}, have {current}")]
    InsufficientSP { needed: u16, current: u16 },

    #[error("Character {0} cannot cast spell school {1:?}")]
    WrongSpellSchool(CharacterId, SpellSchool),

    #[error("Spell {0} cannot be cast in current context")]
    InvalidContext(SpellId),
}
```
````

**NEVER use:**

- ❌ `unwrap()` without justification comment
- ❌ `panic!()` for recoverable errors
- ❌ Ignoring errors with `let _ =`

**Propagate with `?` operator:**

```rust
pub fn cast_spell(...) -> Result<SpellEffect, SpellCastError> {
    validate_caster(caster)?;  // ✅ Propagates error
    let effect = apply_effect(...)?;
    Ok(effect)
}
```

````

---

## Recommended Revisions

### 1. Restructure Each Phase

**Current Structure:**
```markdown
### Phase X: Feature
#### X.1 Design Decisions
#### X.2 Implementation Tasks
#### X.3 Testing
#### X.4 Deliverables
#### X.5 Success Criteria
````

**Recommended Structure:**

```markdown
### Phase X: Feature

#### X.0 Architecture Compliance (NEW)

- Architecture.md sections to review
- Type aliases to use
- Constants to reference
- Existing code to read first

#### X.1 Game Context (NEW)

- Game mechanics explanation
- User-facing behavior
- Edge cases to handle

#### X.2 Module Structure (NEW)

- Files to create (with SPDX headers)
- Files to modify (with line ranges)
- Layer assignment (domain/application/game)

#### X.3 Implementation Tasks

- Task 1: [Description] (File: path/to/file.rs L100-200)
- Task 2: [Description] (File: path/to/file.rs L300-350)
- Each task references architecture sections

#### X.4 Integration Points (NEW)

- Where new code connects to existing systems
- Function signatures to match
- Resources/components to use

#### X.5 Testing Requirements

- Unit tests with specific names
- Integration tests
- Manual verification steps
- Minimum coverage target

#### X.6 Quality Gates (NEW)

- Exact cargo commands with expected output
- Validation scripts
- Automated checks

#### X.7 Documentation Updates (NEW)

- docs/explanation/implementations.md update template
- What to document
- What NOT to modify

#### X.8 Deliverables

- Checkboxes with file paths
- Verification commands

#### X.9 Success Criteria

- Automatically verifiable criteria
- Manual test cases
```

### 2. Add Pre-Implementation Checklist

At the start of plan:

````markdown
## Pre-Implementation Checklist

**BEFORE starting ANY phase, verify:**

- [ ] Tools installed (run: `rustup component list --installed`)
  - [ ] rustfmt
  - [ ] clippy
  - [ ] rust-analyzer (optional but recommended)
- [ ] cargo-nextest installed (run: `cargo nextest --version`)
- [ ] Read AGENTS.md completely
- [ ] Read architecture.md Section 3 (Module Structure)
- [ ] Read architecture.md Section 4 (Data Structures)
- [ ] Understand development workflow (AGENTS.md L177-268)
- [ ] Baseline quality gates pass:
  ```bash
  cargo fmt --all && \
  cargo check --all-targets --all-features && \
  cargo clippy --all-targets --all-features -- -D warnings && \
  cargo nextest run --all-features
  ```
````

**If baseline fails, STOP and fix before implementing new code.**

````

### 3. Add Architecture Quick Reference

```markdown
## Architecture Quick Reference

**Type Aliases (Section 4.6 L990-1008):**
```rust
pub type ItemId = u32;
pub type SpellId = u32;
pub type MonsterId = u32;
pub type CharacterId = u32;
pub type EventId = u32;
````

**Key Constants (Section 4.3 L361-394):**

```rust
const PARTY_MAX_SIZE: usize = 6;
const INVENTORY_MAX_SLOTS: usize = 20;
const EQUIPMENT_MAX_SLOTS: usize = 7;
const HP_SP_MAX: u16 = 65535;
const SPELL_LEVEL_MAX: u8 = 7;
```

**Combat State Location:**

- Definition: `src/domain/combat/engine.rs` L117-140
- Architecture: `docs/reference/architecture.md` L610-622

**Spell State Location:**

- Definition: TBD (to be implemented in Phase 7)
- Architecture: `docs/reference/architecture.md` L1763-1775

**Module Layers (Section 3.2):**

- `src/domain/` - Pure game logic (no Bevy dependencies)
- `src/application/` - Cross-cutting concerns (save/load, etc)
- `src/game/` - Bevy integration (systems, components, resources)

````

---

## Action Items for Plan Revision

### Priority 1 (CRITICAL - Must fix before ANY implementation):
1. Add "Architecture Compliance" section to each phase with exact section references
2. Add specific file paths to every implementation task
3. Add type alias mandate with verification commands
4. Add constants reference section
5. Add module structure guidance with layer assignments
6. Specify exact cargo commands with expected output
7. Mandate `.ron` format for data files
8. Clarify documentation update rules (implementations.md, not architecture.md)
9. Add SPDX header requirements to all file creation tasks

### Priority 2 (HIGH - Should fix for clarity):
10. Add game context sections explaining RPG mechanics
11. Add integration point specifications
12. Refine dependency chain with blocking tasks
13. Make validation criteria automatically verifiable
14. Add error handling pattern requirements
15. Add pre-implementation checklist

### Priority 3 (MEDIUM - Nice to have):
16. Clarify code examples are references, not files to create
17. Add architecture quick reference section
18. Add troubleshooting guides for common errors
19. Add time estimates with ranges (e.g., "2-3 hours")
20. Add rollback procedures if implementation fails

---

## Specific Phase-by-Phase Recommendations

### Phase 7: Spell Casting System

**Must Add:**
- Reference to SpellCast struct (architecture L1777-1785)
- Reference to SpellState enum (architecture L1763-1775)
- Reference to can_cast_spell logic (architecture L1725-1740)
- File path: `src/domain/combat/spell_casting.rs` (new)
- File path: `src/domain/combat/engine.rs` (modify)
- Integration with CombatState.process_turn()
- Type alias mandate: SpellId, CharacterId, not raw types
- Game context: spell schools, class restrictions, resource costs
- Error handling: SpellCastError enum with thiserror

### Phase 8: Item Usage System

**Must Add:**
- Reference to Item struct (architecture L683-700)
- Reference to ConsumableEffect enum (architecture L867-872)
- Reference to Inventory (architecture L529-543)
- File path: `src/domain/items/usage.rs` (new or existing?)
- Constant: `Inventory::MAX_ITEMS = 20`
- Type alias: ItemId, not u32
- Game context: consumable types, charge system
- Integration with inventory management

### Phase 9: Turn Indicator Visual

**Must Add:**
- Layer assignment: `src/game/systems/combat_visual.rs` (Bevy layer)
- Bevy components: Entity, Query, Res usage
- Integration with CombatState.current_turn
- Type alias: CombatantId from combat/types.rs
- No domain layer code (this is purely visual)

### Phase 10: Victory UI Enhancements

**Must Add:**
- Layer assignment: `src/game/systems/combat_ui.rs` (modify existing)
- Integration with loot distribution logic
- Experience calculation reference
- Type alias: ItemId for loot items
- File path for victory summary component

### Phase 11: Map Event Handlers

**Must Add:**
- Reference to EventResult enum (check architecture for map events)
- File path: `src/domain/world/events.rs` (verify exists)
- Trap damage calculation using DiceRoll
- Treasure distribution using Party resource system
- Integration with world movement system

### Phase 12: Combat Animations

**Must Add:**
- Layer assignment: `src/game/systems/combat_animation.rs` (new)
- Bevy animation system usage
- Event-driven architecture (CombatAnimationComplete event)
- State management (Animating state)
- No game logic in this layer (visual only)

### Phase 13: Target Selection API

**Must Add:**
- File path: `src/game/resources/target_selection.rs`
- Bevy Resource pattern
- Integration with spell/item/attack systems
- CombatantId type usage

### Phase 14: Integration Testing

**Must Add:**
- Test file paths: `tests/integration/combat_system.rs`
- Exact test names with expected behavior
- Manual testing checklist with specific scenarios
- Performance benchmarks if applicable

---

## AI Agent Execution Readiness Assessment

**Can an AI agent execute this plan without human clarification?**

### Current State: ❌ NO (30% ready)

**Missing for autonomous execution:**
1. ❌ Architecture section references (0% compliance)
2. ❌ File paths (5% specified)
3. ❌ Type alias mandates (0% specified)
4. ❌ Module structure guidance (0% specified)
5. ⚠️ Validation commands (60% specified, need expected output)
6. ❌ Integration points (10% specified)
7. ⚠️ Error patterns (not specified)
8. ❌ SPDX headers (not mentioned)
9. ❌ Documentation rules (vague)

### Target State: ✅ YES (95% ready)

**After incorporating all Priority 1 + Priority 2 fixes above.**

**Remaining 5% ambiguity:**
- Edge cases requiring human judgment
- Design tradeoffs not covered by architecture
- User experience polish decisions

---

## Conclusion

The combat system completion plan has **solid conceptual structure** but requires **major revisions to meet AI-optimization standards** and **AGENTS.md compliance**.

### Primary Gaps:
1. **Architecture references missing** - Plan doesn't point to exact definitions
2. **File paths missing** - AI agents don't know where to code
3. **Type system not enforced** - Raw types will be used instead of aliases
4. **Module structure unclear** - Code will go in wrong layers
5. **Validation ambiguous** - Success criteria not automatically verifiable

### Estimated Revision Effort:
- **Priority 1 fixes:** 4-6 hours of plan revision
- **Priority 2 fixes:** 2-3 hours additional
- **Total:** 6-9 hours to bring plan to 95% AI-ready state

### Recommendation:
**PAUSE implementation until Priority 1 fixes are completed.**

Attempting to implement from current plan will result in:
- Wrong data structures created
- Code in wrong locations
- Type safety violations
- Failed quality gates
- Architectural drift

**Better approach:**
1. Spend 6 hours revising the plan per recommendations above
2. Get AI agent review of revised plan (should be 95% ready)
3. Begin implementation with high confidence of success

---

## Appendix: Template for Revised Phase Structure

```markdown
### Phase X: [Feature Name]

**Estimated Time:** X-Y hours
**Blocks:** Phase A, Phase B
**Blocked By:** Phase C (reason)

---

#### X.0 MANDATORY: Architecture Compliance

**Read these sections BEFORE implementation:**

| Section | Lines | Topic | Use For |
|---------|-------|-------|---------|
| Architecture 4.X | LXXX-YYY | [StructName] | Exact struct definition |
| Architecture 4.Y | LXXX-YYY | [EnumName] | Exact enum definition |
| Architecture 5.Z | LXXX-YYY | [function_logic] | Algorithm to follow |

**Type Aliases (MANDATORY):**
- Use `TypeId` (NOT `u32`) from Section 4.6 LXXX
- Use `OtherId` (NOT `usize`) from Section 4.6 LYYY

**Constants (MANDATORY):**
- Use `CONSTANT_NAME` (NOT magic number) from Section 4.3 LXXX

**Verification:**
```bash
# Must return ZERO matches
grep "field_name: u32" src/path/to/new_file.rs
````

---

#### X.1 Game Context

**What the player experiences:**
[User-facing description]

**Game mechanics to understand:**

- Mechanic 1: [Explanation with example]
- Mechanic 2: [Explanation with example]

**Edge cases:**

- Edge case 1: [How to handle]
- Edge case 2: [How to handle]

---

#### X.2 Module Structure

**Files to CREATE:**

1. **File:** `src/layer/module/new_file.rs`
   - **Layer:** [domain/application/game] because [reason]
   - **Add SPDX header:**

     ```rust
     // SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
     // SPDX-License-Identifier: Apache-2.0

     //! [Module description]
     ```

**Files to MODIFY:**

2. **File:** `src/layer/module/existing.rs`

   - **Modify:** Lines XXX-YYY (add new function)
   - **Read first:** Lines 1-100 (understand context)

3. **File:** `src/layer/module/mod.rs`
   - **Add:** `pub mod new_file;` at line ~XX

---

#### X.3 Implementation Tasks

**Task X.3.1: [Description]** (Est: Xh)

- **File:** `src/path/to/file.rs` (CREATE at ~150 lines)
- **What:** Implement [StructName] from architecture Section 4.X LXXX-YYY
- **Use:** Type alias `TypeId`, constant `CONSTANT_NAME`
- **Dependencies:** None / Task X.3.Y must complete first
- **Code structure:**
  ```rust
  // High-level pseudo-code showing structure (NOT exact code to copy)
  pub struct StructName {
      field: TypeId,  // Note: TypeId, not u32
  }
  ```

**Task X.3.2: [Description]** (Est: Yh)

- **File:** `src/path/to/file.rs` (MODIFY at lines XXX-YYY)
- **What:** Add function `pub fn function_name()`
- **Algorithm:** Follow architecture Section 5.X LXXX-YYY logic
- **Error handling:** Return `Result<T, ErrorType>` using thiserror
- **Dependencies:** Requires Task X.3.1

---

#### X.4 Integration Points

**Integrates with:**

1. **Existing System:** [SystemName]

   - **File:** `src/path/to/system.rs`
   - **Read first:** Lines XXX-YYY
   - **Integration point:** Add `NewType` to `existing_function()` at line ZZZ
   - **Pattern to follow:** Similar to existing integration at line AAA

2. **Existing Resource:** [ResourceName]
   - **File:** `src/path/to/resource.rs`
   - **Use:** `resource.field` for data access
   - **Do NOT:** Modify resource structure (read-only)

---

#### X.5 Testing Requirements

**Unit Tests (file: `src/path/to/file.rs`):**

Add `#[cfg(test)] mod tests` section at end of file:

1. `test_function_name_success_case` - [What it tests]
2. `test_function_name_failure_case` - [What it tests]
3. `test_function_name_edge_case` - [What it tests]

**Minimum:** 3 tests per public function

**Integration Tests (file: `tests/integration/feature.rs`):**

Create if doesn't exist:

4. `test_full_feature_workflow` - [End-to-end scenario]

**Manual Verification:**

5. Run game, perform action: [Specific steps]
6. Expected result: [What should happen]

**Coverage Target:** >80% of new code

**Verification:**

```bash
cargo nextest run test_function_name
# Expected: X passed, 0 failed
```

---

#### X.6 Quality Gates

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

#### X.7 Documentation Updates

**YOU MUST UPDATE:**

- `docs/explanation/implementations.md`

**Add this section:**

```markdown
## Phase X: [Feature Name] - [Date]

### What Was Built

- [Component]: [Description]

### Files Modified/Created

- `src/path/file.rs` (NEW/MODIFIED)

### Architecture Compliance

- ✅ Used [TypeAlias] from Section 4.X
- ✅ Followed [StructName] from Section 4.Y LXXX-YYY
- ✅ Used constant [CONSTANT] from Section 4.Z

### Quality Gates

- ✅ All gates passed

### Testing

- X new tests, all passing
- Coverage: Y%
```

**DO NOT MODIFY:**

- `docs/reference/architecture.md`
- `README.md`
- Any other files without permission

---

#### X.8 Deliverables

Verify each checkbox with provided command:

- [ ] `src/path/new_file.rs` created with SPDX header
  ```bash
  head -n 2 src/path/new_file.rs | grep -q "SPDX-FileCopyrightText"
  ```
- [ ] `src/path/existing.rs` modified (lines XXX-YYY)
  ```bash
  git diff src/path/existing.rs | grep -A5 "function_name"
  ```
- [ ] X unit tests added and passing
  ```bash
  cargo nextest run test_feature_ --lib | grep "X passed"
  ```
- [ ] Integration test added
  ```bash
  cargo nextest run test_full_feature_workflow
  ```
- [ ] Documentation updated
  ```bash
  grep "Phase X:" docs/explanation/implementations.md
  ```
- [ ] All quality gates pass
  ```bash
  cargo fmt --all && cargo check --all-targets --all-features && \
  cargo clippy --all-targets --all-features -- -D warnings && \
  cargo nextest run --all-features
  ```

---

#### X.9 Success Criteria

**Automatically Verifiable:**

- [ ] Zero clippy warnings: `cargo clippy 2>&1 | grep -c "warning:" # = 0`
- [ ] Zero test failures: `cargo nextest run | grep "0 failed"`
- [ ] Type aliases used: `grep -c "field: u32" src/path/file.rs # = 0`
- [ ] SPDX header present: `grep -q SPDX src/path/file.rs`

**Manually Verifiable:**

- [ ] Feature works in-game: [Specific test case]
- [ ] No regressions: [Existing features still work]
- [ ] Documentation complete: [Review implementations.md entry]

---

**END OF PHASE X**

```

---

**Review Complete.**
```
