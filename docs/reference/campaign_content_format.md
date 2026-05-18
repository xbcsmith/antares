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
    ├── skills.ron         # Numeric skill definitions and scaling rules
    ├── characters.ron     # Pre-made character definitions
    ├── quests.ron         # Quest definitions
    ├── dialogues.ron      # NPC dialogues
    └── maps/              # Map data files
        ├── map_001.ron
        ├── map_002.ron
        └── ...
```

---

## skills.ron Schema

The `skills.ron` file defines numeric, level-scaled character capabilities.
Skills are distinct from proficiencies: proficiencies control binary item-use
permissions, while skills represent ranked capabilities such as perception,
disarm traps, item lore, and diplomacy.

### File Format

```antares/data/skills.ron#L1-80
[
    (
        id: "perception",
        name: "Perception",
        category: Exploration,
        description: "Notice hidden objects, traps, and threats.",
        scaling: Linear(base: 0, per_level: 1),
        max_rank: 50,
        is_trainable: true,
    ),
    (
        id: "disarm_traps",
        name: "Disarm Traps",
        category: Exploration,
        description: "Safely disarm traps.",
        scaling: Step(base: 0, per_levels: 2, amount: 1),
        max_rank: 25,
        is_trainable: true,
    ),
    (
        id: "diplomacy",
        name: "Diplomacy",
        category: Social,
        description: "Persuasion and negotiation.",
        scaling: Flat,
        max_rank: 30,
        is_trainable: true,
    ),
    (
        id: "arcane_lore",
        name: "Arcane Lore",
        category: Knowledge,
        description: "Knowledge of arcane forces.",
        scaling: Table(ranks_by_level: [0, 0, 1, 1, 2, 3]),
        max_rank: 40,
        is_trainable: true,
    ),
]
```

### SkillDefinition Fields

- **`id`** (`SkillId`/String): Unique lowercase snake_case skill identifier.
- **`name`** (String): Display name shown in UI.
- **`category`** (`SkillCategory`): One of `Combat`, `Exploration`, `Knowledge`, `Social`, or `Utility`.
- **`description`** (String): Tooltip/help text.
- **`scaling`** (`SkillScalingMode`): Auto-scaling rule. Supported modes are:
  - `Flat`
  - `Linear(base, per_level)`
  - `Step(base, per_levels, amount)`
  - `Table(ranks_by_level)`
- **`max_rank`** (`SkillRank`/u16): Hard cap for the effective skill rank.
- **`is_trainable`** (bool): Whether NPC skill trainers may improve the skill through paid skill-training services.

### Validation Rules

1. Skill IDs must be non-empty lowercase snake_case.
2. Skill names must not be empty.
3. `max_rank` must be greater than `0`.
4. `Step.per_levels` must be greater than `0`.
5. `Table.ranks_by_level` must not be empty.
6. Table ranks must not exceed `max_rank`.
7. NPC skill trainers may only reference skills with `is_trainable: true`.
8. Class and race `skill_grants` must reference defined skill IDs.
9. Dialogue `SkillCheck` conditions must reference defined skill IDs and must not require ranks above the skill's `max_rank`.

### NPC Skill Trainer Authoring

Campaign authors can create paid skill trainers in the Campaign Builder NPC
editor by enabling **Is Skill Trainer**, selecting one or more trainable skill
IDs from the skill autocomplete selector, and optionally overriding the fee base,
fee multiplier, or trainer-specific max rank. The SDK can create or repair the
matching dialogue branch; in runtime data that branch uses `OpenSkillTraining`
to enter the skill-training screen. The in-game UI then submits `TrainSkill`
requests to purchase individual rank increases.

Use `skill_grants` on classes/races for automatic expertise and NPC skill
trainers for paid persistent improvements. Do not use skills as item-use
permissions; keep item restrictions in proficiencies.

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
  - Running `antares-sdk campaign validate` tool
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

## Inn and Innkeeper System

Inn locations are referenced by innkeeper NPC IDs (String type), not numeric IDs.

Campaign Configuration:

- `starting_innkeeper: String` - NPC ID of the default innkeeper where non-party characters start
  - Must reference an NPC with `is_innkeeper: true`
  - Example: `"tutorial_innkeeper_town"`
  - Default: `"tutorial_innkeeper_town"`

Map Events:

- `EnterInn { innkeeper_id: String, ... }` - Triggers the inn management interface
  - Must reference an NPC with `is_innkeeper: true`
  - Example: `innkeeper_id: "tutorial_innkeeper_town"`

NPC Definition:

- `is_innkeeper: bool` - Marks an NPC as an innkeeper who can manage party roster
  - Required for NPCs referenced by `starting_innkeeper` or `EnterInn` events

Character Location Tracking:

- `CharacterLocation::AtInn(InnkeeperId)` - Character is stored at specified innkeeper's inn
  - Uses string innkeeper NPC ID
  - Example: `AtInn("tutorial_innkeeper_town")`

Example:

```ron
// In npcs.ron:
(
    id: "cozy_inn_mary",
    name: "Mary the Innkeeper",
    description: "A cheerful innkeeper who runs the Cozy Inn.",
    portrait_id: "innkeeper_mary",
    is_innkeeper: true,
    is_merchant: false,
    ...
)

// In campaign.ron:
CampaignMetadata(
    ...
    starting_innkeeper: "cozy_inn_mary",
    ...
)

// In map.ron events:
(x: 5, y: 4): EnterInn(
    name: "Cozy Inn Entrance",
    description: "A welcoming inn...",
    innkeeper_id: "cozy_inn_mary",
),
```

Validation:

- Campaign validator checks that `starting_innkeeper` references an existing NPC
- Campaign validator verifies the NPC has `is_innkeeper: true`
- Map validator checks that all `EnterInn` events reference valid innkeeper NPCs

---

## Validation

Use the `antares-sdk campaign validate` subcommand to check campaign content:

```bash
# Validate a campaign
cargo run --bin antares-sdk -- campaign validate campaigns/my_campaign

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

## GLB Texture Export Convention

When a `.glb` file is imported via the Campaign Builder importer and exported as
a Creature, Item, or Furniture asset, embedded textures are extracted and written
to the campaign directory following a predictable path convention.

### Texture Destination Path

Embedded GLB base-color textures are written to:

```text
assets/textures/imported/<asset_name>/<image_file>
```

- `<asset_name>` is derived from the exported RON file stem. For example, a
  creature exported to `assets/creatures/stone_golem.ron` places its textures
  under `assets/textures/imported/stone_golem/`.
- `<image_file>` is derived from the glTF image `name` field when present,
  sanitized to lowercase with non-alphanumeric characters replaced by `_`. When
  no name is present the fallback is `image_<index>.<ext>`.

### File Name Derivation

| Source                         | Resulting filename example |
| ------------------------------ | -------------------------- |
| glTF image `name: "AlbedoMap"` | `albedo_map.png`           |
| No name, index 0, PNG MIME     | `image_0.png`              |
| No name, index 1, JPEG MIME    | `image_1.jpg`              |

File extension is derived from the glTF image `mimeType` field:

| MIME type    | Extension |
| ------------ | --------- |
| `image/png`  | `.png`    |
| `image/jpeg` | `.jpg`    |
| `image/webp` | `.webp`   |
| other        | subtype   |

### RON texture_path Field

After export, each `MeshDefinition` in the written RON file contains a
`texture_path` field that is a **campaign-relative** path string:

```ron
// In assets/creatures/stone_golem.ron
meshes: [
    (
        // ...
        texture_path: Some("assets/textures/imported/stone_golem/albedo_map.png"),
    ),
]
```

The game runtime (`texture_loading_system` in `src/game/systems/creature_meshes.rs`)
loads this path directly via Bevy's asset server. The path is relative to the
Bevy asset root (the campaign directory), so no further processing is required.

### Deduplication

When two or more mesh primitives within the same export reference identical
byte-for-byte texture data, only one file is written to disk and all affected
`texture_path` fields point to the same destination.

### Unsupported PBR Channels

The following glTF PBR material texture channels are **not imported**:

| Channel                                         | Status                            |
| ----------------------------------------------- | --------------------------------- |
| `pbrMetallicRoughness.baseColorTexture`         | ✅ Supported (Phase 4)            |
| `normalTexture`                                 | ❌ Ignored — warning in UI status |
| `occlusionTexture`                              | ❌ Ignored                        |
| `pbrMetallicRoughness.metallicRoughnessTexture` | ❌ Ignored                        |

When any of these channels are present in the imported GLB, the Campaign Builder
importer displays a status warning:

```text
GLB: 2 mesh(es), 3 embedded image(s), 2 material(s)
  [unsupported PBR: normal/occlusion/metallic-roughness textures ignored]
```

The geometry and supported material properties (base color, metallic factor,
roughness factor, emissive factor, alpha mode) are still imported normally.

### Runtime Compatibility

After export, the game runtime does **not** require the original `.glb` file. It
uses only:

1. The exported RON file (`CreatureDefinition` / item / furniture) containing
   `MeshDefinition` entries with campaign-relative `texture_path` strings.
2. The copied texture files under `assets/textures/imported/`.

This is the same path convention used for OBJ imports, so the runtime
`texture_loading_system` handles both formats identically.

---

## See Also

- `docs/how-to/create_campaign.md` - Campaign creation guide
- `docs/reference/architecture.md` - Game architecture overview
- `docs/explanation/character_definition_implementation_plan.md` - Character system details
- `docs/explanation/party_management_implementation_plan.md` - Party management system
