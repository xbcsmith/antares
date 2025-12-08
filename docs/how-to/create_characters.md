<!-- SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# How to Create Character Definitions

This guide explains how to create character definitions for Antares RPG,
including premade player characters, NPC templates, and campaign-specific
characters.

## Overview

Character definitions are data-driven templates stored in RON files that can be
instantiated into runtime `Character` objects. This separation allows:

- Campaign designers to create characters without code changes
- Visual editing through the SDK Campaign Builder
- Reusable templates for NPCs and recruitable characters
- Consistent character creation across the game

## Character Definition Types

### Premade Characters

Premade characters (`is_premade: true`) are fully-configured characters that
players can select when starting a new game. They have:

- Balanced stats appropriate for their race and class
- Starting equipment and items
- Backstory descriptions
- Portrait assignments

### Template Characters

Template characters (`is_premade: false`) are used as:

- NPCs that can join the party during gameplay
- Base templates for procedurally generated characters
- Test characters for development

## File Locations

### Core Game Characters

Edit the core characters file:

```text
data/characters.ron
```

These are available in all campaigns.

### Campaign-Specific Characters

Create characters for a specific campaign:

```text
campaigns/<campaign_name>/data/characters.ron
```

Campaign characters are only loaded when that campaign is active.

## Using the Campaign Builder

The SDK Campaign Builder provides a visual editor for character definitions.

### Starting the Editor

```bash
cargo run --bin campaign_builder
```

Navigate to the **Characters** tab to manage character definitions.

### Editor Features

The Characters tab provides:

- **List View**: Browse all character definitions with search and filters
- **Add Mode**: Create new character definitions
- **Edit Mode**: Modify existing definitions
- **Filter Options**: Filter by race, class, alignment, or premade status

### Creating a Character in the Editor

1. Click **Add Character**
2. Fill in basic information:
   - **ID**: Unique identifier (e.g., `pregen_human_knight`)
   - **Name**: Display name (e.g., `Sir Galahad`)
   - **Description**: Character backstory
3. Select race and class from dropdowns
4. Choose sex and alignment
5. Set base stats using the stat editor
6. Configure starting resources (gold, gems, food)
7. Select starting items and equipment
8. Set portrait ID
9. Mark as premade if this is a player-selectable character
10. Click **Save**

## Manual RON File Editing

You can also edit character definitions directly in RON files.

### Basic Character Template

```ron
(
    id: "my_character_id",
    name: "Character Name",
    race_id: "human",
    class_id: "knight",
    sex: Male,
    alignment: Good,
    base_stats: (
        might: 12,
        intellect: 10,
        personality: 10,
        endurance: 12,
        speed: 10,
        accuracy: 10,
        luck: 10,
    ),
    portrait_id: 1,
    starting_gold: 100,
    starting_gems: 0,
    starting_food: 10,
    starting_items: [],
    starting_equipment: (
        weapon: None,
        armor: None,
        shield: None,
        helmet: None,
        boots: None,
        accessory1: None,
        accessory2: None,
    ),
    description: "A brief character backstory.",
    is_premade: true,
)
```

### Complete Example File

```ron
// characters.ron - Character definitions
[
    (
        id: "pregen_human_knight",
        name: "Sir Galahad",
        race_id: "human",
        class_id: "knight",
        sex: Male,
        alignment: Good,
        base_stats: (
            might: 14,
            intellect: 8,
            personality: 10,
            endurance: 14,
            speed: 10,
            accuracy: 12,
            luck: 8,
        ),
        portrait_id: 1,
        starting_gold: 100,
        starting_gems: 0,
        starting_food: 10,
        starting_items: [1, 20],
        starting_equipment: (
            weapon: Some(1),
            armor: Some(20),
            shield: None,
            helmet: None,
            boots: None,
            accessory1: None,
            accessory2: None,
        ),
        description: "A noble knight from the northern kingdoms, seeking glory and honor in battle.",
        is_premade: true,
    ),
    (
        id: "pregen_elf_sorcerer",
        name: "Elindra",
        race_id: "elf",
        class_id: "sorcerer",
        sex: Female,
        alignment: Neutral,
        base_stats: (
            might: 8,
            intellect: 16,
            personality: 12,
            endurance: 8,
            speed: 12,
            accuracy: 10,
            luck: 10,
        ),
        portrait_id: 5,
        starting_gold: 50,
        starting_gems: 5,
        starting_food: 10,
        starting_items: [1],
        starting_equipment: (
            weapon: Some(1),
            armor: None,
            shield: None,
            helmet: None,
            boots: None,
            accessory1: None,
            accessory2: None,
        ),
        description: "An elven mage from the ancient forests, skilled in the arcane arts.",
        is_premade: true,
    ),
]
```

## Field Reference

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | String | Unique identifier for the character definition |
| `name` | String | Character display name |
| `race_id` | String | Reference to races.ron (e.g., `"human"`, `"elf"`) |
| `class_id` | String | Reference to classes.ron (e.g., `"knight"`, `"sorcerer"`) |
| `sex` | Enum | `Male`, `Female`, or `Other` |
| `alignment` | Enum | `Good`, `Neutral`, or `Evil` |
| `base_stats` | Struct | Starting attribute values |

### Optional Fields (with defaults)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `portrait_id` | u8 | 0 | Portrait/avatar identifier |
| `starting_gold` | u32 | 0 | Initial gold amount |
| `starting_gems` | u32 | 0 | Initial gems amount |
| `starting_food` | u32 | 10 | Initial food rations |
| `starting_items` | Vec | [] | Item IDs to add to inventory |
| `starting_equipment` | Struct | Empty | Items to equip in slots |
| `description` | String | "" | Character backstory/bio |
| `is_premade` | bool | false | Player-selectable character |

### Base Stats Structure

```ron
base_stats: (
    might: 10,       // Physical strength (3-25)
    intellect: 10,   // Mental acuity, affects Sorcerer SP (3-25)
    personality: 10, // Charisma, affects Cleric SP (3-25)
    endurance: 10,   // Constitution, affects HP (3-25)
    speed: 10,       // Reaction time, combat order (3-25)
    accuracy: 10,    // Precision, to-hit bonus (3-25)
    luck: 10,        // Fortune, critical hits (3-25)
)
```

### Starting Equipment Structure

```ron
starting_equipment: (
    weapon: Some(1),     // ItemId or None
    armor: Some(20),     // ItemId or None
    shield: None,        // ItemId or None
    helmet: None,        // ItemId or None
    boots: None,         // ItemId or None
    accessory1: None,    // ItemId or None
    accessory2: None,    // ItemId or None
)
```

## Balancing Guidelines

### Stat Allocation by Class

| Class | Primary Stats | Secondary Stats |
|-------|--------------|-----------------|
| Knight | Might, Endurance | Accuracy, Speed |
| Paladin | Might, Personality | Endurance, Luck |
| Archer | Accuracy, Speed | Intellect, Luck |
| Cleric | Personality, Endurance | Luck, Intellect |
| Sorcerer | Intellect, Luck | Personality, Speed |
| Robber | Speed, Accuracy | Luck, Might |

### Recommended Stat Ranges

- **Primary stats**: 14-16
- **Secondary stats**: 10-12
- **Tertiary stats**: 8-10
- **Total points**: Approximately 76 for balanced premade characters

### Race Modifiers

Remember that race modifiers are applied during instantiation:

| Race | Modifiers |
|------|-----------|
| Human | None |
| Elf | +1 Intellect, +1 Accuracy, -1 Might, -1 Endurance |
| Dwarf | +1 Endurance, +1 Luck, -1 Intellect, -1 Speed |
| Gnome | +1 Luck, +1 Speed, -1 Might, -1 Personality |
| Half-Elf | +1 Intellect, -1 Personality |
| Half-Orc | +2 Might, +1 Endurance, -2 Intellect, -1 Personality |

### Starting Equipment Tiers

**Tier 1 (Beginner)**:

- Basic weapon (Club, Dagger)
- No armor or basic leather
- 50-100 gold

**Tier 2 (Standard Premade)**:

- Class-appropriate weapon
- Appropriate armor
- 100-200 gold, 0-5 gems

**Tier 3 (Experienced)**:

- Enhanced weapon (+1)
- Quality armor
- 200-500 gold, 5-10 gems

## Instantiation Process

When a character definition is instantiated into a runtime Character:

1. **Validation**: Race, class, and item IDs are verified against databases
2. **Stat Modifiers**: Race modifiers are applied to base_stats
3. **HP Calculation**: Max roll of class HP die + (endurance - 10) / 2, minimum 1
4. **SP Calculation**: Based on class spell_stat:
   - Sorcerer/Archer: (Intellect - 10), minimum 0
   - Cleric/Paladin: (Personality - 10) / 2, minimum 0
   - Knight/Robber: 0
5. **Resistances**: Race resistances are applied
6. **Inventory**: Starting items are added
7. **Equipment**: Starting equipment is placed in slots

### Example HP Calculation

A Human Knight with Endurance 14:

- Class HP die: d10 (max = 10)
- Endurance modifier: (14 - 10) / 2 = 2
- Starting HP: 10 + 2 = 12

### Example SP Calculation

An Elf Sorcerer with Intellect 16 (after +1 elf bonus = 17):

- Spell stat: Intellect
- Starting SP: 17 - 10 = 7

## Common Patterns

### Premade Party Set

Create a balanced party of 6 characters covering different roles:

1. **Tank**: Human Knight (high Might, Endurance)
2. **Healer**: Human Cleric (high Personality, Endurance)
3. **DPS Melee**: Half-Orc Knight (high Might)
4. **DPS Ranged**: Elf Archer (high Accuracy, Speed)
5. **Caster**: Elf Sorcerer (high Intellect)
6. **Utility**: Gnome Robber (high Speed, Luck)

### NPC Recruit Template

```ron
(
    id: "npc_rescued_wizard",
    name: "Captured Mage",
    race_id: "human",
    class_id: "sorcerer",
    sex: Male,
    alignment: Good,
    base_stats: (
        might: 6,
        intellect: 14,
        personality: 10,
        endurance: 8,
        speed: 10,
        accuracy: 8,
        luck: 10,
    ),
    portrait_id: 12,
    starting_gold: 0,
    starting_gems: 0,
    starting_food: 0,
    starting_items: [],
    starting_equipment: (
        weapon: None,
        armor: None,
        shield: None,
        helmet: None,
        boots: None,
        accessory1: None,
        accessory2: None,
    ),
    description: "A wizard captured by goblins, grateful to be rescued.",
    is_premade: false,
)
```

## Validation

### Using Campaign Validator

Validate character definitions:

```bash
cargo run --bin campaign_validator -- campaigns/my_campaign
```

The validator checks:

- Unique character IDs
- Valid race_id references
- Valid class_id references
- Valid item ID references (starting_items and starting_equipment)

### Common Validation Errors

**Invalid race_id**:

```text
Error: Invalid race ID 'elven' for character 'my_elf'
```

Fix: Use correct race ID from races.ron (e.g., `"elf"`, not `"elven"`)

**Invalid item ID**:

```text
Error: Invalid item ID '999' for character 'my_knight'
```

Fix: Verify item exists in items.ron

**Duplicate ID**:

```text
Error: Duplicate character ID: 'pregen_human_knight'
```

Fix: Use unique IDs for each character definition

## Integration with Game Code

### Loading Characters

```rust
use antares::domain::character_definition::CharacterDatabase;
use antares::domain::races::RaceDatabase;
use antares::domain::classes::ClassDatabase;
use antares::domain::items::ItemDatabase;

// Load databases
let races = RaceDatabase::load_from_file("data/races.ron")?;
let classes = ClassDatabase::load_from_file("data/classes.ron")?;
let items = ItemDatabase::load_from_file("data/items.ron")?;
let characters = CharacterDatabase::load_from_file("data/characters.ron")?;

// Get premade characters for selection screen
for char_def in characters.premade_characters() {
    println!("{}: {} ({} {})",
        char_def.id, char_def.name, char_def.race_id, char_def.class_id);
}
```

### Instantiating a Character

```rust
// Player selects a premade character
let knight_def = characters.get_character("pregen_human_knight")
    .expect("Character not found");

// Create runtime character
let knight = knight_def.instantiate(&races, &classes, &items)?;

// Character is ready for gameplay
assert_eq!(knight.name, "Sir Galahad");
assert_eq!(knight.class, Class::Knight);
```

## Tips and Best Practices

### ID Naming Conventions

- Use snake_case for IDs
- Prefix premade characters with `pregen_`
- Prefix NPC templates with `npc_`
- Include race and class in ID for clarity

Examples:

- `pregen_human_knight`
- `pregen_elf_sorcerer`
- `npc_tavern_keeper`
- `npc_quest_wizard`

### Description Writing

Write descriptions that:

- Are 1-3 sentences long
- Mention character origin or background
- Hint at personality or motivation
- Avoid spoilers for campaign content

### Testing Characters

After creating characters:

1. Run the campaign validator
2. Load in Campaign Builder to preview stats
3. Test instantiation in-game
4. Verify equipment appears correctly
5. Check HP/SP calculations match expectations

## See Also

- [Architecture Reference](../reference/architecture.md) - CharacterDefinition data structures
- [Using Item Editor](using_item_editor.md) - Creating items for starting equipment
- [Creating and Validating Campaigns](creating_and_validating_campaigns.md) - Campaign workflow
- [SDK Implementation Plan](../explanation/sdk_implementation_plan.md) - SDK architecture
