# Next Plans

## Generic

## SDK

## Game Engine

### Clean up

Analyze this codebase for refactoring opportunities:

1. Find duplicate code patterns
2. Identify unused exports and dead code
3. Review error handling consistency
4. Check for security vulnerabilities

Compile the findings into a prioritized action plan with a phased approach.

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

✅ PLAN WRITTEN - [Game Wide Mouse Input Support](./game_wide_mouse_input_support_plan.md)

### Game Log

We need a Game Log. It should be a log that shows all the important events that happen in the game. It should show things like when the player picks up an item, when they talk to an NPC, when they enter a new area, when they take damage, etc. The game log should be visible in the UI and should have a scroll bar so that the player can see past events. The game log should also have a filter so that the player can filter the log by event type (e.g. combat events, dialogue events, item events, etc).

Write a plan with a phased approach to implementing a game log in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Game Log Implementation Plan](./game_log_implementation_plan.md)

### Automap and mini map

We need to implement an automap and mini map in the game engine. The automap should be a full map of the current level that is revealed as the player explores. The mini map should be a smaller version of the automap that is always visible in the corner of the screen. The mini map should show the player's current position and the surrounding area. The automap should be accessible from the game menu and should allow the player to see the entire level and their current position on it. The automap should be mapped to the M key and configurable through the game config. We will combine the mini map, compas, and clock into a single UI element in the top right corner of the screen. The mini map should also show important locations like quest objectives, merchants, and points of interest. The automap should have a fog of war effect that hides unexplored areas of the map. The automap should also have a legend that shows what different symbols on the map mean (e.g. red dot for monsters, green dot for merchants, etc).

Write a plan with a phased approach to implementing an automap and mini map in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Automap and Mini Map Implementation Plan](./automap_and_mini_map_implementation_plan.md)

### Unconcious Characters and Dead Characters

Characters with 0 HP are unconcious and should not be allowed to attack monsters. They should also not be allowed to be attacked by monsters. They should be able to be healed by other characters. Unconcious IS a condition. It is a special condition because of combat implecations. We should add it to the Conditions in a Campaign. Characters remain unconcious until they are healed with a Spell, Scroll, or by resting. We should also add Dead to the Conditions in a Campaign. I haven't yet because it requires a lot of wiring because you need to resurrect dead characters either with a Spell, Scroll, or a Priest/Priestess. Dead should be able to be permanent if the Campaign creator wants it to be. We can also add Uncoincious and Dead as conditions from a Spell or Scroll or Consumable.The default should be "until ressurected". The default template for conditions in the SDK should include Unconcious and Dead so that Creators do not forget to add them to their campaigns.

Write a plan with a phased approach to implementing unconcious characters in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETED - [Unconcious Characters Implementation Plan](./unconcious_characters_implementation_plan.md)

### Game Tray Icon Implementation Plan

We need to add a tray icon for the game like the ones we added for the SDK.

✅ PLAN WRITTEN - [Game Tray Icon Implementation Plan](./game_tray_icon_implementation_plan.md)

## Future Features and Fixes

### Notes

Month Year Date in Game Engine View looks horrible.

Trees are still horrible. Grass sucks as well. Is tree bark textures being applied? The trees on Map 1 look no different than before.
You can not tell one tree from the next. Oak, Pine, Palm, Dead all look the same.
Foliage particularly Bushes clip tree trunks. And seems like editing them in the SDK does nothing to change their appearance.

Time does not advance when the party moves. The clock only ever increments the hour when resting. Time should advance every time the party moves a tile (the minutes should advance). Time should advance when the party travels between maps.

Navigating the locked item menu doesn't work like expected. Once a character is seslected the arrow keys move the party instead of moving the selected values. ESC always brings up the game menu when it should ESC the dialog/menu/inventory screen without bringing up the main menu

Mini Map follows the party but does not update with images of the actual map. The party indicator runs out of the box after the party moves a few tile. Same problem with the actual map exists. I went and visited the merchant and got no updates. Datetime has disappeared. The datetime is supposed to be show as well as the compass. See examples of the minimap @screenshots/minimap_blank.png and the full map blank @screenshots/full_map_blank.png

All the doors are facing the wrong way

Show/Hide Tray ICON SDK is not working
