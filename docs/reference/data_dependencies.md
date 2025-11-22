# Data Dependencies Reference

**Purpose**: This document provides a complete reference of available Monster IDs and Item IDs for use in map creation.

**Last Updated**: 2025-01-08

**Source Files**:
- `data/monsters.ron` - Monster definitions
- `data/items.ron` - Item definitions

---

## Monster IDs (MonsterId)

### Available Monsters by ID

| ID | Name | HP | Category | Notes |
|----|------|----|----|-------|
| 1 | Goblin | 8 | Weak | Common enemy, 1d4 damage |
| 2 | Kobold | 5 | Weak | Fast, weak defense |
| 3 | Giant Rat | 3 | Weak | Disease attack |
| 10 | Orc | 25 | Medium | 1d8+2 damage, drops items |
| 11 | Skeleton | 20 | Medium | Undead, cold/paralysis resistant |
| 12 | Wolf | 18 | Medium | Fast (speed 14) |
| 20 | Ogre | 60 | Strong | Regenerates, 2d6+3 damage |
| 21 | Zombie | 35 | Strong | Undead, disease attack |
| 22 | Fire Elemental | 70 | Strong | Fire attacks, magic resistant |
| 30 | Dragon | 200 | Boss | Physical + fire breath |
| 31 | Lich | 150 | Boss | Undead caster, drain attack |

### Monster ID Gaps

**Missing IDs for Map Planning**:
- IDs 4-9: Not defined
- IDs 13-19: Not defined
- IDs 23-29: Not defined
- IDs 32+: Not defined

### Usage in Map Events

```ron
// Encounter with 2 Goblins
Position(x: 10, y: 10): Encounter(
    monster_group: [1, 1],
),

// Mixed group: 2 Wolves + 1 Orc
Position(x: 15, y: 5): Encounter(
    monster_group: [12, 12, 10],
),

// Boss encounter: Dragon
Position(x: 25, y: 25): Encounter(
    monster_group: [30],
),
```

### Recommended Encounters by Map Type

**Starter Town** (Safe Zone):
- No encounters recommended

**Starter Dungeon** (Low-level):
- Goblins (ID 1) - groups of 2-3
- Kobolds (ID 2) - groups of 2-4
- Giant Rats (ID 3) - groups of 3-5
- Mixed: `[1, 1, 2]` or `[3, 3, 3]`

**Forest Area** (Mid-level):
- Wolves (ID 12) - groups of 2-3
- Goblins (ID 1) - large groups of 4-6
- Orc (ID 10) - solo or pairs
- Mixed: `[12, 12, 10]` or `[1, 1, 1, 12]`

**Advanced Dungeons**:
- Skeletons (ID 11) - groups of 2-3
- Zombies (ID 21) - groups of 1-2
- Ogre (ID 20) - solo or pairs
- Fire Elemental (ID 22) - solo

**Boss Areas**:
- Dragon (ID 30) - solo
- Lich (ID 31) - solo or with undead minions

---

## Item IDs (ItemId)

### Available Items by ID

#### Basic Weapons (IDs 1-9)
| ID | Name | Type | Damage | Cost | Notes |
|----|------|------|--------|------|-------|
| 1 | Club | Weapon | 1d3 | 1g | All classes |
| 2 | Dagger | Weapon | 1d4 | 2g | All classes |
| 3 | Short Sword | Weapon | 1d6 | 10g | Not Sorcerer |
| 4 | Long Sword | Weapon | 1d8 | 15g | KPAR only |
| 5 | Mace | Weapon | 1d6 | 8g | KPACR |
| 6 | Battle Axe | Weapon | 1d8 | 15g | KPAR only |
| 7 | Two-Handed Sword | Weapon | 2d6 | 30g | KPAR only |

#### Magical Weapons (IDs 10-19)
| ID | Name | Type | Bonus | Cost | Notes |
|----|------|------|-------|------|-------|
| 10 | Club +1 | Weapon | +1 | 30g | All classes |
| 11 | Flaming Sword | Weapon | +3 | 500g | Fire resist +20, spell charges |
| 12 | Accurate Sword | Weapon | +6 | 6500g | Accuracy +6, good aligned |

#### Basic Armor (IDs 20-29)
| ID | Name | Type | AC Bonus | Cost | Notes |
|----|------|------|----------|------|-------|
| 20 | Leather Armor | Armor | +2 | 5g | All classes |
| 21 | Chain Mail | Armor | +5 | 200g | KPACR |
| 22 | Plate Mail | Armor | +8 | 600g | KPC only |

#### Magical Armor (IDs 30-39)
| ID | Name | Type | AC Bonus | Cost | Notes |
|----|------|------|----------|------|-------|
| 30 | Chain Mail +1 | Armor | +6 | 500g | Fire resist +5 |
| 31 | Dragon Scale Mail | Armor | +10 | 5000g | Fire resist +50 |

#### Accessories (IDs 40-49)
| ID | Name | Type | Effect | Cost | Notes |
|----|------|------|--------|------|-------|
| 40 | Ring of Protection | Ring | AC +2 | 100g | All classes |
| 41 | Amulet of Might | Amulet | Might +3 | 500g | All classes |
| 42 | Belt of Speed | Belt | Speed +5 | 750g | All classes |

#### Consumables (IDs 50-59)
| ID | Name | Type | Effect | Cost | Notes |
|----|------|------|--------|------|-------|
| 50 | Healing Potion | Consumable | Heal 20 HP | 50g | Combat usable |
| 51 | Magic Potion | Consumable | Restore 10 SP | 100g | Non-combat |
| 52 | Cure Poison Potion | Consumable | Cure poison | 75g | Combat usable |

#### Ammunition (IDs 60-69)
| ID | Name | Type | Quantity | Cost | Notes |
|----|------|------|----------|------|-------|
| 60 | Arrows | Ammo | 20 | 5g | All classes |
| 61 | Crossbow Bolts | Ammo | 20 | 7g | All classes |

#### Quest Items (IDs 100+)
| ID | Name | Type | Notes |
|----|------|------|-------|
| 100 | Ruby Whistle | Quest | Luck +2, spell charges |
| 101 | Mace of Undead | Weapon (Cursed) | Evil/neutral only, cursed |

### Item ID Gaps

**Missing IDs for Map Planning**:
- IDs 8-9: Not defined
- IDs 13-19: Not defined
- IDs 23-29: Not defined
- IDs 32-39: Not defined
- IDs 43-49: Not defined
- IDs 53-59: Not defined
- IDs 62-99: Not defined
- IDs 102+: Not defined

### Usage in Map Events

```ron
// Starter treasure: Basic equipment
Position(x: 5, y: 5): Treasure(
    loot: [1, 2, 20], // Club, Dagger, Leather Armor
),

// Mid-level treasure: Better gear
Position(x: 10, y: 10): Treasure(
    loot: [4, 21, 50], // Long Sword, Chain Mail, Healing Potion
),

// Magic treasure: Rare items
Position(x: 15, y: 15): Treasure(
    loot: [11, 30, 40], // Flaming Sword, Chain Mail +1, Ring of Protection
),

// Consumable cache
Position(x: 20, y: 20): Treasure(
    loot: [50, 50, 51, 52], // Healing potions, magic potion, cure poison
),
```

### Recommended Treasure by Map Type

**Starter Town**:
- Basic equipment: `[1, 2, 20]` (Club, Dagger, Leather Armor)
- Starter consumables: `[50, 60]` (Healing Potion, Arrows)
- Low value: 1-20 gold worth

**Starter Dungeon**:
- Basic weapons: `[3, 5]` (Short Sword, Mace)
- Basic armor: `[20, 21]` (Leather, Chain Mail)
- Consumables: `[50, 52]` (Healing, Cure Poison)
- Mid value: 20-100 gold worth

**Forest Area**:
- Better weapons: `[4, 6]` (Long Sword, Battle Axe)
- Better armor: `[21, 22]` (Chain Mail, Plate Mail)
- Magic item chance: `[10, 40]` (Club +1, Ring of Protection)
- Good value: 100-500 gold worth

**Advanced Dungeons**:
- Magic weapons: `[11, 12]` (Flaming Sword, Accurate Sword)
- Magic armor: `[30, 31]` (Chain Mail +1, Dragon Scale)
- Magic accessories: `[40, 41, 42]` (All accessories)
- High value: 500-5000 gold worth

---

## Map Planning Reference

### Corrected Examples for Plan

Based on available data, here are **corrected** examples for the map implementation plan:

#### Starter Dungeon Encounters (Task 3.1)
```ron
// Original plan used ID 2 for "Orc" - but ID 2 is Kobold!
// Corrected:

Position(x: 10, y: 10): Encounter(
    monster_group: [1, 1], // 2 Goblins (ID 1) ✓
),

Position(x: 15, y: 5): Encounter(
    monster_group: [10], // 1 Orc (ID 10, not 2!) ✓
),

Position(x: 8, y: 12): Encounter(
    monster_group: [2, 2, 2], // 3 Kobolds ✓
),

Position(x: 18, y: 18): Encounter(
    monster_group: [11, 11], // 2 Skeletons (harder) ✓
),
```

#### Starter Dungeon Treasure (Task 3.1)
```ron
// Original plan used IDs 5-7
// These exist but may not be ideal for starter dungeon
// Corrected to better progression:

Position(x: 18, y: 18): Treasure(
    loot: [3, 21, 50], // Short Sword, Chain Mail, Healing Potion ✓
),

Position(x: 5, y: 15): Treasure(
    loot: [10, 52], // Club +1, Cure Poison Potion ✓
),
```

#### Forest Area Encounters (Task 3.2)
```ron
// Original plan used IDs 3 for "Wolf" and 4 for "Bear"
// Corrected: ID 3 is Giant Rat, ID 12 is Wolf, no Bear exists!

Position(x: 5, y: 5): Encounter(
    monster_group: [12, 12], // 2 Wolves (ID 12, not 3!) ✓
),

Position(x: 15, y: 10): Encounter(
    monster_group: [10], // 1 Orc (no Bear available) ✓
),

Position(x: 20, y: 20): Encounter(
    monster_group: [1, 1, 1], // 3 Goblins ✓
),

Position(x: 25, y: 15): Encounter(
    monster_group: [12, 10], // Wolf + Orc mixed ✓
),
```

#### Forest Area Treasure (Task 3.2)
```ron
// Original used IDs 10-11 (exists but high value)
// Better progression:

Position(x: 25, y: 25): Treasure(
    loot: [4, 40, 50], // Long Sword, Ring of Protection, Healing Potion ✓
),
```

---

## Validation Checklist for Map Creators

When creating maps, verify:

- [ ] All Monster IDs in Encounter events exist in the table above
- [ ] All Item IDs in Treasure events exist in the table above
- [ ] Monster difficulty matches map type (weak → medium → strong → boss)
- [ ] Treasure value matches map type (starter → mid → advanced)
- [ ] No references to IDs in "Missing IDs" sections
- [ ] Encounter groups are reasonable sizes (1-6 monsters typically)
- [ ] Treasure contains 1-5 items typically

---

## Notes

**Class Codes** (for item disablements):
- K = Knight
- P = Paladin
- A = Archer
- C = Cleric
- S = Sorcerer
- R = Robber

**Alignment Codes**:
- 64 = Good alignment required
- Other flags for evil/neutral

**Type Aliases**:
- `MonsterId = u8` (max value: 255)
- `ItemId = u8` (max value: 255)

---

**This document should be consulted when planning any map with encounters or treasure.**
