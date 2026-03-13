# Next Plans

## Generic

## SDK

## OBJ to RON Conversion

Add an OBJ-to-RON conversion pipeline to the Campaign Builder SDK. Users will
be able to load a Wavefront OBJ file, see each mesh/object-group listed in the
UI, assign colors via a color picker and preset palette, and export the result
as a `CreatureDefinition` RON file (used for both creatures and items). The
default export paths are `assets/creatures/` and `assets/items/` respectively.

✅ COMPLETE - [obj to ron conversion](./obj_to_ron_implementation_plan.md)

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

### Consumable Items

Out-of-combat item use is not implemented. The inventory UI (GameMode::Menu / GameMode::Inventory) only supports Drop and Transfer actions. There is no "Use" action wired to exploration or menu mode. All ConsumableEffect variants (HealHp, RestoreSp, CureCondition, BoostAttribute, BoostResistance) can only be triggered during a combat turn via the UseItemAction message. A player cannot drink a Potion of Fire Resistance, a Healing Potion, or a stat-boosting potion from the inventory screen between fights. This gap affects every consumable type equally. To fix this, an out-of-combat item execution path is needed:

- Add a `UseItemAction` (or equivalent) message type for exploration/menu mode.
- Implement `apply_consumable_effect(character: &mut Character, effect: ConsumableEffect)` as a pure domain function that does not require a `CombatState` — mirroring the logic in `execute_item_use_by_slot` but operating directly on a `Character`.
- Wire a "Use" keybind (e.g. Enter/U) in `inventory_input_system` that fires the new action when the focused item is a consumable.
- Handle the action in a new Bevy system (`handle_use_item_action_exploration`) that applies the effect to the selected character and consumes the inventory charge.
- Confirm `is_combat_usable: false` items are blocked in combat but allowed in exploration.
- Add appropriate feedback (status message or log line) so the player knows the item was consumed.

Write a plan with a phased approach to implementing consumable items out-of-combat. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETED - [consumables outside combat](./consumables_outside_combat_plan.md)

### Consumable Duration and Timed Resistance Effects

Now that the time system (`GameTime`, `advance_time()`, `ActiveSpells::tick()`) is in place, consumable items that boost resistances or attributes should support a timed duration that expires as in-game time passes. Currently `BoostResistance` and `BoostAttribute` permanently modify `character.resistances.*` and `character.stats.*` current values with no expiry. This is acceptable inside combat (state resets at end of combat) but is wrong for out-of-combat use where time flows.

What exists today

- `ActiveSpells` on `GameState` already tracks party-wide timed protections (`fire_protection`, `cold_protection`, `electricity_protection`, `magic_protection`, `fear_protection`, `psychic_protection`, etc.) as `u8` minute countdowns.
- `GameState::advance_time(minutes)` calls `active_spells.tick()` once per minute, decrementing every counter via `saturating_sub(1)`.
- `ConditionDuration::Minutes(u16)` exists on the conditions system with `tick_minute()` already implemented, suitable for per-character timed effects.

What needs to change

**1. Add `duration_minutes: Option<u16>` to `ConsumableData`** (`src/domain/items/types.rs`)

```rust
pub struct ConsumableData {
    pub effect: ConsumableEffect,
    pub is_combat_usable: bool,
    /// Duration in game-world minutes. `None` = instant / permanent.
    /// Used by `BoostResistance` and `BoostAttribute` to expire via `advance_time`.
    #[serde(default)]
    pub duration_minutes: Option<u16>,
}
```

Use `#[serde(default)]` so all existing RON item files deserialize without modification (`None` = permanent, matching current behaviour).

**2. Route `BoostResistance` through `ActiveSpells` out of combat**

When a resistance potion is consumed outside combat, apply the boost to the corresponding `active_spells` field rather than directly to `character.resistances`:

| `ResistanceType`  | `ActiveSpells` field     |
| ----------------- | ------------------------ |
| Fire              | `fire_protection`        |
| Cold              | `cold_protection`        |
| Electricity       | `electricity_protection` |
| Energy            | `magic_protection`       |
| Fear              | `fear_protection`        |
| Physical          | `magic_protection`       |
| Paralysis / Sleep | `psychic_protection`     |

Set the field to `duration_minutes` (saturating at `u8::MAX`). `advance_time` then expires it automatically — no cleanup code required.

**3. Route `BoostAttribute` through per-character timed conditions out of combat**

Stat-boosting potions (Might, Speed, etc.) affect individual characters, not the whole party, so `ActiveSpells` is not the right home. Instead:

- Add a `timed_stat_boosts: Vec<TimedStatBoost>` field to `Character` (or reuse `ActiveCondition` with a new `ConditionEffect::BoostAttribute` variant).
- Each entry stores `(AttributeType, i8, minutes_remaining: u16)`.
- Wire `tick_minute()` on `Character` to decrement and remove expired boosts, reversing the `current` value change when they expire.
- `advance_time` must call `tick_minute()` on every party member.

**4. Campaign Builder — duration field in Items editor**

Add a `Duration (minutes)` `DragValue` (0 = permanent) to the Consumable Properties section of `show_type_editor` in `sdk/campaign_builder/src/items_editor.rs`, visible only when the effect is `BoostResistance` or `BoostAttribute`. Update the preview panel to show `"(60 min)"` beside the effect string when a duration is set.

**5. Backward compatibility**

- All existing `.ron` item files omit `duration_minutes` → deserialize as `None` → permanent behaviour unchanged.
- In-combat use of `BoostResistance` may continue to write directly to `character.resistances` (combat resets on exit); the duration field is advisory for out-of-combat use.

#### Phased approach

| Phase | Scope                                                                                                                                                     |
| ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A     | Add `duration_minutes: Option<u16>` to `ConsumableData`; update all existing struct literals with `..Default::default()` or explicit `None`; update tests |
| B     | Implement out-of-combat `apply_consumable_effect` pure domain function; route `BoostResistance` to `ActiveSpells` with duration                           |
| C     | Per-character timed stat boosts for `BoostAttribute`; wire `tick_minute` into `advance_time`                                                              |
| D     | Campaign Builder UI — duration field in Items editor                                                                                                      |
| E     | Wire "Use" keybind in inventory screen (depends on out-of-combat item use plan above)                                                                     |

Write a plan with a phased approach to implementing consumable effects. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETED - [consumable duration effects](./consumable_duration_effects_plan.md)

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

✅ COMPLETED - [Equipped Weapon Damage Implementation Plan](./equipped_weapon_damage_implementation_plan.md)

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

✅ PLAN WRITTEN - [Dropped Item Persistence Implementation Plan](./dropped_item_persistence_implementation_plan.md)

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

### Combat

We need types of combat events. The party should not always be able to see the monster. We should have different types of combat events. For example, an ambush event where the monster is hidden and the party does not know it is there until it attacks. We should have different types of combat events that can be triggered by different conditions. For example, an ambush event that is triggered when the party enters a certain tile or a certain monster is nearby. The ambush event would cause the monster to attack the party without being visible on the map until it attacks. Another example are ambushes where the party is resting and the monster attacks them while they are resting (occurance should be configurable at the map level). The Capmaign Builder needs to support settign adn editing the combat event type for each encounter in the map.ron file.

Normal Combat - Party sees the monster and can choose to attack or flee. Combat proceeds as normal.
Ambush Combat - Party does not see the monster until it attacks. Party misses the first round because they do not see the monster. After the first round the monster becomes visible and combat proceeds as normal.
Ranged Combat - Party sees the monster and can choose to attack or flee. The monster can attack from a distance and the party can choose to attack from a distance if they have a ranged weapon. Combat proceeds as normal but with the option for ranged attacks.
Magic Combat - Party sees the monster and can choose to attack or flee. The monster can attack with magic and the party can choose to attack with magic if they have a spell equipped. Combat proceeds as normal but with the option for magic attacks.
Boss Combat - Party sees the monster and can choose to attack or flee. The monster is a boss and has special abilities and mechanics. Combat proceeds as normal but with the added complexity of the boss mechanics.

Write a plan with a phased approach to implementing different types of combat events in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETED - [Combat Events Implementation Plan](./combat_events_implementation_plan.md)

### Locked Objects and Keys

We need to implement locked objects and keys in the game engine. Currently there are no locked objects or keys in the game. We should have locked doors, chests, and other containers that require a key to open. The keys should be items that can be found in the world or given as quest rewards. The locked objects should have a locked and unlocked state. When the player interacts with a locked object without the key, they should get a message saying it is locked. When they interact with it with the key, it should unlock and allow them to access whatever is behind it (e.g. a new area, loot, etc). We also need a lockpick skill and lockpicking mechanic that allows the player to attempt to pick the lock on a locked object if they do not have the key. The success of the lockpicking attempt should be based on the player's lockpicking skill and a random chance.

Write a plan with a phased approach to implementing locked objects and keys in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Locked Objects and Keys Implementation Plan](./locked_objects_and_keys_implementation_plan.md)

### Months and Years in the Time System

The current time system tracks day, hour, and minute only (`GameTime { day, hour, minute }`).
Many classic RPGs give the world a richer sense of history and season by also tracking months and
years. This section describes what would need to change to add that support.

Motivation

- Campaign authors may want events that trigger "in winter" or "after year 2".
- The HUD clock and any in-game calendar UI benefit from displaying a full date
  (e.g. "Day 3, Month 2, Year 4" or "3rd of Frostmoon, Year 4").
- Long-running campaigns feel more alive when the world ages alongside the player.

Required Changes

1. Extend `GameTime`

Add `month: u32` and `year: u32` fields to `GameTime` in `src/domain/types.rs`:

```rust
pub struct GameTime {
    pub year:   u32,   // 1-based
    pub month:  u32,   // 1-based
    pub day:    u32,   // 1-based within month
    pub hour:   u8,    // 0–23
    pub minute: u8,    // 0–59
}
```

Keep the existing `GameTime::new(day, hour, minute)` constructor as a convenience
alias that defaults `year = 1, month = 1` for backward compatibility.

1. Add Calendar Constants

Add a `Calendar` struct (or free constants) to define the shape of the in-game year:

```rust
pub const MONTHS_PER_YEAR: u32 = 12;
pub const DAYS_PER_MONTH:  u32 = 30;   // or per-month array for unequal months
pub const DAYS_PER_YEAR:   u32 = MONTHS_PER_YEAR * DAYS_PER_MONTH;
```

Campaign authors could override these defaults via `CampaignConfig` if they want
a world with 13 months of 28 days, for example.

Optional: add named months to `CampaignConfig` so the HUD can display
"Frostmoon" instead of "Month 1".

1. Update `advance_time()`

`GameState::advance_time(minutes)` currently rolls days from hours. It must also
roll months from days, and years from months, using the calendar constants.

4. Extend `TimeCondition`

Add year- and month-aware variants to the `TimeCondition` enum used by map events:

```rust
pub enum TimeCondition {
    DuringPeriods(Vec<TimeOfDay>),
    AfterDay(u32),
    BeforeDay(u32),
    BetweenHours { from: u8, to: u8 },
    // New:
    DuringMonths(Vec<u32>),    // e.g. winter = months 11, 12, 1
    AfterYear(u32),
    BeforeYear(u32),
    BetweenYears { from: u32, to: u32 },
}
```

1. Update the HUD Clock

`ClockDayText` would become `ClockDateText` (or split into separate components
for day, month, year). The `update_clock` system formats a full date string.
A `period_label`-style helper can format month names when the campaign defines them.

1.  Campaign Builder — Starting Date

`CampaignConfig::starting_time` already has `day`, `hour`, `minute`. Extend to
include `year` and `month`. The Campaign Builder's Gameplay section and
`CampaignMetadataEditBuffer` gain two new `DragValue` fields (`starting_year`,
`starting_month`) following the same pattern as `starting_day`/`starting_hour`.

1. Save / Load

`GameTime` is serialized as part of `GameState`. Adding fields with
`#[serde(default)]` keeps existing save files loading without error (they
deserialize `year = 0` / `month = 0` which can be clamped to 1 on load).

Phased Approach

| Phase | Scope                                                                            |
| ----- | -------------------------------------------------------------------------------- |
| A     | Extend `GameTime`, add calendar constants, update `advance_time()`, update tests |
| B     | Add month/year `TimeCondition` variants, update event evaluation                 |
| C     | Update HUD clock, add optional named-month support to `CampaignConfig`           |
| D     | Campaign Builder UI — starting year/month drag values, Files section             |
| E     | Named months editor in Campaign Builder (optional quality-of-life)               |

Write a detailed plan before implementing. Follow the rules in PLAN.md and
AGENTS.md. All existing tests must continue to pass; add new tests for rollover
logic (minute→hour→day→month→year) and for each new `TimeCondition` variant.

✅ COMPLETE - [Time System Extension Plan](./time_system_extension_plan.md)
✅ COMPLETE - [Months Years Plan](./months_years_implementation_plan.md)


### Automap and mini map

We need to implement an automap and mini map in the game engine. The automap should be a full map of the current level that is revealed as the player explores. The mini map should be a smaller version of the automap that is always visible in the corner of the screen. The mini map should show the player's current position and the surrounding area. The automap should be accessible from the game menu and should allow the player to see the entire level and their current position on it. The automap should be mapped to the M key and configurable through the game config. We will combine the mini map, compas, and clock into a single UI element in the top right corner of the screen. The mini map should also show important locations like quest objectives, merchants, and points of interest. The automap should have a fog of war effect that hides unexplored areas of the map. The automap should also have a legend that shows what different symbols on the map mean (e.g. red dot for monsters, green dot for merchants, etc).

Write a plan with a phased approach to implementing an automap and mini map in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Automap and Mini Map Implementation Plan](./automap_and_mini_map_implementation_plan.md)

### Food System

Currently resting depends on food rations of the party. Currently there is no way to obtain food rations in the game. Characters start with X number of Food Rations and never get anymore. InnKeepers and Merchants should sell food. The food rations will replenish the characters food ration. So if you have 4 characters in your party, you will need 4 food rations to rest for 1 night. If you have 2 characters in your party, you will need 2 food rations to rest for 1 night. Food Items are regular items (like consumables) that can be marked as food in the item editor. So I could have an apple, a steak, bread, eggs, etc. All equal 1 food ration.

Food Ration should be a consumable item with a condition "can rest".

Write a plan with a phased approach to implementing a food system in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETE - [Food System Implementation Plan](./food_system_implementation_plan.md)

### Unified Creature Asset Binding (`creature_id` Standard)

#### Problem

There are currently three separate mechanisms for tying an entity to a creature
asset (the procedural mesh definition in `creatures.ron`), and none of them are
consistent:

| Entity type           | Binding mechanism                                                                                     | Field                             |
| --------------------- | ----------------------------------------------------------------------------------------------------- | --------------------------------- |
| `MonsterDefinition`   | Direct field                                                                                          | `visual_id: Option<CreatureId>`   |
| `NpcDefinition`       | Direct field                                                                                          | `creature_id: Option<CreatureId>` |
| `CharacterDefinition` | **None** — resolved at runtime by a fragile name-match heuristic in `resolve_recruitable_creature_id` | (absent)                          |

`MonsterDefinition` uses a different field name (`visual_id`) from `NpcDefinition`
(`creature_id`) for the same concept. `CharacterDefinition` has no field at all;
the rendering system works around this with a three-step runtime fallback:

1. Look up the `character_id` in `NpcDefinition.creature_id` (re-uses the NPC
   system as a proxy).
2. Look up `CharacterDefinition.name` and fuzzy-match it against
   `CreatureDefinition.name` after stripping non-alphanumeric characters and
   lowercasing both sides.
3. Fall back to the placeholder sprite (`sprites/placeholders/npc_placeholder.png`).

Steps 1 and 2 are silent — no compile-time or load-time error is emitted when
the mapping is missing or ambiguous. A campaign author who names a character
`"Old Gareth"` in `characters.ron` and a creature `"old gareth"` in
`creatures.ron` gets a working game; one who uses `"Gareth the Old"` gets a
placeholder sprite with no warning beyond a log message.

The `visual_id` / `creature_id` naming split also means every system that reads
one of these fields must know which struct it is dealing with. There is no shared
trait or convention.

#### Goal

Standardise on a single field name **`creature_id: Option<CreatureId>`** across
all three definition types, and remove the runtime name-match heuristic entirely.

- `MonsterDefinition.visual_id` → renamed to `creature_id`.
- `NpcDefinition.creature_id` → kept as-is (already the right name).
- `CharacterDefinition` → add `creature_id: Option<CreatureId>` as a new
  `#[serde(default)]` field.

After this change the rendering system can use one uniform lookup:

```rust
// Same call works for all three definition types
let creature_id = def.creature_id;
```

`resolve_recruitable_creature_id` is deleted. Its replacement is a one-liner:
`character_def.creature_id`.

#### Scope

**Files to modify:**

- `src/domain/world/monster.rs` — rename `visual_id` → `creature_id` on
  `MonsterDefinition`. Update all `serde` annotations. Remove any migration
  shim for the old name (backwards compatibility is not required).
- `src/domain/character_definition.rs` — add `pub creature_id:
Option<CreatureId>` to `CharacterDefinition` with `#[serde(default)]`.
  Update `CharacterDefinitionBuilder` (if present) to expose the new field.
- `src/domain/world/npc.rs` — no structural change; verify field name is
  already `creature_id` and doc comment is accurate.
- `src/game/systems/map.rs`:
  - Delete `resolve_recruitable_creature_id`.
  - Delete `resolve_encounter_creature_id` or rewrite it to read
    `monster_def.creature_id` directly without the `visual_id` alias.
  - In the `RecruitableCharacter` spawn branch: replace the `resolve_*` call
    with a direct lookup of `character_def.creature_id`.
  - In the `Encounter` spawn branch: replace the `resolve_*` call with a direct
    lookup of `monster_def.creature_id` (first monster in the group that has
    `creature_id: Some(_)`).
  - In the NPC spawn branch: no change required (already reads `npc_def.creature_id`).
- `data/test_campaign/data/monsters.ron` — rename `visual_id` → `creature_id`
  in all fixture entries.
- `campaigns/tutorial/data/monsters.ron` — rename `visual_id` → `creature_id`
  in all entries.
- Any other `.ron` files in `data/` or `campaigns/` that contain `visual_id:`.
- All tests that construct `MonsterDefinition` directly — update field name.
- `docs/reference/architecture.md` — update the data structure tables to reflect
  the unified field name.
- `sdk/campaign_builder/src/monsters_editor.rs`:
  - Rename `visual_id: None` → `creature_id: None` in `default_monster()`.
  - Add a "Creature ID" row to `show_form` (numeric display + Browse picker +
    Clear button + tooltip) — this field is **currently not exposed in the UI
    at all** and must be added.
  - Show resolved creature name in `show_monster_details` and
    `show_preview_static`.
- `sdk/campaign_builder/src/characters_editor.rs`:
  - Add `pub creature_id: String` to `CharacterEditBuffer`.
  - Populate it in `start_edit_character`.
  - Write it back in `save_character` (parse to `Option<CreatureId>`).
  - Add a "Creature ID" row to the character edit form (Browse/Clear/tooltip)
    in the Visual section near `portrait_id`.
  - Show creature name in the character list view and preview panel.
- `sdk/campaign_builder/src/npc_editor.rs`:
  - No structural changes needed (field already exists end-to-end).
  - Upgrade the raw `TextEdit::singleline` creature_id input to the consistent
    Browse/Clear/tooltip picker pattern used in the other two editors.
  - Show resolved creature name in `show_preview` alongside the numeric ID.
- `sdk/campaign_builder/src/advanced_validation.rs`:
  - Rename `visual_id: Some(id)` → `creature_id: Some(id)` in
    `create_test_monster`.

**Files to create:**

None. This is a rename-and-add refactor, not a new subsystem.

#### Campaign Builder Editor Updates

Every Campaign Builder editor that exposes one of these definition types must
be updated to match the domain changes. The current state of each editor is
documented below so nothing is missed.

**`sdk/campaign_builder/src/monsters_editor.rs` — Monsters Editor**

This is the most work. Two separate problems exist:

1. **Field rename in the struct initialiser**: `default_monster()` constructs a
   `MonsterDefinition` with `visual_id: None`. After the domain rename this must
   become `creature_id: None`. The same applies to the `advanced_validation.rs`
   test helper `create_test_monster` which also sets `visual_id: Some(id)`.

2. **Missing UI field — `visual_id` / `creature_id` is not exposed in
   `show_form` at all.** There is no text input, label, or picker for this field
   in the Add/Edit form. A campaign author currently has no way to assign a
   creature asset to a monster through the editor; they must hand-edit the RON
   file. This must be fixed as part of this work.

   Add a "Creature ID" row to `show_form` in the Identity section (near the
   Name and ID fields):

   - A read-only numeric display showing the current `creature_id` value (or
     "None" when absent).
   - A `DragValue` or `TextEdit` (parsing to `Option<CreatureId>`) for direct
     numeric entry.
   - A **"Browse…"** button that opens a modal `ComboBox` or searchable list
     populated from `CreatureAssetManager::list_creature_references()`, showing
     each creature's ID and name side by side so the author can pick one without
     knowing the raw number.
   - A **"Clear"** button that sets the field back to `None`.
   - An `ℹ️` tooltip explaining: "Links this monster to a procedural mesh
     creature definition. When set, the monster spawns as a 3-D creature mesh
     on the map instead of a sprite placeholder."
   - The `show_monster_details` and `show_preview_static` panels should also
     display the resolved creature name (looked up from the `CreatureAssetManager`)
     alongside the numeric ID, or "No creature asset" when `None`.

**`sdk/campaign_builder/src/characters_editor.rs` — Characters Editor**

`CharacterDefinition` currently has no `creature_id` field; `CharacterEditBuffer`
has no corresponding buffer string; and `save_character` does not write one.
Three coordinated changes are required:

1. Add `pub creature_id: String` to `CharacterEditBuffer` (empty string = `None`).
2. In `start_edit_character`, populate the buffer:
   `creature_id: character.creature_id.map_or(String::new(), |id| id.to_string())`.
3. In `save_character`, parse it back:
   `creature_id: if self.buffer.creature_id.is_empty() { None } else { self.buffer.creature_id.parse::<CreatureId>().ok() }`.
4. Add a "Creature ID" row to the character edit form (same Browse/Clear/tooltip
   pattern described for the Monsters Editor above). Place it in the Visual
   section of the form near `portrait_id`.
5. The character list view and the preview panel should display the creature name
   (or "No creature asset") alongside the portrait.

**`sdk/campaign_builder/src/npc_editor.rs` — NPC Editor**

The NPC editor already has a working `creature_id` text field in
`NpcEditBuffer`, `start_edit_npc`, `save_npc`, and `show_preview`. No
structural changes are required by the domain rename.

However, the field is currently a raw `TextEdit::singleline` that accepts free
text. Upgrade it to match the same Browse/Clear/tooltip pattern used in the
Monsters and Characters editors so the experience is consistent:

- Replace the raw text edit with a read-only numeric display + **"Browse…"**
  button that opens the creature picker.
- Keep the raw numeric text entry as a fallback (for keyboard-first users).
- Show the resolved creature name in `show_preview` alongside the numeric ID.

**`sdk/campaign_builder/src/advanced_validation.rs` — Validation**

The `create_test_monster` helper at the bottom of this file sets
`visual_id: Some(id)`. Rename to `creature_id: Some(id)` to match the domain
change. No logic changes needed.

**Campaign Builder Testing Requirements**

- Add a test in `monsters_editor.rs`:
  `test_monsters_editor_creature_id_roundtrips_through_form` — set
  `edit_buffer.creature_id = Some(42)`, call `save_form`, reload into the
  buffer via `start_edit_monster`, assert the buffer shows `"42"`.
- Add a test in `characters_editor.rs`:
  `test_characters_editor_creature_id_roundtrips_through_form` — same pattern
  for `CharacterDefinition`.
- Verify that the NPC editor's existing `test_start_edit_npc_populates_sprite_fields`
  and `test_save_npc_edit_mode` tests still pass unchanged (they already cover
  `creature_id` round-trip for NPCs).

#### Data Migration

Because backwards compatibility is explicitly not required, the rename is a
hard cut. Every `monsters.ron` file must be updated to use `creature_id` before
the build will pass. `cargo check` will catch any missed RON files at
deserialisation time (test suite loads all campaign data).

#### Shared Trait (Optional but Recommended)

Define a small trait in `src/domain/world/creature_binding.rs`:

```rust
pub trait CreatureBound {
    fn creature_id(&self) -> Option<CreatureId>;
}
```

Implement it for `MonsterDefinition`, `NpcDefinition`, and
`CharacterDefinition`. This makes the rendering system's lookup site
type-generic and prevents future drift.

#### Rendering System Simplification

After the migration `spawn_map` no longer needs two resolve helpers. The spawn
branches become symmetric:

| Branch                        | Lookup                                                |
| ----------------------------- | ----------------------------------------------------- |
| `Encounter`                   | `monster_def.creature_id` (first monster with `Some`) |
| `NpcDialogue` / NPC placement | `npc_def.creature_id`                                 |
| `RecruitableCharacter`        | `character_def.creature_id`                           |

All three fall back to the placeholder sprite path when `creature_id` is `None`,
using the same fallback block (no special-cased heuristics).

#### Testing Requirements

- All existing map-rendering tests that exercise recruitable or encounter
  visual spawning must continue to pass after the migration.
- Add a test: `test_character_definition_creature_id_field_roundtrips_ron` —
  serialise a `CharacterDefinition` with `creature_id: Some(7)` to RON and
  deserialise it back; assert the value survives the round-trip.
- Add a test: `test_monster_definition_creature_id_field_roundtrips_ron` —
  same pattern for the renamed field on `MonsterDefinition`.
- Add a test: `test_recruitable_spawn_uses_character_def_creature_id` — build
  a minimal `App` with a `CharacterDefinition` that has `creature_id: Some(N)`
  and a matching `CreatureDefinition`; assert that `spawn_map` produces a
  `CreatureVisual { creature_id: N }` entity for the recruitable event (no
  name-match heuristic involved).
- `cargo nextest run --all-features` must pass with zero failures.

#### Success Criteria

- `MonsterDefinition`, `NpcDefinition`, and `CharacterDefinition` all expose
  `creature_id: Option<CreatureId>`.
- The field `visual_id` no longer exists anywhere in the codebase (source,
  tests, RON data files, and SDK editor code).
- `resolve_recruitable_creature_id` no longer exists.
- Campaign data in `data/test_campaign/` and `campaigns/tutorial/` uses
  `creature_id` in all monster entries.
- **Monsters Editor**: `show_form` exposes a working "Creature ID" Browse/Clear
  field that round-trips correctly through save and reload. The field was
  previously absent from the UI entirely.
- **Characters Editor**: `CharacterEditBuffer` carries `creature_id`; `save_character`
  writes it to `CharacterDefinition`; the edit form exposes it with a Browse/Clear
  picker in the Visual section.
- **NPC Editor**: the creature_id input has been upgraded from a raw text edit to
  a consistent Browse/Clear/tooltip picker; resolved creature name appears in the
  preview panel.
- All Campaign Builder editor tests pass, including the two new round-trip tests
  for Monsters and Characters editors.
- All quality gates pass: `cargo fmt`, `cargo check`, `cargo clippy -D warnings`,
  `cargo nextest run --all-features`.

Write a plan with a phased approach to implementing unified creature asset
binding in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETE - [Unified Creature Asset Binding Implementation Plan](./unified_creature_asset_binding_implementation_plan.md)
