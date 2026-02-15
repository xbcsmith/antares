<!-- SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Monster-to-Creature Mapping Reference

**Purpose**: Quick reference for all monster-to-creature visual mappings in the tutorial campaign.

**Last Updated**: Phase 2 Implementation
**Format**: RON-based creature registry with corresponding monster definitions

---

## Quick Lookup Table

| Monster ID | Monster Name   | Creature ID | Status | Asset File           |
| :--------: | :------------- | :---------: | :----- | :------------------- |
|     1      | Goblin         |      1      | ✅     | `goblin.ron`         |
|     2      | Kobold         |      2      | ✅     | `kobold.ron`         |
|     3      | Giant Rat      |      3      | ✅     | `giant_rat.ron`      |
|     10     | Orc            |     10      | ✅     | `orc.ron`            |
|     11     | Skeleton       |     11      | ✅     | `skeleton.ron`       |
|     12     | Wolf           |     12      | ✅     | `wolf.ron`           |
|     20     | Ogre           |     20      | ✅     | `ogre.ron`           |
|     21     | Zombie         |     21      | ✅     | `zombie.ron`         |
|     22     | Fire Elemental |     22      | ✅     | `fire_elemental.ron` |
|     30     | Dragon         |     30      | ✅     | `dragon.ron`         |
|     31     | Lich           |     31      | ✅     | `lich.ron`           |

## Mapping Strategy

The tutorial campaign uses **1:1 exact ID matching** where `Monster ID = Creature ID`. This strategy:

- Eliminates ID lookup complexity
- Makes relationships obvious in data files
- Reduces potential for mapping errors
- Simplifies content auditing
- Enables quick visual identification

## Variant Creatures (Available for Future Use)

| Creature ID | Creature Name   | Base Monster  | Use Case               | Asset File             |
| :---------: | :-------------- | :------------ | :--------------------- | :--------------------- |
|     32      | RedDragon       | Dragon (30)   | Fire dragon variant    | `red_dragon.ron`       |
|     33      | PyramidDragon   | Dragon (30)   | Ancient dragon variant | `pyramid_dragon.ron`   |
|     151     | DyingGoblin     | Goblin (1)    | Wounded/fleeing state  | `dying_goblin.ron`     |
|     152     | SkeletonWarrior | Skeleton (11) | Elite skeleton         | `skeleton_warrior.ron` |
|     153     | EvilLich        | Lich (31)     | Lich boss encounter    | `evil_lich.ron`        |

**Usage**: Create new monster definitions with enhanced stats and assign `visual_id` to variant creature ID for elite encounters.

## Data Structure

### Monster Definition Field

```ron
visual_id: Some({creature_id})
```

### Creature Registry Entry

```ron
CreatureReference(
    id: {creature_id},
    name: "{Creature Name}",
    filepath: "assets/creatures/{file_name}.ron",
)
```

## Implementation Guide

### Adding a New Monster Mapping

1. Create monster definition with `visual_id: Some({id})`
2. Verify creature exists in `campaigns/tutorial/data/creatures.ron`
3. Run validation:
   ```bash
   cargo nextest run --test tutorial_monster_creature_mapping
   ```

### Creating a Variant Encounter

```ron
(
    id: 200,
    name: "Lich Lord",
    stats: (/* enhanced stats */),
    // ... other fields ...
    visual_id: Some(153),  // Uses EvilLich visual
)
```

## Validation Rules

1. **Creature Existence**: Every `visual_id` must reference an existing creature
2. **No Null References**: Tutorial monsters must have `visual_id` set
3. **Unique Monster IDs**: Each monster must have unique `id`
4. **File Path Validity**: Referenced creature filepath must exist

## Validation Tests

Run all mapping tests:

```bash
cargo nextest run --test tutorial_monster_creature_mapping
```

Tests verify:

- ✅ All 11 monsters have visual_id set
- ✅ All visual_id values reference existing creatures
- ✅ No broken references
- ✅ Creature database loads successfully

## ID Space Allocation

|  Range  | Purpose            | Allocated/Total | Status    |
| :-----: | ------------------ | :-------------: | --------- |
|  1-50   | Monster Creatures  |      11/50      | Active    |
| 51-100  | NPC Creatures      |    Reserved     | Phase 3   |
| 101-150 | Template Creatures |      3/50       | Reserved  |
| 151-200 | Variant Creatures  |      5/50       | Available |

## File References

| File                                         | Purpose                 |
| -------------------------------------------- | ----------------------- |
| `campaigns/tutorial/data/monsters.ron`       | Monster definitions     |
| `campaigns/tutorial/data/creatures.ron`      | Creature registry       |
| `assets/creatures/*.ron`                     | Visual mesh definitions |
| `tests/tutorial_monster_creature_mapping.rs` | Integration tests       |
| `src/domain/combat/database.rs`              | Monster database        |
| `src/domain/visual/creature_database.rs`     | Creature database       |

## Related Documentation

- **Phase 2 Implementation**: `docs/explanation/phase2_monster_visual_mapping.md`
- **Procedural Mesh Integration Plan**: `docs/explanation/tutorial_procedural_mesh_integration_plan.md`
- **Architecture Reference**: `docs/reference/architecture.md` (Sections 4.4, 7.1-7.2)

## Status

✅ **Phase 2 Complete**

All 11 tutorial monsters mapped and validated. Ready for Phase 3: NPC Procedural Mesh Integration.

---

**Last Verified**: All tests passing  
**Test Count**: 4/4 passing  
**Version**: 1.0
