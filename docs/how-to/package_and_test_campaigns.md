# How-To: Package and Test Campaigns

**Phase:** 13 (Distribution Tools)
**Audience:** Campaign Creators
**Difficulty:** Intermediate

---

## Overview

This guide explains how to package your Antares campaign for distribution and test it before release.

---

## Prerequisites

- Completed campaign with all data files (items, spells, monsters, maps)
- Campaign passes validation (zero critical errors)
- Antares game executable installed and accessible
- Campaign Builder Phase 13 or later

---

## Part 1: Validating Your Campaign

Before packaging or testing, ensure your campaign is valid.

### Step 1: Open the Validation Tab

1. Launch Campaign Builder
2. Open your campaign (File → Open Campaign)
3. Click the **Validation** tab

### Step 2: Review Validation Results

**All Checks Passed:**
- Green checkmark: ✅ "All Checks Passed!"
- Your campaign is ready to package and test

**Errors or Warnings Found:**
- Red ❌ indicates critical errors (must fix before export)
- Yellow ⚠️ indicates warnings (fix recommended but not required)

### Step 3: Fix Validation Errors

Common errors and fixes:

| Error | Fix |
|-------|-----|
| "Campaign ID is empty" | Go to Metadata tab, set a unique ID (lowercase, no spaces) |
| "Invalid version format" | Use semantic versioning: `1.0.0` |
| "Party size exceeds roster size" | Adjust max_party_size or max_roster_size in Metadata |
| "Starting level exceeds max level" | Fix level ranges in Metadata |
| "File path empty" | Ensure all data file paths are set (items_file, spells_file, etc.) |

After fixing errors, click Validation tab again to re-check.

---

## Part 2: Managing Campaign Assets

### What Are Assets?

Assets are external files used by your campaign:
- **Tilesets**: Images for map tiles (.png, .jpg)
- **Portraits**: Character/NPC images (.png, .jpg)
- **Music**: Background music (.mp3, .ogg, .flac)
- **Sounds**: Sound effects (.wav, .ogg)
- **Documentation**: README files (.md, .txt)
- **Data**: Custom data files (.ron, .yaml, .json)

### Step 1: Open Asset Manager

1. Click the **Assets** tab in Campaign Builder
2. Asset Manager will scan your campaign directory

### Step 2: Review Asset Statistics

The Asset Manager displays:
- **Total Assets**: Number of files found
- **Total Size**: Combined size of all assets
- **Type Counts**: Assets grouped by type

### Step 3: Check for Unreferenced Assets

At the bottom, you'll see:
- "⚠ X unreferenced assets found"

Unreferenced assets are files not used by your campaign. Consider:
- **Remove**: Delete unused files to reduce package size
- **Keep**: Leave them if you plan to use them later

### Step 4: Organize Assets (Optional)

Recommended directory structure:

```
my_campaign/
├── campaign.ron
├── data/
│   ├── items.ron
│   ├── spells.ron
│   ├── monsters.ron
│   ├── classes.ron
│   └── races.ron
├── maps/
│   ├── town_01.ron
│   └── dungeon_01.ron
├── quests/
│   └── quests.ron
├── dialogues/
│   └── dialogues.ron
├── assets/
│   ├── tilesets/
│   │   ├── dungeon.png
│   │   └── town.png
│   ├── portraits/
│   │   ├── hero.png
│   │   └── npc_merchant.png
│   ├── music/
│   │   ├── battle.mp3
│   │   └── town.mp3
│   └── sounds/
│       ├── hit.wav
│       └── spell.wav
└── README.md
```

### Step 5: Add New Assets (Future)

**Note**: UI for adding assets will be implemented in Phase 15. For now, manually copy files to campaign directory and click **Refresh**.

---

## Part 3: Test Playing Your Campaign

### Step 1: Configure Test Play

Before launching, configure test play settings:

**Game Executable Path:**
- Default: `antares` (assumes game is in PATH)
- Custom: Set path to `antares` executable (e.g., `/usr/local/bin/antares`)

**Debug Mode:**
- Enabled: Game runs with extra logging for debugging
- Disabled: Normal game mode

**Auto-Save:**
- Enabled: Campaign is saved before launching (recommended)
- Disabled: Use current saved state

**Validate First:**
- Enabled: Blocks launch if validation errors exist (recommended)
- Disabled: Launch even with errors (not recommended)

### Step 2: Launch Test Play

**From Code (Current Phase 13):**

```rust
use antares::sdk::campaign_builder::test_play::{TestPlayConfig, TestPlaySession};
use std::path::PathBuf;

// Configure test play
let config = TestPlayConfig {
    game_executable: PathBuf::from("antares"),
    debug_mode: true,
    auto_save: true,
    validate_first: true,
    max_log_lines: 1000,
};

// Launch game
match app.launch_test_play(&config) {
    Ok(session) => {
        println!("Game launched! Campaign: {}", session.campaign_id());
        // Session is now running
    }
    Err(e) => {
        eprintln!("Failed to launch: {}", e);
    }
}
```

**From UI (Phase 15):**
- Click **Tools** → **Test Play** (not yet implemented)

### Step 3: Monitor Test Play

While the game is running:
- Play through your campaign
- Test all maps, events, combat, dialogues
- Check for bugs or balance issues
- Verify all assets load correctly

### Step 4: End Test Play

**Terminate Session:**

```rust
// Stop the game
session.terminate()?;
```

**Game Closes Naturally:**
- Session automatically detects when game exits

### Step 5: Review Logs

After test play, check logs for errors:

```rust
// Output log (stdout)
for line in session.output_log() {
    println!("[OUT] {}", line);
}

// Error log (stderr)
for line in session.error_log() {
    eprintln!("[ERR] {}", line);
}

// Session duration
println!("Test play lasted: {:?}", session.elapsed());
```

---

## Part 4: Packaging Your Campaign

### Step 1: Prepare Campaign Metadata

Before exporting, update campaign metadata:

1. Go to **Metadata** tab
2. Set campaign details:
   - **Name**: Display name for your campaign
   - **Version**: Semantic version (e.g., `1.0.0`)
   - **Author**: Your name or studio
   - **Description**: Brief campaign overview

### Step 2: Increment Version (If Re-Releasing)

If updating an existing campaign:

```rust
use antares::sdk::campaign_builder::packager::{increment_version, VersionIncrement};

// Current version: 1.0.0

// Patch release (bug fixes): 1.0.0 -> 1.0.1
let new_version = increment_version("1.0.0", VersionIncrement::Patch);

// Minor release (new features): 1.0.0 -> 1.1.0
let new_version = increment_version("1.0.0", VersionIncrement::Minor);

// Major release (breaking changes): 1.0.0 -> 2.0.0
let new_version = increment_version("1.0.0", VersionIncrement::Major);
```

**Semantic Versioning Guide:**
- **Patch** (X.Y.Z): Bug fixes, typos, minor tweaks
- **Minor** (X.Y.0): New content, new features (backward compatible)
- **Major** (X.0.0): Major overhaul, breaking changes

### Step 3: Select Export Location

Choose where to save the .zip file:

```rust
use std::path::PathBuf;

let output_path = PathBuf::from("/home/user/my_campaign_v1.0.0.zip");
```

**Naming Convention:**
- Format: `{campaign_id}_v{version}.zip`
- Example: `dark_tower_v1.0.0.zip`

### Step 4: Choose Compression Level

Compression levels (0-9):
- **0**: No compression (fast, large file)
- **6**: Default compression (balanced)
- **9**: Maximum compression (slow, small file)

Recommended: **6** (default)

### Step 5: Export Campaign

```rust
// Export campaign
match app.export_campaign(output_path, compression_level) {
    Ok(_) => {
        println!("Campaign exported successfully!");
    }
    Err(e) => {
        eprintln!("Export failed: {}", e);
    }
}
```

**What Gets Included:**
- `campaign.ron` (campaign metadata)
- All data files (items, spells, monsters, classes, races)
- All maps (`.ron` files in maps directory)
- Quest and dialogue files
- `README.md` (if present)
- All assets in `assets/` directory (if present)

### Step 6: Verify Export

After export:
1. Check that .zip file was created
2. Extract to a temporary directory
3. Verify all files are present
4. Test by importing the campaign

---

## Part 5: Sharing Your Campaign

### Option 1: Manual Distribution

1. Upload .zip to file hosting (Google Drive, Dropbox, GitHub Releases)
2. Share link with players
3. Provide installation instructions:
   ```
   1. Download my_campaign_v1.0.0.zip
   2. Extract to: ~/.antares/campaigns/
   3. Launch Antares
   4. Select "Load Campaign" -> "my_campaign"
   ```

### Option 2: Import Campaign (For Testing)

Test the import process:

```rust
use std::path::PathBuf;

let package_path = PathBuf::from("/downloads/my_campaign_v1.0.0.zip");
let install_dir = PathBuf::from("/home/user/.antares/campaigns");

match app.import_campaign(package_path, install_dir) {
    Ok(_) => {
        println!("Campaign installed successfully!");
    }
    Err(e) => {
        eprintln!("Import failed: {}", e);
    }
}
```

---

## Troubleshooting

### Export Fails: "Campaign has validation errors"

**Problem**: Campaign contains critical errors.

**Solution**:
1. Go to Validation tab
2. Fix all ❌ errors
3. Re-run validation
4. Try export again

---

### Export Fails: "Campaign directory not found"

**Problem**: Campaign not saved or path invalid.

**Solution**:
1. Save campaign (File → Save Campaign)
2. Verify campaign path is set
3. Try export again

---

### Test Play Fails: "Game executable not found"

**Problem**: Antares game not installed or path incorrect.

**Solution**:
1. Install Antares game
2. Set correct path in `TestPlayConfig.game_executable`
3. Verify executable exists: `which antares` or `ls /path/to/antares`

---

### Test Play Fails: "Campaign has validation errors"

**Problem**: Validation errors block launch.

**Solution**:
1. Fix validation errors, OR
2. Disable validation: `config.validate_first = false` (not recommended)

---

### Import Fails: "Archive error"

**Problem**: .zip file is corrupted or invalid format.

**Solution**:
1. Re-download campaign package
2. Verify .zip file is valid (try extracting manually)
3. Contact campaign author for fixed version

---

### Assets Not Loading in Game

**Problem**: Game can't find tileset/music/sound files.

**Solution**:
1. Check Asset Manager for missing assets
2. Verify asset paths in map/event data match actual file names
3. Ensure assets are in correct subdirectories (assets/tilesets/, etc.)
4. Check file extensions (.png, .mp3, etc.) are correct

---

## Best Practices

### Before Every Release

- ✅ Run full validation (zero errors)
- ✅ Test play entire campaign (start to finish)
- ✅ Check all maps are accessible
- ✅ Verify all dialogues work
- ✅ Test combat balance
- ✅ Remove unreferenced assets
- ✅ Update README.md with credits, instructions, changelog

### Versioning Strategy

- **Alpha** (0.x.y): Early development, incomplete
- **Beta** (0.9.x): Feature-complete, testing phase
- **Release** (1.0.0): Public release
- **Updates** (1.x.y): Bug fixes and new content

### Documentation

Include a README.md with:
- Campaign name and version
- Author/credits
- Brief description
- Installation instructions
- Known issues
- Changelog (for updates)

### File Size Optimization

- Compress images (PNG optimization tools)
- Use .ogg for music (smaller than .mp3)
- Remove unused assets
- Use lower bitrate audio if acceptable

---

## Example Workflow

### Initial Release (1.0.0)

1. Complete campaign development
2. Validate campaign (fix all errors)
3. Organize assets in proper directories
4. Test play entire campaign
5. Update metadata (name, author, description, version: `1.0.0`)
6. Write README.md
7. Export: `my_campaign_v1.0.0.zip` (compression: 6)
8. Test import on clean install
9. Upload to file host
10. Share with players

### Update Release (1.0.1)

1. Fix bugs reported by players
2. Increment version: `1.0.0` → `1.0.1`
3. Update README.md changelog
4. Test play fixed content
5. Export: `my_campaign_v1.0.1.zip`
6. Share update

---

## Next Steps

- **Phase 14**: Game engine will support `--campaign` flag for loading campaigns
- **Phase 15**: UI for export wizard, test play log viewer, asset upload

---

## See Also

- [Phase 13 Implementation Documentation](../explanation/phase13_distribution_tools_implementation.md)
- [Campaign Validation Rules](../reference/campaign_validation_rules.md)
- [Asset File Format Guide](../reference/asset_formats.md)
- [Semantic Versioning Specification](https://semver.org/)

---

**Questions or issues? Check the troubleshooting section or file an issue on GitHub.**
