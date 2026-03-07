# Implementations

## Items Procedural Meshes — Phase 1: Domain Layer

**Plan**: [`items_procedural_meshes_implementation_plan.md`](items_procedural_meshes_implementation_plan.md)

### Overview

Phase 1 adds the domain-layer types that drive procedural 3-D world-mesh
generation for dropped items. When a player drops an item from inventory it
will (in later phases) spawn a procedural mesh on the tile; this phase
establishes the pure-Rust data layer that converts any `Item` definition into
a `CreatureDefinition` that the existing `spawn_creature` pipeline can render.

No Bevy dependency is introduced in Phase 1. All new code lives in
`src/domain/` and `src/sdk/`.

---

### Phase 1 Deliverables

**Files changed / created**:

- `src/domain/visual/item_mesh.rs` _(new)_
- `src/domain/visual/mod.rs` _(updated)_
- `src/domain/items/types.rs` _(updated)_
- `src/domain/items/database.rs` _(updated)_
- `src/sdk/validation.rs` _(updated)_
- `src/sdk/error_formatter.rs` _(updated)_

**Files with `mesh_descriptor_override: None` field additions** (backward-compatible):

- `src/domain/combat/item_usage.rs`
- `src/domain/items/equipment_validation.rs`
- `src/domain/transactions.rs`
- `src/game/systems/combat.rs`
- `src/game/systems/dialogue.rs`
- `src/sdk/templates.rs`
- `src/bin/item_editor.rs`
- `tests/cli_editor_tests.rs`
- `tests/merchant_transaction_integration_test.rs`

---

### What was built

#### `ItemMeshCategory` (`src/domain/visual/item_mesh.rs`)

An enum with 17 variants mapping every `ItemType` sub-classification to a
distinct mesh silhouette: `Sword`, `Dagger`, `Blunt`, `Staff`, `Bow`,
`BodyArmor`, `Helmet`, `Shield`, `Boots`, `Ring`, `Amulet`, `Belt`, `Cloak`,
`Potion`, `Scroll`, `Ammo`, `QuestItem`.

#### `ItemMeshDescriptor` (`src/domain/visual/item_mesh.rs`)

The full per-item visual specification: `category`, `blade_length`,
`primary_color`, `accent_color`, `emissive`, `emissive_color`, and `scale`.

`ItemMeshDescriptor::from_item(item: &Item) -> Self` is a **pure function**
that reads `item.item_type`, sub-type classification fields, `tags`, bonus
values, and charge data:

- `WeaponClassification::Simple` with `sides ≤ 4` → `Dagger`; otherwise →
  `Blunt`. `MartialMelee` → `Sword`. `MartialRanged` → `Bow`.
  `Blunt` → `Blunt`.
- Blade length = `(damage.sides × 0.08).clamp(0.25, 1.0)`. Dagger blade is
  multiplied by 0.7 (shorter).
- `two_handed` tag → scale multiplied by `1.45`.
- `ConsumableEffect::HealHp` → red; `RestoreSp` → blue;
  `CureCondition` → `Scroll` category (parchment color);
  `BoostAttribute` / `BoostResistance` → yellow.
- `item.is_magical()` → `emissive = true`, soft white glow.
- `item.is_cursed` → dark purple primary color, purple emissive (overrides
  magical glow — curse takes visual priority).
- Quest items always emit (magenta star mesh).

`ItemMeshDescriptor::to_creature_definition(&self) -> CreatureDefinition`
converts the descriptor into a single-mesh `CreatureDefinition` on the XZ
plane (item lying flat on the ground). The returned definition always passes
`CreatureDefinition::validate()`.

Each mesh category has a dedicated geometry builder that produces a flat
polygon on the XZ plane (Y = 0). All polygon fans use a dedicated centre
vertex (never vertex 0 as the hub) to avoid degenerate triangles.

#### `ItemMeshDescriptorOverride` (`src/domain/visual/item_mesh.rs`)

A `#[serde(default)]`-annotated struct with four optional fields:
`primary_color`, `accent_color`, `scale`, `emissive`. Campaign authors can
embed it in a RON item file to customise the visual without touching gameplay
data. An all-`None` override is identical to no override at all.

#### `Item::mesh_descriptor_override` (`src/domain/items/types.rs`)

Added `#[serde(default)] pub mesh_descriptor_override:
Option<ItemMeshDescriptorOverride>` to the `Item` struct. All existing RON
item files remain valid without modification because `#[serde(default)]`
deserialises the field as `None` when absent.

#### `ItemDatabase::validate_mesh_descriptors` (`src/domain/items/database.rs`)

A new method that calls `ItemMeshDescriptor::from_item` for every loaded item
and validates the resulting `CreatureDefinition`. A new error variant
`ItemDatabaseError::InvalidMeshDescriptor { item_id, message }` is returned
on the first failure.

#### SDK plumbing (`src/sdk/validation.rs`, `src/sdk/error_formatter.rs`)

- `ValidationError::ItemMeshDescriptorInvalid { item_id, message }` — new
  `Error`-severity variant.
- `Validator::validate_item_mesh_descriptors()` — calls
  `ItemDatabase::validate_mesh_descriptors` and converts the result into a
  `Vec<ValidationError>`.
- `validate_all()` now calls `validate_item_mesh_descriptors()`.
- `error_formatter.rs` has an actionable suggestion block for the new variant.

---

### Architecture compliance

- `CreatureDefinition` is reused as the output type — no new rendering path.
- `ItemId`, `ItemType` type aliases used throughout.
- `#[serde(default)]` on `mesh_descriptor_override` preserves full backward
  compatibility with all existing RON files.
- All geometry builders produce non-degenerate triangles (centre-vertex fan).
- No constants are hard-coded; all shape parameters (`BASE_SCALE`,
  `TWO_HANDED_SCALE_MULT`, `BLADE_SIDES_FACTOR`, etc.) are named constants.
- SPDX headers present in `item_mesh.rs`.
- Test data uses `data/items.ron` (Implementation Rule 5 compliant).

---

### Test coverage

**`src/domain/visual/item_mesh.rs`** (inline `mod tests`):

| Test                                                       | What it verifies                                                  |
| ---------------------------------------------------------- | ----------------------------------------------------------------- |
| `test_sword_descriptor_from_short_sword`                   | Short sword → `Sword` category, correct blade length, no emissive |
| `test_dagger_descriptor_short_blade`                       | Dagger → `Dagger` category, blade shorter than same-sides sword   |
| `test_potion_color_heal_is_red`                            | `HealHp` → red primary color                                      |
| `test_potion_color_restore_sp_is_blue`                     | `RestoreSp` → blue                                                |
| `test_potion_color_boost_attribute_is_yellow`              | `BoostAttribute` → yellow                                         |
| `test_cure_condition_produces_scroll`                      | `CureCondition` → `Scroll` category                               |
| `test_magical_item_emissive`                               | `max_charges > 0` → emissive                                      |
| `test_magical_item_emissive_via_bonus`                     | `constant_bonus` → emissive                                       |
| `test_cursed_item_dark_tint`                               | `is_cursed` → dark purple + purple emissive                       |
| `test_cursed_overrides_magical_glow`                       | Cursed+magical → cursed emissive wins                             |
| `test_two_handed_weapon_larger_scale`                      | `two_handed` tag → scale > one-handed                             |
| `test_descriptor_to_creature_definition_valid`             | Round-trip for all categories passes `validate()`                 |
| `test_override_color_applied`                              | `primary_color` override applied                                  |
| `test_override_scale_applied`                              | `scale` override applied                                          |
| `test_override_invalid_scale_ignored`                      | Negative scale override ignored                                   |
| `test_override_emissive_applied`                           | Non-zero emissive override enables flag                           |
| `test_override_zero_emissive_disables`                     | All-zero emissive override disables flag                          |
| `test_quest_item_descriptor_unique_shape`                  | Quest items → `QuestItem` category, always emissive               |
| `test_all_accessory_slots_produce_valid_definitions`       | All 4 accessory slots round-trip                                  |
| `test_all_armor_classifications_produce_valid_definitions` | All 4 armor classes round-trip                                    |
| `test_ammo_descriptor_valid`                               | Ammo → valid definition                                           |
| `test_descriptor_default_override_is_identity`             | Empty override = no override                                      |

**`src/domain/items/database.rs`** (extended `mod tests`):

| Test                                            | What it verifies                                  |
| ----------------------------------------------- | ------------------------------------------------- |
| `test_validate_mesh_descriptors_all_base_items` | Loads `data/items.ron`; all items pass validation |
| `test_validate_mesh_descriptors_empty_db`       | Empty DB → `Ok(())`                               |
| `test_validate_mesh_descriptors_all_item_types` | One item of every `ItemType` variant → `Ok(())`   |

---

## Items Procedural Meshes — Phase 2: Game Engine — Dropped Item Mesh Generation

**Plan**: [`items_procedural_meshes_implementation_plan.md`](items_procedural_meshes_implementation_plan.md)

### Overview

Phase 2 wires the domain-layer types from Phase 1 into the live Bevy game
engine. Dropping an item from inventory now spawns a procedural 3-D mesh on
the party's current tile; static `MapEvent::DroppedItem` entries in RON map
files cause the same mesh to appear on map load; picking up an item despawns
the mesh.

---

### Phase 2 Deliverables

**Files created**:

- `src/game/components/dropped_item.rs` — `DroppedItem` ECS marker component
- `src/game/systems/item_world_events.rs` — `ItemDroppedEvent`, `ItemPickedUpEvent`, spawn / despawn / map-load systems, `ItemWorldPlugin`

**Files modified**:

- `src/domain/world/types.rs` — `MapEvent::DroppedItem` variant added
- `src/domain/world/events.rs` — `DroppedItem` arm in `trigger_event` match
- `src/game/components/mod.rs` — `pub mod dropped_item` + re-export
- `src/game/resources/mod.rs` — `DroppedItemRegistry` resource
- `src/game/systems/mod.rs` — `pub mod item_world_events`
- `src/game/systems/procedural_meshes.rs` — 12 item mesh cache slots, `get_or_create_item_mesh`, 10 per-category spawn functions (`spawn_sword_mesh`, `spawn_dagger_mesh`, `spawn_blunt_mesh`, `spawn_staff_mesh`, `spawn_bow_mesh`, `spawn_armor_mesh`, `spawn_shield_mesh`, `spawn_potion_mesh`, `spawn_scroll_mesh`, `spawn_ring_mesh`, `spawn_ammo_mesh`), `spawn_dropped_item_mesh` dispatcher, 11 config structs
- `src/game/systems/inventory_ui.rs` — drop action fires `ItemDroppedEvent`
- `src/game/systems/events.rs` — `MapEvent::DroppedItem` arm in `handle_events`
- `src/sdk/validation.rs` — `MapEvent::DroppedItem` validation arm
- `src/bin/validate_map.rs` — `MapEvent::DroppedItem` counting arm
- `src/bin/antares.rs` — `ItemWorldPlugin` registered

---

### What was built

#### `DroppedItem` component (`src/game/components/dropped_item.rs`)

`#[derive(Component, Clone, Debug, PartialEq, Eq)]` struct that marks any
entity whose mesh represents an item lying on the ground. Stores `item_id`,
`map_id`, `tile_x`, `tile_y`, and `charges`.

#### `DroppedItemRegistry` resource (`src/game/resources/mod.rs`)

`#[derive(Resource, Default)]` wrapping a `HashMap<(MapId, i32, i32, ItemId),
Entity>`. Provides typed `insert`, `get`, and `remove` helpers. Used to
correlate pickup events with ECS entities for targeted despawn.

#### `MapEvent::DroppedItem` variant (`src/domain/world/types.rs`)

New enum arm with `name: String`, `item_id: ItemId`, and
`#[serde(default)] charges: u16`. All fields that are optional use
`#[serde(default)]` so existing RON map files that pre-date this variant
remain valid without modification.

#### `ItemDroppedEvent` / `ItemPickedUpEvent` (`src/game/systems/item_world_events.rs`)

`#[derive(Message, Clone, Debug)]` event structs carrying `item_id`, `charges`,
`map_id`, `tile_x`, `tile_y` (drop) or the same minus charges (pickup).
Registered with `app.add_message::<…>()` inside `ItemWorldPlugin`.

#### `spawn_dropped_item_system`

Reads `MessageReader<ItemDroppedEvent>`. For each event:

1. Looks up the item from `GameContent`; skips with a warning if not found.
2. Calls `ItemMeshDescriptor::from_item` → `to_creature_definition`.
3. Calls `spawn_creature` at world-space `(tile_x + 0.5, 0.05, tile_y + 0.5)`.
4. Applies a random Y-axis jitter rotation for visual variety.
5. Inserts `DroppedItem`, `MapEntity`, `TileCoord`, and a `Name` component.
6. Registers the entity in `DroppedItemRegistry`.

`GameContent` is wrapped in `Option<Res<…>>` so the system degrades gracefully
when content is not yet loaded.

#### `despawn_picked_up_item_system`

Reads `MessageReader<ItemPickedUpEvent>`. Looks up the entity in
`DroppedItemRegistry` by the four-part key, calls
`commands.entity(entity).despawn()` (Bevy 0.17 — recursive by default), and
removes the registry entry. Unknown keys emit a `warn!` log.

#### `load_map_dropped_items_system`

Stores the last-processed map ID in a `Local<Option<MapId>>`. On map change,
iterates all `MapEvent::DroppedItem` entries on the new map and fires
`ItemDroppedEvent` for each so static map-authored drops share the identical
spawn path as runtime drops.

#### Item mesh config structs & generators (`src/game/systems/procedural_meshes.rs`)

Eleven typed config structs (`SwordConfig`, `DaggerConfig`, `BluntConfig`,
`StaffConfig`, `BowConfig`, `ArmorMeshConfig`, `ShieldConfig`, `PotionConfig`,
`ScrollConfig`, `RingMeshConfig`, `AmmoConfig`) plus a `spawn_dropped_item_mesh`
dispatcher that selects the right generator from `ItemMeshCategory`.

Twelve item mesh cache slots added to `ProceduralMeshCache` (one per category
string: `"sword"`, `"dagger"`, `"blunt"`, `"staff"`, `"bow"`, `"armor"`,
`"shield"`, `"potion"`, `"scroll"`, `"ring"`, `"ammo"`, `"quest"`).
`get_or_create_item_mesh` follows the same pattern as the existing
`get_or_create_furniture_mesh`. `clear_all` and `cached_count` updated.

Notable mesh details:

- **Potion**: `AlphaMode::Blend` on both bottle and liquid inner cylinder;
  liquid colour carries a faint emissive glow matching the liquid tint.
- **Staff**: emissive orb at tip.
- **Shield**: flat `Cylinder` disc with `FRAC_PI_2` X-rotation.
- **Ring**: `Torus` primitive (`minor_radius` = 0.018, `major_radius` = 0.065).
- **Ammo**: three sub-types (`"arrow"`, `"bolt"`, `"stone"`) selected from
  `AmmoConfig::ammo_type`.

#### Inventory drop integration (`src/game/systems/inventory_ui.rs`)

`inventory_action_system` now accepts
`Option<MessageWriter<ItemDroppedEvent>>` and fires it when a drop action
removes an item from a character's inventory. The writer is `Option`-wrapped
so existing tests that do not register the message type continue to pass.

---

### Architecture compliance

| Check                                          | Status                                                                                          |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Data structures match architecture.md §4       | ✅ `ItemId`, `MapId` type aliases used throughout                                               |
| Module placement follows §3.2                  | ✅ Components in `game/components/`, resources in `game/resources/`, systems in `game/systems/` |
| No `unwrap()` without justification            | ✅ All error paths use `warn!` / `Option` guards                                                |
| RON format for data files                      | ✅ `MapEvent::DroppedItem` serde-compatible with existing `.ron` map files                      |
| Constants extracted, not hardcoded             | ✅ `DROPPED_ITEM_Y`, `DROP_ROTATION_JITTER`, `TILE_CENTER_OFFSET`, 7 `ITEM_*_COLOR` constants   |
| SPDX headers on all new `.rs` files            | ✅ `2026 Brett Smith` header on `dropped_item.rs` and `item_world_events.rs`                    |
| Test data in `data/`, not `campaigns/tutorial` | ✅ No test references campaign data                                                             |
| Backward compatibility                         | ✅ `#[serde(default)]` on `MapEvent::DroppedItem` fields; existing RON files unaffected         |

---

### Test coverage

**`src/game/components/dropped_item.rs`** (9 tests):

| Test                                       | What it checks                                  |
| ------------------------------------------ | ----------------------------------------------- |
| `test_dropped_item_component_fields`       | All five fields stored correctly                |
| `test_dropped_item_clone`                  | `Clone` produces equal copy                     |
| `test_dropped_item_debug`                  | `Debug` output non-empty and contains type name |
| `test_dropped_item_equality`               | `PartialEq` symmetric                           |
| `test_dropped_item_inequality_item_id`     | Different `item_id` → not equal                 |
| `test_dropped_item_inequality_map_id`      | Different `map_id` → not equal                  |
| `test_dropped_item_inequality_tile_coords` | Different tiles → not equal                     |
| `test_dropped_item_zero_charges`           | Zero charges accepted                           |
| `test_dropped_item_max_charges`            | `u16::MAX` accepted without overflow            |

**`src/game/resources/mod.rs`** (5 tests):

| Test                                       | What it checks                          |
| ------------------------------------------ | --------------------------------------- |
| `test_dropped_item_registry_default_empty` | Default has no entries                  |
| `test_registry_insert_and_lookup`          | Insert + `get` by key                   |
| `test_registry_remove_on_pickup`           | Remove returns entity; key absent after |
| `test_registry_two_entries`                | Two distinct keys coexist               |
| `test_registry_insert_overwrites`          | Later insert replaces earlier entity    |

**`src/game/systems/item_world_events.rs`** (10 tests):

| Test                                       | What it checks             |
| ------------------------------------------ | -------------------------- |
| `test_item_dropped_event_creation`         | All five fields set        |
| `test_item_picked_up_event_creation`       | All four fields set        |
| `test_item_dropped_event_clone`            | `Clone`                    |
| `test_item_picked_up_event_clone`          | `Clone`                    |
| `test_item_dropped_event_debug`            | `Debug` contains type name |
| `test_item_picked_up_event_debug`          | `Debug` contains type name |
| `test_item_dropped_event_zero_charges`     | Zero charges valid         |
| `test_item_dropped_event_max_charges`      | `u16::MAX` valid           |
| `test_item_picked_up_event_negative_tiles` | Negative tile coords valid |
| `test_dropped_item_y_is_positive`          | Constant assertion         |
| `test_tile_center_offset_is_half`          | Constant assertion         |

**`src/game/systems/procedural_meshes.rs`** (`item_mesh_tests` module, 18 tests):

| Test                                            | What it checks                                       |
| ----------------------------------------------- | ---------------------------------------------------- |
| `test_sword_config_defaults`                    | `blade_length > 0`, `has_crossguard`, `color = None` |
| `test_dagger_config_defaults`                   | `blade_length < sword blade_length`                  |
| `test_potion_config_defaults`                   | Non-zero color components                            |
| `test_scroll_config_defaults`                   | Non-zero alpha; R > 0.5 (parchment)                  |
| `test_cache_item_slots_default_none`            | All 12 item slots `None` at default                  |
| `test_cache_item_slots_cleared_after_clear_all` | `clear_all` resets item slots                        |
| `test_blunt_config_defaults`                    | Positive dimensions                                  |
| `test_staff_config_defaults`                    | Positive `length` and `orb_radius`                   |
| `test_bow_config_defaults`                      | Positive `arc_height`                                |
| `test_armor_mesh_config_defaults`               | Positive dimensions; `is_helmet = false`             |
| `test_shield_config_defaults`                   | Positive `radius`                                    |
| `test_ring_mesh_config_defaults`                | Non-zero alpha                                       |
| `test_ammo_config_defaults`                     | Non-zero alpha; type = `"arrow"`                     |
| `test_item_color_constants_valid`               | All 7 colour constants convert to valid `LinearRgba` |
| `test_sword_config_clone`                       | `Clone`                                              |
| `test_dagger_config_clone`                      | `Clone`                                              |
| `test_potion_config_clone`                      | `Clone`                                              |
| `test_scroll_config_clone`                      | `Clone`                                              |
| `test_ammo_config_clone`                        | `Clone`                                              |

---

## Items Procedural Meshes — Phase 3: Item Mesh RON Asset Files

### Overview

Phase 3 creates the data layer that backs Phase 2's runtime mesh generation:
RON asset files for every dropped-item category, a `CreatureReference` registry
so the campaign loader can discover them, a new `ItemMeshDatabase` type
(thin `CreatureDatabase` wrapper), an extended `CampaignLoader` that loads
the registry (opt-in; missing file is silently skipped), a
`ItemDatabase::link_mesh_overrides` validation hook, and the Python generator
script that keeps the asset files regenerable from a single authoritative
manifest.

### Phase 3 Deliverables

| Deliverable                              | Path                                                            |
| ---------------------------------------- | --------------------------------------------------------------- |
| Generator script                         | `examples/generate_item_meshes.py`                              |
| Tutorial campaign item mesh RON files    | `campaigns/tutorial/assets/items/` (27 files)                   |
| Tutorial campaign item mesh registry     | `campaigns/tutorial/data/item_mesh_registry.ron`                |
| Test-campaign minimal RON fixtures       | `data/test_campaign/assets/items/sword.ron`, `potion.ron`       |
| Test-campaign item mesh registry         | `data/test_campaign/data/item_mesh_registry.ron`                |
| `ItemMeshDatabase` type                  | `src/domain/items/database.rs`                                  |
| `ItemDatabase::link_mesh_overrides`      | `src/domain/items/database.rs`                                  |
| `ItemDatabaseError::UnknownMeshOverride` | `src/domain/items/database.rs`                                  |
| `GameData::item_meshes` field            | `src/domain/campaign_loader.rs`                                 |
| `CampaignLoader::load_item_meshes`       | `src/domain/campaign_loader.rs`                                 |
| Integration tests                        | `src/domain/campaign_loader.rs`, `src/domain/items/database.rs` |

### What was built

#### `examples/generate_item_meshes.py`

Developer convenience tool that generates one `CreatureDefinition` RON file per
item mesh type. The script mirrors all color and scale constants from
`src/domain/visual/item_mesh.rs` so the generated geometry exactly matches what
`ItemMeshDescriptor::build_mesh` would produce at runtime.

- `--output-dir <path>` writes the full 27-file manifest to a custom directory
  (default: `campaigns/tutorial/assets/items/`).
- `--test-fixtures` writes only the two minimal test fixtures
  (`sword.ron`, `potion.ron`) to `data/test_campaign/assets/items/`.
- Geometry helpers: `blade_mesh`, `blunt_mesh`, `staff_mesh`, `bow_mesh`,
  `armor_mesh`, `helmet_mesh`, `shield_mesh`, `boots_mesh`, `ring_mesh`,
  `belt_mesh`, `cloak_mesh`, `potion_mesh`, `scroll_mesh`, `ammo_mesh`,
  `quest_mesh` — each produces a flat XZ-plane silhouette with correct normals
  and an optional `MaterialDefinition` (metallic / roughness / emissive).
- `MANIFEST` table: 27 items covering weapon (9001–9008), armor (9101–9106),
  consumable (9201–9204), accessory (9301–9304), ammo (9401–9403), and quest
  (9501–9502) categories. IDs start at 9000 to avoid collision with creature /
  NPC / template IDs.
- `TEST_MANIFEST`: 2-item subset (`sword` id=9001, `potion` id=9201) for stable
  integration test fixtures.

#### Item mesh RON asset files (`campaigns/tutorial/assets/items/`)

27 `CreatureDefinition` RON files organised into six sub-directories:

```
weapons/    sword, dagger, short_sword, long_sword, great_sword, club, staff, bow
armor/      leather_armor, chain_mail, plate_mail, shield, helmet, boots
consumables/ health_potion, mana_potion, cure_potion, attribute_potion
accessories/ ring, amulet, belt, cloak
ammo/        arrow, bolt, stone
quest/       quest_scroll (2 meshes), key_item
```

Each file is a valid `CreatureDefinition` with:

- `id` in the 9000+ range matching the registry entry.
- One (or two for quest_scroll) flat-lying `MeshDefinition` meshes with
  per-vertex `normals: Some([...])` pointing upward.
- A `MaterialDefinition` with correct metallic / roughness / emissive values.
- An identity `MeshTransform` per mesh.
- `color_tint: None`.

#### `campaigns/tutorial/data/item_mesh_registry.ron`

`Vec<CreatureReference>` listing all 27 tutorial campaign item meshes. The
registry format is identical to `data/creatures.ron`; `CampaignLoader` reuses
`CreatureDatabase::load_from_registry` internally.

#### Test-campaign fixtures

`data/test_campaign/assets/items/sword.ron` (id=9001) and
`data/test_campaign/assets/items/potion.ron` (id=9201) are minimal stable
fixtures committed to the repository. They are referenced by
`data/test_campaign/data/item_mesh_registry.ron` and used exclusively by
integration tests — never by the live tutorial campaign.

#### `ItemMeshDatabase` (`src/domain/items/database.rs`)

Thin `#[derive(Debug, Clone, Default)]` wrapper around `CreatureDatabase`:

```src/domain/items/database.rs#L447-460
pub struct ItemMeshDatabase {
    inner: CreatureDatabase,
}
```

Public API:

| Method                                             | Description                                         |
| -------------------------------------------------- | --------------------------------------------------- |
| `new()` / `default()`                              | Empty database                                      |
| `load_from_registry(registry_path, campaign_root)` | Delegates to `CreatureDatabase::load_from_registry` |
| `as_creature_database()`                           | Returns `&CreatureDatabase` for direct queries      |
| `is_empty()`                                       | True if no entries                                  |
| `count()`                                          | Number of mesh entries                              |
| `has_mesh(id: u32)`                                | True if creature ID present                         |
| `validate()`                                       | Validates all mesh `CreatureDefinition`s            |

Re-exported from `src/domain/items/mod.rs` as `antares::domain::items::ItemMeshDatabase`.

#### `ItemDatabase::link_mesh_overrides` (`src/domain/items/database.rs`)

Forward-compatibility validation hook:

```src/domain/items/database.rs#L435-442
pub fn link_mesh_overrides(
    &self,
    _registry: &ItemMeshDatabase,
) -> Result<(), ItemDatabaseError> {
```

Walks all items that carry a `mesh_descriptor_override`, calls
`ItemMeshDescriptor::from_item` + `CreatureDefinition::validate` to confirm
the override does not break mesh generation. Full registry cross-linking
(verifying that a named creature ID exists in `ItemMeshDatabase`) is reserved
for a future extension of `ItemMeshDescriptorOverride` with an explicit
`creature_id` field.

#### `GameData::item_meshes` and `CampaignLoader::load_item_meshes`

`GameData` now carries:

```src/domain/campaign_loader.rs#L90-95
pub struct GameData {
    pub creatures: CreatureDatabase,
    pub item_meshes: ItemMeshDatabase,
}
```

`CampaignLoader::load_game_data` calls the new `load_item_meshes` helper which:

1. Looks for `data/item_mesh_registry.ron` inside the campaign directory.
2. If absent — returns `ItemMeshDatabase::new()` silently (opt-in per campaign).
3. If present — calls `ItemMeshDatabase::load_from_registry`, propagating any
   read / parse errors as `CampaignError::ReadError`.

`GameData::validate` also calls `item_meshes.validate()` so malformed mesh RON
files are caught at load time.

Note: `GameData` no longer derives `Serialize`/`Deserialize` because
`ItemMeshDatabase` wraps `CreatureDatabase` (which does) but the wrapper itself
is `Debug + Clone` only — sufficient for all current usages.

### Architecture compliance

- [ ] `ItemMeshDatabase` IDs are in the 9000+ range — no collision with
      creature IDs (1–50), NPC IDs (1000+), template IDs (2000+), variant IDs (3000+).
- [ ] RON format used for all asset and registry files — no JSON or YAML.
- [ ] File names follow lowercase + underscore convention (`item_mesh_registry.ron`,
      `health_potion.ron`, etc.).
- [ ] SPDX headers present in `generate_item_meshes.py`.
- [ ] All test data in `data/test_campaign/` — no references to
      `campaigns/tutorial` from tests.
- [ ] `CampaignLoader` opt-in: missing registry file is not an error.
- [ ] `ItemMeshDatabase` does not replace `CreatureDatabase`; it is an additive
      type that sits alongside it.

### Test coverage

**`src/domain/items/database.rs`** — 11 new unit tests:

| Test                                                       | What it verifies                                        |
| ---------------------------------------------------------- | ------------------------------------------------------- |
| `test_item_mesh_database_new_is_empty`                     | `new()` starts empty                                    |
| `test_item_mesh_database_default_is_empty`                 | `default()` == `new()`                                  |
| `test_item_mesh_database_has_mesh_absent`                  | `has_mesh` returns false for absent IDs                 |
| `test_item_mesh_database_validate_empty`                   | `validate()` succeeds on empty DB                       |
| `test_item_mesh_database_as_creature_database`             | Inner DB accessible                                     |
| `test_item_mesh_database_load_from_registry_missing_file`  | Missing file → error                                    |
| `test_item_mesh_database_load_from_registry_test_campaign` | Loads ≥ 2 entries from fixture; ids 9001 & 9201 present |
| `test_item_mesh_database_validate_test_campaign`           | Loaded fixture validates without error                  |
| `test_link_mesh_overrides_empty_item_db`                   | Empty `ItemDatabase` → ok                               |
| `test_link_mesh_overrides_no_override_items_skipped`       | Items without override → ok                             |
| `test_link_mesh_overrides_valid_override_passes`           | Valid override passes mesh validation                   |

**`src/domain/campaign_loader.rs`** — 2 new integration tests:

| Test                                            | What it verifies                                                                            |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `test_campaign_loader_loads_item_mesh_registry` | Full `load_game_data` against `data/test_campaign` populates `item_meshes` with ≥ 2 entries |
| `test_item_mesh_registry_missing_is_ok`         | Missing registry file returns empty `ItemMeshDatabase` without error                        |

All tests reference `data/test_campaign` — never `campaigns/tutorial`
(Implementation Rule 5 compliant).

---

## Procedural Meshes Direction Control

**Plan**: [`procedural_meshes_direction_control_implementation_plan.md`](procedural_meshes_direction_control_implementation_plan.md)

### Overview

All creatures (NPCs, recruitable characters, monsters) and signs spawned as
procedural meshes previously faced the same default direction because
`spawn_creature()` had no rotation parameter and `MapEvent` variants carried
no `facing` field. This implementation adds per-entity cardinal facing across
the full stack: domain data model, ECS spawn systems, runtime event system,
smooth rotation animation, and Campaign Builder SDK UI.

---

### Phase 1: Direction-to-Rotation Foundation

**Files changed**:

- `src/domain/types.rs`
- `src/game/components/creature.rs`
- `src/game/components/mod.rs`
- `src/game/systems/creature_spawning.rs`

**What was built**:

`Direction::direction_to_yaw_radians(&self) -> f32` is a new method on the
`Direction` enum that maps each cardinal to a Y-axis rotation in radians:
North → 0.0, East → π/2, South → π, West → 3π/2. The inverse,
`Direction::from_yaw_radians(yaw: f32) -> Direction`, normalises any yaw
value into `[0, 2π)` and rounds to the nearest 90° cardinal. These two
methods are the single source of truth for the angle mapping; no other file
redefines the cardinal-to-float relationship.

`FacingComponent { direction: Direction }` is a new ECS component in
`creature.rs` (re-exported from `components/mod.rs`). It is the authoritative
runtime facing state for every spawned creature, NPC, and sign entity.

`spawn_creature()` gained a `facing: Option<Direction>` parameter. It
computes `Quat::from_rotation_y(d.direction_to_yaw_radians())` from the
resolved direction, applies it to the parent `Transform`, and inserts
`FacingComponent` on the parent entity. All pre-existing call sites pass
`None`, preserving identity rotation.

---

### Phase 2: Static Map-Time Facing

**Files changed**:

- `src/domain/world/types.rs`
- `src/game/systems/map.rs`
- `src/game/systems/procedural_meshes.rs`
- `campaigns/tutorial/data/maps/map_1.ron`

**What was built**:

`facing: Option<Direction>` with `#[serde(default)]` was added to
`MapEvent::Sign`, `MapEvent::NpcDialogue`, `MapEvent::Encounter`, and
`MapEvent::RecruitableCharacter`. The `#[serde(default)]` annotation keeps
all existing RON files valid without migration — omitted fields deserialise
to `None` (identity rotation).

In `map.rs`, the NPC spawn block now passes `resolved_npc.facing` to
`spawn_creature()`. The sprite-fallback path applies the same yaw rotation
directly to the sprite entity's `Transform`. An `NpcDialogue` event-level
`facing` overrides the NPC placement `facing` when both are present.
`MapEvent::Encounter` and `MapEvent::RecruitableCharacter` spawn blocks
forward their `facing` field to `spawn_creature()`.

`spawn_sign()` in `procedural_meshes.rs` gained a `facing: Option<Direction>`
parameter. Cardinal facing takes precedence over the existing `rotation_y:
Option<f32>` degrees parameter when both are provided. `FacingComponent` is
inserted on sign entities.

The tutorial map was updated: `Old Gareth` (`RecruitableCharacter` at map_1
(15,7)) has `facing: Some(West)` as a functional smoke-test for map-time
facing on event entities. An NPC placement in map_1 has `facing: Some(South)`
as the smoke-test for NPC placement facing.

---

### Phase 3: Runtime Facing Change System

**Files changed**:

- `src/game/systems/facing.rs` (new file)
- `src/game/systems/map.rs`
- `src/game/systems/dialogue.rs`
- `src/domain/world/types.rs`

**What was built**:

A new `src/game/systems/facing.rs` module provides the full runtime facing
system and is registered via `FacingPlugin`.

`SetFacing { entity: Entity, direction: Direction, instant: bool }` is a
Bevy message. `handle_set_facing` reads it each frame: when `instant: true`
it snaps `Transform.rotation` and updates `FacingComponent.direction`
directly; when `instant: false` it inserts a `RotatingToFacing` component
for frame-by-frame slerp (Phase 4).

`ProximityFacing { trigger_distance: u32, rotation_speed: Option<f32> }` is
a marker component inserted by the map loading system on entities whose
`MapEvent` has `proximity_facing: true`. The `face_toward_player_on_proximity`
system queries all entities carrying this component each frame, computes the
4-direction from the entity's `TileCoord` to `GlobalState::party_position`
using the `cardinal_toward()` helper, and emits a `SetFacing` event whenever
the nearest cardinal differs from the current `FacingComponent.direction`.

`proximity_facing: bool` (with `#[serde(default)]`) was added to
`MapEvent::Encounter` and `MapEvent::NpcDialogue`. The map loading system in
`map.rs` inserts `ProximityFacing { trigger_distance: 2, rotation_speed }`
on the spawned entity when this flag is true, forwarding the companion
`rotation_speed` field.

`handle_start_dialogue` in `dialogue.rs` was extended: when the speaker
entity has a `TileCoord`, it computes the direction from the speaker toward
the party and writes a `SetFacing { instant: true }` event so the NPC always
faces the player at dialogue start.

---

### Phase 4: Smooth Rotation Animation

**Files changed**:

- `src/game/systems/facing.rs`
- `src/domain/world/types.rs`

**What was built**:

`RotatingToFacing { target: Quat, speed_deg_per_sec: f32, target_direction: Direction }`
is a scratch ECS component inserted by `handle_set_facing` when `instant:
false`. It is never serialised and carries the logical `target_direction` so
`FacingComponent` can be updated correctly when the rotation completes.

`apply_rotation_to_facing` is a per-frame system that queries all entities
carrying `RotatingToFacing`. Each frame it computes the remaining angle
between the current and target quaternion. When the remaining angle exceeds
the `ROTATION_COMPLETE_THRESHOLD_RAD` (0.01 rad) constant it advances the
rotation using `Quat::slerp` at the configured speed. When within the
threshold it snaps to the exact target, writes the final direction to
`FacingComponent`, and removes the `RotatingToFacing` component. This keeps
the snap paths unchanged and performant.

`rotation_speed: Option<f32>` (with `#[serde(default)]`) was added to
`MapEvent::Encounter` and `MapEvent::NpcDialogue`. When set, the value is
forwarded to `ProximityFacing.rotation_speed` and used as the
`speed_deg_per_sec` when `handle_set_facing` inserts `RotatingToFacing`.
`None` means snap (instant).

---

### Phase 5: Campaign Builder SDK UI

**Files changed**:

- `sdk/campaign_builder/src/map_editor.rs`

**What was built**:

Three fields were added to `EventEditorState`:

- `event_facing: Option<String>` — the selected cardinal direction name, or
  `None` for the engine default (North). Applies to `Sign`, `NpcDialogue`,
  `Encounter`, and `RecruitableCharacter`.
- `event_proximity_facing: bool` — mirrors the `proximity_facing` RON flag.
  Applies to `Encounter` and `NpcDialogue` only.
- `event_rotation_speed: Option<f32>` — mirrors the `rotation_speed` RON
  field. Applies to `Encounter` and `NpcDialogue` only. Suppressed in
  `to_map_event()` when `event_proximity_facing` is `false`.

`Default for EventEditorState` initialises all three to `None`, `false`,
and `None` respectively.

A **Facing** combo-box was added to the bottom of each of the four affected
`match` arms in `show_event_editor()`. Each combo-box uses a unique
`id_salt` to satisfy the egui ID rules:

| Event type             | `id_salt`                           |
| ---------------------- | ----------------------------------- |
| `Sign`                 | `"sign_event_facing_combo"`         |
| `NpcDialogue`          | `"npc_dialogue_event_facing_combo"` |
| `Encounter`            | `"encounter_event_facing_combo"`    |
| `RecruitableCharacter` | `"recruitable_event_facing_combo"`  |

A **Behaviour** section (separator + label + checkbox + conditional
text-input) was added to the `Encounter` and `NpcDialogue` arms only,
surfacing the proximity-facing toggle and the rotation-speed field.
The rotation-speed input renders only when the proximity-facing checkbox
is ticked.

`to_map_event()` was updated for all four variants to parse `event_facing`
via the private `parse_facing()` helper and include it in the constructed
`MapEvent`. For `Encounter` and `NpcDialogue` it also forwards
`proximity_facing` and `rotation_speed` (with the suppression rule above).

`from_map_event()` was updated for all four variants to populate
`event_facing`, `event_proximity_facing`, and `event_rotation_speed` from
the loaded event, preserving backward compatibility for RON files that
predate these fields.

`show_inspector_panel()` was extended for all four event types to display
the `facing` direction when set. For `Encounter` and `NpcDialogue` it also
shows the proximity-facing label and rotation speed when applicable.

---

### Test Coverage

| Module                                   | Key tests added                                                                                                                                                                                                                                                                                                                                                                                                |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/types.rs`                    | `test_direction_to_yaw_north/east/south/west`, `test_direction_roundtrip`, `test_direction_from_yaw_cardinals`, `test_direction_from_yaw_snaps_to_nearest`                                                                                                                                                                                                                                                     |
| `src/game/components/creature.rs`        | `test_facing_component_new`, `test_facing_component_default_is_north`, `test_facing_component_all_directions`, `test_facing_component_clone/equality`                                                                                                                                                                                                                                                          |
| `src/game/systems/creature_spawning.rs`  | `test_spawn_creature_facing_none_is_north`, `test_spawn_creature_facing_south_rotation`                                                                                                                                                                                                                                                                                                                        |
| `src/game/systems/map.rs`                | `test_npc_facing_applied_at_spawn`, `test_facing_component_on_npc`, `test_map_event_encounter_facing`, `test_map_event_sign_facing`, `test_map_event_ron_round_trip`, `test_proximity_facing_inserted_on_encounter_with_flag`, `test_proximity_facing_not_inserted_when_flag_false`, `test_proximity_facing_npc_inserted_when_flag_set`                                                                        |
| `src/game/systems/facing.rs`             | `test_set_facing_snaps_transform`, `test_set_facing_updates_facing_component`, `test_proximity_facing_emits_event`, `test_set_facing_instant_false_inserts_rotating_component`, `test_rotating_to_facing_approaches_target`, `test_rotating_to_facing_completes_and_removes_component`                                                                                                                         |
| `src/game/systems/dialogue.rs`           | `test_dialogue_start_emits_set_facing`, `test_dialogue_start_no_speaker_entity_does_not_panic`, `test_dialogue_start_speaker_without_tile_coord_skips_facing`                                                                                                                                                                                                                                                  |
| `sdk/campaign_builder/src/map_editor.rs` | `test_event_editor_state_default_facing_none`, `test_event_editor_to_sign_with_facing`, `test_event_editor_from_sign_with_facing`, `test_event_editor_from_sign_no_facing`, `test_event_editor_to_encounter_with_facing_and_proximity`, `test_event_editor_from_encounter_with_proximity`, `test_event_editor_facing_round_trip_all_variants`, `test_event_editor_proximity_false_clears_rotation_speed_in_ui` |

---

### Architecture Compliance

- `direction_to_yaw_radians` is the **single source of truth** for the
  cardinal-to-angle mapping; no other file redefines north/south/etc as raw
  floats.
- All new `MapEvent` fields use `#[serde(default)]` — all existing RON files
  remain valid without migration.
- `SetFacing` follows the existing `#[derive(Message)]` broadcast pattern.
- `RotatingToFacing` is a pure ECS scratch component — never serialised,
  never referenced by domain structs.
- `FacingPlugin` registers all three systems (`handle_set_facing`,
  `face_toward_player_on_proximity`, `apply_rotation_to_facing`) in a single
  plugin, keeping the addition self-contained.
- No test references `campaigns/tutorial`; all test fixtures use
  `data/test_campaign`.
