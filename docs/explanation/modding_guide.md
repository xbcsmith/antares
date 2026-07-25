# Antares Modding Guide

**Target Audience**: Campaign creators, modders, content designers
**Difficulty**: Intermediate to Advanced

This guide explains the concepts, patterns, and best practices for creating content for Antares RPG.

---

## Table of Contents

1. [Introduction](#introduction)
2. [Core Concepts](#core-concepts)
3. [Campaign Architecture](#campaign-architecture)
4. [Content Design Patterns](#content-design-patterns)
5. [Interactive Objects — Meshes and Events](#interactive-objects--meshes-and-events)
6. [Advanced Techniques](#advanced-techniques)
7. [Balancing Guidelines](#balancing-guidelines)
8. [Performance Considerations](#performance-considerations)
9. [Publishing Your Mod](#publishing-your-mod)

---

## Introduction

### What is a Mod?

In Antares, a **mod** (modification) is a custom campaign that extends or replaces the core game content. Mods can include:

- New character classes and races
- Custom items and equipment
- Original monsters and encounters
- Unique spells and abilities
- Complete story campaigns
- Gameplay modifications

### Mod Types

**Total Conversion**: Replaces all core content with custom content.

**Campaign**: Adds new story content while using core classes/races/items.

**Content Pack**: Adds specific content types (e.g., 50 new items, 20 new monsters).

**Balance Mod**: Tweaks existing content for different difficulty or gameplay style.

---

## Core Concepts

### Data-Driven Design

Antares uses a **data-driven architecture**. This means:

- Game logic is in Rust code (the engine)
- Game content is in RON data files (your mod)
- You modify content, not code
- No programming required for most mods

**Example**: To add a new weapon, you edit `items.ron`, not Rust source code.

### Entity-Component Pattern

Content entities (characters, items, monsters) are composed of:

- **Core Attributes**: ID, name, basic stats
- **Components**: Optional features (spells, special abilities, bonuses)
- **Modifiers**: Temporary or permanent stat changes

**Example**: A "Flaming Sword" is a weapon (core) + fire damage bonus (component).

### ID System

Every entity has a unique numeric ID:

```rust
ItemId = u32
MonsterId = u32
SpellId = u32
MapId = u32
// etc.
```

**Rules**:

- IDs must be unique within their type
- IDs can be any positive integer
- Use consistent numbering schemes (e.g., weapons 1-100, armor 101-200)
- Document your ID ranges

### Disablement System

The **Disablement** bitfield controls class/race restrictions:

```ron
// Example: Item usable by Knight (bit 1) and Cleric (bit 4)
disablements: Disablement(5)  // 5 = 1 + 4 (binary: 0101)
```

**Bitfield Values**:

- Bit 0 (value 1): Class 1 (e.g., Knight)
- Bit 1 (value 2): Class 2 (e.g., Mage)
- Bit 2 (value 4): Class 3 (e.g., Cleric)
- Bit 3 (value 8): Class 4 (e.g., Rogue)
- etc.

**Special Cases**:

- `Disablement(0)`: All classes can use
- `Disablement(255)`: No classes can use (quest items)

**Combining Restrictions**:

```ron
// Knight (1) + Cleric (4) = 5
disablements: Disablement(5)

// Mage (2) + Rogue (8) = 10
disablements: Disablement(10)

// All classes except Thief (16)
disablements: Disablement(239)  // 255 - 16
```

---

## Campaign Architecture

### Directory Structure

Standard campaign layout:

```
campaigns/my_campaign/
├── campaign.ron           # Campaign metadata
├── README.md             # Description and credits
├── LICENSE.txt           # License (optional)
├── data/
│   ├── classes.ron       # Character classes
│   ├── races.ron         # Playable races
│   ├── items.ron         # All items
│   ├── monsters.ron      # All monsters
│   ├── spells.ron        # All spells
│   ├── landscape.ron     # Reusable static landscape definitions
│   ├── landscape_mesh_registry.ron # Imported landscape mesh registry
│   └── maps/             # Map files
│       ├── town.ron
│       ├── dungeon_01.ron
│       └── ...
├── assets/ (optional)
│   ├── meshes/landscape/ # Imported tree/brush/rock mesh RON files
│   ├── textures/trees/   # Shared tree/foliage texture set
│   ├── textures/landscape/ # Importer-copied landscape textures
│   ├── music/
│   └── images/
└── docs/ (optional)
    ├── walkthrough.md
    └── design_notes.md
```

### Campaign Metadata

The `campaign.ron` file defines campaign identity:

```ron
(
    id: "unique_campaign_id",        // Lowercase, underscores only
    name: "Display Name",             // Human-readable
    version: "1.0.0",                 // Semantic versioning
    author: "Your Name",
    description: "Short description of campaign theme and scope.",
    starting_map: 1,                  // MapId where players start
    min_engine_version: "0.1.0",      // Minimum Antares version
)
```

### Content Files

Each content type has its own RON file:

**classes.ron**: HashMap of ClassId → ClassDefinition

**races.ron**: HashMap of RaceId → RaceDefinition

**items.ron**: HashMap of ItemId → Item

**monsters.ron**: HashMap of MonsterId → Monster

**spells.ron**: HashMap of SpellId → Spell

**landscape.ron**: List of reusable `LandscapeDefinition` entries for static environmental props

**landscape_mesh_registry.ron**: List of imported landscape mesh registry entries

**maps/\*.ron**: Individual map files (one per map)

---

## Content Design Patterns

### Authoring Landscape Props

Use Landscape for static environmental decoration: trees, shrubs, brush, rocks,
grass clumps, ground cover, and decorative ruin fragments. Use Furniture for
interactable or structure-like content such as doors, containers, chairs, tables,
torches, and inspectable objects.

The recommended workflow is:

1. Open the Campaign Builder and load your campaign.
2. Use the **Importer** tab and choose export target **Landscape** for an `.obj`
   or `.glb` model.
3. Export creates or updates the landscape mesh RON under
   `assets/meshes/landscape/` or `assets/meshes/landscape/<category>/`, copies
   textures under `assets/textures/landscape/<asset_slug>/`, upserts
   `data/landscape_mesh_registry.ron`, and upserts `data/landscape.ron` or the
   campaign's configured landscape file.
4. Use the **Landscape** tab to browse definitions grouped by category and check
   mesh/texture validation status.
5. Use the Map Editor **Place Landscape** tool to place decorations on tiles,
   then edit position, offset, y-offset, rotation, scale, tint, and blocking
   override in the placement inspector.
6. Save the map. Reopening the campaign reloads the same stable
   `landscape_placements` RON data.

Keep all mesh and texture references campaign-relative and under `assets/`.
Seed tree/brush landscape meshes may reuse `assets/textures/trees/`; newly
imported landscape textures normally live under
`assets/textures/landscape/<asset_slug>/`. Tutorial seed definitions and SDK
exporter-created definitions use the same validated `LandscapeDefinition`
format; curated data may customize icons, tags, descriptions, and blocking flags
after export as long as IDs, mesh references, and asset paths remain valid.

### Pattern 1: Progressive Equipment

Design items in tiers that match player progression:

```ron
// Tier 1: Starting equipment (levels 1-3)
{
    1: (
        id: 1,
        name: "Rusty Dagger",
        item_type: Weapon((damage: (1, 4), ...)),
        value: 5,
        // ...
    ),

    // Tier 2: Early game (levels 4-6)
    10: (
        id: 10,
        name: "Iron Dagger",
        item_type: Weapon((damage: (1, 6), ...)),
        value: 50,
        // ...
    ),

    // Tier 3: Mid game (levels 7-10)
    20: (
        id: 20,
        name: "Steel Dagger",
        item_type: Weapon((damage: (2, 6), ...)),
        value: 200,
        // ...
    ),
}
```

**Benefits**:

- Players feel progression
- Older items remain useful (sell value, backups)
- Easy to balance

### Pattern 2: Class-Specific Items

Create items tailored to each class:

```ron
// Knight: Heavy armor, two-handed weapons
{
    100: (
        name: "Plate Mail",
        item_type: Armor((ac_bonus: 8, armor_type: Heavy, ...)),
        disablements: Disablement(1),  // Knight only
        // ...
    ),
}

// Mage: Light armor, spell-enhancing items
{
    200: (
        name: "Robe of the Archmage",
        item_type: Armor((ac_bonus: 2, armor_type: Light, ...)),
        disablements: Disablement(2),  // Mage only
        bonuses: [
            (attribute: SpellPower, value: Constant(5)),
        ],
        // ...
    ),
}
```

### Pattern 3: Risk-Reward Items

Cursed or high-risk items with powerful effects:

```ron
{
    300: (
        name: "Berserker Axe",
        item_type: Weapon((damage: (3, 8), ...)),
        cursed: true,  // Cannot be unequipped easily
        bonuses: [
            (attribute: Might, value: Constant(5)),      // +5 Might
            (attribute: ArmorClass, value: Constant(-2)), // -2 AC (negative)
        ],
        // ...
    ),
}
```

**Use Cases**:

- High damage but lowers defense
- Powerful spell effects but drains HP
- Stat boosts but character conditions

### Pattern 4: Consumable Economy

Balance consumable items with appropriate costs:

```ron
{
    // Cheap, weak healing
    400: (
        name: "Minor Healing Potion",
        item_type: Consumable((effect: HealHp((1, 8)), uses: 1)),
        value: 10,
        // ...
    ),

    // Expensive, strong healing
    401: (
        name: "Greater Healing Potion",
        item_type: Consumable((effect: HealHp((4, 8)), uses: 1)),
        value: 100,
        // ...
    ),

    // Multi-use items
    402: (
        name: "Healing Salve",
        item_type: Consumable((effect: HealHp((1, 6)), uses: 5)),
        value: 40,
        // ...
    ),
}
```

### Pattern 5: Encounter Design

Structure map encounters for pacing:

```ron
events: [
    // Easy encounter (map entrance)
    (
        position: (5, 2),
        event_type: Combat([
            (monster_id: 1, count: 2),  // 2 weak monsters
        ]),
    ),

    // Medium encounter (mid-dungeon)
    (
        position: (10, 8),
        event_type: Combat([
            (monster_id: 1, count: 3),
            (monster_id: 2, count: 1),  // Mixed difficulty
        ]),
    ),

    // Boss encounter (end)
    (
        position: (15, 15),
        event_type: Combat([
            (monster_id: 10, count: 1),  // Single powerful boss
        ]),
    ),
]
```

**Pacing Principles**:

- Start easy, ramp up gradually
- Mix combat with treasure and dialogue
- Save hardest encounters for end
- Provide rest/healing opportunities

### Pattern 6: Environmental Storytelling

Use map features to tell stories without dialogue:

```ron
events: [
    // Corpse with loot tells a story
    (
        position: (7, 3),
        event_type: Text("A fallen adventurer lies here, clutching a blood-stained map."),
    ),
    (
        position: (7, 3),
        event_type: Treasure([
            (item_id: 999, quantity: 1),  // "Blood-Stained Map" (quest item)
        ]),
    ),

    // Trap warning from environment
    (
        position: (8, 5),
        event_type: Text("Scorch marks cover the walls. Something dangerous happened here."),
    ),
    (
        position: (9, 5),
        event_type: Damage((2, 6)),  // Fireball trap
    ),
]
```

## Interactive Objects — Meshes and Events

Every interactive tile event in Antares can carry two optional fields that unlock rich
authoring possibilities:

| Field         | Type             | Purpose                                                                                               |
| ------------- | ---------------- | ----------------------------------------------------------------------------------------------------- |
| `mesh_id`     | `Option<String>` | Renders a 3-D object on the event tile (door, chest, signpost, …)                                     |
| `dialogue_id` | `Option<u16>`    | Plays a dialogue tree when the party presses **[E]** on the tile, before the primary game effect runs |

Events that support both fields: `Treasure`, `Sign`, `Container`, `LockedDoor`, `LockedContainer`.

---

### The Unified Model

Antares separates _what happens_ (the event type) from _how it looks_ (`mesh_id`) and
_what the character says_ (`dialogue_id`). This means:

- The same event logic works with **any** registered mesh.
- A `Treasure` chest looks like a wooden crate, an ornate coffer, or a barred passage
  depending on the `mesh_id` you supply — the loot logic is unchanged.
- Supplying a `dialogue_id` inserts a conversation step _before_ the event fires. The
  dialogue can then fire the primary effect via a `TriggerEvent` action at any node, giving
  authors complete control over pacing.

---

### Event Type Reference

| Event                  | `mesh_id` | `dialogue_id` | Key fields                           | TriggerEvent name                                         |
| ---------------------- | --------- | ------------- | ------------------------------------ | --------------------------------------------------------- |
| `Treasure`             | ✓         | ✓             | `loot: Vec<u8>`                      | `collect_treasure`                                        |
| `Sign`                 | ✓         | ✓             | `text: String`                       | _(none needed)_                                           |
| `Container`            | ✓         | ✓             | `id`, `items`, `gold`, `gems`        | `open_container`                                          |
| `LockedDoor`           | ✓         | ✓             | `lock_id`, `key_item_id`             | `unlock_door`                                             |
| `LockedContainer`      | ✓         | ✓             | `lock_id`, `key_item_id`, `items`    | `unlock_container`                                        |
| `Encounter`            | —         | —             | `monster_group`, `combat_event_type` | _(combat auto-fires)_                                     |
| `NpcDialogue`          | —         | via NPC       | `npc_id`                             | _(NPC dialogue auto-fires)_                               |
| `RecruitableCharacter` | —         | ✓             | `character_id`                       | `recruit_character_to_party` / `recruit_character_to_inn` |
| `EnterInn`             | —         | —             | `innkeeper_id`                       | `open_inn_party_management`                               |
| `Teleport`             | —         | —             | `destination`, `map_id`              | _(auto-fires)_                                            |
| `Trap`                 | —         | —             | `damage`, `effect`                   | _(auto-fires)_                                            |

---

### TriggerEvent Reference

Use a `TriggerEvent` action inside any dialogue node or choice to fire game effects
at the exact moment chosen by the author:

| Event name                   | What it does                                                                          | Requires event_context? |
| ---------------------------- | ------------------------------------------------------------------------------------- | ----------------------- |
| `collect_treasure`           | Distributes loot from the triggering `Treasure` event to the party; despawns the mesh | Yes                     |
| `open_container`             | Opens the `Container` event tile in container-inventory mode                          | Yes                     |
| `unlock_door`                | Consumes the key item (if any) and removes the `LockedDoor` event; despawns the mesh  | Yes                     |
| `unlock_container`           | Consumes the key item (if any) and opens the `LockedContainer` for looting            | Yes                     |
| `open_inn_party_management`  | Opens the inn party management screen using the speaker NPC as the innkeeper          | No                      |
| `recruit_character_to_party` | Adds the recruitable character to the active party                                    | No                      |
| `recruit_character_to_inn`   | Sends the recruitable character to the nearest inn roster                             | No                      |

> **Note**: `event_context` is set automatically when a dialogue is triggered by a map event
> (Treasure, Container, LockedDoor, LockedContainer). You do not need to set it manually.

---

### Placement Workflow

Follow these steps to add a visible, interactive object to your campaign:

#### 1. Register the mesh in `object_mesh_registry.ron`

Create or update `data/object_mesh_registry.ron` in your campaign directory:

```ron
ObjectMeshRegistry(
    meshes: {
        "iron_gate": "assets/meshes/objects/iron_gate.ron",
        "treasure_chest_oak": "assets/meshes/objects/treasure_chest_oak.ron",
    }
)
```

Each value is a path **relative to the campaign root** pointing at a `CreatureDefinition`
RON mesh asset. You can import the mesh using the Campaign Builder's **Importer** tab.

#### Editing registry entries with the Objects tab

Hand-editing `object_mesh_registry.ron` works fine, but the Campaign Builder also has a
dedicated **Objects** tab that manages this same file for you. An "Object," in this tab, is
exactly one entry in the registry above: a key (like `"iron_gate"`) paired with the path to
its mesh asset. That key is the same string you type into an event's `mesh_id` field — so
once an Object exists in the registry, it immediately shows up as an option wherever
`mesh_id` is set, and any Object you create or rename here is reflected the next time you
fill in that field in the Map Editor.

**Creating an Object.** The actual mesh import still happens in the **Importer** tab: load
your OBJ or GLB file as usual, and set its export type to **Object Mesh**. This writes the
mesh asset to disk and adds (or updates) the corresponding entry in
`object_mesh_registry.ron` automatically — you don't need to hand-edit the RON file
afterward. As soon as the import finishes, the Campaign Builder switches you over to the
Objects tab and refreshes its list, so your new entry is visible right away. If you're
already on the Objects tab and want to start this process, click **📥 Import Object Mesh**;
it simply jumps you to the Importer with the export type pre-set to Object Mesh — the import
itself still happens there.

**Editing an Object.** Select an entry in the Objects tab's list and click Edit (or use the
row's context menu) to open its fields:

- **Key** — the registry key itself, e.g. `"iron_gate"`. This is a free-text field, since
  you're assigning an identifier rather than referencing one elsewhere — it's the one field
  in this form without autocomplete validation. Renaming a key checks for collisions: if the
  new name is already in use, the edit is rejected with an inline error and the original key
  is kept.
- **Name** — the object's display name.
- **Scale** — a single uniform scale factor applied to the whole mesh.
- **Color tint** — an optional RGBA tint you can toggle on or off; when first enabled it
  defaults to opaque white.
- **Per-mesh material** — for each mesh that makes up the object, you can adjust its base
  color, metallic, roughness, and an optional emissive color. A **↻ Re-import in Importer**
  button sits next to this section as a shortcut back into the Importer flow described above.

Saving these changes writes the updated mesh asset and rewrites
`object_mesh_registry.ron` immediately — there's no need to perform a separate "Save
Campaign" step afterward.

The Objects tab's edit form is intentionally limited to the properties above. It cannot
change mesh geometry — vertices, indices, UVs, or normals — and it cannot add or remove
meshes from an object. Any of that requires going back through the Importer: re-import a new
OBJ/GLB, export it again as Object Mesh using the **same key**, and the existing registry
entry (and its asset) will be overwritten. The **↻ Re-import in Importer** button is just a
shortcut into that same flow.

#### 2. Place the event in the map RON

Open `data/maps/your_map.ron` and add the event under `events:`:

```ron
events: {
    (x: 17, y: 12): LockedDoor(
        name: "Iron Gate",
        lock_id: "iron_gate_17_12",
        key_item_id: Some(42),
        mesh_id: Some("iron_gate"),
        dialogue_id: Some(601),
    ),
}
```

Alternatively, use the **Map Editor** in the Campaign Builder: select the tile, choose
event type "LockedDoor", fill in the Mesh ID and Dialogue ID autocomplete fields, then
click **Save Changes**.

#### 3. Author the dialogue in `dialogues.ron`

```ron
[
    (
        id: 601,
        name: "Iron Gate",
        root_node: 0,
        repeatable: false,
        nodes: {
            0: (
                id: 0,
                text: "A heavy iron gate bars your way.",
                choices: [1, 2],
            ),
            1: (
                id: 1,
                text: "Try to open it.",
                actions: [
                    TriggerEvent(event_name: "unlock_door"),
                ],
                next_node: None,
            ),
            2: (
                id: 2,
                text: "Leave it for now.",
                actions: [],
                next_node: None,
            ),
        },
    ),
]
```

When the player chooses "Try to open it.", the `unlock_door` trigger fires: the engine
checks the party inventory for the matching key item (`key_item_id: Some(42)`), consumes
it, removes the gate event, and despawns the iron gate mesh.

#### 4. Add the key item in `items.ron`

```ron
{
    42: (
        id: 42,
        name: "Iron Gate Key",
        item_type: Key(()),
        value: 0,
        weight: 1,
        description: "A heavy iron key that fits the gate lock.",
        disablements: Disablement(0),
    ),
}
```

---

### Worked Example: Creating a Locked Gate

This end-to-end example wires all four steps together.

**Scenario**: The party must obtain an iron key from a dungeon guard and use it to
open a gate blocking the exit corridor.

**Step 1 — Mesh registration** (`data/object_mesh_registry.ron`):

```ron
ObjectMeshRegistry(
    meshes: {
        "iron_gate": "assets/meshes/objects/iron_gate.ron",
    }
)
```

**Step 2 — Map event** (`data/maps/dungeon_01.ron`):

```ron
events: {
    // Guard drops a key when defeated (Treasure event)
    (x: 8, y: 4): Treasure(
        name: "Guard's Keyring",
        description: "A ring of keys dropped by the guard.",
        loot: [42],
    ),
    // Gate at the end of the corridor
    (x: 15, y: 4): LockedDoor(
        name: "Iron Gate",
        lock_id: "exit_gate",
        key_item_id: Some(42),
        mesh_id: Some("iron_gate"),
        dialogue_id: Some(601),
    ),
}
```

**Step 3 — Dialogue** (`data/dialogues.ron`):

```ron
[
    (
        id: 601,
        name: "Iron Gate",
        root_node: 0,
        repeatable: false,
        nodes: {
            0: (
                id: 0,
                text: "The iron gate is locked tight.",
                choices: [1, 2],
            ),
            1: (
                id: 1,
                text: "Use the iron key.",
                actions: [TriggerEvent(event_name: "unlock_door")],
                next_node: None,
            ),
            2: (
                id: 2,
                text: "Step back.",
                actions: [],
                next_node: None,
            ),
        },
    ),
]
```

**Step 4 — Key item** (`data/items.ron`):

```ron
{
    42: (
        id: 42,
        name: "Iron Key",
        item_type: Key(()),
        value: 5,
        weight: 1,
        description: "A heavy iron key.",
        disablements: Disablement(0),
    ),
}
```

**Result**: When the party steps on tile (15, 4), the dialogue fires. If the party
has item 42, choosing "Use the iron key" calls `unlock_door`, which consumes the key,
removes the `LockedDoor` event from the map, and despawns the iron gate mesh. The tile
becomes passable and the party can continue.

---

### Treasure with Custom Dialogue

Adding `dialogue_id` to a `Treasure` event allows the author to show flavour text or
a choice before distributing loot:

```ron
(x: 10, y: 6): Treasure(
    name: "Ancient Chest",
    description: "An ornately carved wooden chest.",
    loot: [15, 16, 17],
    mesh_id: Some("treasure_chest_oak"),
    dialogue_id: Some(602),
),
```

Dialogue 602:

```ron
(
    id: 602,
    name: "Ancient Chest",
    root_node: 0,
    repeatable: false,
    nodes: {
        0: (
            id: 0,
            text: "You find an ornate wooden chest. Strange runes are carved into the lid.",
            choices: [1, 2],
        ),
        1: (
            id: 1,
            text: "Open the chest.",
            actions: [TriggerEvent(event_name: "collect_treasure")],
            next_node: None,
        ),
        2: (
            id: 2,
            text: "Leave it — something feels wrong.",
            actions: [],
            next_node: None,
        ),
    },
),
```

---

## Creating Innkeepers

When creating an innkeeper NPC, follow these steps to ensure the NPC integrates correctly with the inn/party-management systems:

1. Define the NPC in `data/npcs.ron` with `is_innkeeper: true`. For example:

```ron
(
    id: "your_innkeeper_id",
    name: "Your Innkeeper Name",
    portrait_id: "portrait_asset",
    dialogue_id: Some(999),  // reference to a dialogue tree
    is_innkeeper: true,
    is_merchant: false,
)
```

2. Create a dialogue in `data/dialogues.ron` that offers the player a party-management option. You can either:

   - Add a choice that executes `OpenInnManagement { innkeeper_id: "your_innkeeper_id" }`, or
   - Add a terminal node that triggers `TriggerEvent(event_name: "open_inn_party_management")`. When using `TriggerEvent`, ensure the dialogue is started with the innkeeper as the speaker so the runtime can resolve the correct innkeeper ID.

3. Example (simple template):

```ron
(
    id: 999,
    name: "Default Innkeeper Greeting",
    root_node: 1,
    nodes: {
        1: (
            id: 1,
            text: "Welcome to my establishment! What can I do for you?",
            speaker_override: None,
            choices: [
                (
                    text: "I need to manage my party.",
                    target_node: Some(2),
                    conditions: [],
                    actions: [],
                    ends_dialogue: false,
                ),
                (
                    text: "Nothing right now. Farewell.",
                    target_node: None,
                    conditions: [],
                    actions: [],
                    ends_dialogue: true,
                ),
            ],
            conditions: [],
            actions: [],
            is_terminal: false,
        ),
        2: (
            id: 2,
            text: "Certainly! Let me help you organize your party.",
            speaker_override: None,
            choices: [],
            conditions: [],
            actions: [
                TriggerEvent(
                    event_name: "open_inn_party_management",
                ),
            ],
            is_terminal: true,
        ),
    },
    speaker_name: Some("Innkeeper"),
    repeatable: true,
    associated_quest: None,
),
```

4. Finally, reference the dialogue from your NPC definition:

```ron
dialogue_id: Some(999)
```

Validation notes:

- The SDK validator checks that every `is_innkeeper: true` NPC has a `dialogue_id` configured. Use `cargo run --bin antares-sdk -- campaign validate <campaign_path>` to validate your campaign before packaging.
- The validator will flag missing dialogue IDs as errors.

---

## Advanced Techniques

### Technique 1: Dynamic Stat Bonuses

Items with conditional or temporary bonuses:

```ron
bonuses: [
    // Permanent bonus
    (attribute: Might, value: Constant(3)),

    // Temporary bonus (duration in turns)
    (attribute: Speed, value: Temporary(5, 10)),  // +5 Speed for 10 turns

    // Conditional bonus (implementation-specific)
    (attribute: AttackBonus, value: Constant(2)),  // vs specific enemy type
]
```

### Technique 2: Spell-Effect Items

Items that cast spells when used:

```ron
{
    500: (
        name: "Wand of Fireballs",
        item_type: Accessory((
            slot: Held,
            charges: Some(10),
            spell_id: Some(20),  // Fireball spell
        )),
        // ...
    ),
}
```

### Technique 3: Quest-Gated Content

Use quest states to control access:

```ron
events: [
    // Door only opens if quest completed
    (
        position: (10, 10),
        event_type: ConditionalEvent((
            condition: QuestCompleted(5),
            success: Teleport((destination_map: 2, destination_position: (5, 5))),
            failure: Text("The door is sealed by ancient magic."),
        )),
    ),
]
```

### Technique 4: Multi-Phase Bosses

Bosses that change behavior at HP thresholds:

```ron
{
    1000: (
        name: "Dragon Lord",
        hp: (20, 8),  // High HP pool
        special_attacks: [
            "FireBreath",    // Used at > 50% HP
            "TailSwipe",     // Used at 25-50% HP
            "DesperateFury", // Used at < 25% HP
        ],
        // ...
    ),
}
```

### Technique 5: Interconnected Maps

Create a coherent world with bidirectional exits:

```ron
// Map 1: Town
exits: [
    (
        position: (20, 10),
        destination_map: 2,      // To forest
        destination_position: (1, 10),
        direction: East,
    ),
]

// Map 2: Forest
exits: [
    (
        position: (1, 10),
        destination_map: 1,      // Back to town
        destination_position: (20, 10),
        direction: West,
    ),
    (
        position: (40, 20),
        destination_map: 3,      // To dungeon
        destination_position: (1, 1),
        direction: North,
    ),
]
```

**Map Connectivity Rules**:

- Every map must be reachable from starting map
- Provide return paths (players shouldn't get stuck)
- Use directional exits for immersion

---

## Balancing Guidelines

### Character Balance

**Class Balance**:

- Pure casters: Low HP (d6), high spell damage
- Hybrids: Medium HP (d8), some spells
- Pure fighters: High HP (d10-d12), high physical damage

**Race Balance**:

- Total stat modifiers should sum to +2 to +4
- Negative modifiers balance positive ones
- Special abilities count as +1 to +2 stat points

### Item Balance

**Weapon Damage by Tier**:

- Tier 1 (levels 1-3): 1d4 to 1d6
- Tier 2 (levels 4-6): 1d8 to 2d4
- Tier 3 (levels 7-10): 2d6 to 2d8
- Tier 4 (levels 11+): 2d10 to 4d6

**Armor AC by Tier**:

- Light armor: +1 to +3
- Medium armor: +4 to +6
- Heavy armor: +7 to +10

**Item Value Formula**:

```
Value = Base + (Damage × 10) + (AC × 15) + (Bonus × 20)
```

**Example**:

```
Longsword: 1d8 damage
Base: 50 gold
Damage: 8 × 10 = 80 gold
Total: 130 gold
```

### Monster Balance

**XP Award Formula**:

```
XP = (Level × 50) + (AC × 10) + (HP Average × 5) + (Special Attacks × 50)
```

**Example**:

```
Goblin Shaman (Level 3, AC 12, HP 3d6 avg 10, 1 special attack)
XP = (3 × 50) + (12 × 10) + (10 × 5) + (1 × 50)
XP = 150 + 120 + 50 + 50 = 370
```

**Loot Drop Rate**:

- Common enemies: 25-50% chance
- Elite enemies: 50-75% chance
- Bosses: 100% chance + bonus loot

### Spell Balance

**SP Cost Formula**:

```
SP Cost = (Spell Level × 5) + Target Multiplier

Target Multipliers:
- Single: 0
- AllEnemies: +5
- AllAllies: +3
- Area: +3
```

**Damage Scaling**:

- Level 1: 1d6 to 1d8
- Level 2: 2d6 to 2d8
- Level 3: 3d6 to 3d8
- Level 4+: 4d6 to 4d10

### Encounter Balance

**Combat Difficulty**:

```
Party Power = (Average Party Level) × (Party Size) × 100

Easy Encounter: 50% of Party Power
Medium Encounter: 100% of Party Power
Hard Encounter: 150% of Party Power
Boss Encounter: 200% of Party Power
```

**Example**:

```
Party: 4 characters, average level 5
Party Power = 5 × 4 × 100 = 2000

Medium Encounter: 2000 XP worth of monsters
Could be: 4 × Goblins (500 XP each)
Or: 2 × Ogres (1000 XP each)
```

---

## Performance Considerations

### Map Size

**Recommendations**:

- Small maps: 10×10 to 20×20
- Medium maps: 30×30 to 50×50
- Large maps: 60×60 to 100×100
- Maximum: 200×200 (use sparingly)

**Why**: Larger maps increase memory usage and save file size.

### Event Density

**Guidelines**:

- 1 event per 10-20 tiles (sparse)
- 1 event per 5-10 tiles (moderate)
- 1 event per 2-5 tiles (dense)

**Why**: Too many events slow down map loading and pathfinding.

### Content Database Size

**Recommendations**:

- Items: 100-500 per campaign
- Monsters: 50-200 per campaign
- Spells: 30-100 per campaign
- Maps: 10-50 per campaign

**Why**: Validation time increases with content size.

### RON File Optimization

**Tips**:

- Use consistent indentation (2 or 4 spaces)
- Remove unnecessary whitespace in production
- Split large data files by category
- Compress map tile arrays when possible

---

## Publishing Your Mod

### Pre-Release Checklist

- [ ] Run campaign validator (zero errors)
- [ ] Playtest entire campaign start-to-finish
- [ ] Balance check all encounters
- [ ] Proofread all dialogue and descriptions
- [ ] Document known issues
- [ ] Write README with installation instructions
- [ ] Include credits and license

### Packaging

Use the campaign packager:

```bash
tar -czf my_campaign_v1.0.tar.gz campaigns/my_campaign
```

### README Template

```markdown
# My Campaign Name

**Version**: 1.0.0
**Author**: Your Name
**Difficulty**: Medium
**Estimated Playtime**: 5-10 hours

## Description

Brief description of your campaign's theme, story, and unique features.

## Features

- 5 new character classes
- 10 dungeons
- 50+ new items
- Original storyline

## Installation

1. Extract archive to `campaigns/` directory
2. Launch Antares
3. Select "My Campaign" from campaign list

## Credits

- Design: Your Name
- Testing: Tester Names
- Music: Artist Name (if applicable)

## License

MIT License (or your choice)
```

### Versioning

Use semantic versioning:

- **Major (1.0.0)**: Breaking changes, complete rewrites
- **Minor (0.1.0)**: New features, new content
- **Patch (0.0.1)**: Bug fixes, balance tweaks

### Distribution

Share your campaign:

1. **Official Mod Repository**: Submit to Antares mod database (if available)
2. **GitHub/GitLab**: Host source and releases
3. **Community Forums**: Post download links and discussion
4. **Modding Discord**: Share with community

---

## Best Practices Summary

### Do

✓ Validate your campaign frequently during development
✓ Playtest with fresh characters (not over-leveled)
✓ Document your ID ranges and naming conventions
✓ Use meaningful names for maps, items, and NPCs
✓ Provide multiple solutions to challenges
✓ Balance risk vs. reward
✓ Include README and credits

### Don't

✗ Use duplicate IDs
✗ Create disconnected maps (unreachable)
✗ Reference non-existent content IDs
✗ Make encounters impossibly hard
✗ Hardcode player names or assumptions
✗ Use copyrighted content without permission
✗ Skip validation before release

---

## Resources

- **SDK API Reference**: `docs/reference/sdk_api.md`
- **Campaign Tutorial**: `docs/tutorials/creating_campaigns.md`
- **Tool Guides**: `docs/how-to/`
- **Architecture**: `docs/reference/architecture.md`
- **Community**: [Antares Discord/Forum]

---

## Getting Help

If you encounter issues:

1. Check validation errors first
2. Review this guide and SDK API reference
3. Search community forums for similar issues
4. Ask in modding Discord channel
5. Open GitHub issue with campaign validator output

---

## Conclusion

Antares provides a powerful, flexible modding system. With these patterns and guidelines, you can create professional-quality campaigns.

**Happy modding!**
