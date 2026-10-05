# Implementations

Consolidated log of major implementation work, newest themes first. Per-phase
detail (file lists, quality-gate output, test inventories) was removed; see git
history for the full record.

---

## Audio

### Audio Manager (Phases 1–3 + SDK layout compliance)

- **Domain**: `AudioMap` struct and `audio.ron` format for per-campaign audio
  mapping.
- **Game**: `src/game/systems/audio.rs` loads and applies the `AudioMap` at
  runtime.
- **SDK**: `AudioEditorState` and the Audio tab in the Campaign Builder, with
  audio file preview; layout fixed to comply with SDK AGENTS.md Rules 13, 15, 17.

### Audio RON & SFX Mapping (Phases 1–4)

- `AudioManifest` definition (Phase 1) and campaign loader support for
  `audio.ron` (Phase 2).
- Bevy integration: `AudioManifestResource` and an updated `resolve_audio_path`
  (Phase 3).
- Test fixtures under `data/test_campaign` and integration tests, with no
  references to `campaigns/tutorial` (Phase 4).

---

## Terrain Externalization (Phases 1–6)

Replaced the closed `TerrainType` enum with the open numeric `TerrainId = u32`
backed by data-driven definitions. Breaking, non-backwards-compatible change.

1. **Domain foundation**: `TerrainDefinition` / `TerrainDatabase` registry.
2. **Type migration**: `TerrainType` removed across domain, game resources,
   game systems and SDK; built-in `TERRAIN_*` constants.
3. **Built-in content**: `data/terrain.ron` with 12 built-in terrains and a
   `builtin` short-name module.
4. **Rendering/gameplay**: mesh style, vegetation, height and color now read
   from `TerrainDefinition`.
5. **SDK tooling**: `map_builder`, `texture_generator`, templates, Terrain
   Editor, and `map_editor.rs` migrated to `TerrainId`.
6. **Cleanup**: remaining legacy data migrated, no Rust references to
   `TerrainType` remain, docs updated.

Related fixes: Terrain Editor stored absolute machine-specific texture paths
(now campaign-relative); `map_7` and `map_8` updated to use Sand, Ice and Snow.

---

## Object Mesh Registry Refactor (Phases 1–4)

- `ObjectMeshEntry { id, name, ... }` introduced as the domain type; registry
  is now an array of entries instead of a string-keyed map.
- Legacy `ObjectMeshRegistry(meshes: {...})` fallback removed and all data
  converted.
- SDK data model (`ObjectEntry` with numeric `id` and `name`) and all SDK call
  sites migrated.
- Objects Editor and Map Editor mesh picker show `(id, name)` pairs.

---

## NPC Combat Switch and Eonir Boss Fight (Phases 1–4)

- **Domain**: `NpcCombatSwitch` on `NpcDefinition`.
- **Engine**: `PendingNpcCombatResource`, `npc_combat_switch_system`,
  `start_npc_combat_system`, and an NPC spawn guard.
- **Post-combat**: `defeat_flag` propagated to `GlobalFlags` on victory.
- **Tutorial**: Eonir the Still boss encounter with a combat path and a
  dialogue path.

---

## Inventory UI Refactor (Phases 1–5)

- `InventoryViewMode { Multi, Single }` with number-key switching.
- 8×8 icon grids replaced by scrollable text lists with linear navigation in
  the character, merchant and container panels.
- Dead silhouette-rendering code removed.
- Items now group/stack in character inventory; `ScrollArea` `auto_shrink` and
  merchant scroll-to-selection fixed.

---

## SDK / Campaign Builder

- **Skill trainer repair (Phases 1–4)**: `DialogueTree` repair methods, wired
  into the dialogue editor; Dialogue ID picker in NPC trainer sections; trainer
  badges in the NPC preview panel.
- **Creature Editor**: mesh metadata editing panel (plus visibility fix),
  `registry_dirty` flag, creature browser matching the Character Editor picker.
- **Editor toolbars**: Landscape and Objects editors migrated to `EditorToolbar`
  / `EditorContext`; three broken editors and a missing Campaign Metadata field
  fixed.
- **Creature persistence**: fixed `creatures.ron` failing to load and being
  wiped to `[]` on save.
- **Starting items**: stacked display and correct `charges` in
  `populate_starting_inventory`.
- **Objects Editor**: right-click Duplicate now works.
- **Performance**: NPC tab O(N×D) per-frame validation hotspot removed.

---

## Bug Fixes

| Area            | Problem                                                              | Fix                                                                            |
| --------------- | -------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Inn             | Party members still listed as available to recruit                   | Match party member to roster by name + `InParty`, not position                 |
| Recruitment     | Premade characters duplicated in roster when recruited from the map  | `recruit_from_map` and `execute_recruit_to_inn` reuse the existing roster entry |
| Inventory       | Healing Potions / Food Rations did not stack                         | `build_grouped_inventory` no longer requires `charges == 0`                    |
| Combat          | Long Bow caused endless attacks and always showed "Miss"             | Two combined bugs in ranged attack handling fixed                              |
| HUD             | Grey portrait frame missing for starting characters                  | Timing difference in portrait loading corrected                                |
| Consumables     | Potions and scrolls had no persistent effect                         | Two independent root causes fixed                                              |
| Merchant        | Sold items missing from stock; sell price ignored `sell_cost`        | Stock entry created on sale; shared price helper used                          |
| Map events      | Static `MapEvent::DroppedItem` could not be picked up                | Pickup now handles both dropped-item storage locations                         |
| Rendering       | Furniture `mesh_id` dropped; event objects floating / untextured     | Imported-mesh branch added; Y offset and `asset_server` textures fixed         |
| Containers      | Chest could break after taking an item then pressing ESC             | `ContainerInventory` added to `movement_blocked_for_mode`                      |
| Locked doors    | Dungeon Gate Key did not open Barred Passage                         | Key check runs before `dialogue_id` handling; Pick Lock / Bash fallback added  |

---

## Tutorial Campaign Content

- **Dialogue overhaul**: every character recruitable, lore-consistent trees,
  duplicate id bug (Eonir vs Isolde, 1003) fixed.
- **Maps**: names and descriptions aligned to _The Stillness Prophecy_; Frostspire
  Peaks (`map_8`) populated with Dire Wolves, Skeletons and Eonir via fixed
  events and a random encounter table.
- **Lore**: all eight premade-character lore files and NPC data updated.

---

## Character Sheet

- Character Bio & Navigation Phase 5: mouse input fix for the character sheet.
