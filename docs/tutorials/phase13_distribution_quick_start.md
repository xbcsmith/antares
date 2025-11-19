# Quick Start: Campaign Distribution Tools (Phase 13)

**Phase:** 13 (Distribution Tools)
**Time:** 15 minutes
**Level:** Beginner

---

## What You'll Learn

In this tutorial, you'll learn how to:
1. Validate your campaign for export
2. Manage campaign assets
3. Package your campaign for distribution
4. Test your campaign with the game engine

---

## Prerequisites

- Campaign Builder installed (Phase 13 or later)
- A campaign project created (any stage of completion)
- Basic familiarity with the Campaign Builder UI

---

## Step 1: Open Your Campaign

1. Launch Campaign Builder
2. Click **File** → **Open Campaign**
3. Navigate to your campaign directory
4. Select `campaign.ron`

Your campaign will load into the builder.

---

## Step 2: Validate Your Campaign

Before packaging, ensure your campaign has no errors.

1. Click the **Validation** tab
2. Review the results:
   - ✅ Green checkmark = All good!
   - ❌ Red errors = Must fix before export
   - ⚠️ Yellow warnings = Optional fixes

3. If you see errors, click the error message to see details
4. Go to the **Metadata** tab to fix common errors:
   - Set a campaign ID (lowercase, no spaces)
   - Set version to `1.0.0`
   - Fill in name, author, description

5. Return to **Validation** tab and verify all errors are gone

---

## Step 3: Review Your Assets

Assets are images, sounds, and other files your campaign uses.

1. Click the **Assets** tab
2. You'll see:
   - **Total Assets**: Number of files
   - **Total Size**: Combined file size
   - **Asset List**: All files in your campaign

3. Look for the yellow warning at the bottom:
   - "⚠ X unreferenced assets found"
   - These are files not used by your campaign
   - Consider deleting them to reduce package size

4. Click **🔄 Refresh** if you added files manually

---

## Step 4: Package Your Campaign (Code Example)

Currently, packaging is done via code. UI will be added in Phase 15.

```rust
use antares::sdk::campaign_builder::packager::{increment_version, VersionIncrement};
use std::path::PathBuf;

// Update version if this is a re-release
let new_version = increment_version(&app.campaign.version, VersionIncrement::Patch);
app.campaign.version = new_version;

// Set output path
let output_path = PathBuf::from("/home/user/my_campaign_v1.0.0.zip");

// Export campaign
match app.export_campaign(output_path, 6) {
    Ok(_) => println!("Campaign exported successfully!"),
    Err(e) => eprintln!("Export failed: {}", e),
}
```

**What gets packaged:**
- `campaign.ron` (metadata)
- All data files (items, spells, monsters, classes, races)
- All maps from `maps/` directory
- Quests and dialogues
- `README.md` (if present)
- Assets in `assets/` directory

---

## Step 5: Test Your Campaign (Code Example)

Launch the game with your campaign to test it.

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

// Check if game is available
if !app.can_launch_test_play(&config) {
    eprintln!("Game executable not found!");
    return;
}

// Launch game
match app.launch_test_play(&config) {
    Ok(session) => {
        println!("Game launched! Campaign: {}", session.campaign_id());

        // Wait for game to exit
        while session.is_running() {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        println!("Test play session lasted: {:?}", session.elapsed());
    }
    Err(e) => eprintln!("Failed to launch: {}", e),
}
```

**What happens:**
1. Campaign is saved to disk
2. Validation runs (blocks if errors found)
3. Game launches with `--campaign <id>` flag
4. You can play through your campaign
5. Game output/errors are captured for debugging

---

## Step 6: Share Your Campaign

Once exported, share your campaign:

1. Locate the .zip file (e.g., `my_campaign_v1.0.0.zip`)
2. Upload to file hosting:
   - Google Drive
   - Dropbox
   - GitHub Releases
   - itch.io
3. Share the download link with players
4. Include installation instructions:

```
Installation Instructions:

1. Download my_campaign_v1.0.0.zip
2. Extract the ZIP file
3. Copy the campaign folder to:
   - Linux/Mac: ~/.antares/campaigns/
   - Windows: %APPDATA%\antares\campaigns\
4. Launch Antares
5. Select "Load Campaign" → "my_campaign"
6. Enjoy!
```

---

## Common Tasks

### Updating Your Campaign

If you release an update:

```rust
// Increment version
let new_version = increment_version("1.0.0", VersionIncrement::Patch);
// Result: "1.0.1"

app.campaign.version = new_version;
```

Version types:
- **Patch** (1.0.0 → 1.0.1): Bug fixes
- **Minor** (1.0.0 → 1.1.0): New content
- **Major** (1.0.0 → 2.0.0): Major changes

### Checking Package Contents

Before export, see what will be included:

```rust
let files = app.get_package_files();
for file in files {
    println!("Will include: {}", file.display());
}
```

### Importing a Campaign

To test someone else's campaign:

```rust
let package_path = PathBuf::from("/downloads/their_campaign.zip");
let install_dir = PathBuf::from("/home/user/.antares/campaigns");

match app.import_campaign(package_path, install_dir) {
    Ok(_) => println!("Campaign imported!"),
    Err(e) => eprintln!("Import failed: {}", e),
}
```

---

## Troubleshooting

### "Campaign has validation errors"

**Fix:** Go to Validation tab, fix all ❌ errors, try again.

### "Game executable not found"

**Fix:** Install Antares game or set correct path:
```rust
config.game_executable = PathBuf::from("/usr/local/bin/antares");
```

### "Campaign directory not found"

**Fix:** Save your campaign first (File → Save Campaign).

### Assets not loading in game

**Fix:**
1. Check Asset Manager for missing files
2. Verify file paths in map/event data
3. Ensure assets are in correct directories (assets/tilesets/, etc.)

---

## Next Steps

- **Add Assets**: Copy images/sounds to `assets/` directory, click Refresh
- **Write README**: Create `README.md` with campaign description and credits
- **Get Feedback**: Share with testers, collect feedback
- **Iterate**: Fix bugs, add content, increment version, re-export

---

## Phase 15 Preview

Coming soon in Phase 15:
- **Export Wizard UI**: Step-by-step export dialog
- **Test Play Panel**: Log viewer with real-time output
- **Asset Upload**: Drag-and-drop file upload
- **Asset Preview**: View images, play sounds

---

## Summary

You've learned how to:
- ✅ Validate campaigns before export
- ✅ Review and manage assets
- ✅ Package campaigns as .zip archives
- ✅ Test campaigns with the game engine
- ✅ Share campaigns with players

**Congratulations!** You can now distribute your Antares campaigns!

---

## See Also

- [How-To: Package and Test Campaigns](../how-to/package_and_test_campaigns.md) (detailed guide)
- [Phase 13 Implementation](../explanation/phase13_distribution_tools_implementation.md) (technical details)
- [Campaign Validation Rules](../reference/campaign_validation_rules.md) (validation reference)

---

**Ready to share your adventure? Happy modding!**
