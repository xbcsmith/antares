# Proficiency System

Last updated: 2025-01-26

## Overview

The proficiency system replaces the legacy `Disablement` bitmask system with a flexible, data-driven approach to item restrictions. Instead of hardcoded bit flags, characters can use items based on proficiencies granted by their class and race, combined with item tags for fine-grained restrictions.

**Key Principles:**

- **UNION Logic**: A character can use an item if EITHER their class OR race grants the required proficiency
- **Classification-Based**: Items are classified (weapon type, armor type, magic type) and map to proficiency requirements
- **Tag System**: Fine-grained restrictions beyond proficiency (e.g., `large_weapon`, `heavy_armor`, `arcane_focus`)
- **Race Incompatibility**: Races can declare incompatible tags that override proficiency grants
- **Data-Driven**: All proficiencies, classifications, and tags are defined in RON data files

## Classification Enums

Items are classified to determine their proficiency requirements.

### WeaponClassification

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum WeaponClassification {
    /// Basic weapons anyone can use: clubs, daggers, staffs
    Simple,
    /// Advanced melee weapons: swords, axes, maces (fighters, paladins)
    MartialMelee,
    /// Ranged weapons: bows, crossbows (archers, rangers)
    MartialRanged,
    /// Weapons without edge: maces, hammers, staffs (clerics)
    Blunt,
    /// Unarmed combat: fists, martial arts (monks)
    Unarmed,
}
```

**Mapping to Proficiencies:**

- `Simple` → `proficiency_simple_weapon`
- `MartialMelee` → `proficiency_martial_melee`
- `MartialRanged` → `proficiency_martial_ranged`
- `Blunt` → `proficiency_blunt_weapon`
- `Unarmed` → `proficiency_unarmed`

### ArmorClassification

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ArmorClassification {
    /// Light armor: leather, padded
    Light,
    /// Medium armor: chain mail, scale
    Medium,
    /// Heavy armor: plate mail, full plate
    Heavy,
    /// All shield types
    Shield,
}
```

**Mapping to Proficiencies:**

- `Light` → `proficiency_light_armor`
- `Medium` → `proficiency_medium_armor`
- `Heavy` → `proficiency_heavy_armor`
- `Shield` → `proficiency_shield`

### MagicItemClassification

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MagicItemClassification {
    /// Arcane items: wands, arcane scrolls (sorcerers)
    Arcane,
    /// Divine items: holy symbols, divine scrolls (clerics)
    Divine,
    /// Universal items: potions, rings (anyone)
    Universal,
}
```

**Mapping to Proficiencies:**

- `Arcane` → `proficiency_arcane_magic`
- `Divine` → `proficiency_divine_magic`
- `Universal` → No proficiency required

## Proficiency Resolution Logic

The system uses **UNION logic** to determine if a character can use an item:

### Step 1: Classification Check

```text
required_proficiency = item.classification.to_proficiency_id()

can_use = class.proficiencies.contains(required_proficiency)
          OR race.proficiencies.contains(required_proficiency)
```

### Step 2: Tag Incompatibility Check

```text
for tag in item.tags:
    if race.incompatible_tags.contains(tag):
        can_use = false
        break
```

### Step 3: Alignment Restriction Check

```text
if item.alignment_restriction.is_some():
    if not character.alignment.matches(item.alignment_restriction):
        can_use = false
```

**Important**: Proficiency does NOT override race tag incompatibility. If a race is incompatible with a tag, the character cannot use that item regardless of proficiency.

## Data File Formats

### Class Proficiencies (`data/classes.ron`)

```ron
ClassDefinition(
    id: "knight",
    name: "Knight",
    proficiencies: [
        "proficiency_simple_weapon",
        "proficiency_martial_melee",
        "proficiency_light_armor",
        "proficiency_medium_armor",
        "proficiency_heavy_armor",
        "proficiency_shield",
    ],
    // ... other fields
)
```

### Race Proficiencies (`data/races.ron`)

```ron
RaceDefinition(
    id: "elf",
    name: "Elf",
    proficiencies: [
        "proficiency_martial_ranged",  // Elves get longbow/shortbow
    ],
    incompatible_tags: [
        "large_weapon",  // Elves cannot use large weapons
    ],
    // ... other fields
)
```

### Item Classification (`data/items.ron`)

```ron
Item(
    id: 1,
    name: "Longsword",
    item_type: Weapon(WeaponData(
        damage: (dice: 1, sides: 8, modifier: 0),
        bonus: 0,
        hands_required: 1,
        classification: MartialMelee,
    )),
    tags: [],  // Optional fine-grained tags
    alignment_restriction: None,  // Optional alignment restriction
    // ... other fields
)
```

## Standard Proficiency IDs

The system defines 11 standard proficiency IDs:

**Weapon Proficiencies:**

- `proficiency_simple_weapon` - Clubs, daggers, staffs
- `proficiency_martial_melee` - Swords, axes, maces
- `proficiency_martial_ranged` - Bows, crossbows
- `proficiency_blunt_weapon` - Maces, hammers (clerics)
- `proficiency_unarmed` - Fists, martial arts

**Armor Proficiencies:**

- `proficiency_light_armor` - Leather, padded
- `proficiency_medium_armor` - Chain mail, scale
- `proficiency_heavy_armor` - Plate mail, full plate
- `proficiency_shield` - All shields

**Magic Proficiencies:**

- `proficiency_arcane_magic` - Wands, arcane scrolls
- `proficiency_divine_magic` - Holy symbols, divine scrolls

## Item Tags

Tags provide fine-grained restrictions beyond classification:

**Common Item Tags:**

- `large_weapon` - Weapons too large for small races (e.g., Halfling)
- `heavy_armor` - Armor too heavy for agile races
- `arcane_focus` - Required for certain spells
- `divine_symbol` - Required for divine magic
- `two_handed` - Requires both hands
- `finesse` - Can use DEX instead of STR for attack rolls

**Example Usage:**

```ron
Item(
    id: 10,
    name: "Greatsword",
    item_type: Weapon(WeaponData(
        damage: (dice: 2, sides: 6, modifier: 0),
        bonus: 0,
        hands_required: 2,
        classification: MartialMelee,
    )),
    tags: ["large_weapon", "two_handed"],
    // ...
)
```

A Halfling with `proficiency_martial_melee` still cannot use this weapon because Halflings have `"large_weapon"` in their `incompatible_tags`.

## Examples

### Example 1: Elf Sorcerer with Longbow

**Character:**
- Race: Elf (grants `proficiency_martial_ranged`)
- Class: Sorcerer (no weapon proficiencies)

**Item:**
- Longbow (classification: `MartialRanged`)

**Result:** ✅ **CAN USE** - Race grants required proficiency (UNION logic)

### Example 2: Halfling Knight with Greatsword

**Character:**
- Race: Halfling (incompatible_tags: `["large_weapon"]`)
- Class: Knight (grants `proficiency_martial_melee`)

**Item:**
- Greatsword (classification: `MartialMelee`, tags: `["large_weapon"]`)

**Result:** ❌ **CANNOT USE** - Race incompatibility overrides proficiency

### Example 3: Human Robber with Plate Mail

**Character:**
- Race: Human (no proficiencies)
- Class: Robber (no armor proficiencies)

**Item:**
- Plate Mail (classification: `Heavy`)

**Result:** ❌ **CANNOT USE** - Neither class nor race grants `proficiency_heavy_armor`

### Example 4: Paladin with Holy Symbol

**Character:**
- Race: Human (no proficiencies)
- Class: Paladin (grants `proficiency_divine_magic`)

**Item:**
- Holy Symbol (classification: `Divine`)

**Result:** ✅ **CAN USE** - Class grants required proficiency

## Alignment Restrictions

Alignment restrictions are checked AFTER proficiency and tag checks:

```ron
Item(
    id: 42,
    name: "Holy Avenger",
    item_type: Weapon(WeaponData(
        damage: (dice: 1, sides: 10, modifier: 3),
        bonus: 3,
        hands_required: 1,
        classification: MartialMelee,
    )),
    alignment_restriction: Some(GoodOnly),
    // ...
)
```

**Result:** Only good-aligned characters with `proficiency_martial_melee` can use this weapon.

## Migration from Disablement System

### What Changed

**Old System (Disablement Bitmask):**

```rust
pub struct Item {
    pub disablements: Disablement,  // u8 bitmask: 0xFF = all classes
    // ...
}

pub struct ClassDefinition {
    pub disablement_bit: u8,  // Bit index: 0-5 for classes, 6-7 for alignment
    // ...
}
```

**New System (Proficiency-Based):**

```rust
pub struct Item {
    pub item_type: ItemType,  // Contains classification
    pub tags: Vec<String>,
    pub alignment_restriction: Option<AlignmentRestriction>,
    // ...
}

pub struct ClassDefinition {
    pub proficiencies: Vec<ProficiencyId>,
    // ...
}

pub struct RaceDefinition {
    pub proficiencies: Vec<ProficiencyId>,
    pub incompatible_tags: Vec<String>,
    // ...
}
```

### Benefits of New System

1. **Extensible**: Add new classes/races without changing code
2. **Flexible**: UNION logic allows race-specific proficiencies
3. **Fine-Grained**: Tags provide restrictions beyond class/race
4. **Data-Driven**: All restrictions defined in RON files
5. **No Bit Limits**: Not constrained to 8 classes (old u8 limit)
6. **Clearer Intent**: "proficiency_martial_melee" is more readable than bit 1

### Data Migration Notes

- Remove `disablements: (N)` from all items
- Remove `disablement_bit: N` from all classes and races
- Add `proficiencies: []` to classes and races
- Add `incompatible_tags: []` to races
- Add classification to all weapon/armor items
- Add `tags: []` and `alignment_restriction: None` to items (optional)

## Testing Strategy

### Unit Tests

- Classification to proficiency ID mapping
- UNION logic: class grants, race grants, neither grants
- Tag incompatibility overrides proficiency
- Alignment restriction enforcement

### Integration Tests

See `tests/proficiency_integration_test.rs` for comprehensive tests:

- `test_proficiency_union_class_grants` - Class grants proficiency
- `test_proficiency_union_race_grants` - Race grants proficiency (Elf + longbow)
- `test_proficiency_union_neither_grants` - Neither grants (failure case)
- `test_race_incompatible_tags` - Race tags block usage (Halfling + greatsword)
- `test_proficiency_overrides_race_tag` - Verify proficiency does NOT override tags

### Editor Testing

- SDK editors show classification dropdowns (not bitmask checkboxes)
- CLI editors prompt for proficiency IDs (not bit indices)
- Data validation rejects invalid proficiency IDs and tags

## Implementation References

**Domain Types:**
- `src/domain/items/types.rs` - Classification enums
- `src/domain/proficiency.rs` - ProficiencyDefinition, ProficiencyDatabase
- `src/domain/classes.rs` - ClassDefinition with proficiencies
- `src/domain/races.rs` - RaceDefinition with proficiencies and tags

**Editors:**
- `sdk/campaign_builder/src/items_editor.rs` - SDK item editor UI
- `src/bin/item_editor.rs` - CLI item editor
- `src/bin/class_editor.rs` - CLI class editor
- `src/bin/race_editor.rs` - CLI race editor

**Data Files:**
- `data/items.ron` - Item definitions with classifications
- `data/classes.ron` - Class definitions with proficiencies
- `data/races.ron` - Race definitions with proficiencies and tags
- `data/proficiencies.ron` - Proficiency definitions

**Tests:**
- `tests/proficiency_integration_test.rs` - Integration tests
- `src/domain/items/types.rs` - Unit tests for classification
- `src/domain/proficiency.rs` - Unit tests for proficiency resolution

## Future Enhancements

1. **Dynamic Proficiency Definitions**: Allow campaigns to define custom proficiencies beyond the standard 11
2. **Proficiency Levels**: Add proficiency ranks (Novice, Expert, Master) for nuanced restrictions
3. **Conditional Tags**: Tags that apply only in certain contexts (e.g., `underwater`, `mounted`)
4. **Proficiency XP**: Characters gain proficiencies through use
5. **Multi-Proficiency Items**: Items that require multiple proficiencies (e.g., magic sword requires both martial and arcane)

## Related Documentation

- `docs/reference/architecture.md` - Section 4.5 (Item System)
- `docs/explanation/implementations.md` - Phase 6 completion summary
- `docs/explanation/phase5_cli_editors_implementation.md` - CLI editor updates
- `docs/explanation/quality_of_life_improvements.md` - SDK editor updates

## Summary

The proficiency system provides a robust, extensible foundation for item restrictions that replaces the inflexible bitmask approach. By combining classification-based proficiency requirements with fine-grained item tags and UNION resolution logic, the system supports complex scenarios (like race-specific proficiencies) while remaining simple to understand and maintain.

**Key Takeaway**: If you can answer "Does the class OR race grant this proficiency?" and "Is the race incompatible with any of the item's tags?", you can determine if a character can use an item.
