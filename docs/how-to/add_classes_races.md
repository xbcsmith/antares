# How to Add Custom Classes and Races

This guide explains how to add custom classes and races to Antares campaigns using the data-driven system. No code changes are required - all customization is done through RON data files.

## Overview

Antares uses a data-driven architecture where classes and races are defined in RON (Rusty Object Notation) files. This allows content creators to:

- Add new classes with custom abilities, HP progression, and spell schools
- Add new races with stat modifiers, resistances, and special abilities
- Modify existing class/race definitions for custom campaigns
- Create campaign-specific variations without touching game code

## File Locations

Classes and races are defined in RON files at these locations:

- **Core data**: `data/classes.ron`, `data/races.ron`
- **Campaign-specific**: `campaigns/<name>/data/classes.ron`, `campaigns/<name>/data/races.ron`

Campaign-specific files override or extend the core definitions.

## Adding a New Class

### Step 1: Understand the ClassDefinition Structure

Each class is defined with the following fields:

```ron
(
    id: "warrior",                    // Unique identifier (lowercase, no spaces)
    name: "Warrior",                  // Display name
    description: "A mighty fighter", // Description text
    hp_die: (count: 1, sides: 10, bonus: 2),  // HP gained per level
    spell_school: None,              // None, Some(Cleric), or Some(Sorcerer)
    is_pure_caster: false,           // True if primary role is casting
    spell_stat: None,                // None, Some(Intellect), or Some(Personality)
    disablement_bit_index: 6,        // Unique bit position (0-7)
    special_abilities: [],           // List of special ability IDs
    starting_weapon_id: Some(1),     // Starting weapon item ID
    starting_armor_id: Some(10),     // Starting armor item ID
    starting_items: [20, 21],        // Additional starting item IDs
)
```

### Step 2: Choose a Unique Disablement Bit Index

The `disablement_bit_index` determines which items the class can use. Standard classes use:

| Class    | Bit Index | Mask Value   |
|----------|-----------|--------------|
| Knight   | 0         | 0b0000_0001  |
| Paladin  | 1         | 0b0000_0010  |
| Archer   | 2         | 0b0000_0100  |
| Cleric   | 3         | 0b0000_1000  |
| Sorcerer | 4         | 0b0001_0000  |
| Robber   | 5         | 0b0010_0000  |

Custom classes should use indices 6 or 7, or reuse existing indices if they should share item restrictions with a base class.

### Step 3: Create the Class Definition

Add your class to `classes.ron`:

```ron
[
    // ... existing classes ...
    (
        id: "berserker",
        name: "Berserker",
        description: "A rage-fueled warrior who sacrifices defense for offense",
        hp_die: (count: 1, sides: 12, bonus: 0),
        spell_school: None,
        is_pure_caster: false,
        spell_stat: None,
        disablement_bit_index: 6,
        special_abilities: ["rage", "fearless"],
        starting_weapon_id: Some(5),
        starting_armor_id: None,
        starting_items: [],
    ),
]
```

### Step 4: Update Item Disablements (Optional)

If you want existing items to be usable by your new class, update the item's `disablements` field to include the new class bit:

```ron
// Before: Knight and Paladin only (0b0000_0011)
disablements: (3),

// After: Knight, Paladin, and Berserker (0b0100_0011)
disablements: (67),
```

The mask value is calculated as: `existing_value | (1 << disablement_bit_index)`

## Adding a New Race

### Step 1: Understand the RaceDefinition Structure

Each race is defined with the following fields:

```ron
(
    id: "halfling",                   // Unique identifier (lowercase, no spaces)
    name: "Halfling",                 // Display name
    description: "Small but nimble",  // Description text
    stat_modifiers: (
        might: -2,
        intellect: 0,
        personality: 1,
        endurance: -1,
        speed: 2,
        accuracy: 1,
        luck: 2,
    ),
    resistances: (
        magic: 5,
        fire: 0,
        cold: 0,
        electricity: 0,
        acid: 0,
        fear: 10,
        poison: 5,
        psychic: 0,
    ),
    special_abilities: ["lucky_dodge"],
    age_modifier: 0,
    allowed_classes: ["knight", "archer", "robber", "cleric"],
)
```

### Step 2: Create the Race Definition

Add your race to `races.ron`:

```ron
[
    // ... existing races ...
    (
        id: "halfling",
        name: "Halfling",
        description: "Small folk known for their luck and nimbleness",
        stat_modifiers: (
            might: -2,
            intellect: 0,
            personality: 1,
            endurance: -1,
            speed: 2,
            accuracy: 1,
            luck: 2,
        ),
        resistances: (
            magic: 5,
            fire: 0,
            cold: 0,
            electricity: 0,
            acid: 0,
            fear: 10,
            poison: 5,
            psychic: 0,
        ),
        special_abilities: ["lucky_dodge"],
        age_modifier: -5,
        allowed_classes: ["knight", "archer", "robber", "cleric"],
    ),
]
```

### Step 3: Consider Class Restrictions

The `allowed_classes` field restricts which classes this race can choose. Use an empty list `[]` to allow all classes.

## Using the SDK Campaign Builder

The SDK Campaign Builder provides a graphical interface for managing classes and races:

1. **Launch the Campaign Builder**: `cargo run --bin campaign_builder`
2. **Load your campaign**: File > Open Campaign
3. **Edit Classes**: Navigate to the Classes tab
4. **Edit Races**: Navigate to the Races tab
5. **Save changes**: File > Save Campaign

The Campaign Builder validates your definitions and warns about:

- Duplicate IDs
- Invalid disablement bit indices
- Missing required fields
- Invalid class references in race definitions

## Validation

After creating custom classes or races, validate your campaign:

```bash
cargo run --bin campaign_validator -- campaigns/my_campaign
```

The validator checks:

- RON syntax correctness
- Unique IDs across all definitions
- Valid cross-references (e.g., allowed_classes references valid class IDs)
- Disablement bit conflicts

## Examples

### Example: Monk Class (Hybrid Caster)

```ron
(
    id: "monk",
    name: "Monk",
    description: "A martial artist with minor clerical magic",
    hp_die: (count: 1, sides: 8, bonus: 0),
    spell_school: Some(Cleric),
    is_pure_caster: false,
    spell_stat: Some(Personality),
    disablement_bit_index: 6,
    special_abilities: ["unarmed_combat", "meditation"],
    starting_weapon_id: None,
    starting_armor_id: None,
    starting_items: [],
)
```

### Example: Lizardfolk Race

```ron
(
    id: "lizardfolk",
    name: "Lizardfolk",
    description: "Cold-blooded reptilian humanoids",
    stat_modifiers: (
        might: 2,
        intellect: -1,
        personality: -2,
        endurance: 2,
        speed: 0,
        accuracy: 0,
        luck: -1,
    ),
    resistances: (
        magic: 0,
        fire: -10,
        cold: -20,
        electricity: 0,
        acid: 15,
        fear: 5,
        poison: 20,
        psychic: 0,
    ),
    special_abilities: ["natural_armor", "hold_breath"],
    age_modifier: 10,
    allowed_classes: ["knight", "archer", "robber"],
)
```

## Troubleshooting

### Common Issues

1. **"Duplicate ID" error**: Each class and race must have a unique `id` value.

2. **"Invalid class reference" error**: Check that `allowed_classes` in race definitions only references existing class IDs.

3. **"Disablement bit conflict" warning**: Two classes share the same `disablement_bit_index`. This means they'll have identical item restrictions.

4. **RON parse errors**: Check for:
   - Missing commas between fields
   - Mismatched parentheses
   - Invalid enum values (use `Some(Cleric)` not `Some("Cleric")`)

### Testing Your Definitions

1. Load the campaign in the Campaign Builder
2. Create a test character using the new class/race
3. Verify stat modifiers are applied correctly
4. Check that item restrictions work as expected
5. Test spell casting if the class has a spell school

## Migration Notes

If you're migrating from an older version that used hardcoded Class/Race enums:

1. The `race` field in Character is now `race_id` (String)
2. The `class` field in Character is now `class_id` (String)
3. Use the class/race IDs from your RON files (e.g., "knight", "human")
4. All lookups now go through ClassDatabase and RaceDatabase

See `docs/explanation/implementations.md` for the complete migration history.
