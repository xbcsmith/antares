# AGENTS.md - AI Agent Development Guidelines

**CRITICAL**: This file contains mandatory rules for AI agents working on antares.
Non-compliance will result in rejected code.

---

## Quick Reference for AI Agents

### BEFORE YOU START ANY TASK

#### Step 1: Verify Tools Are Installed

```bash
rustup component add clippy rustfmt
cargo install cargo-audit  # Optional but recommended
```

#### Step 2: MANDATORY - Consult Architecture Document FIRST

**Before writing ANY code, you MUST:**

1. **Read** `docs/reference/architecture.md` sections relevant to your task
2. **Verify** data structures match the architecture EXACTLY
3. **Check** module placement (Section 3.2) - don't create new modules arbitrarily
4. **Confirm** you're working in the correct Development Phase (Section 8)
5. **Use** the exact type names, field names, and signatures defined in architecture
6. **NEVER** modify core data structures (Section 4) without explicit approval

**Rule**: If architecture.md defines it, YOU MUST USE IT EXACTLY AS DEFINED.
Deviation = violation.

#### Step 3: Plan Your Implementation

- Identify which files need changes
- Determine what tests are needed
- Choose correct documentation category (Diataxis)

### AFTER YOU COMPLETE ANY TASK

#### Step 1: Run Quality Checks (ALL MUST PASS)

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

**Expected**: Zero errors, zero warnings, all tests pass.

#### Step 2: Verify Architecture Compliance

- [ ] Data structures match architecture.md Section 4 definitions **EXACTLY**
- [ ] Module placement follows Section 3.2 structure
- [ ] Type aliases used consistently (ItemId, SpellId, etc.)
- [ ] Constants extracted, not hardcoded (MAX_ITEMS, condition flags, etc.)
- [ ] AttributePair pattern used for modifiable stats
- [ ] Game mode context respected (combat vs exploration logic)
- [ ] RON format used for data files, not JSON/YAML
- [ ] No architectural deviations without documentation

#### Step 3: Final Verification

1. **Re-read** relevant architecture.md sections
2. **Confirm** no architectural drift introduced
3. **Update** `docs/explanation/implementations.md`

**If you can't explain WHY your code differs from architecture.md, IT'S WRONG.**

**IF ANY CHECK FAILS, YOU MUST FIX IT BEFORE PROCEEDING.**
</parameter>

---

## IMPLEMENTATION RULES - NEVER VIOLATE

**Detailed rules for implementing code. See "Five Golden Rules" section at end for quick reference.**

### Implementation Rule 1: File Extensions (MOST VIOLATED)

**YOU WILL GET THIS WRONG IF YOU DON'T READ CAREFULLY**

#### Real Files vs. Documentation

- **Real implementation files**: `src/**/*.rs` - actual code that compiles
- **Documentation files**: `docs/**/*.md` - explanations, references, guides
- **Data files**: Use `.ron` for game data (items, spells, monsters, maps)

#### The Test: "Is this code going to be executed?"

**YES - It's real code:**

- ✓ Save as `.rs` in `src/` directory
- ✓ Must compile with `cargo check`
- ✓ Must pass all quality gates

**NO - It's documentation/example:**

- ✓ Keep in `.md` file with proper code blocks
- ✓ Use path annotation: ```path/to/file.rs#L1-10
- ✓ Mark as pseudo-code if not compilable

#### Data File Extensions

Per architecture.md Section 7.1:

- **Items**: `data/items.ron` (NOT .json, NOT .yaml)
- **Spells**: `data/spells.ron`
- **Monsters**: `data/monsters.ron`
- **Maps**: `data/maps/` (RON format)

**WRONG**: Creating `items.json`, `spells.yaml`, etc.
**RIGHT**: Using `.ron` format as specified in architecture

**Why this is violated**: Agents see code examples in architecture.md and think they need to create files for every struct. **NO**. Architecture defines the structure; you implement what's asked.

**YOU MUST:**

- Use `.rs` extension for ALL Rust implementation files
- Use `.md` extension for ALL Markdown files
- Use `.ron` extension for ALL game data files (items, spells, monsters, maps)
- Use `.yaml` extension ONLY for configuration files (if needed)

**NEVER:**

- ❌ Use `.yml` or `.yaml` for **game data** (items, spells, monsters, maps)
- ❌ Use `.json` for game data files
- ❌ Use `.MD` or `.markdown` extensions
- ❌ Create `.rs` files for code that only appears in architecture documentation

**Clarification**: YAML is acceptable for config files (e.g., CI/CD configs), but game content MUST use RON format.

### Implementation Rule 2: Markdown File Naming (SECOND MOST VIOLATED)

**YOU MUST:**

- Use lowercase letters ONLY
- Use underscores to separate words
- Exception: `README.md` is the ONLY uppercase filename allowed

**NEVER:**

- ❌ Use CamelCase (DistributedTracing.md)
- ❌ Use kebab-case (distributed-tracing.md)
- ❌ Use spaces (Distributed Tracing.md)
- ❌ Use uppercase (DISTRIBUTED_TRACING.md)

**Examples:**

```text
✅ CORRECT:
   docs/explanation/distributed_tracing_architecture.md
   docs/how_to/setup_monitoring.md
   docs/reference/api_specification.md
   README.md (ONLY exception)

❌ WRONG:
   docs/explanation/Distributed-Tracing-Architecture.md
   docs/explanation/DistributedTracingArchitecture.md
   docs/explanation/ARCHITECTURE.md
   docs/how_to/setup-monitoring.md
   docs/how_to/Setup Monitoring.md
```

**Why This Matters**: Inconsistent naming breaks documentation linking and makes
files hard to find.

### Implementation Rule 3: Code Quality Gates (MUST ALL PASS)

**Run these commands AFTER implementing your code (not before):**

```bash
# Run in this exact order:

# 1. Format (auto-fixes issues)
cargo fmt --all

# 2. Compile check (fast, no binary)
cargo check --all-targets --all-features

# 3. Lint (treats warnings as errors)
cargo clippy --all-targets --all-features -- -D warnings

# 4. Tests (must have >80% coverage)
cargo test --all-features
```

**Expected Results:**

```text
✅ cargo fmt         → No output (all files formatted)
✅ cargo check       → "Finished" with 0 errors
✅ cargo clippy      → "Finished" with 0 warnings
✅ cargo test        → "test result: ok. X passed; 0 failed"
```

**IF ANY FAIL**: Stop immediately and fix before proceeding.

**Note**: These are validation commands, not planning commands. Run AFTER writing code.

### Implementation Rule 4: Documentation is Mandatory

**YOU MUST:**

- Add `///` doc comments to EVERY public function, struct, enum, module
- Include runnable examples in doc comments (tested by `cargo test`)
- Update `docs/explanation/implementations.md` for EVERY feature/task

**DO NOT:**

- ❌ Create new documentation files without being asked
- ❌ Skip documentation because "code is self-documenting"
- ❌ Put documentation in wrong directory or use wrong filename format

**ONLY UPDATE THESE FILES unless explicitly instructed otherwise:**

- `docs/explanation/implementations.md` (your summary of what you built)
- Code comments (/// doc comments in .rs files)

**Examples:**

````rust
/// Checks if a character can equip a specific item
///
/// # Arguments
///
/// * `character` - The character attempting to equip the item
/// * `item` - The item to be equipped
///
/// # Returns
///
/// Returns `Ok(true)` if the character can equip the item
///
/// # Errors
///
/// Returns `EquipError::ClassRestriction` if character's class cannot use item
/// Returns `EquipError::NoSlotAvailable` if no equipment slot is available
///
/// # Examples
///
/// ```
/// use antares::character::{Character, Class, Race};
/// use antares::items::{Item, WeaponData};
///
/// let knight = Character::new("Sir Lancelot", Race::Human, Class::Knight);
/// let sword = Item::new_weapon("Longsword", WeaponData::default());
///
/// assert!(can_equip_item(&knight, &sword).is_ok());
/// ```
pub fn can_equip_item(character: &Character, item: &Item) -> Result<bool, EquipError> {
    // Check class restrictions
    if !item.disablements.can_use(character.class, character.alignment) {
        return Err(EquipError::ClassRestriction);
    }

    // Check if equipment slots are available
    if !character.equipment.has_slot_for(&item.item_type) {
        return Err(EquipError::NoSlotAvailable);
    }

    Ok(true)
}
````

## Project Overview

### Identity

- **Name**: antares
- **Type**: Antares, a classic turn-based RPG built in Rust.
- **Language**: Rust (latest stable)
- **Key Features**:
  - **Separation of Concerns**: Clear boundaries between game logic, rendering, and I/O
  - **Data-Driven Design**: Game content defined in external data files
  - **Entity-Component Pattern**: Flexible character and monster representation
  - **Deterministic Gameplay**: Pure functions for game logic, making save/load trivial

## Game-Specific Context (READ THIS)

### This is NOT a Generic Rust Project

Antares is a **turn-based RPG** inspired by Might and Magic 1. Understanding the game mechanics is **mandatory** for proper implementation.

### Core Game Concepts You Must Understand

1. **Game Modes** (see `GameMode` enum in architecture):

   - `Exploration`: Party moves through maps
   - `Combat`: Turn-based tactical battles
   - `Menu`: Character management, inventory
   - `Dialogue`: NPC interactions

   **Implication**: State management differs by mode. Don't make assumptions.

2. **Party vs. Roster**:

   - **Roster**: All characters created (up to 20+ stored at inns)
   - **Party**: Active adventuring group (max 6 members)

   **Implication**: Functions that affect "party" ≠ functions that affect "roster"

3. **Resource Systems** (all managed at party level):

   - Gold, Gems, Food - shared resources
   - Light Units - decreases in dark areas
   - Spell Points (SP) - per character
   - Hit Points (HP) - per character

   **Implication**: Inventory operations are complex; consult architecture Section 4

4. **Stat Modifiers** (AttributePair pattern):

   - Every stat has `base` and `current` values
   - Equipment/spells modify `current`, not `base`
   - Resetting restores `current` to `base`

   **Implication**: Never modify stats directly; use the modifier system

5. **Combat Mechanics**:

   - Turn order determined at combat start
   - Handicap system (party/monster advantage)
   - Monsters can regenerate, advance, have special attacks
   - Conditions affect action availability

   **Implication**: Combat state is complex; small changes have ripple effects

### Type System Adherence

The architecture defines specific type aliases (Section 4.6):

- `ItemId`, `SpellId`, `MonsterId`, `MapId`, `CharacterId`, `TownId`, `EventId`

**ALWAYS** use these instead of raw `u32` or `usize`. This isn't optional.

### Constants and Magic Numbers

Architecture defines many constants:

- `Inventory::MAX_ITEMS = 20`
- `Equipment::MAX_EQUIPPED = 7`
- Various condition flags, disablement flags

**NEVER** hardcode these values. Reference the constants or extract them if missing.

## Development Workflow

### Step-by-Step Process (FOLLOW EXACTLY)

#### Phase 1: Preparation

1. **Understand the Task**

   - Read requirements completely
   - Identify which architecture layers are affected
   - Check for existing similar code

2. **Search Existing Code**

   ```bash
   # Find relevant files
   grep -r "function_name" src/
   find src/ -name "*feature*.rs"
   ```

3. **Plan Changes**
   - List files to create/modify
   - Identify tests needed
   - Determine documentation category

#### Phase 2: Implementation

1. **Write Code**

   ````rust
   // Follow this pattern for ALL public items:

   /// One-line description
   ///
   /// Longer explanation of behavior and purpose.
   ///
   /// # Arguments
   ///
   /// * `param` - Description
   ///
   /// # Returns
   ///
   /// Description of return value
   ///
   /// # Errors
   ///
   /// Returns `ErrorType` if condition
   ///
   /// # Examples
   ///
   /// ```
   /// use antares::character::{Character, Class, Race};
   /// use antares::items::{Item, ItemId};
   ///
   /// let character = Character::new("Hero", Race::Human, Class::Knight);
   /// let item_id: ItemId = 42;
   /// let result = can_use_item(&character, item_id);
   /// assert!(result.is_ok());
   /// ```
   pub fn can_use_item(character: &Character, item_id: ItemId) -> Result<bool, GameError> {
       // Implementation
   }
   ````

2. **Write Tests (MANDATORY)**

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use crate::character::{Character, Class, Race};
       use crate::items::{Item, ItemType, Disablement};

       #[test]
       fn test_knight_can_equip_sword() {
           // Arrange
           let knight = Character::new("Test Knight", Race::Human, Class::Knight);
           let sword = Item::new_weapon("Longsword", WeaponData::default());

           // Act
           let result = can_equip_item(&knight, &sword);

           // Assert
           assert!(result.is_ok());
           assert_eq!(result.unwrap(), true);
       }

       #[test]
       fn test_sorcerer_cannot_equip_plate_armor() {
           let sorcerer = Character::new("Mage", Race::Elf, Class::Sorcerer);
           let plate = Item::new_armor("Plate Mail", ArmorData::heavy());

           let result = can_equip_item(&sorcerer, &plate);
           assert!(result.is_err());
       }

       #[test]
       fn test_inventory_full_boundary() {
           let mut character = Character::new("Test", Race::Human, Class::Knight);
           // Fill inventory to MAX_ITEMS
           for i in 0..Inventory::MAX_ITEMS {
               character.inventory.add_item(ItemId::from(i)).unwrap();
           }

           // Attempt to add one more
           let result = character.inventory.add_item(ItemId::from(99));
           assert!(matches!(result, Err(InventoryError::Full)));
       }
   }
   ```

3. **Run Quality Checks Incrementally**

   ```bash
   # After writing code
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings

   # After writing tests
   cargo test --all-features

   # Before committing - verify all checks pass
   cargo fmt --all
   cargo check --all-targets --all-features
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-features
   ```

#### Phase 3: Documentation

**ONLY UPDATE THIS FILE**

- docs/explanation/implementations.md

**NEVER modify architecture.md - it is the source of truth that you follow, not update.**

### Phase 4: Validation (CRITICAL)

**Run the quality checks from Implementation Rule 3 (see above) or "AFTER YOU COMPLETE" section.**

All four cargo commands MUST pass:

- `cargo fmt --all`
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`

**THEN** verify the complete "Validation Checklist" section near the end of this document.

**IF ANY VALIDATION FAILS: Stop and fix immediately.**

---

## Rust Coding Standards

### Error Handling (MANDATORY PATTERNS)

**YOU MUST:**

- Use `Result<T, E>` for ALL recoverable errors
- Use `?` operator for error propagation
- Use `thiserror` for custom error types
- Use descriptive error messages

**NEVER:**

- ❌ Use `unwrap()` without justification
- ❌ Use `expect()` without descriptive message
- ❌ Ignore errors with `let _ =`
- ❌ Return `panic!` for recoverable errors

**Correct Patterns:**

```rust
// ✅ GOOD - Proper error handling
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(String),

    #[error("Invalid YAML syntax: {0}")]
    ParseError(String),
}

pub fn load_config(path: &str) -> Result<Config, ConfigError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::ReadError(e.to_string()))?;

    let config: Config = serde_yaml::from_str(&contents)
        .map_err(|e| ConfigError::ParseError(e.to_string()))?;

    config.validate()?;
    Ok(config)
}

// ❌ BAD - Using unwrap
pub fn load_config(path: &str) -> Config {
    let contents = std::fs::read_to_string(path).unwrap(); // NEVER
    serde_yaml::from_str(&contents).unwrap() // NEVER
}

// ⚠️ ACCEPTABLE - unwrap with justification
pub fn get_app_version() -> String {
    // SAFETY: This is set at compile time and cannot fail
    env!("CARGO_PKG_VERSION").to_string()
}
```

### Testing Standards (MANDATORY)

#### Game State Testing Requirements

Turn-based RPG state is complex. Your tests MUST cover:

**1. State Transition Tests**

```antares/examples/state_test_example.rs#L1-15
#[test]
fn test_combat_to_exploration_preserves_party_state() {
    let mut game_state = GameState::new();
    game_state.mode = GameMode::Combat;

    // Modify party during combat
    game_state.party.members[0].hp.current = 10;

    // Transition to exploration
    game_state.end_combat();

    // Verify state preservation
    assert_eq!(game_state.mode, GameMode::Exploration);
    assert_eq!(game_state.party.members[0].hp.current, 10);
}
```

**2. Resource Management Tests**: Test gold, gems, food, light consumption and sharing.

**3. Modifier System Tests**: Test that equipment/spells modify `current` but preserve `base` values.

**4. Condition Interaction Tests**: Test that conditions correctly prevent/allow actions (silenced can't cast, paralyzed can't act, etc.).

**5. Boundary Tests for Game Limits**:

- Max party size (6)
- Max inventory (20 items per character)
- Max equipped items (7 slots)
- Stat ranges (0-255 for most)

**Don't write generic tests. Write tests that exercise actual game mechanics.**

#### Standard Testing Requirements

**YOU MUST:**

- Write tests for ALL public functions
- Test both success and failure cases
- Test edge cases and boundaries
- Achieve >80% code coverage
- Use descriptive test names: `test_{function}_{condition}_{expected}`

**Test Structure Template:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Success case
    #[test]
    fn test_parse_config_with_valid_yaml() {
        let yaml = "key: value";
        let result = parse_config(yaml);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().key, "value");
    }

    // Failure case
    #[test]
    fn test_parse_config_with_invalid_yaml() {
        let yaml = "invalid: : yaml";
        let result = parse_config(yaml);
        assert!(result.is_err());
    }

    // Edge case
    #[test]
    fn test_parse_config_with_empty_string() {
        let result = parse_config("");
        assert!(result.is_err());
    }

    // Boundary condition
    #[test]
    fn test_parse_config_with_max_size() {
        let yaml = "x".repeat(MAX_CONFIG_SIZE);
        let result = parse_config(&yaml);
        assert!(result.is_ok());
    }

    // Error propagation
    #[test]
    fn test_parse_config_propagates_validation_error() {
        let yaml = "invalid_field: value";
        let result = parse_config(yaml);
        assert!(matches!(result, Err(ConfigError::ValidationError(_))));
    }
}
```

---

## Git Rules

**LEAVE ALL GIT OPERATIONS TO THE USER**

**DO NOT CREATE BRANCHES**

**DO NOT COMMIT CODE**

---

## Documentation Organization (Diataxis Framework)

**YOU MUST categorize documentation correctly:**

**CRITICAL**: Game data files use **RON format** (.ron), not JSON or YAML.
See architecture.md Section 7.2 for examples.

If you create data files in wrong format, they WILL be rejected by cargo check.

### Category 1: Tutorials (`docs/tutorials/`)

**Purpose**: Learning-oriented, step-by-step lessons

**Use for**:

- Getting started guides
- Learning path tutorials
- Hands-on examples

**Example**: `docs/tutorials/getting_started.md`

### Category 2: How-To Guides (`docs/how_to/`)

**Purpose**: Task-oriented, problem-solving recipes

**Use for**:

- Installation steps
- Configuration guides
- Troubleshooting procedures

**Example**: `docs/how_to/setup_monitoring.md`

### Category 3: Explanations (`docs/explanation/`) ← DEFAULT FOR YOUR SUMMARIES

**Purpose**: Understanding-oriented, conceptual discussion

**Use for**:

- Architecture explanations
- Design decisions
- Implementation summaries ← **YOU TYPICALLY CREATE THESE**
- Concept clarifications

**Example**: `docs/explanation/phase4_observability_implementation.md`

### Category 4: Reference (`docs/reference/`)

**Purpose**: Information-oriented, technical specifications

**Use for**:

- API documentation
- Configuration reference
- Command reference

**Example**: `docs/reference/api_specification.md`

### Decision Tree: Where to Put Documentation?

```text
Is it a step-by-step tutorial?
├─ YES → docs/tutorials/
└─ NO
   ├─ Is it solving a specific task?
   │  ├─ YES → docs/how_to/
   │  └─ NO
   │     ├─ Is it explaining concepts/architecture?
   │     │  ├─ YES → docs/explanation/  ← MOST COMMON FOR AI AGENTS
   │     │  └─ NO
   │     │     └─ Is it reference material?
   │     │        └─ YES → docs/reference/
```

---

## Common Violation Patterns (LEARN FROM PAST MISTAKES)

### ❌ Anti-Pattern 1: Modifying Core Data Structures

**WRONG**:

```antares/examples/wrong_struct.rs#L1-5
pub struct Character {
    pub name: String,
    pub level: u8,
    pub experience: u32,
    pub my_new_field: bool,  // ❌ UNAUTHORIZED CHANGE
}
```

**RIGHT**: Core structs from architecture.md are **IMMUTABLE** without approval.

---

### ❌ Anti-Pattern 2: Using Raw Types Instead of Aliases

**WRONG**:

```antares/examples/wrong_types.rs#L1-3
pub fn get_item(id: u32) -> Option<Item> {  // ❌ Should use ItemId
    // ...
}
```

**RIGHT**:

```antares/examples/right_types.rs#L1-3
pub fn get_item(id: ItemId) -> Option<Item> {  // ✓ Uses type alias
    // ...
}
```

---

### ❌ Anti-Pattern 3: Hardcoding Constants

**WRONG**:

```antares/examples/wrong_constant.rs#L1-3
if inventory.items.len() >= 20 {  // ❌ Magic number
    return Err(InventoryError::Full);
}
```

**RIGHT**:

```antares/examples/right_constant.rs#L1-3
if inventory.items.len() >= Inventory::MAX_ITEMS {  // ✓ Uses constant
    return Err(InventoryError::Full);
}
```

---

### ❌ Anti-Pattern 4: Direct Stat Modification

**WRONG**:

````antares/examples/wrong_stat.rs#L1-2
character.stats.might.current = 25;  // ❌ Direct modification
character.stats.might.base = 25;     // ❌ NEVER modify base directly
```

**RIGHT**:

```antares/examples/right_stat.rs#L1-2
character.stats.might.modify(5);  // ✓ Uses modifier system
character.stats.might.reset();    // ✓ Resets current to base
```

---

### ❌ Anti-Pattern 5: Creating New Modules Without Justification

**WRONG**: Creating `src/utils/`, `src/helpers/`, `src/common/` without checking Section 3.2.

**RIGHT**: Follow the module structure in architecture.md. Propose new modules in discussion.

---

### ❌ Anti-Pattern 6: Wrong Data Format

**WRONG**: Creating `data/items.json`, `config.yaml`

**RIGHT**: Use `.ron` format per Section 7.1 of architecture.md

---

## Emergency Procedures

### When Quality Checks Fail

**SYSTEMATIC DEBUG PROCESS:**

```bash
# Step 1: Fix formatting (always do this first)
cargo fmt --all

# Step 2: Fix compilation errors
cargo check --all-targets --all-features
# Read each error message
# Fix root cause, not symptoms
# Re-run after each fix

# Step 3: Fix clippy warnings (one at a time)
cargo clippy --all-targets --all-features -- -D warnings
# Fix first warning
# Re-run clippy
# Repeat until zero warnings

# Step 4: Fix failing tests
cargo test --all-features -- --nocapture
# Read test failure output
# Fix failing tests or update expectations
# Re-run tests

# Step 5: Verify all checks pass
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
````

### When Tests Fail

**DIAGNOSTIC COMMANDS:**

```bash
# Run with detailed output
cargo test -- --nocapture --test-threads=1

# Run specific test
cargo test test_name -- --nocapture

# Run tests in specific module
cargo test module::tests:: -- --nocapture

# Show backtrace on panic
RUST_BACKTRACE=1 cargo test

# Run with debug logging
RUST_LOG=debug cargo test
```

**DEBUGGING STRATEGY:**

1. Read the test failure message carefully
2. Understand what the test expects
3. Add `println!` or `dbg!` to see actual values
4. Fix the code or update the test
5. Re-run until passing

### When Clippy Reports Warnings

**FIXING PROCESS:**

```bash
# List all warnings
cargo clippy --all-targets --all-features 2>&1 | grep "warning:"

# Fix warnings by category:

# 1. Unused code
#    - Remove if truly unused
#    - Add #[allow(dead_code)] with justification if needed

# 2. Complexity warnings
#    - Refactor complex functions
#    - Extract helper functions

# 3. Style warnings
#    - Follow clippy suggestions
#    - Run cargo fix if available

# 4. Correctness warnings
#    - Fix immediately (these are bugs)

# Re-run after each fix
cargo clippy --all-targets --all-features -- -D warnings
```

---

## Validation Checklist

**BEFORE CLAIMING TASK IS COMPLETE, VERIFY ALL:**

### Code Quality

- [ ] `cargo fmt --all` applied successfully
- [ ] `cargo check --all-targets --all-features` passes with zero errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero
      warnings
- [ ] `cargo test --all-features` passes with >80% coverage
- [ ] No `unwrap()` or `expect()` without justification
- [ ] All public items have doc comments with examples
- [ ] All functions have at least 3 tests (success, failure, edge case)

### Testing

- [ ] Unit tests added for ALL new functions
- [ ] Integration tests added if needed
- [ ] Test count increased from before (verify with `cargo test --lib`)
- [ ] Both success and failure cases tested
- [ ] Edge cases and boundaries covered
- [ ] All tests use descriptive names: `test_{function}_{condition}_{expected}`

### Documentation

- [ ] Documentation files updated `docs/explanation/implementations.md`
- [ ] Filename uses lowercase_with_underscores.md
- [ ] README.md exception is ONLY uppercase filename
- [ ] No emojis anywhere in documentation
- [ ] All code blocks specify language (`rust, not`)
- [ ] Documentation includes: Overview, Components, Details, Testing, Examples
- [ ] Markdownlint passes (if configured)

### Files and Structure

- [ ] Rust files use `.rs` extension in `src/`
- [ ] Game data files use `.ron` extension (NOT `.json` or `.yaml`)
- [ ] Markdown files use `.md` extension in `docs/`
- [ ] No uppercase in filenames except `README.md`
- [ ] Files placed in correct architecture layer (Section 3.2)
- [ ] Documentation in correct Diataxis category

### Git

- [ ] Branch name follows `pr-<feat>-<issue>` format (lowercase)
- [ ] Commit message follows conventional commits
- [ ] Commit message includes JIRA issue in uppercase
- [ ] Commit message first line ≤72 characters
- [ ] Commit uses imperative mood ("add" not "added")

### Architecture

- [ ] Data structures match architecture.md Section 4 definitions **EXACTLY**
- [ ] Module placement follows Section 3.2 structure
- [ ] Type aliases used consistently (ItemId, SpellId, MonsterId, etc.)
- [ ] Constants extracted, not hardcoded (MAX_ITEMS, condition flags, etc.)
- [ ] AttributePair pattern used for modifiable stats
- [ ] Game mode context respected (combat vs exploration logic)
- [ ] RON format used for data files, not JSON/YAML
- [ ] No architectural deviations without documentation in `docs/explanation/`
- [ ] Changes respect layer boundaries
- [ ] Domain layer has no infrastructure dependencies
- [ ] Proper separation of concerns maintained
- [ ] No circular dependencies introduced

---

## Quick Command Reference

### Essential Cargo Commands

```bash
# Build and check
cargo build                                      # Debug build
cargo build --release                            # Optimized build
cargo check --all-targets --all-features         # Fast compile check

# Quality
cargo fmt --all                                  # Format all code
cargo fmt --all -- --check                       # Check formatting
cargo clippy --all-targets --all-features -- -D warnings  # Lint

# Testing
cargo test                                       # Run all tests
cargo test --lib                                 # Library tests only
cargo test --all-features                        # With all features
cargo test -- --nocapture                        # Show output
cargo test test_name                             # Specific test

# Documentation
cargo doc --open                                 # Generate and open docs
cargo doc --no-deps --open                       # Without dependencies

# Maintenance
cargo clean                                      # Remove build artifacts
cargo update                                     # Update dependencies
cargo tree                                       # Show dependency tree
cargo audit                                      # Security check
```

### Project-Specific Commands

```bash
# Quality validation workflow
cargo fmt --all                                  # Format code
cargo check --all-targets --all-features         # Check compilation
cargo clippy --all-targets --all-features -- -D warnings  # Lint
cargo test --all-features                        # Run tests

# Additional make commands (if available)
make test                                        # Run tests
make build                                       # Build project
make clean                                       # Clean artifacts

# Adding dependencies
cargo add <crate_name>                           # Add to Cargo.toml
cargo add <crate_name> --dev                     # Dev dependency
cargo add <crate_name> --features=<feature>      # With feature
```

---

## THE FIVE GOLDEN RULES - QUICK REFERENCE

**These supersede detailed rules above if there's any confusion. When in doubt, follow these five:**

**IF YOU REMEMBER ONLY FIVE THINGS:**

### Golden Rule 1: Consult Architecture First

**BEFORE writing code:**

- Read `docs/reference/architecture.md` relevant sections
- Use EXACT data structures, type aliases, and constants as defined
- NEVER modify core structs (Section 4) without approval

### Golden Rule 2: File Extensions & Formats

**For implementation:**

- `.rs` for Rust code in `src/`
- `.ron` for game data (items, spells, monsters, maps) - NOT .json or .yaml

**For documentation:**

- `.md` for all docs in `docs/`
- Use `lowercase_with_underscores.md` (exception: `README.md`)

### Golden Rule 3: Type System Adherence

**Always use:**

- Type aliases: `ItemId`, `SpellId`, `MonsterId`, `MapId`, etc. (not raw `u32`)
- Constants: `Inventory::MAX_ITEMS`, `Equipment::MAX_EQUIPPED` (not magic numbers)
- `AttributePair` pattern for modifiable stats (`base` + `current`)

### Golden Rule 4: Quality Checks

```text
All four cargo commands MUST pass before claiming done:
- cargo fmt --all
- cargo check --all-targets --all-features
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --all-features
```

---

## The Golden Workflow

**FOLLOW THIS SEQUENCE FOR EVERY TASK:**

```text
1. Read architecture.md sections relevant to your task
2. Verify data structures, types, and constants match architecture
3. Create branch: pr-<feat>-<issue>
4. Implement code with /// doc comments
5. Use type aliases (ItemId, SpellId, etc.) not raw types
6. Add tests (>80% coverage) with game-specific test cases
7. Run: cargo fmt --all
8. Run: cargo check --all-targets --all-features
9. Run: cargo clippy --all-targets --all-features -- -D warnings
10. Run: cargo test --all-features
11. Update: docs/explanation/implementations.md
12. Verify: No architectural deviations from architecture.md
13. Commit with proper format: <type>(<scope>): <description> (JIRA-ISSUE)
14. Verify: All checklist items above are checked
```

**IF YOU FOLLOW THIS WORKFLOW, YOUR CODE WILL BE ACCEPTED.**

**IF YOU SKIP STEPS OR VIOLATE RULES, YOUR CODE WILL BE REJECTED.**

---

## Living Document

This file is continuously updated as new patterns emerge. Last updated: 2024

**For AI Agents**: You are a master Rust developer. Follow these rules
precisely. Update the implementation summaries in `docs/explanation/implementations`.
