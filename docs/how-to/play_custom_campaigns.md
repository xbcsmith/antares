# How-To: Play Custom Campaigns

**Phase:** 14 (Game Engine Campaign Integration)
**Audience:** Players
**Difficulty:** Beginner

---

## Overview

This guide explains how to download, install, and play custom campaigns in Antares.

---

## Prerequisites

- Antares game installed
- Basic familiarity with command-line interface
- A campaign package (.zip file) or campaign directory

---

## Part 1: Finding Campaigns

### Where to Find Campaigns

Campaigns can be found from various sources:

- **Official Campaigns**: Bundled with Antares or available from the official repository
- **Community Campaigns**: Shared by other players on forums, GitHub, or itch.io
- **Self-Made Campaigns**: Created using the Campaign Builder

### Campaign File Formats

Campaigns come in two formats:

1. **Campaign Package (.zip)**: Compressed archive ready to install
2. **Campaign Directory**: Uncompressed folder with campaign files

---

## Part 2: Installing Campaigns

### Option A: Install from .zip Package

If you downloaded a .zip campaign package:

1. **Locate your campaigns directory**:
   - Linux/Mac: `~/.antares/campaigns/` or `~/antares/campaigns/`
   - Windows: `%APPDATA%\antares\campaigns\`
   - Custom: Specified with `--campaigns-dir` flag

2. **Extract the campaign package**:
   ```bash
   # Extract to campaigns directory
   unzip my_campaign.zip -d ~/.antares/campaigns/
   ```

3. **Verify extraction**:
   ```bash
   ls ~/.antares/campaigns/my_campaign/
   # Should see: campaign.ron, data/, maps/, etc.
   ```

### Option B: Use Existing Campaign Directory

If you have a campaign directory (e.g., from Campaign Builder):

1. **Copy the campaign directory**:
   ```bash
   cp -r /path/to/my_campaign ~/.antares/campaigns/
   ```

2. **Verify campaign structure**:
   ```bash
   ls ~/.antares/campaigns/my_campaign/
   # Expected files: campaign.ron, data/, maps/, assets/
   ```

### Campaign Directory Structure

A typical campaign directory looks like:

```
my_campaign/
├── campaign.ron          # Campaign metadata
├── data/
│   ├── items.ron        # Item definitions
│   ├── spells.ron       # Spell definitions
│   ├── monsters.ron     # Monster definitions
│   ├── classes.ron      # Character classes
│   ├── races.ron        # Character races
│   ├── quests.ron       # Quest data
│   └── dialogues.ron    # NPC dialogues
├── maps/
│   ├── town_01.ron      # Map files
│   └── dungeon_01.ron
├── assets/
│   ├── tilesets/        # Tileset images
│   ├── music/           # Background music
│   ├── sounds/          # Sound effects
│   └── portraits/       # Character portraits
└── README.md            # Campaign info
```

---

## Part 3: Listing Available Campaigns

Before playing, see what campaigns are installed:

```bash
antares --list-campaigns
```

**Output Example:**

```
Available Campaigns:

  tutorial - Tutorial Campaign v1.0.0
    Author: Antares Team
    A beginner-friendly campaign introducing game mechanics

  dark_tower - Dark Tower v1.2.0
    Author: John Smith
    An epic adventure through the cursed Dark Tower

  desert_quest - Desert Quest v2.0.5
    Author: Jane Doe
    Survive the scorching desert and uncover ancient secrets
```

If no campaigns are found:

```
Available Campaigns:

  No campaigns found in "campaigns"

  Create campaigns using the Campaign Builder tool:
  $ cargo run --bin campaign_builder
```

---

## Part 4: Validating Campaigns

Before playing, it's recommended to validate the campaign:

```bash
antares --validate-campaign tutorial
```

**Successful Validation:**

```
Validating campaign: tutorial

Campaign: tutorial

Validation Results:
  Errors: 0
  Warnings: 0

✓ Campaign is valid!

To play this campaign:
  $ antares --campaign tutorial
```

**Failed Validation:**

```
Validating campaign: broken_campaign

Campaign: broken_campaign

Validation Results:
  Errors: 2
  Warnings: 1

Errors:
  - Missing data file: data/items.ron
  - Invalid starting map ID: 999

Warnings:
  - Starting gold is very high: 50000

Campaign validation failed
```

**Fix Validation Errors:**

- Contact the campaign author for a fixed version
- If you're the author, use Campaign Builder to fix issues

---

## Part 5: Starting a New Game with Campaign

### Launch Game with Campaign

```bash
antares --campaign tutorial
```

**What Happens:**

1. Game loads the campaign metadata
2. Campaign starting configuration is applied:
   - Starting gold and food
   - Starting map and position
   - Campaign-specific data loaded
3. Game launches in exploration mode

**Output:**

```
Loading campaign: tutorial
Campaign loaded: Tutorial Campaign v1.0.0
Author: Antares Team
A beginner-friendly campaign introducing game mechanics

========================================
           ANTARES RPG
========================================

Campaign: Tutorial Campaign v1.0.0

Party Gold: 500
Party Food: 100
Game Mode: Exploration
Day: 1, Time: 6:00

Available commands:
  status  - Show game status
  save    - Save game
  load    - Load game
  quit    - Quit game

antares>
```

### Game Commands

Once in the game, you can use these commands:

**status** - Show current game status:
```
antares> status

=== Game Status ===
Campaign: Tutorial Campaign v1.0.0
Author: Antares Team

Game Mode: Exploration
Day: 1, Time: 6:00

Party:
  Members: 0
  Gold: 500
  Food: 100
  Gems: 0

Roster:
  Total Characters: 0
```

**save** - Save your progress:
```
antares> save
Enter save name: tutorial_save_1
Game saved: tutorial_save_1
```

**load** - Load a saved game:
```
antares> load
Available saves:
  1. tutorial_save_1
  2. tutorial_save_2
Enter save number: 1
Game loaded: tutorial_save_1
```

**quit** - Exit the game:
```
antares> quit
Thanks for playing Antares!
```

---

## Part 6: Saving and Loading

### Saving Your Game

1. **Save During Gameplay**:
   ```
   antares> save
   Enter save name: my_progress
   Game saved: my_progress
   ```

2. **Save File Location**:
   - Default: `saves/` directory
   - Custom: Specified with `--saves-dir` flag
   - Format: RON (`.ron` extension)

3. **Save File Contents**:
   - Game state (party, roster, world)
   - Campaign reference (ID, version, name)
   - Timestamp
   - Save version

### Loading a Saved Game

**Option 1: Load from game menu**:
```
antares> load
Available saves:
  1. my_progress
  2. checkpoint_1
  3. before_boss
Enter save number: 1
Game loaded: my_progress
```

**Option 2: Load from command line**:
```bash
antares --load my_progress
```

**Option 3: Continue last save**:
```bash
antares --continue
```

### Important Notes About Saves

- **Campaign Reference**: Saves remember which campaign they belong to
- **Version Tracking**: Saves track campaign version for compatibility
- **Campaign Required**: To load a save, the campaign must be installed
- **Campaign Not Embedded**: The campaign itself is not saved, only a reference to it

---

## Part 7: Using Custom Directories

### Override Default Directories

You can use custom locations for campaigns and saves:

```bash
# Custom campaigns directory
antares --campaign tutorial --campaigns-dir ~/my_campaigns

# Custom saves directory
antares --campaigns-dir ~/my_campaigns --saves-dir ~/my_saves

# Both custom directories
antares --campaign tutorial \
        --campaigns-dir ~/my_campaigns \
        --saves-dir ~/my_saves
```

### Why Use Custom Directories?

- **Organization**: Keep campaigns separate from game installation
- **Backup**: Easier to backup campaigns and saves
- **Multiple Installs**: Use same campaigns with different game versions
- **Sharing**: Share campaign directory with other players

---

## Troubleshooting

### Problem: "Campaign 'X' not found"

**Cause**: Campaign is not installed or in wrong directory.

**Solution**:
1. Check campaigns directory: `ls ~/.antares/campaigns/`
2. Verify campaign ID matches directory name
3. Re-install campaign if missing
4. Use `--list-campaigns` to see available campaigns

---

### Problem: "Campaign validation failed"

**Cause**: Campaign has errors (missing files, invalid data).

**Solution**:
1. Run validation: `antares --validate-campaign X`
2. Check error messages
3. Contact campaign author for fixed version
4. If you're the author, fix in Campaign Builder

---

### Problem: "Save file version mismatch"

**Cause**: Save was created with different game version.

**Solution**:
1. Update game to version that created the save, OR
2. Accept that save may not be compatible
3. Start new game with current version

---

### Problem: "Campaign version mismatch"

**Cause**: Save references older/newer campaign version.

**Solution**:
1. Update campaign to version that matches save, OR
2. Accept potential compatibility issues
3. Start new game with current campaign version

---

### Problem: Save file won't load

**Cause**: Corrupted save or parse error.

**Solution**:
1. Check save file exists: `ls saves/my_save.ron`
2. Verify save file is valid RON format
3. Try loading a different save
4. If all saves fail, reinstall game

---

## Best Practices

### Before Starting a Campaign

- ✅ Read the campaign's README.md
- ✅ Validate the campaign
- ✅ Check author/version information
- ✅ Backup the campaign directory
- ✅ Note the recommended difficulty

### During Gameplay

- ✅ Save frequently (especially before tough encounters)
- ✅ Use descriptive save names (`before_boss`, `chapter_2_start`)
- ✅ Keep multiple save files (don't overwrite only save)
- ✅ Note campaign version in save name if needed

### After Playing

- ✅ Backup your save files
- ✅ Leave feedback for campaign author
- ✅ Report bugs or issues encountered
- ✅ Share your experience with the community

---

## Example Workflow

### Complete Campaign Playthrough

```bash
# 1. Download campaign
curl -O https://example.com/dark_tower_v1.2.0.zip

# 2. Install campaign
unzip dark_tower_v1.2.0.zip -d ~/.antares/campaigns/

# 3. List campaigns to verify
antares --list-campaigns
# Output: dark_tower - Dark Tower v1.2.0

# 4. Validate campaign
antares --validate-campaign dark_tower
# Output: ✓ Campaign is valid!

# 5. Start new game
antares --campaign dark_tower

# 6. Play and save progress
antares> save
Enter save name: dark_tower_chapter_1
Game saved: dark_tower_chapter_1

# 7. Later, resume game
antares --load dark_tower_chapter_1

# 8. Complete campaign and share feedback!
```

---

## Advanced Tips

### Playing Multiple Campaigns

You can have multiple campaigns installed and switch between them:

```bash
# Play tutorial campaign
antares --campaign tutorial

# Later, play a different campaign
antares --campaign dark_tower
```

Each campaign maintains separate save files (distinguished by campaign reference).

### Organizing Campaigns by Genre

```bash
# Create genre-specific directories
mkdir -p ~/campaigns/fantasy
mkdir -p ~/campaigns/scifi
mkdir -p ~/campaigns/horror

# Install campaigns by genre
cp dark_tower ~/campaigns/fantasy/
cp space_quest ~/campaigns/scifi/
cp haunted_manor ~/campaigns/horror/

# Play using custom directory
antares --campaign dark_tower --campaigns-dir ~/campaigns/fantasy
```

### Sharing Progress with Friends

Save files can be shared if both players have the same campaign installed:

```bash
# Export your save
cp saves/my_progress.ron ~/shared/

# Friend imports save
cp ~/shared/my_progress.ron saves/

# Friend loads save
antares --load my_progress
```

**Note**: Campaign must be identical (same version) for save compatibility.

---

## Campaign Quality Indicators

When choosing campaigns, look for:

- ✅ **Complete README**: Author info, description, version history
- ✅ **Recent Updates**: Active maintenance shows quality
- ✅ **Community Feedback**: Positive reviews and ratings
- ✅ **Validation Passes**: No errors when validating
- ✅ **Asset Quality**: Professional tilesets, music, sounds
- ✅ **Balanced Difficulty**: Not too easy or frustratingly hard

---

## Next Steps

- **Create Your Own**: Try the Campaign Builder to make campaigns
- **Join Community**: Share experiences and discover new campaigns
- **Contribute**: Help improve campaigns with feedback and bug reports
- **Explore**: Try different campaign genres and styles

---

## See Also

- [Phase 14 Implementation Documentation](../explanation/phase14_campaign_integration_implementation.md) (technical details)
- [Campaign Builder User Guide](./use_campaign_builder.md) (create campaigns)
- [Campaign Validation Rules](../reference/campaign_validation_rules.md) (validation reference)
- [Save Game Format](../reference/save_game_format.md) (save file structure)

---

**Enjoy your adventures in custom campaigns!**
