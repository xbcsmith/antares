# Creating Your First Campaign

**Difficulty**: Beginner
**Time**: 30-45 minutes
**Prerequisites**: Antares SDK installed

This tutorial walks you through creating a complete playable campaign from scratch using the data-driven architecture with proficiency-based restrictions and character definitions.

---

## Table of Contents

1. [What You'll Build](#what-youll-build)
2. [Setup](#setup)
3. [Step 1: Campaign Structure](#step-1-campaign-structure)
4. [Step 2: Campaign Metadata](#step-2-campaign-metadata)
5. [Step 3: Define Classes](#step-3-define-classes)
6. [Step 4: Define Races](#step-4-define-races)
7. [Step 5: Create Items](#step-5-create-items)
8. [Step 6: Create Monsters](#step-6-create-monsters)
9. [Step 7: Create Spells](#step-7-create-spells)
10. [Step 8: Create Character Definitions](#step-8-create-character-definitions)
11. [Step 9: Create Maps](#step-9-create-maps)
12. [Step 10: Validate](#step-10-validate)
13. [Step 11: Test](#step-11-test)
14. [Next Steps](#next-steps)

---

## What You'll Build

By the end of this tutorial, you'll have created **"The Cursed Village"**, a small campaign featuring:

- 2 character classes (Knight, Mage) with proficiencies
- 2 playable races (Human, Elf) with racial traits
- 5 items with proficiency requirements (weapons and armor)
- 3 monster types (Goblin, Dire Wolf, Necromancer)
- 3 spells (Heal, Fireball, Lightning Bolt)
- 2 pre-made character definitions
- 2 maps (Village, Cursed Crypt)

---

## Setup

### 1. Create Campaign Directory

```bash
cd campaigns
mkdir cursed_village
cd cursed_village
```

### 2. Install SDK Tools (if not already installed)

```bash
cd antares
cargo build --release --bin class_editor
cargo build --release --bin race_editor
cargo build --release --bin item_editor
cargo build --release --bin validate_campaign
```

These CLI editors make content creation easier than writing RON by hand. The validator ensures your campaign is properly structured.
Binaries will be in `target/release/`.

---

## Step 1: Campaign Structure

Create the required directory structure:

```bash
mkdir -p data/maps
touch campaign.ron
touch data/classes.ron
touch data/races.ron
touch data/items.ron
touch data/monsters.ron
touch data/spells.ron
touch data/character_definitions.ron
```

Your directory should now look like:

```
campaigns/cursed_village/
├── campaign.ron
└── data/
    ├── classes.ron
    ├── races.ron
    ├── items.ron
    ├── monsters.ron
    ├── spells.ron
    ├── character_definitions.ron
    └── maps/
```

---

## Step 2: Campaign Metadata

Edit `campaign.ron`:

```ron
(
    id: "cursed_village",
    name: "The Cursed Village",
    version: "1.0.0",
    author: "Your Name",
    description: "A small village plagued by undead. Investigate the cursed crypt and defeat the necromancer.",
    starting_map: 1,
    min_engine_version: "0.1.0",
)
```

**Field Explanations**:

- `id`: Unique identifier (lowercase, underscores only)
- `name`: Display name shown to players
- `version`: Semantic version (major.minor.patch)
- `starting_map`: MapId where players start (we'll create map 1 later)
- `min_engine_version`: Minimum Antares version required

---

## Step 3: Define Classes

### Using the Class Editor (Recommended)

```bash
../../target/release/class_editor data/classes.ron
```

Follow the prompts to add two classes:

**Knight**:

- ID: 1
- Name: Knight
- HP Die: d10 (10 sides)
- Pure Caster: No
- Spell School: None
- Spell Stat: None
- Special Abilities: None

**Mage**:

- ID: 2
- Name: Mage
- HP Die: d6 (6 sides)
- Pure Caster: Yes
- Spell School: Elemental
- Spell Stat: Intellect
- Special Abilities: None

Save and exit.

### Manual Creation (Alternative)

Create `data/classes.ron` (now using proficiency system):

```ron
{
    1: (
        id: 1,
        name: "Knight",
        hp_die: 10,
        spell_school: None,
        is_pure_caster: false,
        spell_stat: None,
        spell_progression: [],
        proficiencies: [ShortSword, LongSword, Axe, Mace, Shield, ChainArmor, PlateArmor],
        special_abilities: [],
    ),
    2: (
        id: 2,
        name: "Mage",
        hp_die: 6,
        spell_school: Some(Elemental),
        is_pure_caster: true,
        spell_stat: Some(Intellect),
        spell_progression: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        proficiencies: [Staff, Dagger, Cloth],
        special_abilities: [],
    ),
}
```

**Key Points**:

- `hp_die`: Number of sides on hit point die (d6 = 6, d8 = 8, d10 = 10)
- `proficiencies`: List of equipment/weapon types the class can use
- `is_pure_caster`: True if class gets spells every level
- `spell_progression`: Number of spells gained at each level (empty for non-casters)

**Proficiency System**: Classes now use proficiencies instead of disablement bits. Items require specific proficiencies to use.

---

## Step 4: Define Races

### Using the Race Editor (Recommended)

```bash
../../target/release/race_editor data/races.ron
```

**Human**:

- ID: 1
- Name: Human
- Stat Modifiers: All 0 (balanced)
- Proficiencies: None (no restrictions)
- Racial Tags: Medium (size)

**Elf**:

- ID: 2
- Name: Elf
- Stat Modifiers: Intellect +2, Endurance -1
- Proficiencies: LongBow (racial bonus)
- Racial Tags: Medium, Elf

### Manual Creation (Alternative)

Edit `data/races.ron`:

```ron
{
    1: (
        id: 1,
        name: "Human",
        stat_modifiers: (
            might: 0,
            intellect: 0,
            speed: 0,
            endurance: 0,
            accuracy: 0,
            personality: 0,
            luck: 0,
        ),
        resistances: 0,
        proficiencies: [],
        racial_tags: [Medium],
    ),
    2: (
        id: 2,
        name: "Elf",
        stat_modifiers: (
            might: 0,
            intellect: 2,
            speed: 0,
            endurance: -1,
            accuracy: 0,
            personality: 0,
            luck: 0,
        ),
        resistances: 0,
        proficiencies: [LongBow],
        racial_tags: [Medium, Elf],
    ),
}
```

**Racial Proficiencies**: Races can grant bonus proficiencies (e.g., Elves with LongBow).

**Racial Tags**: Used for item restrictions based on size or biology (e.g., Small races can't use Medium-sized weapons).

---

## Step 5: Create Items

### Using the Item Editor (Recommended)

```bash
../../target/release/item_editor data/items.ron
```

Create these items:

**1. Rusty Sword** (ID: 1)

- Type: Weapon (Short Sword classification)
- Damage: 1d6
- Value: 10
- Required Proficiency: ShortSword
- Usable by: Knights (have ShortSword proficiency)

**2. Steel Longsword** (ID: 2)

- Type: Weapon (Long Sword classification)
- Damage: 1d8
- Value: 100
- Required Proficiency: LongSword
- Usable by: Knights only

**3. Wooden Staff** (ID: 3)

- Type: Weapon (Staff classification)
- Damage: 1d4
- Value: 5
- Required Proficiency: Staff
- Usable by: Mages only

**4. Leather Armor** (ID: 4)

- Type: Armor (Light armor)
- AC Bonus: +2
- Value: 50
- Required Proficiency: None (light armor has no restriction)
- Usable by: All

**5. Healing Potion** (ID: 5)

- Type: Consumable
- Effect: Restore 2d8 HP
- Value: 25
- Required Proficiency: None
- Usable by: All

### Manual Creation (Alternative)

Edit `data/items.ron` (now using proficiency system):

```ron
{
    1: (
        id: 1,
        name: "Rusty Sword",
        item_type: Weapon((
            damage: (1, 6),
            damage_type: Physical,
            attack_bonus: 0,
            crit_chance: 5,
            classification: ShortSword,
        )),
        value: 10,
        weight: 3,
        tags: [Medium],
        alignment_restriction: None,
        bonuses: [],
        cursed: false,
        identified: true,
    ),
    2: (
        id: 2,
        name: "Steel Longsword",
        item_type: Weapon((
            damage: (1, 8),
            damage_type: Physical,
            attack_bonus: 1,
            crit_chance: 5,
            classification: LongSword,
        )),
        value: 100,
        weight: 4,
        tags: [Medium],
        alignment_restriction: None,
        bonuses: [],
        cursed: false,
        identified: true,
    ),
    3: (
        id: 3,
        name: "Wooden Staff",
        item_type: Weapon((
            damage: (1, 4),
            damage_type: Physical,
            attack_bonus: 0,
            crit_chance: 5,
            classification: Staff,
        )),
        value: 5,
        weight: 2,
        tags: [Medium],
        alignment_restriction: None,
        bonuses: [
            (attribute: SpellPower, value: Constant(2)),
        ],
        cursed: false,
        identified: true,
    ),
    4: (
        id: 4,
        name: "Leather Armor",
        item_type: Armor((
            ac_bonus: 2,
            classification: Light,
            dexterity_cap: None,
        )),
        value: 50,
        weight: 10,
        tags: [Medium],
        alignment_restriction: None,
        bonuses: [],
        cursed: false,
        identified: true,
    ),
    5: (
        id: 5,
        name: "Healing Potion",
        item_type: Consumable((
            effect: HealHp((2, 8)),
            uses: 1,
        )),
        value: 25,
        weight: 1,
        tags: [],
        alignment_restriction: None,
        bonuses: [],
        cursed: false,
        identified: true,
    ),
}
```

**Proficiency System**:

- Items now specify `classification` (e.g., ShortSword, Staff, Light)
- Classes/races have `proficiencies` lists
- Characters can use items if they have the matching proficiency
- `tags` restrict by race (e.g., Medium-sized weapons for Medium+ races)
- `alignment_restriction` can limit items to Good, Neutral, or Evil characters

---

## Step 6: Create Monsters

Edit `data/monsters.ron`:

```ron
{
    1: (
        id: 1,
        name: "Goblin",
        level: 1,
        hp: (2, 6), // 2d6 HP
        ac: 12,
        attack_bonus: 2,
        damage: (1, 6),
        xp_value: 50,
        special_attacks: [],
        loot_table: [
            (item_id: 1, chance: 50), // 50% chance Rusty Sword
            (item_id: 5, chance: 25), // 25% chance Healing Potion
        ],
    ),
    2: (
        id: 2,
        name: "Dire Wolf",
        level: 2,
        hp: (3, 8),
        ac: 14,
        attack_bonus: 4,
        damage: (1, 8),
        xp_value: 100,
        special_attacks: [],
        loot_table: [],
    ),
    3: (
        id: 3,
        name: "Necromancer",
        level: 5,
        hp: (5, 8),
        ac: 13,
        attack_bonus: 3,
        damage: (1, 6),
        xp_value: 500,
        special_attacks: [
            "DrainLife",
            "SummonUndead",
        ],
        loot_table: [
            (item_id: 3, chance: 100), // Always drops staff
            (item_id: 5, chance: 50),  // 50% potion
        ],
    ),
}
```

**Monster Design Tips**:

- `level`: Determines difficulty (1-10 typical)
- `ac`: Armor Class (10-20 range)
- `attack_bonus`: Added to attack rolls (+0 to +10)
- `xp_value`: Experience points awarded (balance per level)
- `loot_table`: `chance` is percentage (0-100)

---

## Step 7: Create Spells

Edit `data/spells.ron`:

```ron
{
    1: (
        id: 1,
        name: "Heal",
        level: 1,
        sp_cost: 5,
        target: Single,
        effect: HealHp((1, 8)),
        school: Divine,
        description: "Restores 1d8 hit points to a single target.",
    ),
    2: (
        id: 2,
        name: "Fireball",
        level: 3,
        sp_cost: 15,
        target: AllEnemies,
        effect: Damage((
            dice: (3, 6),
            damage_type: Fire,
        )),
        school: Elemental,
        description: "Deals 3d6 fire damage to all enemies.",
    ),
    3: (
        id: 3,
        name: "Lightning Bolt",
        level: 2,
        sp_cost: 10,
        target: Single,
        effect: Damage((
            dice: (2, 8),
            damage_type: Lightning,
        )),
        school: Elemental,
        description: "Strikes a single enemy with 2d8 lightning damage.",
    ),
}
```

**Spell Balance Guidelines**:

- Level 1 spells: 5-10 SP, 1d6-1d8 damage
- Level 2 spells: 10-15 SP, 2d6-2d8 damage
- Level 3+ spells: 15-25 SP, 3d6-4d8 damage or area effects

---

## Step 8: Create Character Definitions

Character definitions are pre-made character templates that can be used for:

- Quick-start characters for players
- NPCs with complete stats
- Campaign-specific heroes

### Using the SDK (Recommended)

Use the Campaign Builder's Character Editor:

```bash
cargo run --bin campaign_builder
# Navigate to "Characters" tab
# Click "Add Character"
# Fill in the form and click "Save"
```

### Manual Creation

Create `data/character_definitions.ron`:

```ron
[
    (
        id: 1,
        name: "Sir Roland",
        race_id: 1,  // Human
        class_id: 1,  // Knight
        base_stats: (
            might: 16,
            intellect: 10,
            speed: 12,
            endurance: 14,
            accuracy: 13,
            personality: 11,
            luck: 10,
        ),
        starting_equipment: (
            weapon_id: Some(2),  // Steel Longsword
            armor_ids: [4],      // Leather Armor
            accessory_ids: [],
        ),
        starting_gold: 100,
        alignment: Good,
    ),
    (
        id: 2,
        name: "Elara the Wise",
        race_id: 2,  // Elf
        class_id: 2,  // Mage
        base_stats: (
            might: 8,
            intellect: 16,
            speed: 12,
            endurance: 10,
            accuracy: 11,
            personality: 14,
            luck: 12,
        ),
        starting_equipment: (
            weapon_id: Some(3),  // Wooden Staff
            armor_ids: [4],      // Leather Armor
            accessory_ids: [],
        ),
        starting_gold: 75,
        alignment: Neutral,
    ),
]
```

**Key Points**:

- `id`: Unique character definition ID
- `race_id`/`class_id`: Must match IDs in races.ron and classes.ron
- `base_stats`: Starting attribute values
- `starting_equipment`: Items the character begins with
- `alignment`: Good, Neutral, or Evil

---

## Step 9: Create Maps

### Map 1: Village (Starting Area)

Create `data/maps/village.ron`:

```ron
(
    id: 1,
    name: "Cursed Village",
    width: 10,
    height: 10,
    environment: Outdoor,
    tiles: [
        // Row 0 (top)
        Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor,
        // Row 1
        Floor, Floor, Floor, Wall, Wall, Wall, Wall, Floor, Floor, Floor,
        // Row 2
        Floor, Floor, Floor, Wall, Floor, Floor, Wall, Floor, Floor, Floor,
        // Row 3
        Floor, Floor, Floor, Wall, Floor, Floor, Wall, Floor, Floor, Floor,
        // Row 4
        Floor, Floor, Floor, Wall, Wall, Door, Wall, Floor, Floor, Floor,
        // Row 5
        Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor,
        // Row 6
        Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor,
        // Row 7
        Forest, Forest, Forest, Floor, Floor, Floor, Floor, Forest, Forest, Forest,
        // Row 8
        Forest, Forest, Forest, Floor, Floor, Floor, Floor, Forest, Forest, Forest,
        // Row 9 (bottom)
        Forest, Forest, Forest, Forest, Floor, Floor, Forest, Forest, Forest, Forest,
    ],
    events: [
        (
            position: (5, 2),
            event_type: Text("The village elder: 'Please help us! The crypt to the east is cursed!'"),
        ),
        (
            position: (4, 9),
            event_type: Combat([
                (monster_id: 1, count: 2), // 2 Goblins
            ]),
        ),
    ],
    npcs: [
        (
            id: 1,
            name: "Village Elder",
            position: (5, 2),
            dialogue_id: None,
        ),
    ],
    exits: [
        (
            position: (9, 4),
            destination_map: 2,
            destination_position: (1, 5),
            direction: East,
        ),
    ],
    starting_position: (5, 5),
)
```

**Map Key**:

- Tiles are listed left-to-right, top-to-bottom
- `(x, y)` coordinates: (0,0) = top-left
- Events trigger when party enters tile
- Exits connect to other maps

### Map 2: Cursed Crypt (Dungeon)

Create `data/maps/cursed_crypt.ron`:

```ron
(
    id: 2,
    name: "Cursed Crypt",
    width: 12,
    height: 12,
    environment: Indoor,
    tiles: [
        // Row 0
        Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall,
        // Row 1
        Wall, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Wall,
        // Row 2
        Wall, Floor, Wall, Wall, Wall, Floor, Floor, Wall, Wall, Wall, Floor, Wall,
        // Row 3
        Wall, Floor, Wall, Floor, Floor, Floor, Floor, Floor, Floor, Wall, Floor, Wall,
        // Row 4
        Wall, Floor, Wall, Floor, Wall, Wall, Wall, Wall, Floor, Wall, Floor, Wall,
        // Row 5
        Wall, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Wall,
        // Row 6
        Wall, Floor, Wall, Floor, Wall, Wall, Wall, Wall, Floor, Wall, Floor, Wall,
        // Row 7
        Wall, Floor, Wall, Floor, Floor, Floor, Floor, Floor, Floor, Wall, Floor, Wall,
        // Row 8
        Wall, Floor, Wall, Wall, Wall, Floor, Floor, Wall, Wall, Wall, Floor, Wall,
        // Row 9
        Wall, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Wall,
        // Row 10
        Wall, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Floor, Wall,
        // Row 11
        Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall, Wall,
    ],
    events: [
        (
            position: (6, 3),
            event_type: Combat([
                (monster_id: 2, count: 1), // 1 Dire Wolf
            ]),
        ),
        (
            position: (6, 6),
            event_type: Treasure([
                (item_id: 2, quantity: 1), // Steel Longsword
                (item_id: 5, quantity: 2), // 2 Healing Potions
            ]),
        ),
        (
            position: (6, 9),
            event_type: Combat([
                (monster_id: 3, count: 1), // Necromancer (boss)
            ]),
        ),
    ],
    npcs: [],
    exits: [
        (
            position: (1, 5),
            destination_map: 1,
            destination_position: (9, 4),
            direction: West,
        ),
    ],
    starting_position: (1, 5),
)
```

**Dungeon Design Tips**:

- Use `Wall` tiles for structure
- Place treasure after combat challenges
- Boss encounters at the end
- Include exit back to town

---

## Step 10: Validate

### Run the Campaign Validator

```bash
../../target/release/campaign_validator campaigns/cursed_village
```

**Expected Output**:

```
Loading campaign from: campaigns/cursed_village
✓ Loaded campaign: The Cursed Village v1.0.0

Content Summary:
  Classes: 2
  Races: 2
  Items: 5
  Monsters: 3
  Spells: 3
  Maps: 2

Running validation...
✓ No errors found

Campaign is valid!
```

### Common Validation Errors

If you see errors, here's how to fix them:

**Error: "MissingItem { context: 'treasure_event', item_id: 2 }"**

- Fix: Item ID 2 doesn't exist in `items.ron`
- Solution: Add the missing item or change the ID

**Error: "DisconnectedMap { map_id: 2 }"**

- Fix: Map 2 has no path back to starting map
- Solution: Add an exit connecting to map 1

**Error: "DuplicateId { entity_type: 'monster', id: 1 }"**

- Fix: Two monsters have the same ID
- Solution: Change one ID to a unique value

---

## Step 11: Test

### Manual Testing Checklist

1. **Character Creation**:

   - Can you create a Knight?
   - Can you create a Mage?
   - Do race stat modifiers apply?

2. **Village Map**:

   - Does the village render correctly?
   - Does the NPC dialogue appear?
   - Can you enter combat with goblins?

3. **Inventory**:

   - Can you equip the Rusty Sword?
   - Can Knights equip Steel Longsword?
   - Can Mages NOT equip Steel Longsword? (class restriction)

4. **Crypt Map**:

   - Can you travel from village to crypt?
   - Do encounters trigger?
   - Can you find treasure?
   - Can you defeat the Necromancer?

5. **Combat**:
   - Do spells work?
   - Do healing potions restore HP?
   - Do you gain XP after combat?

---

## Next Steps

### Expand Your Campaign

Now that you have the basics, try:

1. **Add More Classes**: Cleric, Rogue, Ranger
2. **Add Quests**: See `docs/how-to/using_quest_and_dialogue_tools.md`
3. **Add Dialogue Trees**: Create NPC conversations with choices
4. **Create More Maps**: Towns, dungeons, wilderness
5. **Add Magic Items**: Weapons with spell effects, cursed items
6. **Balance Testing**: Adjust monster difficulty and loot

### Advanced Topics

- **Branching Quests**: Multiple completion paths
- **Conditional Events**: Events that trigger based on quest state
- **Secret Areas**: Hidden rooms and bonus content
- **Boss Mechanics**: Special attacks and multi-phase fights

### Resources

- **SDK API Reference**: `docs/reference/sdk_api.md`
- **How-To Guides**: `docs/how-to/`
- **Modding Guide**: `docs/explanation/modding_guide.md`
- **Architecture**: `docs/reference/architecture.md`

---

## Troubleshooting

### Campaign Won't Load

**Problem**: `FileNotFound` error

**Solution**: Check directory structure matches expected layout:

```
campaigns/cursed_village/
├── campaign.ron
└── data/
    ├── classes.ron
    ├── races.ron
    ├── items.ron
    ├── monsters.ron
    ├── spells.ron
    └── maps/
        ├── village.ron
        └── cursed_crypt.ron
```

### RON Parse Errors

**Problem**: `ParseError: expected ','`

**Solution**: Check for:

- Missing commas between fields
- Missing closing parentheses `)` or braces `}`
- Incorrect nesting
- Typos in field names

Use `validate_ron_syntax` from SDK to check syntax.

### Items Not Appearing

**Problem**: Items defined but not showing in game

**Solution**: Verify:

- Item ID matches between `items.ron` and map events
- Item is marked as `identified: true`
- Character has required proficiency for the item

---

## Congratulations!

You've created your first Antares campaign! 🎉

Share your campaign with others by packaging it:

```bash
../../target/release/campaign_validator --package campaigns/cursed_village cursed_village_v1.0.tar.gz
```

For more advanced campaign creation techniques, see the **Modding Guide**.

---

**Last Updated**: 2025-01-25
**Version**: 2.0
