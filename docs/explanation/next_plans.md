# Next Plans

## Generic

## SDK

## Add Mac Os Tray Icon

We need to add a Mac OS tray icon for Antares Campaign Builder SDK. A good macOS tray icon is usually:

square
high-contrast
simple silhouette
readable at 22×22 and 44×44
not overly detailed

Here is the icon @assets/icons/antares_tray.png

Here is the egui code that shows how to set an app icon:

<https://github.com/emilk/egui/blob/a5973e5cac461a23c853cb174b28c8e9317ecce6/crates/eframe/src/native/app_icon.rs#L200>

The @scripts/generate_icons.sh script generates icons for the web and macOS platforms from a source PNG.

Write a plan with a phased approach for adding a Mac OS tray icon. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETED - [MacOS Tray Icon](./macos_tray_icon.md)

## OBJ to RON Conversion

Add an OBJ-to-RON conversion pipeline to the Campaign Builder SDK. Users will
be able to load a Wavefront OBJ file, see each mesh/object-group listed in the
UI, assign colors via a color picker and preset palette, and export the result
as a `CreatureDefinition` RON file (used for both creatures and items). The
default export paths are `assets/creatures/` and `assets/items/` respectively.

✅ COMPLETED - [obj to ron conversion](./obj_to_ron_implementation_plan.md)

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

✅ PLAN WRITTEN - [Armor Classification Expansion Implementation Plan](./armor_classification_expansion_implementation_plan.md)

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

✅ COMPLETED - [Dropped Item Persistence Implementation Plan](./dropped_item_persistence_implementation_plan.md)

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

## Future Features

### Game Log

We need a Game Log. It should be a log that shows all the important events that happen in the game. It should show things like when the player picks up an item, when they talk to an NPC, when they enter a new area, when they take damage, etc. The game log should be visible in the UI and should have a scroll bar so that the player can see past events. The game log should also have a filter so that the player can filter the log by event type (e.g. combat events, dialogue events, item events, etc).

Write a plan with a phased approach to implementing a game log in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Game Log Implementation Plan](./game_log_implementation_plan.md)

### Locked Objects and Keys

We need to implement locked objects and keys in the game engine. Currently there are no locked objects or keys in the game. We should have locked doors, chests, and other containers that require a key to open. The keys should be items that can be found in the world or given as quest rewards. The locked objects should have a locked and unlocked state. When the player interacts with a locked object without the key, they should get a message saying it is locked. When they interact with it with the key, it should unlock and allow them to access whatever is behind it (e.g. a new area, loot, etc). We also need a lockpick skill and lockpicking mechanic that allows the player to attempt to pick the lock on a locked object if they do not have the key. The success of the lockpicking attempt should be based on the player's lockpicking skill and a random chance.

Write a plan with a phased approach to implementing locked objects and keys in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Locked Objects and Keys Implementation Plan](./locked_objects_and_keys_implementation_plan.md)

### Automap and mini map

We need to implement an automap and mini map in the game engine. The automap should be a full map of the current level that is revealed as the player explores. The mini map should be a smaller version of the automap that is always visible in the corner of the screen. The mini map should show the player's current position and the surrounding area. The automap should be accessible from the game menu and should allow the player to see the entire level and their current position on it. The automap should be mapped to the M key and configurable through the game config. We will combine the mini map, compas, and clock into a single UI element in the top right corner of the screen. The mini map should also show important locations like quest objectives, merchants, and points of interest. The automap should have a fog of war effect that hides unexplored areas of the map. The automap should also have a legend that shows what different symbols on the map mean (e.g. red dot for monsters, green dot for merchants, etc).

Write a plan with a phased approach to implementing an automap and mini map in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Automap and Mini Map Implementation Plan](./automap_and_mini_map_implementation_plan.md)

### Doors as Furniture

Doors should be classified as furniture and should be placed on the map as furniture. They should not be classified as walls and should not be placed on the map as walls. They should be placed on the map as furniture and should be placed on the map as furniture.


Doors are environmental objects with state — they can be open/closed/locked, just like chests can be locked/unlocked. The existing FurnitureFlags already has locked: bool and blocking: bool, which are exactly what doors need.

Doors need procedural mesh — Adding FurnitureType::Door (or DoorSingle / DoorDouble) gives doors the same procedural mesh pipeline that already builds detailed 3D geometry for thrones, bookshelves, etc. You'd create a DoorConfig and a spawn_door function alongside the existing ones.

Material support for free — FurnitureMaterial::Wood already has appropriate PBR properties for a wooden door. You could also have Metal gates, Stone doors, etc.

DoorFrameConfig and StructureType::DoorFrame you already have in the architecture are the perfect companion — the frame is a structure, the door panel itself becomes furniture. They compose together on the same tile.

Write a plan with a phased approach to implementing doors as furniture in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Doors as Furniture Implementation Plan](./doors_as_furniture_implementation_plan.md)

### Furniture as RON

We need to create a furniture.ron that defines reusable furniture templates (like "Iron-Bound Dungeon Door" with preset material, scale, tint, flags). Right now the FurnitureType::default_presets() method serves this role in code, but using a data-driven approach would let campaign authors define custom furniture. It would also let us create custom chests, tables, beds, etc.

We should be able to import OBJ files in the Campaign Builder and have them be converted to furniture templates.

We should have all the functionality we have around Items.

Write a plan with a phased approach to implementing furniture as RON in the game engine and the SDK. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Furniture as RON Implementation Plan](./furniture_as_ron_implementation_plan.md)

### Unconcious Characters and Dead Characters

Characters with 0 HP are unconcious and should not be allowed to attack monsters. They should also not be allowed to be attacked by monsters. They should be able to be healed by other characters. Unconcious IS a condition. It is a special condition because of combat implecations. We should add it to the Conditions in a Campaign. CHaracters remain unconcious until they are healed with a Spell, Scroll, or by resting. We should also add Dead to the Conditions in a Campaign. I haven't yet because it requires a lot of wiring because you need to resurrect dead characters either with a Spell, Scroll, or a Priest/Priestess. Dead should be able to be permanent if the Campaign creator wants it to be. We can also add Uncoincious and Dead as conditions from a Spell or Scroll or Consumable.The default should be "until ressurected". The default template for conditions in the SDK should include Unconcious and Dead so that Creators do not forget to add them to their campaigns.

Write a plan with a phased approach to implementing unconcious characters in the game engine. THINK HARD and follow the rules in @PLAN.md

[Unconcious Characters Implementation Plan](./unconcious_characters_implementation_plan.md)

### Notes

Month Year Date in Game Engine View looks horrible.

Trees are still horrible. Grass sucks as well. Is tree bark textures being applied? The trees on Map 1 look no different than before.
You can not tell one tree from the next. Oak, Pine, Palm, Dead all look the same.
Foliage particularly Bushes clip tree trunks. And seems like editing them in the SDK does nothing to change their appearance.


Time does not advance when the party moves. The clock only ever increments the hour when resting. Time should advance every time the party moves a tile (the minutes should advance). Time should advance when the party travels between maps.
