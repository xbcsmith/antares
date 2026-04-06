# Next Plans

merchant_inventory_ui.rs`— high confidence, good fit
2.`events.rs`— medium confidence, broader churn
3.`inventory_ui.rs`exploration item use /`lock_ui.rs` / input systems — lower priority unless you want to push the rule much further
the main remaining non-dialogue-engine direct-log areas:

- `input/exploration_interact.rs`
- `input/exploration_movement.rs`
- `input.rs` wiring around those helpers
- parts of `inventory_ui.rs` if you want stricter consistency

## Generic

## SDK

Analyze the @sdk codebase for refactoring opportunities use subagents to do the following:

1. Find duplicate code patterns, look for places we can consolidate code and create reusable functions or components.
2. Identify unused exports, specific "#[ignore]", and dead code "#[dead_code]", #[allow(unused_mut)], #[allow(clippy::too_many_arguments)], "#[allow(deprecated)]" to see if there are any exports that can be removed or refactored to reduce clutter and improve maintainability.
3. Review error handling consistency
4. Look for unfinished TODOs, FIXMEs, and place holders in the codebase as well as references to Phases in the codebase that should be removed.
5. References to Phases in the codebase that should be removed.

We do not care about backwards compatability. Compile the findings into a prioritized action plan with a phased approach.

Write a plan with a phased approach to cleaning up the codebase. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [SDK Codebase Cleanup Plan](./sdk_codebase_cleanup_plan.md)

## Game Engine

### Clean up

Analyze the @src codebase for refactoring opportunities use subagents to do the following:

1. Find duplicate code patterns, look for places we can consolidate code and create reusable functions or components.
2. Identify unused exports, specific "#[ignore]", and dead code "#[dead_code]", #[allow(unused_mut)], #[allow(clippy::too_many_arguments)], "#[allow(deprecated)]" to see if there are any exports that can be removed or refactored to reduce clutter and improve maintainability.
3. Review error handling consistency
4. Look for unfinished TODOs, FIXMEs, and place holders in the codebase as well as references to Phases in the codebase that should be removed.
5. References to Phases in the codebase that should be removed.

We do not care about backwards compatability. Compile the findings into a prioritized action plan with a phased approach.

Write a plan with a phased approach to cleaning up the codebase. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Game Codebase Cleanup Plan](./game_codebase_cleanup_plan.md)

### Feature Completion

Game log placement, time advancement, recruited character mesh persistence,
lock UI input handling, trap/treasure mechanics, dialogue recruitment actions,
audio system, SDK validation stubs, and more.

✅ PLAN WRITTEN - [Game Feature Completion Plan](./game_feature_completion_plan.md)

### Custom Fonts

Supporting custom fonts requires updates to the campaign config to allow specify a custom Dialogue Font, a Custom Game Menu font. I would expect it to work like this. Default Dialogue Font --> Custom Font in Campaign. The custom Font path should be ./campaigns/<campaign name>/fonts/<font-name>.ttf and it should be configurable by the Campaign Config RON file. If no custom font is specified in the Campaign Config RON file, the default font should be used.

Write a plan with a phased approach to implementing custom fonts. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [custom fonts](./custom_fonts_plan.md)

### Encounter Visibility Follow-up (Skeleton)

Applied now:

1. Encounter trigger flow now supports both behaviors: auto-combat when stepping on an encounter tile, and explicit interaction from adjacent tiles.
2. Encounter marker visual lifted slightly above tile geometry to improve readability and reduce floor occlusion.

Add this as next follow-up:

3. Optional portrait fallback in the combat skeleton HP box when mesh readability is still poor from camera angle or scene clutter.

### Game-Wide Mouse Input Support

Mouse input currently does not work reliably across the game engine (combat,
inn management, menus, and in-world UI interactions). We need a full engine-
wide mouse input pass, not one-off fixes per screen.

Work required:

- Audit every gameplay mode and UI surface for mouse interaction support:
  `Exploration`, `Combat`, `Menu`, `Dialogue`, `InnManagement`, and editor-like
  in-game panels.
- Define a unified click/hover/press interaction model so mouse behavior is
  consistent across all systems.
- Ensure Bevy `Interaction` handling (`Pressed`, `Hovered`, `None`) is wired
  consistently and does not depend on keyboard-first assumptions.
- Add a shared input utility layer for mouse activation detection to avoid
  duplicated ad-hoc patterns across systems.
- Validate mouse support for all combat actions and target selection paths.
- Validate mouse support for game menu navigation, save/load dialogs, and
  settings controls.
- Validate mouse support for innkeeper party management and recruitment-related
  UI flows.
- Add regression tests for mouse input in each major mode to prevent future
  breakage.

Write a plan with a phased approach to implementing game-wide mouse input support in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETED - [Game Wide Mouse Input Support](./game_wide_mouse_input_support_plan.md)

### Game Log

We need a Game Log. It should be a log that shows all the important events that happen in the game. It should show things like when the player picks up an item, when they talk to an NPC, when they enter a new area, when they take damage, etc. The game log should be visible in the UI and should have a scroll bar so that the player can see past events. The game log should also have a filter so that the player can filter the log by event type (e.g. combat events, dialogue events, item events, etc).

Write a plan with a phased approach to implementing a game log in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETED - [Game Log Implementation Plan](./game_log_implementation_plan.md)

### Automap and mini map

We need to implement an automap and mini map in the game engine. The automap should be a full map of the current level that is revealed as the player explores. The mini map should be a smaller version of the automap that is always visible in the corner of the screen. The mini map should show the player's current position and the surrounding area. The automap should be accessible from the game menu and should allow the player to see the entire level and their current position on it. The automap should be mapped to the M key and configurable through the game config. We will combine the mini map, compas, and clock into a single UI element in the top right corner of the screen. The mini map should also show important locations like quest objectives, merchants, and points of interest. The automap should have a fog of war effect that hides unexplored areas of the map. The automap should also have a legend that shows what different symbols on the map mean (e.g. red dot for monsters, green dot for merchants, etc).

Write a plan with a phased approach to implementing an automap and mini map in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETED - [Automap and Mini Map Implementation Plan](./automap_and_mini_map_implementation_plan.md)

### Unconcious Characters and Dead Characters

Characters with 0 HP are unconcious and should not be allowed to attack monsters. They should also not be allowed to be attacked by monsters. They should be able to be healed by other characters. Unconcious IS a condition. It is a special condition because of combat implecations. We should add it to the Conditions in a Campaign. Characters remain unconcious until they are healed with a Spell, Scroll, or by resting. We should also add Dead to the Conditions in a Campaign. I haven't yet because it requires a lot of wiring because you need to resurrect dead characters either with a Spell, Scroll, or a Priest/Priestess. Dead should be able to be permanent if the Campaign creator wants it to be. We can also add Uncoincious and Dead as conditions from a Spell or Scroll or Consumable.The default should be "until ressurected". The default template for conditions in the SDK should include Unconcious and Dead so that Creators do not forget to add them to their campaigns.

Write a plan with a phased approach to implementing unconcious characters in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETED - [Unconcious Characters Implementation Plan](./unconcious_characters_implementation_plan.md)

### Game Tray Icon Implementation Plan

We need to add a tray icon for the game like the ones we added for the SDK.

✅ PLAN WRITTEN - [Game Tray Icon Implementation Plan](./game_tray_icon_implementation_plan.md)

### Spell System Updates

We need to fix the Combat Spell System. Curently there is no concept of a player Spell book, learned spells (from scroll or NPC), the combat system has no way for a Character to cast spells in combat. Spell casting should cost Spell Points per cast and in some cases Gems or other Consumables (configurable in spell data). We will need to add support for spell point bar to the HUD layout like the Hit Point Bar (but blue). Spells also need to be cast outside of combat. We will need to add support for casting spells outside of combat as well. We will need to add support for spell effects that can be applied outside of combat as well. We will need to add support for spell effects that can be applied in combat as well. We will need to add support for spell effects that can be applied in both combat and out of combat as well. We will need to add support for spell effects that can be applied to characters, monsters, and the environment as well. We will need to add support for spell effects that can be applied to characters, monsters, and the environment in both combat and out of combat as well. We will need to add support for spell effects that can be applied to characters, monsters, and the environment in both combat and out of combat as well.

We will need to add support for spell effects that can be applied to characters, monsters, and the environment in both combat and out of combat as well. Research the Code base to find out how to implement the spell system updates. We will need to add new components, systems, and UI elements to support the new spell system. We will also need to update the existing combat system to support spell casting and spell effects. We will also need to update the existing character system to support learned spells and spell points. We will also need to update the existing inventory system to support spell scrolls and other spell-related items. We will also need to update the existing dialogue system to support learning spells from NPCs. We will also need to update the existing quest system to support quests that reward spells or require spell casting. We will also need to update the existing save/load system to support saving and loading learned spells, spell points, and spell effects. The SDK will need to be updated to support creating spells, spell scrolls, and other spell-related items as well as supporting learning spells from NPCs and quests.

Write a plan with a phased approach to implementing the spell system updates in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Spell System Updates Implementation Plan](./spell_system_updates_implementation_plan.md)

## Future Features and Fixes

FIXED - When interaction with Recruitable Character is initiated from an adjacent tile, after the recruit dialog is finished and the player joins the party the Recruitable Characters mesh does not disappear until they are walked over. It should disappear immediately on recruitment. It looks remove_event`is called at the party's position instead of the adjacent tile where the event actually is, the event stays on the map, so the visual persists. The party then has to walk to that adjacent tile, triggering`check_for_events`again, which finds the`RecruitableCharacter` event but logs that it's not...

FIXED - Combat should be able to be initiatiated using the E key or Mouse when standing on an adjacent tile to an encounter trigger, not just by stepping on the encounter trigger tile. This is important for accessibility and also for allowing players to choose when to engage in combat rather than being forced into it by stepping on a tile.

FIXED - Campaign Builder --> Maps --> Edit Map --> Click on NPC options are Edit NPC (goes straight to the NPC editor) and Remove NPC. Need a way to edit the Event so I can control what direction the NPC is facing. Currently there is no way to edit the NPC's facing direction, so all NPCs face the same direction which looks bad. We should add an Edit Event option that allows you to edit the event's facing direction as well as other properties of the event like the dialogue it triggers, the quests it gives, etc.

FIXED - Spell Casting pop up during combat should be in the upper left hand corner of the screen. Currently it is low and to the left and is covered by the grey box that shows the Action buttons and the Monster HP. You can't get to it to click cancel when a Character has no combat spells or spell points left. We should move the Spell Casting pop up to the upper right hand corner of the screen so that it is not covered by the grey box and so that it is more visible and easier to click on.

## Things you never think of until you have to implement them

### Spell Management

@spell_system_updates_implementation_plan.md did not cover player interaction with the spell system outside of combat.

There is no way to manage spells for a character in the game engine. We need to add a Spell Book Management UI where you can see the spells a character has learned, the spell points they have, and the spell scrolls they have in their inventory. You should also be able to learn new spells from NPCs and from quests. We should also add support for spell effects that can be applied outside of combat as well as in combat. We should also add support for spell effects that can be applied to characters, monsters, and the environment as well.

@spell_system_updates_implementation_plan.md Phase 4 added ways to learn spells but did not add any management for the Character Spell Book in the game engine or the SDK.

Write a plan with a phased approach to implementing the rest of the spell system in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETED - [Spell Management Implementation Plan](./spell_management_implementation_plan.md)

### SpellBook eGui Conversion

✅ PLAN WRITTEN - [SpellBook eGui Conversion Plan](./spellbook_egui_conversion_plan.md)

### SDK Fixes

Campaign Builder --> Maps --> Edit Map --> Add Event --> Container can contain items but there is no way to add Gold or Gems to the container in the SDK. We should add the ability to add Gold and Gems to the container in the SDK as well as in the game engine when looting a container.

Campaign Builder --> Maps --> Edit Map --> Edit Event in the right column the Event Editor should be placed right under the Event Details instead of at the bottom of the column.

Campaign Builder --> Maps --> Edit Map --> Place Event --> Container or Furniture does not update the map RON file on save. I can place a furniture or container on the map and it shows up in the editor but when I save the map and look at the map RON file, the container or furniture is not listed in the map RON file. We should update the map RON file when a container or furniture is placed on the map so that it is saved properly.

Campaign Builder --> Furniture --> Edit Furniture does not have a Back to List button that is required to get back to the list of furniture. We should add a Back to List button to the Edit Furniture screen that takes you back to the list of furniture.

Campaign Builder --> Stock Templates --> Edit Stock Template does not load the description of the Stock Template in the editor. We should load the description of the Stock Template in the editor so that it can be edited as well.

Campaign Builder --> Stock Templates --> Display does not show the Description. We should show the Description of the Stock Template in the Display screen.

Campaign Builder --> NPC --> NPC Editor when you designate an NPC as a Merchant and clickthe Create Merchant Dialog button no dialog is created for the merchant. We should create a default dialog for the merchant when the Create Merchant Dialog button is clicked in the NPC Editor.

Campaign Builder --> Characters --> Display does not have starting spells listed in the character details. We should add starting spells to the character details.

Campaign Builder --> Characters --> Edit Character Starting Spells Auto Complete always uses Cleric Spells for Sorcerers when there are identical spells in both disciplines. So Awaken is always set to the Cleric spell instead of the Sorcerer spell. We should fix the Auto Complete to check the character's class and only show spells that are available to that class.

Campaign Builder --> Characters --> Edit Character Starting Spells area is very small so you can only see 2 spells before you have to scroll. We should make the Edit Character Starting Spells area larger so that 5 spells can be seen at once without scrolling.

Campaign Builder --> Validation --> NPC Stock Templates are flagged as unknown stock templates. The stock templates exist and are listed in the Stock Template section of the Campaign Builder but they are flagged as unknown in the Validation section. We should fix the validation to recognize the NPC Stock Templates so that they are not flagged as unknown.

Campaign Builder --> Config Editor --> Key Bindings is missing the bindign for the Spellbook.

Write a plan with a phased approach to implementing the SDK fixes in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [SDK Fixes Implementation Plan](./sdk_fixes_implementation_plan.md)

### Combat Fixes

In Combat Defense system is not implemented. What happens when a CHaracter selects Defend in its action round? We should add a defense system to combat where when characters choose to defend instead of attacking. Defending should reduce the damage taken from the next attack by a certain percentage based on the character's defense stat and the type of attack. We should also add support for spells and items that can increase defense or provide temporary invulnerability.

Combat Use Item system is not implemented. There is no way for a character to use an item in combat. We should add a system for using items in combat where characters can choose to use an item from their inventory instead of attacking or defending. Using an item should consume the item and apply its effects immediately. We should also add support for items that can be used outside of combat as well as in combat.

Out of combat use item system is not implemented. There is no way for a character to use an item outside of combat. We should add a system for using items outside of combat where characters can choose to use an item from their inventory while exploring the world or interacting with NPCs. Using an item should consume the item and apply its effects immediately. We should also add support for items that can be used in combat as well as outside of combat.

Write a plan with a phased approach to implementing the combat fixes in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Combat Fixes Implementation Plan](./combat_fixes_implementation_plan.md)

## Notes

Month Year Date in Game Engine View looks horrible.

Trees are still horrible. Grass sucks as well. Is tree bark textures being applied? You can not tell one tree from the next. Oak, Pine, Palm, Dead all look the same.Foliage particularly Bushes clip tree trunks. And seems like editing them in the SDK does nothing to change their appearance.

All the doors are facing the wrong way

Show/Hide Tray ICON SDK is not working

Game log is not part of a Game Save. Loading a save game from the main menu on restart does not restore the game log. The game log should be saved and loaded with the rest of the game state.

Players can't pickup dropped items. There is a dropped sword in Map 1 and the game logs when I walk over it but the item does not get added to my inventory. The player should be able to pickup a dropped item from an adjacent tile by pressing the E key or clicking on it with the mouse. The item should then be added to the player's inventory and removed from the ground.


Conditions should have a duration of until the next rest, until the end of combat, or until a certain number of turns have passed. This would allow for more strategic use of conditions and would also allow for conditions that are meant to last for a certain amount of time rather than being permanent until removed. The SDK should be updated to support setting the duration of conditions as well as the game engine. The game engine should then handle the expiration of conditions based on their duration.
