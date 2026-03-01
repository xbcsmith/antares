# Monster-to-Creature Mapping Reference

**Purpose**: Quick reference for all monster-to-creature visual mappings in the tutorial campaign.

**Last Updated**: 2026-02-28
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

## How the Mapping Works

There are **three independent ID spaces** that connect through a shared `CreatureId` (`u32`).

### 1. Monster IDs (`MonsterId = u8`)

Defined in `monsters.ron`. These identify *game-logic* entries — stats, attacks, loot, AI behaviour, etc. Each monster carries an optional `visual_id: Option<CreatureId>` field that is a foreign key into the creature registry. If `None`, the monster has no 3D mesh.

### 2. NPC IDs (`NpcId = String`)

Defined in `npcs.ron`. These use human-readable string keys instead of numbers. Each NPC also carries an optional `creature_id: Option<CreatureId>` that points into the same creature registry.

### 3. Creature Registry (`creatures.ron` → `CreatureId = u32`)

A flat list of `CreatureReference` entries — each has an `id`, a `name`, and a `filepath` to a `.ron` mesh asset. **It is purely a visual asset index.** It knows nothing about game logic (no stats, no loot).

### The Monster Chain

```
Map event: Combat(MonsterSpawn { monster_id: 1, count: 3 })
    │
    ▼  look up in MonsterDatabase
Monster { id: 1, name: "Goblin", visual_id: Some(1), ... }
    │
    ▼  look up in CreatureDatabase (loaded from registry)
CreatureReference { id: 1, filepath: "assets/creatures/goblin.ron" }
    │
    ▼  load mesh asset
CreatureDefinition { meshes: [...], mesh_transforms: [...] }
```

### The NPC Chain

```
Map NpcPlacement { npc_id: "village_elder", position: ... }
    │
    ▼  look up in NpcDatabase by string ID
NpcDefinition { id: "village_elder", creature_id: Some(1000), ... }
    │
    ▼  look up in CreatureDatabase
CreatureReference { id: 1000, filepath: "assets/creatures/villageelder.ron" }
    │
    ▼  load mesh asset
CreatureDefinition { meshes: [...], mesh_transforms: [...] }
```

### Why the Numbers Can Look Confusing

The tutorial monsters happen to have `MonsterId == CreatureId` (both `1` for Goblin, `11` for Skeleton, etc.), but **this is a coincidence of convention, not a structural constraint.** They are different types: `MonsterId` is `u8`, `CreatureId` is `u32`. The `visual_id` field on `Monster` is the explicit cross-reference — not a positional index.

NPCs make this clear: they use string IDs (`"village_elder"`) while their creature visual lands at `CreatureId` 1000, a completely separate number in a separate range.

## Mapping Strategy

The tutorial campaign uses **1:1 exact ID matching** where `Monster ID = Creature ID` for monsters. This strategy:

- Eliminates ID lookup complexity
- Makes relationships obvious in data files
- Reduces potential for mapping errors
- Simplifies content auditing
- Enables quick visual identification

## Variant Creatures (Available for Future Use)

Variant creatures live in the **Variants range (3000–3999)**. Template alternatives (e.g. ancient versions) live in the **Templates range (2000–2999)**.

| Creature ID | Creature Name   | Base Monster  | Use Case               | Asset File               |
| :---------: | :-------------- | :------------ | :--------------------- | :----------------------- |
|     32      | RedDragon       | Dragon (30)   | Fire dragon variant    | `reddragon.ron`          |
|     33      | PyramidDragon   | Dragon (30)   | Ancient dragon variant | `pyramiddragon.ron`      |
|    2021     | AncientWolf     | Wolf (12)     | Ancient wolf template  | `ancientwolf.ron`        |
|    2032     | AncientSkeleton | Skeleton (11) | Ancient skeleton       | `ancientskeleton.ron`    |
|    3000     | DyingGoblin     | Goblin (1)    | Wounded/fleeing state  | `dyinggoblin.ron`        |
|    3001     | SkeletonWarrior | Skeleton (11) | Elite skeleton         | `skeletonwarrior.ron`    |
|    3002     | EvilLich        | Lich (31)     | Lich boss encounter    | `evillich.ron`           |

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
    id: 32,
    name: "Lich Lord",
    stats: (/* enhanced stats */),
    // ... other fields ...
    visual_id: Some(3002),  // Uses EvilLich visual (Variants range)
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

|    Range    | Purpose              | Allocated/Total | Status   |
| :---------: | -------------------- | :-------------: | -------- |
|   1–999     | Monster Creatures    |     14/999      | Active   |
| 1000–1999   | NPC Creatures        |     13/1000     | Active   |
| 2000–2999   | Template Creatures   |      2/1000     | Active   |
| 3000–3999   | Variant Creatures    |      3/1000     | Active   |
|   4000+     | Custom/Campaign      |    Unlimited    | Open     |

NPC `creature_id` values always fall in the 1000–1999 range. Monster `visual_id` values fall in the 1–999 range. The ranges are enforced by `CreatureIdManager` in the SDK.

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
- **Creature ID Manager**: `sdk/campaign_builder/src/creature_id_manager.rs`
- **NPC Definition**: `src/domain/world/npc.rs` (`creature_id` field)
- **Monster Definition**: `src/domain/combat/monster.rs` (`visual_id` field)

## Status

✅ **Phase 2 Complete** — 14 monsters and 13 NPCs mapped and validated.

---

**Last Verified**: 2026-02-28
**Test Count**: 4/4 passing
**Version**: 1.1
