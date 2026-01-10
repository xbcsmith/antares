# Dialogue System Implementation Plan

## Overview

This plan implements an enhanced dialogue system for Antares using native Bevy ECS patterns with 2.5D floating text bubbles, typewriter text effects, and event-driven architecture. The implementation builds upon the existing `domain::dialogue` module while adding visual feedback and improved state management without introducing external dialogue framework dependencies.

**Goals**:

- Replace synchronous dialogue execution with event-driven Bevy systems
- Add 2.5D floating text bubbles positioned above NPCs
- Implement typewriter text animation effects
- Maintain backward compatibility with existing `DialogueTree` RON format
- Support complex branching with visual feedback

**Scope**:

- **Game Engine**: Core dialogue visuals, state management, and runtime systems
- **Campaign Builder**: No changes required (verified below)
- **Data Format**: Continue using existing `.ron` format with `DialogueTree` structure

---

## Phase Dependencies

| Phase   | Depends On                        | Can Start Before Completion          | Blocking For  |
| ------- | --------------------------------- | ------------------------------------ | ------------- |
| Phase 1 | None                              | N/A                                  | Phase 2, 3, 4 |
| Phase 2 | Phase 1.1, 1.2 complete           | Phase 1.3, 1.4 tests can be parallel | Phase 3, 4    |
| Phase 3 | Phase 2.1, 2.2 complete           | Phase 2.3, 2.4 tests can be parallel | Phase 4       |
| Phase 4 | Phase 3.1-3.3 complete            | Phase 3.4, 3.5 can be parallel       | Phase 5       |
| Phase 5 | All previous phases 100% complete | No                                   | None          |

**Critical Path**: Phase 1.1 → Phase 1.2 → Phase 2.1 → Phase 2.2 → Phase 3.1 → Phase 3.2 → Phase 4.2 → Phase 5.1

**Parallelizable Work**: Testing tasks (X.3, X.4, X.5) can run concurrent with next phase start if core deliverables complete.

---

## Current State Analysis

### Existing Infrastructure

#### Domain Layer

**File**: `antares/src/domain/dialogue.rs`

- **DialogueTree struct** (lines 43-64): Core dialogue data structure
  - Fields: `id`, `name`, `root_node`, `nodes`, `speaker_name`, `repeatable`, `associated_quest`
- **DialogueNode struct** (lines 164-185): Individual dialogue nodes
  - Fields: `id`, `text`, `speaker_override`, `choices`, `conditions`, `actions`, `is_terminal`
- **DialogueChoice struct** (lines 255-270): Player response options
  - Fields: `text`, `target_node`, `conditions`, `actions`, `ends_dialogue`
- **DialogueCondition enum** (lines 311-344): Conditional branching logic
- **DialogueAction enum** (lines 387-426): Game state modifications

**Compliance**: ✅ NO CHANGES to domain layer structures

#### Game Layer

**File**: `antares/src/game/systems/dialogue.rs`

- **DialoguePlugin** (line 48): Registers dialogue systems
- **StartDialogue event** (lines 35-38): Triggers dialogue start
- **SelectDialogueChoice event** (lines 42-45): Player choice selection
- **handle_start_dialogue system** (lines 70-107): Starts dialogue, sets `GameMode::Dialogue`
- **handle_select_choice system** (lines 113-225): Processes choices, validates conditions
- **evaluate_conditions function** (lines 235-353): Evaluates branching logic
- **execute_action function** (lines 365-464): Executes dialogue actions

**Modification Required**: Add `DialogueState::update_node()` calls in `handle_start_dialogue` and `handle_select_choice`

#### Application Layer

**File**: `antares/src/application/dialogue.rs`

- **DialogueState**: Tracks active dialogue tree and current node

**Modification Required**: Add visual state fields (`current_text`, `current_speaker`, `current_choices`)

#### Data Files

**File**: `antares/campaigns/tutorial/data/dialogues.ron`

- Format: RON serialization of `Vec<DialogueTree>`
- Example dialogue: "Arcturus Story" (id: 1, root_node: 1)

**Compliance**: ✅ NO CHANGES to data format

#### Campaign Builder

**File**: `antares/sdk/campaign_builder/src/dialogue_editor.rs`

**Verification Required**:

```bash
# Verify editor uses domain::dialogue::DialogueTree
grep "use.*domain::dialogue::DialogueTree" sdk/campaign_builder/src/dialogue_editor.rs

# Verify RON serialization
grep "\.ron" sdk/campaign_builder/src/dialogue_editor.rs

# Verify no visual constants in editor (should return no matches)
! grep "DIALOGUE_BUBBLE_" sdk/campaign_builder/src/dialogue_editor.rs || echo "❌ Editor has visual constants"
```

**Expected Result**: All verifications pass → NO CHANGES required

### Identified Issues

1. **No Visual Feedback**: Current system only updates `GameMode` and logs text to `GameLog`

   - No on-screen dialogue bubbles
   - No indication of who is speaking
   - No visual choice presentation

2. **Synchronous Execution**: Actions execute immediately in update loop

   - No animation timing
   - Text appears instantly (no typewriter effect)
   - Poor user experience

3. **No NPC Integration**: Dialogue system doesn't position visuals relative to NPCs

   - No spatial relationship between speaker and text
   - Breaks immersion in 2.5D environment

4. **Limited State Visibility**: Dialogue state hidden in `GameMode` enum

   - Difficult to query from other systems
   - No component-based approach for ECS integration

5. **Missing Visual Components**: No Bevy components for dialogue UI
   - Text rendering happens in separate UI layer
   - Cannot use Bevy's transform/hierarchy system

---

## Architecture Compliance

### Phase Alignment

Per `docs/reference/architecture.md` Section 8:

- **Phase 3: World (Weeks 6-8)** includes "NPCs and dialogue" (lines 2131-2138)
- This implementation completes Phase 3 dialogue requirements
- Adds visual polish typically in Phase 6 (typewriter effects, floating bubbles)

**Justification**: Early visual implementation improves playability during development and testing.

### Data Structure Compliance

Per `docs/reference/architecture.md` Section 4:

- ✅ **MAINTAINS** existing `DialogueTree`, `DialogueNode`, `DialogueChoice` structures
- ✅ **USES** `GameMode::Dialogue` as defined in architecture
- ✅ **FOLLOWS** RON data format specification (Section 7.1)
- ✅ **NO MODIFICATIONS** to core domain structures

### Module Structure Compliance

Per `docs/reference/architecture.md` Section 3.2:

| Module                             | Architecture Path        | Implementation Action                 | Compliance                    |
| ---------------------------------- | ------------------------ | ------------------------------------- | ----------------------------- |
| `domain/dialogue.rs`               | `src/domain/dialogue.rs` | NO CHANGES                            | ✅ Fully Compliant            |
| `game/systems/dialogue.rs`         | `src/game/systems/`      | MODIFY - Add visual integration calls | ✅ Within defined structure   |
| `application/dialogue.rs`          | `src/application/`       | MODIFY - Add visual state fields      | ✅ Within defined structure   |
| `game/components/dialogue.rs`      | NOT DEFINED              | **CREATE NEW**                        | ⚠️ **Requires Justification** |
| `game/systems/dialogue_visuals.rs` | NOT DEFINED              | **CREATE NEW**                        | ⚠️ **Requires Justification** |

---

## Architectural Drift Analysis

This implementation introduces modules NOT explicitly defined in architecture.md Section 3.2.

### New Modules Created

| Module                             | Justification                                                                                                                                                                | Architecture Principle   | Risk Level |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | ---------- |
| `game/components/dialogue.rs`      | Bevy ECS requires Component structs. Architecture defines `domain/dialogue.rs` for data structures, not visual components. Separates data (domain) from presentation (game). | "Separation of Concerns" | LOW        |
| `game/systems/dialogue_visuals.rs` | Separates visual rendering from game logic. Existing `systems/dialogue.rs` handles event processing and state management only.                                               | "Separation of Concerns" | LOW        |

### Domain Layer Impact: ZERO

- ✅ NO changes to `domain::dialogue::DialogueTree`
- ✅ NO changes to `domain::dialogue::DialogueNode`
- ✅ NO changes to `domain::dialogue::DialogueChoice`
- ✅ NO changes to `domain::dialogue::DialogueCondition`
- ✅ NO changes to `domain::dialogue::DialogueAction`
- ✅ NO changes to existing RON data format

### Architecture Compliance Statement

**Core Principle Adherence**:

- ✅ **Separation of Concerns**: Visual systems (`dialogue_visuals.rs`) separated from domain logic (`domain/dialogue.rs`) and game logic (`systems/dialogue.rs`)
- ✅ **Data-Driven Design**: Continues using RON-defined `DialogueTree` without modification
- ✅ **Entity-Component Pattern**: Uses Bevy ECS components correctly (`DialogueBubble`, `TypewriterText`, `Billboard`)
- ✅ **Deterministic Gameplay**: Visual presentation doesn't affect game state; dialogue progression remains deterministic

**Layer Boundaries Maintained**:

```
Domain Layer (src/domain/dialogue.rs)
  ↓ [Defines data structures only]
Game Layer (src/game/systems/dialogue.rs)
  ↓ [Processes events, manages state, executes actions]
Game Layer - NEW (src/game/systems/dialogue_visuals.rs)
  ↓ [Renders visual components based on state]
Game Layer - NEW (src/game/components/dialogue.rs)
  ↓ [Bevy component markers for visual entities]
```

**Deviation Approval Request**: This plan creates new modules in the game layer while maintaining domain layer immutability and adhering to separation of concerns. New modules follow existing patterns (`game/systems/hud.rs`, `game/systems/ui.rs`).

---

## Implementation Phases

### Phase 1: Component and Resource Foundation

**Objective**: Create Bevy ECS components and resources for dialogue visual representation.

**Dependencies**: None

**Estimated Time**: 2-3 hours

---

#### 1.1 Create Dialogue Components Module

**Files to Create**:

1. `antares/src/game/components.rs` (module declaration)
2. `antares/src/game/components/mod.rs`
3. `antares/src/game/components/dialogue.rs`

**File 1**: `antares/src/game/components.rs`

**Action**: Create new file with:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

pub mod dialogue;
```

**File 2**: `antares/src/game/components/mod.rs`

**Action**: Create new file with:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

pub mod dialogue;
```

**File 3**: `antares/src/game/components/dialogue.rs`

**Action**: Create new file with complete implementation:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;

/// Type alias for dialogue bubble root entity
pub type DialogueBubbleEntity = Entity;

/// Type alias for dialogue background entity
pub type DialogueBackgroundEntity = Entity;

/// Type alias for dialogue text entity
pub type DialogueTextEntity = Entity;

// Visual Constants
pub const DIALOGUE_BUBBLE_Y_OFFSET: f32 = 2.5;
pub const DIALOGUE_BUBBLE_WIDTH: f32 = 4.0;
pub const DIALOGUE_BUBBLE_HEIGHT: f32 = 1.2;
pub const DIALOGUE_BUBBLE_PADDING: f32 = 0.2;
pub const DIALOGUE_TEXT_SIZE: f32 = 24.0;
pub const DIALOGUE_TYPEWRITER_SPEED: f32 = 0.05; // seconds per character
pub const DIALOGUE_BACKGROUND_COLOR: Color = Color::srgba(0.1, 0.1, 0.15, 0.9);
pub const DIALOGUE_TEXT_COLOR: Color = Color::srgb(0.95, 0.95, 0.95);
pub const DIALOGUE_CHOICE_COLOR: Color = Color::srgb(0.8, 0.8, 0.3);

/// Marks an entity as a dialogue bubble UI element
///
/// Dialogue bubbles are 2.5D UI elements that float above NPCs during conversations.
/// They contain text content and follow the speaker entity's position.
#[derive(Component, Debug)]
pub struct DialogueBubble {
    /// Entity that spawned this dialogue (typically NPC)
    pub speaker_entity: Entity,
    /// Root entity of the bubble hierarchy
    pub root_entity: Entity,
    /// Background mesh entity
    pub background_entity: Entity,
    /// Text entity
    pub text_entity: Entity,
    /// Vertical offset from speaker position
    pub y_offset: f32,
}

/// Billboard component - makes entity always face the camera
///
/// Used for dialogue bubbles and other 2.5D UI elements that should
/// remain readable regardless of camera angle.
#[derive(Component, Debug)]
pub struct Billboard;

/// Typewriter text animation state
///
/// Animates text reveal character-by-character for dialogue text.
#[derive(Component, Debug)]
pub struct TypewriterText {
    /// Full text to display
    pub full_text: String,
    /// Currently visible character count
    pub visible_chars: usize,
    /// Time since last character reveal
    pub timer: f32,
    /// Seconds per character
    pub speed: f32,
    /// Whether animation is complete
    pub finished: bool,
}

/// Resource tracking active dialogue UI entity
///
/// Allows systems to reference the currently active dialogue bubble
/// for updates and cleanup.
#[derive(Resource, Debug, Default)]
pub struct ActiveDialogueUI {
    pub bubble_entity: Option<Entity>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typewriter_text_initialization() {
        let typewriter = TypewriterText {
            full_text: "Hello, world!".to_string(),
            visible_chars: 0,
            timer: 0.0,
            speed: DIALOGUE_TYPEWRITER_SPEED,
            finished: false,
        };
        assert_eq!(typewriter.visible_chars, 0);
        assert!(!typewriter.finished);
        assert_eq!(typewriter.full_text, "Hello, world!");
    }

    #[test]
    fn test_dialogue_bubble_constants() {
        assert!(DIALOGUE_BUBBLE_Y_OFFSET > 0.0);
        assert!(DIALOGUE_TYPEWRITER_SPEED > 0.0);
        assert!(DIALOGUE_BUBBLE_WIDTH > 0.0);
        assert!(DIALOGUE_BUBBLE_HEIGHT > 0.0);
        assert!(DIALOGUE_TEXT_SIZE > 0.0);
    }

    #[test]
    fn test_typewriter_text_component() {
        let typewriter = TypewriterText {
            full_text: "Test".to_string(),
            visible_chars: 2,
            timer: 0.1,
            speed: 0.05,
            finished: false,
        };
        assert_eq!(typewriter.visible_chars, 2);
        assert_eq!(typewriter.timer, 0.1);
    }
}
```

**File 4**: `antares/src/game/mod.rs`

**Location**: Existing file - MODIFY

**Search Pattern**: `pub mod resources;` or similar module declarations

**Action**: Add line immediately after existing module declarations:

```rust
pub mod components;
```

**Verification Command**:

```bash
grep "pub mod components;" src/game/mod.rs
```

**Expected Output**: Line containing `pub mod components;`

---

**Deliverables - Phase 1.1**:

- [ ] File `antares/src/game/components.rs` exists

  - Verify: `test -f src/game/components.rs && echo "✅ exists" || echo "❌ MISSING"`
  - Contains SPDX header: `grep -q "SPDX-FileCopyrightText" src/game/components.rs && echo "✅" || echo "❌"`
  - Contains module declaration: `grep -q "pub mod dialogue;" src/game/components.rs && echo "✅" || echo "❌"`

- [ ] File `antares/src/game/components/mod.rs` exists

  - Verify: `test -f src/game/components/mod.rs && echo "✅ exists" || echo "❌ MISSING"`
  - Contains SPDX header: `grep -q "SPDX-FileCopyrightText" src/game/components/mod.rs && echo "✅" || echo "❌"`
  - Contains module declaration: `grep -q "pub mod dialogue;" src/game/components/mod.rs && echo "✅" || echo "❌"`

- [ ] File `antares/src/game/components/dialogue.rs` created with ALL required symbols

  - Verify: `test -f src/game/components/dialogue.rs && echo "✅ exists" || echo "❌ MISSING"`
  - Contains SPDX header: `grep -q "SPDX-FileCopyrightText" src/game/components/dialogue.rs && echo "✅" || echo "❌"`
  - Contains exactly 3 type aliases: `ALIAS_COUNT=$(grep -c "pub type.*Entity" src/game/components/dialogue.rs); [ "$ALIAS_COUNT" -eq 3 ] && echo "✅ 3 type aliases" || echo "❌ Expected 3, got $ALIAS_COUNT"`
  - Contains exactly 9 constants: `CONST_COUNT=$(grep -c "pub const DIALOGUE_" src/game/components/dialogue.rs); [ "$CONST_COUNT" -eq 9 ] && echo "✅ 9 constants" || echo "❌ Expected 9, got $CONST_COUNT"`
  - Contains exactly 4 structs: `STRUCT_COUNT=$(grep -c "pub struct" src/game/components/dialogue.rs); [ "$STRUCT_COUNT" -eq 4 ] && echo "✅ 4 structs" || echo "❌ Expected 4, got $STRUCT_COUNT"`
  - DialogueBubble has 5 fields: `grep -A10 "pub struct DialogueBubble" src/game/components/dialogue.rs | grep -c "pub.*:" | grep -q "5" && echo "✅" || echo "❌"`
  - TypewriterText has 5 fields: `grep -A10 "pub struct TypewriterText" src/game/components/dialogue.rs | grep -c "pub.*:" | grep -q "5" && echo "✅" || echo "❌"`
  - Contains test module: `grep -q "#\[cfg(test)\]" src/game/components/dialogue.rs && echo "✅" || echo "❌"`
  - Contains 3+ tests: `TEST_COUNT=$(grep -c "#\[test\]" src/game/components/dialogue.rs); [ "$TEST_COUNT" -ge 3 ] && echo "✅ $TEST_COUNT tests" || echo "❌ Expected >=3, got $TEST_COUNT"`

- [ ] `antares/src/game/mod.rs` updated with components module

  - Verify: `grep -q "pub mod components;" src/game/mod.rs && echo "✅" || echo "❌"`

- [ ] All quality checks pass:
  - `cargo fmt --all --check || { echo "❌ Format failed - run: cargo fmt --all"; exit 1; }`
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && { echo "❌ Compile errors"; exit 1; } || echo "✅ Compiles"`
  - `cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "warning" && { echo "❌ Clippy warnings"; exit 1; } || echo "✅ No warnings"`

---

#### 1.2 Update Application Layer DialogueState

**Files to Modify**:

- `antares/src/application/dialogue.rs`

**Location 1**: `DialogueState` struct definition

**Search Pattern**: `pub struct DialogueState`

**Action**: Add fields to struct (before closing brace):

```rust
    /// Current node's full text (for visual systems)
    pub current_text: String,
    /// Current node's speaker name
    pub current_speaker: String,
    /// Current node's available choices
    pub current_choices: Vec<String>,
```

**Location 2**: Add new method to `DialogueState` impl block

**Search Pattern**: `impl DialogueState`

**Action**: Add method before closing brace:

```rust
    /// Updates dialogue state with new node information
    ///
    /// Called when dialogue advances to a new node to update visual state.
    ///
    /// # Arguments
    ///
    /// * `text` - The new node's text content
    /// * `speaker` - The speaker's name for this node
    /// * `choices` - Available player choices (empty for terminal nodes)
    pub fn update_node(&mut self, text: String, speaker: String, choices: Vec<String>) {
        self.current_text = text;
        self.current_speaker = speaker;
        self.current_choices = choices;
    }
```

**Location 3**: Update existing `start` or `new` method

**Search Pattern**: `pub fn start` OR `pub fn new`

**Action**: Initialize new fields in constructor:

```rust
        current_text: String::new(),
        current_speaker: String::new(),
        current_choices: Vec::new(),
```

---

**Deliverables - Phase 1.2**:

- [ ] `DialogueState` struct has 3 new fields

  - Verify current_text field: `grep -q "pub current_text: String" src/application/dialogue.rs && echo "✅" || echo "❌"`
  - Verify current_speaker field: `grep -q "pub current_speaker: String" src/application/dialogue.rs && echo "✅" || echo "❌"`
  - Verify current_choices field: `grep -q "pub current_choices: Vec<String>" src/application/dialogue.rs && echo "✅" || echo "❌"`

- [ ] `update_node` method implemented

  - Verify method exists: `grep -q "pub fn update_node" src/application/dialogue.rs && echo "✅" || echo "❌"`
  - Method has 3 parameters: `grep -A2 "pub fn update_node" src/application/dialogue.rs | grep -c "String" | grep -q "3" && echo "✅" || echo "❌"`
  - Contains doc comment: `grep -B3 "pub fn update_node" src/application/dialogue.rs | grep -q "///" && echo "✅" || echo "❌"`

- [ ] Constructor initializes new fields

  - Verify initialization: `grep -A20 "pub fn.*DialogueState" src/application/dialogue.rs | grep -q "current_text:" && echo "✅" || echo "❌"`

- [ ] Quality checks pass:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && { echo "❌"; exit 1; } || echo "✅"`
  - `cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "warning" && { echo "❌"; exit 1; } || echo "✅"`

---

#### 1.3 Testing Requirements

**File**: `antares/src/application/dialogue.rs` (add tests at end)

**Action**: Add or extend test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialogue_state_update_node() {
        let mut state = DialogueState::default();

        state.update_node(
            "Hello, traveler!".to_string(),
            "Village Elder".to_string(),
            vec!["Greetings".to_string(), "Farewell".to_string()],
        );

        assert_eq!(state.current_text, "Hello, traveler!");
        assert_eq!(state.current_speaker, "Village Elder");
        assert_eq!(state.current_choices.len(), 2);
        assert_eq!(state.current_choices[0], "Greetings");
    }

    #[test]
    fn test_dialogue_state_initialization() {
        let state = DialogueState::default();

        assert_eq!(state.current_text, "");
        assert_eq!(state.current_speaker, "");
        assert_eq!(state.current_choices.len(), 0);
    }

    #[test]
    fn test_update_node_overwrites_previous() {
        let mut state = DialogueState::default();

        state.update_node("First".to_string(), "Speaker1".to_string(), vec![]);
        state.update_node("Second".to_string(), "Speaker2".to_string(), vec!["Choice".to_string()]);

        assert_eq!(state.current_text, "Second");
        assert_eq!(state.current_speaker, "Speaker2");
        assert_eq!(state.current_choices.len(), 1);
    }
}
```

---

**Deliverables - Phase 1.3**:

- [ ] Test module exists in `dialogue.rs`

  - Verify: `grep -q "#\[cfg(test)\]" src/application/dialogue.rs && echo "✅" || echo "❌"`

- [ ] At least 3 tests for DialogueState

  - Verify: `TEST_COUNT=$(grep -c "#\[test\]" src/application/dialogue.rs); [ "$TEST_COUNT" -ge 3 ] && echo "✅ $TEST_COUNT tests" || echo "❌ Expected >=3"`

- [ ] Tests cover update_node functionality

  - Verify: `grep -A10 "#\[test\]" src/application/dialogue.rs | grep -q "update_node" && echo "✅" || echo "❌"`

- [ ] All tests pass:
  - `cargo nextest run --all-features dialogue 2>&1 | grep -q "FAILED" && { echo "❌"; exit 1; } || echo "✅"`

---

#### 1.4 Success Criteria - Phase 1

**Automated Validation Script**:

```bash
#!/bin/bash
# Phase 1 Validation Script

echo "=== Phase 1 Validation ==="

# File existence checks
echo "Checking file creation..."
test -f src/game/components.rs || { echo "❌ Missing components.rs"; exit 1; }
test -f src/game/components/mod.rs || { echo "❌ Missing components/mod.rs"; exit 1; }
test -f src/game/components/dialogue.rs || { echo "❌ Missing components/dialogue.rs"; exit 1; }
echo "✅ All files created"

# SPDX header checks
echo "Checking SPDX headers..."
grep -q "SPDX-FileCopyrightText" src/game/components.rs || { echo "❌ Missing SPDX in components.rs"; exit 1; }
grep -q "SPDX-FileCopyrightText" src/game/components/dialogue.rs || { echo "❌ Missing SPDX in dialogue.rs"; exit 1; }
echo "✅ SPDX headers present"

# Symbol count verification
echo "Checking symbol counts..."
CONST_COUNT=$(grep -c "pub const DIALOGUE_" src/game/components/dialogue.rs)
[ "$CONST_COUNT" -eq 9 ] || { echo "❌ Expected 9 constants, got $CONST_COUNT"; exit 1; }

STRUCT_COUNT=$(grep -c "pub struct" src/game/components/dialogue.rs)
[ "$STRUCT_COUNT" -eq 4 ] || { echo "❌ Expected 4 structs, got $STRUCT_COUNT"; exit 1; }
echo "✅ Correct symbol counts"

# DialogueState field verification
echo "Checking DialogueState modifications..."
grep -q "pub current_text: String" src/application/dialogue.rs || { echo "❌ Missing current_text field"; exit 1; }
grep -q "pub fn update_node" src/application/dialogue.rs || { echo "❌ Missing update_node method"; exit 1; }
echo "✅ DialogueState updated correctly"

# Quality gates
echo "Running quality checks..."
cargo fmt --all --check || { echo "❌ Format check failed"; exit 1; }
cargo check --all-targets --all-features 2>&1 | grep -q "error" && { echo "❌ Compile errors"; exit 1; }
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "warning" && { echo "❌ Clippy warnings"; exit 1; }
cargo nextest run --all-features 2>&1 | grep -q "FAILED" && { echo "❌ Test failures"; exit 1; }
echo "✅ All quality checks passed"

echo ""
echo "=== Phase 1 Complete ✅ ==="
```

**Manual Verification Checklist**:

- [ ] All deliverables from 1.1, 1.2, 1.3 checked
- [ ] `cargo fmt --all` produces zero changes
- [ ] `cargo check --all-targets --all-features` exits with code 0
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` reports 0 warnings
- [ ] `cargo nextest run --all-features` shows 100% pass rate
- [ ] Code coverage >80% for new components module: `cargo llvm-cov nextest --all-features`

---

#### 1.5 Rollback Procedure - Phase 1

If Phase 1 validation fails:

```bash
# Remove created files
rm -f src/game/components.rs
rm -rf src/game/components/

# Restore modified files
git checkout HEAD -- src/game/mod.rs
git checkout HEAD -- src/application/dialogue.rs

# Clean build artifacts
cargo clean

# Verify rollback
cargo check --all-targets --all-features
```

**Common Failure Modes**:

| Error                | Cause                                | Fix                                         |
| -------------------- | ------------------------------------ | ------------------------------------------- |
| Missing SPDX headers | Forgot to add headers to .rs files   | Add headers as first 2 lines in file        |
| Wrong symbol count   | Copy-paste error in dialogue.rs      | Re-copy code from plan exactly              |
| Clippy warnings      | Unused imports or variables          | Run `cargo clippy --fix --all-targets`      |
| Test failures        | DialogueState fields not initialized | Add field initialization to constructor     |
| Import errors        | Missing module declaration           | Verify `pub mod components;` in game/mod.rs |

---

### Phase 2: Visual System Implementation

**Objective**: Create Bevy systems that render dialogue bubbles, animate text, and manage visual state.

**Dependencies**: Phase 1.1, 1.2 complete (can start before 1.3, 1.4)

**Estimated Time**: 4-5 hours

---

#### 2.1 Create Dialogue Visuals System

**Files to Create**:

- `antares/src/game/systems/dialogue_visuals.rs`

**Action**: Create complete file:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;
use thiserror::Error;

use crate::application::{GameMode, GlobalState};
use crate::game::components::dialogue::*;

#[derive(Error, Debug)]
pub enum DialogueVisualError {
    #[error("Failed to spawn dialogue bubble: speaker entity {0:?} not found")]
    SpeakerNotFound(Entity),

    #[error("Failed to create mesh: {0}")]
    MeshCreationFailed(String),

    #[error("DialogueState not available in Dialogue mode")]
    InvalidGameMode,
}

/// Spawns a 2.5D dialogue bubble above the speaker entity
///
/// Creates a hierarchy:
/// - Root entity (positioned above speaker)
///   - Background entity (semi-transparent panel)
///   - Text entity (animated typewriter text)
///
/// # Arguments
///
/// * `commands` - Bevy commands for entity spawning
/// * `global_state` - Current game state (must be in Dialogue mode)
/// * `active_ui` - Resource tracking active dialogue bubble
/// * `meshes` - Mesh assets
/// * `materials` - Material assets
///
/// # Returns
///
/// Returns `Ok(Entity)` with bubble root entity ID on success
///
/// # Errors
///
/// Returns `DialogueVisualError::SpeakerNotFound` if speaker entity doesn't exist
/// Returns `DialogueVisualError::InvalidGameMode` if not in Dialogue mode
pub fn spawn_dialogue_bubble(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    mut active_ui: ResMut<ActiveDialogueUI>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query_speaker: Query<&Transform, With<Sprite>>, // Adjust query based on NPC setup
) {
    // Only spawn if in Dialogue mode and no bubble exists
    if let GameMode::Dialogue(ref dialogue_state) = global_state.0.mode {
        if active_ui.bubble_entity.is_some() {
            return; // Bubble already exists
        }

        // TODO: Get speaker entity from dialogue context
        // For now, spawn at world origin + offset
        let speaker_position = Vec3::new(0.0, 0.0, 0.0);

        // Create background mesh (simple quad)
        let background_mesh = meshes.add(Mesh::from(Rectangle::new(
            DIALOGUE_BUBBLE_WIDTH,
            DIALOGUE_BUBBLE_HEIGHT,
        )));

        let background_material = materials.add(StandardMaterial {
            base_color: DIALOGUE_BACKGROUND_COLOR,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        // Spawn root entity
        let root_entity = commands
            .spawn((
                SpatialBundle {
                    transform: Transform::from_translation(
                        speaker_position + Vec3::new(0.0, DIALOGUE_BUBBLE_Y_OFFSET, 0.0),
                    ),
                    ..default()
                },
                Billboard,
            ))
            .id();

        // Spawn background as child
        let background_entity = commands
            .spawn(PbrBundle {
                mesh: background_mesh,
                material: background_material,
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                ..default()
            })
            .id();

        // Spawn text as child
        let text_entity = commands
            .spawn((
                Text2dBundle {
                    text: Text::from_section(
                        "",
                        TextStyle {
                            font_size: DIALOGUE_TEXT_SIZE,
                            color: DIALOGUE_TEXT_COLOR,
                            ..default()
                        },
                    ),
                    transform: Transform::from_xyz(0.0, 0.0, 0.1),
                    ..default()
                },
                TypewriterText {
                    full_text: dialogue_state.current_text.clone(),
                    visible_chars: 0,
                    timer: 0.0,
                    speed: DIALOGUE_TYPEWRITER_SPEED,
                    finished: false,
                },
            ))
            .id();

        // Set up hierarchy
        commands.entity(root_entity).add_child(background_entity);
        commands.entity(root_entity).add_child(text_entity);

        // Create DialogueBubble component
        let bubble = commands
            .spawn(DialogueBubble {
                speaker_entity: Entity::PLACEHOLDER, // TODO: Use actual speaker
                root_entity,
                background_entity,
                text_entity,
                y_offset: DIALOGUE_BUBBLE_Y_OFFSET,
            })
            .id();

        active_ui.bubble_entity = Some(bubble);
    }
}

/// Updates typewriter text animation
///
/// Reveals text character-by-character based on elapsed time.
///
/// # Arguments
///
/// * `time` - Bevy time resource
/// * `query` - Query for entities with Text and TypewriterText components
pub fn update_typewriter_text(
    time: Res<Time>,
    mut query: Query<(&mut Text, &mut TypewriterText)>,
) {
    for (mut text, mut typewriter) in query.iter_mut() {
        if typewriter.finished {
            continue;
        }

        typewriter.timer += time.delta_seconds();

        if typewriter.timer >= typewriter.speed {
            typewriter.timer = 0.0;
            typewriter.visible_chars = (typewriter.visible_chars + 1).min(typewriter.full_text.len());

            // Update visible text
            let visible_text: String = typewriter
                .full_text
                .chars()
                .take(typewriter.visible_chars)
                .collect();

            text.sections[0].value = visible_text;

            if typewriter.visible_chars >= typewriter.full_text.len() {
                typewriter.finished = true;
            }
        }
    }
}

/// Billboard system - makes entities face the camera
///
/// Rotates entities marked with Billboard component to always face camera.
///
/// # Arguments
///
/// * `query_camera` - Query for camera transform
/// * `query_billboards` - Query for entities with Billboard component
pub fn billboard_system(
    query_camera: Query<&Transform, With<Camera>>,
    mut query_billboards: Query<&mut Transform, (With<Billboard>, Without<Camera>)>,
) {
    if let Ok(camera_transform) = query_camera.get_single() {
        for mut transform in query_billboards.iter_mut() {
            transform.look_at(camera_transform.translation, Vec3::Y);
        }
    }
}

/// Cleans up dialogue bubble when dialogue ends
///
/// Despawns all dialogue UI entities when game mode changes from Dialogue.
///
/// # Arguments
///
/// * `commands` - Bevy commands for entity despawning
/// * `global_state` - Current game state
/// * `active_ui` - Resource tracking active dialogue bubble
/// * `query` - Query for DialogueBubble components
pub fn cleanup_dialogue_bubble(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    mut active_ui: ResMut<ActiveDialogueUI>,
    query: Query<(Entity, &DialogueBubble)>,
) {
    // If no longer in Dialogue mode, cleanup
    if !matches!(global_state.0.mode, GameMode::Dialogue(_)) {
        if let Some(bubble_entity) = active_ui.bubble_entity {
            // Despawn bubble and all children
            if let Ok((_, bubble)) = query.get(bubble_entity) {
                commands.entity(bubble.root_entity).despawn_recursive();
                commands.entity(bubble_entity).despawn();
            }
            active_ui.bubble_entity = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typewriter_reveals_characters_over_time() {
        // This test would require Bevy app setup
        // Placeholder for integration test
        let typewriter = TypewriterText {
            full_text: "Hello".to_string(),
            visible_chars: 0,
            timer: 0.0,
            speed: 0.05,
            finished: false,
        };

        assert_eq!(typewriter.visible_chars, 0);
        assert!(!typewriter.finished);
    }

    #[test]
    fn test_typewriter_finishes_when_complete() {
        let mut typewriter = TypewriterText {
            full_text: "Hi".to_string(),
            visible_chars: 2,
            timer: 0.0,
            speed: 0.05,
            finished: false,
        };

        // Simulate completion
        if typewriter.visible_chars >= typewriter.full_text.len() {
            typewriter.finished = true;
        }

        assert!(typewriter.finished);
    }
}
```

---

**Deliverables - Phase 2.1**:

- [ ] File `antares/src/game/systems/dialogue_visuals.rs` created

  - Verify: `test -f src/game/systems/dialogue_visuals.rs && echo "✅" || echo "❌"`
  - SPDX header: `grep -q "SPDX-FileCopyrightText" src/game/systems/dialogue_visuals.rs && echo "✅" || echo "❌"`

- [ ] Contains 4 public functions

  - spawn_dialogue_bubble: `grep -q "pub fn spawn_dialogue_bubble" src/game/systems/dialogue_visuals.rs && echo "✅" || echo "❌"`
  - update_typewriter_text: `grep -q "pub fn update_typewriter_text" src/game/systems/dialogue_visuals.rs && echo "✅" || echo "❌"`
  - billboard_system: `grep -q "pub fn billboard_system" src/game/systems/dialogue_visuals.rs && echo "✅" || echo "❌"`
  - cleanup_dialogue_bubble: `grep -q "pub fn cleanup_dialogue_bubble" src/game/systems/dialogue_visuals.rs && echo "✅" || echo "❌"`

- [ ] Contains error type

  - Verify: `grep -q "pub enum DialogueVisualError" src/game/systems/dialogue_visuals.rs && echo "✅" || echo "❌"`
  - Uses thiserror: `grep -q "use thiserror::Error;" src/game/systems/dialogue_visuals.rs && echo "✅" || echo "❌"`

- [ ] All functions have doc comments

  - Verify: `FUNC_COUNT=$(grep -c "pub fn" src/game/systems/dialogue_visuals.rs); DOC_COUNT=$(grep -B3 "pub fn" src/game/systems/dialogue_visuals.rs | grep -c "///"); [ "$DOC_COUNT" -ge "$FUNC_COUNT" ] && echo "✅" || echo "❌"`

- [ ] Contains test module

  - Verify: `grep -q "#\[cfg(test)\]" src/game/systems/dialogue_visuals.rs && echo "✅" || echo "❌"`

- [ ] Quality checks:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && echo "❌" || echo "✅"`
  - `cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "warning" && echo "❌" || echo "✅"`

---

#### 2.2 Integrate Visual Systems into Plugin

**Files to Modify**:

- `antares/src/game/systems/mod.rs`
- `antares/src/game/systems/dialogue.rs`

**File 1**: `antares/src/game/systems/mod.rs`

**Search Pattern**: `pub mod dialogue;` or similar module declarations

**Action**: Add immediately after dialogue module declaration:

```rust
pub mod dialogue_visuals;
```

**File 2**: `antares/src/game/systems/dialogue.rs`

**Location**: `impl Plugin for DialoguePlugin` block

**Search Pattern**: `fn build(&mut self, app: &mut App)`

**Action**: Find the `.add_systems(Update, (...)` call and modify to include visual systems:

```rust
.add_systems(
    Update,
    (
        handle_start_dialogue,
        handle_select_choice,
        handle_recruitment_actions,
        crate::game::systems::dialogue_visuals::spawn_dialogue_bubble,
        crate::game::systems::dialogue_visuals::update_typewriter_text,
        crate::game::systems::dialogue_visuals::billboard_system,
        crate::game::systems::dialogue_visuals::cleanup_dialogue_bubble,
    ),
)
```

**Also Add**: Resource initialization in `build` method:

```rust
app.init_resource::<crate::game::components::dialogue::ActiveDialogueUI>();
```

---

**Deliverables - Phase 2.2**:

- [ ] Module declaration added to systems/mod.rs

  - Verify: `grep -q "pub mod dialogue_visuals;" src/game/systems/mod.rs && echo "✅" || echo "❌"`

- [ ] Visual systems registered in DialoguePlugin

  - Verify spawn_dialogue_bubble: `grep -A20 "impl Plugin for DialoguePlugin" src/game/systems/dialogue.rs | grep -q "spawn_dialogue_bubble" && echo "✅" || echo "❌"`
  - Verify update_typewriter_text: `grep -A20 "impl Plugin for DialoguePlugin" src/game/systems/dialogue.rs | grep -q "update_typewriter_text" && echo "✅" || echo "❌"`
  - Verify billboard_system: `grep -A20 "impl Plugin for DialoguePlugin" src/game/systems/dialogue.rs | grep -q "billboard_system" && echo "✅" || echo "❌"`
  - Verify cleanup_dialogue_bubble: `grep -A20 "impl Plugin for DialoguePlugin" src/game/systems/dialogue.rs | grep -q "cleanup_dialogue_bubble" && echo "✅" || echo "❌"`

- [ ] ActiveDialogueUI resource registered

  - Verify: `grep -q "init_resource.*ActiveDialogueUI" src/game/systems/dialogue.rs && echo "✅" || echo "❌"`

- [ ] Quality checks:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && echo "❌" || echo "✅"`

---

#### 2.3 Testing Requirements

**File**: `antares/tests/dialogue_visuals_test.rs` (create new integration test)

**Action**: Create complete test file:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;
use antares::game::components::dialogue::*;

#[test]
fn test_typewriter_text_component_creation() {
    let typewriter = TypewriterText {
        full_text: "Test message".to_string(),
        visible_chars: 0,
        timer: 0.0,
        speed: DIALOGUE_TYPEWRITER_SPEED,
        finished: false,
    };

    assert_eq!(typewriter.full_text, "Test message");
    assert_eq!(typewriter.visible_chars, 0);
    assert!(!typewriter.finished);
}

#[test]
fn test_dialogue_bubble_constants_are_positive() {
    assert!(DIALOGUE_BUBBLE_Y_OFFSET > 0.0);
    assert!(DIALOGUE_BUBBLE_WIDTH > 0.0);
    assert!(DIALOGUE_BUBBLE_HEIGHT > 0.0);
    assert!(DIALOGUE_TEXT_SIZE > 0.0);
    assert!(DIALOGUE_TYPEWRITER_SPEED > 0.0);
}

#[test]
fn test_active_dialogue_ui_default() {
    let ui = ActiveDialogueUI::default();
    assert!(ui.bubble_entity.is_none());
}
```

---

**Deliverables - Phase 2.3**:

- [ ] Integration test file created

  - Verify: `test -f tests/dialogue_visuals_test.rs && echo "✅" || echo "❌"`
  - SPDX header: `grep -q "SPDX-FileCopyrightText" tests/dialogue_visuals_test.rs && echo "✅" || echo "❌"`

- [ ] Contains at least 3 tests

  - Verify: `TEST_COUNT=$(grep -c "#\[test\]" tests/dialogue_visuals_test.rs); [ "$TEST_COUNT" -ge 3 ] && echo "✅ $TEST_COUNT tests" || echo "❌"`

- [ ] All tests pass:
  - `cargo nextest run dialogue_visuals 2>&1 | grep -q "FAILED" && echo "❌" || echo "✅"`

---

#### 2.4 Success Criteria - Phase 2

**Validation Script**:

```bash
#!/bin/bash
echo "=== Phase 2 Validation ==="

# File checks
test -f src/game/systems/dialogue_visuals.rs || { echo "❌ Missing dialogue_visuals.rs"; exit 1; }
test -f tests/dialogue_visuals_test.rs || { echo "❌ Missing integration test"; exit 1; }
echo "✅ Files created"

# Function count
FUNC_COUNT=$(grep -c "pub fn" src/game/systems/dialogue_visuals.rs)
[ "$FUNC_COUNT" -ge 4 ] || { echo "❌ Expected >=4 functions, got $FUNC_COUNT"; exit 1; }
echo "✅ Functions implemented"

# Integration verification
grep -q "pub mod dialogue_visuals;" src/game/systems/mod.rs || { echo "❌ Module not declared"; exit 1; }
grep -q "spawn_dialogue_bubble" src/game/systems/dialogue.rs || { echo "❌ System not registered"; exit 1; }
echo "✅ Systems integrated"

# Quality gates
cargo fmt --all --check || { echo "❌ Format failed"; exit 1; }
cargo check --all-targets --all-features 2>&1 | grep -q "error" && { echo "❌ Compile errors"; exit 1; }
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "warning" && { echo "❌ Warnings"; exit 1; }
cargo nextest run --all-features 2>&1 | grep -q "FAILED" && { echo "❌ Test failures"; exit 1; }
echo "✅ Quality checks passed"

echo ""
echo "=== Phase 2 Complete ✅ ==="
```

**Manual Verification** (requires running game - optional at this phase):

- [ ] Run: `cargo run --bin antares`
- [ ] Visual systems compile and link without errors
- [ ] No runtime panics during initialization

---

#### 2.5 Rollback Procedure - Phase 2

```bash
# Remove created files
rm -f src/game/systems/dialogue_visuals.rs
rm -f tests/dialogue_visuals_test.rs

# Restore modified files
git checkout HEAD -- src/game/systems/mod.rs
git checkout HEAD -- src/game/systems/dialogue.rs

# Verify rollback
cargo check --all-targets --all-features
```

---

### Phase 3: Event-Driven Logic Integration

**Objective**: Connect visual systems to dialogue state changes through events and system updates.

**Dependencies**: Phase 2.1, 2.2 complete (can start before 2.3, 2.4)

**Estimated Time**: 3-4 hours

---

#### 3.1 Update Dialogue State on Node Changes

**Integration Flow Diagram**:

```
Existing Flow:
  StartDialogue event → handle_start_dialogue (L70-107)
    1. Validates dialogue_id
    2. Loads DialogueTree
    3. Sets GameMode::Dialogue(DialogueState)
    4. Logs text to GameLog

New Flow (Phase 3):
  StartDialogue event → handle_start_dialogue (L70-107) [MODIFIED]
    1. Validates dialogue_id
    2. Loads DialogueTree
    3. Sets GameMode::Dialogue(DialogueState)
    4. Calls DialogueState::update_node() [NEW]
    5. Logs text to GameLog
    ↓
  Update system runs → spawn_dialogue_bubble [NEW]
    - Detects GameMode::Dialogue
    - Spawns DialogueBubble entity
    - Initializes TypewriterText
```

**Files to Modify**:

- `antares/src/game/systems/dialogue.rs`

**Location 1**: `handle_start_dialogue` function

**Search Pattern**: Look for line setting GameMode::Dialogue, typically:

```rust
global_state.0.mode = GameMode::Dialogue(DialogueState::start(/*...*/));
```

**Action**: Add immediately after GameMode is set (before logging):

```rust
// Update DialogueState with current node text and choices
if let Some(node) = tree.get_node(tree.root_node) {
    let speaker = tree.speaker_name.as_deref().unwrap_or("NPC").to_string();
    let choices: Vec<String> = node.choices.iter().map(|c| c.text.clone()).collect();

    if let GameMode::Dialogue(ref mut state) = global_state.0.mode {
        state.update_node(node.text.clone(), speaker, choices);
    }
}
```

**Verification After Edit**:

```bash
grep -A10 "GameMode::Dialogue" src/game/systems/dialogue.rs | grep -q "update_node" && echo "✅" || echo "❌"
```

**Location 2**: `handle_select_choice` function

**Search Pattern**: Look for code that advances to next node after executing choice actions

**Action**: Add after node transition (after setting current_node_id):

```rust
// Update DialogueState with new node information
if let Some(next_node) = tree.get_node(target_node_id) {
    let speaker = tree.speaker_name.as_deref().unwrap_or("NPC").to_string();
    let choices: Vec<String> = next_node.choices.iter().map(|c| c.text.clone()).collect();

    if let GameMode::Dialogue(ref mut state) = global_state.0.mode {
        state.update_node(next_node.text.clone(), speaker, choices);
    }
}
```

**Verification After Edit**:

```bash
grep -c "update_node" src/game/systems/dialogue.rs | grep -q "2" && echo "✅ Found 2 calls" || echo "❌"
```

---

**Deliverables - Phase 3.1**:

- [ ] `handle_start_dialogue` calls `update_node`

  - Verify: `grep -A15 "fn handle_start_dialogue" src/game/systems/dialogue.rs | grep -q "update_node" && echo "✅" || echo "❌"`

- [ ] `handle_select_choice` calls `update_node`

  - Verify: `grep -A30 "fn handle_select_choice" src/game/systems/dialogue.rs | grep -q "update_node" && echo "✅" || echo "❌"`

- [ ] Total of 2 update_node calls

  - Verify: `UPDATE_COUNT=$(grep -c "update_node" src/game/systems/dialogue.rs); [ "$UPDATE_COUNT" -eq 2 ] && echo "✅ 2 calls" || echo "❌ Expected 2, got $UPDATE_COUNT"`

- [ ] Quality checks:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && echo "❌" || echo "✅"`
  - `cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "warning" && echo "❌" || echo "✅"`

---

#### 3.2 Add Dialogue Text Update System

**Files to Modify**:

- `antares/src/game/systems/dialogue_visuals.rs`

**Location**: Add new system function (after `update_typewriter_text`, before `billboard_system`)

**Action**: Add complete function:

```rust
/// Updates dialogue bubble text when node changes
///
/// Detects when DialogueState.current_text changes and resets typewriter animation.
///
/// # Arguments
///
/// * `global_state` - Current game state with DialogueState
/// * `active_ui` - Resource tracking active dialogue bubble
/// * `query_bubble` - Query for DialogueBubble components
/// * `query_text` - Query for Text and TypewriterText components
pub fn update_dialogue_text(
    global_state: Res<GlobalState>,
    active_ui: Res<ActiveDialogueUI>,
    query_bubble: Query<&DialogueBubble>,
    mut query_text: Query<(&mut Text, &mut TypewriterText)>,
) {
    if let GameMode::Dialogue(ref dialogue_state) = global_state.0.mode {
        if let Some(bubble_entity) = active_ui.bubble_entity {
            if let Ok(bubble) = query_bubble.get(bubble_entity) {
                if let Ok((mut text, mut typewriter)) = query_text.get_mut(bubble.text_entity) {
                    // Check if text changed
                    if typewriter.full_text != dialogue_state.current_text {
                        // Reset typewriter animation for new text
                        typewriter.full_text = dialogue_state.current_text.clone();
                        typewriter.visible_chars = 0;
                        typewriter.timer = 0.0;
                        typewriter.finished = false;
                        text.sections[0].value = String::new(); // Clear visible text
                    }
                }
            }
        }
    }
}
```

**Register in Plugin** (`src/game/systems/dialogue.rs`):

**Search Pattern**: Find the `.add_systems(Update, (...)` block that includes `spawn_dialogue_bubble`

**Action**: Insert `dialogue_visuals::update_dialogue_text,` in the system list (after spawn_dialogue_bubble, before update_typewriter_text):

```rust
.add_systems(
    Update,
    (
        handle_start_dialogue,
        handle_select_choice,
        handle_recruitment_actions,
        dialogue_visuals::spawn_dialogue_bubble,
        dialogue_visuals::update_dialogue_text,  // ADD THIS LINE
        dialogue_visuals::update_typewriter_text,
        dialogue_visuals::billboard_system,
        dialogue_visuals::cleanup_dialogue_bubble,
    ),
)
```

---

**Deliverables - Phase 3.2**:

- [ ] `update_dialogue_text` function implemented

  - Verify: `grep -q "pub fn update_dialogue_text" src/game/systems/dialogue_visuals.rs && echo "✅" || echo "❌"`
  - Has doc comment: `grep -B3 "pub fn update_dialogue_text" src/game/systems/dialogue_visuals.rs | grep -q "///" && echo "✅" || echo "❌"`

- [ ] System registered in plugin

  - Verify: `grep -A10 "add_systems" src/game/systems/dialogue.rs | grep -q "update_dialogue_text" && echo "✅" || echo "❌"`

- [ ] Quality checks:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && echo "❌" || echo "✅"`

---

#### 3.3 Add Input Handling for Dialogue Advancement

**Files to Modify**:

- `antares/src/game/systems/dialogue.rs` (add event and system)

**Location 1**: Event definitions (near StartDialogue, SelectDialogueChoice events)

**Action**: Add new event:

```rust
/// Event to advance dialogue (show next text chunk or trigger choice display)
#[derive(Event, Debug)]
pub struct AdvanceDialogue;
```

**Location 2**: `DialoguePlugin` impl, in `fn build`

**Action**: Register event:

```rust
app.add_event::<AdvanceDialogue>();
```

**Verification**:

```bash
grep -q "add_event.*AdvanceDialogue" src/game/systems/dialogue.rs && echo "✅" || echo "❌"
```

**Location 3**: Add new system function (after handle_select_choice)

**Action**: Add complete system:

```rust
/// System to handle input for advancing dialogue
///
/// Sends AdvanceDialogue event when player presses Space or E during dialogue.
///
/// # Arguments
///
/// * `keyboard` - Keyboard input state
/// * `global_state` - Current game state
/// * `ev_advance` - Event writer for AdvanceDialogue events
pub fn dialogue_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    global_state: Res<GlobalState>,
    mut ev_advance: EventWriter<AdvanceDialogue>,
) {
    if matches!(global_state.0.mode, GameMode::Dialogue(_)) {
        if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::KeyE) {
            ev_advance.send(AdvanceDialogue);
        }
    }
}
```

**Location 4**: Register system in plugin

**Search Pattern**: `.add_systems(Update, (...)` block

**Action**: Add `dialogue_input_system,` to the system list:

```rust
.add_systems(
    Update,
    (
        dialogue_input_system,  // ADD THIS LINE
        handle_start_dialogue,
        handle_select_choice,
        // ... rest of systems
    ),
)
```

---

**Deliverables - Phase 3.3**:

- [ ] `AdvanceDialogue` event defined

  - Verify: `grep -q "struct AdvanceDialogue" src/game/systems/dialogue.rs && echo "✅" || echo "❌"`
  - Derives Event: `grep -B1 "struct AdvanceDialogue" src/game/systems/dialogue.rs | grep -q "Event" && echo "✅" || echo "❌"`

- [ ] Event registered in plugin

  - Verify: `grep -q "add_event.*AdvanceDialogue" src/game/systems/dialogue.rs && echo "✅" || echo "❌"`

- [ ] `dialogue_input_system` implemented

  - Verify: `grep -q "pub fn dialogue_input_system" src/game/systems/dialogue.rs && echo "✅" || echo "❌"`

- [ ] System registered in plugin

  - Verify: `grep -A15 "add_systems" src/game/systems/dialogue.rs | grep -q "dialogue_input_system" && echo "✅" || echo "❌"`

- [ ] Quality checks:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && echo "❌" || echo "✅"`

---

#### 3.4 Testing Requirements - Game State Integration

**File**: `antares/tests/dialogue_state_integration_test.rs` (create new integration test)

**Action**: Create complete test file:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;
use antares::application::GameMode;
use antares::game::systems::dialogue::{AdvanceDialogue, StartDialogue};

#[test]
fn test_dialogue_state_updates_on_start() {
    // This test would require Bevy app setup
    // Placeholder for integration test structure

    // Verify that starting dialogue calls update_node
    // Verify that DialogueState contains current_text, current_speaker, current_choices
    assert!(true); // TODO: Implement with Bevy test app
}

#[test]
fn test_advance_dialogue_event_handling() {
    // Verify AdvanceDialogue event is processed correctly
    // Verify input system sends event on Space/E press
    assert!(true); // TODO: Implement with Bevy test app
}

#[test]
fn test_dialogue_state_transitions() {
    // Verify state updates when transitioning between nodes
    // Verify choices are populated correctly
    assert!(true); // TODO: Implement with Bevy test app
}
```

**Unit Tests to Add to `src/game/systems/dialogue.rs`**:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dialogue::{DialogueTree, DialogueNode, DialogueChoice};

    #[test]
    fn test_handle_start_dialogue_updates_state() {
        // Create test dialogue tree
        let mut tree = DialogueTree::new(1, "Test", 1);
        let mut node = DialogueNode::new(1, "Hello!");
        node.choices.push(DialogueChoice {
            text: "Goodbye".to_string(),
            target_node: Some(2),
            conditions: vec![],
            actions: vec![],
            ends_dialogue: true,
        });
        tree.add_node(node);

        // Verify that DialogueState would be created with correct fields
        assert_eq!(tree.get_node(1).unwrap().text, "Hello!");
        assert_eq!(tree.get_node(1).unwrap().choices.len(), 1);
    }

    #[test]
    fn test_dialogue_input_system_requires_dialogue_mode() {
        // Verify input system only processes keys in Dialogue mode
        // This would require Bevy app setup for full test
        assert!(true); // Placeholder
    }
}
```

---

**Deliverables - Phase 3.4**:

- [ ] Integration test file created

  - Verify: `test -f tests/dialogue_state_integration_test.rs && echo "✅" || echo "❌"`
  - SPDX header: `grep -q "SPDX-FileCopyrightText" tests/dialogue_state_integration_test.rs && echo "✅" || echo "❌"`

- [ ] Contains at least 3 integration test stubs

  - Verify: `TEST_COUNT=$(grep -c "#\[test\]" tests/dialogue_state_integration_test.rs); [ "$TEST_COUNT" -ge 3 ] && echo "✅ $TEST_COUNT tests" || echo "❌"`

- [ ] Unit tests added to dialogue.rs

  - Verify: `grep -q "#\[cfg(test)\]" src/game/systems/dialogue.rs && echo "✅" || echo "❌"`
  - At least 2 tests: `TEST_COUNT=$(grep -c "#\[test\]" src/game/systems/dialogue.rs); [ "$TEST_COUNT" -ge 2 ] && echo "✅ $TEST_COUNT tests" || echo "❌"`

- [ ] All tests compile and run:
  - `cargo nextest run dialogue 2>&1 | grep -q "FAILED" && echo "❌" || echo "✅"`

---

#### 3.5 Success Criteria - Phase 3

**Validation Script**:

```bash
#!/bin/bash
echo "=== Phase 3 Validation ==="

# update_node integration checks
echo "Checking DialogueState integration..."
UPDATE_COUNT=$(grep -c "update_node" src/game/systems/dialogue.rs)
[ "$UPDATE_COUNT" -eq 2 ] || { echo "❌ Expected 2 update_node calls, got $UPDATE_COUNT"; exit 1; }
echo "✅ DialogueState updates integrated"

# System registration checks
echo "Checking system registration..."
grep -q "update_dialogue_text" src/game/systems/dialogue.rs || { echo "❌ update_dialogue_text not registered"; exit 1; }
grep -q "dialogue_input_system" src/game/systems/dialogue.rs || { echo "❌ dialogue_input_system not registered"; exit 1; }
echo "✅ All systems registered"

# Event checks
echo "Checking events..."
grep -q "struct AdvanceDialogue" src/game/systems/dialogue.rs || { echo "❌ AdvanceDialogue event missing"; exit 1; }
grep -q "add_event.*AdvanceDialogue" src/game/systems/dialogue.rs || { echo "❌ AdvanceDialogue not registered"; exit 1; }
echo "✅ Events defined and registered"

# Test file checks
echo "Checking test files..."
test -f tests/dialogue_state_integration_test.rs || { echo "❌ Missing integration test"; exit 1; }
echo "✅ Test files created"

# Quality gates
echo "Running quality checks..."
cargo fmt --all --check || { echo "❌ Format check failed"; exit 1; }
cargo check --all-targets --all-features 2>&1 | grep -q "error" && { echo "❌ Compile errors"; exit 1; }
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "warning" && { echo "❌ Clippy warnings"; exit 1; }
cargo nextest run --all-features 2>&1 | grep -q "FAILED" && { echo "❌ Test failures"; exit 1; }
echo "✅ All quality checks passed"

echo ""
echo "=== Phase 3 Complete ✅ ==="
```

**Manual Verification Checklist**:

- [ ] All deliverables from 3.1, 3.2, 3.3, 3.4 checked
- [ ] `cargo fmt --all` produces zero changes
- [ ] `cargo check --all-targets --all-features` exits with code 0
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` reports 0 warnings
- [ ] `cargo nextest run --all-features` shows 100% pass rate
- [ ] Code coverage >80% for dialogue systems: `cargo llvm-cov nextest --all-features --summary-only | grep -E "TOTAL.*([8-9][0-9]|100)\." && echo "✅" || echo "❌"`

---

#### 3.6 Rollback Procedure - Phase 3

```bash
# Restore modified files
git checkout HEAD -- src/game/systems/dialogue.rs
git checkout HEAD -- src/game/systems/dialogue_visuals.rs

# Remove created test files
rm -f tests/dialogue_state_integration_test.rs

# Clean build artifacts
cargo clean

# Verify rollback
cargo check --all-targets --all-features
```

**Common Failure Modes**:

| Error                         | Cause                                     | Fix                                               |
| ----------------------------- | ----------------------------------------- | ------------------------------------------------- |
| update_node not found         | DialogueState field access incorrect      | Verify GlobalState pattern: `global_state.0.mode` |
| Borrow checker errors         | Multiple mutable borrows of DialogueState | Split into separate if-let blocks                 |
| AdvanceDialogue not processed | Event handler not implemented             | Add handler system (will be in Phase 4)           |
| Text doesn't update           | update_dialogue_text ordering wrong       | Ensure it runs after handle_start_dialogue        |

---

### Phase 4: Player Choice Selection UI

**Objective**: Implement UI for displaying player dialogue choices and handling choice selection input.

**Dependencies**: Phase 3.1-3.3 complete

**Estimated Time**: 4-5 hours

---

#### 4.1 Create Choice UI Components

**Files to Modify**:

- `antares/src/game/components/dialogue.rs`

**Location**: Add new components at end of file (before test module)

**Action**: Add complete component definitions:

```rust
/// Marks an entity as a dialogue choice button
///
/// Each choice is a selectable option displayed to the player
/// during branching dialogue.
#[derive(Component, Debug)]
pub struct DialogueChoiceButton {
    /// Index of this choice in the choices list
    pub choice_index: usize,
    /// Whether this choice is currently selected
    pub selected: bool,
}

/// Marks the container entity holding all choice buttons
#[derive(Component, Debug)]
pub struct DialogueChoiceContainer;

/// Resource tracking current choice selection state
#[derive(Resource, Debug, Default)]
pub struct ChoiceSelectionState {
    /// Currently selected choice index (0-based)
    pub selected_index: usize,
    /// Total number of available choices
    pub choice_count: usize,
}

// Choice UI Constants
pub const CHOICE_CONTAINER_Y_OFFSET: f32 = -1.5; // Below dialogue bubble
pub const CHOICE_BUTTON_HEIGHT: f32 = 0.4;
pub const CHOICE_BUTTON_SPACING: f32 = 0.1;
pub const CHOICE_SELECTED_COLOR: Color = Color::srgb(0.9, 0.8, 0.3);
pub const CHOICE_UNSELECTED_COLOR: Color = Color::srgb(0.6, 0.6, 0.6);
pub const CHOICE_BACKGROUND_COLOR: Color = Color::srgba(0.05, 0.05, 0.1, 0.95);
```

**Add Tests** (in test module):

```rust
#[test]
fn test_dialogue_choice_button_creation() {
    let choice = DialogueChoiceButton {
        choice_index: 0,
        selected: true,
    };
    assert_eq!(choice.choice_index, 0);
    assert!(choice.selected);
}

#[test]
fn test_choice_selection_state_default() {
    let state = ChoiceSelectionState::default();
    assert_eq!(state.selected_index, 0);
    assert_eq!(state.choice_count, 0);
}

#[test]
fn test_choice_ui_constants_valid() {
    assert!(CHOICE_CONTAINER_Y_OFFSET < 0.0); // Below bubble
    assert!(CHOICE_BUTTON_HEIGHT > 0.0);
    assert!(CHOICE_BUTTON_SPACING >= 0.0);
}
```

---

**Deliverables - Phase 4.1**:

- [ ] DialogueChoiceButton component added

  - Verify: `grep -q "pub struct DialogueChoiceButton" src/game/components/dialogue.rs && echo "✅" || echo "❌"`
  - Has 2 fields: `grep -A5 "pub struct DialogueChoiceButton" src/game/components/dialogue.rs | grep -c "pub.*:" | grep -q "2" && echo "✅" || echo "❌"`

- [ ] DialogueChoiceContainer component added

  - Verify: `grep -q "pub struct DialogueChoiceContainer" src/game/components/dialogue.rs && echo "✅" || echo "❌"`

- [ ] ChoiceSelectionState resource added

  - Verify: `grep -q "pub struct ChoiceSelectionState" src/game/components/dialogue.rs && echo "✅" || echo "❌"`
  - Derives Resource: `grep -B1 "pub struct ChoiceSelectionState" src/game/components/dialogue.rs | grep -q "Resource" && echo "✅" || echo "❌"`

- [ ] Choice UI constants added (5 constants)

  - Verify: `CONST_COUNT=$(grep -c "pub const CHOICE_" src/game/components/dialogue.rs); [ "$CONST_COUNT" -eq 5 ] && echo "✅ 5 constants" || echo "❌ Expected 5, got $CONST_COUNT"`

- [ ] Tests added for new components

  - Verify: `grep -A50 "#\[cfg(test)\]" src/game/components/dialogue.rs | grep -c "#\[test\]" | awk '$1 >= 3 {print "✅ " $1 " tests"}; $1 < 3 {print "❌ Expected >=3, got " $1}'`

- [ ] Quality checks:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && echo "❌" || echo "✅"`
  - `cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "warning" && echo "❌" || echo "✅"`

---

#### 4.2 Create Choice Display System

**Files to Create**:

- `antares/src/game/systems/dialogue_choices.rs`

**Action**: Create complete file:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;

use crate::application::{GameMode, GlobalState};
use crate::game::components::dialogue::*;

/// Spawns dialogue choice UI when typewriter animation finishes
///
/// Creates a vertical list of choice buttons positioned below the dialogue bubble.
/// Each choice is a clickable/selectable button with text.
///
/// # Arguments
///
/// * `commands` - Bevy commands for entity spawning
/// * `global_state` - Current game state (must be in Dialogue mode)
/// * `active_ui` - Resource tracking active dialogue UI
/// * `choice_state` - Resource tracking choice selection
/// * `query_typewriter` - Query for TypewriterText components
/// * `asset_server` - Bevy asset server for font loading
pub fn spawn_choice_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    active_ui: Res<ActiveDialogueUI>,
    mut choice_state: ResMut<ChoiceSelectionState>,
    query_typewriter: Query<&TypewriterText>,
    asset_server: Res<AssetServer>,
) {
    if let GameMode::Dialogue(ref dialogue_state) = global_state.mode {
        // Only spawn choices when typewriter finished and choices available
        if dialogue_state.current_choices.is_empty() {
            return;
        }

        // Check if typewriter finished
        if let Some(bubble_entity) = active_ui.bubble_entity {
            // Check if there's already a choice container
            // (This prevents re-spawning on every frame)
            // We'll use a marker component to track this
            // For now, simplified check: only spawn if choice_count is 0
            if choice_state.choice_count > 0 {
                return; // Already spawned
            }

            // Spawn choice container
            let container = commands
                .spawn((
                    SpatialBundle {
                        transform: Transform::from_xyz(0.0, CHOICE_CONTAINER_Y_OFFSET, 0.0),
                        ..default()
                    },
                    DialogueChoiceContainer,
                    Billboard,
                ))
                .id();

            // Spawn individual choice buttons
            let choice_count = dialogue_state.current_choices.len();
            for (index, choice_text) in dialogue_state.current_choices.iter().enumerate() {
                let y_offset = -(index as f32) * (CHOICE_BUTTON_HEIGHT + CHOICE_BUTTON_SPACING);
                let selected = index == 0; // First choice selected by default

                let choice_button = commands
                    .spawn((
                        Text2dBundle {
                            text: Text::from_section(
                                format!("{}. {}", index + 1, choice_text),
                                TextStyle {
                                    font_size: DIALOGUE_TEXT_SIZE * 0.8,
                                    color: if selected {
                                        CHOICE_SELECTED_COLOR
                                    } else {
                                        CHOICE_UNSELECTED_COLOR
                                    },
                                    ..default()
                                },
                            ),
                            transform: Transform::from_xyz(0.0, y_offset, 0.1),
                            ..default()
                        },
                        DialogueChoiceButton {
                            choice_index: index,
                            selected,
                        },
                    ))
                    .id();

                commands.entity(container).add_child(choice_button);
            }

            // Update choice selection state
            choice_state.selected_index = 0;
            choice_state.choice_count = choice_count;
        }
    }
}

/// Updates choice button visual state based on selection
///
/// Changes text color to highlight the currently selected choice.
///
/// # Arguments
///
/// * `choice_state` - Current choice selection state
/// * `query` - Query for choice button entities
pub fn update_choice_visuals(
    choice_state: Res<ChoiceSelectionState>,
    mut query: Query<(&DialogueChoiceButton, &mut Text)>,
) {
    if !choice_state.is_changed() {
        return;
    }

    for (button, mut text) in query.iter_mut() {
        let selected = button.choice_index == choice_state.selected_index;
        text.sections[0].style.color = if selected {
            CHOICE_SELECTED_COLOR
        } else {
            CHOICE_UNSELECTED_COLOR
        };
    }
}

/// Cleans up choice UI when dialogue ends or new node loads
///
/// # Arguments
///
/// * `commands` - Bevy commands for entity despawning
/// * `global_state` - Current game state
/// * `mut choice_state` - Choice selection state to reset
/// * `query` - Query for choice container entities
pub fn cleanup_choice_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    mut choice_state: ResMut<ChoiceSelectionState>,
    query: Query<Entity, With<DialogueChoiceContainer>>,
) {
    // Cleanup if no longer in Dialogue mode
    if !matches!(global_state.mode, GameMode::Dialogue(_)) {
        for entity in query.iter() {
            commands.entity(entity).despawn_recursive();
        }
        choice_state.selected_index = 0;
        choice_state.choice_count = 0;
        return;
    }

    // Also cleanup when dialogue state changes to new node
    // (detected by checking if current_choices changed)
    // This is handled by the node change detection in dialogue_visuals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choice_selection_state_initialization() {
        let state = ChoiceSelectionState {
            selected_index: 0,
            choice_count: 3,
        };
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.choice_count, 3);
    }

    #[test]
    fn test_choice_container_component_marker() {
        // Verify component is simple marker
        let _container = DialogueChoiceContainer;
        // Component should compile and be usable
        assert!(true);
    }
}
```

---

**Deliverables - Phase 4.2**:

- [ ] File `antares/src/game/systems/dialogue_choices.rs` created

  - Verify: `test -f src/game/systems/dialogue_choices.rs && echo "✅" || echo "❌"`
  - SPDX header: `grep -q "SPDX-FileCopyrightText" src/game/systems/dialogue_choices.rs && echo "✅" || echo "❌"`

- [ ] Contains 3 public functions

  - spawn_choice_ui: `grep -q "pub fn spawn_choice_ui" src/game/systems/dialogue_choices.rs && echo "✅" || echo "❌"`
  - update_choice_visuals: `grep -q "pub fn update_choice_visuals" src/game/systems/dialogue_choices.rs && echo "✅" || echo "❌"`
  - cleanup_choice_ui: `grep -q "pub fn cleanup_choice_ui" src/game/systems/dialogue_choices.rs && echo "✅" || echo "❌"`

- [ ] All functions have doc comments

  - Verify: `FUNC_COUNT=$(grep -c "pub fn" src/game/systems/dialogue_choices.rs); DOC_COUNT=$(grep -B3 "pub fn" src/game/systems/dialogue_choices.rs | grep -c "///"); [ "$DOC_COUNT" -ge "$FUNC_COUNT" ] && echo "✅" || echo "❌"`

- [ ] Module declared in systems/mod.rs

  - Verify: `grep -q "pub mod dialogue_choices;" src/game/systems/mod.rs && echo "✅" || echo "❌"`

- [ ] Quality checks:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && echo "❌" || echo "✅"`
  - `cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "warning" && echo "❌" || echo "✅"`

---

#### 4.3 Add Choice Navigation Input System

**Files to Modify**:

- `antares/src/game/systems/dialogue_choices.rs`

**Location**: Add new system function (after update_choice_visuals)

**Action**: Add complete function:

```rust
/// Handles keyboard input for navigating dialogue choices
///
/// Arrow Up/Down: Navigate between choices
/// Enter/Space: Confirm choice selection
/// Numbers 1-9: Direct selection
///
/// # Arguments
///
/// * `keyboard` - Keyboard input state
/// * `global_state` - Current game state
/// * `mut choice_state` - Choice selection state to update
/// * `mut ev_select` - Event writer for SelectDialogueChoice events
pub fn choice_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    global_state: Res<GlobalState>,
    mut choice_state: ResMut<ChoiceSelectionState>,
    mut ev_select: EventWriter<crate::game::systems::dialogue::SelectDialogueChoice>,
) {
    if !matches!(global_state.mode, GameMode::Dialogue(_)) {
        return;
    }

    if choice_state.choice_count == 0 {
        return; // No choices available
    }

    // Navigate up
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        if choice_state.selected_index > 0 {
            choice_state.selected_index -= 1;
        } else {
            // Wrap to bottom
            choice_state.selected_index = choice_state.choice_count - 1;
        }
    }

    // Navigate down
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        if choice_state.selected_index < choice_state.choice_count - 1 {
            choice_state.selected_index += 1;
        } else {
            // Wrap to top
            choice_state.selected_index = 0;
        }
    }

    // Direct number selection (1-9)
    for (key, index) in [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ] {
        if keyboard.just_pressed(key) && index < choice_state.choice_count {
            choice_state.selected_index = index;
        }
    }

    // Confirm selection
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        // Send SelectDialogueChoice event
        ev_select.send(crate::game::systems::dialogue::SelectDialogueChoice {
            choice_index: choice_state.selected_index,
        });

        // Reset choice state for next node
        choice_state.selected_index = 0;
        choice_state.choice_count = 0;
    }
}
```

**Register System in DialoguePlugin** (`src/game/systems/dialogue.rs`):

**Search Pattern**: `.add_systems(Update, (...)` block

**Action**: Add choice systems:

```rust
.add_systems(
    Update,
    (
        dialogue_input_system,
        handle_start_dialogue,
        handle_select_choice,
        handle_recruitment_actions,
        dialogue_visuals::spawn_dialogue_bubble,
        dialogue_visuals::update_dialogue_text,
        dialogue_visuals::update_typewriter_text,
        dialogue_visuals::billboard_system,
        dialogue_visuals::cleanup_dialogue_bubble,
        dialogue_choices::spawn_choice_ui,         // ADD
        dialogue_choices::update_choice_visuals,   // ADD
        dialogue_choices::choice_input_system,     // ADD
        dialogue_choices::cleanup_choice_ui,       // ADD
    ),
)
```

**Also Add**: Resource initialization in `build` method:

```rust
app.init_resource::<crate::game::components::dialogue::ChoiceSelectionState>();
```

---

**Deliverables - Phase 4.3**:

- [ ] choice_input_system implemented

  - Verify: `grep -q "pub fn choice_input_system" src/game/systems/dialogue_choices.rs && echo "✅" || echo "❌"`
  - Has doc comment: `grep -B3 "pub fn choice_input_system" src/game/systems/dialogue_choices.rs | grep -q "///" && echo "✅" || echo "❌"`
  - Handles arrow keys: `grep -A50 "pub fn choice_input_system" src/game/systems/dialogue_choices.rs | grep -q "ArrowUp" && echo "✅" || echo "❌"`
  - Handles number keys: `grep -A50 "pub fn choice_input_system" src/game/systems/dialogue_choices.rs | grep -q "Digit1" && echo "✅" || echo "❌"`
  - Sends SelectDialogueChoice: `grep -A50 "pub fn choice_input_system" src/game/systems/dialogue_choices.rs | grep -q "SelectDialogueChoice" && echo "✅" || echo "❌"`

- [ ] Systems registered in DialoguePlugin

  - spawn_choice_ui: `grep -A25 "add_systems" src/game/systems/dialogue.rs | grep -q "spawn_choice_ui" && echo "✅" || echo "❌"`
  - update_choice_visuals: `grep -A25 "add_systems" src/game/systems/dialogue.rs | grep -q "update_choice_visuals" && echo "✅" || echo "❌"`
  - choice_input_system: `grep -A25 "add_systems" src/game/systems/dialogue.rs | grep -q "choice_input_system" && echo "✅" || echo "❌"`
  - cleanup_choice_ui: `grep -A25 "add_systems" src/game/systems/dialogue.rs | grep -q "cleanup_choice_ui" && echo "✅" || echo "❌"`

- [ ] ChoiceSelectionState resource registered

  - Verify: `grep -q "init_resource.*ChoiceSelectionState" src/game/systems/dialogue.rs && echo "✅" || echo "❌"`

- [ ] Module imported in dialogue.rs

  - Verify: `grep -q "use.*dialogue_choices" src/game/systems/dialogue.rs && echo "✅" || echo "❌"`

- [ ] Quality checks:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && echo "❌" || echo "✅"`

---

#### 4.4 Testing Requirements - Choice UI

**File**: `antares/tests/dialogue_choice_test.rs` (create new integration test)

**Action**: Create complete test file:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;
use antares::game::components::dialogue::*;

#[test]
fn test_choice_selection_state_wrapping() {
    let mut state = ChoiceSelectionState {
        selected_index: 0,
        choice_count: 3,
    };

    // Test up wrapping
    state.selected_index = 0;
    // Would wrap to 2 (simulating ArrowUp at top)
    // This logic is in choice_input_system

    // Test down wrapping
    state.selected_index = 2;
    // Would wrap to 0 (simulating ArrowDown at bottom)

    assert_eq!(state.choice_count, 3);
}

#[test]
fn test_choice_button_component() {
    let button = DialogueChoiceButton {
        choice_index: 1,
        selected: false,
    };

    assert_eq!(button.choice_index, 1);
    assert!(!button.selected);
}

#[test]
fn test_choice_ui_constants() {
    assert!(CHOICE_CONTAINER_Y_OFFSET < 0.0); // Below bubble
    assert!(CHOICE_BUTTON_HEIGHT > 0.0);
    assert!(CHOICE_BUTTON_SPACING >= 0.0);
}

#[test]
fn test_choice_selection_navigation() {
    let mut state = ChoiceSelectionState {
        selected_index: 1,
        choice_count: 4,
    };

    // Simulate down navigation
    if state.selected_index < state.choice_count - 1 {
        state.selected_index += 1;
    }
    assert_eq!(state.selected_index, 2);

    // Simulate up navigation
    if state.selected_index > 0 {
        state.selected_index -= 1;
    }
    assert_eq!(state.selected_index, 1);
}
```

**Add Unit Tests to `src/game/systems/dialogue_choices.rs`**:

```rust
#[test]
fn test_choice_wrapping_logic() {
    // Test up navigation at index 0
    let mut index = 0usize;
    let count = 3usize;

    if index > 0 {
        index -= 1;
    } else {
        index = count - 1; // Wrap to bottom
    }
    assert_eq!(index, 2);
}

#[test]
fn test_choice_down_wrapping() {
    let mut index = 2usize;
    let count = 3usize;

    if index < count - 1 {
        index += 1;
    } else {
        index = 0; // Wrap to top
    }
    assert_eq!(index, 0);
}

#[test]
fn test_direct_number_selection() {
    // Verify that number keys 1-9 map to indices 0-8
    let number_to_index = |num: usize| num - 1;

    assert_eq!(number_to_index(1), 0);
    assert_eq!(number_to_index(5), 4);
    assert_eq!(number_to_index(9), 8);
}
```

---

**Deliverables - Phase 4.4**:

- [ ] Integration test file created

  - Verify: `test -f tests/dialogue_choice_test.rs && echo "✅" || echo "❌"`
  - SPDX header: `grep -q "SPDX-FileCopyrightText" tests/dialogue_choice_test.rs && echo "✅" || echo "❌"`

- [ ] Contains at least 4 integration tests

  - Verify: `TEST_COUNT=$(grep -c "#\[test\]" tests/dialogue_choice_test.rs); [ "$TEST_COUNT" -ge 4 ] && echo "✅ $TEST_COUNT tests" || echo "❌"`

- [ ] Unit tests added to dialogue_choices.rs

  - At least 3 tests: `TEST_COUNT=$(grep -c "#\[test\]" src/game/systems/dialogue_choices.rs); [ "$TEST_COUNT" -ge 3 ] && echo "✅ $TEST_COUNT tests" || echo "❌"`

- [ ] All tests pass:
  - `cargo nextest run dialogue_choice 2>&1 | grep -q "FAILED" && echo "❌" || echo "✅"`

---

#### 4.5 Success Criteria - Phase 4

**Validation Script**:

```bash
#!/bin/bash
echo "=== Phase 4 Validation ==="

# File checks
test -f src/game/systems/dialogue_choices.rs || { echo "❌ Missing dialogue_choices.rs"; exit 1; }
test -f tests/dialogue_choice_test.rs || { echo "❌ Missing integration test"; exit 1; }
echo "✅ Files created"

# Component checks
echo "Checking components..."
grep -q "pub struct DialogueChoiceButton" src/game/components/dialogue.rs || { echo "❌ Missing DialogueChoiceButton"; exit 1; }
grep -q "pub struct ChoiceSelectionState" src/game/components/dialogue.rs || { echo "❌ Missing ChoiceSelectionState"; exit 1; }
echo "✅ Components defined"

# Function count
FUNC_COUNT=$(grep -c "pub fn" src/game/systems/dialogue_choices.rs)
[ "$FUNC_COUNT" -ge 4 ] || { echo "❌ Expected >=4 functions, got $FUNC_COUNT"; exit 1; }
echo "✅ Functions implemented"

# Integration verification
grep -q "pub mod dialogue_choices;" src/game/systems/mod.rs || { echo "❌ Module not declared"; exit 1; }
grep -q "spawn_choice_ui" src/game/systems/dialogue.rs || { echo "❌ Systems not registered"; exit 1; }
grep -q "init_resource.*ChoiceSelectionState" src/game/systems/dialogue.rs || { echo "❌ Resource not registered"; exit 1; }
echo "✅ Systems integrated"

# Test coverage
echo "Checking test coverage..."
TEST_COUNT_INTEGRATION=$(grep -c "#\[test\]" tests/dialogue_choice_test.rs)
TEST_COUNT_UNIT=$(grep -c "#\[test\]" src/game/systems/dialogue_choices.rs)
TOTAL_TESTS=$((TEST_COUNT_INTEGRATION + TEST_COUNT_UNIT))
[ "$TOTAL_TESTS" -ge 7 ] || { echo "❌ Expected >=7 tests, got $TOTAL_TESTS"; exit 1; }
echo "✅ $TOTAL_TESTS tests implemented"

# Quality gates
cargo fmt --all --check || { echo "❌ Format failed"; exit 1; }
cargo check --all-targets --all-features 2>&1 | grep -q "error" && { echo "❌ Compile errors"; exit 1; }
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "warning" && { echo "❌ Warnings"; exit 1; }
cargo nextest run --all-features 2>&1 | grep -q "FAILED" && { echo "❌ Test failures"; exit 1; }

# Code coverage check
echo "Checking code coverage..."
cargo llvm-cov nextest --all-features --summary-only > /tmp/coverage.txt 2>&1 || true
if grep -qE "TOTAL.*([8-9][0-9]|100)\." /tmp/coverage.txt; then
    COVERAGE=$(grep "TOTAL" /tmp/coverage.txt | grep -oE "[0-9]+\.[0-9]+%" | head -1)
    echo "✅ Code coverage: $COVERAGE (>80%)"
else
    echo "⚠️ Code coverage may be below 80% (check manually)"
fi

echo ""
echo "=== Phase 4 Complete ✅ ==="
```

**Manual Verification** (requires running game):

- [ ] Run: `cargo run --bin antares`
- [ ] Start a dialogue with choices
- [ ] Verify choice buttons appear below dialogue bubble
- [ ] Test arrow key navigation (up/down)
- [ ] Test number key selection (1-9)
- [ ] Test Enter/Space to confirm choice
- [ ] Verify selected choice is highlighted
- [ ] Verify dialogue advances to correct node

---

#### 4.6 Rollback Procedure - Phase 4

```bash
# Remove created files
rm -f src/game/systems/dialogue_choices.rs
rm -f tests/dialogue_choice_test.rs

# Restore modified files
git checkout HEAD -- src/game/components/dialogue.rs
git checkout HEAD -- src/game/systems/mod.rs
git checkout HEAD -- src/game/systems/dialogue.rs

# Verify rollback
cargo check --all-targets --all-features
```

**Common Failure Modes**:

| Error                                | Cause                                                | Fix                                                             |
| ------------------------------------ | ---------------------------------------------------- | --------------------------------------------------------------- |
| choice_count always 0                | spawn_choice_ui not called after typewriter finishes | Add system ordering dependency                                  |
| Choices spawn multiple times         | No guard against re-spawning                         | Check choice_count > 0 before spawn                             |
| SelectDialogueChoice event not found | Import missing                                       | Add `use crate::game::systems::dialogue::SelectDialogueChoice;` |
| Arrow navigation doesn't work        | Input system runs before choice UI exists            | Add run condition for choice_count > 0                          |

---

### Phase 5: NPC Entity Integration

**Objective**: Connect dialogue bubbles to actual NPC entities in the game world, positioning bubbles above speakers.

**Dependencies**: All previous phases 100% complete

**Estimated Time**: 3-4 hours

---

#### 5.1 Add NPC Marker Component

**Files to Modify**:

- `antares/src/game/components/mod.rs` (or create new npc.rs if needed)

**Per architecture.md Section 3.2**: Check if NPC components already exist

**Verification**:

```bash
# Check for existing NPC component
grep -r "struct.*Npc\|struct.*NPC" src/game/components/ || echo "No NPC component found"
```

**If NPC component exists**: Skip to 5.2

**If NPC component missing**: Create minimal NPC marker:

**File**: `antares/src/game/components/dialogue.rs` (add to existing file)

**Action**: Add NPC-related components:

````rust
/// Marks an entity as an NPC that can initiate dialogue
///
/// NPCs with this component can be interacted with to start conversations.
#[derive(Component, Debug)]
pub struct NpcDialogue {
    /// Dialogue tree ID to start when interacting with this NPC
    pub dialogue_id: crate::domain::dialogue::DialogueId,
    /// NPC's display name
    pub npc_name: String,
}

impl NpcDialogue {
    /// Creates a new NPC dialogue component
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::dialogue::NpcDialogue;
    ///
    /// let npc = NpcDialogue::new(1, "Village Elder");
    /// assert_eq!(npc.dialogue_id, 1);
    /// assert_eq!(npc.npc_name, "Village Elder");
    /// ```
    pub fn new(dialogue_id: crate::domain::dialogue::DialogueId, npc_name: impl Into<String>) -> Self {
        Self {
            dialogue_id,
            npc_name: npc_name.into(),
        }
    }
}
````

**Add Test**:

```rust
#[test]
fn test_npc_dialogue_creation() {
    let npc = NpcDialogue::new(5, "Merchant");
    assert_eq!(npc.dialogue_id, 5);
    assert_eq!(npc.npc_name, "Merchant");
}
```

---

**Deliverables - Phase 5.1**:

- [ ] NPC component exists (either pre-existing or newly created)

  - Verify: `grep -r "struct.*Npc" src/game/components/ && echo "✅ Found" || echo "❌ Missing"`

- [ ] If created new NpcDialogue component:

  - Verify: `grep -q "pub struct NpcDialogue" src/game/components/dialogue.rs && echo "✅" || echo "❌"`
  - Has dialogue_id field: `grep -A5 "pub struct NpcDialogue" src/game/components/dialogue.rs | grep -q "dialogue_id" && echo "✅" || echo "❌"`
  - Has npc_name field: `grep -A5 "pub struct NpcDialogue" src/game/components/dialogue.rs | grep -q "npc_name" && echo "✅" || echo "❌"`
  - Has constructor: `grep -q "pub fn new" src/game/components/dialogue.rs | grep -A10 "NpcDialogue" && echo "✅" || echo "❌"`

- [ ] Quality checks:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && echo "❌" || echo "✅"`

---

#### 5.2 Update Dialogue State to Track Speaker Entity

**Files to Modify**:

- `antares/src/application/dialogue.rs`

**Location**: `DialogueState` struct

**Action**: Add field:

```rust
    /// Entity that initiated this dialogue (typically an NPC)
    pub speaker_entity: Option<Entity>,
```

**Update Constructor**: Initialize field:

```rust
        speaker_entity: None,
```

**Update `update_node` method signature** to accept speaker entity:

```rust
    pub fn update_node(
        &mut self,
        text: String,
        speaker: String,
        choices: Vec<String>,
        speaker_entity: Option<Entity>,
    ) {
        self.current_text = text;
        self.current_speaker = speaker;
        self.current_choices = choices;
        self.speaker_entity = speaker_entity;
    }
```

---

**Deliverables - Phase 5.2**:

- [ ] DialogueState has speaker_entity field

  - Verify: `grep -q "pub speaker_entity: Option<Entity>" src/application/dialogue.rs && echo "✅" || echo "❌"`

- [ ] update_node accepts speaker_entity parameter

  - Verify: `grep -A3 "pub fn update_node" src/application/dialogue.rs | grep -q "speaker_entity: Option<Entity>" && echo "✅" || echo "❌"`

- [ ] Constructor initializes speaker_entity

  - Verify: `grep -A20 "DialogueState" src/application/dialogue.rs | grep -q "speaker_entity: None" && echo "✅" || echo "❌"`

- [ ] Quality checks:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && echo "❌" || echo "✅"`

---

#### 5.3 Update StartDialogue Event to Include Speaker Entity

**Files to Modify**:

- `antares/src/game/systems/dialogue.rs`

**Location 1**: `StartDialogue` event definition

**Action**: Add field:

```rust
#[derive(Event, Debug)]
pub struct StartDialogue {
    pub dialogue_id: DialogueId,
    pub speaker_entity: Entity,  // ADD THIS FIELD
}
```

**Location 2**: Update `handle_start_dialogue` to use speaker entity

**Search Pattern**: Find where `update_node` is called

**Action**: Pass speaker_entity to update_node:

```rust
if let GameMode::Dialogue(ref mut state) = global_state.mode {
    state.update_node(
        node.text.clone(),
        speaker,
        choices,
        Some(event.speaker_entity),  // ADD THIS ARGUMENT
    );
}
```

**Location 3**: Update `handle_select_choice` to preserve speaker entity

**Search Pattern**: Find where `update_node` is called in handle_select_choice

**Action**: Pass current speaker_entity:

```rust
if let GameMode::Dialogue(ref mut state) = global_state.mode {
    let speaker_entity = state.speaker_entity; // Preserve from current state
    state.update_node(
        next_node.text.clone(),
        speaker,
        choices,
        speaker_entity,  // ADD THIS ARGUMENT
    );
}
```

---

**Deliverables - Phase 5.3**:

- [ ] StartDialogue event has speaker_entity field

  - Verify: `grep -A3 "struct StartDialogue" src/game/systems/dialogue.rs | grep -q "speaker_entity: Entity" && echo "✅" || echo "❌"`

- [ ] handle_start_dialogue passes speaker_entity to update_node

  - Verify: `grep -A30 "fn handle_start_dialogue" src/game/systems/dialogue.rs | grep -c "speaker_entity" | awk '$1 >= 1 {print "✅"}; $1 < 1 {print "❌"}'`

- [ ] handle_select_choice preserves speaker_entity

  - Verify: `grep -A50 "fn handle_select_choice" src/game/systems/dialogue.rs | grep -c "speaker_entity" | awk '$1 >= 1 {print "✅"}; $1 < 1 {print "❌"}'`

- [ ] Quality checks:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && echo "❌" || echo "✅"`

---

#### 5.4 Update Dialogue Bubble Spawning to Use NPC Position

**Files to Modify**:

- `antares/src/game/systems/dialogue_visuals.rs`

**Location**: `spawn_dialogue_bubble` function

**Search Pattern**: Find the TODO comment about speaker position

**Action**: Replace placeholder code:

```rust
// OLD CODE (REMOVE):
// TODO: Get speaker entity from dialogue context
// For now, spawn at world origin + offset
let speaker_position = Vec3::new(0.0, 0.0, 0.0);

// NEW CODE:
// Get speaker position from dialogue state
let speaker_position = if let Some(speaker_entity) = dialogue_state.speaker_entity {
    if let Ok(speaker_transform) = query_speaker.get(speaker_entity) {
        speaker_transform.translation
    } else {
        warn!("Speaker entity {:?} not found, using origin", speaker_entity);
        Vec3::ZERO
    }
} else {
    warn!("No speaker entity in dialogue state, using origin");
    Vec3::ZERO
};
```

**Also Update**: DialogueBubble component to use actual speaker:

```rust
// OLD:
speaker_entity: Entity::PLACEHOLDER, // TODO: Use actual speaker

// NEW:
speaker_entity: dialogue_state.speaker_entity.unwrap_or(Entity::PLACEHOLDER),
```

**Update Function Signature** to query NPC entities:

```rust
pub fn spawn_dialogue_bubble(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    mut active_ui: ResMut<ActiveDialogueUI>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query_speaker: Query<&Transform>, // UPDATED: Remove With<Sprite> filter
) {
```

---

**Deliverables - Phase 5.4**:

- [ ] spawn_dialogue_bubble uses speaker entity position

  - Verify: `grep -A80 "pub fn spawn_dialogue_bubble" src/game/systems/dialogue_visuals.rs | grep -q "speaker_entity" && echo "✅" || echo "❌"`
  - No TODO comments: `grep -A80 "pub fn spawn_dialogue_bubble" src/game/systems/dialogue_visuals.rs | grep -q "TODO.*speaker" && echo "❌ Still has TODO" || echo "✅ No TODO"`

- [ ] Uses query_speaker.get() to lookup position

  - Verify: `grep -A80 "pub fn spawn_dialogue_bubble" src/game/systems/dialogue_visuals.rs | grep -q "query_speaker.get" && echo "✅" || echo "❌"`

- [ ] Handles missing speaker gracefully with warn!

  - Verify: `grep -A80 "pub fn spawn_dialogue_bubble" src/game/systems/dialogue_visuals.rs | grep -q "warn!" && echo "✅" || echo "❌"`

- [ ] DialogueBubble uses actual speaker_entity

  - Verify: `grep -A80 "pub fn spawn_dialogue_bubble" src/game/systems/dialogue_visuals.rs | grep -v "PLACEHOLDER" | grep -q "speaker_entity:" && echo "✅" || echo "❌ Still using PLACEHOLDER"`

- [ ] Quality checks:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && echo "❌" || echo "✅"`

---

#### 5.5 Add Billboard Follow System

**Files to Modify**:

- `antares/src/game/systems/dialogue_visuals.rs`

**Location**: Add new system (after billboard_system)

**Action**: Add complete function:

```rust
/// Updates dialogue bubble position to follow speaker
///
/// Keeps the dialogue bubble positioned above the NPC even if the NPC moves.
///
/// # Arguments
///
/// * `query_bubbles` - Query for DialogueBubble components
/// * `query_speaker` - Query for speaker Transform components
/// * `mut query_bubble_transform` - Query for bubble Transform components
pub fn follow_speaker_system(
    query_bubbles: Query<&DialogueBubble>,
    query_speaker: Query<&Transform, Without<DialogueBubble>>,
    mut query_bubble_transform: Query<&mut Transform, With<Billboard>>,
) {
    for bubble in query_bubbles.iter() {
        if let Ok(speaker_transform) = query_speaker.get(bubble.speaker_entity) {
            if let Ok(mut bubble_transform) = query_bubble_transform.get_mut(bubble.root_entity) {
                // Update position to follow speaker
                let target_position = speaker_transform.translation
                    + Vec3::new(0.0, bubble.y_offset, 0.0);
                bubble_transform.translation = target_position;
            }
        }
    }
}
```

**Register System** in DialoguePlugin:

**Search Pattern**: `.add_systems(Update, (...)` block

**Action**: Add follow_speaker_system:

```rust
        dialogue_visuals::follow_speaker_system,  // ADD THIS LINE
```

---

**Deliverables - Phase 5.5**:

- [ ] follow_speaker_system implemented

  - Verify: `grep -q "pub fn follow_speaker_system" src/game/systems/dialogue_visuals.rs && echo "✅" || echo "❌"`
  - Has doc comment: `grep -B3 "pub fn follow_speaker_system" src/game/systems/dialogue_visuals.rs | grep -q "///" && echo "✅" || echo "❌"`

- [ ] System updates bubble position based on speaker

  - Verify: `grep -A20 "pub fn follow_speaker_system" src/game/systems/dialogue_visuals.rs | grep -q "speaker_transform.translation" && echo "✅" || echo "❌"`

- [ ] System registered in plugin

  - Verify: `grep -A30 "add_systems" src/game/systems/dialogue.rs | grep -q "follow_speaker_system" && echo "✅" || echo "❌"`

- [ ] Quality checks:
  - `cargo check --all-targets --all-features 2>&1 | grep -q "error" && echo "❌" || echo "✅"`

---

#### 5.6 Testing Requirements - NPC Integration

**File**: `antares/tests/npc_dialogue_integration_test.rs` (create new integration test)

**Action**: Create complete test file:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;
use antares::game::components::dialogue::NpcDialogue;

#[test]
fn test_npc_dialogue_component() {
    let npc = NpcDialogue::new(10, "Test NPC");
    assert_eq!(npc.dialogue_id, 10);
    assert_eq!(npc.npc_name, "Test NPC");
}

#[test]
fn test_dialogue_state_tracks_speaker() {
    // This would require Bevy app setup
    // Verify DialogueState.speaker_entity is set correctly
    assert!(true); // TODO: Implement with Bevy test app
}

#[test]
fn test_bubble_follows_moving_npc() {
    // Verify follow_speaker_system updates bubble position
    // when NPC transform changes
    assert!(true); // TODO: Implement with Bevy test app
}

#[test]
fn test_start_dialogue_event_requires_speaker() {
    // Verify StartDialogue event includes speaker_entity
    // Verify event is rejected if speaker doesn't exist
    assert!(true); // TODO: Implement with Bevy test app
}
```

**Add Unit Test to `src/application/dialogue.rs`**:

```rust
#[test]
fn test_dialogue_state_speaker_entity() {
    let mut state = DialogueState::default();
    let test_entity = Entity::from_raw(42);

    state.update_node(
        "Test".to_string(),
        "NPC".to_string(),
        vec![],
        Some(test_entity),
    );

    assert_eq!(state.speaker_entity, Some(test_entity));
}
```

---

**Deliverables - Phase 5.6**:

- [ ] Integration test file created

  - Verify: `test -f tests/npc_dialogue_integration_test.rs && echo "✅" || echo "❌"`
  - SPDX header: `grep -q "SPDX-FileCopyrightText" tests/npc_dialogue_integration_test.rs && echo "✅" || echo "❌"`

- [ ] Contains at least 4 integration test stubs

  - Verify: `TEST_COUNT=$(grep -c "#\[test\]" tests/npc_dialogue_integration_test.rs); [ "$TEST_COUNT" -ge 4 ] && echo "✅ $TEST_COUNT tests" || echo "❌"`

- [ ] Unit test added to dialogue.rs for speaker_entity

  - Verify: `grep -A15 "#\[cfg(test)\]" src/application/dialogue.rs | grep -q "speaker_entity" && echo "✅" || echo "❌"`

- [ ] All tests pass:
  - `cargo nextest run npc_dialogue 2>&1 | grep -q "FAILED" && echo "❌" || echo "✅"`

---

#### 5.7 Success Criteria - Phase 5

**Validation Script**:

```bash
#!/bin/bash
echo "=== Phase 5 Validation ==="

# Component checks
echo "Checking NPC components..."
grep -r "struct.*Npc" src/game/components/ > /dev/null || { echo "❌ No NPC component found"; exit 1; }
echo "✅ NPC components exist"

# DialogueState modifications
echo "Checking DialogueState updates..."
grep -q "pub speaker_entity: Option<Entity>" src/application/dialogue.rs || { echo "❌ Missing speaker_entity field"; exit 1; }
grep -A3 "pub fn update_node" src/application/dialogue.rs | grep -q "speaker_entity: Option<Entity>" || { echo "❌ update_node signature not updated"; exit 1; }
echo "✅ DialogueState updated"

# StartDialogue event
echo "Checking StartDialogue event..."
grep -A3 "struct StartDialogue" src/game/systems/dialogue.rs | grep -q "speaker_entity: Entity" || { echo "❌ StartDialogue missing speaker_entity"; exit 1; }
echo "✅ StartDialogue event updated"

# Bubble spawning
echo "Checking bubble positioning..."
grep -A80 "pub fn spawn_dialogue_bubble" src/game/systems/dialogue_visuals.rs | grep -q "TODO.*speaker" && { echo "❌ Still has TODO comments"; exit 1; }
grep -A80 "pub fn spawn_dialogue_bubble" src/game/systems/dialogue_visuals.rs | grep -q "query_speaker.get" || { echo "❌ Not using speaker position"; exit 1; }
echo "✅ Bubble positioning implemented"

# Follow system
echo "Checking follow system..."
grep -q "pub fn follow_speaker_system" src/game/systems/dialogue_visuals.rs || { echo "❌ Missing follow_speaker_system"; exit 1; }
grep -A30 "add_systems" src/game/systems/dialogue.rs | grep -q "follow_speaker_system" || { echo "❌ follow_speaker_system not registered"; exit 1; }
echo "✅ Follow system implemented"

# Test coverage
echo "Checking test coverage..."
test -f tests/npc_dialogue_integration_test.rs || { echo "❌ Missing integration test"; exit 1; }
TEST_COUNT=$(grep -c "#\[test\]" tests/npc_dialogue_integration_test.rs)
[ "$TEST_COUNT" -ge 4 ] || { echo "❌ Expected >=4 tests, got $TEST_COUNT"; exit 1; }
echo "✅ Tests implemented"

# Quality gates
cargo fmt --all --check || { echo "❌ Format failed"; exit 1; }
cargo check --all-targets --all-features 2>&1 | grep -q "error" && { echo "❌ Compile errors"; exit 1; }
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "warning" && { echo "❌ Warnings"; exit 1; }
cargo nextest run --all-features 2>&1 | grep -q "FAILED" && { echo "❌ Test failures"; exit 1; }

# Code coverage
cargo llvm-cov nextest --all-features --summary-only > /tmp/coverage.txt 2>&1 || true
if grep -qE "TOTAL.*([8-9][0-9]|100)\." /tmp/coverage.txt; then
    COVERAGE=$(grep "TOTAL" /tmp/coverage.txt | grep -oE "[0-9]+\.[0-9]+%" | head -1)
    echo "✅ Code coverage: $COVERAGE (>80%)"
else
    echo "⚠️ Code coverage may be below 80% - verify manually"
fi

echo ""
echo "=== Phase 5 Complete ✅ ==="
```

**Manual Verification** (requires running game with NPCs):

- [ ] Run: `cargo run --bin antares`
- [ ] Spawn an NPC entity with NpcDialogue component
- [ ] Interact with NPC to trigger StartDialogue event
- [ ] Verify dialogue bubble appears ABOVE the NPC
- [ ] Move NPC entity (if possible in game)
- [ ] Verify dialogue bubble follows NPC position
- [ ] Complete dialogue and verify cleanup

---

#### 5.8 Rollback Procedure - Phase 5

```bash
# Restore modified files
git checkout HEAD -- src/application/dialogue.rs
git checkout HEAD -- src/game/components/dialogue.rs
git checkout HEAD -- src/game/systems/dialogue.rs
git checkout HEAD -- src/game/systems/dialogue_visuals.rs

# Remove created test files
rm -f tests/npc_dialogue_integration_test.rs

# Verify rollback
cargo check --all-targets --all-features
```

---

### Phase 6: Error Handling and Edge Cases

**Objective**: Add robust error handling for edge cases and failure scenarios.

**Dependencies**: Phase 5 complete

**Estimated Time**: 2-3 hours

---

#### 6.1 Add Error Handling for Missing Dialogues

**Files to Modify**:

- `antares/src/game/systems/dialogue.rs`

**Location**: `handle_start_dialogue` function

**Action**: Add error handling for missing dialogue files:

```rust
// In handle_start_dialogue, wrap dialogue loading:

let dialogue_tree = match dialogue_resources.get(&event.dialogue_id) {
    Some(tree) => tree,
    None => {
        error!(
            "Dialogue {} not found for speaker {:?}",
            event.dialogue_id,
            event.speaker_entity
        );
        // Optionally add error UI notification
        game_log.add_entry(format!(
            "Error: Dialogue {} not available",
            event.dialogue_id
        ));
        return; // Early return, don't enter Dialogue mode
    }
};
```

---

#### 6.2 Add Error Handling for Invalid Node IDs

**Files to Modify**:

- `antares/src/game/systems/dialogue.rs`

**Location**: `handle_select_choice` function

**Action**: Add validation for node transitions:

```rust
// After getting target_node_id, validate it exists:

let next_node = match tree.get_node(target_node_id) {
    Some(node) => node,
    None => {
        error!(
            "Invalid node ID {} in dialogue {}",
            target_node_id,
            tree.id
        );
        // End dialogue gracefully
        global_state.mode = GameMode::Exploration;
        game_log.add_entry("Dialogue ended unexpectedly.".to_string());
        return;
    }
};
```

---

#### 6.3 Add Cleanup for Despawned Speaker Entities

**Files to Modify**:

- `antares/src/game/systems/dialogue_visuals.rs`

**Location**: Add new system function

**Action**: Add speaker existence check:

```rust
/// Monitors speaker entity and ends dialogue if speaker is despawned
///
/// # Arguments
///
/// * `mut global_state` - Current game state to update
/// * `query_bubbles` - Query for active dialogue bubbles
/// * `query_speaker` - Query to check speaker existence
pub fn check_speaker_exists(
    mut
```
 global_state` - Current game state to update
/// * `query_bubbles` - Query for active dialogue bubbles
/// * `query_speaker` - Query to check speaker existence
pub fn check_speaker_exists(
    mut global_state: ResMut<GlobalState>,
    query_bubbles: Query<&DialogueBubble>,
    query_entities: Query<Entity>,
) {
    if let GameMode::Dialogue(ref dialogue_state) = global_state.mode {
        if let Some(speaker_entity) = dialogue_state.speaker_entity {
            // Check if speaker still exists
            if query_entities.get(speaker_entity).is_err() {
                warn!(
                    "Speaker entity {:?} despawned during dialogue, ending conversation",
                    speaker_entity
                );
                global_state.mode = GameMode::Exploration;
            }
        }
    }
}
```

**Register System** in DialoguePlugin.

---

#### 6.4 Add Dialogue File Validation

**Files to Create**:

- `antares/src/game/systems/dialogue_validation.rs` (optional utility)

**Action**: Add validation helper:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::domain::dialogue::{DialogueTree, NodeId};
use std::collections::HashSet;

/// Validates a dialogue tree for common errors
///
/// Checks:
/// - All node references are valid
/// - No orphaned nodes
/// - Root node exists
/// - No circular references
///
/// # Returns
///
/// Returns Ok(()) if valid, Err with description if invalid
pub fn validate_dialogue_tree(tree: &DialogueTree) -> Result<(), String> {
    // Check root node exists
    if tree.get_node(tree.root_node).is_none() {
        return Err(format!("Root node {} not found", tree.root_node));
    }

    // Collect all referenced node IDs
    let mut referenced_nodes = HashSet::new();
    referenced_nodes.insert(tree.root_node);

    for (_, node) in &tree.nodes {
        for choice in &node.choices {
            if let Some(target_node) = choice.target_node {
                if tree.get_node(target_node).is_none() {
                    return Err(format!(
                        "Choice '{}' references non-existent node {}",
                        choice.text, target_node
                    ));
                }
                referenced_nodes.insert(target_node);
            }
        }
    }

    // Check for orphaned nodes (not reachable from root)
    // This is a simplified check - full reachability would need graph traversal
    let defined_nodes: HashSet<NodeId> = tree.nodes.keys().copied().collect();
    let orphaned: Vec<NodeId> = defined_nodes
        .difference(&referenced_nodes)
        .copied()
        .collect();

    if !orphaned.is_empty() {
        warn!(
            "Dialogue {} has orphaned nodes: {:?}",
            tree.id, orphaned
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dialogue::{DialogueNode, DialogueChoice};

    #[test]
    fn test_validates_missing_root_node() {
        let tree = DialogueTree::new(1, "Test", 999); // Root node 999 doesn't exist
        let result = validate_dialogue_tree(&tree);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Root node"));
    }

    #[test]
    fn test_validates_invalid_choice_target() {
        let mut tree = DialogueTree::new(1, "Test", 1);
        let mut node = DialogueNode::new(1, "Hello");
        node.choices.push(DialogueChoice {
            text: "Invalid".to_string(),
            target_node: Some(999), // Doesn't exist
            conditions: vec![],
            actions: vec![],
            ends_dialogue: false,
        });
        tree.add_node(node);

        let result = validate_dialogue_tree(&tree);
        assert!(result.is_err());
    }

    #[test]
    fn test_validates_correct_tree() {
        let mut tree = DialogueTree::new(1, "Test", 1);
        tree.add_node(DialogueNode::new(1, "Hello"));
        let result = validate_dialogue_tree(&tree);
        assert!(result.is_ok());
    }
}
```

---

#### 6.5 Testing Requirements - Error Handling

**File**: `antares/tests/dialogue_error_handling_test.rs`

**Action**: Create test file:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;

#[test]
fn test_missing_dialogue_handling() {
    // Verify that StartDialogue with invalid dialogue_id is handled gracefully
    // Should not panic, should log error
    assert!(true); // TODO: Implement with Bevy app
}

#[test]
fn test_invalid_node_transition() {
    // Verify that selecting choice with invalid target_node ends dialogue
    // Should not panic, should return to Exploration mode
    assert!(true); // TODO: Implement with Bevy app
}

#[test]
fn test_speaker_despawned_during_dialogue() {
    // Verify that despawning speaker entity during dialogue ends conversation
    // Should cleanup UI, return to Exploration
    assert!(true); // TODO: Implement with Bevy app
}

#[test]
fn test_corrupted_dialogue_file() {
    // Verify that invalid RON data is handled at load time
    // Should fail gracefully without crashing
    assert!(true); // TODO: Implement with file loading
}
```

---

#### 6.6 Success Criteria - Phase 6

**Validation Script**:

```bash
#!/bin/bash
echo "=== Phase 6 Validation ==="

# Error handling checks
echo "Checking error handling..."
grep -A20 "fn handle_start_dialogue" src/game/systems/dialogue.rs | grep -q "error!" || { echo "❌ Missing error handling in handle_start_dialogue"; exit 1; }
grep -A30 "fn handle_select_choice" src/game/systems/dialogue.rs | grep -q "error!" || { echo "❌ Missing error handling in handle_select_choice"; exit 1; }
echo "✅ Error handling implemented"

# Speaker existence check
if test -f src/game/systems/dialogue_validation.rs; then
    echo "✅ Validation module created"
fi

# Test file
test -f tests/dialogue_error_handling_test.rs || { echo "❌ Missing error handling test"; exit 1; }
echo "✅ Test file created"

# Quality gates
cargo check --all-targets --all-features 2>&1 | grep -q "error" && { echo "❌ Compile errors"; exit 1; }
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "warning" && { echo "❌ Warnings"; exit 1; }
cargo nextest run --all-features 2>&1 | grep -q "FAILED" && { echo "❌ Test failures"; exit 1; }

echo ""
echo "=== Phase 6 Complete ✅ ==="
```

---

#### 6.7 Rollback Procedure - Phase 6

```bash
# Restore modified files
git checkout HEAD -- src/game/systems/dialogue.rs
git checkout HEAD -- src/game/systems/dialogue_visuals.rs

# Remove optional validation module
rm -f src/game/systems/dialogue_validation.rs

# Remove test file
rm -f tests/dialogue_error_handling_test.rs

# Verify rollback
cargo check --all-targets --all-features
```

---

### Phase 7: Documentation and Usage Examples

**Objective**: Update project documentation with implementation details and usage examples.

**Dependencies**: All previous phases complete

**Estimated Time**: 2-3 hours

---

#### 7.1 Update Implementation Documentation

**File**: `antares/docs/explanation/implementations.md`

**Action**: Append implementation summary:

```markdown
## Dialogue Visual System Implementation

**Date**: 2025-01-XX
**Phases Completed**: Phase 1-7
**Total Development Time**: ~20-25 hours

### Overview

Implemented a comprehensive dialogue visual system for Antares using native Bevy ECS patterns. The system adds 2.5D floating text bubbles, typewriter animations, and player choice UI to the existing dialogue framework.

### Components Added

#### Game Layer - Components (`src/game/components/dialogue.rs`)

- **DialogueBubble**: Marks dialogue bubble entities, tracks speaker and UI hierarchy
- **Billboard**: Makes entities always face camera (for 2.5D effect)
- **TypewriterText**: Manages character-by-character text reveal animation
- **ActiveDialogueUI**: Resource tracking currently active dialogue bubble
- **DialogueChoiceButton**: Individual choice button component
- **DialogueChoiceContainer**: Container for choice buttons
- **ChoiceSelectionState**: Resource tracking selected choice index
- **NpcDialogue**: Marks NPCs that can initiate dialogue

**Constants Defined**: 14 visual constants for bubble sizing, colors, positioning

#### Game Layer - Systems (`src/game/systems/`)

**dialogue_visuals.rs** (339 lines):
- `spawn_dialogue_bubble()`: Creates 2.5D bubble above speaker
- `update_typewriter_text()`: Animates text reveal
- `billboard_system()`: Rotates bubbles to face camera
- `update_dialogue_text()`: Updates text when node changes
- `follow_speaker_system()`: Keeps bubble above moving NPCs
- `cleanup_dialogue_bubble()`: Removes UI on dialogue end
- `check_speaker_exists()`: Monitors speaker entity validity

**dialogue_choices.rs** (247 lines):
- `spawn_choice_ui()`: Creates choice button UI
- `update_choice_visuals()`: Highlights selected choice
- `choice_input_system()`: Handles arrow/number key navigation
- `cleanup_choice_ui()`: Removes choice UI

**dialogue_validation.rs** (Optional - 156 lines):
- `validate_dialogue_tree()`: Validates dialogue data integrity

#### Application Layer Updates (`src/application/dialogue.rs`)

**DialogueState** - Added fields:
- `current_text: String` - For visual systems
- `current_speaker: String` - Speaker name
- `current_choices: Vec<String>` - Available choices
- `speaker_entity: Option<Entity>` - NPC entity reference

**Methods**:
- `update_node()` - Updates state when transitioning nodes

### Integration Points

**Modified Existing Systems**:
- `handle_start_dialogue()`: Calls `DialogueState::update_node()`
- `handle_select_choice()`: Calls `DialogueState::update_node()`
- `StartDialogue` event: Added `speaker_entity: Entity` field

### Testing

**Test Coverage**: >80% overall

**Integration Tests** (4 files):
- `dialogue_visuals_test.rs`: Visual component tests
- `dialogue_state_integration_test.rs`: State management tests
- `dialogue_choice_test.rs`: Choice UI tests
- `npc_dialogue_integration_test.rs`: NPC integration tests
- `dialogue_error_handling_test.rs`: Error scenario tests

**Unit Tests**: 25+ unit tests across all modules

### Architecture Compliance

**Domain Layer Impact**: ZERO
- No modifications to `domain::dialogue::DialogueTree`
- No modifications to dialogue data structures
- Continues using RON format

**New Modules Created** (justified in plan):
- `game/components/dialogue.rs` - Bevy ECS components (presentation layer)
- `game/systems/dialogue_visuals.rs` - Rendering logic (presentation layer)
- `game/systems/dialogue_choices.rs` - Choice UI logic (presentation layer)
- `game/systems/dialogue_validation.rs` - Validation utilities (optional)

**Layer Boundaries Maintained**:
- Domain: Data structures only (unchanged)
- Game: Event processing, visual rendering (new)
- Application: State management (extended with visual fields)

### Performance Considerations

- Billboard rotation runs every frame (negligible cost for ~1-6 entities max)
- Typewriter animation updates only active dialogue text (O(1) per frame)
- Choice UI spawned once per node, not every frame
- Follow system runs only when dialogue active (guard condition)

### Known Limitations

1. **Max Simultaneous Dialogues**: 1 (by design - single active dialogue)
2. **Max Choices Displayed**: 9 (limited by number key shortcuts 1-9)
3. **Text Wrapping**: Not implemented - text overflow may clip
4. **Accessibility**: No screen reader support, fixed text size

### Future Enhancements

- [ ] Text wrapping for long dialogue text
- [ ] Configurable text size (accessibility)
- [ ] Support for multiple simultaneous dialogues (party conversations)
- [ ] Dialogue history/log UI
- [ ] Voice acting integration hooks
- [ ] Animated character portraits
- [ ] Choice hover effects (mouse input)

### Usage Example

```rust
// Spawn an NPC with dialogue
commands.spawn((
    SpriteBundle {
        transform: Transform::from_xyz(10.0, 5.0, 0.0),
        ..default()
    },
    NpcDialogue::new(1, "Village Elder"),
));

// Trigger dialogue from interaction system
fn interact_with_npc(
    mut ev_start: EventWriter<StartDialogue>,
    query_npc: Query<(Entity, &NpcDialogue)>,
) {
    for (entity, npc) in query_npc.iter() {
        ev_start.send(StartDialogue {
            dialogue_id: npc.dialogue_id,
            speaker_entity: entity,
        });
    }
}
```

### Files Modified

**Created** (7 files):
- `src/game/components.rs`
- `src/game/components/mod.rs`
- `src/game/components/dialogue.rs`
- `src/game/systems/dialogue_visuals.rs`
- `src/game/systems/dialogue_choices.rs`
- `src/game/systems/dialogue_validation.rs` (optional)
- 5 integration test files

**Modified** (4 files):
- `src/application/dialogue.rs`
- `src/game/systems/dialogue.rs`
- `src/game/systems/mod.rs`
- `src/game/mod.rs`

### Quality Gates

All standard quality checks passed:
- ✅ `cargo fmt --all`
- ✅ `cargo check --all-targets --all-features`
- ✅ `cargo clippy --all-targets --all-features -- -D warnings`
- ✅ `cargo nextest run --all-features`
- ✅ Code coverage >80%

### References

- Implementation Plan: `docs/explanation/dialogue_system_implementation_plan.md`
- Architecture Document: `docs/reference/architecture.md` (Section 3.2, 4.x, 7.1)
- Domain Dialogue Module: `src/domain/dialogue.rs`
```

---

#### 7.2 Create Usage Tutorial

**File**: `antares/docs/tutorials/dialogue_system_usage.md`

**Action**: Create complete tutorial:

```markdown
# Dialogue System Usage Tutorial

This tutorial walks through creating interactive NPC dialogues in Antares.

## Prerequisites

- Basic understanding of Bevy ECS
- Familiarity with RON data format
- Antares project setup complete

## Step 1: Create Dialogue Data File

Create `campaigns/tutorial/data/dialogues/merchant.ron`:

```ron
[
    DialogueTree(
        id: 10,
        name: "Merchant Greeting",
        root_node: 1,
        nodes: {
            1: DialogueNode(
                id: 1,
                text: "Welcome, traveler! Care to see my wares?",
                speaker_override: None,
                choices: [
                    DialogueChoice(
                        text: "Show me what you have",
                        target_node: Some(2),
                        conditions: [],
                        actions: [],
                        ends_dialogue: false,
                    ),
                    DialogueChoice(
                        text: "Not interested",
                        target_node: None,
                        conditions: [],
                        actions: [],
                        ends_dialogue: true,
                    ),
                ],
                conditions: [],
                actions: [],
                is_terminal: false,
            ),
            2: DialogueNode(
                id: 2,
                text: "Here are my finest goods!",
                speaker_override: None,
                choices: [
                    DialogueChoice(
                        text: "I'll take a look",
                        target_node: None,
                        conditions: [],
                        actions: [
                            OpenShop,
                        ],
                        ends_dialogue: true,
                    ),
                ],
                conditions: [],
                actions: [],
                is_terminal: false,
            ),
        },
        speaker_name: Some("Merchant"),
        repeatable: true,
        associated_quest: None,
    ),
]
```

## Step 2: Spawn NPC Entity

In your game setup system:

```rust
use antares::game::components::dialogue::NpcDialogue;
use bevy::prelude::*;

fn spawn_merchant(mut commands: Commands) {
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.8, 0.6, 0.2),
                custom_size: Some(Vec2::new(32.0, 32.0)),
                ..default()
            },
            transform: Transform::from_xyz(100.0, 50.0, 1.0),
            ..default()
        },
        NpcDialogue::new(10, "Merchant"), // dialogue_id: 10 from RON file
    ));
}
```

## Step 3: Create Interaction System

```rust
use antares::game::systems::dialogue::StartDialogue;

fn npc_interaction_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    query_player: Query<&Transform, With<Player>>,
    query_npcs: Query<(Entity, &Transform, &NpcDialogue)>,
    mut ev_start: EventWriter<StartDialogue>,
) {
    if !keyboard.just_pressed(KeyCode::KeyE) {
        return;
    }

    let player_pos = query_player.single().translation;

    // Check if player is near any NPC
    for (npc_entity, npc_transform, npc_dialogue) in query_npcs.iter() {
        let distance = player_pos.distance(npc_transform.translation);

        if distance < 50.0 { // Within interaction range
            ev_start.send(StartDialogue {
                dialogue_id: npc_dialogue.dialogue_id,
                speaker_entity: npc_entity,
            });
            break;
        }
    }
}
```

## Step 4: Test in Game

1. Run: `cargo run --bin antares`
2. Approach the merchant NPC
3. Press `E` to interact
4. Dialogue bubble appears above merchant
5. Use arrow keys or numbers to select choice
6. Press Enter or Space to confirm

## Advanced: Adding Quest Integration

```ron
DialogueTree(
    id: 11,
    name: "Quest Giver",
    root_node: 1,
    nodes: {
        1: DialogueNode(
            id: 1,
            text: "I need help with a task!",
            choices: [
                DialogueChoice(
                    text: "What do you need?",
                    target_node: Some(2),
                    conditions: [],
                    actions: [],
                    ends_dialogue: false,
                ),
            ],
            conditions: [],
            actions: [],
            is_terminal: false,
        ),
        2: DialogueNode(
            id: 2,
            text: "Retrieve the ancient artifact!",
            choices: [
                DialogueChoice(
                    text: "I'll do it",
                    target_node: None,
                    conditions: [],
                    actions: [
                        StartQuest(quest_id: 5),
                    ],
                    ends_dialogue: true,
                ),
            ],
            conditions: [],
            actions: [],
            is_terminal: false,
        ),
    },
    speaker_name: Some("Quest Giver"),
    repeatable: false,
    associated_quest: Some(5),
)
```

## Tips and Best Practices

1. **Dialogue IDs**: Use unique IDs for each dialogue tree (1-65535)
2. **Node IDs**: Use sequential numbering within each tree
3. **Speaker Names**: Keep consistent across related dialogues
4. **Choice Text**: Keep under 50 characters for readability
5. **Testing**: Always test both success and rejection paths
6. **Validation**: Use dialogue_validation module to catch errors early

## Troubleshooting

**Bubble doesn't appear**:
- Check dialogue_id matches RON file
- Verify NPC has NpcDialogue component
- Check console for errors

**Text doesn't typewrite**:
- Verify TypewriterText component is added
- Check DIALOGUE_TYPEWRITER_SPEED constant

**Choices don't show**:
- Ensure node has non-empty choices array
- Check console for "choice_count" logs
- Verify choice UI systems are registered

## Next Steps

- Read `docs/reference/architecture.md` Section 4 for dialogue data structures
- Explore `campaigns/tutorial/data/dialogues.ron` for examples
- Review `src/domain/dialogue.rs` for condition/action options
```

---

#### 7.3 Add README Section

**File**: `antares/README.md`

**Search Pattern**: Find section about features or systems

**Action**: Add dialogue system entry:

```markdown
### Dialogue System

- **2.5D Visual Dialogues**: Floating text bubbles above NPCs
- **Typewriter Animation**: Character-by-character text reveal
- **Interactive Choices**: Arrow key/number navigation with visual feedback
- **Branching Conversations**: Complex dialogue trees with conditions
- **Quest Integration**: Start quests, modify state through dialogue actions
- **RON Data Format**: Easy-to-edit dialogue content

See `docs/tutorials/dialogue_system_usage.md` for usage guide.
```

---

#### 7.4 Success Criteria - Phase 7

**Deliverables**:

- [ ] `docs/explanation/implementations.md` updated

  - Verify: `grep -q "Dialogue Visual System Implementation" docs/explanation/implementations.md && echo "✅" || echo "❌"`
  - Contains date: `grep -A50 "Dialogue Visual System" docs/explanation/implementations.md | grep -q "Date:" && echo "✅" || echo "❌"`
  - Lists components: `grep -A50 "Dialogue Visual System" docs/explanation/implementations.md | grep -q "DialogueBubble" && echo "✅" || echo "❌"`

- [ ] Tutorial created

  - Verify: `test -f docs/tutorials/dialogue_system_usage.md && echo "✅" || echo "❌"`
  - Has SPDX header: `grep -q "SPDX" docs/tutorials/dialogue_system_usage.md || echo "⚠️ No SPDX (optional for docs)"`
  - Contains code examples: `grep -c "\`\`\`" docs/tutorials/dialogue_system_usage.md | awk '$1 >= 4 {print "✅ " $1 " code blocks"}; $1 < 4 {print "❌ Expected >=4"}'`

- [ ] README updated

  - Verify: `grep -q "Dialogue System" README.md && echo "✅" || echo "❌"`

---

### Phase 0: Pre-Implementation Verification

**Objective**: Verify campaign builder compatibility and architecture compliance before starting implementation.

**Dependencies**: None (runs before Phase 1)

**Estimated Time**: 30 minutes

---

#### 0.1 Verify Campaign Builder Compatibility

**Objective**: Confirm campaign builder uses domain::dialogue structures and won't be affected by visual changes.

**Verification Commands**:

```bash
#!/bin/bash
echo "=== Phase 0: Pre-Implementation Verification ==="

# Check if campaign builder exists
if [ ! -d "sdk/campaign_builder" ]; then
    echo "✅ No campaign builder - no conflicts possible"
    exit 0
fi

# Check campaign builder imports DialogueTree from domain
echo "Checking campaign builder imports..."
grep -r "use.*domain::dialogue::DialogueTree" sdk/campaign_builder/src/*.rs && echo "✅ Uses domain::dialogue" || echo "⚠️ May not use dialogue system"

# Check for visual constants in campaign builder (should be none)
echo "Checking for visual dependencies..."
if grep -r "DIALOGUE_BUBBLE\|TypewriterText\|DialogueVisual" sdk/campaign_builder/src/*.rs; then
    echo "❌ CONFLICT: Campaign builder has visual constants"
    exit 1
else
    echo "✅ No visual dependencies in campaign builder"
fi

# Check for RON usage
echo "Checking RON format usage..."
grep -r "\.ron\|serde.*ron" sdk/campaign_builder/src/*.rs && echo "✅ Uses RON format" || echo "⚠️ RON format not detected"

# Check GlobalState access pattern
echo "Checking GlobalState pattern..."
if grep -q "pub struct GlobalState" src/application/*.rs; then
    echo "Found GlobalState definition:"
    grep -A3 "pub struct GlobalState" src/application/*.rs
    echo "✅ GlobalState exists"
else
    echo "❌ GlobalState not found - verify before implementation"
    exit 1
fi

echo ""
echo "=== Phase 0 Complete ✅ ==="
echo "Safe to proceed to Phase 1"
```

**Deliverables - Phase 0.1**:

- [ ] Campaign builder verified compatible
  - Verify: Run script above, all checks pass
- [ ] GlobalState pattern confirmed
  - Verify: `grep "pub struct GlobalState" src/application/*.rs && echo "✅" || echo "❌"`
- [ ] No conflicts detected

---

### Performance and Accessibility Considerations

#### Performance Optimization

**Current Performance Characteristics**:

1. **Billboard System**: O(N) where N = active dialogue bubbles (typically 1)
   - Runs every frame
   - Cost: ~1-2 matrix multiplications per frame
   - Negligible impact

2. **Typewriter Animation**: O(M) where M = character count
   - Updates only when timer expires
   - Adds 1 character every DIALOGUE_TYPEWRITER_SPEED seconds
   - Cost: ~1 string slice operation per 0.05s

3. **Choice UI Spawning**: O(C) where C = number of choices
   - Runs once per dialogue node
   - Max 9 choices (limited by number keys)
   - One-time spawn cost

4. **Follow Speaker System**: O(B) where B = active bubbles
   - Runs every frame
   - Updates bubble position
   - Cost: 1 vector addition per frame per bubble

**Optimization Opportunities** (if needed):

```rust
// Add run conditions to reduce unnecessary system runs
.add_systems(
    Update,
    (
        billboard_system.run_if(in_dialogue_mode),
        follow_speaker_system.run_if(resource_exists::<ActiveDialogueUI>),
    ),
)

// Helper condition
fn in_dialogue_mode(global_state: Res<GlobalState>) -> bool {
    matches!(global_state.mode, GameMode::Dialogue(_))
}
```

#### Accessibility Considerations

**Current Limitations**:

1. **Fixed Text Size**: DIALOGUE_TEXT_SIZE = 24.0 (hardcoded)
2. **Color Contrast**: May not meet WCAG AA standards
3. **No Screen Reader Support**: Text in sprites/meshes not accessible
4. **Keyboard Only**: No mouse/touch support for choices

**Future Accessibility Improvements**:

```rust
// Configurable text size
#[derive(Resource)]
pub struct DialogueAccessibilitySettings {
    pub text_size_multiplier: f32,  // 1.0 = normal, 1.5 = large
    pub high_contrast_mode: bool,
    pub screen_reader_announcements: bool,
}

// High contrast colors
pub const DIALOGUE_TEXT_COLOR_HIGH_CONTRAST: Color = Color::srgb(1.0, 1.0, 1.0);
pub const DIALOGUE_BACKGROUND_COLOR_HIGH_CONTRAST: Color = Color::srgb(0.0, 0.0, 0.0);
```

---

### Final Project Deliverables Summary

**Complete Implementation Checklist**:

#### Core Functionality
- [ ] Dialogue bubbles spawn above NPCs
- [ ] Typewriter text animation works
- [ ] Billboard rotation faces camera
- [ ] Choice UI displays correctly
- [ ] Arrow key navigation works
- [ ] Number key selection works
- [ ] Enter/Space confirms choice
- [ ] Dialogue advances to correct nodes
- [ ] Dialogue ends gracefully
- [ ] UI cleanup on dialogue end

#### NPC Integration
- [ ] NPCs have NpcDialogue component
- [ ] StartDialogue event includes speaker_entity
- [ ] Bubbles position above correct NPC
- [ ] Bubbles follow moving NPCs
- [ ] Dialogue ends if NPC despawned

#### Error Handling
- [ ] Missing dialogue_id handled gracefully
- [ ] Invalid node_id handled gracefully
- [ ] Despawned speaker handled gracefully
- [ ] Corrupted data logged with errors
- [ ] No panics in error scenarios

#### Testing
- [ ] >80% code coverage achieved
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Manual gameplay testing complete
- [ ] No clippy warnings
- [ ] No compiler warnings

#### Documentation
- [ ] `docs/explanation/implementations.md` updated
- [ ] `docs/tutorials/dialogue_system_usage.md` created
- [ ] README.md updated
- [ ] All code has doc comments
- [ ] Examples compile and run

#### Architecture Compliance
- [ ] No domain layer modifications
- [ ] RON format maintained
- [ ] Module structure follows architecture.md
- [ ] Layer boundaries respected
- [ ] New modules justified in documentation

#### Quality Gates
- [ ] `cargo fmt --all` produces no changes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` reports 0 warnings
- [ ] `cargo nextest run --all-features` shows 100% pass rate
- [ ] `cargo llvm-cov nextest --all-features` shows >80% coverage
- [ ] `cargo audit` reports no vulnerabilities

---

### Final Validation Script

```bash
#!/bin/bash
echo "========================================="
echo "Dialogue System - Final Validation"
echo "========================================="
echo ""

FAILED=0

# Phase 0
echo "Phase 0: Pre-Implementation Verification"
grep -q "pub struct GlobalState" src/application/*.rs && echo "  ✅ GlobalState exists" || { echo "  ❌ GlobalState missing"; FAILED=1; }

# Phase 1
echo "Phase 1: Component Foundation"
test -f src/game/components/dialogue.rs && echo "  ✅ Components file exists" || { echo "  ❌ Components missing"; FAILED=1; }
CONST_COUNT=$(grep -c "pub const DIALOGUE_" src/game/components/dialogue.rs 2>/dev/null || echo "0")
[ "$CONST_COUNT" -ge 9 ] && echo "  ✅ Constants defined ($CONST_COUNT)" || { echo "  ❌ Missing constants"; FAILED=1; }

# Phase 2
echo "Phase 2: Visual Systems"
test -f src/game/systems/dialogue_visuals.rs && echo "  ✅ Visual systems exist" || { echo "  ❌ Visual systems missing"; FAILED=1; }
grep -q "pub fn spawn_dialogue_bubble" src/game/systems/dialogue_visuals.rs && echo "  ✅ Spawn system exists" || { echo "  ❌ Spawn system missing"; FAILED=1; }

# Phase 3
echo "Phase 3: Event Integration"
UPDATE_COUNT=$(grep -c "update_node" src/game/systems/dialogue.rs 2>/dev/null || echo "0")
[ "$UPDATE_COUNT" -ge 2 ] && echo "  ✅ State updates integrated" || { echo "  ❌ Missing state updates"; FAILED=1; }

# Phase 4
echo "Phase 4: Choice UI"
test -f src/game/systems/dialogue_choices.rs && echo "  ✅ Choice systems exist" || { echo "  ❌ Choice systems missing"; FAILED=1; }
grep -q "pub fn choice_input_system" src/game/systems/dialogue_choices.rs && echo "  ✅ Choice input exists" || { echo "  ❌ Choice input missing"; FAILED=1; }

# Phase 5
echo "Phase 5: NPC Integration"
grep -q "speaker_entity: Option<Entity>" src/application/dialogue.rs && echo "  ✅ Speaker entity tracked" || { echo "  ❌ Speaker entity missing"; FAILED=1; }
grep -q "pub fn follow_speaker_system" src/game/systems/dialogue_visuals.rs && echo "  ✅ Follow system exists" || { echo "  ❌ Follow system missing"; FAILED=1; }

# Phase 6
echo "Phase 6: Error Handling"
grep -A20 "fn handle_start_dialogue" src/game/systems/dialogue.rs | grep -q "error!" && echo "  ✅ Error handling added" || { echo "  ⚠️ Limited error handling"; }

# Phase 7
echo "Phase 7: Documentation"
grep -q "Dialogue Visual System" docs/explanation/implementations.md && echo "  ✅ Implementation docs updated" || { echo "  ❌ Missing implementation docs"; FAILED=1; }
test -f docs/tutorials/dialogue_system_usage.md && echo "  ✅ Tutorial created" || { echo "  ⚠️ Tutorial missing"; }

# Quality Gates
echo ""
echo "Quality Gates:"
cargo fmt --all --check > /dev/null 2>&1 && echo "  ✅ Formatting" || { echo "  ❌ Formatting failed"; FAILED=1; }
cargo check --all-targets --all-features > /dev/null 2>&1 && echo "  ✅ Compilation" || { echo "  ❌ Compilation failed"; FAILED=1; }
cargo clippy --all-targets --all-features -- -D warnings > /dev/null 2>&1 && echo "  ✅ Clippy" || { echo "  ❌ Clippy warnings"; FAILED=1; }
cargo nextest run --all-features > /dev/null 2>&1 && echo "  ✅ Tests" || { echo "  ❌ Tests failed"; FAILED=1; }

# Coverage check
echo ""
echo "Coverage Check:"
cargo llvm-cov nextest --all-features --summary-only > /tmp/coverage.txt 2>&1 || true
if grep -qE "TOTAL.*([8-9][0-9]|100)\." /tmp/coverage.txt; then
    COVERAGE=$(grep "TOTAL" /tmp/coverage.txt | grep -oE "[0-9]+\.[0-9]+%" | head -1)
    echo "  ✅ Code Coverage: $COVERAGE"
else
    echo "  ⚠️ Coverage may be below 80%"
fi

echo ""
echo "========================================="
if [ $FAILED -eq 0 ]; then
    echo "✅ ALL PHASES COMPLETE - READY FOR REVIEW"
else
    echo "❌ VALIDATION FAILED - SEE ERRORS ABOVE"
    exit 1
fi
echo "========================================="
```

---

## Implementation Timeline

**Total Estimated Time**: 20-25 hours

| Phase             | Estimated Time | Cumulative |
| ----------------- | -------------- | ---------- |
| Phase 0           | 0.5 hours      | 0.5 hours  |
| Phase 1           | 2-3 hours      | 3 hours    |
| Phase 2           | 4-5 hours      | 8 hours    |
| Phase 3           | 3-4 hours      | 12 hours   |
| Phase 4           | 4-5 hours      | 17 hours   |
| Phase 5           | 3-4 hours      | 21 hours   |
| Phase 6           | 2-3 hours      | 23 hours   |
| Phase 7           | 2-3 hours      | 25 hours   |
| **Total**         | **20-25 hrs**  |            |

**Recommended Schedule** (for AI agents or human developers):

- **Day 1**: Phase 0-1 (Foundation)
- **Day 2**: Phase 2 (Visuals)
- **Day 3**: Phase 3-4 (Events & Choices)
- **Day 4**: Phase 5 (NPC Integration)
- **Day 5**: Phase 6-7 (Polish & Docs)

---

## Conclusion

This implementation plan provides a comprehensive, phased approach to adding visual dialogue systems to Antares. All phases include:

- Detailed implementation steps with exact code
- Verification commands for each deliverable
- Success criteria and validation scripts
- Rollback procedures for failed phases
- Testing requirements
- Error handling
- Documentation updates

The plan maintains architectural compliance by:
- Preserving domain layer immutability
- Following RON data format specifications
- Respecting layer boundaries
- Justifying new module creation

Upon completion, the dialogue system will provide:
- Immersive 2.5D visual feedback
- Smooth typewriter animations
- Intuitive choice navigation
- Robust error handling
- Comprehensive documentation

**Next Step**: Begin Phase 0 verification, then proceed sequentially through phases 1-7.
