# Proficiency System Migration Plan

## Overview

This plan details the migration from the current **Disablement Bits** system to a new **Proficiency** system for item usage restrictions in Antares. The current system uses a `u8` bitmask where items declare which classes can use them via numeric flags. The new system inverts this relationship: classes declare what item categories they are proficient in, and items declare what proficiency they require.

**Why migrate?**

- Human-readable data files (no more cryptic `disablements: (43)`)
- Flexible custom campaigns (add new proficiencies without code changes)
- Intuitive game design (fighters are proficient in swords, not "items with bit 0 set")
- Unlimited class expansion (not limited to 8 bits)

## Current State Analysis

### Existing Infrastructure

The disablement system is implemented across multiple layers:

| Layer      | File                                         | Current Implementation                                      |
| ---------- | -------------------------------------------- | ----------------------------------------------------------- |
| Domain     | `src/domain/items/types.rs`                  | `Disablement(pub u8)` struct with hardcoded class constants |
| Domain     | `src/domain/classes.rs`                      | `ClassDefinition.disablement_bit_index: u8` field           |
| SDK Editor | `sdk/campaign_builder/src/items_editor.rs`   | Hardcoded class list in UI                                  |
| SDK Editor | `sdk/campaign_builder/src/classes_editor.rs` | Bit index text field                                        |
| CLI        | `src/bin/class_editor.rs`                    | `get_next_disablement_bit()` auto-allocation                |
| CLI        | `src/bin/item_editor.rs`                     | Flag composition via OR operations                          |
| CLI        | `src/bin/race_editor.rs`                     | Has `disablement_bit` field (unused in data)                |
| Data       | `data/classes.ron`                           | `disablement_bit: 0..5` per class                           |
| Data       | `data/items.ron`                             | `disablements: (255)` numeric masks                         |
| Campaigns  | `campaigns/tutorial/data/*.ron`              | Same patterns                                               |

### Identified Issues

1. **Not human-friendly**: `disablements: (43)` requires binary math to understand
2. **Limited to 8 classes**: `u8` bitmask constrains class count
3. **Hardcoded UI**: Editors have static class lists, not dynamic from data
4. **Inverted logic**: Items define allowed classes instead of classes defining proficiencies
5. **Mixed concerns**: Class bits 0-5 and alignment bits 6-7 in same mask
6. **No validation**: Invalid bit combinations are silently accepted

## Proposed Type Design

### Design Principles

Based on feedback, the design follows these principles:

1. **No proficiency inheritance** - Proficiencies are independent; `martial_melee` does NOT imply `simple_weapon`
2. **Race proficiencies with UNION logic** - Character can use item if EITHER class OR race grants proficiency
3. **Item tags for fine-grained restrictions** - Items have tags (e.g., `large_weapon`), races have incompatible tags
4. **Alignment as separate field** - Not mixed with proficiencies
5. **Item classification** - `ItemType` sub-structs get classification fields that map to proficiency requirements
6. **Tags are arbitrary strings** - No separate tags.ron file; tags are free-form for flexibility

### Proficiency Resolution Logic

```rust
/// Combined check for character (class + race) with UNION logic
fn can_character_use_item(
    class: &ClassDefinition,
    race: &RaceDefinition,
    item: &Item,
) -> bool {
    // 1. Proficiency check: class OR race grants it (UNION)
    let has_proficiency = match item.required_proficiency() {
        None => true,
        Some(prof) => class.proficiencies.contains(&prof) || race.proficiencies.contains(&prof),
    };

    // 2. Item tag check: race must not have incompatible tags
    let race_compatible = !item.tags.iter()
        .any(|tag| race.incompatible_item_tags.contains(tag));

    // 3. Alignment check
    let alignment_ok = check_alignment(character, item);

    has_proficiency && race_compatible && alignment_ok
}
```

**Example Resolution:**

| Character       | Item      | Class Prof?          | Race Prof?                   | Race Compat?              | Result      |
| --------------- | --------- | -------------------- | ---------------------------- | ------------------------- | ----------- |
| Elf Sorcerer    | Long Bow  | NO (martial_ranged)  | YES (elf has martial_ranged) | YES                       | ✓ CAN USE   |
| Halfling Archer | Long Bow  | YES (martial_ranged) | -                            | NO (large_weapon tag)     | ✗ CAN'T USE |
| Halfling Archer | Short Bow | YES                  | -                            | YES (no large_weapon tag) | ✓ CAN USE   |
| Human Sorcerer  | Long Bow  | NO                   | NO                           | YES                       | ✗ CAN'T USE |

### Item Classification System

The key insight is that proficiency requirements should derive from item classification, not be a separate field. Each item type gets a classification enum:

```rust
// src/domain/items/types.rs

/// Weapon classification determines proficiency requirement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponClassification {
    Simple,       // Clubs, daggers, staffs - anyone can use
    MartialMelee, // Swords, axes - fighters, paladins
    MartialRanged,// Bows, crossbows - archers, rangers
    Blunt,        // Maces, hammers - clerics (no edge weapons)
    Unarmed,      // Fists, martial arts - monks
}

/// Armor classification determines proficiency requirement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArmorClassification {
    Light,  // Leather, padded
    Medium, // Chain mail, scale
    Heavy,  // Plate mail, full plate
    Shield, // All shield types
}

/// Magic item classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MagicItemClassification {
    Arcane,  // Wands, arcane scrolls - sorcerers
    Divine,  // Holy symbols, divine scrolls - clerics
    Universal, // Potions, rings - anyone
}
```

### Modified ItemType Sub-Structs

```rust
// src/domain/items/types.rs

pub struct WeaponData {
    pub damage: DiceRoll,
    pub bonus: i8,
    pub hands_required: u8,
    pub classification: WeaponClassification, // NEW - determines proficiency
}

pub struct ArmorData {
    pub ac_bonus: i8,
    pub weight: u16,
    pub classification: ArmorClassification, // NEW
}

pub struct AccessoryData {
    pub slot: AccessorySlot,
    pub classification: Option<MagicItemClassification>, // NEW - None for mundane
}

// Consumables, Ammo, Quest items typically have no proficiency requirement
```

### Item Tags System

Items have arbitrary string tags for fine-grained restrictions beyond proficiency:

```rust
// src/domain/items/types.rs

pub struct Item {
    // ... existing fields ...
    /// Arbitrary tags for fine-grained restrictions (e.g., "large_weapon", "two_handed")
    #[serde(default)]
    pub tags: Vec<String>,
}
```

**Standard Tags (by convention, not enforced):**

- `large_weapon` - Too big for small races (Halfling, Gnome)
- `two_handed` - Requires both hands
- `heavy_armor` - Encumbering armor
- `elven_crafted` - Made by elves (may have bonuses for elves)
- `dwarven_crafted` - Made by dwarves
- `requires_strength` - Needs high strength

### Core Proficiency Types

```rust
// src/domain/proficiency.rs

/// Unique identifier for a proficiency
/// Maps to classification enum variants (e.g., "simple_weapon", "light_armor")
pub type ProficiencyId = String;

/// A proficiency definition loaded from data files
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProficiencyDefinition {
    /// Unique identifier (e.g., "martial_melee")
    pub id: ProficiencyId,
    /// Display name (e.g., "Martial Melee Weapons")
    pub name: String,
    /// Category for UI grouping
    pub category: ProficiencyCategory,
    /// Description for tooltips
    #[serde(default)]
    pub description: String,
}

/// Category for grouping proficiencies in UI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProficiencyCategory {
    Weapon,
    Armor,
    Shield,
    MagicItem,
}

/// Database of all proficiency definitions
pub struct ProficiencyDatabase {
    proficiencies: HashMap<ProficiencyId, ProficiencyDefinition>,
}

impl ProficiencyDatabase {
    /// Get proficiency ID from weapon classification
    pub fn proficiency_for_weapon(classification: WeaponClassification) -> ProficiencyId {
        match classification {
            WeaponClassification::Simple => "simple_weapon".to_string(),
            WeaponClassification::MartialMelee => "martial_melee".to_string(),
            WeaponClassification::MartialRanged => "martial_ranged".to_string(),
            WeaponClassification::Blunt => "blunt_weapon".to_string(),
            WeaponClassification::Unarmed => "unarmed".to_string(),
        }
    }

    /// Get proficiency ID from armor classification
    pub fn proficiency_for_armor(classification: ArmorClassification) -> ProficiencyId {
        match classification {
            ArmorClassification::Light => "light_armor".to_string(),
            ArmorClassification::Medium => "medium_armor".to_string(),
            ArmorClassification::Heavy => "heavy_armor".to_string(),
        }
    }
}
```

### Standard Proficiency IDs

The base game will define these standard proficiencies in `data/proficiencies.ron`:

**Weapon Proficiencies (no inheritance - each is independent):**

- `simple_weapon` - Clubs, daggers, staffs, quarterstaffs (magic users get this only)
- `martial_melee` - Swords, axes, maces, flails (fighters, paladins)
- `martial_ranged` - Bows, crossbows (archers, rangers)
- `blunt_weapon` - Maces, hammers, staffs without edge (clerics)
- `unarmed` - Fists, martial arts (monks)

**Armor Proficiencies:**

- `light_armor` - Leather, padded
- `medium_armor` - Chain mail, scale
- `heavy_armor` - Plate mail, full plate
- `shield` - All shield types

**Magic Item Proficiencies:**

- `arcane_item` - Wands, arcane scrolls
- `divine_item` - Holy symbols, divine scrolls

### Modified Class Definition

```rust
// src/domain/classes.rs (modified)

pub struct ClassDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub hp_die: DiceRoll,
    pub spell_school: Option<SpellSchool>,
    pub is_pure_caster: bool,
    pub spell_stat: Option<SpellStat>,
    // REMOVED: pub disablement_bit_index: u8,
    // NEW:
    pub proficiencies: Vec<ProficiencyId>,
    pub special_abilities: Vec<String>,
    pub starting_weapon_id: Option<ItemId>,
    pub starting_armor_id: Option<ItemId>,
    pub starting_items: Vec<ItemId>,
}
```

### Modified Race Definition

```rust
// src/domain/races.rs (new or modified)

pub struct RaceDefinition {
    pub id: String,
    pub name: String,
    pub stat_modifiers: StatModifiers,
    pub resistances: Resistances,
    pub special_abilities: Vec<String>,
    // REMOVED: pub disablement_bit: u8, (was unused)
    // NEW:
    pub proficiencies: Vec<ProficiencyId>,         // Racial weapon proficiencies (UNION with class)
    pub incompatible_item_tags: Vec<String>,       // Item tags this race CAN'T use (e.g., "large_weapon")
}
```

### Modified Item Definition

```rust
// src/domain/items/types.rs (modified)

pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub item_type: ItemType,  // Now includes classification
    pub base_cost: u32,
    pub sell_cost: u32,
    // REMOVED: pub disablements: Disablement,
    // NEW: proficiency derived from item_type classification
    pub alignment_restriction: Option<AlignmentRestriction>,
    pub tags: Vec<String>,    // NEW: arbitrary tags for fine-grained restrictions
    pub constant_bonus: Option<Bonus>,
    pub temporary_bonus: Option<Bonus>,
    pub spell_effect: Option<SpellId>,
    pub max_charges: u16,
    pub is_cursed: bool,
    pub icon_path: Option<String>,
}

/// Alignment restriction (separate from proficiency)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlignmentRestriction {
    GoodOnly,
    EvilOnly,
}
```

### Proficiency Check Functions

```rust
// src/domain/proficiency.rs

impl Item {
    /// Get the required proficiency for this item based on its classification
    pub fn required_proficiency(&self) -> Option<ProficiencyId> {
        match &self.item_type {
            ItemType::Weapon(data) => Some(ProficiencyDatabase::proficiency_for_weapon(data.classification)),
            ItemType::Armor(data) => Some(ProficiencyDatabase::proficiency_for_armor(data.classification)),
            ItemType::Accessory(data) => data.classification.map(ProficiencyDatabase::proficiency_for_magic_item),
            ItemType::Consumable(_) => None, // Anyone can use potions
            ItemType::Ammo(_) => None,       // Ammo handled by weapon
            ItemType::Quest(_) => None,      // Quest items have no restriction
        }
    }
}

impl ClassDefinition {
    /// Check if this class can use an item based on proficiency
    pub fn can_use_item(&self, item: &Item) -> bool {
        match item.required_proficiency() {
            None => true,
            Some(prof_id) => self.proficiencies.contains(&prof_id),
        }
    }
}

impl RaceDefinition {
    /// Check if this race can use an item based on item tags
    pub fn can_use_item(&self, item: &Item) -> bool {
        // Race can't use items with incompatible tags
        !item.tags.iter().any(|tag| self.incompatible_item_tags.contains(tag))
    }

    /// Check if race grants proficiency for an item
    pub fn has_proficiency(&self, item: &Item) -> bool {
        match item.required_proficiency() {
            None => false,
            Some(prof_id) => self.proficiencies.contains(&prof_id),
        }
    }
}

/// Combined check for character (class + race) using UNION logic
pub fn can_character_use_item(
    class: &ClassDefinition,
    race: &RaceDefinition,
    item: &Item,
) -> bool {
    // 1. Proficiency: class OR race grants it (UNION)
    let has_proficiency = match item.required_proficiency() {
        None => true,
        Some(ref prof) => class.proficiencies.contains(prof) || race.proficiencies.contains(prof),
    };

    // 2. Race compatibility: no incompatible tags
    let race_compatible = race.can_use_item(item);

    has_proficiency && race_compatible
    // Note: alignment check done separately
}
```

### Example Data Files

**New `data/proficiencies.ron`:**

```ron
[
    // Weapon proficiencies (no inheritance - each independent)
    (
        id: "simple_weapon",
        name: "Simple Weapons",
        category: Weapon,
        description: "Basic weapons: clubs, daggers, staffs",
    ),
    (
        id: "martial_melee",
        name: "Martial Melee Weapons",
        category: Weapon,
        description: "Advanced melee: swords, axes, maces",
    ),
    (
        id: "martial_ranged",
        name: "Martial Ranged Weapons",
        category: Weapon,
        description: "Ranged weapons: bows, crossbows",
    ),
    (
        id: "blunt_weapon",
        name: "Blunt Weapons",
        category: Weapon,
        description: "Weapons without edge: maces, hammers, staffs",
    ),
    (
        id: "unarmed",
        name: "Unarmed Combat",
        category: Weapon,
        description: "Martial arts and fist fighting",
    ),
    // Armor proficiencies
    (
        id: "light_armor",
        name: "Light Armor",
        category: Armor,
        description: "Leather, padded armor",
    ),
    (
        id: "medium_armor",
        name: "Medium Armor",
        category: Armor,
        description: "Chain mail, scale armor",
    ),
    (
        id: "heavy_armor",
        name: "Heavy Armor",
        category: Armor,
        description: "Plate mail, full plate",
    ),
    (
        id: "shield",
        name: "Shields",
        category: Shield,
        description: "All shield types",
    ),
    // Magic item proficiencies
    (
        id: "arcane_item",
        name: "Arcane Items",
        category: MagicItem,
        description: "Wands, arcane scrolls",
    ),
    (
        id: "divine_item",
        name: "Divine Items",
        category: MagicItem,
        description: "Holy symbols, divine scrolls",
    ),
]
```

**New `data/races.ron` format:**

```ron
[
    (
        id: "human",
        name: "Human",
        stat_modifiers: (might: 0, intellect: 0, personality: 0, endurance: 0, speed: 0, accuracy: 0, luck: 0),
        resistances: (fire: 0, cold: 0, electricity: 0, poison: 0, energy: 0),
        special_abilities: [],
        proficiencies: [],              // No racial weapon bonuses
        incompatible_item_tags: [],     // No restrictions
    ),
    (
        id: "dwarf",
        name: "Dwarf",
        stat_modifiers: (might: 1, intellect: 0, personality: -1, endurance: 2, speed: -1, accuracy: 0, luck: 0),
        resistances: (fire: 0, cold: 0, electricity: 0, poison: 10, energy: 0),
        special_abilities: ["darkvision", "stonecunning"],
        proficiencies: ["martial_melee"],   // Dwarves are naturally good with axes/hammers
        incompatible_item_tags: [],         // Dwarves can use any size weapon
    ),
    (
        id: "elf",
        name: "Elf",
        stat_modifiers: (might: -1, intellect: 1, personality: 0, endurance: -1, speed: 1, accuracy: 1, luck: 0),
        resistances: (fire: 0, cold: 0, electricity: 0, poison: 0, energy: 5),
        special_abilities: ["darkvision", "keen_senses"],
        proficiencies: ["martial_ranged"],  // Elves are naturally good with bows
        incompatible_item_tags: [],         // No restrictions
    ),
    (
        id: "halfling",
        name: "Halfling",
        stat_modifiers: (might: -1, intellect: 0, personality: 1, endurance: 0, speed: 1, accuracy: 1, luck: 2),
        resistances: (fire: 0, cold: 0, electricity: 0, poison: 5, energy: 0),
        special_abilities: ["lucky", "nimble"],
        proficiencies: ["simple_weapon"],       // Basic weapons only
        incompatible_item_tags: ["large_weapon", "heavy_armor"],  // Too small for large items
    ),
    (
        id: "gnome",
        name: "Gnome",
        stat_modifiers: (might: -1, intellect: 2, personality: 0, endurance: 0, speed: 0, accuracy: 0, luck: 1),
        resistances: (fire: 0, cold: 0, electricity: 0, poison: 0, energy: 5),
        special_abilities: ["darkvision", "gnome_cunning"],
        proficiencies: ["simple_weapon"],       // Basic weapons
        incompatible_item_tags: ["large_weapon", "heavy_armor"],  // Too small for large items
    ),
    (
        id: "half_orc",
        name: "Half-Orc",
        stat_modifiers: (might: 2, intellect: -1, personality: -1, endurance: 1, speed: 0, accuracy: 0, luck: 0),
        resistances: (fire: 0, cold: 0, electricity: 0, poison: 0, energy: 0),
        special_abilities: ["darkvision", "relentless"],
        proficiencies: ["martial_melee"],   // Natural fighters
        incompatible_item_tags: [],         // No restrictions
    ),
]
```

**New `data/classes.ron` format:**

```ron
[
    (
        id: "knight",
        name: "Knight",
        description: "A brave warrior specializing in melee combat",
        hp_die: (count: 1, sides: 10, bonus: 0),
        spell_school: None,
        is_pure_caster: false,
        spell_stat: None,
        proficiencies: [
            "simple_weapon",
            "martial_melee",
            "martial_ranged",
            "light_armor",
            "medium_armor",
            "heavy_armor",
            "shield",
        ],
        special_abilities: ["multiple_attacks", "heavy_armor"],
        starting_weapon_id: None,
        starting_armor_id: None,
        starting_items: [],
    ),
    (
        id: "sorcerer",
        name: "Sorcerer",
        description: "A master of arcane magic",
        hp_die: (count: 1, sides: 4, bonus: 0),
        spell_school: Some(Sorcerer),
        is_pure_caster: true,
        spell_stat: Some(Intellect),
        proficiencies: [
            "simple_weapon",  // Only simple weapons
            "arcane_item",
        ],
        special_abilities: ["arcane_mastery", "spell_penetration"],
        starting_weapon_id: None,
        starting_armor_id: None,
        starting_items: [],
    ),
    (
        id: "cleric",
        name: "Cleric",
        description: "A devoted priest with divine magic",
        hp_die: (count: 1, sides: 6, bonus: 0),
        spell_school: Some(Cleric),
        is_pure_caster: true,
        spell_stat: Some(Personality),
        proficiencies: [
            "simple_weapon",
            "blunt_weapon",   // Clerics use blunt weapons (no edge)
            "light_armor",
            "medium_armor",
            "shield",
            "divine_item",
        ],
        special_abilities: ["turn_undead", "divine_intervention"],
        starting_weapon_id: None,
        starting_armor_id: None,
        starting_items: [],
    ),
    (
        id: "monk",
        name: "Monk",
        description: "A martial artist with inner discipline",
        hp_die: (count: 1, sides: 8, bonus: 0),
        spell_school: None,
        is_pure_caster: false,
        spell_stat: None,
        proficiencies: [
            "simple_weapon",  // Staffs, etc.
            "unarmed",        // Fists and martial arts
        ],
        special_abilities: ["martial_arts", "stunning_fist"],
        starting_weapon_id: None,
        starting_armor_id: None,
        starting_items: [],
    ),
]
```

**New `data/items.ron` format:**

```ron
[
    (
        id: 1,
        name: "Club",
        item_type: Weapon((
            damage: (count: 1, sides: 3, bonus: 0),
            bonus: 0,
            hands_required: 1,
            classification: Simple,
        )),
        base_cost: 1,
        sell_cost: 0,
        alignment_restriction: None,
        tags: [],  // No special tags
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    ),
    (
        id: 4,
        name: "Long Sword",
        item_type: Weapon((
            damage: (count: 1, sides: 8, bonus: 0),
            bonus: 0,
            hands_required: 1,
            classification: MartialMelee,
        )),
        base_cost: 15,
        sell_cost: 7,
        alignment_restriction: None,
        tags: [],  // Standard size sword
        // ...
    ),
    (
        id: 5,
        name: "Mace",
        item_type: Weapon((
            damage: (count: 1, sides: 6, bonus: 0),
            bonus: 0,
            hands_required: 1,
            classification: Blunt,
        )),
        base_cost: 8,
        sell_cost: 4,
        alignment_restriction: None,
        tags: [],
        // ...
    ),
    (
        id: 7,
        name: "Two-Handed Sword",
        item_type: Weapon((
            damage: (count: 2, sides: 6, bonus: 0),
            bonus: 0,
            hands_required: 2,
            classification: MartialMelee,
        )),
        base_cost: 30,
        sell_cost: 15,
        alignment_restriction: None,
        tags: ["two_handed", "large_weapon"],  // Halflings/Gnomes can't use
        // ...
    ),
    (
        id: 15,
        name: "Long Bow",
        item_type: Weapon((
            damage: (count: 1, sides: 8, bonus: 0),
            bonus: 0,
            hands_required: 2,
            classification: MartialRanged,
        )),
        base_cost: 75,
        sell_cost: 35,
        alignment_restriction: None,
        tags: ["two_handed", "large_weapon"],  // Too big for Halflings
        // ...
    ),
    (
        id: 16,
        name: "Short Bow",
        item_type: Weapon((
            damage: (count: 1, sides: 6, bonus: 0),
            bonus: 0,
            hands_required: 2,
            classification: MartialRanged,
        )),
        base_cost: 30,
        sell_cost: 15,
        alignment_restriction: None,
        tags: ["two_handed"],  // No large_weapon tag - Halflings CAN use
        // ...
    ),
    (
        id: 20,
        name: "Leather Armor",
        item_type: Armor((
            ac_bonus: 2,
            weight: 15,
            classification: Light,
        )),
        base_cost: 5,
        sell_cost: 2,
        alignment_restriction: None,
        tags: [],
        // ...
    ),
    (
        id: 22,
        name: "Plate Mail",
        item_type: Armor((
            ac_bonus: 8,
            weight: 50,
            classification: Heavy,
        )),
        base_cost: 600,
        sell_cost: 300,
        alignment_restriction: None,
        tags: ["heavy_armor"],  // Halflings/Gnomes can't use
        // ...
    ),
    (
        id: 50,
        name: "Healing Potion",
        item_type: Consumable((effect: HealHp(20), is_combat_usable: true)),
        base_cost: 50,
        sell_cost: 25,
        alignment_restriction: None,
        tags: [],  // Anyone can use potions
        // ...
    ),
]
```

## Implementation Phases

### Data Files to Update

The following data files will be updated across ALL phases:

**Engine Data (`data/`):**

- `data/proficiencies.ron` - NEW: Standard proficiency definitions
- `data/classes.ron` - Add `proficiencies: [...]`
- `data/races.ron` - Add `proficiencies: [...]` and `incompatible_item_tags: [...]`
- `data/items.ron` - Add `classification` to weapon/armor, add `tags: [...]`

**Tutorial Campaign (`campaigns/tutorial/data/`):**

- `campaigns/tutorial/data/classes.ron` - Same changes as engine data
- `campaigns/tutorial/data/races.ron` - Same changes as engine data
- `campaigns/tutorial/data/items.ron` - Same changes as engine data

### Phase 1: Core Type Definitions

Create the foundational proficiency and classification types without breaking existing code.

#### 1.1 Create Classification Enums

Add to `src/domain/items/types.rs`:

- `WeaponClassification` enum (Simple, MartialMelee, MartialRanged, Blunt, Unarmed)
- `ArmorClassification` enum (Light, Medium, Heavy, Shield)
- `MagicItemClassification` enum (Arcane, Divine, Universal)
- `AlignmentRestriction` enum (GoodOnly, EvilOnly)

#### 1.2 Create Proficiency Module

Create new file `src/domain/proficiency.rs` with:

- `ProficiencyId` type alias
- `ProficiencyCategory` enum
- `ProficiencyDefinition` struct
- `ProficiencyDatabase` struct with `load_from_file()`, `get()`, `all()`, `validate()`
- Helper functions to map classification → proficiency ID
- `can_character_use_item()` function with UNION logic

#### 1.3 Create Proficiency Data File

Create `data/proficiencies.ron` with standard proficiency definitions:

- 5 weapon proficiencies (simple_weapon, martial_melee, martial_ranged, blunt_weapon, unarmed)
- 4 armor proficiencies (light_armor, medium_armor, heavy_armor, shield)
- 2 magic item proficiencies (arcane_item, divine_item)

**Note:** Tags (e.g., `large_weapon`, `heavy_armor`) are NOT defined in a separate file - they are arbitrary strings used directly in items.ron and races.ron.

#### 1.4 Export from Domain Module

Update `src/domain/mod.rs` to export the new proficiency module.

#### 1.5 Testing Requirements

- Unit tests for `ProficiencyDatabase::load_from_file()`
- Unit tests for `ProficiencyDatabase::validate()`
- Tests for duplicate ID detection
- Tests for category filtering
- Tests for classification → proficiency ID mapping

#### 1.6 Deliverables

- [ ] `src/domain/items/types.rs` - Classification enums + `tags: Vec<String>` field added
- [ ] `src/domain/proficiency.rs` - New module with UNION logic
- [ ] `data/proficiencies.ron` - Standard proficiencies (11 total)
- [ ] `src/domain/mod.rs` - Export proficiency module
- [ ] Tests achieving >80% coverage

#### 1.7 Success Criteria

- `cargo check` passes
- `cargo clippy` passes with no warnings
- `cargo test` passes
- `ProficiencyDatabase` can load from RON file
- Classification enums serialize/deserialize correctly

### Phase 2: Class and Race Definition Migration

Add proficiencies to both classes and races while keeping old fields for backward compatibility.

#### 2.1 Update ClassDefinition Struct

Modify `src/domain/classes.rs`:

- Add `proficiencies: Vec<ProficiencyId>` field with `#[serde(default)]`
- Add `can_use_item(&self, item: &Item) -> bool` method
- Keep `disablement_bit_index` temporarily for migration period

#### 2.2 Create/Update RaceDefinition

Create or modify `src/domain/races.rs`:

- Add `proficiencies: Vec<ProficiencyId>` field (racial weapon bonuses, UNION with class)
- Add `incompatible_item_tags: Vec<String>` field (item tags race cannot use)
- Add `can_use_item(&self, item: &Item) -> bool` method (checks tags)
- Add `has_proficiency(&self, item: &Item) -> bool` method
- Remove unused `disablement_bit` field from race editor struct

#### 2.3 Update Database Validation

Add validation in `ClassDatabase::validate()` and `RaceDatabase::validate()`:

- Warn if class has no proficiencies
- Validate proficiency IDs exist in `ProficiencyDatabase`
- Check for deprecated `disablement_bit_index` usage

#### 2.4 Update Data Files

Modify `data/classes.ron`:

- Add `proficiencies: [...]` to each class definition
- Keep `disablement_bit` temporarily

Modify `data/races.ron`:

- Add `proficiencies: []` to each race (racial weapon bonuses)
- Add `incompatible_item_tags: []` to each race (size restrictions, etc.)
- Add racial bonuses (e.g., dwarves with martial_melee, elves with martial_ranged)
- Add restrictions for small races (halflings, gnomes get `["large_weapon", "heavy_armor"]`)

Modify `campaigns/tutorial/data/classes.ron` and `campaigns/tutorial/data/races.ron`:

- Same changes as above

#### 2.5 Testing Requirements

- Unit tests for `ClassDefinition::can_use_item()`
- Unit tests for `RaceDefinition::can_use_item()` (tag checking)
- Unit tests for `RaceDefinition::has_proficiency()`
- Tests for combined `can_character_use_item()` with UNION logic
- Test: Elf Sorcerer CAN use Long Bow (race grants proficiency)
- Test: Halfling Archer CANNOT use Long Bow (large_weapon tag)
- Test: Halfling Archer CAN use Short Bow (no large_weapon tag)
- Integration test loading classes and races with proficiencies

#### 2.6 Deliverables

- [ ] `src/domain/classes.rs` - Modified with proficiencies
- [ ] `src/domain/races.rs` - New/modified with proficiencies
- [ ] `data/classes.ron` - Updated with proficiencies
- [ ] `data/races.ron` - Updated with proficiencies and incompatible_item_tags
- [ ] `campaigns/tutorial/data/classes.ron` - Updated with proficiencies
- [ ] `campaigns/tutorial/data/races.ron` - Updated with proficiencies and incompatible_item_tags

#### 2.7 Success Criteria

- All quality gates pass
- Classes and races load with both old and new fields
- `can_use_item()` works for both class and race checks
- Proficiency UNION logic works (class OR race grants proficiency)
- Item tag restrictions work for races

### Phase 3: Item Definition Migration

Add classification to ItemType sub-structs and derive proficiency from classification.

#### 3.1 Update ItemType Sub-Structs

Modify `src/domain/items/types.rs`:

- Add `classification: WeaponClassification` to `WeaponData`
- Add `classification: ArmorClassification` to `ArmorData`
- Add `classification: Option<MagicItemClassification>` to `AccessoryData`
- Add `alignment_restriction: Option<AlignmentRestriction>` field to `Item`
- Add `tags: Vec<String>` field to `Item` (for fine-grained restrictions)
- Add `required_proficiency(&self) -> Option<ProficiencyId>` method to `Item`
- Keep `disablements: Disablement` temporarily for migration
- Mark `Disablement` struct as `#[deprecated]`

#### 3.2 Create Migration Mapping

Create migration helper to convert old masks to classifications:

| Old Mask Pattern                    | Item Type     | New Classification                      |
| ----------------------------------- | ------------- | --------------------------------------- |
| `255` (all classes)                 | Weapon        | `Simple`                                |
| `63` (all class bits, no alignment) | Weapon        | Based on weapon name/type               |
| `43` (KPAR)                         | Weapon        | `MartialMelee`                          |
| `75` (KPACR)                        | Weapon        | `Blunt` (includes cleric)               |
| `255`                               | Armor (light) | `Light`                                 |
| `75`                                | Armor (chain) | `Medium`                                |
| `11`                                | Armor (plate) | `Heavy`                                 |
| Good bit set                        | Any           | `alignment_restriction: Some(GoodOnly)` |
| Evil bit set                        | Any           | `alignment_restriction: Some(EvilOnly)` |

#### 3.3 Update Item Data Files

Modify `data/items.ron`:

- Add `classification: Simple/MartialMelee/etc.` to weapon data
- Add `classification: Light/Medium/Heavy/Shield` to armor data
- Add `tags: ["large_weapon", "two_handed", etc.]` to appropriate items
- Add `alignment_restriction: None` or `Some(GoodOnly)`/`Some(EvilOnly)` to Item
- Remove numeric `disablements` values

Modify `campaigns/tutorial/data/items.ron`:

- Same changes as above
- Ensure Long Bow has `tags: ["large_weapon", "two_handed"]`
- Ensure Short Bow has `tags: ["two_handed"]` (no large_weapon)
- Ensure Two-Handed Sword has `tags: ["large_weapon", "two_handed"]`
- Ensure Plate Mail has `tags: ["heavy_armor"]`

#### 3.4 Testing Requirements

- Tests for `Item::required_proficiency()` deriving from classification
- Tests for item tags field
- Tests for alignment restrictions
- Tests for items with no requirements (consumables, quest items)
- Tests for each classification type
- Migration script tests

#### 3.5 Deliverables

- [ ] `src/domain/items/types.rs` - Modified ItemType sub-structs with classification + tags
- [ ] `data/items.ron` - Migrated to classification + tags system
- [ ] `campaigns/tutorial/data/items.ron` - Migrated with appropriate tags
- [ ] Migration script/tool (recommended)

#### 3.6 Success Criteria

- All quality gates pass
- Items load correctly with new classification fields
- `Item::required_proficiency()` correctly derives from classification
- Proficiency checks work end-to-end (class + race)

### Phase 4: SDK Editor Updates

Update the campaign builder UI to use classifications and proficiencies instead of bit flags.

#### 4.1 Update Classes Editor

Modify `sdk/campaign_builder/src/classes_editor.rs`:

- Replace `disablement_bit_index` text field with multi-select proficiency picker
- Load proficiencies from `ProficiencyDatabase`
- Display checkboxes grouped by `ProficiencyCategory` (Weapon, Armor, Shield, MagicItem)
- Remove bit allocation logic

#### 4.2 Update Races Editor

Add to `sdk/campaign_builder/src/races_editor.rs` (or create if needed):

- Add proficiency multi-select for racial bonuses (UNION with class)
- Add `incompatible_item_tags` text list editor (comma-separated or multi-line)
- Display common tags as suggestions (large_weapon, heavy_armor, two_handed)
- Display proficiencies grouped by category

#### 4.3 Update Items Editor

Modify `sdk/campaign_builder/src/items_editor.rs`:

- Replace `show_disablement_editor()` with classification dropdown per item type
- Add `WeaponClassification` dropdown for weapons
- Add `ArmorClassification` dropdown for armor
- Add `MagicItemClassification` dropdown for accessories
- Add `tags` list editor (text field, comma-separated)
- Add alignment restriction dropdown (None / Good Only / Evil Only)
- Remove hardcoded class lists
- Show derived proficiency requirement as read-only info
- Show tag suggestions for common tags

#### 4.4 Update Validation

Modify `sdk/campaign_builder/src/validation.rs`:

- Validate class proficiency IDs exist
- Validate race proficiency IDs exist
- Validate classification is set for weapons/armor
- Warn about potential mismatches

#### 4.5 Testing Requirements

- UI tests for proficiency multi-select
- UI tests for classification dropdowns
- UI tests for tag editing
- Tests for alignment restriction
- Validation tests

#### 4.6 Deliverables

- [ ] `sdk/campaign_builder/src/classes_editor.rs` - Proficiency UI
- [ ] `sdk/campaign_builder/src/races_editor.rs` - Race proficiency UI
- [ ] `sdk/campaign_builder/src/items_editor.rs` - Classification UI
- [ ] `sdk/campaign_builder/src/validation.rs` - New validation rules

#### 4.7 Success Criteria

- SDK builds and runs
- Can edit class proficiencies via checkboxes
- Can edit race proficiencies and restrictions
- Can set weapon/armor classification via dropdown
- Shows derived proficiency requirement
- Validation catches invalid proficiency IDs

### Phase 5: CLI Editor Updates

Update command-line editors to use the new system.

#### 5.1 Update Class Editor CLI

Modify `src/bin/class_editor.rs`:

- Remove `get_next_disablement_bit()` function
- Add `select_proficiencies()` function with multi-select menu
- Load available proficiencies from `ProficiencyDatabase`
- Update `add_class()` and `edit_class()` to use proficiencies
- Update `preview_class()` to show proficiencies list

#### 5.2 Update Item Editor CLI

Modify `src/bin/item_editor.rs`:

- Remove flag composition via OR
- Add classification selection menu per item type
- Add tags input (comma-separated)
- Add alignment restriction selection (None/Good/Evil)
- Update preview to show classification, tags, and derived proficiency

#### 5.3 Update Race Editor CLI

Modify `src/bin/race_editor.rs`:

- Remove `disablement_bit` field from local `RaceDefinition` struct
- Add `select_proficiencies()` for racial bonuses
- Add `input_incompatible_tags()` for item tag restrictions
- Update `preview_race()` to show proficiencies and incompatible tags

#### 5.4 Testing Requirements

- CLI editor unit tests
- Round-trip tests (create → save → load → verify)
- Test classification → proficiency derivation display

#### 5.5 Deliverables

- [ ] `src/bin/class_editor.rs` - Proficiency multi-select
- [ ] `src/bin/item_editor.rs` - Classification + tags selection
- [ ] `src/bin/race_editor.rs` - Proficiency + incompatible tags, removed disablement_bit

#### 5.6 Success Criteria

- CLI editors build and run
- Can create/edit classes with proficiencies
- Can create/edit items with classification
- Can create/edit races with proficiencies and restrictions

### Phase 6: Cleanup and Deprecation Removal

Remove deprecated disablement system after migration is complete.

#### 6.1 Remove Deprecated Fields

- Remove `disablement_bit_index` from `ClassDefinition`
- Remove `disablements` field from `Item`
- Remove `Disablement` struct entirely
- Remove `disablement_mask()` method
- Remove `disablement_bit` from race editor local struct

#### 6.2 Update All Data Files

- Remove all `disablement_bit` entries from class RON files
- Remove all `disablements` entries from item RON files
- Verify all weapons have `classification` field
- Verify all armor has `classification` field
- Verify all files use new format

#### 6.3 Update Documentation

- Rename `docs/explanation/disablement_bits.md` → `docs/explanation/proficiency_system.md`
- Update content to describe new proficiency system
- Update `docs/reference/architecture.md` if it references disablement bits
- Update `docs/explanation/implementations.md`
- Document classification → proficiency mapping

#### 6.4 Testing Requirements

- Full test suite passes
- No references to deprecated types in code
- Integration tests with new format only
- Test all classification types

#### 6.5 Deliverables

- [ ] `src/domain/items/types.rs` - Disablement struct removed
- [ ] `src/domain/classes.rs` - disablement_bit_index removed
- [ ] `src/bin/race_editor.rs` - disablement_bit removed
- [ ] All data files using new format only
- [ ] Documentation updated and renamed

#### 6.6 Success Criteria

- `grep -r "disablement" src/` returns no code matches (only docs/comments)
- `grep -r "disablement_bit" data/` returns no matches
- All quality gates pass
- Full test coverage maintained

## File Change Summary

### New Files

| File                                     | Description                                                   |
| ---------------------------------------- | ------------------------------------------------------------- |
| `src/domain/proficiency.rs`              | Proficiency types, database, UNION logic, and check functions |
| `src/domain/races.rs`                    | Race definition with proficiencies (if not exists)            |
| `data/proficiencies.ron`                 | Standard proficiency definitions (11 total)                   |
| `docs/explanation/proficiency_system.md` | New documentation (renamed from disablement_bits.md)          |

### Modified Files

| File                                         | Changes                                                                                                                                                                       |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/mod.rs`                          | Export proficiency and races modules                                                                                                                                          |
| `src/domain/classes.rs`                      | Add `proficiencies: Vec<ProficiencyId>`, remove `disablement_bit_index`                                                                                                       |
| `src/domain/items/types.rs`                  | Add classification enums to WeaponData/ArmorData/AccessoryData, add `tags: Vec<String>`, add `AlignmentRestriction`, add `Item::required_proficiency()`, remove `Disablement` |
| `sdk/campaign_builder/src/classes_editor.rs` | Proficiency multi-select UI grouped by category                                                                                                                               |
| `sdk/campaign_builder/src/races_editor.rs`   | Add proficiency and incompatible_item_tags editors                                                                                                                            |
| `sdk/campaign_builder/src/items_editor.rs`   | Classification dropdown, tags editor, alignment restriction selector                                                                                                          |
| `sdk/campaign_builder/src/validation.rs`     | Proficiency existence validation                                                                                                                                              |
| `src/bin/class_editor.rs`                    | Proficiency multi-select, remove `get_next_disablement_bit()`                                                                                                                 |
| `src/bin/item_editor.rs`                     | Classification + tags selection, remove OR flag composition                                                                                                                   |
| `src/bin/race_editor.rs`                     | Add proficiency + incompatible_item_tags editing, remove `disablement_bit`                                                                                                    |
| `data/classes.ron`                           | New format with proficiencies list                                                                                                                                            |
| `data/races.ron`                             | New format with proficiencies and incompatible_item_tags                                                                                                                      |
| `data/items.ron`                             | New format with classification in ItemType sub-structs + tags                                                                                                                 |
| `campaigns/tutorial/data/classes.ron`        | New format with proficiencies                                                                                                                                                 |
| `campaigns/tutorial/data/races.ron`          | New format with proficiencies and incompatible_item_tags                                                                                                                      |
| `campaigns/tutorial/data/items.ron`          | New format with classification + tags (large_weapon, heavy_armor, etc.)                                                                                                       |

### Deleted/Renamed Files

| File                                   | Action                            |
| -------------------------------------- | --------------------------------- |
| `docs/explanation/disablement_bits.md` | Rename to `proficiency_system.md` |

## Design Decisions (Resolved)

1. **Proficiency inheritance?** → **No inheritance (flat list)**

   - Each proficiency is independent
   - `martial_melee` does NOT imply `simple_weapon`
   - A Monk has `simple_weapon` + `unarmed`, not `martial_melee`
   - A Sorcerer only has `simple_weapon`

2. **Race restrictions?** → **Add race proficiencies + item tags**

   - Races have `proficiencies: Vec<ProficiencyId>` for bonuses (e.g., elves with martial_ranged)
   - Races have `incompatible_item_tags: Vec<String>` for item-level restrictions (e.g., halflings can't use large_weapon)
   - Combined check: `can_character_use_item(class, race, item)` with UNION + tag check

3. **Alignment handling?** → **Separate field**

   - `alignment_restriction: Option<AlignmentRestriction>` on Item
   - `AlignmentRestriction::GoodOnly` or `AlignmentRestriction::EvilOnly`
   - Not mixed with proficiency system

4. **Item classification?** → **Classification in ItemType sub-structs + tags**

   - `WeaponData.classification: WeaponClassification`
   - `ArmorData.classification: ArmorClassification`
   - `AccessoryData.classification: Option<MagicItemClassification>`
   - `Item.tags: Vec<String>` for fine-grained restrictions
   - Proficiency requirement derived via `Item::required_proficiency()`

5. **Tags definition?** → **Arbitrary strings, no separate file**
   - Tags are free-form strings in items.ron and races.ron
   - Common conventions: `large_weapon`, `heavy_armor`, `two_handed`, `elven_crafted`
   - No validation file - maximum flexibility for campaign creators

## Open Questions (Remaining)

1. **Shield handling?** → **Resolved: Armor with Shield classification**

   - Shields use `ArmorClassification::Shield`
   - Proficiency ID: `shield`

2. **Consumable restrictions?** Should some consumables require proficiency (e.g., arcane scrolls)?

   - Option A: All consumables usable by anyone (current design)
   - Option B: Add optional classification to ConsumableData

3. **Migration tooling?** Should we build an automated migration script or manually convert data files?
   - Option A: Manual conversion (fewer files - data/ and campaigns/tutorial/)
   - Option B: Script for future campaign migrations

## Timeline Estimate

| Phase                                      | Duration | Dependencies |
| ------------------------------------------ | -------- | ------------ |
| Phase 1: Core Types + Classification Enums | 2-3 days | None         |
| Phase 2: Class + Race Migration            | 2-3 days | Phase 1      |
| Phase 3: Item Migration (Classification)   | 2-3 days | Phase 1      |
| Phase 4: SDK Editors                       | 2-3 days | Phases 2, 3  |
| Phase 5: CLI Editors                       | 1-2 days | Phases 2, 3  |
| Phase 6: Cleanup                           | 1 day    | Phases 4, 5  |

**Total: 10-15 days**

Note: Timeline increased slightly due to:

- Additional classification enums
- Race proficiency system
- More complex Item::required_proficiency() derivation

## Risk Mitigation

1. **Data loss**: Keep old fields during migration period; don't delete until verified
2. **Breaking campaigns**: Migrate `campaigns/tutorial/` as part of each phase
3. **Editor complexity**: Test editors manually after each phase
4. **Validation gaps**: Add comprehensive validation in Phase 4 before removing old system
