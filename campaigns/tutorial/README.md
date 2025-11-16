# Tutorial Campaign: First Steps

**Version**: 1.0.0  
**Author**: Antares Development Team  
**Difficulty**: Beginner  
**Estimated Playtime**: 15-30 minutes

---

## Description

"First Steps" is a small introductory campaign designed to teach new players the core mechanics of Antares RPG. This campaign demonstrates:

- Character creation with basic classes and races
- Combat encounters of varying difficulty
- Treasure hunting and loot collection
- Item usage and equipment management
- Basic map exploration

This campaign also serves as a **reference implementation** for modders learning to create their own campaigns.

---

## Features

- **2 Character Classes**: Warrior (fighter) and Wizard (spellcaster)
- **2 Playable Races**: Human (balanced) and Elf (magic-focused)
- **4 Items**: Weapons, armor, and healing potions
- **3 Monster Types**: Goblins, Wolves, and a Bandit Leader boss
- **3 Spells**: Heal, Magic Missile, and Fireball
- **1 Map**: Training Grounds (outdoor area)

---

## Installation

### Using Campaign Loader

1. This campaign should already be in your `campaigns/tutorial/` directory
2. Launch Antares
3. Select "Tutorial: First Steps" from the campaign list
4. Create a new party and start playing

### Manual Installation (if distributing)

1. Extract archive to your Antares installation's `campaigns/` directory
2. The final path should be: `campaigns/tutorial/`
3. Launch Antares and select the campaign

---

## Walkthrough

### Starting Out

1. **Character Creation**: Create a party with at least one Warrior and one Wizard
2. **Starting Position**: You begin in the center of the Training Grounds (7, 7)
3. **First Objective**: Move north to meet the Training Master

### Exploration

- **Training Master**: NPC at position (7, 2) welcomes you
- **Treasure Chest**: At position (5, 3) contains a Short Sword and 2 Healing Potions
- **Building**: The walled structure in the north contains the treasure

### Combat Encounters

1. **First Fight** (7, 8): 2 Goblins - easy encounter to learn combat
2. **Second Fight** (10, 10): 1 Wolf - medium difficulty
3. **Boss Fight** (7, 13): Bandit Leader - hardest encounter, drops guaranteed loot

### Completion

- After defeating the Bandit Leader, move to (7, 14) for the completion message
- You have successfully completed the tutorial!

---

## Tips for New Players

- **Equip Items**: Make sure to equip weapons and armor before combat
- **Use Healing Potions**: Don't save them all for later - use them when needed
- **Class Restrictions**: Warriors can use any weapon, but some items are class-specific
- **Spell Points**: Wizards have limited SP - manage them carefully
- **Explore Thoroughly**: Check every tile for hidden treasures and events

---

## For Modders

This campaign is intentionally simple and well-documented. Use it as a template for your own campaigns.

### File Structure

```
campaigns/tutorial/
├── campaign.ron           # Campaign metadata
├── README.md             # This file
└── data/
    ├── classes.ron       # 2 classes: Warrior, Wizard
    ├── races.ron         # 2 races: Human, Elf
    ├── items.ron         # 4 items: weapons, armor, potions
    ├── monsters.ron      # 3 monsters with loot tables
    ├── spells.ron        # 3 spells of varying levels
    └── maps/
        └── start_area.ron # Single 15×15 map
```

### Learning Points

- **Minimal Content**: Shows the minimum required files for a valid campaign
- **Simple Structure**: Easy to understand and modify
- **Complete Example**: All content types represented
- **Validation-Ready**: Passes all campaign validator checks
- **Cross-References**: Demonstrates proper ID referencing

### Validation

To validate this campaign:

```bash
cargo run --bin campaign_validator campaigns/tutorial
```

Expected result: Zero errors, zero warnings.

---

## Credits

- **Design**: Antares Development Team
- **Purpose**: Educational reference implementation
- **License**: MIT License

---

## See Also

- **Campaign Creation Tutorial**: `docs/tutorials/creating_campaigns.md`
- **Modding Guide**: `docs/explanation/modding_guide.md`
- **SDK API Reference**: `docs/reference/sdk_api.md`

---

## Feedback

This tutorial campaign is designed to be the best possible introduction to Antares. If you have suggestions for improvement, please open an issue or submit a pull request.

Happy adventuring!
