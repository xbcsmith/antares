# How to Write Character Definitions (RON Format)

This guide explains how to write character definitions in RON format for Antares campaigns, with a focus on the AttributePair migration completed in Phases 1 & 2.

## Quick Start

For most characters, use the **simple format**:

```ron
(
    id: "my_knight",
    name: "Sir Aldric",
    race_id: "human",
    class_id: "knight",
    sex: Male,
    alignment: Good,
    base_stats: (
        might: 16,
        intellect: 10,
        personality: 12,
        endurance: 15,
        speed: 11,
        accuracy: 13,
        luck: 10,
    ),
    hp_override: Some(50),  // Optional: omit to auto-calculate
    portrait_id: "knight_01",
    starting_gold: 100,
    starting_gems: 0,
    starting_food: 10,
    starting_items: [],
    starting_equipment: (
        weapon: Some(4),
        armor: Some(21),
        shield: None,
        helmet: None,
        boots: None,
        accessory1: None,
        accessory2: None,
    ),
    description: "A stalwart knight.",
    is_premade: true,
    starts_in_party: false,
)
```

## Stat Formats

### Simple Format (Recommended)

The simple format automatically expands single numbers to `AttributePair { base: N, current: N }`:

```ron
base_stats: (
    might: 15,        // Becomes: (base: 15, current: 15)
    intellect: 10,
    personality: 12,
    endurance: 14,
    speed: 11,
    accuracy: 13,
    luck: 10,
)
```

**Use this format for:**
- Normal characters without buffs/debuffs
- Most campaign characters
- Template characters for character creation

### Full Format (Advanced)

The full format lets you specify `base` and `current` separately:

```ron
base_stats: (
    might: (base: 15, current: 18),  // Character starts with +3 Might buff
    intellect: 10,                    // Simple format still works for other stats
    personality: 12,
    endurance: 14,
    speed: 11,
    accuracy: 13,
    luck: (base: 10, current: 15),   // Character starts with +5 Luck buff
)
```

**Use this format for:**
- Tutorial characters with starting buffs (teaching buff mechanics)
- Boss NPCs with pre-applied enhancements
- Special story characters (e.g., "Blessed Champion")
- Characters under temporary magical effects

**Important:** The system will clamp `current > base` to `base` with a warning during instantiation. This is intentional to prevent stat overflow, but you can still define pre-buffed characters in RON.

### Mixed Format (Allowed)

You can mix simple and full formats in the same character:

```ron
base_stats: (
    might: (base: 15, current: 18),  // Buffed
    intellect: 10,                    // Normal
    personality: 12,                  // Normal
    endurance: (base: 14, current: 14), // Explicit (same as simple)
    speed: 11,
    accuracy: 13,
    luck: 10,
)
```

## HP Override

### Auto-Calculated HP (Recommended)

Omit `hp_override` to let the system calculate HP based on class and endurance:

```ron
// No hp_override field
// System calculates: class_hp_base + (endurance - 10)
```

**Use this for:**
- Template characters
- Most recruitable NPCs
- Characters where HP should scale naturally

### Fixed HP (Simple Format)

Provide a single number to set both base and current HP:

```ron
hp_override: Some(50),  // Becomes: (base: 50, current: 50)
```

**Use this for:**
- Pre-made characters with specific HP values
- Tutorial characters (consistent experience)
- Boss NPCs with exact HP requirements

### Wounded/Buffed HP (Full Format)

Use full format to set HP with `current != base`:

```ron
hp_override: Some((base: 50, current: 30)),  // Wounded: 30/50 HP
```

**Use this for:**
- Wounded recruitable NPCs (rescue missions)
- Characters with temporary HP buffs
- Story characters in specific health states

## Field Reference

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | String | Unique identifier (use `snake_case`) |
| `name` | String | Display name |
| `race_id` | String | Must match ID in `races.ron` |
| `class_id` | String | Must match ID in `classes.ron` |
| `sex` | Enum | `Male`, `Female`, or `Other` |
| `alignment` | Enum | `Good`, `Neutral`, or `Evil` |
| `base_stats` | Stats | See "Stat Formats" above |

### Optional Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `hp_override` | Option<u16> | None | Override calculated HP (see "HP Override") |
| `portrait_id` | String | `"0"` | Portrait asset key (normalized: lowercase, underscores) |
| `starting_gold` | u32 | 0 | Starting gold pieces |
| `starting_gems` | u32 | 0 | Starting gems |
| `starting_food` | u8 | 10 | Starting food units |
| `starting_items` | Vec<ItemId> | `[]` | Item IDs from `items.ron` (inventory) |
| `starting_equipment` | Equipment | empty | Equipped items (see below) |
| `description` | String | `""` | Character backstory/description |
| `is_premade` | bool | false | True for pre-made characters, false for templates |
| `starts_in_party` | bool | false | True if character joins starting party |

### Starting Equipment

```ron
starting_equipment: (
    weapon: Some(4),      // Item ID from items.ron
    armor: Some(21),
    shield: None,         // No shield equipped
    helmet: None,
    boots: None,
    accessory1: None,
    accessory2: None,
)
```

**Note:** All equipment slots default to `None` if omitted.

## Validation Rules

The system validates character definitions during loading:

### ID and Name Validation
- `id` must not be empty
- `name` must not be empty
- `race_id` must not be empty
- `class_id` must not be empty

### Portrait ID Validation
- Must be normalized (lowercase, spaces → underscores)
- **Valid:** `"knight_01"`, `"elf_mage"`, `"character_040"`
- **Invalid:** `"Knight 01"`, `"Elf-Mage"`, `123` (numeric)

### Stat Validation
- All stats must be in range 0-255 (u8)
- Standard RPG range is 3-18 for base values
- `current` can differ from `base` but will be clamped if > base

### HP Validation
- HP must be in range 0-65535 (u16)
- `current` will be clamped to `base` if greater

### Reference Validation
- `race_id` must exist in `data/races.ron`
- `class_id` must exist in `data/classes.ron`
- All item IDs must exist in `data/items.ron`

## Common Patterns

### Tutorial Character (Starting Buff)

```ron
(
    id: "tutorial_blessed_knight",
    name: "Kira the Blessed",
    race_id: "human",
    class_id: "knight",
    sex: Female,
    alignment: Good,
    base_stats: (
        might: (base: 14, current: 18),  // +4 Might from tutorial blessing
        intellect: 10,
        personality: 11,
        endurance: 14,
        speed: 12,
        accuracy: 12,
        luck: 10,
    ),
    hp_override: Some((base: 50, current: 60)),  // +10 HP from blessing
    description: "A young knight blessed by the temple for tutorial purposes. The blessing provides temporary bonuses to Might and HP.",
    is_premade: true,
    starts_in_party: true,
)
```

### Wounded Recruitable NPC

```ron
(
    id: "wounded_veteran",
    name: "Old Gareth",
    race_id: "dwarf",
    class_id: "knight",
    sex: Male,
    alignment: Neutral,
    base_stats: (
        might: 14,
        intellect: 9,
        personality: 10,
        endurance: 16,
        speed: 8,
        accuracy: 11,
        luck: 9,
    ),
    hp_override: Some((base: 50, current: 25)),  // Wounded: 25/50 HP
    description: "A grizzled veteran recovering from battle wounds. Rescue him and heal his injuries to recruit this powerful ally.",
    is_premade: true,
    starts_in_party: false,
)
```

### Template Character (Auto HP)

```ron
(
    id: "template_human_fighter",
    name: "Human Fighter",
    race_id: "human",
    class_id: "knight",
    sex: Male,
    alignment: Neutral,
    base_stats: (
        might: 14,
        intellect: 10,
        personality: 10,
        endurance: 13,
        speed: 11,
        accuracy: 12,
        luck: 10,
    ),
    // No hp_override - HP calculated from class + endurance
    description: "A standard human fighter template for character generation.",
    is_premade: false,  // Template, not pre-made
    starts_in_party: false,
)
```

## Legacy Format (Deprecated)

Old campaign files may use separate `hp_base` and `hp_current` fields:

```ron
hp_base: Some(50),
hp_current: Some(30),
```

**This format still works** (backward compatibility) but is **deprecated**. It automatically converts to:

```ron
hp_override: Some((base: 50, current: 30))
```

**For new characters, always use `hp_override` instead.**

## Best Practices

1. **Use simple format by default** - Only use full format when you need `current != base`
2. **Document buffs/debuffs** - Explain in `description` why a character has modified stats
3. **Normalize portrait IDs** - Use lowercase with underscores: `"character_040"` not `"Character 040"`
4. **Validate references** - Ensure all race/class/item IDs exist in their respective files
5. **Test loading** - Run `cargo test test_phase2` to verify your RON files load correctly
6. **Omit hp_override for templates** - Let the system calculate HP for character creation templates
7. **Use hp_override for pre-made characters** - Provides consistent player experience

## Example Files

- **Tutorial campaign:** `campaigns/tutorial/data/characters.ron` (9 characters, simple format)
- **Core data:** `data/characters.ron` (6 pre-made characters, simple format)
- **Format examples:** `data/examples/character_definition_formats.ron` (all formats demonstrated)

## Troubleshooting

### "Character definition not found"
- Check that `id` matches the lookup key exactly (case-sensitive)
- Verify the character is in the correct RON file for your campaign

### "Failed to load character database"
- Run `cargo check` to see detailed RON parsing errors
- Check for missing commas, mismatched parentheses, or typos
- Ensure all required fields are present

### "Portrait ID must be normalized"
- Use lowercase letters and underscores only
- Convert `"Knight 01"` → `"knight_01"`
- Remove special characters and hyphens

### "Invalid race_id/class_id"
- Verify the ID exists in `data/races.ron` or `data/classes.ron`
- Check for typos (IDs are case-sensitive)

### "Item ID not found"
- Ensure all `starting_items` and `starting_equipment` IDs exist in `data/items.ron`
- Check the item database for valid IDs

## Further Reading

- **Architecture:** `docs/reference/architecture.md` (Section 4 - Data Structures)
- **Migration Plan:** `docs/explanation/character_definition_attribute_pair_migration_plan.md`
- **Implementation:** `docs/explanation/implementations.md` (Phase 2 completion summary)
