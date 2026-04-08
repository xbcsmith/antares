# AGENTS.md - AI Agent Development Guidelines

**CRITICAL**: This file contains mandatory rules for AI agents working on antares.
Non-compliance will result in rejected code.

---

## Quick Reference for AI Agents

### BEFORE YOU START ANY TASK

1. **Read** `docs/reference/architecture.md` sections relevant to your task
2. **Verify** data structures, type names, and field names match architecture EXACTLY — no deviations
3. **NEVER** modify core data structures (Section 4) without explicit approval

**Rule**: If architecture.md defines it, USE IT EXACTLY AS DEFINED. Deviation = violation.
**Rule**: BE CONSISTENT WITH NAMING CONVENTIONS AND STYLE GUIDELINES.
**Rule**: WE DO NOT CARE ABOUT BACKWARDS COMPATIBILITY RIGHT NOW.

### AFTER YOU COMPLETE ANY TASK

1. Run all four quality gates (see **Implementation Rule 3**). Zero errors, zero warnings.
2. Verify architecture compliance:
   - [ ] Data structures match architecture.md Section 4 **EXACTLY**
   - [ ] Module placement follows Section 3.2
   - [ ] Type aliases used consistently (ItemId, SpellId, etc.)
   - [ ] Constants extracted, not hardcoded (MAX_ITEMS, condition flags, etc.)
   - [ ] AttributePair pattern used for modifiable stats
   - [ ] Game mode context respected (combat vs exploration logic)
   - [ ] RON format used for data files, not JSON/YAML
   - [ ] No architectural deviations without documentation
3. Update `docs/explanation/implementations.md`

**If you can't explain WHY your code differs from architecture.md, IT'S WRONG.**

**IF ANY CHECK FAILS, YOU MUST FIX IT BEFORE PROCEEDING.**

---

## IMPLEMENTATION RULES - NEVER VIOLATE

**Detailed rules for implementing code. See the Golden Workflow at the end of this document.**

### Implementation Rule 1: File Extensions (MOST VIOLATED)

**YOU WILL GET THIS WRONG IF YOU DON'T READ CAREFULLY**

#### Real Files vs. Documentation

- **Real implementation files**: `src/**/*.rs` - actual code that compiles
- **Documentation files**: `docs/**/*.md` - explanations, references, guides
- **Data files**: Use `.ron` for game data (items, spells, monsters, maps)

#### Copyright and License

Add SPDX FileCopyrightText and License-Identifier as the first lines
in ALL **Real implementation files**: `src/**/*.rs`

```rust
// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

```

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
   docs/how-to/setup_monitoring.md
   docs/reference/api_specification.md
   README.md (ONLY exception)

❌ WRONG:
   docs/explanation/Distributed-Tracing-Architecture.md
   docs/explanation/DistributedTracingArchitecture.md
   docs/explanation/ARCHITECTURE.md
   docs/how-to/setup-monitoring.md
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
cargo nextest run --all-features
```

**Expected Results:**

```text
✅ cargo fmt         → No output (all files formatted)
✅ cargo check       → "Finished" with 0 errors
✅ cargo clippy      → "Finished" with 0 warnings
✅ cargo nextest run        → "test result: ok. X passed; 0 failed"
```

**IF ANY FAIL**: Stop immediately and fix before proceeding.

**Note**: These are validation commands, not planning commands. Run AFTER writing code.

### Implementation Rule 4: Documentation is Mandatory

**YOU MUST:**

- Add `///` doc comments to EVERY public function, struct, enum, module
- Include runnable examples in doc comments (tested by `cargo nextest run`)
- Update `docs/explanation/implementations.md` for EVERY feature/task

**DO NOT:**

- ❌ Create new documentation files without being asked
- X Create validation reports or summaries without being asked
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

- `ItemId`, `SpellId`, `MonsterId`, `MapId`, `CharacterId`, `InnkeeperId`, `EventId`

**ALWAYS** use these instead of raw `u32` or `usize`. This isn't optional.

### Constants and Magic Numbers

Architecture defines many constants:

- `Inventory::MAX_ITEMS = 20`
- `Equipment::MAX_EQUIPPED = 7`
- Various condition flags, disablement flags

**NEVER** hardcode these values. Reference the constants or extract them if missing.

---

## Rust Coding Standards

In Rust: 1000 lines is overkill. Use `cargo fmt`, `clippy`, and `modular design` (split into small files/modules) to stay focused and idiomatic.

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
- Max inventory (64 items per character — `Inventory::MAX_ITEMS`)
- Max equipped items (7 slots — `Equipment::MAX_EQUIPPED`)
- Stat ranges (0-255 for most)

**Don't write generic tests. Write tests that exercise actual game mechanics.**

#### Standard Testing Requirements

**YOU MUST:**

- Write tests for ALL public functions
- Test both success and failure cases
- Test edge cases and boundaries
- Achieve >80% code coverage
- Use descriptive test names: `test_{function}_{condition}_{expected}`
- doctests are updated anytime the function signature or behavior changes

---

### Implementation Rule 5: Test Data Must Live in `data/`, Never in `campaigns/tutorial`

**THIS IS THE THIRD MOST VIOLATED RULE**

#### The Rule

All test fixtures and test data files MUST be placed under `data/` (the
stable, repo-managed test fixture directory). Tests MUST NOT reference
`campaigns/tutorial` or any other path under `campaigns/`.

`campaigns/tutorial` is the **live game campaign**. Its content changes as the
game is developed. Tests that depend on it become brittle — they break whenever
campaign data is edited, and they can introduce RON parse errors or missing
files that silently skip or hard-fail entire test suites.

#### Where Test Data Lives

```text
data/                         ← stable fixture root (immutable during tests)
  characters.ron              ← shared character definitions used by unit tests
  classes.ron
  items.ron
  ...
  test_campaign/              ← self-contained campaign fixture for integration tests
    campaign.ron
    config.ron                ← must include ALL ControlsConfig keys (e.g. inventory)
    data/
      characters.ron
      classes.ron
      conditions.ron
      creatures.ron
      dialogues.ron
      items.ron
      monsters.ron
      npc_stock_templates.ron ← MUST exist if any test loads npc stock from campaign
      npcs.ron
      proficiencies.ron
      quests.ron
      races.ron
      spells.ron
      maps/
        map_1.ron ... map_N.ron
```

#### YOU MUST:

- Point ALL tests that load campaign content at `data/test_campaign`
- Use `CampaignLoader::new("data")` with id `"test_campaign"` in loader-based tests
- Use `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/test_campaign")` in
  packager / path-based tests
- Add any missing data files to `data/test_campaign/data/` rather than
  borrowing them from `campaigns/tutorial`
- Keep `data/test_campaign` fully self-contained: every file referenced by
  `campaign.ron` MUST exist under `data/test_campaign/`

#### NEVER:

- ❌ Write a test that hard-codes `"campaigns/tutorial"` or any subpath of it
- ❌ Write a test that uses `CampaignLoader::new("campaigns")` with id `"tutorial"`
- ❌ Write a test that joins `env!("CARGO_MANIFEST_DIR")` with `"campaigns/tutorial"`
- ❌ Copy a file from `campaigns/tutorial/` into a test instead of using the fixture
- ❌ Skip a test with an early-return guard that checks whether `campaigns/tutorial`
  exists (this hides failures on clean checkouts)

#### The Only Legitimate Uses of `campaigns/tutorial` in Source Code

| Location                                              | Reason                                          |
| ----------------------------------------------------- | ----------------------------------------------- |
| `src/bin/antares.rs` default branch                   | Runtime game binary default campaign            |
| `src/game/systems/campaign_loading.rs` startup system | Bevy startup loads live campaign                |
| `src/bin/update_tutorial_maps.rs`                     | Maintenance tool that operates on tutorial maps |
| `docs/` and `sdk/` doc comments / examples            | Illustrative path strings, not file I/O         |

Everything else MUST use `data/test_campaign`.

**Examples:**

```text
✅ CORRECT — integration test:
   let loader = CampaignLoader::new("data");
   let campaign = loader.load_campaign("test_campaign")?;

✅ CORRECT — path-based test:
   let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/test_campaign");

✅ CORRECT — config test:
   let cfg = std::path::Path::new(manifest_dir).join("data/test_campaign/config.ron");

❌ WRONG — any of the above with "campaigns/tutorial" instead
```

**Why This Matters**: A single typo in `campaigns/tutorial/data/characters.ron`
(e.g. a trailing `.` instead of `,`) broke two previously-passing SDK tests
and caused a confusing failure that had nothing to do with the code under test.
Fixture data is reviewed and stable; live campaign data is not.

---

### Implementation Rule 6: egui Multi-Column Layouts Must Use `allocate_ui` with Explicit Column Rects

**THIS IS THE FOURTH MOST VIOLATED RULE — every new egui screen with columns gets this wrong**

#### The Three Traps (all discovered the hard way on the Spell Book screen)

**Trap 1 — `auto_shrink = true` (egui default) collapses ScrollArea to content height.**
`ScrollArea::vertical()` defaults to `auto_shrink = true`. With 3 spell rows the
area is ~60 px tall regardless of window size. Setting `max_height` alone does NOT
fix this — `max_height` is a ceiling, not a floor.

**Trap 2 — `ui.available_height()` inside `ui.horizontal` returns line height (~20 px).**
Inside a `left_to_right` layout, `available_height()` reports only the line height,
not the remaining window height. Any `max_height` computed there is useless.

**Trap 3 — `auto_shrink([false, false])` consumes all horizontal space.**
Disabling x-shrink inside `ui.horizontal` causes the `ScrollArea` to fill the
entire remaining width, pushing subsequent columns (e.g. the Detail panel) off
screen entirely. You will lose whole columns with zero compiler warning.

#### The Correct Pattern — `allocate_ui` with explicit column rects

Pre-compute ALL column dimensions from `ui.available_size()` **before** entering
`ui.horizontal`, then use `ui.allocate_ui(egui::vec2(width, col_h), …)` for each
column. This guarantees every column gets the exact rect you specify:

```antares/src/game/systems/right_multi_column.rs#L1-32
// ── Title bar with hints right-aligned (no bottom bar needed) ────────────────
ui.horizontal(|ui| {
    ui.heading("📚 Spell Book");
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label("[ESC] Close");
        ui.separator();
        ui.label("[↑↓] Select Spell");
        ui.separator();
        ui.label("[Tab] Switch Char");
        ui.separator();
        ui.label("[C] Cast Spell");
    });
});
ui.separator();

// Pre-compute geometry from available_size() BEFORE ui.horizontal.
let available = ui.available_size();
let col_h   = available.y;           // full remaining height
let left_w  = 160.0_f32;
let right_w = 220.0_f32;
// 2 separators: 1 px line + item_spacing.x each side, times two.
let sep_total = (1.0 + 2.0 * ui.spacing().item_spacing.x) * 2.0;
let center_w = (available.x - left_w - right_w - sep_total).max(200.0);

ui.horizontal(|ui| {
    // Left column — allocate_ui gives it an exact (left_w × col_h) rect.
    ui.allocate_ui(egui::vec2(left_w, col_h), |ui| {
        render_left_column(ui, ...);
    });
    ui.separator();

    // Center column — fills remaining width between left and right.
    ui.allocate_ui(egui::vec2(center_w, col_h), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("screen_center_scroll")
            .auto_shrink([true, false])  // shrink x to content; never shrink y
            .show(ui, |ui| { render_center_column(ui, ...); });
    });
    ui.separator();

    // Right column — fixed width, always visible.
    ui.allocate_ui(egui::vec2(right_w, col_h), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("screen_right_scroll")
            .auto_shrink([true, false])  // shrink x to content; never shrink y
            .show(ui, |ui| { render_right_column(ui, ...); });
    });
});
```

Key points:

- `ui.allocate_ui(vec2(w, col_h), …)` — the allocated rect is exactly `col_h`
  tall, so the inner `ScrollArea` sees `available_height() = col_h` and fills it.
- `.auto_shrink([true, false])` — `false` for y (don't shrink below `col_h`),
  `true` for x (don't consume extra horizontal space and break adjacent columns).
- Navigation hints go **in the title bar** (right-aligned). This eliminates the
  bottom bar and any height-reservation arithmetic.

#### The Hint Bar Pattern

**WRONG — separate bottom bar requires height calculation:**

```antares/src/game/systems/wrong_bottom_bar.rs#L1-8
ui.separator();
let available_h = ui.available_height() - 36.0; // fragile magic number
ui.horizontal(|ui| { /* columns */ });
ui.separator();
ui.horizontal_centered(|ui| {
    ui.label("[C] Cast   [Tab] Switch   [↑↓] Select");
});
```

**RIGHT — hints inline in title bar, columns own 100 % of remaining height:**

```antares/src/game/systems/right_title_hints.rs#L1-6
ui.horizontal(|ui| {
    ui.heading("Screen Title");
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label("[ESC] Close");
        ui.separator();
        ui.label("[Key] Action"); // add as many as needed
    });
});
ui.separator();
// Now ui.available_size().y is the full remaining height — no subtraction needed.
```

#### YOU MUST:

- Call `ui.available_size()` **before** `ui.horizontal` and store `col_h = available.y`
- Use `ui.allocate_ui(egui::vec2(col_width, col_h), …)` for **every** column
- Use `.auto_shrink([true, false])` on every `ScrollArea` inside a column
- Put navigation hints in the title bar row (right-aligned) — no separate bottom bar

#### NEVER:

- ❌ Use `ui.vertical(…)` + `ScrollArea` without `allocate_ui` — height is not propagated
- ❌ Use `auto_shrink([false, false])` — disabling x-shrink consumes all remaining width, destroying adjacent columns
- ❌ Call `ui.available_height()` inside `ui.horizontal` — returns line height (~20 px), not window height
- ❌ Use only `max_height(available_h)` without `allocate_ui` — `auto_shrink = true` still collapses the area
- ❌ Put navigation hints in a separate bottom bar — requires fragile height arithmetic

#### Reference Implementation

`src/game/systems/spellbook_ui.rs` — `spellbook_ui_system` is the canonical
example of this pattern in the game codebase.

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

### Category 2: How-To Guides (`docs/how-to/`)

**Purpose**: Task-oriented, problem-solving recipes

**Use for**:

- Installation steps
- Configuration guides
- Troubleshooting procedures

**Example**: `docs/how-to/setup_monitoring.md`

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
   │  ├─ YES → docs/how-to/
   │  └─ NO
   │     ├─ Is it explaining concepts/architecture?
   │     │  ├─ YES → docs/explanation/  ← MOST COMMON FOR AI AGENTS
   │     │  └─ NO
   │     │     └─ Is it reference material?
   │     │        └─ YES → docs/reference/
```

## The Golden Workflow

**FOLLOW THIS SEQUENCE FOR EVERY TASK:**

```text
1.  Read architecture.md sections relevant to your task
2.  Verify data structures, types, and constants match architecture
3.  Implement code with /// doc comments
4.  Use type aliases (ItemId, SpellId, etc.) not raw types
5.  Add tests (>80% coverage) with game-specific test cases
      - All test data/fixtures use data/test_campaign, NOT campaigns/tutorial
      - Add missing fixture files to data/test_campaign/data/ as needed
5a. (SDK / Campaign Builder UI only) Run the egui ID audit in sdk/AGENTS.md:
      every loop uses push_id, every ScrollArea has id_salt,
      every ComboBox uses from_id_salt,
      no SidePanel/TopBottomPanel/CentralPanel skipped by a same-frame guard
      (show a placeholder instead), every layout-driving state mutation calls
      request_repaint()
5b. (Any egui screen with multi-column layout) Verify column layout (Rule 6):
      - available_size() called BEFORE ui.horizontal; col_h = available.y
      - every column uses ui.allocate_ui(egui::vec2(col_w, col_h), ...)
      - every ScrollArea inside a column uses .auto_shrink([true, false])
      - navigation hints are in the title bar row, NOT a separate bottom bar
      - auto_shrink([false, false]) is NEVER used — it destroys adjacent columns
6.  Run: cargo fmt --all
7.  Run: cargo check --all-targets --all-features
8.  Run: cargo clippy --all-targets --all-features -- -D warnings
9.  Run: cargo nextest run --all-features
10. Update: docs/explanation/implementations.md
11. Verify: No architectural deviations from architecture.md
12. Verify: All checklist items above are checked
      - [ ] No test references campaigns/tutorial (Implementation Rule 5)
      - [ ] Any new campaign-level fixture data added to data/test_campaign/
      - [ ] Any egui multi-column screen: allocate_ui(vec2(col_w, col_h)) for every
            column; ScrollArea uses auto_shrink([true, false]); no bottom hint bar (Rule 6)
```

**IF YOU FOLLOW THIS WORKFLOW, YOUR CODE WILL BE ACCEPTED.**

**IF YOU SKIP STEPS OR VIOLATE RULES, YOUR CODE WILL BE REJECTED.**

---

## SDK-Specific Rules

Rules that apply only to code under `sdk/` are kept in `sdk/AGENTS.md` to
avoid bloating this file. Read that file in full before touching anything in
`sdk/campaign_builder`.

---

## Living Document

This file is continuously updated as new patterns emerge. Last updated: 2025

**For AI Agents**: You are a master Rust developer. Follow these rules
precisely. Update the implementation summaries in `docs/explanation/implementations`.
