# Tutorial Campaign Creature Mappings

This document provides a complete reference of all creature visual mappings used in the tutorial campaign. Use this as a quick lookup for creature IDs, assignments, and availability.

## Table of Contents

- [Monster-to-Creature Mappings](#monster-to-creature-mappings)
- [NPC-to-Creature Mappings](#npc-to-creature-mappings)
- [Creature ID Assignment Ranges](#creature-id-assignment-ranges)
- [Available Unused Creatures](#available-unused-creatures)
- [Guidelines for Adding New Creatures](#guidelines-for-adding-new-creatures)

## Monster-to-Creature Mappings

All 11 tutorial campaign monsters have procedural mesh visuals assigned.

| Monster ID | Monster Name   | Creature ID | Creature Name  | Creature File          | Status     |
|-----------|----------------|-------------|----------------|------------------------|------------|
| 1         | Goblin         | 1           | Goblin         | goblin.ron             | ✅ Assigned |
| 2         | Kobold         | 2           | Kobold         | kobold.ron             | ✅ Assigned |
| 3         | Giant Rat      | 3           | GiantRat       | giant_rat.ron          | ✅ Assigned |
| 4         | Orc            | 10          | Orc            | orc.ron                | ✅ Assigned |
| 5         | Skeleton       | 11          | Skeleton       | skeleton.ron           | ✅ Assigned |
| 6         | Wolf           | 12          | Wolf           | wolf.ron               | ✅ Assigned |
| 7         | Ogre           | 20          | Ogre           | ogre.ron               | ✅ Assigned |
| 8         | Zombie         | 21          | Zombie         | zombie.ron             | ✅ Assigned |
| 9         | Fire Elemental | 22          | FireElemental  | fire_elemental.ron     | ✅ Assigned |
| 10        | Dragon         | 30          | Dragon         | dragon.ron             | ✅ Assigned |
| 11        | Lich           | 31          | Lich           | lich.ron               | ✅ Assigned |

**Summary**: 11/11 monsters have creature visuals (100% coverage)

## NPC-to-Creature Mappings

All 12 NPCs have procedural mesh visuals assigned.

| NPC ID                           | NPC Name                    | Creature ID | Creature Name       | Creature File           | Shared |
|----------------------------------|-----------------------------|-----------  |---------------------|-------------------------|--------|
| tutorial_elder_village           | Village Elder Town Square   | 51          | VillageElder        | village_elder.ron       | Yes (2×)|
| tutorial_innkeeper_town          | InnKeeper Town Square       | 52          | Innkeeper           | innkeeper.ron           | Yes (2×)|
| tutorial_merchant_town           | Merchant Town Square        | 53          | Merchant            | merchant.ron            | Yes (2×)|
| tutorial_priestess_town          | High Priestess Town Square  | 55          | HighPriestess       | high_priestess.ron      | No     |
| tutorial_wizard_arcturus         | Arcturus                    | 56          | WizardArcturus      | wizard_arcturus.ron     | No     |
| tutorial_wizard_arcturus_brother | Arcturus Brother            | 58          | OldGareth           | old_gareth.ron          | No     |
| tutorial_ranger_lost             | Lost Ranger                 | 57          | Ranger              | ranger.ron              | No     |
| tutorial_elder_village2          | Village Elder Mountain Pass | 51          | VillageElder        | village_elder.ron       | Yes (2×)|
| tutorial_innkeeper_town2         | Innkeeper Mountain Pass     | 52          | Innkeeper           | innkeeper.ron           | Yes (2×)|
| tutorial_merchant_town2          | Merchant Mountain Pass      | 53          | Merchant            | merchant.ron            | Yes (2×)|
| tutorial_priest_town2            | High Priest Mountain Pass   | 54          | HighPriest          | high_priest.ron         | No     |
| tutorial_goblin_dying            | Dying Goblin                | 151         | DyingGoblin         | dying_goblin.ron        | No     |

**Summary**: 12 NPCs using 9 unique creature visuals

**Reused Creatures**:
- Creature ID 51 (VillageElder): 2 NPCs
- Creature ID 52 (Innkeeper): 2 NPCs
- Creature ID 53 (Merchant): 2 NPCs

## Creature ID Assignment Ranges

The tutorial campaign uses the following ID ranges for organization:

| Range      | Purpose                        | Count Used | Count Available | Notes                              |
|------------|--------------------------------|------------|-----------------|------------------------------------|
| 1-50       | Monster Creatures              | 11         | 39              | Combat enemies, hostile creatures  |
| 51-100     | NPC Creatures                  | 9          | 41              | Townspeople, quest givers          |
| 101-150    | Template Creatures             | 3          | 47              | Character creation examples        |
| 151-200    | Variant Creatures              | 3          | 47              | Elite monsters, boss variants      |

**Total Registered**: 32 creatures
**Total Used**: 20 unique creatures (11 monster + 9 NPC)
**Total Unused**: 12 creatures

## Available Unused Creatures

These creatures are registered in `data/creatures.ron` but not currently assigned to any monster or NPC. They are available for future content expansion.

### Monster Variant Creatures (Available for Elite Encounters)

| Creature ID | Creature Name      | Creature File           | Suggested Use                  |
|-------------|--------------------|-------------------------|--------------------------------|
| 32          | RedDragon          | red_dragon.ron          | Fire dragon boss encounter     |
| 33          | PyramidDragon      | pyramid_dragon.ron      | Ancient dragon boss            |
| 152         | SkeletonWarrior    | skeleton_warrior.ron    | Elite skeleton variant         |
| 153         | EvilLich           | evil_lich.ron           | Boss lich variant              |

### Character/NPC Creatures (Available for Future NPCs)

| Creature ID | Creature Name      | Creature File           | Suggested Use                  |
|-------------|--------------------|-------------------------|--------------------------------|
| 59          | ApprenticeZara     | apprentice_zara.ron     | Apprentice wizard NPC          |
| 60          | Kira               | kira.ron                | Character NPC                  |
| 61          | Mira               | mira.ron                | Character NPC                  |
| 62          | Sirius             | sirius.ron              | Character NPC                  |
| 63          | Whisper            | whisper.ron             | Character NPC                  |

### Template Creatures (Available as Examples)

| Creature ID | Creature Name          | Creature File                | Suggested Use                  |
|-------------|------------------------|------------------------------|--------------------------------|
| 101         | TemplateHumanFighter   | template_human_fighter.ron   | Fighter character template     |
| 102         | TemplateElfMage        | template_elf_mage.ron        | Mage character template        |
| 103         | TemplateDwarfCleric    | template_dwarf_cleric.ron    | Cleric character template      |

**Recommendations**:
- Use variant creatures (32, 33, 152, 153) for boss encounters or elite enemy spawns
- Use character creatures (59-63) when adding new quest-giving NPCs
- Use template creatures (101-103) as visual references for character creation

## Guidelines for Adding New Creatures

### Step 1: Choose an Appropriate ID

Follow the ID range conventions:
- **1-50**: Combat monsters
- **51-100**: Friendly NPCs
- **101-150**: Templates/examples
- **151-200**: Variants/bosses
- **201+**: Custom campaign content

### Step 2: Create the Creature Definition File

Create a new `.ron` file in `assets/creatures/`:

```ron
// assets/creatures/my_new_creature.ron
CreatureDefinition(
    id: 201,  // Use next available ID in appropriate range
    name: "MyNewCreature",
    meshes: [
        // Define mesh primitives (cubes, spheres, cylinders, cones)
        // Example:
        // MeshDefinition(
        //     vertices: [...],
        //     indices: [...],
        //     normals: [...],
        //     uvs: [...],
        //     color: (1.0, 0.5, 0.0, 1.0),  // Orange
        // ),
    ],
    mesh_transforms: [
        // Position each mesh
        // MeshTransform(
        //     translation: (0.0, 1.0, 0.0),
        //     rotation: (0.0, 0.0, 0.0, 1.0),
        //     scale: (1.0, 1.0, 1.0),
        // ),
    ],
    scale: 1.0,
    color_tint: (1.0, 1.0, 1.0, 1.0),  // White (no tint)
)
```

### Step 3: Register in Creature Registry

Add an entry to `data/creatures.ron`:

```ron
CreatureReference(
    id: 201,
    name: "MyNewCreature",
    filepath: "assets/creatures/my_new_creature.ron",
),
```

### Step 4: Assign to Monster or NPC

**For Monsters** (`data/monsters.ron`):
```ron
(
    id: 12,
    name: "My Monster",
    // ... other fields ...
    visual_id: Some(201),  // Link to creature
    // ... other fields ...
)
```

**For NPCs** (`data/npcs.ron`):
```ron
(
    id: "my_npc_id",
    name: "My NPC",
    // ... other fields ...
    creature_id: Some(201),  // Link to creature
    sprite: None,
    // ... other fields ...
)
```

### Step 5: Validate

Run the campaign validator to ensure all references are valid:

```bash
cargo run --bin campaign_validator -- campaigns/tutorial
```

### Best Practices

1. **Naming Convention**: Use PascalCase for creature names (e.g., `SkeletonWarrior`, `FireDragon`)
2. **File Naming**: Use snake_case for filenames (e.g., `skeleton_warrior.ron`, `fire_dragon.ron`)
3. **ID Gaps**: Leave gaps between IDs for future additions (e.g., use 10, 20, 30 instead of 10, 11, 12)
4. **Reuse Creatures**: Multiple NPCs/monsters can share the same creature visual for consistency
5. **Color Tints**: Use `color_tint` to create variants without duplicating mesh definitions
6. **Testing**: Always test new creatures in-game before committing

## Version History

- **v1.0** (2025-01-XX): Initial creature mapping documentation
  - Documented all 32 registered creatures
  - Mapped 11 monsters and 12 NPCs to creature visuals
  - Identified 12 unused creatures available for expansion

## See Also

- `README.md` - Campaign overview and visual assets documentation
- `data/creatures.ron` - Creature registry file
- `data/monsters.ron` - Monster definitions with visual_id mappings
- `data/npcs.ron` - NPC definitions with creature_id mappings
- `assets/creatures/` - Individual creature mesh definition files
