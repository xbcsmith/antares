# Game Menu Implementation Plan

## Overview

Implement a configurable in-game menu system accessible via ESC key that provides Resume, Save Game, Load Game, Settings, and Quit functionality. The menu will integrate with the existing GameState architecture, preserve configuration in save files, and follow established UI patterns from the dialogue and inn systems.

## Current State Analysis

### Existing Infrastructure

**Game State Architecture** (`src/application/mod.rs#L311-337`):

- ✅ `GameState` struct exists with world, roster, party, active_spells, mode, time, quests
- ✅ `GameMode` enum exists at `src/application/mod.rs#L40-50` with variants: Exploration, Combat(CombatState), Menu, Dialogue(DialogueState), InnManagement(InnManagementState)
- ❌ `GameMode::Menu` is a simple variant without associated state (unlike Dialogue and InnManagement)
- ❌ No `GameConfig` field in `GameState` for per-save configuration storage

**Configuration System** (`src/sdk/game_config.rs#L118-130`):

- ✅ `GameConfig` struct exists with graphics, audio, controls, camera subsections
- ✅ `GameConfig` derives `Serialize, Deserialize` (L115-119) - safe for save files
- ✅ `GameConfig::default()` and `GameConfig::load_or_default()` available
- ✅ Full validation system via `GameConfig::validate()` (L196-202)
- ❌ Currently SDK-only, not integrated with application layer

**Input System** (`src/game/systems/input.rs#L102-121`):

- ✅ `GameAction` enum exists with `Menu` variant (L120)
- ✅ `KeyMap` system exists for key binding (L131+)
- ❌ No handler in `handle_input` function for `GameAction::Menu`

**UI Systems**:

- ✅ Dialogue UI pattern exists: `src/game/systems/dialogue.rs` (screen-space Bevy UI)
- ✅ Inn UI pattern exists: `src/game/systems/inn_ui.rs`
- ✅ HUD system exists: `src/game/systems/hud.rs`
- ❌ No menu system or MenuPlugin exists

**Save System** (`src/application/save_game.rs#L120-193`):

- ✅ `SaveGame` struct wraps `GameState` (L131)
- ✅ `SaveGameManager::save()` (L286-297) and `load()` (L324-337) methods exist
- ✅ `SaveGameManager::list_saves()` exists (L359-379) - returns Vec of save metadata
- ✅ Serde serialization/deserialization fully functional
- ❌ No UI integration for save/load operations

**Module Structure** (`src/game/systems/mod.rs#L1-18`):

- ✅ Flat module structure: audio, camera, dialogue, hud, inn_ui, input, map, quest, etc.
- ❌ No `menu` module registered

### Identified Issues

**Issue 1: GameMode::Menu lacks associated state**

- **Current**: `Menu` is a unit variant, cannot store previous mode for Resume
- **Impact**: Cannot return to previous mode when closing menu
- **Solution**: Change to `Menu(MenuState)` following DialogueState pattern

**Issue 2: No MenuState struct defined**

- **Impact**: Cannot track menu context (previous mode, current submenu, selected option)
- **Solution**: Create `src/application/menu.rs` with MenuState definition

**Issue 3: GameConfig not in GameState**

- **Current**: GameConfig is SDK-only, loaded per campaign
- **Impact**: Settings cannot be saved per save file
- **Solution**: Add `pub config: GameConfig` field to GameState with `#[serde(default)]`

**Issue 4: No input handler for menu toggle**

- **Current**: GameAction::Menu exists but no handler in `handle_input`
- **Impact**: ESC key has no effect
- **Solution**: Add menu toggle logic to `handle_input` system

**Issue 5: No menu UI implementation**

- **Impact**: Menu cannot be displayed or interacted with
- **Solution**: Create MenuPlugin with spawn/cleanup/interaction systems

**Issue 6: Breaking change for save files**

- **Current**: Adding `config` field breaks old saves
- **Impact**: Existing save files won't load
- **Solution**: Use `#[serde(default)]` attribute for backward compatibility

## Architectural Decisions

### Decision 1: MenuState Structure

**Selected Approach**: Menu as stateful variant `Menu(MenuState)` matching DialogueState pattern

**Rationale**:

- Consistency with existing DialogueState and InnManagementState patterns
- Enables storing previous mode for Resume functionality
- Allows tracking submenu state and selections
- Facilitates clean state transitions

**Definition** (to be implemented in `src/application/menu.rs`):

```rust
pub struct MenuState {
    /// The mode that was active before entering the menu
    pub previous_mode: Box<GameMode>,

    /// Current submenu being displayed
    pub current_submenu: MenuType,

    /// Selected option index in current submenu
    pub selected_index: usize,

    /// List of available save files (populated when entering SaveLoad submenu)
    pub save_list: Vec<SaveGameInfo>,
}

pub enum MenuType {
    Main,
    SaveLoad,
    Settings,
}

pub struct SaveGameInfo {
    pub filename: String,
    pub timestamp: String,
    pub character_names: Vec<String>,
    pub location: String,
    pub game_version: String,
}
```

### Decision 2: GameConfig Integration

**Selected Approach**: Embed GameConfig in GameState with `#[serde(default)]`

**Rationale**:

- Settings persist per save file (each character's adventure can have different settings)
- Backward compatible with existing saves (old saves get default config)
- Simplifies save/load logic (config automatically serialized)
- Allows runtime config changes without modifying campaign files

**Migration Strategy**:

- Add `#[serde(default)]` attribute to prevent deserialization errors
- Old saves load successfully with `GameConfig::default()`
- No manual migration code needed

### Decision 3: File Structure

**Menu Module Location**: `src/game/systems/menu.rs` (NOT `src/game/systems/ui/menu.rs`)

**Rationale**:

- Existing systems use flat structure (dialogue.rs, inn_ui.rs, hud.rs)
- No ui/ subdirectory exists
- Maintains consistency with project conventions

**Application Layer**: `src/application/menu.rs` for MenuState definition

**Rationale**:

- Matches existing pattern (dialogue.rs defines DialogueState in application layer)
- Separates state management from Bevy UI rendering

## Data Structure Definitions

### MenuState (Application Layer)

**File**: `src/application/menu.rs` (NEW)
**Lines**: 1-150

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::application::GameMode;
use serde::{Deserialize, Serialize};

/// State for the in-game menu system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuState {
    /// The game mode that was active before opening the menu
    /// Stored as Box to enable recursive GameMode definition
    pub previous_mode: Box<GameMode>,

    /// Current submenu being displayed
    pub current_submenu: MenuType,

    /// Selected option index in current submenu (0-based)
    pub selected_index: usize,

    /// Cached list of save files (populated when SaveLoad submenu opens)
    #[serde(default)]
    pub save_list: Vec<SaveGameInfo>,
}

/// Menu screen types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuType {
    /// Main menu with Resume/Save/Load/Settings/Quit
    Main,

    /// Save/Load game screen
    SaveLoad,

    /// Settings configuration screen
    Settings,
}

/// Information about a save file for display in save/load UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveGameInfo {
    /// Filename without path
    pub filename: String,

    /// Human-readable timestamp
    pub timestamp: String,

    /// Names of characters in the party
    pub character_names: Vec<String>,

    /// Current location description
    pub location: String,

    /// Save file version
    pub game_version: String,
}

impl MenuState {
    /// Create a new MenuState, storing the previous game mode
    pub fn new(previous_mode: GameMode) -> Self {
        Self {
            previous_mode: Box::new(previous_mode),
            current_submenu: MenuType::Main,
            selected_index: 0,
            save_list: Vec::new(),
        }
    }

    /// Get the mode to return to when closing the menu
    pub fn get_resume_mode(&self) -> GameMode {
        (*self.previous_mode).clone()
    }

    /// Switch to a different submenu
    pub fn set_submenu(&mut self, submenu: MenuType) {
        self.current_submenu = submenu;
        self.selected_index = 0; // Reset selection
    }

    /// Move selection up (with wrapping)
    pub fn select_previous(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = item_count - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    /// Move selection down (with wrapping)
    pub fn select_next(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        self.selected_index = (self.selected_index + 1) % item_count;
    }
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            previous_mode: Box::new(GameMode::Exploration),
            current_submenu: MenuType::Main,
            selected_index: 0,
            save_list: Vec::new(),
        }
    }
}
```

### Menu Components (Game Layer)

**File**: `src/game/components/menu.rs` (NEW)
**Lines**: 1-100

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;

/// Marker component for the menu root UI entity
#[derive(Component)]
pub struct MenuRoot;

/// Marker component for the main menu panel
#[derive(Component)]
pub struct MainMenuPanel;

/// Marker component for the save/load panel
#[derive(Component)]
pub struct SaveLoadPanel;

/// Marker component for the settings panel
#[derive(Component)]
pub struct SettingsPanel;

/// Menu button types
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuButton {
    Resume,
    SaveGame,
    LoadGame,
    Settings,
    Quit,
    Back,
    Confirm,
    Cancel,
    SelectSave(usize), // Index into save list
}

/// Volume slider component
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeSlider {
    Master,
    Music,
    Sfx,
    Ambient,
}

/// Menu UI constants
pub const MENU_BACKGROUND_COLOR: Color = Color::srgba(0.1, 0.1, 0.15, 0.95);
pub const MENU_WIDTH: f32 = 500.0;
pub const MENU_HEIGHT: f32 = 600.0;
pub const BUTTON_WIDTH: f32 = 400.0;
pub const BUTTON_HEIGHT: f32 = 50.0;
pub const BUTTON_SPACING: f32 = 15.0;
pub const BUTTON_FONT_SIZE: f32 = 24.0;
pub const TITLE_FONT_SIZE: f32 = 36.0;
pub const BUTTON_NORMAL_COLOR: Color = Color::srgb(0.25, 0.25, 0.35);
pub const BUTTON_HOVER_COLOR: Color = Color::srgb(0.35, 0.35, 0.55);
pub const BUTTON_PRESSED_COLOR: Color = Color::srgb(0.15, 0.15, 0.25);
pub const BUTTON_TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
```

### Constants and Configuration

**Menu System Constants**:

- `MENU_WIDTH`: 500.0 (pixels)
- `MENU_HEIGHT`: 600.0 (pixels)
- `BUTTON_HEIGHT`: 50.0 (pixels)
- `BUTTON_SPACING`: 15.0 (pixels)
- `MENU_FONT_SIZE`: 24.0 (points)
- `TITLE_FONT_SIZE`: 36.0 (points)

**Main Menu Options** (in order):

1. Resume
2. Save Game
3. Load Game
4. Settings
5. Quit

**Keyboard Controls**:

- ESC: Toggle menu open/close (Resume if in menu)
- Up Arrow: Select previous option
- Down Arrow: Select next option
- Enter/Space: Confirm selection
- Backspace: Go back to previous submenu

## Implementation Phases

### Phase 1: Core Menu State Infrastructure

#### 1.1 Define MenuState and Related Types

**File**: `src/application/menu.rs` (NEW)
**Lines**: 1-150
**Estimated Time**: 1 hour

**Tasks**:

1. Create new file `src/application/menu.rs`
2. Add SPDX copyright header (lines 1-2)
3. Define `MenuState` struct with all fields (lines 10-25)
4. Define `MenuType` enum with Main/SaveLoad/Settings variants (lines 27-35)
5. Define `SaveGameInfo` struct (lines 37-50)
6. Implement `MenuState::new()` method (lines 52-60)
7. Implement `MenuState::get_resume_mode()` method (lines 62-65)
8. Implement `MenuState::set_submenu()` method (lines 67-71)
9. Implement `MenuState::select_previous()` method (lines 73-82)
10. Implement `MenuState::select_next()` method (lines 84-92)
11. Implement `Default` trait for MenuState (lines 94-103)
12. Add comprehensive doc comments for all public items

**Validation**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

**Expected Output**: All commands exit with code 0

#### 1.2 Register Menu Module in Application Layer

**File**: `src/application/mod.rs`
**Target Lines**: 1-20 (module declarations)
**Estimated Time**: 15 minutes

**Tasks**:

1. Add `pub mod menu;` after line 10 (after existing module declarations)
2. Add `pub use menu::{MenuState, MenuType, SaveGameInfo};` in public exports section

**Changes**:

```rust
// Around line 10
pub mod menu;

// In pub use section (around line 20-30)
pub use menu::{MenuState, MenuType, SaveGameInfo};
```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: No errors, menu module compiles successfully

#### 1.3 Update GameMode Enum

**File**: `src/application/mod.rs`
**Target Lines**: 40-50 (GameMode enum definition)
**Estimated Time**: 30 minutes

**Tasks**:

1. Change line 44 from `Menu,` to `Menu(MenuState),`
2. Update all pattern matches in the file to handle `Menu(state)` instead of `Menu`
3. Ensure GameMode derives Clone (required for Box<GameMode>)

**Before**:

```rust
pub enum GameMode {
    Exploration,
    Combat(crate::domain::combat::engine::CombatState),
    Menu,
    Dialogue(crate::application::dialogue::DialogueState),
    InnManagement(InnManagementState),
}
```

**After**:

```rust
pub enum GameMode {
    Exploration,
    Combat(crate::domain::combat::engine::CombatState),
    Menu(MenuState),
    Dialogue(crate::application::dialogue::DialogueState),
    InnManagement(InnManagementState),
}
```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: Compiler errors about pattern matches - fix in next step

#### 1.4 Fix Pattern Matches for Menu Variant

**File**: Multiple files that match on GameMode
**Estimated Time**: 1 hour

**Tasks**:

1. Search for all `GameMode::Menu` pattern matches:
   ```bash
   grep -rn "GameMode::Menu" src/
   ```
2. Update each match from `Menu` to `Menu(_)` or `Menu(menu_state)` as appropriate
3. Primary file: `src/game/systems/input.rs` (if it has mode checks)

**Validation**:

```bash
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

**Expected Output**: Zero errors, zero warnings

#### 1.5 Add GameConfig to GameState

**File**: `src/application/mod.rs`
**Target Lines**: 311-337 (GameState struct)
**Estimated Time**: 30 minutes

**Tasks**:

1. Add import at top of file: `use crate::sdk::game_config::GameConfig;`
2. Add field to GameState struct after line 330:
   ```rust
   /// Game configuration (graphics, audio, controls, camera)
   /// Stored per-save to allow different settings per playthrough
   #[serde(default)]
   pub config: GameConfig,
   ```
3. Update `GameState::new()` method (around line 350-370) to initialize config:
   ```rust
   config: GameConfig::default(),
   ```
4. Update `GameState::new_game()` method to use campaign config if available:
   ```rust
   config: campaign.as_ref()
       .and_then(|c| GameConfig::load_or_default(&c.base_path.join("config.ron")).ok())
       .unwrap_or_default(),
   ```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: GameState compiles with new field

#### 1.6 Testing Requirements

**File**: `tests/unit/menu_state_test.rs` (NEW)
**Lines**: 1-200
**Estimated Time**: 1.5 hours

**Test Cases**:

1. `test_menu_state_new_stores_previous_mode`

   - Create MenuState with Exploration mode
   - Assert previous_mode is Exploration
   - Assert current_submenu is Main
   - Assert selected_index is 0

2. `test_menu_state_get_resume_mode_returns_previous`

   - Create MenuState with Combat mode
   - Call get_resume_mode()
   - Assert returned mode matches original

3. `test_menu_state_set_submenu_resets_selection`

   - Create MenuState
   - Set selected_index to 3
   - Call set_submenu(MenuType::Settings)
   - Assert current_submenu is Settings
   - Assert selected_index is 0

4. `test_menu_state_select_next_wraps_around`

   - Create MenuState with selected_index = 0
   - Call select_next(5) // 5 items
   - Assert selected_index is 1
   - Call select_next(5) four more times
   - Assert selected_index wraps to 0

5. `test_menu_state_select_previous_wraps_around`

   - Create MenuState with selected_index = 0
   - Call select_previous(5)
   - Assert selected_index is 4

6. `test_menu_state_serialization`

   - Create MenuState
   - Serialize to RON with `ron::to_string()`
   - Deserialize back
   - Assert all fields match

7. `test_menu_type_variants`

   - Verify MenuType::Main, SaveLoad, Settings all compile
   - Test equality comparisons

8. `test_save_game_info_creation`
   - Create SaveGameInfo with test data
   - Assert all fields accessible

**Test File Template**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use antares::application::menu::{MenuState, MenuType, SaveGameInfo};
use antares::application::GameMode;

#[test]
fn test_menu_state_new_stores_previous_mode() {
    let menu_state = MenuState::new(GameMode::Exploration);

    assert!(matches!(*menu_state.previous_mode, GameMode::Exploration));
    assert_eq!(menu_state.current_submenu, MenuType::Main);
    assert_eq!(menu_state.selected_index, 0);
}

// ... additional tests
```

**Validation**:

```bash
cargo nextest run menu_state --all-features
```

**Expected Output**: All 8 tests pass (8 passed; 0 failed)

#### 1.7 Deliverables

- [ ] `src/application/menu.rs` created with MenuState, MenuType, SaveGameInfo
- [ ] MenuState implements new(), get_resume_mode(), set_submenu(), select_previous(), select_next()
- [ ] MenuState implements Default trait
- [ ] `src/application/mod.rs` updated to export menu module
- [ ] GameMode::Menu changed to Menu(MenuState) variant
- [ ] All GameMode pattern matches updated throughout codebase
- [ ] GameConfig field added to GameState with `#[serde(default)]`
- [ ] GameState::new() and new_game() initialize config field
- [ ] Unit tests created in `tests/unit/menu_state_test.rs`
- [ ] All tests passing (8/8)

#### 1.8 Success Criteria

**Automated Checks**:

```bash
cargo fmt --all                                              # Exit code 0
cargo check --all-targets --all-features                     # Exit code 0
cargo clippy --all-targets --all-features -- -D warnings     # Exit code 0, zero warnings
cargo nextest run menu_state --all-features                  # Exit code 0, 8/8 tests pass
```

**Manual Verification**:

- MenuState can be constructed with any GameMode variant
- Serialization/deserialization works with ron crate
- GameState serializes with config field
- Old save files still load (config field gets default value)

---

### Phase 2: Menu Components and UI Structure

#### 2.1 Define Menu Components

**File**: `src/game/components/menu.rs` (NEW)
**Lines**: 1-100
**Estimated Time**: 45 minutes

**Tasks**:

1. Create file with SPDX header
2. Define marker components: MenuRoot, MainMenuPanel, SaveLoadPanel, SettingsPanel
3. Define MenuButton enum with all button types
4. Define VolumeSlider enum
5. Define UI constants (colors, sizes)
6. Add doc comments for all public items

**Complete Structure** (see Data Structure Definitions section above)

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: Components module compiles

#### 2.2 Register Menu Components Module

**File**: `src/game/components.rs`
**Target Lines**: Module declarations
**Estimated Time**: 10 minutes

**Tasks**:

1. Add `pub mod menu;` to module list
2. Add `pub use menu::*;` to exports

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: Components accessible from game module

#### 2.3 Create MenuPlugin Structure

**File**: `src/game/systems/menu.rs` (NEW)
**Lines**: 1-800
**Estimated Time**: 3 hours

**Tasks**:

1. Create file with SPDX header
2. Define MenuPlugin struct
3. Implement Plugin trait with system registration
4. Create system function stubs (will implement in later phases):
   - `menu_setup`
   - `menu_cleanup`
   - `menu_button_interaction`
   - `handle_menu_keyboard`
   - `update_button_colors`
   - `spawn_main_menu`
   - `spawn_save_load_menu`
   - `spawn_settings_menu`

**Plugin Structure**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;
use crate::application::menu::{MenuState, MenuType};
use crate::application::GameMode;
use crate::game::components::menu::*;
use crate::game::resources::GlobalState;

/// Plugin for in-game menu system
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameMode::Menu(_)), menu_setup)
            .add_systems(
                Update,
                (
                    handle_menu_keyboard,
                    menu_button_interaction,
                    update_button_colors,
                )
                    .run_if(in_state_variant(GameMode::Menu)),
            )
            .add_systems(OnExit(GameMode::Menu(_)), menu_cleanup);
    }
}

/// Spawns menu UI when entering Menu mode
fn menu_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    global_state: Res<GlobalState>,
) {
    // Implementation in Phase 3
    todo!("Implement in Phase 3")
}

/// Cleans up menu UI when exiting Menu mode
fn menu_cleanup(
    mut commands: Commands,
    menu_query: Query<Entity, With<MenuRoot>>,
) {
    // Implementation in Phase 3
    todo!("Implement in Phase 3")
}

// ... other system stubs
```

**Note**: Systems will use `todo!()` placeholders until Phase 3-5

**Validation**:

```bash
cargo check --all-targets --all-features
# Expect: Compiles with todo! warnings
```

**Expected Output**: Compiles successfully (todo! warnings acceptable)

#### 2.4 Register MenuPlugin

**File**: `src/game/systems/mod.rs`
**Target Lines**: 1-18
**Estimated Time**: 5 minutes

**Tasks**:

1. Add `pub mod menu;` to module list (maintain alphabetical order)

**Changes**:

```rust
pub mod audio;
pub mod camera;
pub mod dialogue;
pub mod dialogue_choices;
pub mod dialogue_validation;
pub mod dialogue_visuals;
pub mod events;
pub mod hud;
pub mod inn_ui;
pub mod input;
pub mod map;
pub mod menu;  // ADD THIS LINE
pub mod quest;
pub mod recruitment_dialog;
pub mod ui;
```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: Menu module accessible

#### 2.5 Add MenuPlugin to Game App

**File**: `src/game/mod.rs`
**Target Lines**: Plugin registration section (around line 50-100)
**Estimated Time**: 10 minutes

**Tasks**:

1. Find the app plugin registration section
2. Add `use systems::menu::MenuPlugin;` to imports
3. Add `MenuPlugin` to app.add_plugins() call

**Search Command**:

```bash
grep -n "add_plugins" src/game/mod.rs
```

**Expected Change**:

```rust
app.add_plugins((
    // ... existing plugins ...
    menu::MenuPlugin,
));
```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: MenuPlugin registered, app compiles

#### 2.6 Testing Requirements

**File**: `tests/unit/menu_components_test.rs` (NEW)
**Estimated Time**: 45 minutes

**Test Cases**:

1. `test_menu_button_variants` - Verify all MenuButton enum variants compile
2. `test_volume_slider_variants` - Verify all VolumeSlider variants compile
3. `test_menu_constants_defined` - Verify constants have expected values
4. `test_menu_root_component` - Verify MenuRoot component can be created

**Validation**:

```bash
cargo nextest run menu_components --all-features
```

**Expected Output**: 4/4 tests pass

#### 2.7 Deliverables

- [ ] `src/game/components/menu.rs` created with all marker components
- [ ] MenuButton and VolumeSlider enums defined
- [ ] UI constants defined
- [ ] `src/game/components.rs` exports menu module
- [ ] `src/game/systems/menu.rs` created with MenuPlugin
- [ ] MenuPlugin registered in `src/game/systems/mod.rs`
- [ ] MenuPlugin added to app in `src/game/mod.rs`
- [ ] Unit tests created for components
- [ ] All tests passing (4/4)

#### 2.8 Success Criteria

**Automated Checks**:

```bash
cargo fmt --all                                              # Exit code 0
cargo check --all-targets --all-features                     # Exit code 0
cargo clippy --all-targets --all-features -- -D warnings     # Exit code 0 (ignore todo! warnings)
cargo nextest run menu_components --all-features             # Exit code 0, 4/4 tests pass
```

**Manual Verification**:

- MenuPlugin appears in plugin list
- Menu components can be attached to entities
- Constants accessible from menu module

---

### Phase 3: Input System Integration

#### 3.1 Add Menu Toggle Handler

**File**: `src/game/systems/input.rs`
**Target Lines**: Find `handle_input` function (search for "fn handle_input")
**Estimated Time**: 1.5 hours

**Tasks**:

1. Locate `handle_input` function (likely around line 200-400)
2. Add logic to check for `GameAction::Menu` keypress
3. Implement state transition logic:
   - If in Menu mode: Resume to previous mode
   - If in other mode: Enter Menu mode with current mode stored
4. Handle edge cases (Combat mode, Dialogue mode)

**Search Command**:

```bash
grep -n "fn handle_input" src/game/systems/input.rs
```

**Implementation Pattern**:

```rust
fn handle_input(
    mut global_state: ResMut<GlobalState>,
    key_map: Res<KeyMap>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // ... existing code ...

    // Check for menu toggle
    if key_map.is_action_just_pressed(&GameAction::Menu, &keyboard) {
        match &global_state.0.mode {
            GameMode::Menu(menu_state) => {
                // Resume to previous mode
                let resume_mode = menu_state.get_resume_mode();
                info!("Closing menu, resuming to: {:?}", resume_mode);
                global_state.0.mode = resume_mode;
            }
            current_mode => {
                // Enter menu, store current mode
                info!("Opening menu from: {:?}", current_mode);
                let menu_state = MenuState::new(current_mode.clone());
                global_state.0.mode = GameMode::Menu(menu_state);
            }
        }
    }

    // ... rest of existing code ...
}
```

**Special Handling**:

- Combat mode: Allow menu (pause combat)
- Dialogue mode: Allow menu (pause dialogue)
- InnManagement mode: Allow menu (pause inn UI)

**Validation**:

```bash
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

**Expected Output**: Zero errors, zero warnings

#### 3.2 Add Keyboard Navigation in Menu

**File**: `src/game/systems/menu.rs`
**Target Function**: `handle_menu_keyboard` (stub created in Phase 2)
**Estimated Time**: 1 hour

**Tasks**:

1. Replace `todo!()` with actual implementation
2. Check for Up/Down arrow keys to change selected_index
3. Check for Enter/Space to confirm selection
4. Check for Backspace to go back to previous menu
5. Update MenuState in GlobalState resource

**Implementation**:

```rust
fn handle_menu_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut global_state: ResMut<GlobalState>,
) {
    let GameMode::Menu(menu_state) = &mut global_state.0.mode else {
        return; // Not in menu mode
    };

    // Determine item count based on current submenu
    let item_count = match menu_state.current_submenu {
        MenuType::Main => 5, // Resume, Save, Load, Settings, Quit
        MenuType::SaveLoad => menu_state.save_list.len().max(1), // At least "Back" button
        MenuType::Settings => 6, // Settings options + Back
    };

    // Handle arrow keys
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        menu_state.select_previous(item_count);
    }

    if keyboard.just_pressed(KeyCode::ArrowDown) {
        menu_state.select_next(item_count);
    }

    // Handle Enter/Space for confirmation
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        handle_menu_selection(&mut global_state);
    }

    // Handle Backspace for going back
    if keyboard.just_pressed(KeyCode::Backspace) {
        if menu_state.current_submenu != MenuType::Main {
            menu_state.set_submenu(MenuType::Main);
        }
    }
}

fn handle_menu_selection(global_state: &mut ResMut<GlobalState>) {
    // Implementation in Phase 4-5 based on selected option
    todo!("Implement in Phase 4-5")
}
```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: Compiles (handle_menu_selection has todo!)

#### 3.3 Testing Requirements

**File**: `tests/integration/menu_toggle_test.rs` (NEW)
**Estimated Time**: 2 hours

**Test Cases**:

1. `test_esc_toggles_exploration_to_menu`

   - Start in Exploration mode
   - Simulate ESC key press
   - Assert mode is Menu(MenuState)
   - Assert MenuState.previous_mode is Exploration

2. `test_esc_in_menu_returns_to_exploration`

   - Start in Menu mode (with previous_mode = Exploration)
   - Simulate ESC key press
   - Assert mode is Exploration

3. `test_esc_toggles_combat_to_menu`

   - Start in Combat mode
   - Simulate ESC key press
   - Assert mode is Menu(MenuState)
   - Assert previous_mode is Combat

4. `test_menu_resume_preserves_combat_state`

   - Create Combat mode with specific state
   - Toggle to menu
   - Resume from menu
   - Assert Combat state preserved

5. `test_arrow_up_changes_selection`

   - Start in Menu mode
   - Set selected_index to 2
   - Simulate ArrowUp key
   - Assert selected_index is 1

6. `test_arrow_down_wraps_selection`

   - Start in Menu mode
   - Set selected_index to 4 (last option in main menu)
   - Simulate ArrowDown key
   - Assert selected_index wraps to 0

7. `test_backspace_returns_to_main_menu`
   - Start in Settings submenu
   - Simulate Backspace key
   - Assert current_submenu is Main

**Test Helper Functions**:

```rust
fn create_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GlobalState(GameState::new()));
    app.insert_resource(KeyMap::default());
    app.add_systems(Update, handle_input);
    app
}

fn simulate_key_press(app: &mut App, key: KeyCode) {
    let mut keyboard = app.world.resource_mut::<ButtonInput<KeyCode>>();
    keyboard.press(key);
    app.update();
    keyboard.release(key);
}
```

**Validation**:

```bash
cargo nextest run menu_toggle --all-features
```

**Expected Output**: 7/7 tests pass

#### 3.4 Deliverables

- [ ] Menu toggle handler added to `handle_input` in `src/game/systems/input.rs`
- [ ] State transitions work: Exploration ↔ Menu, Combat ↔ Menu, Dialogue ↔ Menu
- [ ] `handle_menu_keyboard` implemented for arrow key navigation
- [ ] Enter/Space key triggers selection (stub for now)
- [ ] Backspace returns to main menu from submenus
- [ ] Integration tests created in `tests/integration/menu_toggle_test.rs`
- [ ] All tests passing (7/7)

#### 3.5 Success Criteria

**Automated Checks**:

```bash
cargo fmt --all                                              # Exit code 0
cargo check --all-targets --all-features                     # Exit code 0
cargo clippy --all-targets --all-features -- -D warnings     # Exit code 0
cargo nextest run menu_toggle --all-features                 # Exit code 0, 7/7 tests pass
```

**Manual Verification**:

- ESC key toggles between game and menu
- Arrow keys change selection in menu
- Backspace navigates back in menu hierarchy

---

### Phase 4: Menu UI Rendering

#### 4.1 Implement Main Menu UI

**File**: `src/game/systems/menu.rs`
**Target Function**: `menu_setup` and `spawn_main_menu`
**Estimated Time**: 3 hours

**Tasks**:

1. Replace `todo!()` in `menu_setup` with implementation
2. Implement `spawn_main_menu` to create UI hierarchy
3. Load font from assets
4. Create root node with background
5. Create title text
6. Create buttons for each menu option
7. Style buttons with colors and hover states

**Implementation**:

```rust
fn menu_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    global_state: Res<GlobalState>,
) {
    let GameMode::Menu(menu_state) = &global_state.0.mode else {
        warn!("menu_setup called but not in Menu mode");
        return;
    };

    match menu_state.current_submenu {
        MenuType::Main => spawn_main_menu(&mut commands, &asset_server, menu_state),
        MenuType::SaveLoad => spawn_save_load_menu(&mut commands, &asset_server, menu_state),
        MenuType::Settings => spawn_settings_menu(&mut commands, &asset_server, menu_state),
    }
}

fn spawn_main_menu(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    menu_state: &MenuState,
) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    // Root container
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.5).into(),
                ..default()
            },
            MenuRoot,
        ))
        .with_children(|parent| {
            // Menu panel
            parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(MENU_WIDTH),
                            height: Val::Px(MENU_HEIGHT),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::FlexStart,
                            padding: UiRect::all(Val::Px(20.0)),
                            ..default()
                        },
                        background_color: MENU_BACKGROUND_COLOR.into(),
                        ..default()
                    },
                    MainMenuPanel,
                ))
                .with_children(|panel| {
                    // Title
                    panel.spawn(TextBundle::from_section(
                        "GAME MENU",
                        TextStyle {
                            font: font.clone(),
                            font_size: TITLE_FONT_SIZE,
                            color: Color::WHITE,
                        },
                    ));

                    // Spacing
                    panel.spawn(NodeBundle {
                        style: Style {
                            height: Val::Px(40.0),
                            ..default()
                        },
                        ..default()
                    });

                    // Buttons
                    spawn_menu_button(panel, "Resume", MenuButton::Resume, menu_state.selected_index == 0, &font);
                    spawn_menu_button(panel, "Save Game", MenuButton::SaveGame, menu_state.selected_index == 1, &font);
                    spawn_menu_button(panel, "Load Game", MenuButton::LoadGame, menu_state.selected_index == 2, &font);
                    spawn_menu_button(panel, "Settings", MenuButton::Settings, menu_state.selected_index == 3, &font);
                    spawn_menu_button(panel, "Quit", MenuButton::Quit, menu_state.selected_index == 4, &font);
                });
        });
}

fn spawn_menu_button(
    parent: &mut ChildBuilder,
    text: &str,
    button_type: MenuButton,
    is_selected: bool,
    font: &Handle<Font>,
) {
    let bg_color = if is_selected {
        BUTTON_HOVER_COLOR
    } else {
        BUTTON_NORMAL_COLOR
    };

    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(BUTTON_WIDTH),
                    height: Val::Px(BUTTON_HEIGHT),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::all(Val::Px(BUTTON_SPACING / 2.0)),
                    ..default()
                },
                background_color: bg_color.into(),
                ..default()
            },
            button_type,
        ))
        .with_children(|button| {
            button.spawn(TextBundle::from_section(
                text,
                TextStyle {
                    font: font.clone(),
                    font_size: BUTTON_FONT_SIZE,
                    color: BUTTON_TEXT_COLOR,
                },
            ));
        });
}
```

**Validation**:

```bash
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

**Expected Output**: Compiles, clippy clean

#### 4.2 Implement Menu Cleanup

**File**: `src/game/systems/menu.rs`
**Target Function**: `menu_cleanup`
**Estimated Time**: 30 minutes

**Tasks**:

1. Replace `todo!()` with implementation
2. Query all entities with MenuRoot component
3. Despawn them recursively

**Implementation**:

```rust
fn menu_cleanup(
    mut commands: Commands,
    menu_query: Query<Entity, With<MenuRoot>>,
) {
    for entity in menu_query.iter() {
        commands.entity(entity).despawn_recursive();
        info!("Despawned menu UI");
    }
}
```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: Compiles successfully

#### 4.3 Implement Button Interaction

**File**: `src/game/systems/menu.rs`
**Target Function**: `menu_button_interaction`
**Estimated Time**: 1 hour

**Tasks**:

1. Replace `todo!()` with implementation
2. Query buttons with Interaction component
3. Handle hover states
4. Handle click events
5. Update GlobalState based on button type

**Implementation**:

```rust
fn menu_button_interaction(
    interaction_query: Query<(&Interaction, &MenuButton), Changed<Interaction>>,
    mut global_state: ResMut<GlobalState>,
) {
    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            handle_button_press(button, &mut global_state);
        }
    }
}

fn handle_button_press(button: &MenuButton, global_state: &mut ResMut<GlobalState>) {
    match button {
        MenuButton::Resume => {
            let GameMode::Menu(menu_state) = &global_state.0.mode else {
                return;
            };
            let resume_mode = menu_state.get_resume_mode();
            info!("Resume button pressed, returning to: {:?}", resume_mode);
            global_state.0.mode = resume_mode;
        }
        MenuButton::SaveGame => {
            info!("Save Game button pressed");
            // Implementation in Phase 5
            if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
                menu_state.set_submenu(MenuType::SaveLoad);
            }
        }
        MenuButton::LoadGame => {
            info!("Load Game button pressed");
            // Implementation in Phase 5
            if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
                menu_state.set_submenu(MenuType::SaveLoad);
            }
        }
        MenuButton::Settings => {
            info!("Settings button pressed");
            // Implementation in Phase 5
            if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
                menu_state.set_submenu(MenuType::Settings);
            }
        }
        MenuButton::Quit => {
            info!("Quit button pressed");
            std::process::exit(0);
        }
        _ => {
            // Other button types handled in Phase 5
        }
    }
}
```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: Compiles successfully

#### 4.4 Implement Button Color Updates

**File**: `src/game/systems/menu.rs`
**Target Function**: `update_button_colors`
**Estimated Time**: 45 minutes

**Tasks**:

1. Replace `todo!()` with implementation
2. Query buttons and get current selected_index from MenuState
3. Update button colors based on selection

**Implementation**:

```rust
fn update_button_colors(
    mut button_query: Query<(&MenuButton, &mut BackgroundColor)>,
    global_state: Res<GlobalState>,
) {
    let GameMode::Menu(menu_state) = &global_state.0.mode else {
        return;
    };

    // Map button types to indices
    for (button, mut bg_color) in button_query.iter_mut() {
        let button_index = match button {
            MenuButton::Resume => 0,
            MenuButton::SaveGame => 1,
            MenuButton::LoadGame => 2,
            MenuButton::Settings => 3,
            MenuButton::Quit => 4,
            _ => continue, // Other buttons handled separately
        };

        if button_index == menu_state.selected_index {
            *bg_color = BUTTON_HOVER_COLOR.into();
        } else {
            *bg_color = BUTTON_NORMAL_COLOR.into();
        }
    }
}
```

**Validation**:

```bash
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

**Expected Output**: Zero warnings

#### 4.5 Testing Requirements

**File**: `tests/integration/menu_ui_test.rs` (NEW)
**Estimated Time**: 2 hours

**Test Cases**:

1. `test_menu_ui_spawns_on_enter`

   - Transition to Menu mode
   - Run update cycle
   - Query for MenuRoot entity
   - Assert entity exists

2. `test_menu_ui_despawns_on_exit`

   - Spawn menu UI
   - Transition to Exploration mode
   - Run update cycle
   - Query for MenuRoot entity
   - Assert entity does not exist

3. `test_main_menu_has_five_buttons`

   - Spawn main menu
   - Query for MenuButton components
   - Assert count is 5

4. `test_resume_button_closes_menu`

   - Enter menu from Exploration
   - Click Resume button
   - Assert mode is Exploration

5. `test_quit_button_functionality`

   - Mock exit function
   - Click Quit button
   - Verify exit called

6. `test_selected_button_has_hover_color`
   - Set selected_index to 2
   - Spawn menu
   - Query button at index 2
   - Assert background color is BUTTON_HOVER_COLOR

**Validation**:

```bash
cargo nextest run menu_ui --all-features
```

**Expected Output**: 6/6 tests pass

#### 4.6 Deliverables

- [ ] `menu_setup` implemented to spawn UI based on submenu type
- [ ] `spawn_main_menu` creates full UI hierarchy with title and 5 buttons
- [ ] `menu_cleanup` despawns menu UI recursively
- [ ] `menu_button_interaction` handles button clicks
- [ ] `update_button_colors` highlights selected button
- [ ] Resume button closes menu and returns to previous mode
- [ ] Quit button exits application
- [ ] Other buttons transition to respective submenus (stubs)
- [ ] Integration tests created for UI rendering
- [ ] All tests passing (6/6)

#### 4.7 Success Criteria

**Automated Checks**:

```bash
cargo fmt --all                                              # Exit code 0
cargo check --all-targets --all-features                     # Exit code 0
cargo clippy --all-targets --all-features -- -D warnings     # Exit code 0
cargo nextest run menu_ui --all-features                     # Exit code 0, 6/6 tests pass
```

**Manual Verification**:

- Run `cargo run`
- Press ESC
- Menu appears with 5 buttons
- Selected button is highlighted
- Arrow keys change selection
- Enter key activates selected button
- Resume button closes menu

---

### Phase 5: Save/Load Menu Integration

#### 5.1 Implement Save/Load UI

**File**: `src/game/systems/menu.rs`
**Target Function**: `spawn_save_load_menu`
**Estimated Time**: 3 hours

**Tasks**:

1. Replace `todo!()` with implementation
2. Query SaveGameManager to get list of saves
3. Create scrollable list of save slots
4. Display save metadata (timestamp, party, location)
5. Add Save/Load mode toggle
6. Add Confirm and Cancel buttons

**Implementation**:

```rust
fn spawn_save_load_menu(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    menu_state: &MenuState,
) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.5).into(),
                ..default()
            },
            MenuRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(MENU_WIDTH),
                            height: Val::Px(MENU_HEIGHT),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(20.0)),
                            ..default()
                        },
                        background_color: MENU_BACKGROUND_COLOR.into(),
                        ..default()
                    },
                    SaveLoadPanel,
                ))
                .with_children(|panel| {
                    // Title
                    panel.spawn(TextBundle::from_section(
                        "SAVE / LOAD GAME",
                        TextStyle {
                            font: font.clone(),
                            font_size: TITLE_FONT_SIZE,
                            color: Color::WHITE,
                        },
                    ));

                    // Save list (scrollable)
                    panel
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                height: Val::Px(400.0),
                                flex_direction: FlexDirection::Column,
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|list| {
                            if menu_state.save_list.is_empty() {
                                list.spawn(TextBundle::from_section(
                                    "No save files found",
                                    TextStyle {
                                        font: font.clone(),
                                        font_size: 18.0,
                                        color: Color::srgb(0.7, 0.7, 0.7),
                                    },
                                ));
                            } else {
                                for (index, save_info) in menu_state.save_list.iter().enumerate() {
                                    spawn_save_slot(list, save_info, index, menu_state.selected_index == index, &font);
                                }
                            }
                        });

                    // Action buttons
                    panel
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceAround,
                                margin: UiRect::top(Val::Px(20.0)),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|buttons| {
                            spawn_menu_button(buttons, "Save", MenuButton::Confirm, false, &font);
                            spawn_menu_button(buttons, "Load", MenuButton::Confirm, false, &font);
                            spawn_menu_button(buttons, "Back", MenuButton::Back, false, &font);
                        });
                });
        });
}

fn spawn_save_slot(
    parent: &mut ChildBuilder,
    save_info: &SaveGameInfo,
    index: usize,
    is_selected: bool,
    font: &Handle<Font>,
) {
    let bg_color = if is_selected {
        BUTTON_HOVER_COLOR
    } else {
        BUTTON_NORMAL_COLOR
    };

    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Percent(95.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    margin: UiRect::all(Val::Px(5.0)),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                background_color: bg_color.into(),
                ..default()
            },
            MenuButton::SelectSave(index),
        ))
        .with_children(|slot| {
            // Filename
            slot.spawn(TextBundle::from_section(
                &save_info.filename,
                TextStyle {
                    font: font.clone(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ));

            // Timestamp
            slot.spawn(TextBundle::from_section(
                format!("Saved: {}", save_info.timestamp),
                TextStyle {
                    font: font.clone(),
                    font_size: 14.0,
                    color: Color::srgb(0.8, 0.8, 0.8),
                },
            ));

            // Party members
            if !save_info.character_names.is_empty() {
                slot.spawn(TextBundle::from_section(
                    format!("Party: {}", save_info.character_names.join(", ")),
                    TextStyle {
                        font: font.clone(),
                        font_size: 14.0,
                        color: Color::srgb(0.8, 0.8, 0.8),
                    },
                ));
            }

            // Location
            slot.spawn(TextBundle::from_section(
                format!("Location: {}", save_info.location),
                TextStyle {
                    font: font.clone(),
                    font_size: 14.0,
                    color: Color::srgb(0.8, 0.8, 0.8),
                },
            ));
        });
}
```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: Compiles successfully

#### 5.2 Populate Save List

**File**: `src/game/systems/menu.rs`
**New Function**: `populate_save_list`
**Estimated Time**: 1.5 hours

**Tasks**:

1. Create system to run when entering SaveLoad submenu
2. Query SaveGameManager for save files
3. Parse save metadata
4. Populate MenuState.save_list

**Implementation**:

```rust
fn populate_save_list(
    mut global_state: ResMut<GlobalState>,
    save_manager: Res<SaveGameManager>,
) {
    let GameMode::Menu(menu_state) = &mut global_state.0.mode else {
        return;
    };

    if menu_state.current_submenu != MenuType::SaveLoad || !menu_state.save_list.is_empty() {
        return; // Already populated or not in save/load menu
    }

    match save_manager.list_saves() {
        Ok(save_files) => {
            menu_state.save_list = save_files
                .into_iter()
                .map(|save_file| {
                    // Extract metadata from save file
                    let character_names = save_file.game_state.party.members
                        .iter()
                        .map(|c| c.name.clone())
                        .collect();

                    let location = format!(
                        "Map {}, ({}, {})",
                        save_file.game_state.world.current_map,
                        save_file.game_state.world.party_position.x,
                        save_file.game_state.world.party_position.y
                    );

                    SaveGameInfo {
                        filename: save_file.filename,
                        timestamp: save_file.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                        character_names,
                        location,
                        game_version: save_file.version.clone(),
                    }
                })
                .collect();

            info!("Loaded {} save files", menu_state.save_list.len());
        }
        Err(e) => {
            error!("Failed to list saves: {}", e);
            menu_state.save_list = Vec::new();
        }
    }
}
```

**Register System**:
Add to MenuPlugin::build():

```rust
.add_systems(Update, populate_save_list.run_if(in_state_variant(GameMode::Menu)))
```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: Compiles successfully

#### 5.3 Implement Save Operation

**File**: `src/game/systems/menu.rs`
**New Function**: `save_game_operation`
**Estimated Time**: 1.5 hours

**Tasks**:

1. Create system to handle save confirmation
2. Use SaveGameManager::save()
3. Generate unique filename (e.g., save_YYYYMMDD_HHMMSS.ron)
4. Show success/error feedback
5. Return to main menu on success

**Implementation**:

```rust
fn save_game_operation(
    global_state: &mut ResMut<GlobalState>,
    save_manager: &Res<SaveGameManager>,
) {
    // Generate filename with timestamp
    let timestamp = chrono::Local::now();
    let filename = format!("save_{}.ron", timestamp.format("%Y%m%d_%H%M%S"));

    // Create SaveGame from current state
    let save_game = crate::application::save_game::SaveGame::new(global_state.0.clone());

    // Attempt to save
    match save_manager.save(&filename, &save_game) {
        Ok(_) => {
            info!("Game saved successfully: {}", filename);
            // Return to main menu
            if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
                menu_state.set_submenu(MenuType::Main);
            }
        }
        Err(e) => {
            error!("Failed to save game: {}", e);
            // TODO: Show error message in UI
        }
    }
}
```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: Compiles successfully

#### 5.4 Implement Load Operation

**File**: `src/game/systems/menu.rs`
**New Function**: `load_game_operation`
**Estimated Time**: 1.5 hours

**Tasks**:

1. Create system to handle load confirmation
2. Use SaveGameManager::load()
3. Replace GlobalState with loaded state
4. Validate loaded state
5. Return to Exploration mode on success

**Implementation**:

```rust
fn load_game_operation(
    global_state: &mut ResMut<GlobalState>,
    save_manager: &Res<SaveGameManager>,
    selected_filename: &str,
) {
    match save_manager.load(selected_filename) {
        Ok(save_game) => {
            // Validate version compatibility
            if let Err(e) = save_game.validate_version() {
                error!("Save file version mismatch: {}", e);
                // TODO: Show error message in UI
                return;
            }

            info!("Game loaded successfully: {}", selected_filename);

            // Replace game state
            global_state.0 = save_game.game_state;

            // Return to exploration mode
            global_state.0.mode = GameMode::Exploration;
        }
        Err(e) => {
            error!("Failed to load game: {}", e);
            // TODO: Show error message in UI
        }
    }
}
```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: Compiles successfully

#### 5.5 Testing Requirements

**File**: `tests/integration/menu_save_load_test.rs` (NEW)
**Estimated Time**: 2.5 hours

**Test Cases**:

1. `test_save_list_populates_on_submenu_enter`

   - Enter SaveLoad submenu
   - Assert save_list is populated
   - Verify metadata extracted correctly

2. `test_save_operation_creates_file`

   - Call save_game_operation
   - Check filesystem for new save file
   - Verify file contains valid RON

3. `test_load_operation_restores_state`

   - Create test GameState
   - Save it
   - Modify current state
   - Load the save
   - Assert state matches original

4. `test_save_slot_ui_displays_metadata`

   - Spawn save slot with test data
   - Query text components
   - Verify timestamp, party names, location displayed

5. `test_empty_save_list_shows_message`
   - Set save_list to empty
   - Spawn SaveLoad menu
   - Assert "No save files found" text exists

**Validation**:

```bash
cargo nextest run menu_save_load --all-features
```

**Expected Output**: 5/5 tests pass

#### 5.6 Deliverables

- [ ] `spawn_save_load_menu` implemented with scrollable save list
- [ ] `populate_save_list` system extracts save metadata
- [ ] `save_game_operation` creates new save files
- [ ] `load_game_operation` loads and validates save files
- [ ] Save slots display filename, timestamp, party, location
- [ ] Empty save list shows "No save files found" message
- [ ] Back button returns to main menu
- [ ] Integration tests for save/load operations
- [ ] All tests passing (5/5)

#### 5.7 Success Criteria

**Automated Checks**:

```bash
cargo fmt --all                                              # Exit code 0
cargo check --all-targets --all-features                     # Exit code 0
cargo clippy --all-targets --all-features -- -D warnings     # Exit code 0
cargo nextest run menu_save_load --all-features              # Exit code 0, 5/5 tests pass
```

**Manual Verification**:

- Open menu → Save Game
- See list of existing saves
- Create new save (verify file created in filesystem)
- Open menu → Load Game
- Select a save and load it
- Verify game state restored correctly

---

### Phase 6: Settings Menu Integration

#### 6.1 Implement Settings UI

**File**: `src/game/systems/menu.rs`
**Target Function**: `spawn_settings_menu`
**Estimated Time**: 3 hours

**Tasks**:

1. Replace `todo!()` with implementation
2. Create sections for Graphics, Audio, Controls, Camera
3. Add sliders for volume controls
4. Add dropdowns for graphics settings
5. Add Apply/Reset/Back buttons

**Implementation**:

```rust
fn spawn_settings_menu(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    menu_state: &MenuState,
) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.5).into(),
                ..default()
            },
            MenuRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(MENU_WIDTH),
                            height: Val::Px(MENU_HEIGHT),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(20.0)),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        background_color: MENU_BACKGROUND_COLOR.into(),
                        ..default()
                    },
                    SettingsPanel,
                ))
                .with_children(|panel| {
                    // Title
                    panel.spawn(TextBundle::from_section(
                        "SETTINGS",
                        TextStyle {
                            font: font.clone(),
                            font_size: TITLE_FONT_SIZE,
                            color: Color::WHITE,
                        },
                    ));

                    // Audio Settings Section
                    spawn_settings_section(panel, "Audio", &font);
                    spawn_volume_slider(panel, "Master Volume", VolumeSlider::Master, &font);
                    spawn_volume_slider(panel, "Music Volume", VolumeSlider::Music, &font);
                    spawn_volume_slider(panel, "SFX Volume", VolumeSlider::Sfx, &font);
                    spawn_volume_slider(panel, "Ambient Volume", VolumeSlider::Ambient, &font);

                    // Graphics Settings Section
                    spawn_settings_section(panel, "Graphics", &font);
                    spawn_setting_label(panel, "Resolution: 1920x1080", &font);
                    spawn_setting_label(panel, "Fullscreen: Enabled", &font);
                    spawn_setting_label(panel, "VSync: Enabled", &font);
                    spawn_setting_label(panel, "MSAA: 4x", &font);

                    // Controls Section (read-only for now)
                    spawn_settings_section(panel, "Controls", &font);
                    spawn_setting_label(panel, "Move Forward: W", &font);
                    spawn_setting_label(panel, "Turn Left: A", &font);
                    spawn_setting_label(panel, "Turn Right: D", &font);
                    spawn_setting_label(panel, "Interact: E", &font);
                    spawn_setting_label(panel, "Menu: ESC", &font);

                    // Action buttons
                    panel
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceAround,
                                margin: UiRect::top(Val::Px(20.0)),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|buttons| {
                            spawn_menu_button(buttons, "Apply", MenuButton::Confirm, false, &font);
                            spawn_menu_button(buttons, "Reset", MenuButton::Cancel, false, &font);
                            spawn_menu_button(buttons, "Back", MenuButton::Back, false, &font);
                        });
                });
        });
}

fn spawn_settings_section(parent: &mut ChildBuilder, title: &str, font: &Handle<Font>) {
    parent.spawn(NodeBundle {
        style: Style {
            height: Val::Px(20.0),
            ..default()
        },
        ..default()
    });

    parent.spawn(TextBundle::from_section(
        title,
        TextStyle {
            font: font.clone(),
            font_size: 22.0,
            color: Color::srgb(0.9, 0.9, 0.5),
        },
    ));
}

fn spawn_setting_label(parent: &mut ChildBuilder, text: &str, font: &Handle<Font>) {
    parent.spawn(TextBundle::from_section(
        text,
        TextStyle {
            font: font.clone(),
            font_size: 16.0,
            color: Color::srgb(0.8, 0.8, 0.8),
        },
    ).with_style(Style {
        margin: UiRect::vertical(Val::Px(5.0)),
        ..default()
    }));
}

fn spawn_volume_slider(
    parent: &mut ChildBuilder,
    label: &str,
    slider_type: VolumeSlider,
    font: &Handle<Font>,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(90.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                margin: UiRect::vertical(Val::Px(10.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            // Label
            row.spawn(TextBundle::from_section(
                label,
                TextStyle {
                    font: font.clone(),
                    font_size: 18.0,
                    color: Color::WHITE,
                },
            ));

            // Slider bar (simplified for now - actual slider interaction in next step)
            row.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(200.0),
                        height: Val::Px(20.0),
                        ..default()
                    },
                    background_color: Color::srgb(0.3, 0.3, 0.3).into(),
                    ..default()
                },
                slider_type,
            ));

            // Value display
            row.spawn(TextBundle::from_section(
                "80%",
                TextStyle {
                    font: font.clone(),
                    font_size: 16.0,
                    color: Color::srgb(0.9, 0.9, 0.9),
                },
            ));
        });
}
```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: Compiles successfully

#### 6.2 Implement Settings Apply Logic

**File**: `src/game/systems/menu.rs`
**New Function**: `apply_settings`
**Estimated Time**: 1.5 hours

**Tasks**:

1. Read values from UI sliders
2. Update GameConfig in GlobalState
3. Trigger audio system updates
4. Trigger graphics updates (if applicable)
5. Return to main menu

**Implementation**:

```rust
fn apply_settings(
    mut global_state: ResMut<GlobalState>,
    slider_query: Query<&VolumeSlider>,
) {
    // Read slider values and update config
    // For now, this is a stub - full slider interaction requires more complex UI

    info!("Applying settings...");

    // Update audio volumes (example)
    for slider in slider_query.iter() {
        match slider {
            VolumeSlider::Master => {
                // Update global_state.0.config.audio.master_volume
            }
            VolumeSlider::Music => {
                // Update global_state.0.config.audio.music_volume
            }
            VolumeSlider::Sfx => {
                // Update global_state.0.config.audio.sfx_volume
            }
            VolumeSlider::Ambient => {
                // Update global_state.0.config.audio.ambient_volume
            }
        }
    }

    // Return to main menu
    if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
        menu_state.set_submenu(MenuType::Main);
    }
}
```

**Validation**:

```bash
cargo check --all-targets --all-features
```

**Expected Output**: Compiles successfully

#### 6.3 Testing Requirements

**File**: `tests/integration/menu_settings_test.rs` (NEW)
**Estimated Time**: 1.5 hours

**Test Cases**:

1. `test_settings_menu_spawns`

   - Enter Settings submenu
   - Assert SettingsPanel entity exists
   - Verify sections present (Audio, Graphics, Controls)

2. `test_volume_sliders_present`

   - Spawn settings menu
   - Query VolumeSlider components
   - Assert 4 sliders exist (Master, Music, SFX, Ambient)

3. `test_back_button_returns_to_main`

   - Enter settings submenu
   - Click Back button
   - Assert current_submenu is Main

4. `test_settings_display_current_values`
   - Set specific config values
   - Spawn settings menu
   - Verify UI displays current values

**Validation**:

```bash
cargo nextest run menu_settings --all-features
```

**Expected Output**: 4/4 tests pass

#### 6.4 Deliverables

- [ ] `spawn_settings_menu` implemented with all sections
- [ ] Audio settings with volume sliders
- [ ] Graphics settings display (read-only for now)
- [ ] Controls display (read-only)
- [ ] Apply button updates GameConfig
- [ ] Reset button restores defaults (stub)
- [ ] Back button returns to main menu
- [ ] Integration tests for settings menu
- [ ] All tests passing (4/4)

#### 6.5 Success Criteria

**Automated Checks**:

```bash
cargo fmt --all                                              # Exit code 0
cargo check --all-targets --all-features                     # Exit code 0
cargo clippy --all-targets --all-features -- -D warnings     # Exit code 0
cargo nextest run menu_settings --all-features               # Exit code 0, 4/4 tests pass
```

**Manual Verification**:

- Open menu → Settings
- See audio, graphics, and controls sections
- Volume sliders visible
- Back button returns to main menu

---

### Phase 7: Documentation and Final Integration

#### 7.1 Update Architecture Documentation

**File**: `docs/reference/architecture.md`
**Target Section**: Section 4.1 GameMode enum
**Estimated Time**: 30 minutes

**Tasks**:

1. Update GameMode enum documentation to show `Menu(MenuState)`
2. Add reference to menu system in Section 5 (Key Systems)

**Changes**:

Add to Section 4.1 (around line 133-138):

```markdown
pub enum GameMode {
Exploration,
Combat(CombatState),
Menu(MenuState), // Now stateful - stores previous mode for Resume
Dialogue(DialogueState),
InnManagement(InnManagementState),
}
```

Add new subsection in Section 5 (around line 1342):

```markdown
#### 5.6 Menu System

The menu system provides in-game access to save/load, settings, and game controls.

**Key Components**:

- `MenuState`: Tracks current submenu, previous game mode, and save file list
- `MenuPlugin`: Bevy plugin managing UI rendering and interaction
- `GameConfig`: Configuration stored in save files for per-playthrough settings

**Integration**:

- ESC key toggles menu (defined in `GameAction::Menu`)
- Menu preserves previous mode for Resume functionality
- Settings changes persist in save files
```

**Validation**: Manual review

#### 7.2 Create How-To Guide

**File**: `docs/how-to/using_game_menu.md` (NEW)
**Estimated Time**: 45 minutes

**Content**:

```markdown
# Using the Game Menu

This guide explains how to use the in-game menu system in Antares.

## Opening the Menu

Press **ESC** at any time during gameplay to open the menu.

## Menu Options

### Resume

Returns to the game exactly where you left off. The game state is preserved, including:

- Party position and facing direction
- Active combat (if paused during combat)
- Dialogue state (if paused during dialogue)
- All character stats and inventory

**Keyboard**: Press **ESC** again to resume

### Save Game

Saves your current progress to a file.

**Steps**:

1. Select "Save Game" from the main menu
2. A new save file is created with timestamp (e.g., `save_20250120_143022.ron`)
3. The save includes:
   - All character data (stats, inventory, equipment)
   - World state (maps, position, time)
   - Quest progress
   - Game configuration settings

**Note**: Save files are stored in `saves/` directory

### Load Game

Loads a previously saved game.

**Steps**:

1. Select "Load Game" from the main menu
2. Browse available save files
3. Use **Arrow Keys** to select a save
4. Press **Enter** to load
5. The game returns to Exploration mode at the saved location

**Save File Information**:
Each save displays:

- Filename
- Save timestamp
- Party member names
- Current location

### Settings

Configure game settings.

**Available Settings**:

**Audio**:

- Master Volume (0-100%)
- Music Volume (0-100%)
- SFX Volume (0-100%)
- Ambient Volume (0-100%)

**Graphics** (read-only for now):

- Resolution
- Fullscreen mode
- VSync
- MSAA

**Controls** (read-only):

- Key bindings display

**Steps**:

1. Select "Settings" from main menu
2. Adjust sliders with arrow keys (future feature)
3. Click "Apply" to save changes
4. Click "Back" to return without saving

### Quit

Exits the game immediately.

**Warning**: Make sure to save before quitting!

## Keyboard Controls

| Key        | Action                   |
| ---------- | ------------------------ |
| ESC        | Open/Close menu (Resume) |
| Up Arrow   | Select previous option   |
| Down Arrow | Select next option       |
| Enter      | Confirm selection        |
| Space      | Confirm selection        |
| Backspace  | Go back to previous menu |

## Tips

- Settings are saved per save file, so different playthroughs can have different configurations
- Use unique names for save files to avoid confusion
- The game automatically pauses when the menu is open
- You can open the menu during combat to pause the action
```

**Validation**: Manual review for clarity

#### 7.3 Update Implementations Documentation

**File**: `docs/explanation/implementations.md`
**Target**: Add new section at end
**Estimated Time**: 1 hour

**Content**:

````markdown
## Game Menu System - COMPLETED

### Summary

Implemented a comprehensive in-game menu system accessible via ESC key. The menu provides Resume, Save Game, Load Game, Settings, and Quit functionality. Configuration settings are stored in save files, allowing per-playthrough customization. The implementation follows established patterns from the dialogue and inn systems, using Bevy UI for rendering and MenuState for state management.

### Components Implemented

1. **MenuState** (`src/application/menu.rs`)

   - Stores previous game mode for Resume functionality
   - Tracks current submenu (Main/SaveLoad/Settings)
   - Manages selection index and save file list
   - Implements navigation methods

2. **Menu Components** (`src/game/components/menu.rs`)

   - Marker components: MenuRoot, MainMenuPanel, SaveLoadPanel, SettingsPanel
   - MenuButton enum for button types
   - VolumeSlider enum for audio controls
   - UI constants for styling

3. **MenuPlugin** (`src/game/systems/menu.rs`)

   - Bevy plugin managing menu lifecycle
   - Systems: menu_setup, menu_cleanup, button_interaction, keyboard_navigation
   - Save/load operations integration
   - Settings UI rendering

4. **GameConfig Integration** (`src/application/mod.rs`)
   - Added `config: GameConfig` field to GameState
   - Backward compatible with `#[serde(default)]`
   - Persists settings in save files

### Changes Made

#### Application Layer

**File**: `src/application/menu.rs` (NEW, 150 lines)

- Defined MenuState struct with previous_mode, current_submenu, selected_index
- Defined MenuType enum (Main, SaveLoad, Settings)
- Defined SaveGameInfo struct for save metadata
- Implemented navigation methods: select_previous(), select_next(), set_submenu()
- Implemented Default trait for MenuState

**File**: `src/application/mod.rs`

- Line 10: Added `pub mod menu;`
- Line 25: Added public exports for MenuState, MenuType, SaveGameInfo
- Line 44: Changed `Menu,` to `Menu(MenuState),` in GameMode enum
- Line 330: Added `pub config: GameConfig` field to GameState with `#[serde(default)]`
- Lines 350-370: Updated GameState::new() to initialize config
- Updated all GameMode pattern matches from `Menu` to `Menu(_)` or `Menu(menu_state)`

#### Game Layer - Components

**File**: `src/game/components/menu.rs` (NEW, 100 lines)

- Defined marker components: MenuRoot, MainMenuPanel, SaveLoadPanel, SettingsPanel
- Defined MenuButton enum with variants: Resume, SaveGame, LoadGame, Settings, Quit, Back, Confirm, Cancel, SelectSave(usize)
- Defined VolumeSlider enum: Master, Music, Sfx, Ambient
- Defined UI constants: colors, sizes, spacing

**File**: `src/game/components.rs`

- Added `pub mod menu;`
- Added `pub use menu::*;`

#### Game Layer - Systems

**File**: `src/game/systems/menu.rs` (NEW, 800+ lines)

- Implemented MenuPlugin with system registration
- Implemented menu_setup() to spawn UI based on current_submenu
- Implemented menu_cleanup() to despawn all menu entities
- Implemented handle_menu_keyboard() for arrow key navigation
- Implemented menu_button_interaction() for mouse clicks
- Implemented update_button_colors() to highlight selected option
- Implemented spawn_main_menu() with 5 buttons
- Implemented spawn_save_load_menu() with scrollable save list
- Implemented spawn_settings_menu() with audio/graphics/controls sections
- Implemented populate_save_list() to extract save metadata
- Implemented save_game_operation() to create save files
- Implemented load_game_operation() to load and validate saves
- Implemented apply_settings() to update GameConfig

**File**: `src/game/systems/mod.rs`

- Line 12: Added `pub mod menu;`

**File**: `src/game/systems/input.rs`

- Lines ~250-280: Added menu toggle handler in handle_input()
- Checks for GameAction::Menu keypress
- Toggles between current mode and Menu(MenuState)
- Preserves previous mode for Resume

#### Plugin Registration

**File**: `src/game/mod.rs`

- Added `use systems::menu::MenuPlugin;`
- Added `MenuPlugin` to app.add_plugins()

### Testing

**Unit Tests**: `tests/unit/menu_state_test.rs` (8 tests)

1. test_menu_state_new_stores_previous_mode ✓
2. test_menu_state_get_resume_mode_returns_previous ✓
3. test_menu_state_set_submenu_resets_selection ✓
4. test_menu_state_select_next_wraps_around ✓
5. test_menu_state_select_previous_wraps_around ✓
6. test_menu_state_serialization ✓
7. test_menu_type_variants ✓
8. test_save_game_info_creation ✓

**Unit Tests**: `tests/unit/menu_components_test.rs` (4 tests)

1. test_menu_button_variants ✓
2. test_volume_slider_variants ✓
3. test_menu_constants_defined ✓
4. test_menu_root_component ✓

**Integration Tests**: `tests/integration/menu_toggle_test.rs` (7 tests)

1. test_esc_toggles_exploration_to_menu ✓
2. test_esc_in_menu_returns_to_exploration ✓
3. test_esc_toggles_combat_to_menu ✓
4. test_menu_resume_preserves_combat_state ✓
5. test_arrow_up_changes_selection ✓
6. test_arrow_down_wraps_selection ✓
7. test_backspace_returns_to_main_menu ✓

**Integration Tests**: `tests/integration/menu_ui_test.rs` (6 tests)

1. test_menu_ui_spawns_on_enter ✓
2. test_menu_ui_despawns_on_exit ✓
3. test_main_menu_has_five_buttons ✓
4. test_resume_button_closes_menu ✓
5. test_quit_button_functionality ✓
6. test_selected_button_has_hover_color ✓

**Integration Tests**: `tests/integration/menu_save_load_test.rs` (5 tests)

1. test_save_list_populates_on_submenu_enter ✓
2. test_save_operation_creates_file ✓
3. test_load_operation_restores_state ✓
4. test_save_slot_ui_displays_metadata ✓
5. test_empty_save_list_shows_message ✓

**Integration Tests**: `tests/integration/menu_settings_test.rs` (4 tests)

1. test_settings_menu_spawns ✓
2. test_volume_sliders_present ✓
3. test_back_button_returns_to_main ✓
4. test_settings_display_current_values ✓

**Total Tests**: 34/34 passing
**Coverage**: >85% for new modules

### Validation

All quality checks passed:

```bash
cargo fmt --all                                              # ✓ Exit code 0
cargo check --all-targets --all-features                     # ✓ Exit code 0
cargo clippy --all-targets --all-features -- -D warnings     # ✓ Exit code 0, zero warnings
cargo nextest run --all-features                             # ✓ Exit code 0, 34 new tests pass
```
````

### Manual Verification

Tested the following workflows:

1. **Menu Toggle**

   - Started game in Exploration mode
   - Pressed ESC → Menu opened
   - Pressed ESC again → Returned to Exploration
   - ✓ State preserved correctly

2. **Save Game**

   - Opened menu → Selected Save Game
   - New save file created: `saves/save_20250120_143022.ron`
   - Verified file contains valid RON with GameState and GameConfig
   - ✓ Save successful

3. **Load Game**

   - Opened menu → Selected Load Game
   - Save list displayed with metadata (timestamp, party, location)
   - Selected a save → Game loaded
   - ✓ State restored correctly

4. **Settings Menu**

   - Opened menu → Selected Settings
   - Saw audio, graphics, and controls sections
   - Volume sliders displayed
   - Back button returned to main menu
   - ✓ UI functional

5. **Combat Pause**

   - Entered combat
   - Pressed ESC → Menu opened (combat paused)
   - Selected Resume → Returned to combat
   - ✓ Combat state preserved

6. **Configuration Persistence**
   - Changed audio settings
   - Saved game
   - Quit and restarted
   - Loaded save
   - ✓ Settings preserved in save file

### Architectural Notes

**GameMode Enum Update**:
Changed from `Menu` (unit variant) to `Menu(MenuState)` (tuple variant) for consistency with Dialogue and InnManagement patterns. This enables storing the previous mode for Resume functionality.

**Backward Compatibility**:
Used `#[serde(default)]` on GameConfig field to ensure old save files load successfully with default configuration.

**UI Pattern**:
Followed Bevy UI screen-space pattern established by dialogue system refactor. Menu is rendered as a full-screen overlay with centered panel.

**State Management**:
MenuState is part of GameMode (application layer), while menu rendering is in game layer (MenuPlugin). This maintains separation of concerns.

### Known Limitations

1. **Volume Sliders**: Currently read-only visual representation. Interactive slider dragging not yet implemented.
2. **Graphics Settings**: Display-only. Runtime graphics changes require additional integration with Bevy rendering.
3. **Key Rebinding**: Controls section is read-only. Key rebinding UI requires additional input system work.
4. **Save File Naming**: Auto-generated timestamps. Custom save names not yet supported.
5. **Error Handling**: Save/load errors logged but not displayed in UI. Toast notification system needed.

### Future Enhancements

1. Implement interactive volume sliders with drag support
2. Add runtime graphics settings changes
3. Add key rebinding UI
4. Add custom save file naming
5. Add in-menu error/success toast notifications
6. Add save file deletion functionality
7. Add settings preview before applying
8. Add gamepad support for menu navigation

### Documentation

- ✓ Updated `docs/reference/architecture.md` Section 4.1 with Menu(MenuState)
- ✓ Created `docs/how-to/using_game_menu.md` with user guide
- ✓ Updated this file (`docs/explanation/implementations.md`)

````

**Validation**: Manual review

#### 7.4 Update README

**File**: `README.md`
**Target**: Features section and controls section
**Estimated Time**: 15 minutes

**Tasks**:

1. Add "In-game menu system" to features list
2. Add keyboard controls section for menu

**Search for features section and add**:
```markdown
- In-game menu system with save/load, settings, and configuration
````

**Add new section**:

```markdown
## Keyboard Controls

| Key         | Action                 |
| ----------- | ---------------------- |
| W           | Move forward           |
| A           | Turn left              |
| D           | Turn right             |
| E           | Interact               |
| ESC         | Open/Close menu        |
| Arrow Keys  | Navigate menu options  |
| Enter/Space | Confirm menu selection |
```

**Validation**: Manual review

#### 7.5 Final Integration Testing

**Estimated Time**: 2 hours

**Manual Test Suite**:

1. **Full Game Loop Test**

   - Start new game
   - Play for 5 minutes
   - Open menu (ESC)
   - Save game
   - Make changes (move, pick up item)
   - Open menu
   - Load previous save
   - Verify state restored to save point

2. **All Menu Paths Test**

   - Open menu from Exploration
   - Navigate to each submenu (SaveLoad, Settings)
   - Use Back button to return
   - Use ESC to close from each submenu

3. **Settings Persistence Test**

   - Open Settings
   - Note current values
   - Create save
   - Quit game
   - Restart and load save
   - Open Settings
   - Verify values match

4. **Edge Cases Test**
   - Open menu during dialogue → Verify dialogue pauses
   - Open menu during combat → Verify combat pauses
   - Open menu at inn → Verify inn UI pauses
   - Load save with different party composition → Verify loads correctly

**Validation Checklist**:

- [ ] ESC toggles menu in all game modes
- [ ] Resume returns to correct mode
- [ ] Save creates valid file
- [ ] Load restores complete state
- [ ] Settings display current config
- [ ] All buttons functional
- [ ] Keyboard navigation works
- [ ] Mouse interaction works
- [ ] UI scales correctly
- [ ] No console errors or warnings

#### 7.6 Deliverables

- [ ] `docs/reference/architecture.md` updated with Menu system
- [ ] `docs/how-to/using_game_menu.md` created with user guide
- [ ] `docs/explanation/implementations.md` updated with complete summary
- [ ] `README.md` updated with features and controls
- [ ] All manual tests passing
- [ ] No regressions in existing functionality

#### 7.7 Success Criteria

**Documentation Complete**:

- All four documentation files updated
- User guide comprehensive and clear
- Implementation notes detailed

**System Fully Functional**:

- All 34 automated tests passing
- All manual tests passing
- No clippy warnings
- No console errors

**Production Ready**:

- Feature complete per original requirements
- Backward compatible with existing saves
- No known critical bugs

---

## Final Validation Checklist

### Code Quality

- [ ] `cargo fmt --all` - Exit code 0
- [ ] `cargo check --all-targets --all-features` - Exit code 0
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - Exit code 0, zero warnings
- [ ] All functions have doc comments with examples
- [ ] All public types have doc comments
- [ ] SPDX headers on all new files

### Testing

- [ ] `cargo nextest run --all-features` - All 34 new tests pass
- [ ] Unit tests: 12/12 passing
- [ ] Integration tests: 22/22 passing
- [ ] No test failures or panics
- [ ] Coverage >85% for new modules

### Functionality

- [ ] ESC key toggles menu in all game modes
- [ ] Arrow keys navigate menu selections
- [ ] Enter/Space confirms selections
- [ ] Resume returns to previous mode (Exploration/Combat/Dialogue/Inn)
- [ ] Save creates timestamped file in saves/ directory
- [ ] Load restores complete game state
- [ ] Settings menu displays all configuration sections
- [ ] Quit button exits application
- [ ] All UI elements render correctly
- [ ] No visual artifacts or glitches

### Integration

- [ ] MenuPlugin registered in app
- [ ] MenuState serializes/deserializes correctly
- [ ] GameConfig persists in save files
- [ ] Old saves load with default config
- [ ] No conflicts with existing systems (dialogue, inn, combat)
- [ ] Input system correctly handles menu toggle
- [ ] State transitions work in all scenarios

### Documentation

- [ ] Architecture document updated
- [ ] How-to guide created
- [ ] Implementation summary complete
- [ ] README updated
- [ ] All code has inline comments where needed
- [ ] No TODO comments without issue tracking

### Performance

- [ ] Menu opens instantly (<16ms)
- [ ] No frame drops when toggling menu
- [ ] Save operation completes quickly (<1s for typical save)
- [ ] Load operation completes quickly (<2s for typical save)
- [ ] UI responsive to input
- [ ] No memory leaks

### Backward Compatibility

- [ ] Old save files load successfully
- [ ] Config field uses `#[serde(default)]`
- [ ] No data loss when loading old saves
- [ ] Version mismatch handled gracefully

---

## Summary

This implementation plan provides a complete, AI-optimized roadmap for implementing the game menu system. Each phase includes:

- Exact file paths with line numbers
- Specific data structures and function signatures
- Machine-verifiable validation criteria
- Comprehensive test specifications
- Clear deliverables and success criteria

**Total Estimated Time**: 25-30 hours

**Phases**:

1. Core Menu State Infrastructure (4 hours)
2. Menu Components and UI Structure (3 hours)
3. Input System Integration (4 hours)
4. Menu UI Rendering (5 hours)
5. Save/Load Menu Integration (6 hours)
6. Settings Menu Integration (4 hours)
7. Documentation and Final Integration (3 hours)

**Implementation Order**: Phases must be completed sequentially as each depends on the previous.

**Breaking Changes**: Acknowledged and mitigated with `#[serde(default)]` for backward compatibility.
