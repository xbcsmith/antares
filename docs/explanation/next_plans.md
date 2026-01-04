# Next Plans

## SDK

### Remove Event Triggers

✅ COMPLETED - [remove per tile event triggers implementation](./remove_per_tile_event_triggers_implementation_plan.md)

### Config Editor Implementation

[config editor implementation](./config_editor_implementation_plan.md)

### Portrait Support Implementation

✅ COMPLETED - [portrait support implementation](./portrait_support_implementation_plan.md)

## Starting Position Implementation

✅ COMPLETED - Need to be able to set starting position for player characters in map editor. (It is done in the campaign.ron)

## Game Engine

## npc externalization plan

✅ COMPLETED - [npc externalization plan](./npc_externalization_implementation_plan.md)

## npc gameplay fix plan

✅ COMPLETED - [npc gameplay fix plan](./npc_gameplay_fix_implementation_plan.md)

### Tile Visual Metadata

✅ COMPLETED - [Tile Visual Metadata](./tile_visual_metadata_implementation_plan.md)

### Sprite Support (After Tile Visual Metadata)

[Sprite Support](./sprite_support_implementation_plan.md)

## Character Definition updates

Full domain change: Change `CharacterDefinition` to store `AttributePair`/`AttributePair16` for stats (or an optional `current_stats` structure), update serialization, instantiation, and tests to support base+current for all stats. This is the most consistent but also the most invasive (more tests, docs, and backward-compatibility considerations).
