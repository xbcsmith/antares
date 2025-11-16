# Getting Started: Creating Your First Campaign

Welcome to Antares campaign creation! This tutorial will walk you through creating your first complete campaign from scratch, teaching you the basics of campaign structure, content creation, and validation.

**Time Required**: 30-45 minutes

---

## What You'll Learn

- How to set up a campaign directory structure
- Creating campaign metadata (campaign.ron)
- Adding basic content (items, monsters, maps)
- Creating a simple quest
- Validating your campaign
- Packaging for distribution

---

## Prerequisites

Before starting, ensure you have:

- Antares SDK installed
- A text editor (VS Code, Sublime, vim, etc.)
- Basic understanding of RON format (Rusty Object Notation)
- Terminal/command line access

---

## Part 1: Creating the Campaign Structure

### Step 1: Copy the Example Template

The quickest way to start is by copying the example campaign:

```bash
# Navigate to your Antares directory
cd antares

# Copy the example campaign
cp -r campaigns/example campaigns/my_first_campaign

# Navigate to your new campaign
cd campaigns/my_first_campaign
```

### Step 2: Understand the Directory Structure

Your campaign directory should look like this:

```
my_first_campaign/
├── campaign.ron       # Campaign metadata and configuration
├── README.md         # Campaign documentation
├── data/             # All game content
│   ├── items.ron
│   ├── spells.ron
│   ├── monsters.ron
│   ├── classes.ron
│   ├── races.ron
│   ├── quests.ron
│   ├── dialogues.ron
│   └── maps/         # Map files
└── assets/           # Optional: custom graphics/audio
    ├── tilesets/
    ├── music/
    ├── sounds/
    └── images/
```

**Key Directories**:
- `data/` - Contains all game content in RON format
- `assets/` - Optional custom graphics and audio
- Root level - Metadata and documentation files

---

## Part 2: Configuring Campaign Metadata

### Step 3: Edit campaign.ron

Open `campaign.ron` in your text editor and customize it:

```ron
Campaign(
    // Basic Information
    id: "my_first_campaign",
    name: "My First Adventure",
    version: "1.0.0",
    author: "Your Name",
    description: "A beginner campaign exploring the basics of Antares.",
    engine_version: "0.1.0",
    required_features: [],

    // Game Configuration
    config: CampaignConfig(
        starting_map: 1,
        starting_position: Position(x: 10, y: 10),
        starting_direction: North,
        starting_gold: 150,
        starting_food: 75,
        max_party_size: 6,
        max_roster_size: 20,
        difficulty: Normal,
        permadeath: false,
        allow_multiclassing: false,
        starting_level: 1,
        max_level: 20,
    ),

    // Data Files (use defaults)
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

    // Asset Directories (optional)
    assets: CampaignAssets(
        tilesets: "assets/tilesets",
        music: "assets/music",
        sounds: "assets/sounds",
        images: "assets/images",
    ),
)
```

**Important Fields**:
- `id`: Must match directory name (lowercase, no spaces)
- `name`: Display name shown to players
- `version`: Use semantic versioning (1.0.0)
- `starting_position`: Where players start on the map

### Step 4: Update the README

Edit `README.md` with your campaign information:

```markdown
# My First Adventure

An introductory campaign for learning Antares game creation.

## Description

This campaign introduces players to the basic mechanics of Antares while
providing a fun adventure through dungeons and towns.

## Story

You are a novice adventurer seeking fortune and glory. Your journey begins
in the small town of Millhaven, where rumors speak of treasure in the
nearby ruins...

## Features

- 3 unique maps to explore
- 5 quests with varying objectives
- Custom items and monsters
- Beginner-friendly difficulty

## Requirements

- Antares Engine v0.1.0 or later

## Credits

Created as a learning exercise for Antares campaign development.
```

---

## Part 3: Adding Content

### Step 5: Create a Simple Item

Edit `data/items.ron` to add your first custom item:

```ron
[
    // A basic starting weapon
    Item(
        id: 1,
        name: "Rusty Sword",
        description: "An old sword, but still functional.",
        item_type: Weapon(
            damage: DiceRoll(count: 1, sides: 6, bonus: 0),
            weapon_type: Sword,
        ),
        value: 10,
        weight: 3,
        usable_by: [],  // Empty means all classes can use
        cursed: false,
    ),

    // A basic healing potion
    Item(
        id: 2,
        name: "Healing Potion",
        description: "Restores 20 hit points when consumed.",
        item_type: Consumable(
            effect: Heal(amount: 20),
            charges: 1,
        ),
        value: 50,
        weight: 1,
        usable_by: [],
        cursed: false,
    ),
]
```

**Key Points**:
- Each item needs a unique `id`
- `item_type` determines what the item does
- `value` is the gold cost
- Empty `usable_by` means all classes can use it

### Step 6: Create a Simple Monster

Edit `data/monsters.ron`:

```ron
[
    // A weak starting enemy
    Monster(
        id: 1,
        name: "Goblin",
        description: "A small, green-skinned creature.",
        level: 1,
        hp: DiceRoll(count: 2, sides: 6, bonus: 2),  // 4-14 HP
        armor_class: 12,
        attack_bonus: 2,
        damage: DiceRoll(count: 1, sides: 4, bonus: 1),  // 2-5 damage
        experience: 50,
        loot: [
            (item_id: 1, chance: 0.1),  // 10% chance to drop Rusty Sword
        ],
        special_abilities: [],
    ),

    // A stronger enemy
    Monster(
        id: 2,
        name: "Orc Warrior",
        description: "A fierce orc wielding a crude axe.",
        level: 3,
        hp: DiceRoll(count: 3, sides: 8, bonus: 6),  // 9-30 HP
        armor_class: 14,
        attack_bonus: 4,
        damage: DiceRoll(count: 1, sides: 8, bonus: 2),  // 3-10 damage
        experience: 150,
        loot: [
            (item_id: 1, chance: 0.3),
            (item_id: 2, chance: 0.2),
        ],
        special_abilities: [],
    ),
]
```

### Step 7: Create Your First Map

Create `data/maps/map001.ron` for your starting town:

```ron
Map(
    id: 1,
    name: "Millhaven Town",
    width: 20,
    height: 20,

    // Tile data (20x20 grid)
    tiles: [
        // Row 0
        [Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall,
         Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall],

        // Row 1
        [Wall, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor,
         Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Wall],

        // Rows 2-18: Fill with Floor tiles surrounded by Walls
        // (abbreviated for clarity)

        // Row 19
        [Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall,
         Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall],
    ],

    // Map events
    events: [
        // A treasure chest
        (
            position: Position(x: 5, y: 5),
            event: Treasure(
                loot: [1, 2],  // Rusty Sword and Healing Potion
            ),
        ),

        // An encounter
        (
            position: Position(x: 10, y: 10),
            event: Encounter(
                monster_group: [1, 1],  // Two goblins
            ),
        ),
    ],

    // NPCs
    npcs: [
        Npc(
            id: 1,
            name: "Town Elder",
            position: Position(x: 15, y: 15),
            dialogue_id: 1,
        ),
    ],

    // Exits to other maps
    exits: [],
)
```

**Map Tips**:
- Start with small maps (20x20 is manageable)
- Use Wall for boundaries
- Floor for walkable areas
- Place events strategically
- Test frequently!

### Step 8: Create a Simple Quest

Edit `data/quests.ron`:

```ron
[
    Quest(
        id: 1,
        name: "The Goblin Menace",
        description: "The Town Elder has asked you to deal with the goblin problem.",

        stages: [
            // Stage 1: Accept the quest
            QuestStage(
                stage_number: 1,
                name: "Speak with the Elder",
                description: "Find and talk to the Town Elder.",
                objectives: [
                    TalkToNpc(
                        npc_id: 1,
                        map_id: 1,
                    ),
                ],
                require_all_objectives: true,
            ),

            // Stage 2: Complete the task
            QuestStage(
                stage_number: 2,
                name: "Defeat the Goblins",
                description: "Defeat 5 goblins.",
                objectives: [
                    KillMonsters(
                        monster_id: 1,
                        quantity: 5,
                    ),
                ],
                require_all_objectives: true,
            ),

            // Stage 3: Return for reward
            QuestStage(
                stage_number: 3,
                name: "Return to the Elder",
                description: "Report back to the Town Elder.",
                objectives: [
                    TalkToNpc(
                        npc_id: 1,
                        map_id: 1,
                    ),
                ],
                require_all_objectives: true,
            ),
        ],

        // Quest rewards
        rewards: [
            Experience(200),
            Gold(100),
            Items([(2, 3)]),  // 3 Healing Potions
        ],

        min_level: Some(1),
        max_level: None,
        required_quests: [],
        repeatable: false,
        is_main_quest: true,
        quest_giver_npc: Some(1),
        quest_giver_map: Some(1),
        quest_giver_position: Some(Position(x: 15, y: 15)),
    ),
]
```

### Step 9: Create a Simple Dialogue

Edit `data/dialogues.ron`:

```ron
[
    DialogueTree(
        id: 1,
        name: "Town Elder Greeting",
        root_node: 1,
        speaker_name: Some("Town Elder"),
        repeatable: true,
        associated_quest: Some(1),

        nodes: {
            // Initial greeting
            1: DialogueNode(
                id: 1,
                text: "Greetings, adventurer! Our town is plagued by goblins. Will you help us?",
                choices: [
                    DialogueChoice(
                        text: "I'll help you.",
                        target_node: Some(2),
                        conditions: [],
                        actions: [
                            StartQuest(quest_id: 1),
                        ],
                        ends_dialogue: false,
                    ),
                    DialogueChoice(
                        text: "Tell me more about the goblins.",
                        target_node: Some(3),
                        conditions: [],
                        actions: [],
                        ends_dialogue: false,
                    ),
                    DialogueChoice(
                        text: "Maybe later.",
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

            // Quest accepted
            2: DialogueNode(
                id: 2,
                text: "Thank you! The goblins have been spotted near the old ruins. Good luck!",
                choices: [],
                conditions: [],
                actions: [],
                is_terminal: true,
            ),

            // More information
            3: DialogueNode(
                id: 3,
                text: "The goblins have been raiding our farms. We need someone brave to drive them away.",
                choices: [
                    DialogueChoice(
                        text: "I'll do it.",
                        target_node: Some(2),
                        conditions: [],
                        actions: [
                            StartQuest(quest_id: 1),
                        ],
                        ends_dialogue: false,
                    ),
                    DialogueChoice(
                        text: "I need to think about it.",
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
        },
    ),
]
```

---

## Part 4: Validation and Testing

### Step 10: Validate Your Campaign

Run the campaign validator to check for errors:

```bash
# From the antares directory
campaign_validator campaigns/my_first_campaign
```

**Expected Output**:
```
Campaign: My First Adventure v1.0.0
Author: Your Name
Engine: 0.1.0

[1/5] Validating campaign structure...
[2/5] Loading content database...
  Classes: 0
  Items: 2
  Monsters: 2
  Maps: 1
  Quests: 1
  Dialogues: 1
[3/5] Validating cross-references...
[4/5] Validating quests...
[5/5] Validating dialogues...

✓ Campaign is VALID

Warnings (2):
  1. No classes defined
  2. No spells defined

No errors found!
```

### Step 11: Fix Any Errors

If validation fails, you'll see specific error messages:

**Common Errors**:

1. **"Invalid monster ID: X"**
   - Quest references a monster that doesn't exist
   - Fix: Add the monster or change the quest objective

2. **"Dialogue X: Node Y is orphaned"**
   - Dialogue has an unreachable node
   - Fix: Connect the node or remove it

3. **"starting_level > max_level"**
   - Configuration error in campaign.ron
   - Fix: Ensure starting_level ≤ max_level

### Step 12: Add Classes and Spells (Optional)

To fix the warnings, you can add basic classes and spells, or use the core game's default classes:

**Option 1**: Use core game classes
- Leave `data/classes.ron` and `data/spells.ron` empty
- The game will use default classes and spells

**Option 2**: Copy from core game
```bash
cp ../path/to/core/classes.ron data/classes.ron
cp ../path/to/core/spells.ron data/spells.ron
```

---

## Part 5: Packaging for Distribution

### Step 13: Package Your Campaign

Once validated, package your campaign:

```bash
# Using the campaign packager (if implemented)
cargo run --bin campaign_packager -- package \
    campaigns/my_first_campaign \
    my_first_adventure_v1.0.0.tar.gz
```

**Or manually create an archive**:
```bash
cd campaigns
tar -czf my_first_adventure_v1.0.0.tar.gz my_first_campaign/
```

### Step 14: Test Installation

Test installing your packaged campaign:

```bash
# Extract to a test location
mkdir test_install
cd test_install
tar -xzf ../my_first_adventure_v1.0.0.tar.gz

# Validate the extracted campaign
campaign_validator my_first_campaign
```

---

## Part 6: Next Steps

### Congratulations!

You've created your first Antares campaign! Here's what you learned:

✅ Campaign directory structure
✅ Campaign metadata configuration
✅ Creating items and monsters
✅ Building maps
✅ Designing quests
✅ Writing dialogues
✅ Validation and testing
✅ Packaging for distribution

### Take It Further

**Expand Your Campaign**:
1. Add more maps (dungeons, towns, wilderness)
2. Create additional quests with branching paths
3. Design unique items and equipment
4. Add more monster types
5. Create complex dialogue trees
6. Implement quest chains (prerequisites)

**Learn Advanced Features**:
- Multi-stage dungeons with multiple levels
- Quest rewards that unlock new quests
- Conditional dialogues based on quest state
- Custom difficulty balancing
- Special monster abilities
- Unique item effects

**Resources**:
- `/docs/how-to/creating_and_validating_campaigns.md` - Detailed how-to guide
- `/docs/how-to/using_quest_and_dialogue_tools.md` - Advanced quest/dialogue features
- `/campaigns/example/` - Example campaign template
- `/docs/explanation/sdk_and_campaign_architecture.md` - Architecture details

### Share Your Campaign

Once you're happy with your campaign:

1. Validate one final time
2. Create a release package
3. Write clear installation instructions
4. Share with the community!

---

## Troubleshooting

### Campaign Won't Load

**Problem**: "campaign.ron not found"
- **Solution**: Ensure the file exists in campaign root directory

**Problem**: "RON parsing error"
- **Solution**: Check syntax (commas, brackets, parentheses)
- Use a RON validator or IDE with syntax highlighting

### Validation Errors

**Problem**: "No maps defined"
- **Solution**: Create at least one map in `data/maps/`

**Problem**: "Quest X: Invalid objective"
- **Solution**: Check that referenced monsters/items/maps exist

### Content Not Working

**Problem**: Quest won't start
- **Solution**: Verify dialogue has `StartQuest` action
- Verify quest_giver_npc matches NPC in dialogue

**Problem**: Monster encounters don't work
- **Solution**: Ensure monster IDs in map events match `data/monsters.ron`

---

## Quick Reference

### Essential Files
```
campaign.ron          # Required: Campaign metadata
README.md            # Required: Documentation
data/items.ron       # Optional: Custom items
data/monsters.ron    # Optional: Custom monsters
data/maps/map001.ron # Required: At least one map
data/quests.ron      # Optional: Quests
data/dialogues.ron   # Optional: Dialogues
```

### Common Commands
```bash
# Validate campaign
campaign_validator campaigns/my_campaign

# Validate with verbose output
campaign_validator -v campaigns/my_campaign

# Package campaign
tar -czf my_campaign_v1.0.0.tar.gz my_campaign/
```

### RON Syntax Reminder
```ron
// Single-line comment

/* Multi-line
   comment */

// Structure
StructName(
    field_name: value,
    another_field: 123,
)

// List
[item1, item2, item3]

// Optional values
Some(value)
None
```

---

## Feedback and Help

Having trouble? Check these resources:

1. **Documentation**: `/docs/` directory
2. **Examples**: `/campaigns/example/`
3. **Validation**: Use `campaign_validator -v` for detailed output

Happy campaign creating!
