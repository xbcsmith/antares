# Next Plans

## SDK

### Metadata Editor

On the Metadata Editor, the Files tab is not complete.
Missing NPC and Conditions file paths from the Metadata --> Files editor.

[metadata files tab completion](./metadata_files_tab_completion_plan.md)

### Proficiencies Editor

Missing a Proficiencies editor tab.
Add a Proficiencies Editor tab to the Campaign Builder

[proficiencies editor](./proficiencies_editor_implementation_plan.md)

### Dialog Editor

The Campaign Builder dialog editor is not complete.
Add New Node does nothing.
Nodes are not editable. We should be able to edit all the data in a node.
Unable to create new nodes makes it impossible to create dialog trees.

[dialog editor completion](./dialog_editor_completion_implementation_plan.md)

### Remove Event Triggers

✅ COMPLETED - [remove per tile event triggers implementation](./remove_per_tile_event_triggers_implementation_plan.md)

### Config Editor Implementation

[config editor implementation](./config_editor_implementation_plan.md)

### Portrait Support Implementation

✅ COMPLETED - [portrait support implementation](./portrait_support_implementation_plan.md)

## Starting Position Implementation

✅ COMPLETED - Need to be able to set starting position for player characters in map editor. (It is done in the campaign.ron)

## Game Engine

### Character Definition updates

Full domain change: Change `CharacterDefinition` to store `AttributePair`/`AttributePair16` for stats (or an optional `current_stats` structure), update serialization, instantiation, and tests to support base+current for all stats. This is the most consistent but also the most invasive (more tests, docs, and backward-compatibility considerations).

### Sprite Support (After Tile Visual Metadata)

[Sprite Support](./sprite_support_implementation_plan.md)

### Game Play

No interaction with Doors you can walk right through them. (pressing the E key when in front of a door does nothing)
No interaction with NPC. No description messages are displayed and there is no dialog tree. (pressing the E key when in front of an NPC does nothing)
Signs are not implemented in world. There is no sprite or graphic to represent them and no dialog when they are triggered. They do show up in the logs when you walk over them. No description messages are displayed when the player interacts with a sign. (pressing the E key when in front of a sign does nothing)
Teleport tiles are not implemented in world. There is no sprite or graphic to represent them. They do show up in the logs when you walk over them, but the player is not teleported. No description messages are displayed when the player interacts with a teleport tile. (pressing the E key when in front of a teleport tile does nothing)
HUD text for HP is cut off by the bottom of the screen. Increasing the window size does not help.
HUD Portraits are offset and have a large blank area to the right side of the portrait image. If the plan is to display conditions next to the portrait, then this is fine, but if not, the portrait should be centered in the portrait area.
HUD character names do not need numbers next to them. Order can be determined by position on the HUD.

[game engine fixes](./game_engine_fixes_implementation_plan.md)

## npc externalization plan

✅ COMPLETED - [npc externalization plan](./npc_externalization_implementation_plan.md)

## npc gameplay fix plan

✅ COMPLETED - [npc gameplay fix plan](./npc_gameplay_fix_implementation_plan.md)

### Tile Visual Metadata

✅ COMPLETED - [Tile Visual Metadata](./tile_visual_metadata_implementation_plan.md)
