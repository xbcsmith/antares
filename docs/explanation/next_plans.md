# Next Plans

## Generic

### Campaign Data Documentation

Document the campaign data structure, including all the RON files that make up a campaign, and how they relate to each other.

✅ COMPLETED - [campaign data documentation](./campaign_data_documentation_plan.md)

## SDK

### Campaign Builder command line

Specify the campaign file to open from command line.

The Campaign Builder SDK should mimic the game engine with a `--campaign ./path/to/campaign` flag that automatically loads the `campaign.ron` file in the `./path/to/campaign`.

### Creatures Editor

Campaign Builder --> Creatures Editor

Creatures Editor is not loading the creatures.ron file in the tutorial campaign.

We should be able to control the creature mapping from creatures.ron file to assets/creatures/foo.ron

Campaign Builder --> Metadata Editor --> Files has no place to add the creatures file

Campaign Builder --> Creatures Editor

[@create_creatures.md](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/docs/how-to/create_creatures.md) Tools does not have a link to the Creature Editor. The only way to spawn a creature editor is to select New from The Campaign Builder --> Creatures tab. The Creatures tab has a list of creatures but no preview is rendered when selected. I would expect a Preview in the right column to appear with a Edit button that opens the Creatures Assets .ron file in the Creature Editor window. I would also expect to have a place where I can register an creature asset .ron in the creatures.ron registry. I thought we covered this in the [@creature_editor_enhanced_implementation_plan.md](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/creature_editor_enhanced_implementation_plan.md) [@creature_templates.md](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/docs/reference/creature_templates.md) are not displayed in the template browser window. and I can't figure out how to get to them.

✅ COMPLETED -[creature editor ui fix](./finished/creature_editor_ux_fixes_implementation_plan.md)

Clashing ID in the Creature Template browser.

Creature Template browser has clashing ID. ID Clashes when things like Windows or CollapsingHeaders share names or when things like Plot and Grid:s aren't given unique id_salt:s. Sometimes the solution is to use ui.push_id

Tools --> Creature Editor does not open the Creature Editor.

Campaign Builder --> Creature Editor --> Click Creature does not show anything in the right panel.

### Map Editor Multiselection

Campaign Builder --> Map Editor --> Select Map --> Edit

The map editor multi select and apply does not update the map data. It should update the map data with the selected changes.
If I select paint tile grass none, in Visual Properties select short grass from the dropdown, click multi select, select several tiles, and then click apply to tiles, save map, save campaign the map is not updated.

✅ COMPLETED -

### Terrain-Specific Settings and Visual Presets

Campaign Builder --> Map Editor --> Select Map --> Edit

The Terrain-Specific Settings do nothing. The Terrain-Specific Settings should be applied when the user clicks the apply button for Visual Properties. The Terrain-Specific Settings should respect the multi-selection function as well.

Visual Properties and Terrain-Specific Settings should reset to defaults after the user clicks the apply button for Visual Properties and when the Back to list button is clicked.

Visual Preset buttons do nothing. The Visual Preset button should immediately apply the selected visual preset to the selected tiles. Visual Presets needs a multi-select mode.

✅ COMPLETED -

### Map Editor NPC

Campaign Builder --> Map Editor --> Select Map --> Edit

NPCs placed on the map are not editable. Selecting an NPC on the map does not bring up the NPC editor. It should bring up the NPC editor with the selected NPC's data loaded.

✅ COMPLETED

Campaign Builder --> Map Editor --> Select Map --> Edit

NPCs placed on the map can not be removed. Add a remove NPC button that removes the selected NPC from the map.

### NPC Editor

Campaign Builder --> Dialogue Editor --> Edit

The edit NPC editor does not save changes to the NPC. It should save the NPC to the campaign data.

✅ COMPLETED

### Quest Editor

Campaign Builder --> Quest Editor --> Edit

The rewards section and ojectives section of the editor run off the edge of the window and is not scrollable. It should fit the window and scale as it does and be scrollable so that all rewards can be seen and edited.

✅ COMPLETED

### Map Editor Events

Campaign Builder --> Map Editor --> Select Map --> Edit Map. Event editing now fully supported:

- Create new events using PlaceEvent tool
- Edit existing events via Inspector Panel "Edit Event" button
- Remove events via Inspector Panel or event editor
- Visual feedback shows which event is being edited

✅ COMPLETED - [event editing implementation](./event_editing_implementation_plan.md)

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

### Clean up

Analyze this codebase for refactoring opportunities:

1. Find duplicate code patterns
2. Identify unused exports and dead code
3. Review error handling consistency
4. Check for security vulnerabilities

Compile the findings into a prioritized action plan with a phased approach.

### Advanced Procedural Meshes

The procedural meshes are not complete. We need to implement advanced procedural meshes in the game engine so that we can create complex environments and objects in the game. We need to create more complicated trees, shrubs, grass, signs, thrones, benches, tables, chairs, chests, torches, structures, and objects. Research procedural mesh generation in Bevy and implement advanced procedural meshes in the game engine. I would like to keep the feel of the game the same but with more detailed and complex objects.

Valoren has examples of procedural meshes. I do not want the blocky look of Valoren. But it has some good examples of procedural meshes.

Articles on procedural meshes:

https://clynamen.github.io/blog/2021/01/04/terrain_generation_bevy/

https://medium.com/@heyfebin/the-impatient-programmers-guide-to-bevy-and-rust-chapter-2-let-there-be-a-world-procedural-57710a20eb43

https://dev.to/mikeam565/rust-game-dev-log-5-improved-terrain-generation-dynamic-grass-in-an-endless-world-291i

Write a plan with a phased approach to implementing advanced procedural meshes in the game engine. THINK HARD and follow the rules in @PLAN.md

### Party View Point

The view point of the party is not centered. When the party approaches a door the door is not centereed on the screen it is off by half. This behavior applies to all objects in the game. It makes navigation very difficult. The party view point should be centered.

✅ COMPLETED - [viewport centering fix implementation](./viewport_centering_fix_implementation.md)

### Combat System

The combat system is not implemented in the game engine. When the player encounters an enemy, nothing happens. We need to implement a turn-based combat system in the game engine so that when the player encounters an enemy, the combat system is triggered and the player can fight the enemy. The combat system should include health points, attack points, defense points, and special abilities for both the player and the enemy. The combat system should also include a victory condition for the player to win the fight and a defeat condition for the player to lose the fight.

Core Components of a Bevy Combat System

    State Management: Use States to manage game flow (e.g., CombatState::PlayerTurn, CombatState::EnemyTurn).
    Components: Define data structures for combatants, such as Health, Damage, Defense, and Position.
    Systems: Create functions that query components to perform logic, such as attack_system or damage_calculation_system.
    Events: Use Bevy's event system for actions, such as AttackEvent or DamageEvent, to handle interactions between entities.
    Plugin Architecture: Separate logic into plugins for modularity (e.g., CombatPlugin, WeaponPlugin).

✅ COMPLETED - [combat system implementation plan.md](/Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/combat_system_implementation_plan.md)

### InnKeeper Party management

InnKeeper Party management is broken. It triggers automatically before the player enters the inn. It should be triggered by a dialog node. There is no mouse support and no keyboard support so there is no way to navigate the party management window. ESC does not work to close the window. Basically the game is stuck at the InnKeeper Party management window. Characters that were recruited and in the party already still appear in the recruitable list. The recruit screen should only show characters that are not already in the party. Characters that are in the party should appear in the party list with an option to remove them from the party. Removing a character from the party should return them to the recruitable list. ESC should close the party management window and return to the game.

✅ COMPLETED - [innkeeper party management fixes plan.md](/Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/innkeeper_party_management_fixes_plan.md)

### Character Recruitment Events

We finished the @dialogue_bevy_ui_refactor_plan.md and now we have working chat windows. Several issues still exist. For example the character Whisper is using the default character recruitment dialogue (id 100) instead of her dialogue (id 102). Same goes for Zara. WHen asked to join the party choices do not clear the bubble so when we move the bubble stays with the dialogue. Whisper does not join the party. When we talk to the NPC after talking to Whisper the choices do not clear. The NPC Choices are appened to the bubble. InnKeeper Party management

Character recruitment events are not implemented in the game engine. When a dialog node triggers a recruitment event, nothing happens. We need to implement character recruitment events in the game engine so that when a dialog node triggers a recruitment event, the specified character is added to the player's party. The event needs to be able to have a dialog attached to it that plays when the character is recruited. Then the character should either jopin the party and appear in the party HUD or travel to the inn removing the event from the map in-game. If there is no dialog the character should use the default recruitment dialog.

Current Behavior:

2026-01-10T21:48:04.284407Z INFO antares::game::systems::input: Interacting with recruitable character 'Apprentice Zara' (ID: npc_apprentice_zara) at Position { x: 11, y: 6 }
2026-01-10T21:48:04.301660Z INFO antares::game::systems::dialogue_visuals: Speaker entity PLACEHOLDER despawned during dialogue, ending conversation

Example Recruitable Characters:

RecruitableCharacter(
name: "Apprentice Zara",
description: "A young gnome apprentice studies a spellbook intently.",
character_id: "npc_apprentice_zara",
dialog_id: 101,
),

RecruitableCharacter(
name: "Old Gareth",
description: "A grizzled dwarf veteran stands here, repairing armor.",
character_id: "npc_old_gareth",
dialog_id: 100,
),

✅ COMPLETED - [character recruitment implementation](./character_recruitment_implementation_plan.md)

### Custom Fonts

Supporting custom fonts requires updates to the campaign config to allow specify a custom Dialogue Font, a Custom Game Menu font. I would expect it to work like this. Default Dialogue Font --> Custom Font in Campaign. The custom Font path should be ./campaigns/<campaign name>/fonts/<font-name>.ttf and it should be configurable by the Campaign Config RON file. If no custom font is specified in the Campaign Config RON file, the default font should be used.

### Game menu implementation

The in game menu is off by one when using the keyboard to select the options on the main menu. Save Game brings up a Save Game dialog that works with the mouse but can not be navigated with the keyboard. Load Game does nothing. It should bring up the list of saved games to load. The Save Game and Load Game dialogs should be navigable with the keyboard. The Save Game dialog has a Save and Load button that do nothing. They should save and load the selected game. THINK HARD and follow the rules in @AGENTS.md

A configurable Keyboard Key (default ESC) should bring up the game menu. Currently it does nothing. We need to implement the game menu. It should include options like new game, save game, load game, delete game, quit, etc. We should also add a Configuration menu option that allows the user to change settings like volume, graphics quality, key bindings, etc and store it in the Save Game RON File. This config would override the default Campaign Config RON file settings once set. Keeping a list of recent saves that the use can pick from to load would also be a feature.

✅ COMPLETED - [game menu implementation](./game_menu_implementation_plan.md)

### Teleport to Map

The teleport to map event works but the target map is not rendering correctly. The NPC are appearing but the tiles are not rendering. We need to fix the teleport to map event so that when the player is teleported to a new map, the map renders correctly.

✅ COMPLETED

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

✅ COMPLETED - [dialog system implementation](./dialog_system_implementation_plan.md)

### Character Definition updates

Full domain change: Change `CharacterDefinition` to store `AttributePair`/`AttributePair16` for stats (or an optional `current_stats` structure), update serialization, instantiation, and tests to support base+current for all stats. This is the most consistent but also the most invasive (more tests, docs, and backward-compatibility considerations).

✅ COMPLETED - [character definition attribute pair](./character_definition_attribute_pair_migration_plan.md)

### Sprite Support (After Tile Visual Metadata)

✅ COMPLETED - [Sprite Support](./sprite_support_implementation_plan.md)

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

### Warnings

2026-01-17T21:23:14.969390Z WARN bevy_ecs::error::handler: Encountered an error in command `<bevy_ecs::system::commands::entity_command::despawn::{{closure}} as bevy_ecs::error::command_handling::CommandWithEntity<core::result::Result<(), bevy_ecs::world::error::EntityMutableFetchError>>>::with_entity::{{closure}}`: The entity with ID 600v217 does not exist (enable `track_location` feature for more details)

    If you were attempting to apply a command to this entity,
    and want to handle this error gracefully, consider using `EntityCommands::queue_handled` or `queue_silenced`.

✅ COMPLETED -

### Procedural Meshes Implementation

✅ COMPLETED - [procedural meshes implementation](./procedural_meshes_implementation_plan.md)

## Inventory System

### ArmorClassification Expansion

The current `ArmorClassification` enum in `src/domain/items/types.rs` only has four variants: `Light`, `Medium`, `Heavy`, and `Shield`. This means helmets and boots cannot be represented as distinct armor classifications, and the `Equipment` struct fields `equipment.helmet` and `equipment.boots` have no corresponding `ItemType` variant to route items into those slots during equip operations. Slot resolution currently falls through to item `tags` as a workaround.

The goal is to expand `ArmorClassification` so that every named slot on `Equipment` has a first-class classification, and so that the total AC score contributed by a character's equipped armor accounts for all worn pieces: body armor, shield, helmet, and boots.

Work required:

- Add `Helmet` and `Boots` variants to `ArmorClassification` in `src/domain/items/types.rs`.
- Update `ArmorClassification::to_proficiency_id()` in `src/domain/items/types.rs` to map the new variants to appropriate proficiency IDs (likely reusing the existing light/heavy armor proficiency IDs, or adding dedicated ones if needed).
- Update `has_slot_for_item()` in `src/domain/items/equipment_validation.rs` to route `Armor(ArmorClassification::Helmet)` to `equipment.helmet` and `Armor(ArmorClassification::Boots)` to `equipment.boots`.
- Update `do_equip()` (once implemented in `src/domain/transactions.rs`) to use the classification directly for slot resolution instead of relying on item `tags`.
- Update `data/items.ron` and `campaigns/tutorial/data/items.ron` to set `classification: Helmet` or `classification: Boots` on all helmet and boot items that currently use tags as a workaround.
- Update AC calculation (wherever total armor class is computed from equipped items) to sum contributions from body armor slot, shield slot, helmet slot, and boots slot. Each slot should contribute its `ArmorData::ac_bonus` to the character's effective AC.
- Update `src/sdk/validation.rs` to validate that items with `classification: Helmet` are only assigned to `equipment.helmet` and items with `classification: Boots` are only assigned to `equipment.boots`.
- Update all tests in `src/domain/items/equipment_validation.rs` and `src/domain/items/types.rs` that reference `ArmorClassification` to cover the new variants.

This change is not backward compatible with existing RON item data that uses `tags` for helmet and boot slot routing. All affected item definitions must be migrated at the same time.

### Equipped Weapon Damage in Combat

Currently `perform_attack_action_with_rng` in `src/game/systems/combat.rs` hardcodes `Attack::physical(DiceRoll::new(1, 4, 0))` for every player attack, regardless of what the character has equipped in `equipment.weapon`. Monster attacks correctly read from their `attacks` list, but player characters always deal 1d4 physical damage. This means a Fighter wielding a longsword (1d8+2) deals the same damage as an unarmed apprentice.

The goal is to derive the player attack from the character's equipped weapon when one is present, and fall back to a defined unarmed attack when the weapon slot is empty.

Work required:

- Add a new function `get_character_attack` in `src/domain/combat/engine.rs` (or `src/domain/items/types.rs`) with the following logic:
  - Accept `character: &Character` and `item_db: &ItemDatabase`.
  - If `character.equipment.weapon` is `Some(item_id)`, look up the item in `item_db`. If the item is a `Weapon`, construct and return `Attack::physical(weapon_data.damage)` with `bonus` applied (add `weapon_data.bonus` to the `DiceRoll` bonus field or apply it as a flat damage modifier after the roll). If the item is not found or is not a weapon type, fall through to the unarmed default.
  - If `character.equipment.weapon` is `None`, or the lookup fails, return the unarmed fallback: `Attack::physical(DiceRoll::new(1, 2, 0))` with `WeaponClassification::Unarmed`. The `"unarmed"` proficiency already exists in `data/proficiencies.ron`.
- Update `perform_attack_action_with_rng` in `src/game/systems/combat.rs` to replace the hardcoded `DiceRoll::new(1, 4, 0)` line with a call to `get_character_attack(&character, item_db)`, where `item_db` is retrieved from the `ContentDatabase` already available via the `content: &GameContent` parameter.
- Define a module-level constant `UNARMED_DAMAGE: DiceRoll = DiceRoll { count: 1, sides: 2, bonus: 0 }` in `src/domain/combat/engine.rs` so the fallback value is not a magic literal.
- The `WeaponData::bonus` field (i16) should be added to the total damage after the dice roll, floored at 1. A negative bonus on a cursed weapon can reduce damage but never below 1.
- Update `perform_monster_turn_with_rng` to remain unchanged — monsters already use `choose_monster_attack` which reads from `monster.attacks`.

Testing requirements:

- `test_player_attack_uses_equipped_weapon_damage` — equip a weapon with known `DiceRoll`, run `perform_attack_action_with_rng` with a seeded RNG, assert damage is within the weapon's range, not the old hardcoded 1d4 range.
- `test_player_attack_unarmed_when_no_weapon_equipped` — character with `equipment.weapon = None`, run attack, assert damage is within 1d2 range (1–2 before might bonus).
- `test_get_character_attack_returns_weapon_data` — unit test for `get_character_attack` directly: equip item_id 1 (a known weapon), assert returned `Attack.damage` matches the weapon's `WeaponData.damage`.
- `test_get_character_attack_returns_unarmed_fallback` — unit test: `equipment.weapon = None`, assert returned `Attack.damage == UNARMED_DAMAGE`.
- `test_get_character_attack_invalid_item_id_falls_back_to_unarmed` — equip a non-existent item_id (not in ItemDatabase), assert fallback to `UNARMED_DAMAGE` rather than panic.

### Dropped Items World Persistence

When a character drops an item (via `drop_item()` in `src/domain/transactions.rs`), the item is currently removed from the character's inventory and discarded. There is no mechanism to place it in the game world at the position where it was dropped, nor to represent it as a pickable entity on the map.

The goal is to persist dropped items as world entities so that:

- A dropped item appears at the tile position where it was dropped.
- The player can walk over or interact with the tile to pick the item up.
- Dropped items survive session save and load.
- Dropped items are scoped to the map they were dropped on.

Work required:

- Add a `DroppedItem` event type to `src/domain/world/events.rs` (or a dedicated `src/domain/world/dropped_items.rs` module) with fields: `item_id: ItemId`, `charges: u8`, `position: Position`, `map_id: MapId`.
- Add a `dropped_items: Vec<DroppedItem>` field to the `Map` struct in `src/domain/world/types.rs` so that dropped items are stored per-map and serialized with world state.
- Update `drop_item()` in `src/domain/transactions.rs` to accept a `position: Position` and `map_id: MapId` and insert a `DroppedItem` record into the appropriate map.
- Add a pickup operation `pickup_item()` in `src/domain/transactions.rs` that removes the `DroppedItem` record from the map and adds the item to a character's inventory (subject to the same `Inventory::MAX_ITEMS` capacity check as `buy_item`).
- Add a `PickupItem` `EventResult` variant to `src/domain/world/events.rs` so that walking over a dropped item tile can trigger the pickup flow.
- Spawn a visual marker entity (procedural mesh or sprite) in the game engine for each `DroppedItem` on the current map when the map loads.
- Despawn the visual marker when the item is picked up.
- Ensure `SaveGame` serialization captures `dropped_items` on each map as part of the world state round-trip.

This item is tracked here for future planning. It does not need to be addressed in the current inventory system implementation plan.
