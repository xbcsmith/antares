# Game Menu Implementation - Quick Reference Guide

**Plan Document**: `docs/explanation/game_menu_implementation_plan.md` (3,051 lines)
**Review Summary**: `docs/explanation/game_menu_plan_review_summary.md`
**Status**: ✅ APPROVED FOR IMPLEMENTATION

---

## Quick Start

### For AI Agents

```bash
# Read the full plan
cat docs/explanation/game_menu_implementation_plan.md

# Start with Phase 1
# Follow each subsection (1.1, 1.2, etc.) in order
# Run validation commands after each subsection
# Check deliverables before moving to next phase
```

### For Human Developers

1. Read **Current State Analysis** section (lines 7-139)
2. Read **Architectural Decisions** section (lines 141-252)
3. Implement phases sequentially (1 → 7)
4. Run quality checks after each phase

---

## Implementation Order (MANDATORY)

```
Phase 1: Core State Infrastructure    → 4 hours  (8 deliverables)
Phase 2: Components & Plugin           → 3 hours  (8 deliverables)
Phase 3: Input Integration            → 4 hours  (7 deliverables)
Phase 4: UI Rendering                 → 5 hours  (10 deliverables)
Phase 5: Save/Load Integration        → 6 hours  (9 deliverables)
Phase 6: Settings Integration         → 4 hours  (8 deliverables)
Phase 7: Documentation                → 3 hours  (6 deliverables)
```

**TOTAL**: 25-30 hours

---

## Files to Create

### Application Layer
- `src/application/menu.rs` (NEW, 150 lines) - MenuState definition
- `tests/unit/menu_state_test.rs` (NEW, 8 tests)

### Game Layer - Components
- `src/game/components/menu.rs` (NEW, 100 lines) - UI components
- `tests/unit/menu_components_test.rs` (NEW, 4 tests)

### Game Layer - Systems
- `src/game/systems/menu.rs` (NEW, 800+ lines) - MenuPlugin & UI
- `tests/integration/menu_toggle_test.rs` (NEW, 7 tests)
- `tests/integration/menu_ui_test.rs` (NEW, 6 tests)
- `tests/integration/menu_save_load_test.rs` (NEW, 5 tests)
- `tests/integration/menu_settings_test.rs` (NEW, 4 tests)

### Documentation
- `docs/how-to/using_game_menu.md` (NEW, user guide)
- `docs/explanation/game_menu_plan_review_summary.md` (CREATED)
- `docs/explanation/game_menu_quick_reference.md` (THIS FILE)

---

## Files to Modify

### Core Changes
```
src/application/mod.rs
├── Line 10: Add pub mod menu;
├── Line 25: Add pub use menu::{MenuState, MenuType, SaveGameInfo};
├── Line 44: Change Menu, to Menu(MenuState),
├── Line 330: Add pub config: GameConfig field with #[serde(default)]
└── Lines 350-370: Update new() and new_game() to initialize config
```

### Module Registration
```
src/game/components.rs
├── Add pub mod menu;
└── Add pub use menu::*;

src/game/systems/mod.rs
└── Line 12: Add pub mod menu;

src/game/mod.rs
└── Add MenuPlugin to app.add_plugins()
```

### Input System
```
src/game/systems/input.rs
└── Lines ~250-280: Add menu toggle handler in handle_input()
```

### Documentation
```
docs/reference/architecture.md
├── Section 4.1: Update GameMode enum
└── Section 5: Add subsection 5.6 Menu System

README.md
├── Features section: Add menu system
└── Add keyboard controls table
```

---

## Data Structures

### MenuState (Application Layer)
```rust
pub struct MenuState {
    pub previous_mode: Box<GameMode>,
    pub current_submenu: MenuType,
    pub selected_index: usize,
    pub save_list: Vec<SaveGameInfo>,
}

pub enum MenuType { Main, SaveLoad, Settings }

pub struct SaveGameInfo {
    pub filename: String,
    pub timestamp: String,
    pub character_names: Vec<String>,
    pub location: String,
    pub game_version: String,
}
```

### Menu Components (Game Layer)
```rust
// Marker components
pub struct MenuRoot;
pub struct MainMenuPanel;
pub struct SaveLoadPanel;
pub struct SettingsPanel;

// Button types
pub enum MenuButton {
    Resume, SaveGame, LoadGame, Settings, Quit,
    Back, Confirm, Cancel, SelectSave(usize),
}

// Volume controls
pub enum VolumeSlider {
    Master, Music, Sfx, Ambient,
}
```

### Constants
```rust
pub const MENU_WIDTH: f32 = 500.0;
pub const MENU_HEIGHT: f32 = 600.0;
pub const BUTTON_HEIGHT: f32 = 50.0;
pub const BUTTON_SPACING: f32 = 15.0;
pub const BUTTON_FONT_SIZE: f32 = 24.0;
pub const TITLE_FONT_SIZE: f32 = 36.0;
```

---

## Validation Commands

### After Every Phase
```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**ALL MUST EXIT WITH CODE 0**

### Phase-Specific Tests
```bash
# Phase 1
cargo nextest run menu_state --all-features              # 8/8 tests

# Phase 2
cargo nextest run menu_components --all-features         # 4/4 tests

# Phase 3
cargo nextest run menu_toggle --all-features             # 7/7 tests

# Phase 4
cargo nextest run menu_ui --all-features                 # 6/6 tests

# Phase 5
cargo nextest run menu_save_load --all-features          # 5/5 tests

# Phase 6
cargo nextest run menu_settings --all-features           # 4/4 tests

# Final
cargo nextest run --all-features                         # 34/34 new tests
```

---

## Critical Decisions

### 1. GameMode Variant
**Decision**: Use `Menu(MenuState)` not `Menu`
**Rationale**: Consistency with DialogueState pattern, enables Resume functionality

### 2. GameConfig Storage
**Decision**: Embed in GameState with `#[serde(default)]`
**Rationale**: Per-save settings, backward compatible

### 3. File Structure
**Decision**: `src/game/systems/menu.rs` (flat, not ui/menu.rs)
**Rationale**: Matches existing module structure

### 4. Breaking Changes
**Decision**: Accept with mitigation via `#[serde(default)]`
**Rationale**: Old saves load with default config

---

## Testing Requirements

### Unit Tests (12 tests)
- MenuState creation, transitions, serialization (8 tests)
- Component variants and constants (4 tests)

### Integration Tests (22 tests)
- Menu toggle and navigation (7 tests)
- UI rendering and interaction (6 tests)
- Save/load operations (5 tests)
- Settings menu (4 tests)

**Coverage Target**: >85% for new modules

---

## Common Pitfalls (AVOID)

### ❌ Wrong File Paths
- Don't create: `src/game/systems/ui/menu.rs`
- Correct path: `src/game/systems/menu.rs`

### ❌ Missing Line Numbers
- Don't write: "Modify GameState"
- Correct: "File: src/application/mod.rs, Lines: 311-337"

### ❌ Incomplete Pattern Matches
- Don't leave: `GameMode::Menu =>`
- Update to: `GameMode::Menu(_) =>` or `GameMode::Menu(menu_state) =>`

### ❌ Forgetting SPDX Headers
Every new .rs file MUST start with:
```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0
```

### ❌ Skipping Validation
Run ALL four cargo commands after EVERY phase:
- fmt, check, clippy, nextest

---

## Integration Points

### Input System
- `handle_input()` in `src/game/systems/input.rs`
- Check for `GameAction::Menu`
- Toggle between current mode and Menu(MenuState)

### Save System
- `SaveGameManager::save()` - Create save files
- `SaveGameManager::load()` - Restore game state
- `SaveGameManager::list_saves()` - Populate save list

### UI Systems
- Follow dialogue system pattern (screen-space Bevy UI)
- Use NodeBundle for panels, ButtonBundle for buttons
- Marker components for cleanup

### Audio System
- Apply volume changes from settings
- Update AudioConfig in GameState.config

---

## Success Criteria

### Phase Completion
- [ ] All deliverables checked off
- [ ] All validation commands pass
- [ ] All tests passing for that phase
- [ ] No clippy warnings
- [ ] Code documented with /// comments

### Final Completion
- [ ] All 34 tests passing
- [ ] All 7 phases complete
- [ ] Documentation updated (4 files)
- [ ] Manual verification passed
- [ ] No regressions in existing functionality

---

## Manual Verification Checklist

### Basic Functionality
- [ ] Press ESC → Menu opens
- [ ] Press ESC in menu → Returns to game (Resume)
- [ ] Arrow keys navigate selections
- [ ] Enter confirms selection
- [ ] All 5 main menu buttons present

### Save/Load
- [ ] Save creates file in saves/ directory
- [ ] Save file contains valid RON
- [ ] Load restores exact game state
- [ ] Save list displays metadata correctly

### Settings
- [ ] Settings menu displays all sections
- [ ] Volume sliders visible
- [ ] Back button returns to main menu

### Edge Cases
- [ ] Menu works during combat
- [ ] Menu works during dialogue
- [ ] Menu works at inn
- [ ] Old save files load with default config

---

## Debugging Tips

### Compilation Errors
```bash
# Check specific module
cargo check --package antares --lib

# Verbose output
cargo check --verbose
```

### Pattern Match Issues
```bash
# Find all GameMode matches
grep -rn "GameMode::" src/
```

### Test Failures
```bash
# Run with output
cargo nextest run -- --nocapture

# Run specific test
cargo nextest run test_name -- --nocapture

# Show backtrace
RUST_BACKTRACE=1 cargo nextest run
```

### UI Not Showing
- Check MenuPlugin registered in app
- Verify OnEnter system firing
- Check MenuRoot entity exists
- Verify asset paths correct

---

## Resources

### Primary Documents
- **Full Plan**: `docs/explanation/game_menu_implementation_plan.md`
- **Review Summary**: `docs/explanation/game_menu_plan_review_summary.md`
- **Architecture**: `docs/reference/architecture.md`
- **Agent Rules**: `AGENTS.md`

### Reference Code
- **DialogueState**: `src/application/dialogue.rs`
- **Dialogue UI**: `src/game/systems/dialogue.rs`
- **Inn UI**: `src/game/systems/inn_ui.rs`
- **GameConfig**: `src/sdk/game_config.rs`
- **SaveGameManager**: `src/application/save_game.rs`

### Testing Examples
- **Dialogue Tests**: `tests/dialogue_visuals_test.rs`
- **Character Tests**: `tests/unit/character_test.rs`

---

## Timeline

### Recommended Schedule

**Week 1** (Days 1-3):
- Phase 1: Core State Infrastructure
- Phase 2: Components & Plugin
- Phase 3: Input Integration

**Week 2** (Days 4-5):
- Phase 4: UI Rendering
- Phase 5: Save/Load Integration

**Week 3** (Days 6-7):
- Phase 6: Settings Integration
- Phase 7: Documentation & Testing

**Buffer**: Reserve 1-2 days for bug fixes and polish

---

## Getting Help

### If Stuck
1. Re-read relevant phase in full plan
2. Check "Common Pitfalls" section above
3. Review reference code (dialogue/inn systems)
4. Run diagnostics commands
5. Check existing tests for patterns

### Before Asking for Help
- [ ] Read complete phase instructions
- [ ] Ran all validation commands
- [ ] Checked diagnostics output
- [ ] Searched for similar code patterns
- [ ] Reviewed relevant architecture sections

---

## Final Notes

- **Phases are sequential** - Do not skip ahead
- **Validation is mandatory** - Run after every subsection
- **Tests are required** - Not optional
- **Documentation is deliverable** - Not an afterthought
- **Follow patterns** - Dialogue/Inn systems are your guide

**Good luck!** The plan is comprehensive - trust the process and validate frequently.
