antares/docs/explanation/thing_based_event_model.md#L1-400
# Thing-based Event Model — Design & Migration Plan

## Purpose
This document outlines a proposed redesign to model map objects as "Things" which can carry zero or more "Events". The primary goals are:
- Allow multiple events to be attached to a single placeable object (Thing).
- Make NPCs, Traps, Containers, Portals, Signs, etc. first-class "Things".
- Make Events a composable subset of a Thing (so a Thing may own zero or more Events).
- Expose per-Thing and per-Event icons for clear map rendering (resolve the "all red events" problem).
- Provide a migration strategy from the existing `Map.events` and `Map.npcs` to the new structure while maintaining backward compatibility.

## Summary of the issue
Currently:
- `Map` contains `events: HashMap<Position, MapEvent>` — only one event per tile is supported.
- Map editor visually colors event tiles red; all events share the same color, making them indistinguishable.
- Users want to place multiple distinct events on the same tile and assign icons to represent them visually.
- NPCs are conceptually a Thing and their dialog is an Event; modeling them separately leads to conceptual duplication.

Proposed solution:
- Introduce a `Thing` abstraction to represent placeable objects on a map.
- `Thing` can be `Npc`, `Trap`, `Chest`, `Portal`, `Sign`, `Decoration`, etc.
- Each `Thing` contains a list of `Event`s that trigger behavior (Dialogue, Condition apply, Damage, Teleport, Treasure, Encounter, etc.).
- Add `things: Vec<Thing>` to `Map`, and migrate `map.events` and `map.npcs` into `things` progressively.

## Design overview — Domain data model

High-level type summary (conceptual):

- `ThingId` — unique identifier for placed Things.
- `EventId` — unique identifier for events attached to Things.
- `Thing` — placeable object; has `position`, `name`, `description`, `kind` (ThingKind), `icon_path`, and `events: Vec<Event>`.
- `ThingKind` — enumerates known kinds: `Npc`, `Trap`, `Container`, `Portal`, `Sign`, `Decoration`, etc.
- `Event` — an `id`, `event_type` (enum), optional `icon_path`, `trigger` (when to fire), and event-specific payload (like teleport destination, damage, condition, loot).

A sample (illustrative Rust) definition is provided below:

```dev/null/example.rs#L1-80
// Hypothetical structs for concept clarity. The code below is an illustrative sketch.

pub type ThingId = u16;
pub type EventId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thing {
    pub id: ThingId,
    pub position: Position,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub kind: ThingKind,
    #[serde(default)]
    pub icon_path: Option<String>,  // icon used for rendering this Thing
    #[serde(default)]
    pub events: Vec<Event>,        // zero or more events attached
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThingKind {
    Npc { npc_id: u16 },
    Trap {},
    Chest {},
    Portal {},
    Sign {},
    Decoration {},
    // additional kinds...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub event_type: EventType,
    #[serde(default)]
    pub icon_path: Option<String>,  // optional override icon
    pub trigger: EventTrigger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    Dialogue { npc_id: u16 },
    Teleport { map_id: MapId, destination: Position },
    Treasure { loot: Vec<ItemId> },
    ConditionApply { condition_id: String, duration: Option<u32> },
    Damage { amount: u16 },
    Encounter { monster_group: Vec<u8> },
    // ...
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EventTrigger {
    OnStep,
    OnInteract,
    OnUse,
    OnTimer,
    // ...
}
```

Notes:
- The `Thing.kind` field encodes the object's intrinsic purpose. A `Npc` Thing can contain a `Dialogue` event.
- `Event.trigger` determines how and when an event fires; this allows the same `Event` type to be reused for different interaction types.

## Map structure
Replace (or add, during migration) the map's event model from:

- `events: HashMap<Position, MapEvent>`

to:

- `things: Vec<Thing>` (or `HashMap<ThingId, Thing>` + `HashMap<Position, Vec<ThingId>>` for quick lookup).
  
A `Map` now looks conceptually like:

```dev/null/example_map.ron#L1-48
Map(
    id: 1,
    name: "Example Map",
    width: 10,
    height: 10,
    // ... tiles ...
    things: [
        // NPC thing with a dialogue event:
        (
            id: 100,
            position: (x: 5, y: 5),
            name: "Merchant",
            kind: Npc { npc_id: 3 },
            icon_path: "data/assets/icons/merchant.png",
            events: [
                ( id: 1, event_type: Dialogue { npc_id: 3 }, trigger: OnInteract )
            ]
        ),
        // Trap with condition:
        (
            id: 200,
            position: (x: 8, y: 8),
            name: "Poison Trap",
            kind: Trap {},
            icon_path: "data/assets/icons/trap.png",
            events: [
                ( id: 2, event_type: ConditionApply { condition_id: "poison", duration: 30 }, trigger: OnStep )
            ]
        ),
    ],
)
```

## Runtime behavior (triggering events)
- Old function `trigger_event(world, pos)` concept will be generalized:
  - On `OnStep` or `OnInteract`, `world` collects `things_at_position`.
  - For each `Thing`, iterate `Thing.events` in order and process the events that match the trigger type.
  - `Event` semantics:
    - `Treasure` or one-time events: may be removed/consumed after trigger.
    - `Dialogue` or persistent signs: remain as repeatable events.
    - `Trap` events: usually `OnStep` and consumed/disabled after trigger if so configured.
  - `trigger_event` returns a list of `EventResult` or `EventResultSequence` because multiple events can fire for a single trigger.

Migration from the existing single-event APIs:
- Update `World` event processing to search `things` at the tile rather than `MapEvent` from `events`.

## UI & Editor changes
- `map_editor.rs` modifications:
  - Replace `tile_color` "has_event" check with an overlay marker for `Thing` icons.
  - For each tile, fetch `things_here` and render `Thing` icons. For `Thing.events`, render little event badges (or overlay stacked small icons).
  - When `things_here.len() > N`, display " +N " overflow badges indicating additional events.
  - Inspector UI:
    - Show the Thing name, description, icon, kind, and a list of events.
    - Add event add/remove/reorder UI and event-type-specific editors (e.g., teleport destination, trap damage, dialogue id).
  - Placement tools:
    - Tools to place Things: choose `ThingKind` (Npc / Trap / Chest / Portal / Sign / Decoration).
    - Tools to attach or edit events for a Thing.

Rendering UX specifics:
- Primary rendering: Thing icon (e.g., `merchant.png`) at tile center.
- Event badges: small icons (16x16) shown in corners or stacked; tooltip shows a detailed list when hovered.
- Click behavior:
  - Single click selects the topmost Thing; `shift` or `tooltip` shows stacked events or a modal button to list all Things for that tile.
  - Right-click opens context: list of Things, select item to edit or manage events.

## Asset & Icon management
- Update `asset_manager` to:
  - Add `AssetType::Icon`.
  - Track icons used by Things and Events.
  - Provide icon chooser UI in the editor for both `Thing.icon_path` and `Event.icon_path`.
- Icon fallback:
  - If `Event.icon_path` is absent, use `Thing.icon_path`.
  - If neither exist, use default type icon (e.g., trap default, chest default).

## Data migration strategy

Migration needs to be incremental and backwards-compatible. Do not drop existing `events` and `npcs` at once. Recommended phased approach:

### Phase 0: Add `things` while keeping `events` and `npcs`
- Add `things: Vec<Thing>` to `Map` with `#[serde(default)]`.
- Keep `events: HashMap<Position, MapEvent>` and `npcs: Vec<Npc>` present.
- Implement runtime/load migration:
  - On load, convert `map.events` entries into `Thing` entries with `Thing.kind` set to a generic `EventOnly` or specific kind inferred from `EventType`.
  - Similarly convert `map.npcs` into `Thing` entries (`ThingKind::Npc`).
- Support both representations for a short period to give older tools/time to migrate.

Pseudocode example (conceptual):

```dev/null/example_migration.rs#L1-80
fn migrate_events_and_npcs_to_things(map: &mut Map) {
    // Convert existing events (HashMap<Position, MapEvent>) to Thing-based ones.
    for (pos, map_event) in map.events.drain() {
        let thing = Thing {
            id: generate_thing_id(),
            position: pos,
            name: map_event.name.clone(),
            kind: infer_kind_from_map_event(&map_event), // Trap/Sign/etc.
            icon_path: None,
            events: vec![Event::from_map_event(map_event)],
        };
        map.things.push(thing);
    }

    // Convert existing npcs (Vec<Npc>) to Things
    for npc in map.npcs.drain(..) {
        let thing = Thing {
            id: generate_thing_id(),
            position: npc.position,
            name: npc.name.clone(),
            kind: ThingKind::Npc { npc_id: npc.id },
            icon_path: None,
            events: vec![],
        };
        map.things.push(thing);
    }
}
```

### Phase 1: Update all readers/writers to prefer `things`
- Editor & CLI & engine should read `things` first and fall back to legacy `npcs/events` fields only if `things` is empty or not present.

### Phase 2: Migrate saved assets to new scheme
- Provide a migration tool or automatic migration on save (dumping as `things`) to convert old map formats to the new `things` structure, with `#[serde]` compatibility for the intermediate period.

### Phase 3: Remove deprecated fields
- Once `things` adoption is complete and the codebase uses `things` consistently, remove `events` and `npcs` from `Map`, and update all code accordingly.

## Validator & semantic checks
- Update `validate_map` to:
  - Not error on overlapping things/events per tile unless explicitly invalid (e.g., two full-size NPC spawns).
  - Validate semantics: e.g., `ThingKind::Portal` should have `EventType::Teleport` attached or specified gateway coords; `Trap` should have `OnStep` events.
  - Constrain incompatible combinations if necessary (like NPCs stacking with chest in certain contexts if not allowed by design).
- Update `validate_gameplay` to account for multiple events per tile:
  - If your game design prohibits more than one event that triggers automatically in the same tile (like two traps both `OnStep`), add validation rules accordingly.

## Tests & QA
Add test coverage for:
- Blueprint loader conversions: ensure `MapBlueprint` `MapEventBlueprint` and `NpcBlueprint` convert to `Thing` properly.
- Serialization & backward compatibility: test reading maps with legacy `events`/`npcs` and confirm they produce `things`.
- Editor behavior: UI tests for placing Things, adding Events, and rendering multiple events on one tile in the map grid.
- Runtime event processing tests:
  - Trigger multiple events at a tile and ensure they process in defined order and produce expected `EventResult` sequence.
- Validator tests cover new semantics where overlapping `Thing`/Event combos are permitted or flagged.
- Performance tests to ensure large numbers of `things` are handled efficiently (e.g., `HashMap<Position, Vec<ThingId>>` for fast lookups if necessary).

## Implementation plan & timeline (rough estimate)
- Phase A (Immediate UX improvement — icons & multiple icon overlay, no domain change)
  - Add event-type icon mapping and stacked icon overlay in `map_editor.rs`.
  - Time: 1–2 hours.

- Phase B (Schema changes & migration)
  - Add `Thing` type, `Map.things`, and converter code.
  - Make `blueprint.rs` accumulate events per position into `things`.
  - Update `asset_manager` to track icons for `things` and `events`.
  - Time: 1–3 days (safe estimate depending on tests & refactors).

- Phase C (SDK & Runtime updates)
  - Update map editor UI to fully manage `things`, event editing UI, and icon picker.
  - Update runtime `trigger_event` and event handlers to iterate events on a Thing and process them.
  - Add tests for all new behaviors and update the docs.
  - Time: 2–4 days.

- Phase D (Polish & cleanup)
  - Remove `map.events`/`map.npcs`, finalize migration, expand validator rules.
  - Clean up deprecated code and tests.
  - Time: 1–2 days.

Total (from initial change to complete refactor): often 1–2 weeks of development depending on resources and scope.

## Migration tooling & suggestions
- Provide a `map_migrate` tool that:
  - Reads a legacy map with `events` or `npcs`.
  - Converts events into `things`.
  - Creates output with `things` and optional icon mapping for each kind or event.
- Editor import dialog:
  - Offer “Migrate old format to Thing-based format” when the user opens older maps.

## Risks and mitigation
- API incompatibility: Changing core `Map` types impacts many modules. Mitigate by staged migration and by keeping both old and new models temporarily.
- Behavior semantics ambiguity: Need a clearly documented processing order and event trigger priorities for user expectation and deterministic processing.
- Test gaps: Provide a phased suite of tests validating equivalence of behavior for legacy maps vs new Thing-based maps.
- Editor UX complexity: Rendering and editing multiple stacked icons and events per Thing needs careful UI design to avoid confusion. UX iteration recommended.

## Discussion & next steps
- I recommend:
  1. Implement the visual fix (unique icons + overlay) to solve the immediate readability problem.
  2. Start Phase 0 by implementing the `things` structure and migration code (reader side), update blueprint conversion logic.
  3. Update editor and runtime to use `things` and process events.
  4. Flesh out final event processing semantics and document them.
  5. Remove `events`/`npcs` afterwards once migration is covered.

If you approve, I can create:
- A PR for the quick visual fix (icons + stacked overlay).
- A follow-up PR that proposes the domain-level change with a migration plan and initial code plus tests for `Thing` and `Event`.

Please review the design and the proposed phases and let me know any preference on:
- Whether the `Thing` should embed full `Npc` data or reference `npc_id` from the `Map`.
- Event ordering/priority semantics for multiple events (e.g., Trap first, Condition second).
- Default behavior for `thing.icon_path` vs `event.icon_path` (which overrides which).
- Any additional constraints or rules for valid `Thing` combinations per tile.
