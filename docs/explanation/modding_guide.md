# Antares Modding Guide

**Target Audience**: Campaign creators, modders, content designers  
**Difficulty**: Intermediate to Advanced

This guide explains the concepts, patterns, and best practices for creating content for Antares RPG.

---

## Table of Contents

1. [Introduction](#introduction)
2. [Core Concepts](#core-concepts)
3. [Campaign Architecture](#campaign-architecture)
4. [Content Design Patterns](#content-design-patterns)
5. [Advanced Techniques](#advanced-techniques)
6. [Balancing Guidelines](#balancing-guidelines)
7. [Performance Considerations](#performance-considerations)
8. [Publishing Your Mod](#publishing-your-mod)

---

## Introduction

### What is a Mod?

In Antares, a **mod** (modification) is a custom campaign that extends or replaces the core game content. Mods can include:

- New character classes and races
- Custom items and equipment
- Original monsters and encounters
- Unique spells and abilities
- Complete story campaigns
- Gameplay modifications

### Mod Types

**Total Conversion**: Replaces all core content with custom content.

**Campaign**: Adds new story content while using core classes/races/items.

**Content Pack**: Adds specific content types (e.g., 50 new items, 20 new monsters).

**Balance Mod**: Tweaks existing content for different difficulty or gameplay style.

---

## Core Concepts

### Data-Driven Design

Antares uses a **data-driven architecture**. This means:

- Game logic is in Rust code (the engine)
- Game content is in RON data files (your mod)
- You modify content, not code
- No programming required for most mods

**Example**: To add a new weapon, you edit `items.ron`, not Rust source code.

### Entity-Component Pattern

Content entities (characters, items, monsters) are composed of:

- **Core Attributes**: ID, name, basic stats
- **Components**: Optional features (spells, special abilities, bonuses)
- **Modifiers**: Temporary or permanent stat changes

**Example**: A "Flaming Sword" is a weapon (core) + fire damage bonus (component).

### ID System

Every entity has a unique numeric ID:

```rust
ItemId = u32
MonsterId = u32
SpellId = u32
MapId = u32
// etc.
```

**Rules**:
- IDs must be unique within their type
- IDs can be any positive integer
- Use consistent numbering schemes (e.g., weapons 1-100, armor 101-200)
- Document your ID ranges

### Disablement System

The **Disablement** bitfield controls class/race restrictions:

```ron
// Example: Item usable by Knight (bit 1) and Cleric (bit 4)
disablements: Disablement(5)  // 5 = 1 + 4 (binary: 0101)
```

**Bitfield Values**:
- Bit 0 (value 1): Class 1 (e.g., Knight)
- Bit 1 (value 2): Class 2 (e.g., Mage)
- Bit 2 (value 4): Class 3 (e.g., Cleric)
- Bit 3 (value 8): Class 4 (e.g., Rogue)
- etc.

**Special Cases**:
- `Disablement(0)`: All classes can use
- `Disablement(255)`: No classes can use (quest items)

**Combining Restrictions**:
```ron
// Knight (1) + Cleric (4) = 5
disablements: Disablement(5)

// Mage (2) + Rogue (8) = 10
disablements: Disablement(10)

// All classes except Thief (16)
disablements: Disablement(239)  // 255 - 16
```

---

## Campaign Architecture

### Directory Structure

Standard campaign layout:

```
campaigns/my_campaign/
├── campaign.ron           # Campaign metadata
├── README.md             # Description and credits
├── LICENSE.txt           # License (optional)
├── data/
│   ├── classes.ron       # Character classes
│   ├── races.ron         # Playable races
│   ├── items.ron         # All items
│   ├── monsters.ron      # All monsters
│   ├── spells.ron        # All spells
│   └── maps/             # Map files
│       ├── town.ron
│       ├── dungeon_01.ron
│       └── ...
├── assets/ (optional)
│   ├── music/
│   └── images/
└── docs/ (optional)
    ├── walkthrough.md
    └── design_notes.md
```

### Campaign Metadata

The `campaign.ron` file defines campaign identity:

```ron
(
    id: "unique_campaign_id",        // Lowercase, underscores only
    name: "Display Name",             // Human-readable
    version: "1.0.0",                 // Semantic versioning
    author: "Your Name",
    description: "Short description of campaign theme and scope.",
    starting_map: 1,                  // MapId where players start
    min_engine_version: "0.1.0",      // Minimum Antares version
)
```

### Content Files

Each content type has its own RON file:

**classes.ron**: HashMap of ClassId → ClassDefinition

**races.ron**: HashMap of RaceId → RaceDefinition

**items.ron**: HashMap of ItemId → Item

**monsters.ron**: HashMap of MonsterId → Monster

**spells.ron**: HashMap of SpellId → Spell

**maps/*.ron**: Individual map files (one per map)

---

## Content Design Patterns

### Pattern 1: Progressive Equipment

Design items in tiers that match player progression:

```ron
// Tier 1: Starting equipment (levels 1-3)
{
    1: (
        id: 1,
        name: "Rusty Dagger",
        item_type: Weapon((damage: (1, 4), ...)),
        value: 5,
        // ...
    ),
    
    // Tier 2: Early game (levels 4-6)
    10: (
        id: 10,
        name: "Iron Dagger",
        item_type: Weapon((damage: (1, 6), ...)),
        value: 50,
        // ...
    ),
    
    // Tier 3: Mid game (levels 7-10)
    20: (
        id: 20,
        name: "Steel Dagger",
        item_type: Weapon((damage: (2, 6), ...)),
        value: 200,
        // ...
    ),
}
```

**Benefits**:
- Players feel progression
- Older items remain useful (sell value, backups)
- Easy to balance

### Pattern 2: Class-Specific Items

Create items tailored to each class:

```ron
// Knight: Heavy armor, two-handed weapons
{
    100: (
        name: "Plate Mail",
        item_type: Armor((ac_bonus: 8, armor_type: Heavy, ...)),
        disablements: Disablement(1),  // Knight only
        // ...
    ),
}

// Mage: Light armor, spell-enhancing items
{
    200: (
        name: "Robe of the Archmage",
        item_type: Armor((ac_bonus: 2, armor_type: Light, ...)),
        disablements: Disablement(2),  // Mage only
        bonuses: [
            (attribute: SpellPower, value: Constant(5)),
        ],
        // ...
    ),
}
```

### Pattern 3: Risk-Reward Items

Cursed or high-risk items with powerful effects:

```ron
{
    300: (
        name: "Berserker Axe",
        item_type: Weapon((damage: (3, 8), ...)),
        cursed: true,  // Cannot be unequipped easily
        bonuses: [
            (attribute: Might, value: Constant(5)),      // +5 Might
            (attribute: ArmorClass, value: Constant(-2)), // -2 AC (negative)
        ],
        // ...
    ),
}
```

**Use Cases**:
- High damage but lowers defense
- Powerful spell effects but drains HP
- Stat boosts but character conditions

### Pattern 4: Consumable Economy

Balance consumable items with appropriate costs:

```ron
{
    // Cheap, weak healing
    400: (
        name: "Minor Healing Potion",
        item_type: Consumable((effect: HealHp((1, 8)), uses: 1)),
        value: 10,
        // ...
    ),
    
    // Expensive, strong healing
    401: (
        name: "Greater Healing Potion",
        item_type: Consumable((effect: HealHp((4, 8)), uses: 1)),
        value: 100,
        // ...
    ),
    
    // Multi-use items
    402: (
        name: "Healing Salve",
        item_type: Consumable((effect: HealHp((1, 6)), uses: 5)),
        value: 40,
        // ...
    ),
}
```

### Pattern 5: Encounter Design

Structure map encounters for pacing:

```ron
events: [
    // Easy encounter (map entrance)
    (
        position: (5, 2),
        event_type: Combat([
            (monster_id: 1, count: 2),  // 2 weak monsters
        ]),
    ),
    
    // Medium encounter (mid-dungeon)
    (
        position: (10, 8),
        event_type: Combat([
            (monster_id: 1, count: 3),
            (monster_id: 2, count: 1),  // Mixed difficulty
        ]),
    ),
    
    // Boss encounter (end)
    (
        position: (15, 15),
        event_type: Combat([
            (monster_id: 10, count: 1),  // Single powerful boss
        ]),
    ),
]
```

**Pacing Principles**:
- Start easy, ramp up gradually
- Mix combat with treasure and dialogue
- Save hardest encounters for end
- Provide rest/healing opportunities

### Pattern 6: Environmental Storytelling

Use map features to tell stories without dialogue:

```ron
events: [
    // Corpse with loot tells a story
    (
        position: (7, 3),
        event_type: Text("A fallen adventurer lies here, clutching a blood-stained map."),
    ),
    (
        position: (7, 3),
        event_type: Treasure([
            (item_id: 999, quantity: 1),  // "Blood-Stained Map" (quest item)
        ]),
    ),
    
    // Trap warning from environment
    (
        position: (8, 5),
        event_type: Text("Scorch marks cover the walls. Something dangerous happened here."),
    ),
    (
        position: (9, 5),
        event_type: Damage((2, 6)),  // Fireball trap
    ),
]
```

---

## Advanced Techniques

### Technique 1: Dynamic Stat Bonuses

Items with conditional or temporary bonuses:

```ron
bonuses: [
    // Permanent bonus
    (attribute: Might, value: Constant(3)),
    
    // Temporary bonus (duration in turns)
    (attribute: Speed, value: Temporary(5, 10)),  // +5 Speed for 10 turns
    
    // Conditional bonus (implementation-specific)
    (attribute: AttackBonus, value: Constant(2)),  // vs specific enemy type
]
```

### Technique 2: Spell-Effect Items

Items that cast spells when used:

```ron
{
    500: (
        name: "Wand of Fireballs",
        item_type: Accessory((
            slot: Held,
            charges: Some(10),
            spell_id: Some(20),  // Fireball spell
        )),
        // ...
    ),
}
```

### Technique 3: Quest-Gated Content

Use quest states to control access:

```ron
events: [
    // Door only opens if quest completed
    (
        position: (10, 10),
        event_type: ConditionalEvent((
            condition: QuestCompleted(5),
            success: Teleport((destination_map: 2, destination_position: (5, 5))),
            failure: Text("The door is sealed by ancient magic."),
        )),
    ),
]
```

### Technique 4: Multi-Phase Bosses

Bosses that change behavior at HP thresholds:

```ron
{
    1000: (
        name: "Dragon Lord",
        hp: (20, 8),  // High HP pool
        special_attacks: [
            "FireBreath",    // Used at > 50% HP
            "TailSwipe",     // Used at 25-50% HP
            "DesperateFury", // Used at < 25% HP
        ],
        // ...
    ),
}
```

### Technique 5: Interconnected Maps

Create a coherent world with bidirectional exits:

```ron
// Map 1: Town
exits: [
    (
        position: (20, 10),
        destination_map: 2,      // To forest
        destination_position: (1, 10),
        direction: East,
    ),
]

// Map 2: Forest
exits: [
    (
        position: (1, 10),
        destination_map: 1,      // Back to town
        destination_position: (20, 10),
        direction: West,
    ),
    (
        position: (40, 20),
        destination_map: 3,      // To dungeon
        destination_position: (1, 1),
        direction: North,
    ),
]
```

**Map Connectivity Rules**:
- Every map must be reachable from starting map
- Provide return paths (players shouldn't get stuck)
- Use directional exits for immersion

---

## Balancing Guidelines

### Character Balance

**Class Balance**:
- Pure casters: Low HP (d6), high spell damage
- Hybrids: Medium HP (d8), some spells
- Pure fighters: High HP (d10-d12), high physical damage

**Race Balance**:
- Total stat modifiers should sum to +2 to +4
- Negative modifiers balance positive ones
- Special abilities count as +1 to +2 stat points

### Item Balance

**Weapon Damage by Tier**:
- Tier 1 (levels 1-3): 1d4 to 1d6
- Tier 2 (levels 4-6): 1d8 to 2d4
- Tier 3 (levels 7-10): 2d6 to 2d8
- Tier 4 (levels 11+): 2d10 to 4d6

**Armor AC by Tier**:
- Light armor: +1 to +3
- Medium armor: +4 to +6
- Heavy armor: +7 to +10

**Item Value Formula**:
```
Value = Base + (Damage × 10) + (AC × 15) + (Bonus × 20)
```

**Example**:
```
Longsword: 1d8 damage
Base: 50 gold
Damage: 8 × 10 = 80 gold
Total: 130 gold
```

### Monster Balance

**XP Award Formula**:
```
XP = (Level × 50) + (AC × 10) + (HP Average × 5) + (Special Attacks × 50)
```

**Example**:
```
Goblin Shaman (Level 3, AC 12, HP 3d6 avg 10, 1 special attack)
XP = (3 × 50) + (12 × 10) + (10 × 5) + (1 × 50)
XP = 150 + 120 + 50 + 50 = 370
```

**Loot Drop Rate**:
- Common enemies: 25-50% chance
- Elite enemies: 50-75% chance
- Bosses: 100% chance + bonus loot

### Spell Balance

**SP Cost Formula**:
```
SP Cost = (Spell Level × 5) + Target Multiplier

Target Multipliers:
- Single: 0
- AllEnemies: +5
- AllAllies: +3
- Area: +3
```

**Damage Scaling**:
- Level 1: 1d6 to 1d8
- Level 2: 2d6 to 2d8
- Level 3: 3d6 to 3d8
- Level 4+: 4d6 to 4d10

### Encounter Balance

**Combat Difficulty**:
```
Party Power = (Average Party Level) × (Party Size) × 100

Easy Encounter: 50% of Party Power
Medium Encounter: 100% of Party Power
Hard Encounter: 150% of Party Power
Boss Encounter: 200% of Party Power
```

**Example**:
```
Party: 4 characters, average level 5
Party Power = 5 × 4 × 100 = 2000

Medium Encounter: 2000 XP worth of monsters
Could be: 4 × Goblins (500 XP each)
Or: 2 × Ogres (1000 XP each)
```

---

## Performance Considerations

### Map Size

**Recommendations**:
- Small maps: 10×10 to 20×20
- Medium maps: 30×30 to 50×50
- Large maps: 60×60 to 100×100
- Maximum: 200×200 (use sparingly)

**Why**: Larger maps increase memory usage and save file size.

### Event Density

**Guidelines**:
- 1 event per 10-20 tiles (sparse)
- 1 event per 5-10 tiles (moderate)
- 1 event per 2-5 tiles (dense)

**Why**: Too many events slow down map loading and pathfinding.

### Content Database Size

**Recommendations**:
- Items: 100-500 per campaign
- Monsters: 50-200 per campaign
- Spells: 30-100 per campaign
- Maps: 10-50 per campaign

**Why**: Validation time increases with content size.

### RON File Optimization

**Tips**:
- Use consistent indentation (2 or 4 spaces)
- Remove unnecessary whitespace in production
- Split large data files by category
- Compress map tile arrays when possible

---

## Publishing Your Mod

### Pre-Release Checklist

- [ ] Run campaign validator (zero errors)
- [ ] Playtest entire campaign start-to-finish
- [ ] Balance check all encounters
- [ ] Proofread all dialogue and descriptions
- [ ] Document known issues
- [ ] Write README with installation instructions
- [ ] Include credits and license

### Packaging

Use the campaign packager:

```bash
campaign_validator --package campaigns/my_campaign my_campaign_v1.0.tar.gz
```

### README Template

```markdown
# My Campaign Name

**Version**: 1.0.0
**Author**: Your Name
**Difficulty**: Medium
**Estimated Playtime**: 5-10 hours

## Description

Brief description of your campaign's theme, story, and unique features.

## Features

- 5 new character classes
- 10 dungeons
- 50+ new items
- Original storyline

## Installation

1. Extract archive to `campaigns/` directory
2. Launch Antares
3. Select "My Campaign" from campaign list

## Credits

- Design: Your Name
- Testing: Tester Names
- Music: Artist Name (if applicable)

## License

MIT License (or your choice)
```

### Versioning

Use semantic versioning:

- **Major (1.0.0)**: Breaking changes, complete rewrites
- **Minor (0.1.0)**: New features, new content
- **Patch (0.0.1)**: Bug fixes, balance tweaks

### Distribution

Share your campaign:

1. **Official Mod Repository**: Submit to Antares mod database (if available)
2. **GitHub/GitLab**: Host source and releases
3. **Community Forums**: Post download links and discussion
4. **Modding Discord**: Share with community

---

## Best Practices Summary

### Do

✓ Validate your campaign frequently during development
✓ Playtest with fresh characters (not over-leveled)
✓ Document your ID ranges and naming conventions
✓ Use meaningful names for maps, items, and NPCs
✓ Provide multiple solutions to challenges
✓ Balance risk vs. reward
✓ Include README and credits

### Don't

✗ Use duplicate IDs
✗ Create disconnected maps (unreachable)
✗ Reference non-existent content IDs
✗ Make encounters impossibly hard
✗ Hardcode player names or assumptions
✗ Use copyrighted content without permission
✗ Skip validation before release

---

## Resources

- **SDK API Reference**: `docs/reference/sdk_api.md`
- **Campaign Tutorial**: `docs/tutorials/creating_campaigns.md`
- **Tool Guides**: `docs/how-to/`
- **Architecture**: `docs/reference/architecture.md`
- **Community**: [Antares Discord/Forum]

---

## Getting Help

If you encounter issues:

1. Check validation errors first
2. Review this guide and SDK API reference
3. Search community forums for similar issues
4. Ask in modding Discord channel
5. Open GitHub issue with campaign validator output

---

## Conclusion

Antares provides a powerful, flexible modding system. With these patterns and guidelines, you can create professional-quality campaigns.

**Happy modding!**
