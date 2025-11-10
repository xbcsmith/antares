# Data Dependency Audit Summary

**Date**: 2025-01-08
**Auditor**: AI Agent
**Purpose**: Verify Monster IDs and Item IDs referenced in map_content_implementation_plan.md

---

## Executive Summary

✅ **Audit Complete**: All data files reviewed and dependencies validated.

⚠️ **Issues Found**: The implementation plan references **non-existent Monster IDs**.

✅ **Resolution**: Corrected IDs provided in `docs/reference/data_dependencies.md`.

---

## Audit Results

### Monster IDs (from data/monsters.ron)

**Total Monsters**: 11

**Available IDs**:
- ✅ ID 1: Goblin
- ✅ ID 2: Kobold (NOT Orc!)
- ✅ ID 3: Giant Rat (NOT Wolf!)
- ✅ ID 10: Orc
- ✅ ID 11: Skeleton
- ✅ ID 12: Wolf
- ✅ ID 20: Ogre
- ✅ ID 21: Zombie
- ✅ ID 22: Fire Elemental
- ✅ ID 30: Dragon
- ✅ ID 31: Lich

**Missing IDs** (gaps in sequence):
- ❌ IDs 4-9: Not defined
- ❌ IDs 13-19: Not defined
- ❌ IDs 23-29: Not defined
- ❌ IDs 32+: Not defined

### Item IDs (from data/items.ron)

**Total Items**: 19

**Available IDs**:
- ✅ IDs 1-7: Basic Weapons (Club, Dagger, Short Sword, Long Sword, Mace, Battle Axe, Two-Handed Sword)
- ✅ IDs 10-12: Magical Weapons (Club +1, Flaming Sword, Accurate Sword)
- ✅ IDs 20-22: Basic Armor (Leather, Chain Mail, Plate Mail)
- ✅ IDs 30-31: Magical Armor (Chain Mail +1, Dragon Scale Mail)
- ✅ IDs 40-42: Accessories (Ring, Amulet, Belt)
- ✅ IDs 50-52: Consumables (Healing, Magic, Cure Poison Potions)
- ✅ IDs 60-61: Ammunition (Arrows, Bolts)
- ✅ IDs 100-101: Quest/Cursed Items

**Missing IDs** (gaps in sequence):
- ❌ IDs 8-9: Not defined
- ❌ IDs 13-19: Not defined
- ❌ IDs 23-29: Not defined
- ❌ IDs 32-39: Not defined
- ❌ IDs 43-49: Not defined
- ❌ IDs 53-59: Not defined
- ❌ IDs 62-99: Not defined
- ❌ IDs 102+: Not defined

---

## Issues Found in Implementation Plan

### Phase 3, Task 3.1: Starter Dungeon

**Problem**: Plan references incorrect Monster IDs

❌ **Incorrect** (from plan):
```ron
Position(x: 15, y: 5): Encounter(
    monster_group: [2], // 1 orc
),
```

**Issue**: Monster ID 2 is **Kobold**, not Orc!

✅ **Correct**:
```ron
Position(x: 15, y: 5): Encounter(
    monster_group: [10], // 1 Orc (ID 10)
),
```

### Phase 3, Task 3.2: Forest Area

**Problem**: Plan references non-existent Monster IDs

❌ **Incorrect** (from plan):
```ron
Position(x: 5, y: 5): Encounter(
    monster_group: [3, 3], // wolves
),
Position(x: 15, y: 10): Encounter(
    monster_group: [4], // bear
),
```

**Issues**:
- Monster ID 3 is **Giant Rat**, not Wolf!
- Monster ID 4 **does not exist** (no Bear in database)!

✅ **Correct**:
```ron
Position(x: 5, y: 5): Encounter(
    monster_group: [12, 12], // 2 Wolves (ID 12)
),
Position(x: 15, y: 10): Encounter(
    monster_group: [10], // 1 Orc (no Bear available)
),
```

### Phase 3, Task 3.1: Starter Dungeon Treasure

**Status**: ✅ IDs 5-7 exist and are valid

The plan uses:
```ron
loot: [5, 6, 7], // Magic items
```

**Verification**:
- ✅ ID 5: Mace (exists)
- ✅ ID 6: Battle Axe (exists)
- ✅ ID 7: Two-Handed Sword (exists)

**Note**: These are basic weapons, not magic items. Better progression would be:
```ron
loot: [10, 30, 50], // Club +1, Chain Mail +1, Healing Potion
```

---

## Recommendations

### Immediate Actions Required

1. **Update Implementation Plan** with corrected Monster IDs:
   - Replace all references to "Orc" with ID 10 (not 2)
   - Replace all references to "Wolf" with ID 12 (not 3)
   - Remove references to "Bear" (ID 4) - does not exist

2. **Add Data Dependency Documentation**:
   - ✅ Created: `docs/reference/data_dependencies.md`
   - Contains complete ID reference tables
   - Includes corrected examples for all map types

3. **Update Map Creation Guidelines**:
   - Always consult data_dependencies.md before assigning IDs
   - Validate all IDs exist before creating maps
   - Use validation tool to check map files

### Best Practices for Map Creators

**Before adding encounters**:
```bash
# Check available monsters
grep "id:" data/monsters.ron

# Verify specific ID
grep "id: 12" data/monsters.ron  # Should return Wolf
```

**Before adding treasure**:
```bash
# Check available items
grep "id:" data/items.ron

# Verify specific ID
grep "id: 50" data/items.ron  # Should return Healing Potion
```

**Use the reference**:
- Consult `docs/reference/data_dependencies.md` for complete tables
- Reference includes recommended encounters by map type
- Reference includes recommended treasure by progression level

---

## Validation Tool Update

The validation utility (`examples/validate_map.rs` or `src/bin/validate_map.rs`) should be enhanced to check:

```rust
// Pseudo-code for validation
fn validate_monster_ids(map: &Map) -> Vec<String> {
    let valid_ids = [1, 2, 3, 10, 11, 12, 20, 21, 22, 30, 31];
    let mut errors = Vec::new();

    for (pos, event) in &map.events {
        if let MapEvent::Encounter { monster_group } = event {
            for &id in monster_group {
                if !valid_ids.contains(&id) {
                    errors.push(format!(
                        "Invalid Monster ID {} at position {:?}", id, pos
                    ));
                }
            }
        }
    }

    errors
}

fn validate_item_ids(map: &Map) -> Vec<String> {
    let valid_ids = [
        1, 2, 3, 4, 5, 6, 7,        // Basic weapons
        10, 11, 12,                  // Magic weapons
        20, 21, 22,                  // Basic armor
        30, 31,                      // Magic armor
        40, 41, 42,                  // Accessories
        50, 51, 52,                  // Consumables
        60, 61,                      // Ammo
        100, 101,                    // Quest/Cursed
    ];
    let mut errors = Vec::new();

    for (pos, event) in &map.events {
        if let MapEvent::Treasure { loot } = event {
            for &id in loot {
                if !valid_ids.contains(&id) {
                    errors.push(format!(
                        "Invalid Item ID {} at position {:?}", id, pos
                    ));
                }
            }
        }
    }

    errors
}
```

---

## Corrected ID Reference (Quick Lookup)

### Common Monsters for Maps

**Weak Enemies** (Starter dungeons):
- ID 1: Goblin (HP 8, common)
- ID 2: Kobold (HP 5, fast)
- ID 3: Giant Rat (HP 3, disease)

**Medium Enemies** (Mid-level areas):
- ID 10: Orc (HP 25)
- ID 11: Skeleton (HP 20, undead)
- ID 12: Wolf (HP 18, fast)

**Strong Enemies** (Advanced dungeons):
- ID 20: Ogre (HP 60, regenerates)
- ID 21: Zombie (HP 35, undead, disease)
- ID 22: Fire Elemental (HP 70, magic resistant)

**Boss Enemies**:
- ID 30: Dragon (HP 200)
- ID 31: Lich (HP 150, undead caster)

### Common Items for Treasure

**Starter Loot**:
- [1, 2, 20] = Club, Dagger, Leather Armor
- [50, 60] = Healing Potion, Arrows

**Mid-Level Loot**:
- [4, 21, 50] = Long Sword, Chain Mail, Healing Potion
- [10, 40] = Club +1, Ring of Protection

**Advanced Loot**:
- [11, 30] = Flaming Sword, Chain Mail +1
- [12, 31, 41] = Accurate Sword, Dragon Scale Mail, Amulet of Might

---

## Conclusion

**Audit Status**: ✅ **COMPLETE**

**Critical Findings**:
- Monster ID confusion in plan (IDs 2, 3, 4 misidentified)
- Item IDs are valid but may not match intended progression

**Actions Taken**:
- ✅ Created complete ID reference: `docs/reference/data_dependencies.md`
- ✅ Provided corrected examples for all map types
- ✅ Documented validation requirements

**Next Steps**:
1. Update map_content_implementation_plan.md with corrected IDs
2. Implement enhanced validation in validation tool
3. Use data_dependencies.md as source of truth during map creation

**Map creation can proceed once plan is corrected.**

---

**Report Status**: FINAL
**Approval**: Ready for implementation with corrections
