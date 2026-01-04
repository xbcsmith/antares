# Campaign Content Format Reference

This document describes the data file formats used in Antares campaigns.

## Overview

Campaign content is defined using RON (Rusty Object Notation) format files located in the campaign's `data/` directory. Each file type defines a specific category of game content.

## File Structure

A typical campaign has the following data files:

```
campaigns/my_campaign/
├── campaign.ron           # Campaign metadata and configuration
└── data/
    ├── classes.ron        # Character classes
    ├── races.ron          # Character races
    ├── items.ron          # Items and equipment
    ├── spells.ron         # Spells and magic
    ├── monsters.ron       # Monster definitions
    ├── characters.ron     # Pre-made character definitions
    ├── quests.ron         # Quest definitions
    ├── dialogues.ron      # NPC dialogues
    └── maps/              # Map data files
        ├── map_001.ron
        ├── map_002.ron
        └── ...
```

---

## characters.ron Schema

The `characters.ron` file defines pre-made characters that can be used in the campaign. These characters can start in the party or be available for recruitment.

### File Format

```ron
(
    characters: [
        (
            id: "character_id_here",
            name: "Character Name",
            race_id: "race_id",
            class_id: "class_id",
            sex: Male,  // or Female
            alignment: Good,  // Good, Neutral, or Evil
            base_stats: (
                might: 10,
                intellect: 10,
                personality: 10,
                endurance: 10,
                speed: 10,
                accuracy: 10,
                luck: 10,
            ),
            portrait_id: "portrait_filename",
            starting_gold: 100,
            starting_items: [1, 2, 3],  // ItemId values
            starting_equipment: (
                weapon_hand: Some(10),
                offhand: None,
                missile: None,
                head: None,
                neck: None,
                body: None,
                hands: None,
                feet: None,
                finger_left: None,
                finger_right: None,
            ),
            hp_base: None,  // Optional override for base HP
            sp_base: None,  // Optional override for base SP
            is_premade: true,
            starts_in_party: false,
        ),
        // ... more characters
    ],
)
```

### CharacterDefinition Fields

#### Required Fields

- **`id`** (String): Unique identifier for this character definition
  - Must be unique across all characters in the campaign
  - Convention: use lowercase with underscores, e.g., `"tutorial_human_knight"`

- **`name`** (String): Display name shown in-game
  - Example: `"Kira"`, `"Aldric the Brave"`

- **`race_id`** (RaceId/String): Reference to a race defined in `races.ron`
  - Must match an existing race ID
  - Example: `"human"`, `"elf"`, `"dwarf"`

- **`class_id`** (ClassId/String): Reference to a class defined in `classes.ron`
  - Must match an existing class ID
  - Example: `"knight"`, `"sorcerer"`, `"cleric"`

- **`sex`** (Sex): Character sex
  - Values: `Male` or `Female`
  - Used for pronouns and certain game mechanics

- **`alignment`** (Alignment): Starting alignment
  - Values: `Good`, `Neutral`, or `Evil`
  - Affects equipment restrictions and quest availability

- **`base_stats`** (Stats): Base attribute values before race/class modifiers
  - All attributes must be specified
  - Typical range: 3-18 for starting characters
  - Fields: `might`, `intellect`, `personality`, `endurance`, `speed`, `accuracy`, `luck`

#### Optional Fields

- **`portrait_id`** (String, optional, default: `""`): Portrait filename stem
  - Normalized to lowercase with spaces replaced by underscores
  - Empty string `""` indicates no portrait
  - Example: `"human_knight"` → looks for `portraits/human_knight.png`

- **`starting_gold`** (u32, optional, default: `0`): Initial gold amount
  - Gold is shared across the party
  - Example: `100`

- **`starting_items`** (Vec\<ItemId\>, optional, default: `[]`): Items added to inventory
  - List of item IDs from `items.ron`
  - Items are placed in character's personal inventory
  - Example: `[1, 2, 5]` (ItemId values)

- **`starting_equipment`** (EquipmentSlots, optional): Pre-equipped items
  - All slots must be specified (use `None` for empty slots)
  - Items must also be compatible with character's class
  - Slots: `weapon_hand`, `offhand`, `missile`, `head`, `neck`, `body`, `hands`, `feet`, `finger_left`, `finger_right`

- **`hp_base`** (Option\<u16\>, optional, default: `None`): Base HP override
  - If `None`, HP is calculated from class and endurance
  - If set, overrides the calculated base HP
  - Example: `Some(10)`

- **`sp_base`** (Option\<u16\>, optional, default: `None`): Base SP override
  - If `None`, SP is calculated from class and intellect
  - If set, overrides the calculated base SP
  - Example: `Some(5)`

- **`is_premade`** (bool, optional, default: `false`): Whether this is a pre-made character
  - Pre-made characters appear in character selection/recruitment
  - Non-premade characters are templates for random generation
  - Should be `true` for most campaign characters

- **`starts_in_party`** (bool, optional, default: `false`): Whether character starts in the active party
  - If `true`, this character will be added to the starting party when creating a new game
  - **Maximum 6 characters can have this flag set to true** (enforced at campaign load time)
  - Use case: Pre-made tutorial characters that should immediately be available
  - Characters with `starts_in_party: false` start at the campaign's starting inn

### starts_in_party Field Details

The `starts_in_party` field controls party membership at game start:

#### Behavior

- **`starts_in_party: true`**: Character is placed directly in the active adventuring party
  - Party starts with this character immediately available
  - No recruitment step needed
  - Ideal for tutorial campaigns or story-driven starts

- **`starts_in_party: false`** (default): Character starts at the starting inn
  - Character must be recruited from the inn
  - Allows player choice in party composition
  - Standard for most characters

#### Constraints

- **Maximum 6 starting party members**: The game enforces a maximum party size of 6 characters
- Campaign validation will fail if more than 6 characters have `starts_in_party: true`
- This constraint is checked when:
  - Loading a campaign with `CampaignLoader`
  - Running `campaign_validator` tool
  - Initializing a new game with `GameState::initialize_roster()`

#### Example

```ron
(
    characters: [
        // Tutorial character - starts in party
        (
            id: "tutorial_knight",
            name: "Kira",
            race_id: "human",
            class_id: "knight",
            sex: Female,
            alignment: Good,
            base_stats: (
                might: 15,
                intellect: 10,
                personality: 12,
                endurance: 14,
                speed: 11,
                accuracy: 13,
                luck: 10,
            ),
            portrait_id: "human_knight_f",
            starting_gold: 50,
            starting_items: [1, 2],  // Sword and shield
            is_premade: true,
            starts_in_party: true,  // Starts in party
        ),

        // Optional recruit - starts at inn
        (
            id: "recruit_mage",
            name: "Aldric",
            race_id: "elf",
            class_id: "sorcerer",
            sex: Male,
            alignment: Good,
            base_stats: (
                might: 8,
                intellect: 16,
                personality: 14,
                endurance: 9,
                speed: 12,
                accuracy: 10,
                luck: 11,
            ),
            portrait_id: "elf_mage_m",
            starting_gold: 25,
            starting_items: [10],  // Staff
            is_premade: true,
            starts_in_party: false,  // Starts at inn, can be recruited
        ),
    ],
)
```

### Validation Rules

The campaign validator checks:

1. **Unique IDs**: No duplicate character IDs
2. **Valid References**:
   - `race_id` must exist in `races.ron`
   - `class_id` must exist in `classes.ron`
   - All item IDs in `starting_items` and `starting_equipment` must exist in `items.ron`
3. **Party Size**: Maximum 6 characters with `starts_in_party: true`
4. **Equipment Compatibility**: Starting equipment must be usable by the character's class
5. **Stat Ranges**: Base stats should be within reasonable ranges (typically 3-25)

### Error Messages

Common validation errors:

```
✗ Too many starting party members: 7 characters have starts_in_party=true, but max party size is 6
✗ Missing race 'invalid_race' referenced in character 'my_character'
✗ Missing class 'invalid_class' referenced in character 'my_character'
✗ Missing item ID 999 referenced in character 'my_character' starting items
```

---

## Validation

Use the `campaign_validator` tool to check campaign content:

```bash
# Validate a campaign
cargo run --bin campaign_validator -- campaigns/my_campaign

# Example output:
✓ Campaign structure valid
✓ 3 starting party members (max 6)
✓ 3 recruitable NPCs found
✓ Map events valid
✓ All character IDs referenced in maps exist in characters.ron
```

### Validation Checks

The validator performs:

1. **Structure validation**: Campaign directory structure and required files
2. **Cross-reference validation**: All ID references point to existing content
3. **Character validation**: Party size limits, valid references
4. **Quest validation**: Quest objectives and rewards are valid
5. **Dialogue validation**: Dialogue trees are well-formed
6. **Map validation**: Map events reference valid entities

---

## See Also

- `docs/how-to/create_campaign.md` - Campaign creation guide
- `docs/reference/architecture.md` - Game architecture overview
- `docs/explanation/character_definition_implementation_plan.md` - Character system details
- `docs/explanation/party_management_implementation_plan.md` - Party management system
