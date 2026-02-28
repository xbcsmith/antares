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

| Subsystem | Location | Status |
|---|---|---|
| `MeshDefinition` / `CreatureDefinition` domain types | `src/domain/visual/mod.rs` | ✅ Complete |
| `CreatureDatabase` | `src/domain/visual/creature_database.rs` | ✅ Complete |
| `mesh_definition_to_bevy` conversion | `src/game/systems/creature_meshes.rs` | ✅ Complete |
| `spawn_creature` hierarchical entity spawner | `src/game/systems/creature_spawning.rs` | ✅ Complete |
| `ProceduralMeshCache` | `src/game/systems/procedural_meshes.rs` | ✅ Complete |
| Furniture spawn pipeline (config structs + spawn fns) | `src/game/systems/procedural_meshes.rs` | ✅ Complete |
| `MapEvent::Furniture` → `spawn_furniture_with_rendering` | `src/game/systems/furniture_rendering.rs` | ✅ Complete |
| `ItemDatabase` with full `Item` / `ItemType` definitions | `src/domain/items/database.rs` | ✅ Complete |
| `MapEvent` enum | `src/domain/world/types.rs` | ✅ Complete |
| `GameContent` resource with loaded databases | `src/application/resources.rs` | ✅ Complete |

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

| Generator | Config struct | Produced shape |
|---|---|---|
| `spawn_sword_mesh` | `SwordConfig { blade_length, blade_width, has_crossguard, color }` | Elongated box blade + crossguard quad + handle box |
| `spawn_dagger_mesh` | `DaggerConfig { blade_length, color }` | Short blade + small handle |
| `spawn_blunt_mesh` | `BluntConfig { head_radius, handle_length, color }` | Cylindrical head + handle |
| `spawn_staff_mesh` | `StaffConfig { length, orb_radius, color }` | Long thin cylinder + sphere tip |
| `spawn_bow_mesh` | `BowConfig { arc_height, color }` | Curved arc of quads + string line |
| `spawn_armor_mesh` | `ArmorMeshConfig { width, height, color, is_helmet }` | Layered box chest piece or dome |
| `spawn_shield_mesh` | `ShieldConfig { radius, color }` | Hexagonal polygon disc |
| `spawn_potion_mesh` | `PotionConfig { liquid_color, bottle_color }` | Tapered cylinder body + sphere stopper; liquid interior uses `AlphaMode::Blend` |
| `spawn_scroll_mesh` | `ScrollConfig { color }` | Rolled cylinder pair |
| `spawn_ring_mesh` | `RingMeshConfig { color }` | Torus approximated with arc of thin quads |
| `spawn_ammo_mesh` | `AmmoConfig { ammo_type, color }` | Arrow shaft + fletching; bolt variant; stone sphere |

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

| `BonusAttribute` | Accent color |
|---|---|
| `ResistFire` | Orange / amber |
| `ResistCold` | Icy blue |
| `ResistElectricity` | Yellow |
| `ResistPoison` | Acid green |
| `ResistMagic` | Purple |
| `Might` | Warm red |
| `AC` / `HP` | Teal |
| `SP` / `Intellect` | Deep blue |

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

#### 5.1 Item Mesh Preview in Item Editor

In `sdk/campaign_builder/` (egui-based), extend the existing item editor tab
to add a "3D Ground Preview" section. The preview uses the same embedded Bevy
renderer approach already established for creature preview. It:

- Creates an `ItemMeshDescriptor` from the currently edited `Item` on each
  change.
- Calls `to_creature_definition()` and feeds it to the in-editor preview
  renderer.
- Shows a "Regenerate Preview" button.
- Shows the `ItemMeshCategory` label so authors know which shape archetype was
  selected.

Follow the egui ID audit rules from `sdk/AGENTS.md`: every loop must use
`push_id`, every `ScrollArea` has `id_salt`, every `ComboBox` uses
`from_id_salt`.

#### 5.2 Item Mesh Override Editor

Add an expandable "Mesh Overrides" section to the item editor UI:

- Toggle enabling `mesh_descriptor_override`.
- Color pickers for `primary_color` and `accent_color` (RGBA).
- Slider for `scale` (0.25–4.0).
- Checkbox for `emissive`.
- "Reset to defaults" button that sets `mesh_descriptor_override: None`.

All state mutations call `request_repaint()`.

#### 5.3 Item Mesh Registry Editor

Add an "Item Meshes" editor tab to the campaign builder, listing registered
item mesh RON files from `item_mesh_registry.ron`. Functionality:

- View list of registered item mesh entries.
- Add / remove entries.
- Open the underlying RON file in the system editor.
- Validate all registered meshes (calls `ItemMeshDatabase::validate`).

#### 5.4 Testing Requirements

Follow SDK testing rules from `sdk/AGENTS.md`.

- `test_item_mesh_preview_panel_no_crash` — constructing the preview panel with
  a default `Item` does not panic.
- `test_item_mesh_override_toggle` — enabling override in UI populates the
  override struct; disabling clears it.
- `test_item_mesh_registry_editor_loads` — registry editor renders without
  panic when given an empty registry.

#### 5.5 Deliverables

- [ ] 3-D Ground Preview panel in item editor
- [ ] Mesh Override editor section in item editor
- [ ] Item Mesh Registry editor tab
- [ ] SDK tests passing

#### 5.6 Success Criteria

- Campaign authors can see a live preview of the item's ground mesh.
- Override color changes reflect immediately in the preview.
- `cargo clippy --all-targets --all-features -- -D warnings` passes with zero
  warnings for all SDK code.

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

| Category | Quality check |
|---|---|
| Sword | Blade clearly longer than dagger, crossguard visible |
| Dagger | Compact, sits flat convincingly |
| Blunt (Club) | Boxy head distinct from sword shapes |
| Staff | Tall cylinder + visible sphere tip |
| Bow | Curved arc shape recognizable |
| Armor | Plate silhouette different from leather |
| Helmet | Dome shape distinct from chest |
| Shield | Hexagonal disc shape visible |
| Potion | Rounded bottle with color-coded liquid |
| Scroll | Rolled paper cylinder shape |
| Ring | Torus shape visible even at tile scale |
| Arrow | Thin shaft + fletching |
| Quest Item | Unique glowing scroll shape |

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

| Risk | Mitigation |
|---|---|
| Item mesh silhouette too small at tile scale | Scale all items by 1.5× relative to creature scale; adjust per category in generator |
| Z-fighting between item mesh and floor tile | Ground all items at Y = 0.05 (5 cm above floor) |
| Many simultaneous drops causes frame spike | `ProceduralMeshCache` caches base meshes; color variation via material only, not mesh duplication |
| `DroppedItemRegistry` accumulates stale entries on crash/reload | Registry is rebuilt from `MapEvent::DroppedItem` events on each map load |
| Item mesh RON files diverge from domain shapes | `validate_mesh_descriptors` in SDK validation catches mismatches at authoring time |
| Torus (ring) too thin to see | Minimum ring outer radius of 0.15 world units; increase segment count to 12 for the torus arc |
