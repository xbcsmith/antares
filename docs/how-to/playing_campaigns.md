# How to Play Campaigns in Antares

This guide explains how to play campaigns created with the Campaign Builder.

---

## Prerequisites

- Antares game installed
- At least one campaign in the `campaigns/` directory
- Rust toolchain installed (for running from source)

---

## Quick Start

### 1. List Available Campaigns

```bash
cargo run --bin antares -- --list-campaigns
```

This displays all campaigns with their metadata:

```
Available Campaigns:

  tutorial - Tutorial: First Steps v1.0.0
    Author: Antares Development Team
    A small introductory campaign demonstrating core game mechanics.

  my_adventure - My Custom Adventure v1.0.0
    Author: Your Name
    An epic quest through dangerous dungeons.
```

### 2. Validate a Campaign (Optional)

Before playing, you can check if a campaign is valid:

```bash
cargo run --bin antares -- --validate-campaign tutorial
```

Output:

```
Validating campaign: tutorial

Validation Results:
  Errors: 0
  Warnings: 0

✓ Campaign is valid!

To play this campaign:
  $ antares --campaign tutorial
```

### 3. Launch the Game

Start a new game with a campaign:

```bash
cargo run --bin antares -- --campaign tutorial
```

Start a new game without a campaign (core content only):

```bash
cargo run --bin antares
```

---

## In-Game Commands

Once the game is running, you have these commands available:

### Status Command

Shows current game state:

```
antares> status
```

Output:

```
=== Game Status ===
Campaign: Tutorial: First Steps v1.0.0
Author: Antares Development Team

Game Mode: Exploration
Day: 1, Time: 6:00

Party:
  Members: 0
  Gold: 100
  Food: 50
  Gems: 0

Roster:
  Total Characters: 0
```

### Save Command

Save your current game:

```
antares> save
Enter save name: my_save
Game saved: my_save
```

Save files are stored in the `saves/` directory as RON files.

### Load Command

Load a previously saved game:

```
antares> load
Available saves:
  1. my_save
  2. quicksave
  3. autosave
Enter save number: 1
Game loaded: my_save
```

### Quit Command

Exit the game:

```
antares> quit
Thanks for playing Antares!
```

---

## Advanced Usage

### Custom Directories

Specify custom campaign and save directories:

```bash
cargo run --bin antares -- \
  --campaigns-dir /path/to/campaigns \
  --saves-dir /path/to/saves \
  --campaign my_campaign
```

### Continue from Last Save

Automatically load the most recent save:

```bash
cargo run --bin antares -- --continue
```

Note: Currently loads the first save alphabetically. Future versions will track last played.

### Load Specific Save

Load a save directly on launch:

```bash
cargo run --bin antares -- --load my_save
```

---

## Save File Format

Save files are stored as RON (Rusty Object Notation) for human readability.

**Location**: `saves/my_save.ron`

**Structure**:

```ron
(
    version: "0.1.0",
    timestamp: "2025-01-15T10:30:00Z",
    campaign_reference: Some((
        id: "tutorial",
        version: "1.0.0",
        name: "Tutorial: First Steps",
    )),
    game_state: (
        world: (...),
        roster: (...),
        party: (...),
        // ... rest of game state
    )
)
```

---

## Campaign Structure

Campaigns are located in the `campaigns/` directory:

```
campaigns/
├── tutorial/
│   ├── campaign.ron          # Campaign metadata and config
│   ├── data/
│   │   ├── items.ron         # Items definitions
│   │   ├── spells.ron        # Spells definitions
│   │   ├── monsters.ron      # Monsters definitions
│   │   ├── classes.ron       # Character classes
│   │   ├── races.ron         # Character races
│   │   ├── quests.ron        # Quests
│   │   ├── dialogues.ron     # NPC dialogues
│   │   └── maps/             # Map files
│   │       ├── town.ron
│   │       └── dungeon.ron
│   └── assets/               # Graphics, sounds, music
│       ├── tilesets/
│       ├── music/
│       └── sounds/
└── my_campaign/
    └── ...
```

---

## Troubleshooting

### Campaign Not Found

**Error**: `Campaign 'my_campaign' not found`

**Solution**:
1. Check campaign exists in `campaigns/` directory
2. Verify campaign name matches directory name exactly
3. List campaigns with `--list-campaigns` to see available options

### Campaign Validation Failed

**Error**: `Campaign has X validation error(s)`

**Solution**:
1. Run `--validate-campaign <id>` to see specific errors
2. Open campaign in Campaign Builder to fix issues
3. Common issues:
   - Missing required data files
   - Invalid references (items, spells, monsters)
   - Missing maps

### Save File Version Mismatch

**Error**: `Save file version mismatch: expected 0.1.0, found 0.0.9`

**Solution**:
- Save file is from older version of Antares
- Currently requires exact version match
- Future versions will support migration

### Save File Campaign Missing

**Error**: `Campaign 'old_campaign' referenced in save file not found`

**Solution**:
- Save file references a campaign that's been deleted or renamed
- Reinstall the campaign or start a new game

---

## Campaign Starting Configuration

When you start a new game with a campaign, the following are applied from the campaign's config:

- **Starting Gold**: Party's initial gold (e.g., 100)
- **Starting Food**: Party's initial food units (e.g., 50)
- **Starting Map**: Map ID where game begins
- **Starting Position**: X,Y coordinates on starting map
- **Starting Direction**: Direction party faces (North/South/East/West)
- **Max Party Size**: Maximum active party members (default: 6)
- **Max Roster Size**: Maximum total characters (default: 20)
- **Difficulty**: Easy/Normal/Hard/Brutal
- **Permadeath**: Character death is permanent (true/false)
- **Multiclassing**: Allow characters to change classes (true/false)

---

## Tips for Players

### First Time Playing

1. Start with the `tutorial` campaign to learn mechanics
2. Save frequently - save files are small and fast
3. Use `status` command to check resources before major decisions

### Multiple Campaigns

- Each save file tracks its campaign reference
- You can play multiple campaigns simultaneously
- Switch between campaigns by loading different saves

### Backup Saves

Save files are plain text RON files in `saves/`:

```bash
# Backup all saves
cp -r saves/ saves_backup/

# Backup specific save
cp saves/my_save.ron saves/my_save_backup.ron
```

---

## Building from Source

If you're running from source:

```bash
# Development build (faster compilation, slower execution)
cargo run --bin antares -- --campaign tutorial

# Release build (optimized)
cargo run --release --bin antares -- --campaign tutorial
```

---

## Next Steps

- **Create Your Own Campaign**: See [Creating Campaigns Tutorial](../tutorials/creating_campaigns.md)
- **Modding Guide**: See [Modding Guide](../explanation/modding_guide.md) for comprehensive gameplay concepts
- **Campaign Validation**: See [Creating and Validating Campaigns](creating_and_validating_campaigns.md)

---

## Support

If you encounter issues:

1. Check this guide's Troubleshooting section
2. Validate your campaign with `--validate-campaign`
3. Check campaign structure matches expected format
4. Report bugs on GitHub with save file and campaign details
