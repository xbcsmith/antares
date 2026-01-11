# Game Menu Implementation Plan

## Goal Description
Implement a configurable Game Menu that can be toggled with a key (default ESC). The menu should provide options to Resume, Save Game, Load Game, Configure Settings, and Quit. Configuration settings should be stored in the Save Game file and override defaults.

## User Review Required
> [!IMPORTANT]
> This change adds `config: GameConfig` to the `GameState` struct. This is a breaking change for existing save files unless migration logic is handled or we accept breaking backward compatibility for this phase. Given the stage of development, we assume breaking changes are acceptable, but verification will be needed.

## Proposed Changes

### Domain Layer
#### [MODIFY] [mod.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/mod.rs)
- Modify `GameState` struct to include `pub config: GameConfig`.
- Update `GameState::new()` to initialize with `GameConfig::default()`.
- Update `GameState::new_game()` to initialize with campaign config or defaults.

### Input System
#### [MODIFY] [input.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/input.rs)
- In `handle_input`, check for `GameAction::Menu`.
- Implement logic to toggle `GlobalState` mode between `Exploration` (or current mode) and `Menu`.

### UI Implementation
#### [NEW] [menu.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/ui/menu.rs)
- Create a new module for the Menu UI.
- Implement `MenuPlugin`.
- Create systems for:
    - `menu_setup`: Spawns the menu UI when entering `GameMode::Menu`.
    - `menu_cleanup`: Despawns the menu UI when exiting.
    - `menu_action`: Handles button clicks (Resume, Save, Load, Settings, Quit).
    - `settings_menu`: UI for changing volume, graphics, etc.
    - `save_load_menu`: UI for listing and selecting saves.

#### [MODIFY] [mod.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/mod.rs)
- Register the new `menu` module.

#### [MODIFY] [mod.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/mod.rs)
- Add `MenuPlugin` to the game app.

### Save/Load System
#### [MODIFY] [save_game.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/application/save_game.rs)
- Implicitly supported via `GameState` changes, but need to verify `GameConfig` serializes correctly within `SaveGame`.
- Ensure `GameConfig` overrides are applied when loading a save.

## Verification Plan

### Automated Tests
- **Unit Tests**:
    - Verify `GameState` serialization with `GameConfig`.
    - Verify `SaveGame` includes `GameConfig`.
    - Test `handle_input` toggles `GameMode`.

### Manual Verification
- **Run the Game**:
    - Start the game (`cargo run`).
    - Press `ESC`. Verify Menu opens.
    - Press `ESC` or `Resume`. Verify Menu closes.
    - Change a setting (e.g., volume). Save the game.
    - Restart game. Load the save. Verify setting is preserved.
    - Try `Save Game`. Check file system for new save.
    - Try `Load Game`. Select a save and load it.
    - Try `Quit`. Verify app exits.
