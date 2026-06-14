# Unified Interactive Objects and Events Implementation Plan

## Overview

Five separate systems currently handle interactive map objects — landscape placements, furniture
events, treasure events, locked-door events, and container events — each with its own mesh
registration story, placement format, and [E]-key dispatch path. The goal is to unify them around
the **landscape pattern**: register a mesh → place the mesh → put an event on the same tile →
[E] triggers the event. Every interactive object should optionally carry a `mesh_id` (rendered
mesh) and an optional `dialogue_id` (pre-interaction dialogue), so authors can build anything from
a silent loot chest to a scripted barred passage.

## Current State Analysis

### Existing Infrastructure

| System | Mesh support | Placement location | [E] dispatch path | State |
|---|---|---|---|---|
| Landscape | `mesh_id` → `landscape_mesh_registry.ron` (direct) | `landscape_placements[]` array | None (visual only) | Immutable |
| Furniture | `furniture_id` → `furniture.ron` definition → optional `furniture_mesh_registry.ron` (indirect, two-level) | `events{}` map | `try_interact_furniture_door()` (`exploration_interact.rs:147`) | Mutable (`DoorState`) |
| Treasure | None | `events{}` map | `try_interact_adjacent_world_events()` (`exploration_interact.rs:688`) | Consumed on pickup |
| Container | None | `events{}` map | `try_interact_adjacent_world_events()` (`exploration_interact.rs:688`) | Persistent item list |
| LockedContainer | None | `events{}` map | `try_interact_locked_container_event()` (`exploration_interact.rs:345`) | Persistent `LockState` |
| LockedDoor | None | `events{}` map | `try_interact_locked_door_event()` (`exploration_interact.rs:231`) | Persistent `LockState` |
| RecruitableCharacter | Creature mesh (own registry) | `events{}` map | `try_interact_npc_or_recruitable()` (`exploration_interact.rs:501`) | Persistent |
| DroppedItem | None (spawned via `ItemDroppedEvent` path, `item_mesh_registry.ron`) | `events{}` map | item-pickup path | Consumed on pickup |

**Important corrections to prior assumptions:**

- `Furniture` does **not** carry a `mesh_id`. The variant (`src/domain/world/types.rs:2203`) has
  `furniture_id: Option<FurnitureId>` which resolves through a `FurnitureDefinition` in
  `furniture.ron` (`src/domain/world/furniture.rs:157`), which *optionally* points at
  `furniture_mesh_registry.ron`. Mesh resolution for furniture is therefore **indirect and
  two-level** — it is not the simple direct-`mesh_id` pattern that Landscape uses.
- **Three** separate mesh registries exist, not two: `landscape_mesh_registry.ron` (direct
  `mesh_id`, loaded at `src/domain/campaign_loader.rs:492` and `src/sdk/database.rs:1349,1553`),
  `furniture_mesh_registry.ron` (referenced indirectly via furniture definitions, loaded at
  `src/domain/campaign_loader.rs:438`), and `item_mesh_registry.ron` (loaded at
  `src/domain/campaign_loader.rs:410`, `src/domain/items/database.rs:458`).

Six of the eight event types carry no direct mesh field. There is no shared event-mesh spawning
path. The model this plan adopts as canonical is the **Landscape direct-`mesh_id` pattern**, not
the Furniture indirect pattern.

### Identified Issues

1. `MapEvent::Treasure` has no mesh field — the "Barred Passage" in `map_1.ron` is a Treasure with
   no visual representation and no dialogue.
2. `Container`, `LockedContainer`, `LockedDoor` have no mesh fields; authors cannot give them a
   visible 3-D object without layering a separate Furniture or Landscape entry on the same tile
   with no guarantee of despawn linkage.
3. Three disconnected mesh registries (`landscape_mesh_registry.ron`,
   `furniture_mesh_registry.ron`, `item_mesh_registry.ron`) require authors to know which registry
   covers which object type, and use two different resolution styles (direct `mesh_id` vs.
   indirect `furniture_id → definition → registry`).
4. No event type (except `RecruitableCharacter`) despawns a mesh when the event is removed from
   the map.
5. No event type (except `NpcDialogue` / `RecruitableCharacter`) can route the [E] interaction
   through a dialogue tree before executing its effect.

---

## Implementation Phases

### Phase 1: Domain Model — `mesh_id` and `dialogue_id` on Interactive Events

#### 1.1 Foundation Work

Add two optional fields to each interactive `MapEvent` variant in
[`src/domain/world/types.rs`](../../src/domain/world/types.rs):

- `mesh_id: Option<String>` — references a mesh in the shared registry (Phase 4)
- `dialogue_id: Option<crate::domain::dialogue::DialogueId>` — optional pre-interaction dialogue

Target variants and their definition sites:

| Variant | Line | Notes |
|---|---|---|
| `Treasure` | `types.rs:2057` | currently `name`, `description`, `loot: Vec<u8>` only |
| `Sign` | `types.rs:2094` | |
| `Container` | `types.rs:2253` | keyed by `id: String` |
| `LockedContainer` | `types.rs:2343` | keyed by `lock_id: String` |
| `LockedDoor` | `types.rs:2319` | keyed by `lock_id: String` |

Excluded variants and why:

- `Furniture` (`types.rs:2203`) is **not** in scope: it does not have a `mesh_id`; it resolves a
  mesh indirectly via `furniture_id → FurnitureDefinition → furniture_mesh_registry.ron`. Adding a
  direct `mesh_id` here would create two conflicting mesh sources on one variant. Furniture mesh
  unification is handled separately in Phase 4 (registry merge), not by adding a field here.
- `RecruitableCharacter` (`types.rs:2158`) already has its own `dialogue_id` and creature-mesh
  path — left unchanged.
- `DroppedItem` (`types.rs:2299`) is **out of scope**: it has a distinct spawn path
  (`ItemDroppedEvent` / `item_mesh_registry.ron`) and is consumed on pickup. A future plan may
  fold it in; this plan does not.

All new fields must carry `#[serde(default, skip_serializing_if = "Option::is_none")]`. The
`default` keeps existing RON files valid on read; `skip_serializing_if` prevents the Phase 6 map
editor from writing `mesh_id: None, dialogue_id: None` into every event entry on re-serialization,
keeping campaign diffs clean.

#### 1.2 Add Foundation Functionality

Extend `MapEvent` serialization tests in
[`src/domain/world/types.rs`](../../src/domain/world/types.rs) to assert that existing RON round-
trips still parse cleanly with the new defaulted fields.

#### 1.3 Integrate Foundation Work

The five target events do **not** all flow through the same dispatch path, so do not blanket-forward
the new fields through `EventResult`. Per-path integration:

- `Treasure` and `Sign` flow through `trigger_event` →
  `EventResult` (`src/domain/world/events.rs:20,262`). Forward `mesh_id`/`dialogue_id` on the
  existing `EventResult::Treasure` (`events.rs:32`) and `EventResult::Sign` variants so the
  `try_interact_adjacent_world_events` caller (`exploration_interact.rs:688`) can inspect them.
- `Container`, `LockedContainer`, and `LockedDoor` are handled by dedicated dispatchers
  (`try_interact_locked_door_event` `:231`, `try_interact_locked_container_event` `:345`, and the
  container branch of `try_interact_adjacent_world_events`) that read the `MapEvent` directly from
  the map, not through `EventResult`. For these, read `mesh_id`/`dialogue_id` straight off the
  matched `MapEvent` variant inside the dispatcher. Do **not** invent new `EventResult` variants
  for them.

Document in this section which events use which path so the implementing agent does not assume a
single funnel.

#### 1.4 Testing Requirements

- All existing `MapEvent` RON parse tests must still pass.
- New unit tests: verify `Treasure { mesh_id: None, dialogue_id: None }` round-trips; verify
  `Treasure { mesh_id: Some("barred_door"), dialogue_id: Some(42) }` round-trips.

#### 1.5 Deliverables

- [x] `mesh_id` and `dialogue_id` fields added to `Treasure`, `Sign`, `Container`,
  `LockedContainer`, `LockedDoor` in `types.rs`
- [x] `EventResult` variants updated to surface the new fields
- [x] Round-trip serialization tests pass

#### 1.6 Success Criteria

`cargo test` passes. No existing RON campaign file requires modification.

---

### Phase 2: Rendering — Unified Event-Mesh Spawning

#### 2.1 Feature Work

In [`src/game/systems/map.rs`](../../src/game/systems/map.rs), introduce:

- `EventMeshMarker { map_id: MapId, position: Position }` — Bevy component marking a mesh entity
  spawned for a map event.
- `spawn_event_meshes(world: &World, commands: &mut Commands, ...)` — iterates `map.events`, and
  for any event with `mesh_id: Some(id)` looks up the mesh asset path in the shared registry
  (Phase 4 unifies registries; Phase 2 can read `landscape_mesh_registry.ron` as an interim) and
  spawns the mesh entity with the `EventMeshMarker` component.
- `despawn_event_mesh(map_id, position, commands, query)` — finds the `EventMeshMarker` entity
  matching `(map_id, position)` and despawns it; called alongside any `map.remove_event()` call.

Wire `spawn_event_meshes` into the map-load path (currently in the `MapPlugin` setup system).
Wire `despawn_event_mesh` into the `handle_despawn_recruitable_visual` pattern already used for
`RecruitableCharacter` so that Treasure/Container/LockedDoor mesh entities are removed when the
event is consumed.

#### 2.2 Integrate Feature

Reuse the `DespawnRecruitableVisual` message or introduce a parallel `DespawnEventMesh { map_id,
position }` message so systems that consume events (dialogue.rs, exploration_interact.rs) can
trigger mesh removal without holding a direct reference to `Commands`.

#### 2.3 Configuration Updates

No new RON format change. Authors set `mesh_id` in the event entry in the map file. Mesh asset
paths are still resolved via the existing registry files until Phase 4 unifies them.

#### 2.4 Testing Requirements

- Unit test: place a `Treasure { mesh_id: Some("test_mesh"), .. }` event on a map, run the spawn
  system, assert an entity with `EventMeshMarker { position }` exists.
- Unit test: remove the event, run the despawn path, assert the entity is gone.

#### 2.5 Deliverables

- [x] `EventMeshMarker` component defined
- [x] `spawn_event_meshes` system implemented and wired into map load
- [x] `DespawnEventMesh` message and `handle_despawn_event_mesh` handler implemented
- [x] `cleanup_event_mesh_markers` passive cleanup wired into `MapManagerPlugin`
- [ ] Existing `RecruitableCharacter` mesh spawning refactored to use the shared path (deferred to Phase 4 — recruitable path uses creature-specific components distinct from generic event meshes)

#### 2.6 Success Criteria

Setting `mesh_id` on any supported event type causes a visible mesh to appear on the map tile.
Consuming the event (looting treasure, unlocking a door) removes the mesh.

---

### Phase 3: Interaction — Dialogue Routing for `dialogue_id` Events

#### 3.1 Feature Work

In [`src/game/systems/input/exploration_interact.rs`](../../src/game/systems/input/exploration_interact.rs),
update the [E]-dispatch for `Treasure`, `Sign`, `Container`, `LockedContainer`, `LockedDoor`:

- If the event's `dialogue_id` is `Some(id)`, open the dialogue tree **instead of** executing the
  immediate effect. **Note: there is no existing `open_dialogue_for_event()` helper** — this is new
  work. Dialogue is started by setting `GameMode::Dialogue(DialogueState { .. })` (see how
  `try_interact_npc_or_recruitable` at `exploration_interact.rs:501` and `dialogue.rs:211–224`
  construct and enter dialogue state). Implement a new helper (suggested name
  `open_dialogue_for_event`) that builds a `DialogueState` for the given `dialogue_id` and carries
  the event's position forward (see next bullet).
- The existing "remove event after dialogue" mechanism keys off
  `DialogueState.recruitment_context.event_position` (`dialogue.rs:789–809`), which is
  **recruitment-specific**. Do not overload it. Introduce a generalized
  `event_context: Option<EventInteractionContext>` field on `DialogueState` carrying at minimum
  `event_position: Position` (and `map_id`), set when opening a dialogue for a Treasure/Container/
  Door event, so the `TriggerEvent` handlers can locate and remove the originating event.
- The dialogue must be able to trigger the original effect via a `DialogueAction::TriggerEvent`
  action (mechanism exists: `dialogue.rs:1010` `execute_action`, matched at `:1095`). Add new
  `event_name` branches alongside the existing `"open_inn_party_management"` /
  `"recruit_character_to_party"` / `"recruit_character_to_inn"` branches:
  - `"collect_treasure"` — runs the existing loot-collection + event-removal logic currently inside
    `try_interact_adjacent_world_events` (`exploration_interact.rs:688`); extract it into a shared
    function callable from both [E]-dispatch and the dialogue handler rather than duplicating it.
  - `"open_container"` — opens the container UI (reuse the `EventResult::EnterContainer` path)
  - `"unlock_door"` — attempts key/lockpick unlock (reuse `try_interact_locked_door_event` logic)
  - `"unlock_container"` — attempts key/lockpick unlock for containers (reuse
    `try_interact_locked_container_event` logic)

When `dialogue_id` is `None`, existing behavior is preserved exactly.

#### 3.2 Integrate Feature

Update `execute_action` in `dialogue.rs` (`:1010`) to handle the new `TriggerEvent` names. Each
handler reads the originating tile from the new `DialogueState.event_context.event_position`
(introduced in 3.1 — **not** `recruitment_context`), calls `map.remove_event(position)`, and emits
`DespawnEventMesh` so the mesh disappears after the dialogue-driven interaction. Mirror the existing
recruitment flow at `dialogue.rs:789–809`, which already does `remove_event` +
`DespawnRecruitableVisual`.

#### 3.3 Configuration Updates

Campaign authors add `dialogue_id: Some(42)` to an event entry in the map RON; the dialogue tree
with id `42` in `dialogues.ron` presents choices and ends with a `TriggerEvent` action.

#### 3.4 Testing Requirements

- Unit test in `dialogue.rs`: `TriggerEvent("collect_treasure")` with a treasure at
  `event_context.event_position` (the new generalized context) removes the event and emits
  `DespawnEventMesh`.
- Unit test in `exploration_interact.rs` (or integration): pressing [E] on a `Treasure` with
  `dialogue_id: Some(id)` opens the dialogue rather than immediately granting loot.

#### 3.5 Deliverables

- [ ] `EventInteractionContext` + `event_context: Option<EventInteractionContext>` field added to
  `DialogueState` (generalized, separate from `recruitment_context`)
- [ ] New `open_dialogue_for_event` helper in `exploration_interact.rs` that builds `DialogueState`
  and sets `event_context`
- [ ] Loot-collection logic extracted from `try_interact_adjacent_world_events` into a shared
  function callable from both [E]-dispatch and the dialogue handler
- [ ] `dialogue_id` routing added to [E]-dispatch for Treasure, Sign, Container, LockedContainer,
  LockedDoor
- [ ] `TriggerEvent` handlers: `"collect_treasure"`, `"open_container"`, `"unlock_door"`,
  `"unlock_container"`
- [ ] Event removal + `DespawnEventMesh` wired inside each handler

#### 3.6 Success Criteria

A `Treasure { dialogue_id: Some(42), .. }` event opens a dialogue on [E]. A `TriggerEvent`
action inside that dialogue collects the loot and removes the event (and its mesh if present).

---

### Phase 4: Mesh Registry Unification

#### 4.1 Foundation Work

Create `campaigns/{campaign}/data/object_mesh_registry.ron` as the single source of mesh asset
paths for all interactive objects (landscape, furniture, events). Schema mirrors
`landscape_mesh_registry.ron`:

```
ObjectMeshRegistry(meshes: { "barred_door": "path/to/barred_door.ron", ... })
```

Add `object_mesh_registry.ron` loading in the **same modules that load the existing registries** —
the prior reference to `src/sdk/campaign_loader.rs` was incorrect (that file deserializes
`CampaignConfig`, not mesh registries). The registry loaders live in:

- [`src/domain/campaign_loader.rs`](../../src/domain/campaign_loader.rs) — loads
  `item_mesh_registry.ron` (`:410`), `furniture_mesh_registry.ron` (`:438`), and
  `landscape_mesh_registry.ron` (`:492`).
- [`src/sdk/database.rs`](../../src/sdk/database.rs) — loads `landscape_mesh_registry.ron`
  (`:1349`, `:1553`).

Add an `object_mesh_registry.ron` load path to both, following each module's existing
exists-guard + load convention.

**Scope decision — `item_mesh_registry.ron`:** there are **three** legacy registries, not two.
This phase unifies `landscape_mesh_registry.ron` and `furniture_mesh_registry.ron` into
`object_mesh_registry.ron` (both cover placeable interactive objects). `item_mesh_registry.ron`
stays separate — it backs the distinct `DroppedItem` / item-pickup spawn path, which is out of
scope (see Phase 1.1). State this explicitly so the implementing agent does not silently drop or
merge the item registry.

#### 4.2 Integrate Feature

- Update `spawn_event_meshes` (Phase 2) to resolve `mesh_id` via `object_mesh_registry.ron`.
- Update landscape placement spawning to also resolve via `object_mesh_registry.ron`.
- Keep `furniture_mesh_registry.ron` and `landscape_mesh_registry.ron` as deprecated aliases
  (still loaded, merged into the unified registry at load time) so existing campaigns don't break.

#### 4.3 Configuration Updates

SDK Campaign Builder: add an "Object Meshes" section to the importer (matching the existing
landscape/furniture importer UI) so authors can add mesh entries to `object_mesh_registry.ron`.

#### 4.4 Testing Requirements

- Integration test: load a campaign with `object_mesh_registry.ron`; confirm landscape and event
  meshes resolve correctly.
- Integration test: load a campaign with only the legacy registries; confirm backward-compat merge
  still resolves all mesh IDs.

#### 4.5 Deliverables

- [ ] `object_mesh_registry.ron` schema + loader added to `src/domain/campaign_loader.rs` and
  `src/sdk/database.rs` (not `src/sdk/campaign_loader.rs`)
- [ ] New `object_mesh_registry.ron` files carry an SPDX license header per `PLAN.md` / SPDX spec
- [ ] Backward-compat merge of legacy `furniture_mesh_registry.ron` (resolved through furniture
  definitions) and `landscape_mesh_registry.ron`; `item_mesh_registry.ron` left untouched
- [ ] `spawn_event_meshes` updated to use unified registry
- [ ] SDK Campaign Builder "Object Meshes" importer panel

#### 4.6 Success Criteria

A mesh ID resolves correctly regardless of whether it was defined in `object_mesh_registry.ron`,
`furniture_mesh_registry.ron`, or `landscape_mesh_registry.ron`. Campaign load validation reports
unknown mesh IDs with a clear error.

---

### Phase 5: Campaign Data — Convert Barred Passage and Author Guide

#### 5.1 Feature Work

Update `campaigns/tutorial/data/maps/map_1.ron`: replace the Barred Passage `Treasure` entry with:

```ron
(x: 17, y: 12): Treasure(
    name: "Barred Passage",
    description: "A heavy iron bar blocks the passage.",
    loot: [],
    mesh_id: Some("barred_passage"),
    dialogue_id: Some(500),
),
```

Register `"barred_passage"` in `campaigns/tutorial/data/object_mesh_registry.ron` pointing to the
appropriate mesh asset (reuse an existing door or gate mesh).

Add dialogue tree `500` to `campaigns/tutorial/data/dialogues.ron`:

```
Node 1: "The passage is barred shut. You need to find a way to open it."
  Choice: "Leave it for now." → ends_dialogue: true
```

(A future quest phase can add a `TriggerEvent("unlock_door")` choice once a key item exists.)

#### 5.2 Integrate Feature

Verify that the Barred Passage mesh spawns on map load and that pressing [E] opens the dialogue.
Confirm the mesh remains after dialogue (no loot collected, event not consumed).

#### 5.3 Configuration Updates

None beyond the RON changes above.

#### 5.4 Testing Requirements

- Tutorial campaign integration test: Barred Passage tile at `(17, 12)` has a mesh entity with
  `EventMeshMarker` after map load.
- Tutorial campaign integration test: the Treasure event at `(17, 12)` references dialogue id 500
  which exists in `dialogues.ron`.

#### 5.5 Deliverables

- [ ] `map_1.ron` Barred Passage updated with `mesh_id` and `dialogue_id`
- [ ] Barred Passage mesh registered in `object_mesh_registry.ron`
- [ ] Dialogue tree 500 added to `dialogues.ron`
- [ ] Tutorial integration tests updated/passing

#### 5.6 Success Criteria

Running the tutorial campaign shows a visible barred passage mesh at the Barred Passage tile.
Pressing [E] displays "The passage is barred shut." No crash, no silent loot event.

---

### Phase 6: SDK Map Editor — Event Placement with Mesh and Dialogue

This phase must satisfy every applicable rule in [`sdk/AGENTS.md`](../../sdk/AGENTS.md). Each
rule number below maps directly to that document.

#### 6.1 Foundation Work — `map_editor.rs` Query Helpers

Extend [`src/sdk/map_editor.rs`](../../src/sdk/map_editor.rs) with query helpers the Campaign
Builder UI calls (never called inside the widget render loop — see Rule 14):

- `browse_event_mesh_ids(registry) -> Vec<String>` — all IDs in the unified object mesh registry.
- `suggest_event_mesh_ids(registry, partial) -> Vec<String>` — prefix-filtered candidate list.
- `browse_dialogue_ids(db) -> Vec<(DialogueId, String)>` — all `(id, title)` pairs.
- `suggest_dialogue_ids(db, partial) -> Vec<(DialogueId, String)>` — prefix-filtered.
- `place_map_event(map, pos, event) -> Result<(), MapEditorError>` — validates position is
  in-bounds and unoccupied, then inserts; errors descriptively otherwise.
- `remove_map_event(map, pos) -> Option<MapEvent>` — removes and returns the event.
- `set_event_mesh_id(map, pos, mesh_id: Option<String>) -> Result<(), MapEditorError>` — patches
  `mesh_id` on an existing event; returns `Err(UnknownMesh)` if the ID is not in the registry.
- `set_event_dialogue_id(map, pos, id: Option<DialogueId>) -> Result<(), MapEditorError>` —
  patches `dialogue_id`; returns `Err(UnknownDialogue)` if the ID is absent from the database.

**Rule 13** — The map event data-file loader in the Campaign Builder must follow the standard load
pattern: exists-guard before reading, `warn!` on missing file, reset flag before auto-load, and
use the shared `load_ron_file` helper (same pattern as the landscape and furniture loaders).

#### 6.2 Campaign Builder UI Panel

Add a Map Event inspector panel to the Campaign Builder, following the standard recipe from
`sdk/AGENTS.md § Future Editor Standardization Pattern`:

**Layout (Rule 9)** — Use `TwoColumnLayout::new("map_event_editor").show_split(ui, left, right)`
from `ui_helpers.rs`. Never use `SidePanel::right().show_inside()` for the list/detail split.

**Pre-computation (Rule 10)** — Before calling `show_split`, extract all data needed by both
closures into owned locals: `tile_rows: Vec<TileRowData>` (position, event type name, has-mesh
flag), `selected_event_snapshot: Option<EventEditSnapshot>`, `available_mesh_ids: Vec<String>`,
`available_dialogue_ids: Vec<(DialogueId, String)>`. These are fields on the editor state struct
refreshed when the campaign directory changes — never computed inside the closure.

**Left panel — tile list (Rules 1, 2, 15):**
- Wrap in `ScrollArea::vertical().id_salt("map_event_tile_list_scroll").show(ui, |ui| { ... })`.
- Each row loop body uses `ui.push_id(tile_position, |ui| { ... })` with the `Position` as the
  stable unique key.
- Each row rendered via `show_standard_list_item(ui, config)` with a `StandardListItemConfig`
  carrying the tile coordinate as the primary label and a `MetadataBadge` showing the event type
  name (e.g., `"Treasure"`, `"Container"`) in a colored badge.
- No bare `selectable_label` with embedded metadata strings; no inline `context_menu` blocks.
- Mutations (`pending_select`, `pending_delete`) are deferred and applied after `show_split`
  returns.
- Clicking a row calls `ui.ctx().request_repaint()` (Rule 7).

**Right panel — event editor form (Rules 3, 7, 14):**
- **Event type** — `ComboBox::from_id_salt("map_event_type_combo", ...)` listing all supported
  `MapEvent` variant names. Changing selection calls `request_repaint()`.
- **`mesh_id` field** — `autocomplete_mesh_id_selector(ui, "event_mesh_id", "Mesh ID:",
  &mut self.edit_buffer.mesh_id, &self.available_mesh_ids)`. This requires adding a new
  `autocomplete_mesh_id_selector` helper to `ui_helpers.rs` modelled on
  `autocomplete_creature_selector`: uses `make_autocomplete_id`, `load_autocomplete_buffer` /
  `store_autocomplete_buffer`, shows a tooltip with the resolved asset path or a `⚠` for unknown
  IDs, and includes a built-in **Clear** button.
- **`dialogue_id` field** — `autocomplete_dialogue_selector(ui, "event_dialogue_id",
  "Dialogue ID:", &mut self.edit_buffer.dialogue_id_buf, &self.available_dialogue_ids)`. Same
  pattern; shows dialogue title in the tooltip.
- **Companion requirements (Rule 14):** editor state struct carries
  `available_mesh_ids: Vec<String>` and `available_dialogue_ids: Vec<(DialogueId, String)>`
  rebuilt when campaign directory changes. Both autocomplete buffers are cleared in the
  `reset_autocomplete_buffers` block. Picker-sync (when Browse modal writes an ID, call
  `store_autocomplete_buffer` so the text field updates immediately).

**Bottom action row (Rules 12, 16):**
After the edit form content, render:
```
ui.separator();
ui.horizontal_wrapped(|ui| {
    ⬅ Back to List  |  💾 Save  |  ✕ Cancel  |  🗑 Remove Event
});
```
Each button that changes layout-driving state calls `ui.ctx().request_repaint()`. Use
`ui.horizontal_wrapped` — never `ui.horizontal` — so the row doesn't clip on narrow windows.
`Save` calls `set_event_mesh_id` and `set_event_dialogue_id` (validating against registry/db)
then serializes the map RON. Inline error shown in the panel if validation fails.

#### 6.3 Integrate Feature

Wire autocomplete cache rebuilds alongside the existing `available_portraits` /
`available_sprite_sheets` rebuild in the Campaign Builder's `show()` function. Wire RON save
through the same path the landscape editor uses. Validate `mesh_id` via `set_event_mesh_id` before
saving; surface `Err(UnknownMesh)` as a visible inline error, not a silent no-op.

#### 6.4 Testing Requirements (Rule 11 — Test Contracts, Not Implementation Constants)

Tests assert observable contracts, not internal constants:

- `place_map_event` succeeds on a valid empty position; returns `Err` on occupied position; returns
  `Err` on out-of-bounds position.
- `set_event_mesh_id` with a known mesh ID returns `Ok`; with unknown ID returns
  `Err(MapEditorError::UnknownMesh)`.
- `set_event_dialogue_id` with a known dialogue ID returns `Ok`; with unknown ID returns
  `Err(MapEditorError::UnknownDialogue)`.
- `suggest_event_mesh_ids` returns only entries where the ID starts with the supplied prefix
  (case-insensitive).
- `browse_dialogue_ids` returns one entry per dialogue tree in the database.
- `autocomplete_mesh_id_selector` unit tests (in `ui_helpers.rs`): display format, ID extraction,
  unknown-ID `⚠` fallback, empty-value initialisation (Rule 14 requirement).

#### 6.5 Deliverables

- [ ] `browse_event_mesh_ids`, `suggest_event_mesh_ids` added to `map_editor.rs`
- [ ] `browse_dialogue_ids`, `suggest_dialogue_ids` added to `map_editor.rs`
- [ ] `place_map_event`, `remove_map_event`, `set_event_mesh_id`, `set_event_dialogue_id` added
- [ ] `MapEditorError::UnknownMesh` and `MapEditorError::UnknownDialogue` variants added
- [ ] `autocomplete_mesh_id_selector` added to `ui_helpers.rs` (Rule 14)
- [ ] `autocomplete_dialogue_selector` added to `ui_helpers.rs` (Rule 14) if not present
- [ ] Campaign Builder map event panel: `TwoColumnLayout`, `show_standard_list_item` list,
  `ComboBox::from_id_salt` type picker, autocomplete fields, `horizontal_wrapped` action row
- [ ] `available_mesh_ids` / `available_dialogue_ids` cache fields + buffer-clear + picker-sync
- [ ] All `map_editor.rs` and `ui_helpers.rs` unit tests pass
- [ ] AGENTS.md acceptance checklist passes for the new panel

#### 6.6 Success Criteria

An author opens the Campaign Builder, selects a tile on the map, picks "Treasure" from the event
type ComboBox, types `"bar"` in the Mesh ID field and sees `"barred_passage"` suggested, selects
it, sets Dialogue ID to `500` (title shown in tooltip), clicks Save. The map RON file updates
correctly. No bare `TextEdit` for reference ID fields; no `SidePanel::right`; no `ui.horizontal`
action row; no loop without `push_id`.

---

### Phase 7: Documentation

#### 7.1 Author Guide — Interactive Object Placement

Update [`docs/explanation/modding_guide.md`](./modding_guide.md) with a new section:
**"Interactive Objects — Meshes and Events"** covering:

- The unified model: every interactive event can carry an optional mesh and an optional dialogue.
- The placement workflow: register mesh in `object_mesh_registry.ron` → place event in map RON or
  via the Map Editor → optionally author a dialogue tree → wire dialogue back to the event via
  `dialogue_id`.
- Reference table of all supported event types, which fields they expose (`mesh_id`, `dialogue_id`,
  `loot`, `key_item_id`, etc.), and what `TriggerEvent` action names are available inside dialogue.
- A worked example: "Creating a Locked Gate" — register a gate mesh, place a `LockedDoor` event
  with `mesh_id: Some("iron_gate")` and `dialogue_id: Some(601)`, author the dialogue, add a key
  item.

#### 7.2 SDK Reference — Map Editor API

Add or update the SDK-level doc comment block at the top of
[`src/sdk/map_editor.rs`](../../src/sdk/map_editor.rs) with:

- A short description of each public function group (browse, suggest, place, validate).
- A code example showing the full workflow: load campaign → place event with mesh → save RON.

#### 7.3 Inline Code Documentation

- Update `src/domain/world/types.rs` doc comments on each `MapEvent` variant to document the new
  `mesh_id` and `dialogue_id` fields and their semantics.
- Update `src/game/systems/map.rs` doc comment on `spawn_event_meshes` and `EventMeshMarker`.
- Update `src/game/systems/dialogue.rs` doc comments on `execute_action` to list all supported
  `TriggerEvent` names and what each one does.

#### 7.4 Deliverables

- [ ] `modding_guide.md` "Interactive Objects" section written
- [ ] Worked "Locked Gate" example in modding guide
- [ ] `TriggerEvent` name reference table in `dialogue.rs` doc comment
- [ ] `map_editor.rs` module-level doc block updated with placement workflow example
- [ ] `MapEvent` variant doc comments updated for `mesh_id` and `dialogue_id`

#### 7.5 Success Criteria

A new campaign author can follow only the modding guide to create a chest with a custom mesh and
a dialogue interaction, without reading source code.
