# Next Plans

## Generic

## SDK

### Custom Font Editor

The SDK Campaign Builder needs a way to specify custom fonts for campaigns.

## Campaign / Story

## Campaign / Story

### Party Member Dialogue — Option A (Talk Button on Character Sheet)

Once a character is recruited, there is currently no way to speak with them.
This plan adds a lightweight in-party conversation system through the existing
character sheet UI.

#### Step 1 — Add `party_dialogue_id` to the character data

- Add an optional `party_dialogue_id: Option<u32>` field to the character
  domain struct and to each premade entry in `characters.ron`
- Assign ids in the 200-series range (201–208 for the eight premade characters)

#### Step 2 — Create party dialogue trees (ids 201–208)

- One short dialogue per character (2–4 nodes) that reflects their current
  story context
- Content drawn from the lore files in
  `campaigns/tutorial/assets/characters/lore/`
- Characters comment on where the party is in the story — something different
  before and after reaching the Astronomer's Temple, for example
- `repeatable: true` so the player can check in at any time

#### Step 3 — Wire a Talk button to the character sheet

- Add a "Talk" button to the character sheet UI (already exists in
  `src/game/systems/`)
- On press: look up the active character's `party_dialogue_id`, open the
  dialogue system with that id
- If `party_dialogue_id` is `None`, show a brief fallback line ("They have
  nothing to say right now.")

#### Out of scope for now

- State-conditional dialogue branches (different text per act) — can be layered
  on top once the basic system exists
- Location-reactive banter (Option C) — separate feature, tracked separately

---

### Eonir the Still — Act IV Boss Fight (Option A)

Eonir is currently an NPC with dialogue (`tutorial_lich_eonir`). To make him
fightable at the end of Act IV while keeping the persuade/fight fork, we use
the classic separate-entity approach: the NPC handles all dialogue and the
`lich_king_eonir` creature handles the boss combat.

#### Step 1 — Add the Lich King creature entry

- Add `lich_king_eonir` to `campaigns/tutorial/data/creatures.ron`
- Give him boss-tier stats befitting a lich who has been still for centuries:
  high endurance, high magic resistance, low speed (he rarely moves)
- Assign regeneration and any special attacks (gravity, shadow barrier) that
  match the `eonir.md` combat description
- Use `creature_id: 1021` — the same ID already on the `tutorial_lich_eonir`
  NPC entry so the two are paired by convention

#### Step 2 — Add a combat dialogue branch to Eonir's dialogue

- In the dialogue file for `dialogue_id: 1003`, add a branch that represents
  the party refusing to negotiate (or failing the persuasion check)
- The combat branch sets an outcome flag, e.g. `eonir_combat_triggered`, that
  the map/event system can react to
- The peaceful branch sets `eonir_relic_returned` and resolves the quest

#### Step 3 — Map event: NPC → monster swap

- On the Frostspire Peaks map, add an event that watches for
  `eonir_combat_triggered`
- When the flag fires: despawn `tutorial_lich_eonir` NPC, spawn
  `lich_king_eonir` monster at the same tile, begin combat
- After combat ends (party wins): set `eonir_defeated`, remove the spawn
  point, trigger the relic-return quest step

#### Step 4 — Post-combat quest resolution

- Both the peaceful path (`eonir_relic_returned`) and the combat path
  (`eonir_defeated`) should converge on the same Act V quest step: the party
  carries the jade icosahedron back to Jyeshtha
- The NPC `tutorial_lich_eonir` should not reappear after either outcome

#### Out of scope for now

- Evil-playthrough support (attacking friendly NPCs, faction hostility) — this
  is tracked as a future Option B/C item
- Multiple boss phases — can be added to `lich_king_eonir` later without
  touching the NPC or dialogue layers

---

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

Fixed - Chests respawn loot when you hit escape.

Fixed - Chest mesh are not showing up.

FIXED - Tutorial Map 4 problems.
Portal from map 4 to map 2 is not working. Going through it takes me back to map 4 instead of map 2. THen I can't walk straight ahead on the map 4 when coming out of the Dark Forrest Portal. I can turn and walk thorough the trees but can not go two tiles straight ahead. When I do try to go straight ahead something is blocking the path. Strangly if you go back to the portal you get ambushed by 2 bandits that are supposed to be on map 2. I have set the destination to mape 2 (18,10) but I can't edit the portal destination location using the mouse in the Campaign Builder becasue the map is offest and no matter how wide I make the SDK window the map is still cut off.

FIXED - Clicking Save Game looks like it saves the game and then puts the characters back to the starting point completely reset like starting a new game. Try to load the save game gives the same results. I can't figure out where the old save games are stored (on Mac) to delete them.

Portraits should support jpg images.

FIXED - Player Overview in game should not try to rescale the length of the full length portrait image. Move the stats under the image to column 2 and let the portrait scale however it needs as long as it is 340 pixels wide. Resize the image to 340 pixels wide if it is not. Dwarfs and Halflings and Gnomes present problems of length so we rescale to 340 pixels wide and let the length scale however it needs.

FIXED - Save Game on map_2 in the middle of map. Load Game puts the party back at the first Inn. Recruitable NPC that were already recruited and are in your party are back to their starting points on the map. Creating duplicate versions of themselves.

FIXED - Campaign Builder --> Importer --> Furniture --> Furniture Mesh Imports are overwriting last imported mesh even after saving. I imported a mesh and it assigned it Furniture ID 13. Next mesh overwrote it as Furniture ID 13.

SDK Campaign Builder adding event Container on map 1. As I add initial items to the container once I get past 4 items I can not see the new items because there are no scroll bars. I should be able to add 100s of items to a container event and be able to scroll through them.
