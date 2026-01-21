# Using the Game Menu

This guide explains how to use the in-game menu system in Antares.

## Opening the Menu

Press **ESC** at any time during gameplay to open the menu. The game will pause, and the main menu will appear.

To close the menu and resume playing, press **ESC** again or select **Resume**.

## Menu Options

### Resume

Returns to the game exactly where you left off. The game state is preserved, including:

- Party position and facing direction
- Active combat (if paused during combat)
- Dialogue state (if paused during dialogue)
- All character stats and inventory

**Keyboard**: Press **ESC** again to resume, or select and confirm with **Enter**

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
4. Use arrow keys to navigate and **Enter** to select a save slot

**Note**: Save files are stored in the `saves/` directory relative to where you launched the game.

**Tip**: Save before exploring dangerous dungeons to avoid losing progress!

### Load Game

Loads a previously saved game.

**Steps**:

1. Select "Load Game" from the main menu
2. Browse available save files with arrow keys
3. Use **Up/Down Arrow** to select a save
4. Press **Enter** to load
5. The game returns to Exploration mode at the saved location

**Save File Information**:

Each save displays:

- Filename (auto-generated with timestamp)
- Save timestamp (when you saved)
- Party member names (characters in your party)
- Current location (map name and position)
- Game version (for compatibility)

**Warning**: Loading a save will overwrite your current progress if you haven't saved!

### Settings

Configure game settings for your playthrough.

**Available Settings**:

**Audio**:

- Master Volume (0-100%) - Overall volume
- Music Volume (0-100%) - Background music loudness
- SFX Volume (0-100%) - Sound effects loudness
- Ambient Volume (0-100%) - Environmental sounds

**Graphics** (read-only for now):

- Resolution (current screen size)
- Fullscreen mode (whether playing full-screen)
- VSync (vertical sync synchronization)
- MSAA (anti-aliasing quality)

**Controls** (read-only):

- Key bindings display (how to control your character)

**Steps**:

1. Select "Settings" from main menu
2. Adjust sliders with arrow keys (feature coming soon)
3. Click "Apply" (or press **Enter** on Confirm button) to save changes
4. Click "Back" (or press **Backspace**) to return without saving

**Note**: Settings are saved per save file, so different playthroughs can have different configurations.

### Quit

Exits the game immediately and returns to the desktop.

**Warning**: Make sure to save before quitting! Any progress since your last save will be lost.

## Keyboard Controls

| Key        | Action                     |
|------------|----------------------------|
| ESC        | Open/Close menu (Resume)   |
| Up Arrow   | Select previous option     |
| Down Arrow | Select next option         |
| Enter      | Confirm selection          |
| Space      | Confirm selection          |
| Backspace  | Go back to previous menu   |

## Tips and Tricks

### Save Frequently

Save before:
- Entering new areas or dungeons
- Starting dangerous encounters
- Making important decisions in dialogue
- Attempting difficult puzzles

### Multiple Save Files

Create multiple save files for different:
- Character builds or strategies
- Points in the story (before major decisions)
- Different party compositions
- Speedrun attempts

### Settings Persistence

Settings are saved with your game, so:
- Each playthrough can have custom audio settings
- Change settings between playthroughs without losing them
- Reset to defaults by loading a save with default settings

### Menu During Combat

You can pause combat by opening the menu:
1. Press ESC during a battle
2. Review your party status
3. Plan your next moves
4. Press ESC again to Resume and continue fighting

**Note**: The menu does NOT allow you to change equipment or cast spells outside of normal combat turns.

### Menu During Dialogue

Similarly, you can pause dialogue to think:
1. Press ESC while talking to an NPC
2. Review dialogue options
3. Press ESC to Resume and continue the conversation

### Saving at Inns

While you can save anywhere, inns provide narrative and gameplay benefits:
- Safe location to rest
- Manage your party and roster
- Clear status conditions
- Save your progress with a safe endpoint

## Troubleshooting

**Q: Can I save during combat?**

A: Yes! Press ESC to open the menu and save. Combat will pause, and you can save your progress.

**Q: What happens if I load a save?**

A: You return to exactly where you saved, including your position, party status, and inventory. Any progress since that save is lost.

**Q: Can I delete save files?**

A: Currently, save files accumulate in the `saves/` directory. You can manually delete them with your file manager, or they'll be overwritten if you save to an existing slot.

**Q: Are my settings saved if I quit without saving the game?**

A: Settings are part of your save file. Changes only persist if you save the game before quitting.

**Q: Can I access the menu from the main menu (before starting a game)?**

A: The in-game menu is only available during gameplay. To change settings before starting, edit your save file or use the game's configuration files.

## Next Steps

- **Explore**: Venture into the world with confidence, knowing you can save anytime
- **Experiment**: Try different character builds and strategies
- **Challenge**: Test yourself in increasingly difficult dungeons
```

Now I'll update the implementations.md file with the Phase 7 documentation:
