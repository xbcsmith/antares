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

Fixed - The barred passage dialog says you need to find a way to open it. Then we just walked through the gate.

Fixed - Game play issue. Casting First Aid always has no effect in combat.

Fixed - Campaign Builder --> Edit NPC -->  Selecting a Creature ID using the Browse Button is painfully slow and the popup takes forever to render the scroll bars

Chests respawn loot when you hit escape. Chest mesh are not showing up.

FIXED - Tutorial Map 4 problems.
Portal from map 4 to map 2 is not working. Going through it takes me back to map 4 instead of map 2. THen I can't walk straight ahead on the map 4 when coming out of the Dark Forrest Portal. I can turn and walk thorough the trees but can not go two tiles straight ahead. When I do try to go straight ahead something is blocking the path. Strangly if you go back to the portal you get ambushed by 2 bandits that are supposed to be on map 2. I have set the destination to mape 2 (18,10) but I can't edit the portal destination location using the mouse in the Campaign Builder becasue the map is offest and no matter how wide I make the SDK window the map is still cut off.

FIXED - Clicking Save Game looks like it saves the game and then puts the characters back to the starting point completely reset like starting a new game. Try to load the save game gives the same results. I can't figure out where the old save games are stored (on Mac) to delete them.

Portraits should support jpg images.

FIXED - Player Overview in game should not try to rescale the length of the full length portrait image. Move the stats under the image to column 2 and let the portrait scale however it needs as long as it is 340 pixels wide. Resize the image to 340 pixels wide if it is not. Dwarfs and Halflings and Gnomes present problems of length so we rescale to 340 pixels wide and let the length scale however it needs.

FIXED - Save Game on map_2 in the middle of map. Load Game puts the party back at the first Inn. Recruitable NPC that were already recruited and are in your party are back to their starting points on the map. Creating duplicate versions of themselves.

FIXED - Campaign Builder --> Importer --> Furniture --> Furniture Mesh Imports are overwriting last imported mesh even after saving. I imported a mesh and it assigned it Furniture ID 13. Next mesh overwrote it as Furniture ID 13.
