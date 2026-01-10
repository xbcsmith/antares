# Next Plans

## Generic

### Campaign Data Documentation

Document the campaign data structure, including all the RON files that make up a campaign, and how they relate to each other.

✅ COMPLETED - [campaign data documentation](./campaign_data_documentation_plan.md)

## SDK

### Map Editor Events

✅ COMPLETED - [event editing implementation](./event_editing_implementation_plan.md)

Campaign Builder --> Map Editor --> Select Map --> Edit Map. Event editing now fully supported:

- Create new events using PlaceEvent tool
- Edit existing events via Inspector Panel "Edit Event" button
- Remove events via Inspector Panel or event editor
- Visual feedback shows which event is being edited

### Config Editor Implementation

✅ COMPLETED - [config editor implementation](./config_editor_implementation_plan.md)

### Metadata Editor

On the Metadata Editor, the Files tab is not complete.
Missing NPC and Conditions file paths from the Metadata --> Files editor.

✅ COMPLETED - [metadata files tab completion](./metadata_files_tab_completion_plan.md)

### Proficiencies Editor

Missing a Proficiencies editor tab.
Add a Proficiencies Editor tab to the Campaign Builder

✅ COMPLETED - [proficiencies editor](./proficiencies_editor_implementation_plan.md)

### Dialog Editor

The Campaign Builder dialog editor is not complete.
Add New Node does nothing.
Nodes are not editable. We should be able to edit all the data in a node.
Unable to create new nodes makes it impossible to create dialog trees.

✅ COMPLETED - [dialog editor completion](./dialog_editor_completion_implementation_plan.md)

### Remove Event Triggers

✅ COMPLETED - [remove per tile event triggers implementation](./remove_per_tile_event_triggers_implementation_plan.md)

### Portrait Support Implementation

✅ COMPLETED - [portrait support implementation](./portrait_support_implementation_plan.md)

## Starting Position Implementation

✅ COMPLETED - Need to be able to set starting position for player characters in map editor. (It is done in the campaign.ron)

## Game Engine

### Procedural Meshes Implementation

[procedural meshes implementation](./procedural_meshes_implementation_plan.md)

### Ingame Dialog System

Need to represent and display dialog trees in the game engine.

bevy_talks is a strong choice because it natively supports RON-based dialogue assets and handles the complex state transitions between dialogue nodes for you.

1.  Data-to-Logic Mapping
    While your custom RON format differs slightly from the default bevy_talks schema, the crate is designed to load .talk.ron files directly via its ron_loader module.

        Built-in Loader: You can load your dialogue with a simple handle: let handle: Handle<TalkData> = asset_server.load("dialogue.talk.ron");.
        Entity-Graph Approach: When you initiate a talk, the plugin spawns the entire dialogue tree as a graph of Bevy Entities. Each dialogue node in your RON becomes an entity with a Talk or CurrentNode component.
        Events: To advance the dialogue or handle choices, you send events like NextActionRequest or ChooseActionRequest. The plugin then updates the CurrentNode automatically.

2.  Implementation of the Floating Text Box
    To achieve a "2.5D retro" floating effect that works with your procedural meshes, follow this technical pattern:

        World-Space Text (Text2d): Use Bevy 0.15's Text2d component instead of UI nodes. This allows the dialogue box to exist at a specific 3D coordinate (e.g., Vec3(0.0, 2.5, 0.0) above your NPC) rather than being stuck to the screen corners.
        Billboard Component: To ensure the text box is always readable from any angle in your 2.5D world, use a Billboard plugin or a system that forces the text entity to rotate and face the Camera3d.
        Procedural Background: Since you are already generating meshes, you can spawn a simple Mesh3d (using a Cuboid or Plane) directly behind the text to act as the "bubble" background.

3.  Workflow for your specific RON
    Because your RON includes custom fields like associated_quest and actions: [TriggerEvent(...)], you can extend bevy_talks by listening for the specific entity changes it triggers:

        System: Create a system that queries for Changed<CurrentNode>.
        Logic: When the node changes, read the text from your loaded TalkData asset.
        Display: Update the Text2d component on your floating box entity.
        Custom Actions: When the plugin reaches a node with your TriggerEvent, use a Bevy EventWriter to fire off your recruitment or quest logic in Rust.

Recommended Tooling:

    bevy_talks: Best for RON-driven data-to-ECS dialogue graphs.
    bevy_text_popup: Useful for quick event-based text triggers.
    bevy_animated_text: Adds the "retro" typewriter effect to your Text2d dialogue.

[dialog system implementation](./dialog_system_implementation_plan.md)

### Character Definition updates

Full domain change: Change `CharacterDefinition` to store `AttributePair`/`AttributePair16` for stats (or an optional `current_stats` structure), update serialization, instantiation, and tests to support base+current for all stats. This is the most consistent but also the most invasive (more tests, docs, and backward-compatibility considerations).

✅ COMPLETED - [character definition attribute pair](./character_definition_attribute_pair_migration_plan.md)

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

✅ COMPLETED - [game engine fixes](./game_engine_fixes_implementation_plan.md)

## npc externalization plan

✅ COMPLETED - [npc externalization plan](./npc_externalization_implementation_plan.md)

## npc gameplay fix plan

✅ COMPLETED - [npc gameplay fix plan](./npc_gameplay_fix_implementation_plan.md)

### Tile Visual Metadata

✅ COMPLETED - [Tile Visual Metadata](./tile_visual_metadata_implementation_plan.md)
