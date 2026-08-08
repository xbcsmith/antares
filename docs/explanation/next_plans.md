# Next Plans

## Generic

## SDK

### Custom Font Editor

The SDK Campaign Builder needs a way to specify custom fonts for campaigns.

## Game Engine

### Bevy Migration

Migrating to Bevy 0.19.0

[Dependency Upgrade Implementation Plan](./dependency_upgrade_implementation_plan.md)

### Game Tray Icon Implementation Plan

We need to add a tray icon for the game like the ones we added for the SDK.

✅ PLAN WRITTEN - [Game Tray Icon Implementation Plan](./game_tray_icon_implementation_plan.md)

## Bugs

### Barred Passage

The barred passage dialog says you need to find a way to open it. Then we just walked through the gate.

Fixed - Game play issue. Casting First Aid always has no effect in combat.

Fixed - Campaign Builder --> Edit NPC -->  Selecting a Creature ID using the Browse Button is painfully slow and the popup takes forever to render the scroll bars

Chests respawn loot when you hit escape. Chest mesh are not showing up.

FIXED - Tutorial Map 4 problems.
Portal from map 4 to map 2 is not working. Going through it takes me back to map 4 instead of map 2. THen I can't walk straight ahead on the map 4 when coming out of the Dark Forrest Portal. I can turn and walk thorough the trees but can not go two tiles straight ahead. When I do try to go straight ahead something is blocking the path. Strangly if you go back to the portal you get ambushed by 2 bandits that are supposed to be on map 2. I have set the destination to mape 2 (18,10) but I can't edit the portal destination location using the mouse in the Campaign Builder becasue the map is offest and no matter how wide I make the SDK window the map is still cut off.

Clicking Save Game looks like it saves the game and then puts the characters back to the starting point completely reset like starting a new game. Try to load the save game gives the same results. I can't figure out where the old save games are stored (on Mac) to delete them.
