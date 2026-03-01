# Items Procedural Meshes Implementation Plan

## Overview

This plan implements procedural 3D meshes for dropped items in the game world.
When a player drops an item from inventory it spawns a procedural mesh on the
tile. When the player picks it up the mesh is despawned. Meshes are generated
from item data in RON files using the same `MeshDefinition` / `CreatureDefinition`
pipeline already used for creatures and furniture. Each `ItemType` variant
(Weapon, Armor, Accessory, Consumable, Ammo, Quest) drives a distinct mesh
shape; sub-type data (blade length, potion color, charge level, etc.) drives
visual variation within each category.

---

## Current State Analysis

### Existing Infrastructure

| Subsystem                                                | Location                                  | Status      |
| -------------------------------------------------------- | ----------------------------------------- | ----------- |
| `MeshDefinition` / `CreatureDefinition` domain types     | `src/domain/visual/mod.rs`                | ✅ Complete |
| `CreatureDatabase`                                       | `src/domain/visual/creature_database.rs`  | ✅ Complete |
| `mesh_definition_to_bevy` conversion                     | `src/game/systems/creature_meshes.rs`     | ✅ Complete |
| `spawn_creature` hierarchical entity spawner             | `src/game/systems/creature_spawning.rs`   | ✅ Complete |
| `ProceduralMeshCache`                                    | `src/game/systems/procedural_meshes.rs`   | ✅ Complete |
| Furniture spawn pipeline (config structs + spawn fns)    | `src/game/systems/procedural_meshes.rs`   | ✅ Complete |
| `MapEvent::Furniture` → `spawn_furniture_with_rendering` | `src/game/systems/furniture_rendering.rs` | ✅ Complete |
| `ItemDatabase` with full `Item` / `ItemType` definitions | `src/domain/items/database.rs`            | ✅ Complete |
| `MapEvent` enum                                          | `src/domain/world/types.rs`               | ✅ Complete |
| `GameContent` resource with loaded databases             | `src/application/resources.rs`            | ✅ Complete |

### Identified Issues

1. **No item visual layer** — `Item` has only `icon_path` (2-D sprite); there
   is no `mesh_id` field or `ItemMeshDefinition` type for 3-D world meshes.
2. **No dropped-item ECS component** — nothing tracks an item lying on the
   ground as a Bevy entity; pick-up / drop events have no visual counterpart.
3. **No item mesh generation functions** — `procedural_meshes.rs` covers
   furniture and trees but has no sword, potion, scroll, ring, etc. generators.
4. **No item mesh RON assets** — `campaigns/tutorial/assets/` contains only
   `creatures/`; there is no `items/` subdirectory with per-item-type mesh
   files.
5. **No drop / pick-up game event** — `MapEvent` has no `DroppedItem` variant;
   the inventory system has no bridge to the world-spawn layer.
6. **`ProceduralMeshCache` has no item slots** — cache fields cover trees,
   furniture, structures, and creatures but not item shapes.

---

## Implementation Phases

---

### Phase 1: Domain Layer — Item Mesh Types

#### 1.1 Add `ItemMeshDescriptor` to the Domain

Create `src/domain/visual/item_mesh.rs`.

Responsibilities:

- Define `ItemMeshDescriptor` — the data that drives procedural generation for
  a single item category. Fields cover shape parameters, primary color, accent
  color, emissive flag, and scale.
- Define the `ItemMeshCategory` enum: `Sword`, `Dagger`, `Blunt`, `Staff`,
  `Bow`, `BodyArmor`, `Helmet`, `Shield`, `Boots`, `Ring`, `Amulet`, `Belt`,
  `Cloak`, `Potion`, `Scroll`, `Ammo`, `QuestItem`.
- Implement `ItemMeshDescriptor::from_item(item: &Item) -> Self` — a pure
  function that reads `item.item_type`, sub-type classification fields, bonus
  values, and tags to produce a descriptor with no Bevy dependency.
- Write `ItemMeshDescriptor::to_creature_definition(&self) -> CreatureDefinition`
  — converts the descriptor to the shared `CreatureDefinition` type so the
  existing `spawn_creature` function can render items without duplication.
- Register the module in `src/domain/visual/mod.rs` with
  `pub mod item_mesh;` and re-export the two public types.

Key decisions:

- Reuse `CreatureDefinition` as the output type — no new rendering path is
  needed; the item sits on the ground as a flat-rotated creature.
- Weapon length is derived from `WeaponData::damage.sides` (more sides → longer
  blade for swords/staves) plus the `two_handed` tag for scale.
- Potion color is derived from `ConsumableData::effect` variant
  (HealHp → red, RestoreSp → blue, CureCondition → green, BoostAttribute →
  yellow).
- Magical items (`Item::is_magical()`) receive an emissive material glow.
- Cursed items receive a dark-tinted material with slight purple emissive.

#### 1.2 Extend `Item` with an Optional Mesh Override

Add `#[serde(default)] pub mesh_descriptor_override: Option<ItemMeshDescriptorOverride>` to
`Item` in `src/domain/items/types.rs`. The override struct lives in
`src/domain/visual/item_mesh.rs` and mirrors a subset of `ItemMeshDescriptor`
fields (`primary_color`, `accent_color`, `scale`, `emissive`), allowing
campaign authors to customize the visual without changing gameplay data.

Use `#[serde(default)]` so all existing RON files remain valid without
modification.

#### 1.3 Extend `ItemDatabase` Validation

Add `validate_mesh_descriptors(&self) -> Result<(), ItemDatabaseError>` to
`ItemDatabase` in `src/domain/items/database.rs`. It calls
`ItemMeshDescriptor::from_item` for every loaded item and validates the
resulting `CreatureDefinition` via the existing `CreatureDefinition::validate`.
Plumb this into `sdk/validation.rs` so campaign validation checks it
automatically.

#### 1.4 Testing Requirements

File: `src/domain/visual/item_mesh.rs` (inline `mod tests`)

- `test_sword_descriptor_from_short_sword` — short sword produces `Sword`
  category, modest blade length, no emissive.
- `test_dagger_descriptor_short_blade` — dagger produces `Dagger` category,
  shorter blade than sword.
- `test_potion_color_heal_is_red` — HealHp consumable yields red primary color.
- `test_potion_color_restore_sp_is_blue` — RestoreSp yields blue.
- `test_magical_item_emissive` — item with `max_charges > 0` produces
  emissive material.
- `test_cursed_item_dark_tint` — `is_cursed: true` produces dark purple tint.
- `test_two_handed_weapon_larger_scale` — `tags: ["two_handed"]` produces
  scale > 1.0 relative to one-handed equivalent.
- `test_descriptor_to_creature_definition_valid` — round-trips through
  `to_creature_definition` and passes `CreatureDefinition::validate`.
- `test_override_color_applied` — `mesh_descriptor_override` with a custom
  primary color produces matching color in the output `CreatureDefinition`.
- `test_quest_item_descriptor_unique_shape` — quest items get a distinct
  `QuestItem` category with scroll-like geometry.

File: `src/domain/items/database.rs` (extend existing `mod tests`)

- `test_validate_mesh_descriptors_all_base_items` — loads `data/items.ron` and
  asserts `validate_mesh_descriptors` returns `Ok(())`.

#### 1.5 Deliverables

- [ ] `src/domain/visual/item_mesh.rs` with `ItemMeshCategory`,
      `ItemMeshDescriptor`, `ItemMeshDescriptorOverride`, `from_item`, and
      `to_creature_definition`
- [ ] `src/domain/visual/mod.rs` updated with `pub mod item_mesh;`
- [ ] `src/domain/items/types.rs` updated with `mesh_descriptor_override` field
- [ ] `src/domain/items/database.rs` updated with `validate_mesh_descriptors`
- [ ] `src/sdk/validation.rs` plumbed to call mesh descriptor validation
- [ ] Unit tests achieving >80 % coverage for new code
- [ ] `docs/explanation/implementations.md` updated

#### 1.6 Success Criteria

- `cargo check --all-targets --all-features` passes with zero errors.
- `cargo clippy --all-targets --all-features -- -D warnings` passes with zero
  warnings.
- `cargo nextest run --all-features` passes 100 %.
- `ItemMeshDescriptor::from_item` produces a valid `CreatureDefinition` for
  every item in `data/items.ron`.
- All existing RON item files load without modification.

---

### Phase 2: Game Engine — Dropped Item Mesh Generation

#### 2.1 Add Item-Specific Mesh Generators

Extend `src/game/systems/procedural_meshes.rs` with item shape generator
functions following the same pattern as `spawn_bench`, `spawn_chest`, etc.:

| Generator           | Config struct                                                      | Produced shape                                                                  |
| ------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------- |
| `spawn_sword_mesh`  | `SwordConfig { blade_length, blade_width, has_crossguard, color }` | Elongated box blade + crossguard quad + handle box                              |
| `spawn_dagger_mesh` | `DaggerConfig { blade_length, color }`                             | Short blade + small handle                                                      |
| `spawn_blunt_mesh`  | `BluntConfig { head_radius, handle_length, color }`                | Cylindrical head + handle                                                       |
| `spawn_staff_mesh`  | `StaffConfig { length, orb_radius, color }`                        | Long thin cylinder + sphere tip                                                 |
| `spawn_bow_mesh`    | `BowConfig { arc_height, color }`                                  | Curved arc of quads + string line                                               |
| `spawn_armor_mesh`  | `ArmorMeshConfig { width, height, color, is_helmet }`              | Layered box chest piece or dome                                                 |
| `spawn_shield_mesh` | `ShieldConfig { radius, color }`                                   | Hexagonal polygon disc                                                          |
| `spawn_potion_mesh` | `PotionConfig { liquid_color, bottle_color }`                      | Tapered cylinder body + sphere stopper; liquid interior uses `AlphaMode::Blend` |
| `spawn_scroll_mesh` | `ScrollConfig { color }`                                           | Rolled cylinder pair                                                            |
| `spawn_ring_mesh`   | `RingMeshConfig { color }`                                         | Torus approximated with arc of thin quads                                       |
| `spawn_ammo_mesh`   | `AmmoConfig { ammo_type, color }`                                  | Arrow shaft + fletching; bolt variant; stone sphere                             |

All generators:

- Accept `commands`, `materials`, `meshes`, `position: types::Position`,
  `map_id: types::MapId`, the typed config, and `cache: &mut ProceduralMeshCache`.
- Spawn a parent entity with child mesh part entities, exactly as `spawn_creature`
  does — reusing `spawn_creature` internally by calling
  `ItemMeshDescriptor::to_creature_definition` and then
  `creature_spawning::spawn_creature` so no mesh-generation code is duplicated.
- Apply a ground-lying rotation: the item rests flat on the XZ plane
  (`rotation_x = -90°` for upright items; swords/daggers lie on their side).
- Return the parent `Entity`.

Add item mesh slots to `ProceduralMeshCache`:

```
item_sword:    Option<Handle<Mesh>>
item_dagger:   Option<Handle<Mesh>>
item_blunt:    Option<Handle<Mesh>>
item_staff:    Option<Handle<Mesh>>
item_bow:      Option<Handle<Mesh>>
item_armor:    Option<Handle<Mesh>>
item_shield:   Option<Handle<Mesh>>
item_potion:   Option<Handle<Mesh>>
item_scroll:   Option<Handle<Mesh>>
item_ring:     Option<Handle<Mesh>>
item_ammo:     Option<Handle<Mesh>>
item_quest:    Option<Handle<Mesh>>
```

Add `get_or_create_item_mesh(category, config, meshes)` following the existing
`get_or_create_furniture_mesh` pattern.

#### 2.2 Add `DroppedItem` ECS Component

Create `src/game/components/dropped_item.rs`:

```
/// Marker component for an item entity lying on the ground.
pub struct DroppedItem {
    pub item_id: ItemId,
    pub map_id:  MapId,
    pub tile_x:  i32,
    pub tile_y:  i32,
    pub charges: u16,
}
```

Register in `src/game/components/mod.rs` with `pub mod dropped_item;` and
re-export `DroppedItem`.

#### 2.3 Add `DroppedItem` Variant to `MapEvent`

Add to the `MapEvent` enum in `src/domain/world/types.rs`:

```
DroppedItem {
    #[serde(default)]
    name: String,
    item_id: ItemId,
    #[serde(default)]
    charges: u16,
}
```

Using `#[serde(default)]` on `charges` preserves backward compatibility.
Existing RON map files without `DroppedItem` events are unaffected.

#### 2.4 Add `ItemDroppedEvent` and `ItemPickedUpEvent` Bevy Events

Create `src/game/systems/item_world_events.rs`:

```
pub struct ItemDroppedEvent {
    pub item_id:   ItemId,
    pub charges:   u16,
    pub map_id:    MapId,
    pub tile_x:    i32,
    pub tile_y:    i32,
}

pub struct ItemPickedUpEvent {
    pub item_id:   ItemId,
    pub map_id:    MapId,
    pub tile_x:    i32,
    pub tile_y:    i32,
}
```

Register both as Bevy `Event` via `#[derive(Event)]`. Register in the game
plugin with `app.add_event::<ItemDroppedEvent>().add_event::<ItemPickedUpEvent>()`.

#### 2.5 Item World Spawn / Despawn Systems

Still in `src/game/systems/item_world_events.rs`, implement:

**`spawn_dropped_item_system`**

- Reads `EventReader<ItemDroppedEvent>`.
- Looks up `Item` from `GameContent`.
- Calls `ItemMeshDescriptor::from_item`, then
  `item_mesh_descriptor.to_creature_definition()`, then
  `spawn_creature(...)` with ground-lying transform (Y = 0.05 to sit just above
  floor, slight random rotation around Y axis for variety).
- Inserts `DroppedItem` component onto the spawned parent entity.
- Stores the entity in a new `DroppedItemRegistry` resource (see §2.6).

**`despawn_picked_up_item_system`**

- Reads `EventReader<ItemPickedUpEvent>`.
- Looks up the entity in `DroppedItemRegistry` by `(map_id, tile_x, tile_y, item_id)`.
- Calls `commands.entity(entity).despawn_recursive()`.
- Removes entry from `DroppedItemRegistry`.

**`load_map_dropped_items_system`**

- Runs on map load (after `spawn_map_system`).
- Iterates `MapEvent::DroppedItem` events on the current map.
- Fires `ItemDroppedEvent` for each, so static map-authored dropped items get
  the same spawn path as runtime drops.

#### 2.6 `DroppedItemRegistry` Resource

Add to `src/game/resources/mod.rs`:

```
pub struct DroppedItemRegistry {
    /// Maps (map_id, tile_x, tile_y, item_id) → Entity
    pub entries: HashMap<(MapId, i32, i32, ItemId), Entity>,
}
```

Initialized as empty in `App::init_resource::<DroppedItemRegistry>()`.

#### 2.7 Inventory Drop Integration

In the existing inventory system (`src/game/systems/inventory_ui.rs` or its
sibling), wherever a drop action is confirmed, send `ItemDroppedEvent` with
the current map ID and party tile position. Wherever a pick-up is confirmed,
send `ItemPickedUpEvent`. No new UI is needed in this phase.

#### 2.8 Testing Requirements

File: `src/game/systems/item_world_events.rs` (`mod tests`)

- `test_dropped_item_component_fields` — `DroppedItem` stores `item_id`,
  `map_id`, `tile_x`, `tile_y`, `charges` correctly.
- `test_item_dropped_event_creation` — event struct populates all fields.
- `test_item_picked_up_event_creation` — event struct populates all fields.
- `test_dropped_item_registry_default_empty` — `DroppedItemRegistry::default()`
  has no entries.
- `test_registry_insert_and_lookup` — insert entry, look up by key, returns
  the expected entity.
- `test_registry_remove_on_pickup` — after remove, key is absent.

File: `src/game/systems/procedural_meshes.rs` (extend `mod tests`)

- `test_sword_config_defaults` — `SwordConfig::default()` has positive
  `blade_length`.
- `test_dagger_config_defaults` — `DaggerConfig::default()` has shorter
  `blade_length` than sword default.
- `test_potion_config_defaults` — `PotionConfig::default()` produces non-zero
  colors.
- `test_cache_item_slots_default_none` — all new item cache slots are `None`
  at construction.
- `test_scroll_config_defaults` — `ScrollConfig::default()` has valid color.

File: `src/game/components/dropped_item.rs` (`mod tests`)

- `test_dropped_item_clone` — `DroppedItem` is `Clone`.
- `test_dropped_item_debug` — `DroppedItem` is `Debug`.

#### 2.9 Deliverables

- [ ] Item generator functions added to `src/game/systems/procedural_meshes.rs`
- [ ] Config structs for each item category added alongside generators
- [ ] Item mesh cache slots added to `ProceduralMeshCache`
- [ ] `src/game/components/dropped_item.rs` with `DroppedItem` component
- [ ] `src/game/components/mod.rs` updated
- [ ] `MapEvent::DroppedItem` variant in `src/domain/world/types.rs`
- [ ] `src/game/systems/item_world_events.rs` with events, systems, registry
- [ ] `DroppedItemRegistry` in `src/game/resources/mod.rs`
- [ ] Inventory drop / pick-up wired to new events
- [ ] `load_map_dropped_items_system` bridging static map events to spawn path
- [ ] Systems registered in the appropriate Bevy plugin
- [ ] All tests passing

#### 2.10 Success Criteria

- Dropping an item from inventory causes a 3-D mesh to appear on the party's
  current tile (verified manually).
- Picking up an item removes its mesh from the world (verified manually).
- A map RON file with a `DroppedItem` event causes the item mesh to appear on
  load.
- `cargo nextest run --all-features` passes 100 %.

---

### Phase 3: Item Mesh RON Asset Files

#### 3.1 Asset Directory Structure

Create the following directory and files:

```
campaigns/tutorial/assets/items/
    weapons/
        sword.ron
        dagger.ron
        short_sword.ron
        long_sword.ron
        great_sword.ron
        club.ron
        staff.ron
        bow.ron
    armor/
        leather_armor.ron
        chain_mail.ron
        plate_mail.ron
        shield.ron
        helmet.ron
        boots.ron
    consumables/
        health_potion.ron
        mana_potion.ron
        cure_potion.ron
        attribute_potion.ron
    accessories/
        ring.ron
        amulet.ron
        belt.ron
        cloak.ron
    ammo/
        arrow.ron
        bolt.ron
        stone.ron
    quest/
        quest_scroll.ron
        key_item.ron
```

Each `.ron` file is a `CreatureDefinition` that was produced by calling
`ItemMeshDescriptor::to_creature_definition` (via the Python generator, see
§3.2), then hand-tweaked for quality. IDs start at 2000 to avoid collision with
creature IDs which start at 1.

#### 3.2 Python Generator Script

Create `examples/generate_item_meshes.py` following the pattern of the existing
creature generator scripts in `examples/`. The script:

- Imports vertex/index generation helpers for box, cylinder, sphere, and torus
  primitives (extracted or copied from existing generator scripts).
- Iterates a manifest of item types with shape parameters.
- Outputs each item's mesh as a valid `CreatureDefinition` RON file into
  `campaigns/tutorial/assets/items/`.
- Accepts `--output-dir` so it can target `data/test_campaign/assets/items/`
  for test fixtures.

The script is a developer convenience tool, not a build step; all generated
`.ron` files are committed to the repository.

#### 3.3 Item Creature Registry

Add an `items/` entry to the campaign's creature registry (or create a
parallel item registry). The simplest approach: add a new
`item_mesh_registry.ron` file in `campaigns/tutorial/data/` that lists
`CreatureReference` entries pointing at the item mesh files. Extend
`CampaignLoader` to load this file into a new `item_meshes` field on
`CreatureDatabase` (or a separate `ItemMeshDatabase` wrapping
`CreatureDatabase`). The load path in `src/sdk/campaign_loader.rs` should look
for `data/item_mesh_registry.ron` in the campaign directory, falling back to
nothing if absent (opt-in per campaign).

#### 3.4 Override Linking in `ItemDatabase`

Add `pub fn link_mesh_overrides(&mut self, registry: &ItemMeshDatabase)` to
`ItemDatabase`. For each item with a non-`None` `mesh_descriptor_override`, it
validates the referenced creature ID exists in the registry. This runs during
campaign validation.

#### 3.5 Testing Requirements

File: `data/test_campaign/assets/items/` — create a minimal set of test
fixtures used by unit tests:

```
data/test_campaign/assets/items/
    sword.ron          (CreatureDefinition id: 2001)
    potion.ron         (CreatureDefinition id: 2002)
```

File: `src/sdk/campaign_loader.rs` (extend existing integration tests)

- `test_campaign_loader_loads_item_mesh_registry` — loads `data/test_campaign`
  and asserts `item_meshes` database contains at least 2 entries.
- `test_item_mesh_registry_missing_is_ok` — a campaign without
  `item_mesh_registry.ron` loads without error.

#### 3.6 Deliverables

- [ ] `campaigns/tutorial/assets/items/` directory with all RON files
- [ ] `examples/generate_item_meshes.py` script
- [ ] `campaigns/tutorial/data/item_mesh_registry.ron`
- [ ] `CampaignLoader` updated to load `item_mesh_registry.ron`
- [ ] `ItemMeshDatabase` type (thin wrapper or type alias for `CreatureDatabase`)
- [ ] `data/test_campaign/assets/items/` with minimal test fixtures
- [ ] `data/test_campaign/data/item_mesh_registry.ron` referencing them
- [ ] Integration tests in `src/sdk/campaign_loader.rs`

#### 3.7 Success Criteria

- All item RON files load without RON parse errors.
- `ItemMeshDatabase` validates all entries without error.
- Generated meshes visually distinguish weapon types at a glance (manual
  verification in-game).

---

### Phase 4: Visual Quality and Variation

#### 4.1 Per-Item Color Variation

Extend `ItemMeshDescriptor::from_item` to derive accent colors from bonus
attributes:

| `BonusAttribute`    | Accent color   |
| ------------------- | -------------- |
| `ResistFire`        | Orange / amber |
| `ResistCold`        | Icy blue       |
| `ResistElectricity` | Yellow         |
| `ResistPoison`      | Acid green     |
| `ResistMagic`       | Purple         |
| `Might`             | Warm red       |
| `AC` / `HP`         | Teal           |
| `SP` / `Intellect`  | Deep blue      |

Metallic quality (from `is_magical`) sets `MaterialDefinition::metallic > 0.5`
and `roughness < 0.3` for a shiny appearance. Non-magical items use
`metallic: 0.0, roughness: 0.8` (matte).

#### 4.2 Slight Randomised Drop Rotation

In `spawn_dropped_item_system`, add a deterministic random Y-axis rotation
derived from `(map_id as u64 ^ (tile_x as u64 * 31) ^ (tile_y as u64 * 17)) % 360`
so items dropped on different tiles appear at varied orientations, improving
visual variety without non-determinism.

#### 4.3 Charge-Level Visual Indicator

For items with charges (`Item::has_charges()`), add a small emissive sphere
"gem" child mesh to the `CreatureDefinition` output. Its color transitions from
full-charge gold → half-charge white → depleted grey, computed from
`InventorySlot::charges / Item::max_charges`. This requires passing `charges`
into `to_creature_definition`; add an optional `charges_fraction: Option<f32>`
parameter.

#### 4.4 Ground Shadow Quad

Each item mesh gains a flat shadow quad as its first child part: a dark
semi-transparent quad (`AlphaMode::Blend`, alpha 0.3, color `[0.0, 0.0, 0.0, 0.3]`)
placed at Y = 0.001 centred under the item. This improves visual grounding on
bright tile surfaces. The quad uses the item's approximate bounding XZ footprint
scaled by 1.2.

#### 4.5 LOD Support

For items with a primary mesh exceeding 200 triangles, add a LOD1 simplified
mesh at distance 8.0 and a LOD2 billboard-style quad at distance 20.0. Follow
the pattern already established in `MeshDefinition::lod_levels` and
`MeshDefinition::lod_distances`. Items under 200 triangles use no LOD (they are
already simple enough).

#### 4.6 Testing Requirements

File: `src/domain/visual/item_mesh.rs` (extend `mod tests`)

- `test_fire_resist_item_accent_orange` — item with `ResistFire` bonus gets
  orange accent.
- `test_magical_item_metallic_material` — `is_magical()` item produces
  `metallic > 0.5`.
- `test_non_magical_item_matte_material` — non-magical item produces
  `metallic: 0.0`.
- `test_charge_fraction_full_color_gold` — `charges_fraction: Some(1.0)` yields
  gold-tinted gem.
- `test_charge_fraction_empty_color_grey` — `charges_fraction: Some(0.0)` yields
  grey gem.
- `test_shadow_quad_present_and_transparent` — first mesh part has alpha < 0.5.
- `test_lod_added_for_complex_mesh` — mesh with > 200 triangles gets
  `lod_levels.is_some()`.

#### 4.7 Deliverables

- [ ] Accent color derivation from `BonusAttribute` in `ItemMeshDescriptor::from_item`
- [ ] Metallic / roughness material derivation from `is_magical()`
- [ ] Deterministic Y-rotation in `spawn_dropped_item_system`
- [ ] Charge gem child mesh with color gradient
- [ ] Ground shadow quad child mesh
- [ ] LOD levels for complex item meshes
- [ ] Unit tests covering all new visual rules

#### 4.8 Success Criteria

- Magical items visibly glow / gleam compared to mundane counterparts.
- A sword and a dagger are clearly distinguishable on the ground.
- A red potion and blue potion have clearly different colors.
- Fully-charged wand has gold gem; depleted wand has grey gem.
- No noticeable frame-rate drop when 10+ items are visible simultaneously.

---

### Phase 5: Campaign Builder SDK Integration

Brings the Item Mesh workflow in the Campaign Builder to parity with the
Creature Builder (`creatures_editor.rs`). The Creature Builder is the
reference implementation; every major capability it provides must be matched
for items.

**Creature Builder capabilities to replicate:**

| Creature Builder feature                                | Item Mesh Builder equivalent                                      |
| ------------------------------------------------------- | ----------------------------------------------------------------- |
| Registry list mode with search, filter, sort            | Item Mesh Registry list with search and `ItemMeshCategory` filter |
| Two-column `TwoColumnLayout` list + detail              | Same layout in item mesh registry                                 |
| Edit mode: mesh list panel + properties panel + preview | Edit mode: mesh override panel + properties + preview             |
| Live embedded preview renderer (`PreviewRenderer`)      | Live preview via `ItemMeshDescriptor::to_creature_definition()`   |
| Undo / redo stack (`CreatureUndoRedo`)                  | Undo / redo for override edits                                    |
| Save-As dialog (writes new `.ron` to `assets/items/`)   | Save-As dialog for item mesh RON files                            |
| Register existing asset dialog                          | Register existing item mesh RON dialog                            |
| Validation panel with errors / warnings / info          | Validation panel for mesh descriptors                             |
| Workflow state machine (`CreaturesWorkflow`)            | Workflow state machine for item mesh editor                       |
| Keyboard shortcuts (`ShortcutManager`)                  | Same keyboard shortcuts                                           |
| Context menu (`ContextMenuManager`)                     | Context menu on registry rows                                     |
| Primitive replacement dialog                            | Not applicable (item meshes are fully procedural)                 |
| `mode_indicator()` / `breadcrumb_string()` for toolbar  | Same helpers                                                      |
| `has_unsaved_changes()`, `can_undo()`, `can_redo()`     | Same helpers                                                      |

---

#### 5.1 New State Struct: `ItemMeshEditorState`

Add `sdk/campaign_builder/src/item_mesh_editor.rs` (new file). Do **not**
extend `ItemsEditorState` — keep item mesh editing as a separate tab to match
the separation between the Items tab and the Creatures tab.

```rust
pub struct ItemMeshEditorState {
    pub mode: ItemMeshEditorMode,
    pub search_query: String,
    pub category_filter: Option<ItemMeshCategory>,
    pub registry_sort_by: ItemMeshRegistrySortBy,
    pub selected_entry: Option<usize>,

    // Edit mode state
    pub edit_buffer: Option<ItemMeshDescriptor>,
    pub override_enabled: bool,
    pub preview_dirty: bool,
    pub preview_error: Option<String>,

    // Undo / redo
    pub undo_redo: ItemMeshUndoRedo,

    // Save-As dialog
    pub show_save_as_dialog: bool,
    pub save_as_path_buffer: String,

    // Register-asset dialog
    pub show_register_asset_dialog: bool,
    pub register_asset_path_buffer: String,
    pub register_asset_error: Option<String>,
    pub available_item_assets: Vec<String>,
    pub last_campaign_dir: Option<PathBuf>,

    // Validation
    pub show_validation_panel: bool,
    pub validation_errors: Vec<String>,
    pub validation_warnings: Vec<String>,

    // Registry delete confirmation
    pub registry_delete_confirm_pending: bool,

    // Import / export
    pub show_import_dialog: bool,
    pub import_export_buffer: String,

    // Preview renderer (same `PreviewRenderer` type used by creatures_editor)
    preview_renderer: Option<PreviewRenderer>,

    // Workflow
    pub workflow: ItemMeshWorkflow,

    // Shortcuts and context menu
    pub shortcut_manager: ShortcutManager,
    pub context_menu_manager: ContextMenuManager,
}
```

`ItemMeshEditorMode`:

```rust
pub enum ItemMeshEditorMode {
    Registry,  // list of all registered item mesh RON files
    Edit,      // editing a single ItemMeshDescriptor
}
```

`ItemMeshRegistrySortBy`:

```rust
pub enum ItemMeshRegistrySortBy {
    Id,
    Name,
    Category,
}
```

Follow all `sdk/AGENTS.md` egui ID rules: every loop uses `push_id`, every
`ScrollArea` has `id_salt`, every `ComboBox` uses `from_id_salt`.

---

#### 5.2 Undo / Redo for Item Mesh Edits

Add `sdk/campaign_builder/src/item_mesh_undo_redo.rs` (new file), modelled
directly on `creature_undo_redo.rs`.

```rust
pub struct ItemMeshUndoRedo {
    undo_stack: Vec<ItemMeshEditAction>,
    redo_stack: Vec<ItemMeshEditAction>,
}

pub enum ItemMeshEditAction {
    SetPrimaryColor { old: [f32; 4], new: [f32; 4] },
    SetAccentColor  { old: [f32; 4], new: [f32; 4] },
    SetScale        { old: f32,      new: f32        },
    SetEmissive     { old: bool,     new: bool       },
    SetOverrideEnabled { old: bool,  new: bool       },
    ReplaceDescriptor { old: ItemMeshDescriptor, new: ItemMeshDescriptor },
}
```

Implement `push`, `undo`, `redo`, `can_undo`, `can_redo`, and `clear`.

---

#### 5.3 Workflow State Machine

Add `sdk/campaign_builder/src/item_mesh_workflow.rs` (new file), modelled on
`creatures_workflow.rs`. The workflow tracks:

- `RegistryMode` — browsing the list of registered item mesh assets.
- `EditMode { file_name: String }` — editing a specific asset file.

Expose `mode_indicator() -> String`, `breadcrumb_string() -> String`,
`enter_edit(file_name)`, and `return_to_registry()`.

---

#### 5.4 Registry Mode UI

In `ItemMeshEditorState::show_registry_mode()`, implement a two-column layout
(`TwoColumnLayout::new("item_mesh_registry")`) with:

**Left column — registry list:**

- Search box (`search_query`).
- Category filter `ComboBox::from_id_salt("item_mesh_category_filter")` with
  `None` ("All Categories") plus each `ItemMeshCategory` variant.
- Sort selector `ComboBox::from_id_salt("item_mesh_sort_by")`.
- Scrollable list of registered entries; each row uses `ui.push_id(idx, …)`.
  Selected row highlighted. Double-click opens Edit mode.
- Category counts badge (mirrors `count_by_category` in creatures editor).
- Toolbar row: **➕ New**, **📁 Register Asset**, **🔄 Reload**.

**Right column — registry preview panel:**

- Shows selected entry name, category, and file path.
- Small read-only live preview (calls `sync_preview_renderer_from_descriptor`).
- **✏️ Edit**, **📋 Duplicate**, **🗑 Delete** (with confirm flag matching
  `registry_delete_confirm_pending` pattern), **📤 Export RON** buttons.
- Context menu on each row with the same four actions.

---

#### 5.5 Edit Mode UI

In `ItemMeshEditorState::show_edit_mode()`, implement a three-panel layout:

**Top bar:**

- Breadcrumb label from `breadcrumb_string()`.
- Mode indicator from `mode_indicator()`.
- Toolbar: **💾 Save**, **💾 Save As**, **↩ Revert**, **✅ Validate**,
  **⬅ Back to Registry**.
- Undo (`Ctrl+Z`) and Redo (`Ctrl+Shift+Z`) buttons, enabled by `can_undo()`
  / `can_redo()`.

**Left panel — mesh override properties:**

- Toggle `override_enabled` checkbox. When disabled, all controls below are
  greyed out (shown but `ui.add_enabled(false, …)`).
- `ItemMeshCategory` display (read-only; derived from the item type).
- Primary color RGBA picker (label + four `Slider` widgets, or `egui::color_picker`).
- Accent color RGBA picker.
- Scale `Slider` (range 0.25–4.0, step 0.05).
- Emissive checkbox.
- **↺ Reset to Defaults** button — sets `override_enabled = false` and
  replaces descriptor with `ItemMeshDescriptor::from_item(&current_item)`.

Every state mutation must:

1. Push an `ItemMeshEditAction` onto the undo stack.
2. Set `preview_dirty = true`.
3. Call `ui.ctx().request_repaint()`.

**Centre panel — live preview:**

- Same `PreviewRenderer` widget used by the creature editor, fed via
  `ItemMeshDescriptor::to_creature_definition()`.
- "Regenerate Preview" button (clears `preview_dirty`, calls
  `sync_preview_renderer_from_descriptor`).
- Shows `ItemMeshCategory` label and triangle-count statistic.
- Shows `preview_error` as a red label if set.
- Camera distance slider (`camera_distance`, range 1.0–20.0).

---

#### 5.6 Inline Validation Panel

In `show_edit_mode()`, below the properties panel, add a collapsible
**✅ Validation** section that shows `validation_errors` (red) and
`validation_warnings` (yellow) populated by calling
`ItemMeshDatabase::validate_descriptor(&descriptor)` whenever **Validate** is
clicked or the descriptor changes (throttled — not every frame).

Mirrors `refresh_validation_state` / `validate_selected_mesh` in
`creatures_editor.rs`.

---

#### 5.7 Save-As Dialog

Add `show_save_as_dialog_window()` modelled on the creature editor's version:

- `egui::Window::new("Save Item Mesh As")` with unique title (Rule 8).
- Path text field pre-populated by `default_save_as_path()` which returns
  `assets/items/<slugified_name>.ron`.
- Validates that the path is under `assets/items/`.
- On confirm: serialises the `ItemMeshDescriptor` to RON, writes the file,
  appends a new entry to `item_mesh_registry.ron`, registers the entry in the
  in-memory registry list, clears `has_unsaved_changes`.

---

#### 5.8 Register Existing Asset Dialog

Add `show_register_asset_dialog_window()` modelled on the creature editor's:

- Lists `.ron` files found in `<campaign_dir>/assets/items/` (refreshed when
  `last_campaign_dir` changes, cached in `available_item_assets`).
- Path autocomplete from the cached list.
- **Validate** button: deserialises the RON, checks for duplicate IDs, sets
  `register_asset_error` on failure.
- **Register** button (enabled only after successful validation): appends the
  entry to the in-memory registry and writes `item_mesh_registry.ron`.
- **Cancel** — closes dialog without modifying registry.

---

#### 5.9 Extend `ItemsEditorState` with Mesh Preview Pane

The existing `items_editor.rs` `show_form()` gets a new **"Ground Mesh
Preview"** collapsible group at the bottom of the edit form, below the Tags
section:

```rust
ui.collapsing("🧊 Ground Mesh Preview", |ui| {
    let descriptor = ItemMeshDescriptor::from_item(&self.edit_buffer);
    ui.label(format!("Category: {:?}", descriptor.category));
    // Embed the preview renderer here (same approach as creatures_editor preview fallback)
    if ui.button("🔄 Refresh Preview").clicked() {
        // trigger preview sync
    }
    // Static label fallback when renderer unavailable:
    ui.label(format!("Shape: {:?}", descriptor.shape));
    if let Some(ovr) = &descriptor.override_params {
        ui.label(format!("Scale override: {:.2}×", ovr.scale));
        ui.label(format!("Emissive: {}", ovr.emissive));
    } else {
        ui.label("No mesh override (auto-generated from item type)");
    }
    if ui.button("✏️ Open in Item Mesh Editor").clicked() {
        // sets a cross-tab navigation signal to open item_mesh_editor for this item
    }
});
```

The "Open in Item Mesh Editor" button sets a `requested_open_item_mesh:
Option<ItemId>` field on `ItemsEditorState` that the parent tab dispatcher
reads to switch tabs — identical to the `requested_open_npc` pattern used in
the maps editor.

---

#### 5.10 Wire `ItemMeshEditorState` into the Campaign Builder Tab Bar

In `sdk/campaign_builder/src/lib.rs` (or wherever tabs are registered),
add an **"Item Meshes"** tab alongside the existing **"Items"** and
**"Creatures"** tabs. The tab:

- Holds an `ItemMeshEditorState` as part of the builder app state.
- Receives the current `item_mesh_registry` (a `Vec<ItemMeshEntry>`) and
  `campaign_dir` from the app state.
- On save, writes `item_mesh_registry.ron` to the campaign directory.
- Cross-tab navigation: when `items_editor.requested_open_item_mesh` is
  `Some(id)`, switch to the Item Meshes tab and call
  `item_mesh_editor.open_for_editing(id)`.

---

#### 5.11 Keyboard Shortcuts

Register the same shortcuts as the creature editor, scoped to the Item Mesh
Editor tab:

| Shortcut                  | Action                            |
| ------------------------- | --------------------------------- |
| `Ctrl+Z`                  | Undo                              |
| `Ctrl+Shift+Z` / `Ctrl+Y` | Redo                              |
| `Ctrl+S`                  | Save                              |
| `Ctrl+Shift+S`            | Save As                           |
| `Escape`                  | Back to Registry (from Edit mode) |

Use the existing `ShortcutManager` type from `keyboard_shortcuts.rs`.

---

#### 5.12 Testing Requirements

Follow all SDK testing rules from `sdk/AGENTS.md`.

**`ItemMeshUndoRedo` unit tests:**

- `test_item_mesh_undo_redo_push_and_undo` — push a `SetScale` action; undo;
  assert `can_undo() == false`, `can_redo() == true`.
- `test_item_mesh_undo_redo_redo` — push, undo, redo; assert `can_redo() == false`.
- `test_item_mesh_undo_redo_clear` — push two actions, clear; assert both
  stacks empty.

**`ItemMeshEditorState` unit tests:**

- `test_item_mesh_editor_state_default` — `ItemMeshEditorState::default()` is
  in `Registry` mode; `selected_entry` is `None`; `override_enabled` is `false`.
- `test_item_mesh_editor_mode_indicator_registry` — `mode_indicator()` returns
  `"Registry Mode"` when in `ItemMeshEditorMode::Registry`.
- `test_item_mesh_editor_mode_indicator_edit` — `mode_indicator()` returns
  `"Asset Editor: sword.ron"` when in Edit mode with `file_name = "sword.ron"`.
- `test_item_mesh_editor_breadcrumb_registry` — `breadcrumb_string()` returns
  `"Item Meshes"` in Registry mode.
- `test_item_mesh_editor_breadcrumb_edit` — `breadcrumb_string()` returns
  `"Item Meshes > sword.ron"` in Edit mode.
- `test_item_mesh_editor_has_unsaved_changes_false_by_default` — fresh state
  returns `false`.
- `test_item_mesh_editor_has_unsaved_changes_true_after_edit` — mutating
  `edit_buffer` sets `has_unsaved_changes() == true`.
- `test_item_mesh_editor_can_undo_false_by_default` — `can_undo() == false`
  on a fresh state.
- `test_item_mesh_editor_can_redo_false_by_default` — `can_redo() == false`.
- `test_item_mesh_editor_back_to_registry_clears_edit_state` — call
  `back_to_registry()`; assert mode is `Registry`, `edit_buffer` is `None`,
  `preview_dirty` is `false`.

**Registry mode tests:**

- `test_available_item_assets_empty_when_no_assets_dir` — given a campaign dir
  with no `assets/items/` subdirectory, `available_item_assets` is empty.
- `test_available_item_assets_populated_from_campaign_dir` — given a temp dir
  with two `.ron` files in `assets/items/`, the list contains both file names.
- `test_available_item_assets_not_refreshed_when_dir_unchanged` — calling
  `refresh_available_assets` twice with the same dir does not re-read the
  filesystem on the second call (cache hit).
- `test_available_item_assets_refreshed_when_dir_changes` — changing
  `last_campaign_dir` triggers a refresh.
- `test_register_asset_validate_duplicate_id_sets_error` — registering an
  asset whose parsed ID already exists in the registry sets
  `register_asset_error`.
- `test_register_asset_cancel_does_not_modify_registry` — cancel after
  populating the path buffer; assert registry unchanged.
- `test_register_asset_success_appends_entry` — validate + register a valid
  RON; assert registry length increased by one.

**Edit mode / save-as tests:**

- `test_perform_save_as_with_path_appends_new_entry` — save-as to a valid
  `assets/items/` path appends an entry to the registry and writes the file.
- `test_perform_save_as_requires_campaign_directory` — save-as with no campaign
  dir set returns an error message.
- `test_perform_save_as_rejects_non_item_asset_paths` — a path outside
  `assets/items/` returns an error.
- `test_revert_edit_buffer_restores_original` — mutate `edit_buffer`, call
  `revert_edit_buffer_from_registry()`; assert buffer matches the original
  registry entry.
- `test_revert_edit_buffer_errors_in_registry_mode` — calling revert in
  Registry mode returns an appropriate error.

**Validation tests:**

- `test_validate_descriptor_reports_invalid_scale` — a descriptor with
  `override_params.scale = 0.0` fails validation with a message containing
  `"scale"`.
- `test_validate_descriptor_passes_for_default_descriptor` — an
  auto-generated `ItemMeshDescriptor::from_item(&default_item())` passes
  validation with no errors.

**Cross-tab navigation test:**

- `test_items_editor_requested_open_item_mesh_set_on_button` — constructing
  an `ItemsEditorState` with a selected item and calling the open-in-mesh-editor
  action sets `requested_open_item_mesh == Some(item_id)`.

---

#### 5.13 Deliverables

- [ ] `sdk/campaign_builder/src/item_mesh_editor.rs` — `ItemMeshEditorState`,
      `ItemMeshEditorMode`, `ItemMeshRegistrySortBy`
- [ ] `sdk/campaign_builder/src/item_mesh_undo_redo.rs` — `ItemMeshUndoRedo`,
      `ItemMeshEditAction`
- [ ] `sdk/campaign_builder/src/item_mesh_workflow.rs` — `ItemMeshWorkflow`
      with `mode_indicator()`, `breadcrumb_string()`, `enter_edit()`,
      `return_to_registry()`
- [ ] Registry mode: two-column list + preview panel with search, filter, sort
- [ ] Registry mode: **New**, **Register Asset**, **Duplicate**, **Delete**
      (with confirm), **Export RON** actions
- [ ] Register-existing-asset dialog with filesystem scan, validation, and
      duplicate-ID guard
- [ ] Edit mode: override properties panel + live preview + validation panel
- [ ] Edit mode: undo / redo wired to all property mutations
- [ ] Save-As dialog writing to `assets/items/` and updating
      `item_mesh_registry.ron`
- [ ] Revert action restoring the edit buffer from the registry entry
- [ ] Keyboard shortcuts (`Ctrl+Z/Y/S`, `Escape`) via `ShortcutManager`
- [ ] Context menu on registry rows via `ContextMenuManager`
- [ ] **"Ground Mesh Preview"** collapsible in existing `items_editor.rs`
      `show_form()`, with "Open in Item Mesh Editor" cross-tab signal
- [ ] **"Item Meshes"** tab wired into the Campaign Builder tab bar
- [ ] `mode_indicator()`, `breadcrumb_string()`, `has_unsaved_changes()`,
      `can_undo()`, `can_redo()` public helpers
- [ ] All 28 new tests pass; all four quality gates pass with zero warnings
- [ ] SPDX headers on all three new `.rs` files

#### 5.14 Success Criteria

A campaign author can:

1. Open the Campaign Builder and navigate to the **Item Meshes** tab.
2. Browse registered item mesh RON files in the registry list, filter by
   `ItemMeshCategory`, and see a live preview of the selected entry.
3. Double-click an entry (or click **✏️ Edit**) to enter Edit mode.
4. Adjust primary/accent color, scale, and emissive flag; see the preview
   update immediately; undo and redo each change.
5. Click **💾 Save As**, confirm the path is under `assets/items/`, and verify
   the `.ron` file is written and the registry is updated.
6. Click **Register Asset**, browse to an existing `.ron` in `assets/items/`,
   validate it, and register it — duplicate IDs are caught and reported.
7. Navigate to the **Items** tab, edit any item, expand **"Ground Mesh
   Preview"**, and click **"Open in Item Mesh Editor"** to be taken directly to
   the mesh editor for that item.
8. All four `cargo` quality gates pass with zero warnings.

---

### Phase 6: Content — Full Item Mesh Coverage

#### 6.1 Complete Mesh Coverage for All Base Items

Run `examples/generate_item_meshes.py` against all items in `data/items.ron`
to ensure every item ID has either:

- An auto-generated mesh via `ItemMeshDescriptor::from_item` (the default), or
- An explicit entry in `item_mesh_registry.ron` pointing to a hand-crafted RON.

Audit any item that uses the fallback `QuestItem` category unexpectedly and
provide a hand-crafted mesh file.

#### 6.2 Visual Quality Pass

Walk through the tutorial campaign with all item types dropped on the ground.
For each category, verify:

| Category     | Quality check                                        |
| ------------ | ---------------------------------------------------- |
| Sword        | Blade clearly longer than dagger, crossguard visible |
| Dagger       | Compact, sits flat convincingly                      |
| Blunt (Club) | Boxy head distinct from sword shapes                 |
| Staff        | Tall cylinder + visible sphere tip                   |
| Bow          | Curved arc shape recognizable                        |
| Armor        | Plate silhouette different from leather              |
| Helmet       | Dome shape distinct from chest                       |
| Shield       | Hexagonal disc shape visible                         |
| Potion       | Rounded bottle with color-coded liquid               |
| Scroll       | Rolled paper cylinder shape                          |
| Ring         | Torus shape visible even at tile scale               |
| Arrow        | Thin shaft + fletching                               |
| Quest Item   | Unique glowing scroll shape                          |

#### 6.3 Tutorial Campaign Authored Drops

Add at least three `MapEvent::DroppedItem` events to tutorial map files to
demonstrate the feature in the shipped campaign — for example, a dropped sword
near the starting room, a potion by the first dungeon entrance, and a ring on
the floor of a treasure chamber.

#### 6.4 Testing Requirements

- `test_all_base_items_have_valid_mesh_descriptor` — iterate every item in
  `data/items.ron`, call `ItemMeshDescriptor::from_item`, call
  `to_creature_definition`, call `validate()`, assert `Ok`.
- `test_item_mesh_registry_tutorial_coverage` — load tutorial campaign, assert
  item mesh registry is non-empty.
- `test_dropped_item_event_in_map_ron` — load a tutorial map containing a
  `DroppedItem` event and assert the event parses without error.

#### 6.5 Deliverables

- [ ] All base items covered by a mesh descriptor (auto or hand-crafted)
- [ ] Visual quality pass completed; any unsatisfactory meshes hand-tuned
- [ ] At least three authored drops in tutorial campaign map RON files
- [ ] Full coverage tests passing

#### 6.6 Success Criteria

- Every item in `data/items.ron` produces a valid, visually distinct mesh.
- Tutorial campaign has demonstrable in-game dropped items.
- No item silently falls through to an invisible or degenerate mesh.

---

## Cross-Cutting Concerns

### SPDX Headers

All new `.rs` files must begin with:

```
// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0
```

### File Extension Rules

- Rust implementation files: `.rs` in `src/`
- Item mesh asset files: `.ron` in `campaigns/tutorial/assets/items/`
- Test fixture files: `.ron` in `data/test_campaign/`
- No `.json`, `.yaml`, or `.yml` for game data

### Test Data Rule

All tests that load campaign content MUST use `data/test_campaign`, never
`campaigns/tutorial`. Any new test fixtures must be added to
`data/test_campaign/assets/items/` and `data/test_campaign/data/`.

### Naming Conventions

New files follow the project lowercase underscore convention:

- `item_mesh.rs`, `dropped_item.rs`, `item_world_events.rs`
- `item_mesh_registry.ron`, `sword.ron`, `health_potion.ron`

### Performance Budget

- Item mesh generation (domain layer only, no Bevy) < 1 ms per item on a
  modern CPU.
- Spawning 20 items simultaneously < 16 ms total (one frame at 60 FPS).
- Each item mesh triangle count: ≤ 300 triangles for melee weapons,
  ≤ 200 for consumables/accessories, ≤ 150 for ammo.

### Error Handling

- `ItemMeshDescriptor::from_item` is infallible (always returns a valid
  descriptor, never panics — unknown subtypes fall back to a simple box).
- `to_creature_definition` is infallible.
- `spawn_dropped_item_system` logs a `warn!` and skips gracefully if the
  item ID is not found in `GameContent`.
- `despawn_picked_up_item_system` logs a `warn!` if the entity is not found in
  the registry (it may have already been cleaned up on map change).

### Map Change Cleanup

When a map change occurs (`MapChangeEvent`), all `DroppedItem` entities
belonging to the previous map must be despawned and their registry entries
removed. This should be handled in a `cleanup_dropped_items_on_map_change`
system that runs in response to `MapChangeEvent`, iterating the
`DroppedItemRegistry` and despawning all entities with a matching `map_id`.

### Backward Compatibility

All new fields on `Item`, `MapEvent`, and registry files use `#[serde(default)]`
so pre-existing RON files load without modification. The mesh system is
entirely additive — no existing system changes behavior.

---

## Implementation Order Summary

```
Phase 1 (Domain types)         → unblocks all other phases
Phase 2 (Engine spawn/despawn) → requires Phase 1
Phase 3 (RON assets)           → can overlap with Phase 2; requires Phase 1
Phase 4 (Visual quality)       → requires Phases 1–3
Phase 5 (SDK / editor)         → requires Phases 1–3; can overlap with Phase 4
Phase 6 (Full coverage)        → requires all prior phases
```

---

## Risk Mitigation

| Risk                                                            | Mitigation                                                                                        |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Item mesh silhouette too small at tile scale                    | Scale all items by 1.5× relative to creature scale; adjust per category in generator              |
| Z-fighting between item mesh and floor tile                     | Ground all items at Y = 0.05 (5 cm above floor)                                                   |
| Many simultaneous drops causes frame spike                      | `ProceduralMeshCache` caches base meshes; color variation via material only, not mesh duplication |
| `DroppedItemRegistry` accumulates stale entries on crash/reload | Registry is rebuilt from `MapEvent::DroppedItem` events on each map load                          |
| Item mesh RON files diverge from domain shapes                  | `validate_mesh_descriptors` in SDK validation catches mismatches at authoring time                |
| Torus (ring) too thin to see                                    | Minimum ring outer radius of 0.15 world units; increase segment count to 12 for the torus arc     |
