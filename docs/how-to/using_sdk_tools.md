# Using SDK Tools

**Target Audience**: Campaign creators, content designers
**Difficulty**: Beginner to Intermediate

This guide covers all the command-line tools in the Antares SDK for creating and managing campaign content.

---

## Table of Contents

1. [Overview](#overview)
2. [Building the Tools](#building-the-tools)
3. [Class Editor](#class-editor)
4. [Race Editor](#race-editor)
5. [Item Editor](#item-editor)
6. [Map Builder](#map-builder)
7. [Campaign Validator](#campaign-validator)
8. [Common Workflows](#common-workflows)
9. [Troubleshooting](#troubleshooting)

---

## Overview

The Antares SDK provides five command-line tools for campaign creation:

| Tool | Purpose | Input/Output |
|------|---------|--------------|
| `class_editor` | Create and edit character classes | `data/classes.ron` |
| `race_editor` | Create and edit playable races | `data/races.ron` |
| `item_editor` | Create and edit items | `data/items.ron` |
| `map_builder` | Create and edit maps | `data/maps/*.ron` |
| `campaign_validator` | Validate and package campaigns | Campaign directory |

All tools work with RON (Rusty Object Notation) format files.

---

## Building the Tools

### Build All Tools

```bash
cargo build --release --bins
```

Binaries will be in `target/release/`.

### Build Individual Tools

```bash
cargo build --release --bin class_editor
cargo build --release --bin race_editor
cargo build --release --bin item_editor
cargo build --release --bin map_builder
cargo build --release --bin campaign_validator
```

### Add to PATH (Optional)

```bash
# Linux/Mac
export PATH=$PATH:$(pwd)/target/release

# Or copy to system location
sudo cp target/release/{class_editor,race_editor,item_editor,map_builder,campaign_validator} /usr/local/bin/
```

---

## Class Editor

**Purpose**: Create and edit character classes for your campaign.

### Basic Usage

```bash
class_editor data/classes.ron
```

### Creating a New Classes File

```bash
class_editor data/classes.ron
# File doesn't exist - creates new file
```

### Editing Existing Classes

```bash
class_editor campaigns/my_campaign/data/classes.ron
```

### Interface

```
Class Editor - classes.ron

Classes:
  [1] Knight (HP: d10, Caster: No)
  [2] Mage (HP: d6, Caster: Yes, School: Elemental)

Commands:
  [a]dd    - Add new class
  [e]dit   - Edit class by ID
  [d]elete - Delete class by ID
  [l]ist   - List all classes
  [p]review - Preview class definition
  [s]ave   - Save and exit
  [q]uit   - Quit without saving

>
```

### Adding a Class

1. Press `a` to add
2. Enter class details:
   - **ID**: Unique number (e.g., 1, 2, 3)
   - **Name**: Class name (e.g., "Knight", "Paladin")
   - **HP Die**: Hit points per level (6, 8, 10, 12)
   - **Pure Caster**: Yes/No (gets spells every level?)
   - **Spell School**: None, Elemental, Divine, Arcane
   - **Spell Stat**: None, Intellect, Personality
   - **Disablement Bit**: Power of 2 for item restrictions (1, 2, 4, 8, etc.)

### Class Design Guidelines

**Fighter Classes**:
- HP Die: d10 or d12
- Pure Caster: No
- Spell School: None

**Hybrid Classes**:
- HP Die: d8
- Pure Caster: No
- Spell School: Divine or Elemental
- Spell Stat: Intellect or Personality

**Pure Caster Classes**:
- HP Die: d6
- Pure Caster: Yes
- Spell School: Elemental, Divine, or Arcane
- Spell Stat: Intellect (Elemental/Arcane) or Personality (Divine)

### Disablement Bits

Each class needs a unique power-of-2 bit:

```
Class 1: bit 1 (value: 1)
Class 2: bit 2 (value: 2)
Class 3: bit 3 (value: 4)
Class 4: bit 4 (value: 8)
Class 5: bit 5 (value: 16)
Class 6: bit 6 (value: 32)
Class 7: bit 7 (value: 64)
Class 8: bit 8 (value: 128)
```

These are used for item restrictions.

### Example Session

```
> a
Add New Class

Enter class ID: 3
Enter class name: Cleric
Enter HP die (6, 8, 10, or 12): 8
Is this a pure caster? (y/n): n
Enter spell school (None/Elemental/Divine/Arcane): Divine
Enter spell stat (None/Intellect/Personality): Personality
Enter disablement bit (power of 2): 4

Class added successfully!

> s
Saving to data/classes.ron...
Saved successfully!
```

---

## Race Editor

**Purpose**: Create and edit playable races for your campaign.

### Basic Usage

```bash
race_editor data/races.ron
```

### Interface

```
Race Editor - races.ron

Races:
  [1] Human (Modifiers: Balanced)
  [2] Elf (Modifiers: INT +2, END -1)

Commands:
  [a]dd    - Add new race
  [e]dit   - Edit race by ID
  [d]elete - Delete race by ID
  [l]ist   - List all races
  [p]review - Preview race definition
  [s]ave   - Save and exit
  [q]uit   - Quit without saving

>
```

### Adding a Race

1. Press `a` to add
2. Enter race details:
   - **ID**: Unique number
   - **Name**: Race name (e.g., "Human", "Elf", "Dwarf")
   - **Stat Modifiers**: Adjustments to base stats (-3 to +3 typical)
     - Might, Intellect, Personality, Endurance, Speed, Accuracy, Luck
   - **Resistances**: Optional (Fire, Cold, Poison, etc.)
   - **Disablement Bit**: Power of 2 for item restrictions

### Race Balance Guidelines

**Total Modifier Budget**: +2 to +4 total points

**Balanced Race** (Human):
```
All modifiers: 0
```

**Specialized Race** (Elf):
```
Intellect: +2
Speed: +1
Endurance: -1
Total: +2
```

**Extreme Specialization** (Dwarf):
```
Endurance: +3
Might: +1
Speed: -2
Total: +2
```

### Example Session

```
> a
Add New Race

Enter race ID: 3
Enter race name: Dwarf

Stat Modifiers:
Enter Might modifier (-3 to +3): 1
Enter Intellect modifier: 0
Enter Personality modifier: 0
Enter Endurance modifier: 3
Enter Speed modifier: -2
Enter Accuracy modifier: 0
Enter Luck modifier: 0

Enter disablement bit (power of 2): 4

Race added successfully!

> s
Saving to data/races.ron...
Saved successfully!
```

---

## Item Editor

**Purpose**: Create and edit items, weapons, armor, and consumables.

### Basic Usage

```bash
item_editor data/items.ron
```

### Interface

```
Item Editor - items.ron

Items:
  [1] Longsword (Weapon, 1d8 damage)
  [2] Plate Mail (Armor, AC +8)
  [3] Healing Potion (Consumable)

Commands:
  [a]dd    - Add new item
  [e]dit   - Edit item (delete and re-add)
  [d]elete - Delete item by ID
  [l]ist   - List all items
  [p]review - Preview item definition
  [s]ave   - Save and exit
  [q]uit   - Quit without saving

>
```

### Adding an Item

1. Press `a` to add
2. Choose item type:
   - **Weapon**: Deals damage in combat
   - **Armor**: Provides AC bonus
   - **Accessory**: Rings, amulets, stat bonuses
   - **Consumable**: Single-use or limited charges
   - **Ammo**: Arrows, bolts
   - **Quest**: Quest-specific items

3. Enter item details based on type

### Weapon Items

**Fields**:
- ID, Name, Value, Weight
- Damage dice (e.g., 1d6, 2d8)
- Damage type (Physical, Fire, Cold, Lightning, Poison, Magical)
- Attack bonus (-5 to +5)
- Critical chance (0-100%)
- Class restrictions (disablement bitfield)
- Bonuses (optional)
- Cursed (yes/no)

**Example**:
```
Item ID: 10
Name: Flaming Longsword
Value: 500
Weight: 4
Damage dice: 1d8
Damage type: Fire
Attack bonus: 2
Crit chance: 10
Usable by: All classes
Bonuses: None
Cursed: No
```

### Armor Items

**Fields**:
- ID, Name, Value, Weight
- AC bonus (1-10 typical)
- Armor type (Light, Medium, Heavy)
- Dexterity cap (optional, for heavy armor)
- Class restrictions
- Bonuses (optional)
- Cursed (yes/no)

**Example**:
```
Item ID: 20
Name: Plate Mail
Value: 1000
Weight: 40
AC bonus: 8
Armor type: Heavy
Dexterity cap: 2
Usable by: Knights only (bit 1)
```

### Consumable Items

**Fields**:
- ID, Name, Value, Weight
- Effect type (HealHp, RestoreSp, CureCondition, Buff)
- Effect parameters (e.g., 2d8 for healing)
- Number of uses (1 = potion, 5 = salve)

**Example**:
```
Item ID: 30
Name: Healing Potion
Value: 50
Weight: 1
Effect: HealHp (2d8)
Uses: 1
```

### Accessory Items

**Fields**:
- ID, Name, Value, Weight
- Slot (Ring, Amulet, Held, Worn)
- Charges (optional, for spell items)
- Spell ID (optional, for wands/staves)
- Bonuses (stat modifiers)

**Example**:
```
Item ID: 40
Name: Ring of Protection
Value: 300
Weight: 0
Slot: Ring
Bonuses: AC +2, Luck +1
```

### Class Restrictions (Disablement)

Use the disablement bitfield to restrict items by class:

```
0   = All classes can use
1   = Class 1 only
2   = Class 2 only
3   = Class 1 OR Class 2 (1 + 2)
4   = Class 3 only
5   = Class 1 OR Class 3 (1 + 4)
7   = Classes 1, 2, 3 (1 + 2 + 4)
255 = No classes (quest item)
```

### Item Bonuses

Items can provide stat bonuses:

**Bonus Types**:
- **Constant**: Permanent while equipped (e.g., +3 Might)
- **Temporary**: Duration-based (e.g., +5 Speed for 10 turns)

**Attributes**:
- Might, Intellect, Personality, Endurance, Speed, Accuracy, Luck
- ArmorClass, AttackBonus, SpellPower

**Example**:
```
Bonuses:
  - Might: Constant(3)
  - ArmorClass: Constant(2)
  - Speed: Temporary(5, 10)  # +5 for 10 turns
```

---

## Map Builder

**Purpose**: Create and edit game maps.

### Basic Usage

```bash
map_builder data/maps/my_map.ron
```

See `docs/how-to/using_map_builder.md` for detailed map builder guide.

### Quick Reference

**Commands**:
- Create new map with dimensions
- Set tiles (Floor, Wall, Door, Water, Forest, etc.)
- Add events (Combat, Treasure, Text, Teleport)
- Place NPCs
- Define exits to other maps
- Validate map structure

**Map Requirements**:
- Width and height (10×10 to 200×200)
- Tile array (width × height tiles)
- Starting position
- Environment (Outdoor, Indoor, Underground)

---

## Campaign Validator

**Purpose**: Validate campaign content and package for distribution.

### Basic Usage

**Validate Campaign**:
```bash
campaign_validator campaigns/my_campaign
```

**Validate All Campaigns**:
```bash
campaign_validator --all
```

**Package Campaign**:
```bash
campaign_validator --package campaigns/my_campaign output.tar.gz
```

### Validation Output

```
Loading campaign from: campaigns/my_campaign
✓ Loaded campaign: My Campaign v1.0.0

Content Summary:
  Classes: 5
  Races: 4
  Items: 120
  Monsters: 45
  Spells: 30
  Maps: 15

Running validation...
✓ No errors found

Campaign is valid!
```

### Common Validation Errors

**MissingItem**:
```
Error: MissingItem { context: "treasure_event", item_id: 42 }
```
Fix: Add item ID 42 to `items.ron` or change the event to use a valid ID.

**MissingMonster**:
```
Error: MissingMonster { map: "dungeon.ron", monster_id: 10 }
```
Fix: Add monster ID 10 to `monsters.ron`.

**DisconnectedMap**:
```
Error: DisconnectedMap { map_id: 5 }
```
Fix: Add an exit connecting map 5 to another map reachable from the starting map.

**DuplicateId**:
```
Error: DuplicateId { entity_type: "item", id: 15 }
```
Fix: Change one of the duplicate IDs to a unique value.

### Packaging

The packager creates a `.tar.gz` archive containing:
- Campaign metadata
- All content files
- README and documentation
- SHA256 checksums

**Package Structure**:
```
my_campaign_v1.0.tar.gz
├── campaign.ron
├── README.md
├── data/
│   ├── classes.ron
│   ├── races.ron
│   ├── items.ron
│   ├── monsters.ron
│   ├── spells.ron
│   └── maps/
│       └── *.ron
└── MANIFEST.json
```

---

## Common Workflows

### Workflow 1: Create a New Campaign from Scratch

```bash
# 1. Create directory structure
mkdir -p campaigns/new_campaign/data/maps

# 2. Create classes
class_editor campaigns/new_campaign/data/classes.ron
# Add 3-5 classes, save

# 3. Create races
race_editor campaigns/new_campaign/data/races.ron
# Add 2-4 races, save

# 4. Create items
item_editor campaigns/new_campaign/data/items.ron
# Add weapons, armor, consumables, save

# 5. Create monsters (manual - use text editor)
# Edit campaigns/new_campaign/data/monsters.ron

# 6. Create spells (manual - use text editor)
# Edit campaigns/new_campaign/data/spells.ron

# 7. Create maps
map_builder campaigns/new_campaign/data/maps/town.ron
map_builder campaigns/new_campaign/data/maps/dungeon.ron
# Design maps, save

# 8. Create campaign metadata (manual)
# Edit campaigns/new_campaign/campaign.ron

# 9. Validate
campaign_validator campaigns/new_campaign

# 10. Package for distribution
campaign_validator --package campaigns/new_campaign new_campaign_v1.0.tar.gz
```

### Workflow 2: Add Content to Existing Campaign

```bash
# 1. Add new items
item_editor campaigns/existing/data/items.ron
# Add items, save

# 2. Validate changes
campaign_validator campaigns/existing

# 3. Test in game
# (Launch Antares and playtest)
```

### Workflow 3: Balance Pass

```bash
# 1. Edit items for balance
item_editor campaigns/my_campaign/data/items.ron
# Adjust damage, value, etc.

# 2. Edit monsters for balance (manual)
# Adjust HP, AC, XP values

# 3. Validate
campaign_validator campaigns/my_campaign

# 4. Playtest and iterate
```

### Workflow 4: Fix Validation Errors

```bash
# 1. Run validator
campaign_validator campaigns/broken_campaign

# 2. Note errors
# Example: "MissingItem { item_id: 42 }"

# 3. Fix errors
item_editor campaigns/broken_campaign/data/items.ron
# Add missing item ID 42

# 4. Re-validate
campaign_validator campaigns/broken_campaign

# 5. Repeat until clean
```

---

## Troubleshooting

### Tool Won't Start

**Problem**: `bash: class_editor: command not found`

**Solution**: Build the tool first:
```bash
cargo build --release --bin class_editor
./target/release/class_editor
```

### File Parse Errors

**Problem**: `ParseError: expected ','`

**Solution**: RON syntax error in file. Check for:
- Missing commas between fields
- Unclosed parentheses or braces
- Typos in field names

Use the validator to identify the exact location.

### Changes Not Saving

**Problem**: Edits don't appear in game

**Solution**:
1. Ensure you pressed `s` to save in the editor
2. Check the correct file path was used
3. Verify file permissions (read/write)
4. Restart the game to reload content

### Validation Fails After Editing

**Problem**: "DuplicateId" error after adding content

**Solution**:
1. Check for ID conflicts in the file
2. Use unique IDs for each entity
3. Use consistent ID numbering scheme (e.g., items 1-100, armor 101-200)

### Item Not Appearing in Game

**Problem**: Item defined but not showing

**Solution**:
1. Verify item ID matches between `items.ron` and map events
2. Check item is marked `identified: true`
3. Ensure disablement bits allow your class to use it
4. Validate campaign to check for errors

---

## Tips and Best Practices

### ID Management

- **Use Ranges**: Weapons 1-100, Armor 101-200, Consumables 201-300
- **Document IDs**: Keep a spreadsheet or comments
- **Leave Gaps**: Use IDs 1, 5, 10, 15 to allow insertions later

### Incremental Development

- **Start Small**: Create 1-2 of each content type first
- **Validate Early**: Run validator after each major change
- **Playtest Often**: Test in-game frequently
- **Iterate**: Refine based on feedback

### Version Control

- **Use Git**: Track changes to your campaign
- **Tag Releases**: `git tag v1.0.0`
- **Branch for Features**: Separate branches for major changes

### Backup

- **Regular Backups**: Copy campaign directory before major changes
- **Use Validator**: Catches issues before they break your campaign

---

## Advanced Usage

### Scripting with Tools

You can automate content creation with shell scripts:

```bash
#!/bin/bash
# Create 10 similar items programmatically

for i in {1..10}; do
  cat >> data/items.ron << EOF
    $i: (
        id: $i,
        name: "Item $i",
        item_type: Weapon((damage: (1, 6), ...)),
        ...
    ),
EOF
done
```

### Batch Validation

Validate multiple campaigns:

```bash
for campaign in campaigns/*/; do
  echo "Validating $campaign..."
  campaign_validator "$campaign"
done
```

### Custom Templates

Create your own item templates:

```bash
cp data/items.ron data/items_template.ron
# Edit template with placeholder IDs
# Use as starting point for new campaigns
```

---

## See Also

- **SDK API Reference**: `docs/reference/sdk_api.md`
- **Campaign Tutorial**: `docs/tutorials/creating_campaigns.md`
- **Map Builder Guide**: `docs/how-to/using_map_builder.md`
- **Modding Guide**: `docs/explanation/modding_guide.md`

---

**Last Updated**: 2024
**SDK Version**: 0.1.0
