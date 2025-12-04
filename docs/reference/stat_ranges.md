# Stat Ranges Reference

This document provides a comprehensive reference for all character, monster, and
game statistic ranges in Antares. These values define the valid bounds for game
data and are enforced by validation in the SDK and editors.

## Overview

Antares uses several numeric types for different statistics:

| Type              | Rust Type          | Range             | Typical Use                         |
| ----------------- | ------------------ | ----------------- | ----------------------------------- |
| `AttributePair`   | `u8` base/current  | 0-255             | Primary attributes, AC, resistances |
| `AttributePair16` | `u16` base/current | 0-65,535          | HP, SP                              |
| Signed modifier   | `i16`              | -32,768 to 32,767 | Temporary buffs/debuffs             |

## Character Statistics

### Primary Attributes

Primary attributes define a character's core capabilities.

| Attribute   | Min | Max | Default | Notes                                      |
| ----------- | --- | --- | ------- | ------------------------------------------ |
| Might       | 3   | 255 | 10      | Physical strength, melee damage bonus      |
| Intellect   | 3   | 255 | 10      | Magical power, spell effectiveness         |
| Personality | 3   | 255 | 10      | Charisma, social interactions, healing     |
| Endurance   | 3   | 255 | 10      | Constitution, HP calculation               |
| Speed       | 3   | 255 | 10      | Initiative, turn order, evasion            |
| Accuracy    | 3   | 255 | 10      | Hit chance, ranged attack bonus            |
| Luck        | 3   | 255 | 10      | Critical hits, random events, loot quality |

**Constants:**

- `ATTRIBUTE_MIN = 3`
- `ATTRIBUTE_MAX = 255`
- `ATTRIBUTE_DEFAULT = 10`

**Notes:**

- Attributes below 3 are generally not achievable through normal gameplay
- Racial and class bonuses are applied at character creation
- Temporary modifiers can push current value above or below these limits

### Hit Points (HP) and Spell Points (SP)

| Stat | Min | Max   | Default | Notes                                    |
| ---- | --- | ----- | ------- | ---------------------------------------- |
| HP   | 0   | 9,999 | Varies  | Based on class and Endurance             |
| SP   | 0   | 9,999 | Varies  | Based on class and Intellect/Personality |

**Constants:**

- `HP_SP_MIN = 0`
- `HP_SP_MAX = 9999`

**Notes:**

- HP at 0 means unconscious; negative HP means dead
- SP at 0 means no spells can be cast
- Maximum is soft-capped at 9,999 for display purposes

### Armor Class (AC)

| Stat | Min | Max | Default | Notes                            |
| ---- | --- | --- | ------- | -------------------------------- |
| AC   | 0   | 30  | 10      | Higher is better (harder to hit) |

**Constants:**

- `AC_MIN = 0`
- `AC_MAX = 30`
- `AC_DEFAULT = 10`

**Notes:**

- AC 10 represents an unarmored character
- Each point of AC reduces enemy hit chance
- Magical bonuses can push AC above 30

### Level and Experience

| Stat       | Min | Max    | Notes                               |
| ---------- | --- | ------ | ----------------------------------- |
| Level      | 1   | 200    | Practical maximum based on XP curve |
| Experience | 0   | 2^64-1 | No practical limit                  |

**Constants:**

- `LEVEL_MIN = 1`
- `LEVEL_MAX = 200`

**Notes:**

- Level 1 is the starting level for all characters
- XP requirements increase exponentially
- Level 200 requires astronomical XP amounts

### Spell Level

| Stat        | Min | Max | Notes                       |
| ----------- | --- | --- | --------------------------- |
| Spell Level | 1   | 7   | Maximum castable spell tier |

**Constants:**

- `SPELL_LEVEL_MIN = 1`
- `SPELL_LEVEL_MAX = 7`

**Notes:**

- Clerics and Sorcerers each have 7 spell levels
- Characters learn higher spell levels as they gain experience

### Age

| Stat | Min | Max | Default | Notes    |
| ---- | --- | --- | ------- | -------- |
| Age  | 18  | 200 | 18      | In years |

**Constants:**

- `AGE_MIN = 18`
- `AGE_MAX = 200`

**Notes:**

- Characters start at age 18
- Aging occurs through magical effects and rest
- Death from old age occurs at varying ages by race

### Food

| Stat | Min | Max | Default | Notes                    |
| ---- | --- | --- | ------- | ------------------------ |
| Food | 0   | 40  | 10      | Food units per character |

**Constants:**

- `FOOD_MIN = 0`
- `FOOD_MAX = 40`
- `FOOD_DEFAULT = 10`

**Notes:**

- Food is consumed during travel
- Starvation causes HP damage
- Food can be purchased in towns

### Resistances

| Resistance  | Min | Max | Default | Notes                      |
| ----------- | --- | --- | ------- | -------------------------- |
| Magic       | 0   | 100 | 0       | Generic spell resistance   |
| Fire        | 0   | 100 | 0       | Fire damage reduction      |
| Cold        | 0   | 100 | 0       | Cold damage reduction      |
| Electricity | 0   | 100 | 0       | Lightning damage reduction |
| Acid        | 0   | 100 | 0       | Acid damage reduction      |
| Poison      | 0   | 100 | 0       | Poison resistance          |
| Fear        | 0   | 100 | 0       | Fear effect resistance     |
| Psychic     | 0   | 100 | 0       | Mental attack resistance   |

**Constants:**

- `RESISTANCE_MIN = 0`
- `RESISTANCE_MAX = 100`

**Notes:**

- Resistance is expressed as a percentage
- 100% resistance means immunity
- Some races have innate resistances

## Party and Roster Limits

| Limit           | Value | Notes                        |
| --------------- | ----- | ---------------------------- |
| Party Size      | 6     | Maximum active party members |
| Roster Size     | 18    | Maximum characters at inns   |
| Inventory Slots | 6     | Per character backpack slots |
| Equipment Slots | 6     | Per character equipped items |

**Constants:**

- `PARTY_MAX_SIZE = 6`
- `ROSTER_MAX_SIZE = 18`
- `INVENTORY_MAX_SLOTS = 6`
- `EQUIPMENT_MAX_SLOTS = 6`

## Monster Statistics

Monsters use similar statistics to characters with some differences.

### Monster HP and AC

| Stat | Min | Max    | Notes                               |
| ---- | --- | ------ | ----------------------------------- |
| HP   | 1   | 65,535 | Boss monsters can have very high HP |
| AC   | 0   | 30     | Same scale as characters            |

### Monster Magic Resistance

| Stat             | Min | Max | Notes                              |
| ---------------- | --- | --- | ---------------------------------- |
| Magic Resistance | 0   | 100 | Percentage chance to resist spells |

## Attribute Modifiers

Attribute modifiers are used by equipment, spells, and conditions to temporarily
alter character statistics.

| Modifier Type      | Min   | Max   | Notes                                    |
| ------------------ | ----- | ----- | ---------------------------------------- |
| Attribute Modifier | -255  | +255  | Applied to AttributePair current value   |
| HP/SP Modifier     | -9999 | +9999 | Applied to AttributePair16 current value |

**Constants:**

- `ATTRIBUTE_MODIFIER_MIN = -255`
- `ATTRIBUTE_MODIFIER_MAX = 255`

**Notes:**

- Modifiers affect the `current` value, not the `base` value
- Calling `reset()` on an AttributePair restores `current` to `base`
- Multiple modifiers stack additively

## Item Statistics

### Item Charges

| Stat    | Min | Max | Notes                          |
| ------- | --- | --- | ------------------------------ |
| Charges | 0   | 255 | For magical/rechargeable items |

**Notes:**

- 0 charges means the item cannot be used (but may be rechargeable)
- Some items have unlimited uses (charges = 255 or special flag)

### Item Value

| Stat      | Min | Max           | Notes                   |
| --------- | --- | ------------- | ----------------------- |
| Base Cost | 0   | 4,294,967,295 | Purchase price in gold  |
| Sell Cost | 0   | 4,294,967,295 | Typically base_cost / 2 |

## Dice Rolls

Dice rolls are used for damage, healing, and random effects.

| Component   | Min  | Max  | Notes          |
| ----------- | ---- | ---- | -------------- |
| Count (NdX) | 1    | 255  | Number of dice |
| Sides (XdN) | 2    | 255  | Faces per die  |
| Bonus (+N)  | -128 | +127 | Flat modifier  |

**Notes:**

- Standard notation: `count`d`sides`+`bonus` (e.g., 2d6+3)
- Minimum sides is 2 (coin flip)
- Negative bonuses are valid (e.g., 1d6-2)

## Currency

| Currency | Min | Max           | Notes                                        |
| -------- | --- | ------------- | -------------------------------------------- |
| Gold     | 0   | 4,294,967,295 | Primary currency                             |
| Gems     | 0   | 4,294,967,295 | Secondary currency, used for expensive items |

## Map Coordinates

| Coordinate | Min | Max | Notes                 |
| ---------- | --- | --- | --------------------- |
| X          | 0   | 255 | Horizontal position   |
| Y          | 0   | 255 | Vertical position     |
| Map ID     | 0   | 255 | Unique map identifier |

**Notes:**

- Maps are 16x16 tiles by default
- Coordinates are 0-indexed

## Validation Rules

### Editor Validation

The campaign builder editors enforce these ranges:

1. **Attribute values** must be within `ATTRIBUTE_MIN` to `ATTRIBUTE_MAX`
2. **HP/SP values** must be within `HP_SP_MIN` to `HP_SP_MAX`
3. **AC values** should be within `AC_MIN` to `AC_MAX`
4. **Attribute modifiers** must be within `ATTRIBUTE_MODIFIER_MIN` to `ATTRIBUTE_MODIFIER_MAX`
5. **Dice counts** must be at least 1
6. **Dice sides** must be at least 2

### Runtime Validation

During gameplay, the engine uses saturating arithmetic to prevent overflow:

- Values cannot go below 0 (for unsigned types)
- Values cannot exceed the type's maximum
- Modifiers are applied using `saturating_add_signed()`

## Code Reference

These constants are defined in:

- `src/domain/character.rs` - Character stat constants
- `src/domain/types.rs` - Common type definitions

Example usage:

```rust
use antares::domain::character::{
    ATTRIBUTE_MIN, ATTRIBUTE_MAX, ATTRIBUTE_DEFAULT,
    HP_SP_MIN, HP_SP_MAX,
    AC_MIN, AC_MAX, AC_DEFAULT,
    ATTRIBUTE_MODIFIER_MIN, ATTRIBUTE_MODIFIER_MAX,
};

// Validate an attribute value
fn validate_attribute(value: u8) -> bool {
    value >= ATTRIBUTE_MIN && value <= ATTRIBUTE_MAX
}

// Validate an attribute modifier
fn validate_modifier(value: i16) -> bool {
    value >= ATTRIBUTE_MODIFIER_MIN && value <= ATTRIBUTE_MODIFIER_MAX
}
```

## See Also

- [Architecture Reference](architecture.md) - Overall system design
- [Character System](../explanation/character_system.md) - Character mechanics
- [Combat System](../explanation/combat_system.md) - Combat mechanics
