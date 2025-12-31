# How-To: Creating and Validating Campaigns

This guide shows you how to create, validate, and prepare campaigns for distribution in Antares.

---

## Prerequisites

- Antares SDK installed
- Basic understanding of RON format
- Text editor for editing data files

---

## Part 1: Creating a New Campaign

### Step 1: Use the Example Template

The easiest way to start is by copying the example campaign:

```bash
# Copy the example campaign
cp -r campaigns/example campaigns/my_campaign

# Navigate to your new campaign
cd campaigns/my_campaign
```

### Step 2: Edit Campaign Metadata

Edit `campaign.ron` with your campaign details:

```ron
Campaign(
    id: "my_campaign",
    name: "My Awesome Campaign",
    version: "1.0.0",
    author: "Your Name",
    description: "An epic adventure through...",
    engine_version: "0.1.0",
    required_features: [],

    config: CampaignConfig(
        starting_map: 1,
        starting_position: Position(x: 10, y: 10),
        starting_direction: North,
        starting_gold: 200,
        starting_food: 100,
        max_party_size: 6,
        max_roster_size: 20,
        difficulty: Normal,
        permadeath: false,
        allow_multiclassing: true,
        starting_level: 1,
        max_level: 20,
    ),

    data: CampaignData(
        items: "data/items.ron",
        spells: "data/spells.ron",
        monsters: "data/monsters.ron",
        classes: "data/classes.ron",
        races: "data/races.ron",
        maps: "data/maps",
        quests: "data/quests.ron",
        dialogues: "data/dialogues.ron",
    ),

    assets: CampaignAssets(
        tilesets: "assets/tilesets",
        music: "assets/music",
        sounds: "assets/sounds",
        images: "assets/images",
    ),
)
```

### Step 3: Update the README

Edit `README.md` with your campaign information:

```markdown
# My Awesome Campaign

A thrilling adventure through dangerous dungeons and mysterious lands.

## Description

Your detailed campaign description here...

## Story

The kingdom is in peril! An ancient evil has awakened...

## Features

- 20 unique maps to explore
- 15 challenging quests
- Custom items and monsters
- Epic boss battles

## Requirements

- Antares Engine v0.1.0 or later

## Installation

1. Copy this campaign to your campaigns/ folder
2. Launch Antares and select "My Awesome Campaign"
3. Start your adventure!
```

### Step 4: Add Content

Edit the data files in the `data/` directory:

**Items** (`data/items.ron`):

```ron
[
    Item(
        id: 1,
        name: "Iron Sword",
        description: "A sturdy iron blade",
        // ... rest of item definition
    ),
    // Add more items...
]
```

**Quests** (`data/quests.ron`):

```ron
[
    Quest(
        id: 1,
        name: "The Missing Sword",
        description: "Recover the stolen sword",
        stages: [
            QuestStage(
                stage_number: 1,
                name: "Find the Thieves",
                objectives: [
                    KillMonsters(monster_id: 3, quantity: 5),
                ],
            ),
        ],
        rewards: [
            Experience(500),
            Gold(100),
        ],
    ),
]
```

**Maps** (`data/maps/map001.ron`):

```ron
Map(
    id: 1,
    name: "Starting Town",
    width: 20,
    height: 20,
    // ... map data
)
```

**Map Events** (`data/maps/map001.ron`):

Events are defined using a position-based system. Add events to the `events` field as a HashMap:

```ron
Map(
    id: 1,
    name: "Tutorial Town",
    description: "A peaceful starting town",
    width: 20,
    height: 20,
    tiles: [
        // ... tile definitions ...
    ],
    events: {
        Position(x: 5, y: 5): MapEvent::Sign(
            name: "Welcome Sign",
            description: "A wooden sign with carved letters",
            text: "Welcome to Antares! Explore the town and talk to the townsfolk.",
        ),
        Position(x: 10, y: 3): MapEvent::Treasure(
            name: "Treasure Chest",
            description: "A locked wooden chest",
            loot: [
                LootItem(item_id: 1, quantity: 50),  // 50 gold
                LootItem(item_id: 42, quantity: 1),  // Magic sword
            ],
        ),
        Position(x: 15, y: 8): MapEvent::Combat(
            name: "Goblin Ambush",
            description: "Goblins leap from the shadows!",
            monsters: [
                MonsterSpawn(monster_id: 1, count: 3),  // 3 goblins
            ],
        ),
    },
    npcs: [],
)
```

**Event Types Available:**

- **Sign**: Display text to player
- **Treasure**: Award items/gold to party
- **Combat**: Trigger tactical battle
- **Teleport**: Transport party to another map/position
- **Trap**: Deal damage or apply conditions
- **NpcDialogue**: Start conversation with NPC

**Important:** Do NOT use the deprecated `event_trigger` field on tiles. Events are now exclusively defined in the `events` HashMap, keyed by position. See `docs/explanation/map_event_system.md` for complete documentation.

---

## Part 2: Validating Your Campaign

### Step 1: Run Basic Validation

Validate your campaign structure and content:

```bash
campaign_validator campaigns/my_campaign
```

**Expected Output**:

```
Campaign: My Awesome Campaign v1.0.0
Author: Your Name
Engine: 0.1.0

[1/5] Validating campaign structure...
[2/5] Loading content database...
  Classes: 6
  Items: 23
  Maps: 5
  Quests: 8
  Dialogues: 12
[3/5] Validating cross-references...
[4/5] Validating quests...
[5/5] Validating dialogues...

✓ Campaign is VALID

No issues found!
```

### Step 2: Verbose Validation

For more detailed output:

```bash
campaign_validator -v campaigns/my_campaign
```

This shows progress through each validation stage with detailed statistics.

### Step 3: Fix Validation Errors

If validation fails, you'll see error messages:

```
✗ Campaign is INVALID

Errors (3):
  1. Missing 'data' directory
  2. Quest 1: Invalid monster ID: 99
  3. No maps defined - campaign cannot be played

Warnings (1):
  1. No classes defined
```

**Common Errors and Fixes**:

#### Error: "Missing 'data' directory"

```bash
mkdir -p campaigns/my_campaign/data
```

#### Error: "campaign.ron not found"

Create the file with proper Campaign structure (see Step 2).

#### Error: "No maps defined"

Add at least one map in `data/maps/`:

```bash
touch campaigns/my_campaign/data/maps/map001.ron
# Edit with map data
```

#### Error: "Quest 1: Invalid monster ID: 99"

The quest references a monster that doesn't exist. Either:

- Add the monster to `data/monsters.ron`, or
- Change the quest to use a valid monster ID

#### Error: "Dialogue 5: Node 10 is orphaned"

The dialogue has an unreachable node. Either:

- Connect the node to the dialogue tree, or
- Remove the orphaned node

#### Error: "starting_level (10) > max_level (5)"

Fix in `campaign.ron`:

```ron
starting_level: 1,
max_level: 20,
```

### Step 4: Address Warnings

Warnings won't prevent your campaign from working but should be addressed:

#### Warning: "No classes defined"

Add classes in `data/classes.ron`:

```ron
[
    Class(
        id: "knight",
        name: "Knight",
        // ... class definition
    ),
]
```

#### Warning: "No items defined"

Add items in `data/items.ron`.

---

## Part 3: Batch Validation

### Validate Multiple Campaigns

To validate all campaigns in a directory:

```bash
campaign_validator --all
```

**Output**:

```
Validating 3 campaigns...

Validating Example Campaign... ✓ VALID
Validating My Campaign... ✗ INVALID
Validating Test Campaign... ✓ VALID

=== Summary ===
Total campaigns: 3
Valid: 2
Invalid: 1
Total errors: 3
Total warnings: 1
```

### Custom Campaigns Directory

```bash
campaign_validator --all -d /path/to/campaigns
```

---

## Part 4: Automated Validation

### CI/CD Integration

**GitHub Actions** (`.github/workflows/validate.yml`):

```yaml
name: Validate Campaigns

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Build validator
        run: cargo build --release --bin campaign_validator
      - name: Validate campaigns
        run: ./target/release/campaign_validator --all campaigns/
```

**GitLab CI** (`.gitlab-ci.yml`):

```yaml
validate-campaigns:
  stage: test
  script:
    - cargo build --release --bin campaign_validator
    - ./target/release/campaign_validator --all campaigns/
```

### Pre-commit Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
echo "Validating campaign..."
campaign_validator campaigns/my_campaign || exit 1
echo "Validation passed!"
```

Make it executable:

```bash
chmod +x .git/hooks/pre-commit
```

### JSON Output for Scripting

Get machine-readable output:

```bash
campaign_validator --json campaigns/my_campaign > validation_report.json
```

**Output**:

```json
{
  "is_valid": true,
  "errors": [],
  "warnings": [],
  "error_count": 0,
  "warning_count": 0
}
```

**Parse with jq**:

```bash
# Check if valid
campaign_validator --json campaigns/my_campaign | jq '.is_valid'

# Get error count
campaign_validator --json campaigns/my_campaign | jq '.error_count'

# List all errors
campaign_validator --json campaigns/my_campaign | jq '.errors[]'
```

---

## Part 5: Campaign Structure Best Practices

### Recommended Directory Layout

```
my_campaign/
├── campaign.ron          # Required: Campaign metadata
├── README.md            # Required: Campaign documentation
├── CHANGELOG.md         # Recommended: Version history
├── LICENSE             # Recommended: License terms
├── data/               # Required: Game content
│   ├── classes.ron
│   ├── races.ron
│   ├── items.ron
│   ├── spells.ron
│   ├── monsters.ron
│   ├── quests.ron
│   ├── dialogues.ron
│   └── maps/
│       ├── map001.ron
│       ├── map002.ron
│       └── ...
└── assets/             # Optional: Custom assets
    ├── tilesets/
    ├── music/
    ├── sounds/
    └── images/
```

### Configuration Guidelines

**Starting Resources**:

- `starting_gold: 100-500` (depending on difficulty)
- `starting_food: 50-100`
- `starting_level: 1` (standard starting level)

**Party Limits**:

- `max_party_size: 6` (standard party size)
- `max_roster_size: 20` (enough for variety)

**Level Range**:

- `starting_level: 1`
- `max_level: 20` (full progression arc)

**Difficulty Settings**:

- `Easy`: For casual players
- `Normal`: Standard difficulty (recommended)
- `Hard`: For experienced players
- `Brutal`: Extreme challenge

### Version Management

Use semantic versioning:

- **1.0.0**: Initial release
- **1.1.0**: New content (maps, quests)
- **1.0.1**: Bug fixes, balance tweaks
- **2.0.0**: Major changes, incompatible with v1.x saves

Update `engine_version` to match minimum required engine:

```ron
engine_version: "0.1.0",
```

---

## Part 6: Testing Your Campaign

### Manual Testing Checklist

- [ ] Campaign loads without errors
- [ ] Starting location is accessible
- [ ] All maps are reachable
- [ ] Quests are completable
- [ ] NPCs have working dialogues
- [ ] Items have correct stats
- [ ] Monsters are balanced
- [ ] All quest objectives are achievable
- [ ] Rewards are appropriate
- [ ] No game-breaking bugs

### Validation Workflow

1. **After Each Content Addition**: Quick validation

   ```bash
   campaign_validator campaigns/my_campaign
   ```

2. **Before Committing**: Full validation

   ```bash
   campaign_validator -v campaigns/my_campaign
   ```

3. **Before Release**: All campaigns
   ```bash
   campaign_validator --all
   ```

### Debug Validation Issues

**Errors Only Mode** (hide warnings):

```bash
campaign_validator -e campaigns/my_campaign
```

**Focus on Specific Issues**:

1. Run validation
2. Fix first error
3. Re-run validation
4. Repeat until no errors

---

## Part 7: Distribution Preparation

### Pre-Distribution Checklist

- [ ] Campaign passes validation with no errors
- [ ] README.md is complete and accurate
- [ ] Version number is correct
- [ ] Author information is correct
- [ ] License file is included (if applicable)
- [ ] Campaign has been playtested
- [ ] All content is original or properly licensed
- [ ] File sizes are reasonable
- [ ] No development/temporary files included

### Files to Exclude

Don't include these in distribution:

- `.git/` directory
- `*.swp`, `*.tmp`, `*~` temporary files
- `.DS_Store` (macOS)
- `Thumbs.db` (Windows)
- Development notes
- Unfinished content

### Create Distribution Package

```bash
# Create a clean copy
cp -r campaigns/my_campaign campaigns/my_campaign_release

# Remove development files
cd campaigns/my_campaign_release
rm -rf .git
find . -name "*.tmp" -delete
find . -name "*~" -delete

# Create archive
cd ..
tar -czf my_campaign_v1.0.0.tar.gz my_campaign_release/
```

---

## Part 8: Troubleshooting

### Campaign Won't Load

**Check**: campaign.ron syntax

```bash
# Test RON parsing
ron-validate campaigns/my_campaign/campaign.ron
```

**Check**: File permissions

```bash
ls -la campaigns/my_campaign/
```

### Content Loading Fails

**Check**: Data file syntax

```bash
# Validate each RON file
ron-validate campaigns/my_campaign/data/*.ron
```

**Check**: File paths in campaign.ron match actual files

### Validation Takes Too Long

**Large Campaigns**: Use errors-only mode

```bash
campaign_validator -e campaigns/my_campaign
```

**Batch Validation**: Validate changed campaigns only

### False Positive Errors

**Report Issues**: If validator reports errors for valid content, report to Antares team

**Workaround**: Document known validator limitations in README

---

## Additional Resources

- **Example Campaign**: `campaigns/example/` - Working template
- **Architecture**: `docs/explanation/sdk_and_campaign_architecture.md` - Campaign structure specification
- **API Reference**: Run `cargo doc --open` and see `antares::sdk::campaign_loader`

---

## Quick Reference

### Common Commands

```bash
# Validate single campaign
campaign_validator campaigns/my_campaign

# Validate all campaigns
campaign_validator --all

# Verbose output
campaign_validator -v campaigns/my_campaign

# JSON output
campaign_validator --json campaigns/my_campaign

# Errors only
campaign_validator -e campaigns/my_campaign

# Custom campaigns directory
campaign_validator --all -d /path/to/campaigns
```

### Exit Codes

- **0**: Campaign is valid (no errors)
- **1**: Campaign is invalid (has errors)

### Required Files

- `campaign.ron` - Campaign metadata (required)
- `README.md` - Campaign documentation (required)
- `data/` directory - Content files (required)

### Recommended Defaults

- `max_party_size: 6`
- `max_roster_size: 20`
- `starting_level: 1`
- `max_level: 20`
- `difficulty: Normal`
