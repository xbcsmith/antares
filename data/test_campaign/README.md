# Tutorial Campaign

A minimal tutorial campaign used by the Antares project for examples, tests,
and developer onboarding. This campaign demonstrates the innkeeper-based inn
system and provides a small set of maps, NPCs, and premade characters.

## Overview

- Campaign ID: `tutorial` (directory name)
- Purpose: Example/tutorial content and test fixture for campaign tooling
- Format: RON data files under `data/` (maps, npcs, characters, etc.)
- Default starting innkeeper: `tutorial_innkeeper_town` (`starting_innkeeper: "tutorial_innkeeper_town"` in `campaign.ron`)

This campaign intentionally uses string-based innkeeper NPC identifiers (NpcId)
for inn references and map `EnterInn` events (e.g., `innkeeper_id: "tutorial_innkeeper_town"`).
NPCs that act as innkeepers are defined in `data/npcs.ron` and have `is_innkeeper: true`.

## Innkeeper Requirements

**MANDATORY**: All NPCs with `is_innkeeper: true` MUST have a `dialogue_id` configured.

- Default template: Dialogue ID `999` (use this template for campaigns under construction).
- Custom dialogues: Must include a party-management option. This can be implemented using either:
  - `OpenInnManagement { innkeeper_id: "<your_innkeeper_id>" }` action on a terminal node, or
  - a node that triggers `TriggerEvent(event_name: "open_inn_party_management")` (the dialogue runtime will open the inn management UI using the dialogue's speaker NPC ID).
- Validation: The SDK validator will report an error if an innkeeper NPC lacks a `dialogue_id`. Use the validator after edits to verify compliance.

### Example Innkeeper Dialogue

See dialogue ID `4` or `9` in `data/dialogues.ron` for reference implementations. The default template (ID `999`) is also provided in this campaign as a starting point.

## Included Content

- `campaign.ron` — campaign metadata (includes `starting_innkeeper`)
- `config.ron` — game configuration for this campaign
- `data/` — campaign data files:
  - `npcs.ron` — NPC definitions (includes `tutorial_innkeeper_town`, `tutorial_innkeeper_town2`)
  - `maps/` — map files with `EnterInn` events referencing innkeeper IDs
  - other data files as needed by the campaign

## How to Run & Validate

- Run the game with this campaign:
  `cargo run --bin antares -- --campaign campaigns/tutorial`

- Validate campaign structure and content:
  `cargo run --bin campaign_validator -- campaigns/tutorial`

The validator checks:

- Required files and directories exist (including this README)
- `starting_innkeeper` is non-empty and references an NPC that has `is_innkeeper: true`
- Map `EnterInn` events reference valid innkeeper NPC IDs
- Cross-file references (maps, NPCs, characters) are consistent

## Notes for Editors

- If you change innkeeper identifiers, update:

  - `campaign.ron` (`starting_innkeeper`)
  - `data/npcs.ron` (ensure the NPC exists and `is_innkeeper: true`)
  - Any `EnterInn` events in `data/maps/` to use the new `innkeeper_id`

- Follow RON formatting conventions and run the SDK validator after edits.

## Visual Assets

### Procedural Mesh System

This campaign uses the Antares procedural mesh system for all creature visuals. Instead of static sprites, monsters and NPCs are rendered using 3D procedural meshes composed of primitive shapes (cubes, spheres, cylinders, cones).

**Benefits**:

- Consistent visual style across all creatures
- Easy to create new variants by modifying mesh composition
- No external art assets required
- Visuals defined in data files (`.ron` format)

### Creature Database

All creature visual definitions are stored in `assets/creatures/*.ron` files. The campaign references these through the creature registry at `data/creatures.ron`.

**Creature ID Ranges**:

- `1-50`: Monster creatures (combat enemies)
- `51-100`: NPC creatures (townspeople, quest givers)
- `101-150`: Template creatures (character creation examples)
- `151-200`: Variant creatures (special versions, bosses)

### How to Add New Creatures

1. **Create the mesh definition file**:

   ```ron
   // assets/creatures/my_creature.ron
   CreatureDefinition(
       id: 201,
       name: "MyCreature",
       meshes: [
           // Add mesh primitives here
       ],
       mesh_transforms: [
           // Position/scale meshes
       ],
       scale: 1.0,
       color_tint: (1.0, 1.0, 1.0, 1.0),
   )
   ```

2. **Register in creature registry** (`data/creatures.ron`):

   ```ron
   CreatureReference(
       id: 201,
       name: "MyCreature",
       filepath: "assets/creatures/my_creature.ron",
   ),
   ```

3. **Link to monster or NPC**:
   - For monsters: Set `visual_id: Some(201)` in `data/monsters.ron`
   - For NPCs: Set `creature_id: Some(201)` in `data/npcs.ron`

### Monster Visuals

All 11 tutorial monsters use procedural mesh creatures:

| Monster ID | Monster Name   | Creature ID | Creature File      |
| ---------- | -------------- | ----------- | ------------------ |
| 1          | Goblin         | 1           | goblin.ron         |
| 2          | Kobold         | 2           | kobold.ron         |
| 3          | Giant Rat      | 3           | giant_rat.ron      |
| 4          | Orc            | 10          | orc.ron            |
| 5          | Skeleton       | 11          | skeleton.ron       |
| 6          | Wolf           | 12          | wolf.ron           |
| 7          | Ogre           | 20          | ogre.ron           |
| 8          | Zombie         | 21          | zombie.ron         |
| 9          | Fire Elemental | 22          | fire_elemental.ron |
| 10         | Dragon         | 30          | dragon.ron         |
| 11         | Lich           | 31          | lich.ron           |

### NPC Visuals

All 12 NPCs use procedural mesh creatures:

| NPC ID                           | NPC Name                    | Creature ID | Creature File       |
| -------------------------------- | --------------------------- | ----------- | ------------------- |
| tutorial_elder_village           | Village Elder Town Square   | 51          | village_elder.ron   |
| tutorial_innkeeper_town          | InnKeeper Town Square       | 52          | innkeeper.ron       |
| tutorial_merchant_town           | Merchant Town Square        | 53          | merchant.ron        |
| tutorial_priestess_town          | High Priestess Town Square  | 55          | high_priestess.ron  |
| tutorial_wizard_arcturus         | Arcturus                    | 56          | wizard_arcturus.ron |
| tutorial_wizard_arcturus_brother | Arcturus Brother            | 58          | old_gareth.ron      |
| tutorial_ranger_lost             | Lost Ranger                 | 57          | ranger.ron          |
| tutorial_elder_village2          | Village Elder Mountain Pass | 51          | village_elder.ron   |
| tutorial_innkeeper_town2         | Innkeeper Mountain Pass     | 52          | innkeeper.ron       |
| tutorial_merchant_town2          | Merchant Mountain Pass      | 53          | merchant.ron        |
| tutorial_priest_town2            | High Priest Mountain Pass   | 54          | high_priest.ron     |
| tutorial_goblin_dying            | Dying Goblin                | 151         | dying_goblin.ron    |

**Note**: Multiple NPCs can share the same creature visual (e.g., both innkeepers use creature ID 52).

### Fallback Sprite System

The `sprite` field in NPC and monster definitions is optional. When `visual_id` or `creature_id` is set, the procedural mesh takes precedence. The sprite system is maintained for backward compatibility but is not actively used in this campaign.

### Troubleshooting

**Creature not rendering**:

1. Verify the creature ID exists in `data/creatures.ron`
2. Check that the filepath in the registry is correct
3. Ensure the individual creature file exists at the specified path
4. Run the campaign validator: `cargo run --bin campaign_validator -- campaigns/tutorial`

**Wrong creature appearing**:

1. Check the `visual_id` (monsters) or `creature_id` (NPCs) matches the intended creature
2. Verify no duplicate creature IDs in the registry
3. Clear the creature cache if running the game (restart)

**Performance issues**:

1. The creature registry loads all creatures at campaign startup
2. Individual creature files are cached after first use
3. For large campaigns (>100 creatures), consider splitting into multiple registry files

## Changelog

- Unreleased
  - Added comprehensive Visual Assets documentation with procedural mesh system details
  - Documented creature database structure and ID assignment ranges
  - Added monster-to-creature and NPC-to-creature mapping tables
  - Included guides for adding new creatures
  - Added troubleshooting section for creature rendering issues
  - Added `README.md` to satisfy campaign validation and document `starting_innkeeper` usage

## License & Credits

- See the project top-level `LICENSE` file for license terms.
- Maintainer / Contact: Brett Smith <xbcsmith@gmail.com>
