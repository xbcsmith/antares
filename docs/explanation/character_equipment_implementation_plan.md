<!-- SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Character Equipment Implementation Plan

## Overview

Characters currently have `Equipment` slots that are populated directly at
creation time and never changed during gameplay. There is no way to equip or
unequip items at runtime: the inventory UI has only Drop, Transfer, and Use
actions. Starting gear bypasses inventory entirely. AC is initialized to `0`
despite a defined `AC_DEFAULT = 10` constant, and damage ignores equipped
weapon bonuses at several callsites. This plan delivers the complete equipment
lifecycle — equip, unequip, AC recalculation, starting-gear-in-inventory, and
the `ArmorClassification` expansion needed for helmet and boots slot routing —
as a series of self-contained, sequentially compilable phases.

## Current State Analysis

### Existing Infrastructure

- `Equipment` in [`src/domain/character.rs`](../../src/domain/character.rs)
  has seven fields: `weapon`, `armor`, `shield`, `helmet`, `boots`,
  `accessory1`, `accessory2`. All are `Option<ItemId>`.
- `can_equip_item` in
  [`src/domain/items/equipment_validation.rs`](../../src/domain/items/equipment_validation.rs)
  validates class/race/alignment restrictions but performs **no mutation**.
- `CharacterDefinition::instantiate` in
  [`src/domain/character_definition.rs`](../../src/domain/character_definition.rs)
  copies `StartingEquipment` fields directly into `Equipment` via
  `create_starting_equipment` — bypassing inventory and `can_equip_item`
  entirely.
- `get_character_attack` in
  [`src/domain/combat/engine.rs`](../../src/domain/combat/engine.rs) already
  reads `equipment.weapon` and derives the correct `Attack` (Phase 2 of
  `equipped_weapon_damage_implementation_plan.md` is complete).
- `resolve_attack` reads `character.ac.current` directly from `CombatState`.
- `ArmorClassification` has only four variants: `Light`, `Medium`, `Heavy`,
  `Shield`. There are no `Helmet` or `Boots` variants; the `helmet` and
  `boots` equipment slots cannot be routed correctly.
- `UNARMED_DAMAGE`, `AC_MIN`, `AC_MAX`, and `AC_DEFAULT = 10` are defined,
  but `Character::new()` initialises `ac: AttributePair::new(0)` — not
  `AC_DEFAULT`.
- `Equipment::equipped_count()` iterates only six slots; `accessory2` is
  silently excluded. `MAX_EQUIPPED` is `6` but there are seven struct fields.
- `transactions.rs` holds `drop_item`, `pickup_item`, `buy_item`, and
  `sell_item`. There is **no** `equip_item`, `unequip_item`, or
  `transfer_item` pure-domain function.
- `inventory_ui.rs` registers `DropItemAction`, `TransferItemAction`, and
  `UseItemExplorationAction`. There is **no** `EquipItemAction` or
  `UnequipItemAction`.

### Identified Issues

1. No `equip_item` or `unequip_item` domain function exists anywhere.
2. `ArmorClassification` has no `Helmet` or `Boots` variant; slot routing is
   impossible without tags as a workaround.
3. Starting gear is placed directly into `Equipment` slots, bypassing
   inventory and all validation — it can never be unequipped.
4. `Character::new()` initialises AC to `0` instead of `AC_DEFAULT` (`10`).
5. No `calculate_armor_class` function exists; `ac.current` is never updated
   when equipment changes.
6. The inventory UI has no Equip or Unequip buttons; no equipment panel shows
   the player what they are wearing.
7. `Equipment::equipped_count()` omits `accessory2`; `MAX_EQUIPPED` is
   inconsistent with the seven-field struct.

---

## Implementation Phases

### Phase 1: ArmorClassification Expansion

Extend `ArmorClassification` with `Helmet` and `Boots` variants, update the
proficiency mapping, make the slot-routing match exhaustive, and migrate all
RON data files atomically. This phase must fully compile and pass all tests
before Phase 2 begins.

This phase incorporates all work described in
`docs/explanation/armor_classification_expansion_implementation_plan.md`.

#### 1.1 Add `Helmet` and `Boots` Variants

In [`src/domain/items/types.rs`](../../src/domain/items/types.rs), add two
variants to `ArmorClassification` after `Shield`:

```
/// Helmet / headgear — maps to equipment.helmet
Helmet,
/// Boots / footwear — maps to equipment.boots
Boots,
```

The `#[default]` attribute remains on `Light`. Update every doc-example that
pattern-matches on `ArmorClassification` so all doctests remain valid.

#### 1.2 Extend `proficiency_for_armor`

In [`src/domain/proficiency.rs`](../../src/domain/proficiency.rs), add the two
new arms to the exhaustive `match` in `proficiency_for_armor`:

```
ArmorClassification::Helmet => "light_armor".to_string(),
ArmorClassification::Boots  => "light_armor".to_string(),
```

Update the function's doc comment and its doctest to cover the new variants.

#### 1.3 Make `has_slot_for_item` Exhaustive

In
[`src/domain/items/equipment_validation.rs`](../../src/domain/items/equipment_validation.rs),
replace the current blanket `ItemType::Armor(_) => true` arm with an
exhaustive classification match:

```
ItemType::Armor(data) => match data.classification {
    ArmorClassification::Light
    | ArmorClassification::Medium
    | ArmorClassification::Heavy  => true,  // equipment.armor
    ArmorClassification::Shield   => true,  // equipment.shield
    ArmorClassification::Helmet   => true,  // equipment.helmet
    ArmorClassification::Boots    => true,  // equipment.boots
},
```

All arms return `true` because `has_slot_for_item` checks slot type existence,
not occupancy. The exhaustive match ensures future variants cause a compile
error rather than silently falling through.

#### 1.4 RON Data Migration

Audit the following files for `Armor` items that rely on `tags` such as
`"helmet"` or `"boots"` to indicate their slot. Replace the workaround tags
with `classification: Helmet` or `classification: Boots` inside the
`Armor(ArmorData { … })` block:

- `data/items.ron`
- `data/test_campaign/data/items.ron`
- `campaigns/tutorial/data/items.ron`

Add at minimum one `Helmet`-classified item and one `Boots`-classified item
to `data/test_campaign/data/items.ron` so Phase 2 tests can reference them.
Suggested IDs: `50` (Iron Helmet, `ac_bonus: 1`, `classification: Helmet`)
and `51` (Leather Boots, `ac_bonus: 1`, `classification: Boots`).

#### 1.5 Update SDK Validation

In [`src/sdk/validation.rs`](../../src/sdk/validation.rs), add checks:

- A character definition with `equipment.helmet = Some(id)` must reference an
  item whose `ArmorData.classification == Helmet`.
- A character definition with `equipment.boots = Some(id)` must reference an
  item whose `ArmorData.classification == Boots`.

These checks fire during campaign packager validation, not at runtime.

#### 1.6 Testing Requirements

All tests use `data/test_campaign` fixtures only (Implementation Rule 5).

- `test_armor_classification_helmet_variant_exists` — construct
  `ArmorClassification::Helmet`; assert `!= Light`.
- `test_armor_classification_boots_variant_exists` — same for `Boots`.
- `test_proficiency_for_armor_helmet_maps_to_light_armor` — assert
  `proficiency_for_armor(Helmet) == "light_armor"`.
- `test_proficiency_for_armor_boots_maps_to_light_armor` — same for `Boots`.
- `test_has_slot_for_helmet_item` — create an `Armor(Helmet)` item; assert
  `has_slot_for_item` returns `true`.
- `test_has_slot_for_boots_item` — same for `Boots`.
- `test_can_equip_helmet_succeeds` — full `can_equip_item` call with a Helmet
  item and a character with `"light_armor"` proficiency; assert `Ok(true)`.
- `test_can_equip_boots_succeeds` — same for `Boots`.
- `test_sdk_validation_helmet_in_wrong_slot_fails` — character definition
  with a `Boots` item in `equipment.helmet`; assert validation error.

#### 1.7 Deliverables

- [ ] `Helmet` and `Boots` variants added to `ArmorClassification`
- [ ] `proficiency_for_armor` extended for both new variants
- [ ] `has_slot_for_item` uses exhaustive classification match
- [ ] RON data files migrated; workaround tags removed
- [ ] Helmet and Boots test items added to `data/test_campaign/data/items.ron`
- [ ] SDK validation enforces helmet/boot slot integrity
- [ ] All nine new tests pass

#### 1.8 Success Criteria

`cargo check --all-targets --all-features` and
`cargo nextest run --all-features` pass with zero errors and zero warnings. The
two new variants are fully recognized by the type system, proficiency layer,
slot-routing code, and SDK validation.

---

### Phase 2: Domain Transaction Functions and AC Calculation

Add the pure-domain functions that perform the equip and unequip mutations,
compute armor class from all equipped slots, and fix the `Equipment` struct
inconsistencies. No Bevy or I/O dependencies are introduced in this phase.

#### 2.1 Add `EquipmentSlot` Enum

Add to [`src/domain/character.rs`](../../src/domain/character.rs), alongside
the `Equipment` struct:

```rust
pub enum EquipmentSlot {
    Weapon,
    Armor,
    Shield,
    Helmet,
    Boots,
    Accessory1,
    Accessory2,
}
```

Add three methods to `EquipmentSlot`:

- `pub fn for_item(item: &Item) -> Option<Self>` — derives the correct
  target slot from the item's `ItemType` and `ArmorClassification`:

  | `ItemType` | `ArmorClassification` / notes | Target slot |
  |---|---|---|
  | `Weapon(_)` | any | `Weapon` |
  | `Armor(data)` | `Light / Medium / Heavy` | `Armor` |
  | `Armor(data)` | `Shield` | `Shield` |
  | `Armor(data)` | `Helmet` | `Helmet` |
  | `Armor(data)` | `Boots` | `Boots` |
  | `Accessory(_)` | `accessory1` empty | `Accessory1` |
  | `Accessory(_)` | `accessory1` occupied | `Accessory2` |
  | `Consumable / Ammo / Quest` | — | `None` |

  The `Accessory` branch requires a `&Equipment` parameter to check slot
  occupancy; adjust the signature to
  `for_item(item: &Item, equipment: &Equipment) -> Option<Self>`.

- `pub fn get(&self, equipment: &Equipment) -> Option<ItemId>` — reads the
  current `Option<ItemId>` from the named slot.

- `pub fn set(&self, equipment: &mut Equipment, value: Option<ItemId>)` —
  writes an `Option<ItemId>` to the named slot.

#### 2.2 Fix `Equipment` Inconsistencies

In [`src/domain/character.rs`](../../src/domain/character.rs):

- Change `MAX_EQUIPPED` from `6` to `7` to match the seven struct fields.
- Fix `equipped_count()` to include `accessory2` in its counted slice:
  `[weapon, armor, shield, helmet, boots, accessory1, accessory2]`.

Update the existing `test_equipment_count` test to assert
`equipped_count() == 7` when all slots are filled.

#### 2.3 Add `calculate_armor_class`

Add to
[`src/domain/items/equipment_validation.rs`](../../src/domain/items/equipment_validation.rs):

```rust
pub fn calculate_armor_class(
    equipment: &Equipment,
    item_db: &ItemDatabase,
) -> u8
```

Logic:
1. Start accumulator at `AC_DEFAULT` (10).
2. For each of `equipment.armor`, `equipment.shield`, `equipment.helmet`,
   `equipment.boots`: if `Some(item_id)` and the item exists and is
   `ItemType::Armor(data)`, add `data.ac_bonus` to the accumulator.
3. Clamp the result to `[AC_MIN, AC_MAX]` (0–30) and return.

Note: `equipment.weapon` and accessories are not consulted here. Accessory
`BonusAttribute::ArmorClass` bonuses via `constant_bonus` are out of scope
for this plan.

Re-export `calculate_armor_class` from `src/domain/items/mod.rs`.

#### 2.4 Fix `Character::new()` AC Initialisation

In [`src/domain/character.rs`](../../src/domain/character.rs), change:

```
ac: AttributePair::new(0),
```

to:

```
ac: AttributePair::new(AC_DEFAULT),
```

Update any test that asserts `character.ac.base == 0` or
`character.ac.current == 0` for a freshly-created character.

#### 2.5 Add `equip_item` to `transactions.rs`

Add to [`src/domain/transactions.rs`](../../src/domain/transactions.rs):

```rust
pub fn equip_item(
    character: &mut Character,
    inventory_slot_index: usize,
    item_db: &ItemDatabase,
    classes: &ClassDatabase,
    races: &RaceDatabase,
) -> Result<(), EquipError>
```

Algorithm (all steps must be atomic — if any step fails, the character state
is unchanged):

1. Bounds-check `inventory_slot_index` against `character.inventory.items`;
   return `EquipError::ItemNotFound` if out of range.
2. Read `item_id` from `character.inventory.items[inventory_slot_index]`.
3. Call `can_equip_item(character, item_id, item_db, classes, races)`;
   propagate any `EquipError`.
4. Look up the `Item` via `item_db.get_item(item_id)` (cannot fail here
   because `can_equip_item` already verified it).
5. Determine `target_slot` via `EquipmentSlot::for_item(item, &character.equipment)`;
   return `EquipError::NoSlotAvailable` if `None`.
6. Save `displaced_id = target_slot.get(&character.equipment)`.
7. Remove the item from `character.inventory.items` at `inventory_slot_index`
   (saves the `InventorySlot`).
8. `target_slot.set(&mut character.equipment, Some(item_id))`.
9. If `displaced_id` is `Some(old_id)`, call
   `character.inventory.add_item(old_id, 0)`. Because a slot was just freed
   in step 7, `add_item` will succeed; on the improbable failure (e.g.,
   `MAX_ITEMS` was already at its limit due to a race condition in future
   async code), re-insert the new item back into the equipment slot, put
   `old_id` back in equipment, and re-add the new item to inventory — or
   simply return `EquipError::NoSlotAvailable`. The safe default is: if
   `add_item` fails, reverse all mutations and return `NoSlotAvailable`.
10. If the equipped item is `ItemType::Armor`, recalculate
    `character.ac.current = calculate_armor_class(&character.equipment, item_db)`.
11. Return `Ok(())`.

Add `EquipError` to `TransactionError` via a new variant
`TransactionError::EquipFailed(EquipError)` so callers that use
`TransactionError` throughout (like the game systems) can wrap the result
uniformly. Alternatively keep `equip_item` returning `Result<(), EquipError>`
directly and let callers convert — choose the approach consistent with
existing error propagation patterns in `transactions.rs`.

#### 2.6 Add `unequip_item` to `transactions.rs`

Add to [`src/domain/transactions.rs`](../../src/domain/transactions.rs):

```rust
pub fn unequip_item(
    character: &mut Character,
    slot: EquipmentSlot,
    item_db: &ItemDatabase,
) -> Result<(), TransactionError>
```

Algorithm:

1. Read `item_id = slot.get(&character.equipment)`; return `Ok(())` silently
   if `None` (nothing to unequip).
2. Check `!character.inventory.is_full()`; return
   `TransactionError::InventoryFull` if full.
3. `slot.set(&mut character.equipment, None)`.
4. `character.inventory.add_item(item_id, 0)`.
5. If the unequipped item is `ItemType::Armor`, recalculate
   `character.ac.current = calculate_armor_class(&character.equipment, item_db)`.
6. Return `Ok(())`.

`item_db` is required in step 5 to determine the item type. If an item_id
does not exist in `item_db`, skip the AC recalculation step (do not panic).

#### 2.7 Testing Requirements

All tests use `data/test_campaign` fixtures.

**`calculate_armor_class` tests** (in `equipment_validation.rs`):

- `test_calculate_ac_no_armor` — empty `Equipment`; assert result `== 10`.
- `test_calculate_ac_body_armor_only` — equip +4 leather (body); assert `14`.
- `test_calculate_ac_all_slots` — equip +4 body, +2 shield, +1 helmet, +1
  boots; assert `18`.
- `test_calculate_ac_clamps_to_max` — equip items whose bonuses total > 30;
  assert result `<= 30`.
- `test_calculate_ac_missing_item_id_skips_slot` — set `equipment.helmet` to
  a non-existent item_id; assert no panic and contribution is `0`.

**`equip_item` tests** (in `transactions.rs`):

- `test_equip_item_weapon_moves_from_inventory_to_slot` — character has sword
  in slot 0; call `equip_item`; assert `equipment.weapon == Some(sword_id)`
  and slot 0 is removed from inventory.
- `test_equip_item_swaps_old_weapon_back_to_inventory` — character already
  has weapon equipped; equip a second weapon from inventory; assert old weapon
  appears in inventory and new weapon is in the slot.
- `test_equip_item_armor_updates_ac` — equip +5 chain mail; assert
  `character.ac.current == 15` (10 + 5).
- `test_equip_item_helmet_routes_to_helmet_slot` — equip a Helmet item;
  assert `equipment.helmet` is set and `equipment.armor` is unchanged.
- `test_equip_item_boots_routes_to_boots_slot` — equip a Boots item; assert
  `equipment.boots` is set.
- `test_equip_item_invalid_class_returns_error` — character class lacks
  required proficiency; assert `EquipError::ClassRestriction`.
- `test_equip_item_out_of_bounds_returns_error` — `inventory_slot_index`
  exceeds inventory length; assert `EquipError::ItemNotFound`.
- `test_equip_item_non_equipable_item_returns_error` — attempt to equip a
  `Consumable` item; assert `EquipError::NoSlotAvailable`.

**`unequip_item` tests** (in `transactions.rs`):

- `test_unequip_item_moves_to_inventory` — equip weapon; unequip; assert
  `equipment.weapon == None` and item appears in inventory.
- `test_unequip_item_reduces_ac` — equip armor; verify AC; unequip; assert
  `character.ac.current == 10`.
- `test_unequip_item_empty_slot_is_noop` — call `unequip_item` on an already
  empty slot; assert `Ok(())` and inventory unchanged.
- `test_unequip_item_inventory_full_returns_error` — fill inventory to
  capacity; attempt unequip; assert
  `TransactionError::InventoryFull`.

#### 2.8 Deliverables

- [ ] `EquipmentSlot` enum with `for_item`, `get`, `set` implemented
- [ ] `Equipment::equipped_count()` fixed to include `accessory2`
- [ ] `Equipment::MAX_EQUIPPED` corrected to `7`
- [ ] `calculate_armor_class(equipment, item_db) -> u8` implemented and
      re-exported from `src/domain/items/mod.rs`
- [ ] `Character::new()` initialises `ac` to `AC_DEFAULT` (10)
- [ ] `equip_item` implemented in `transactions.rs`
- [ ] `unequip_item` implemented in `transactions.rs`
- [ ] All 17 new tests pass

#### 2.9 Success Criteria

A character equipped with items in all four armor-bearing slots has
`ac.current` equal to `AC_DEFAULT` plus the sum of all `ac_bonus` values,
clamped to `[0, 30]`. An unarmed, unarmored character has `ac.current == 10`.
Equipping a weapon moves it from inventory to `equipment.weapon`; unequipping
returns it to inventory. All tests pass with zero warnings.

---

### Phase 3: Starting Equipment in Inventory

Replace the direct-copy `create_starting_equipment` pattern with a flow that
adds starting gear to inventory first and then equips it via the domain
functions introduced in Phase 2. This ensures starting equipment is visible in
inventory, can be unequipped at runtime, and passes all equip validation.

#### 3.1 Change `CharacterDefinition::instantiate`

In
[`src/domain/character_definition.rs`](../../src/domain/character_definition.rs),
modify the `instantiate` function as follows:

**Remove** the call to `create_starting_equipment(&self.starting_equipment)`
and the direct assignment of an `Equipment` struct.

**Replace** with the following two-pass approach after the existing
`populate_starting_inventory` call:

**Pass 1 — add starting equipment items to inventory:**
For each `Some(item_id)` in `self.starting_equipment` (iterate over all seven
fields), add the item to the character's inventory using the same
`add_item(item_id, 0)` pattern as `populate_starting_inventory`. Return
`CharacterDefinitionError::InventoryFull` if the inventory has no room.

**Pass 2 — equip from inventory:**
For each item added in Pass 1, find its inventory slot index and call
`equip_item(&mut character, slot_index, item_db, classes, races)`.
Map `EquipError` to a new `CharacterDefinitionError::InvalidStartingEquipment`
variant (see §3.2).

After all equips are applied, call `calculate_armor_class` once and assign
the result to `character.ac.current` so AC is correct from the first frame.

**Delete** the private `create_starting_equipment` helper function.

#### 3.2 Add `CharacterDefinitionError::InvalidStartingEquipment`

Add a new variant to `CharacterDefinitionError` in
[`src/domain/character_definition.rs`](../../src/domain/character_definition.rs):

```rust
InvalidStartingEquipment {
    item_id: ItemId,
    reason: String,
}
```

This variant is returned when an item listed in `starting_equipment` cannot
be equipped due to class restriction, race restriction, or other
`EquipError`. Campaign authors will see this error during validation.

#### 3.3 Data File Conventions

**Important — avoid duplicate items:** `starting_items` holds items the
character carries in their bag at game start (unequipped). `starting_equipment`
holds items the character wears at game start. An item ID must **not** appear
in both lists; doing so would grant the character two copies.

Audit `data/test_campaign/data/characters.ron` and
`campaigns/tutorial/data/characters.ron` to confirm no item ID appears in
both `starting_items` and any `starting_equipment` field for the same
character. If duplicates exist, remove the item from `starting_items` (the
`starting_equipment` entry is authoritative for equipped items).

#### 3.4 Testing Requirements

All tests use `data/test_campaign` fixtures.

- `test_instantiate_starting_weapon_in_equipment_slot` — character definition
  with `starting_equipment.weapon = Some(4)`; call `instantiate`; assert
  `character.equipment.weapon == Some(4)` and the weapon is NOT in
  `character.inventory`.
- `test_instantiate_starting_weapon_equippable_then_unequippable` — after
  `instantiate`, call `unequip_item(Weapon, ...)`; assert weapon moves to
  inventory and `equipment.weapon == None`.
- `test_instantiate_starting_armor_updates_ac` — character definition with
  `starting_equipment.armor = Some(20)` (Leather Armor, `ac_bonus: 2`);
  assert `character.ac.current == 12`.
- `test_instantiate_no_starting_equipment_ac_is_default` — character with
  empty `starting_equipment`; assert `character.ac.current == 10`.
- `test_instantiate_invalid_starting_equipment_returns_error` — character
  definition whose `starting_equipment.weapon` references an item the
  character's class cannot use; assert
  `CharacterDefinitionError::InvalidStartingEquipment`.

#### 3.5 Deliverables

- [ ] `CharacterDefinition::instantiate` uses two-pass add-then-equip flow
- [ ] `create_starting_equipment` helper removed
- [ ] `CharacterDefinitionError::InvalidStartingEquipment` variant added
- [ ] `data/test_campaign/data/characters.ron` audited for duplicate IDs
- [ ] `campaigns/tutorial/data/characters.ron` audited for duplicate IDs
- [ ] All five new tests pass

#### 3.6 Success Criteria

A freshly instantiated character with starting equipment has all starting
items in equipment slots (not inventory), a correct `ac.current`, and can
unequip any starting item to their inventory. Passing an invalid starting
equipment definition returns a descriptive error rather than silently
bypassing class/race restrictions.

---

### Phase 4: Inventory UI — Equip and Unequip

Add `EquipItemAction` and `UnequipItemAction` message types, wire them into
the existing inventory action system, add an Equip button to the item action
strip, and add an equipment display panel so the player can see and unequip
worn items.

#### 4.1 New Message Types

Add to
[`src/game/systems/inventory_ui.rs`](../../src/game/systems/inventory_ui.rs):

```rust
#[derive(Message)]
pub struct EquipItemAction {
    /// Party member index (0-based) whose inventory contains the item.
    pub party_index: usize,
    /// Slot index in that character's inventory to equip.
    pub slot_index: usize,
}

#[derive(Message)]
pub struct UnequipItemAction {
    /// Party member index (0-based) wearing the item.
    pub party_index: usize,
    /// Which equipment slot to clear.
    pub slot: EquipmentSlot,
}
```

Register both in `InventoryPlugin::build` alongside the existing messages.

#### 4.2 Equip Button in the Action Strip

In `build_action_list` (the helper that assembles the ordered action list for
keyboard navigation), add an `Equip` entry when the selected inventory item is
equipable. An item is equipable if
`EquipmentSlot::for_item(item, &character.equipment)` returns `Some(_)` — i.e.
it is a `Weapon`, `Armor`, `Accessory`, or any item type with a valid slot.

Button order in the rendered action strip:

| Index | Label | Color | Condition |
|---|---|---|---|
| 0 | Equip | Green | Item is equipable |
| 0 or 1 | Use | Blue | Item is `Consumable` |
| 1 or 2 | Drop | Red | Always present |
| 2+ | → Name | Green | One per other open panel |

Add keyboard shortcut: pressing **E** while in `SlotNavigation` phase directly
dispatches `EquipItemAction` for the currently selected equipable item,
bypassing `ActionNavigation` (mirrors the existing **U** shortcut for Use).

#### 4.3 Equipment Display Panel

Add a collapsible equipment strip to each character panel in the inventory UI,
rendered between the character name header and the inventory slot grid. The
strip shows all seven equipment slots in two rows:

**Row 1** (weapons / body armor): Weapon · Armor · Shield
**Row 2** (accessories / extremities): Helmet · Boots · Ring · Ring

Each cell shows:
- Slot name label in small dimmed text.
- Item name (truncated to fit) when occupied, or `—` when empty.
- A highlight border matching the panel focus colour when the cell is selected.

Selecting an equipment cell (via mouse click or arrow navigation within the
strip) shows a single **Unequip** action button below the strip. Pressing
**Enter** on the focused equipment cell dispatches `UnequipItemAction` for
that slot.

The equipment strip is part of the same `render_character_panel` function.
Add a `selected_equip_slot: Option<EquipmentSlot>` field to
`InventoryNavigationState` to track which equipment cell is focused.

Navigation between the inventory grid and the equipment strip: pressing **Up**
from slot row 0 of the inventory grid moves focus to the equipment strip;
pressing **Down** from the equipment strip returns to slot row 0.

#### 4.4 Extend `inventory_action_system`

Add handlers for the two new message types in `inventory_action_system`:

**`EquipItemAction` handler:**
1. Bounds-check `party_index` and `slot_index` → `warn!` and skip on failure.
2. Acquire references to `item_db`, `classes`, and `races` from
   `GameContent`.
3. Call `equip_item(&mut party.members[party_index], slot_index, ...)`.
4. On `Ok`: clear `selected_slot`, reset `nav_state` phase to
   `SlotNavigation`, request repaint.
5. On `Err`: write a `GameLog` error entry with a human-readable message
   (e.g. "Cannot equip: class restriction"); do not panic.

**`UnequipItemAction` handler:**
1. Bounds-check `party_index` → `warn!` and skip.
2. Call `unequip_item(&mut party.members[party_index], slot, item_db)`.
3. On `Ok`: clear `selected_equip_slot`, request repaint.
4. On `Err` (`InventoryFull`): write `GameLog` message "Cannot unequip:
   inventory is full."

#### 4.5 Testing Requirements

All UI tests use the egui harness pattern established in the existing
`ui_helpers.rs` and `inventory_ui.rs` test modules.

- `test_equip_action_dispatched_on_e_key` — construct a navigation state with
  an equipable item selected in slot 0; simulate pressing E; assert
  `EquipItemAction { party_index: 0, slot_index: 0 }` is in the message queue.
- `test_equip_button_absent_for_consumable` — select a consumable item;
  assert no "Equip" entry in the action list.
- `test_equip_button_present_for_weapon` — select a weapon item; assert
  "Equip" is the first entry in the action list.
- `test_equip_action_system_moves_item_to_slot` — send `EquipItemAction`;
  assert `equipment.weapon` is set and inventory slot is removed.
- `test_unequip_action_system_returns_item_to_inventory` — send
  `UnequipItemAction { slot: EquipmentSlot::Weapon }`; assert weapon slot is
  cleared and item appears in inventory.
- `test_unequip_action_system_inventory_full_logs_error` — fill inventory,
  send `UnequipItemAction`; assert `GameLog` contains the "inventory is full"
  message and equipment slot is unchanged.
- `test_equipment_strip_shows_equipped_item_name` — build a character with a
  weapon equipped; render the character panel; assert the weapon name appears
  in the equipment strip cell.

#### 4.6 Deliverables

- [ ] `EquipItemAction` and `UnequipItemAction` message types added and
      registered
- [ ] `build_action_list` adds Equip button for equipable items
- [ ] **E** key shortcut dispatches `EquipItemAction` in `SlotNavigation`
- [ ] Equipment strip rendered in each character panel
- [ ] `selected_equip_slot` added to `InventoryNavigationState`
- [ ] Up/Down navigation between inventory grid and equipment strip
- [ ] `inventory_action_system` handles both new messages with error logging
- [ ] All seven new tests pass

#### 4.7 Success Criteria

A player can select a weapon in their inventory, press **E** or click Equip,
and see it appear in the equipment strip. Selecting a slot in the equipment
strip and pressing Enter (or clicking Unequip) returns the item to inventory.
Attempting to equip an item that violates class or race restrictions shows an
in-game error message. Attempting to unequip when inventory is full shows a
"inventory is full" message with no state mutation.

---

### Phase 5: Documentation and Final Validation

#### 5.1 Update `docs/explanation/implementations.md`

Add a section covering:
- `ArmorClassification` expansion: variants added, proficiency mapping,
  slot routing changes, data files migrated.
- New domain functions: `equip_item`, `unequip_item`,
  `calculate_armor_class`, `EquipmentSlot`.
- Starting equipment flow change: two-pass add-then-equip, removal of
  `create_starting_equipment`.
- Inventory UI additions: Equip/Unequip actions, equipment strip.
- Bug fixes: `Character::new()` AC initialisation, `equipped_count()` /
  `MAX_EQUIPPED` inconsistency.

#### 5.2 Exhaustive Test Coverage Audit

Verify that all existing tests for `ArmorClassification`, `can_equip_item`,
`proficiency_for_armor`, and `CharacterDefinition::instantiate` cover the
new `Helmet` and `Boots` variants and the new equip/unequip paths. Add
missing cases where gaps are found. Priority targets:

- `src/domain/items/types.rs` — `required_proficiency` tests
- `src/domain/items/equipment_validation.rs` — `can_equip_item` tests
- `src/domain/proficiency.rs` — `proficiency_for_armor` tests
- `src/domain/character_definition.rs` — `instantiate` tests

#### 5.3 Final Quality Gate Run

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

All four must produce zero errors and zero warnings.

#### 5.4 Deliverables

- [ ] `docs/explanation/implementations.md` updated
- [ ] Exhaustive test coverage for new variants and new functions confirmed
- [ ] All four quality gates pass with zero errors and zero warnings

---

## Sequence Summary

| Phase | Core Output | Key Files Modified |
|---|---|---|
| 1 | `Helmet`/`Boots` variants, proficiency mapping, exhaustive slot routing, RON migration | `src/domain/items/types.rs`, `src/domain/proficiency.rs`, `src/domain/items/equipment_validation.rs`, `src/sdk/validation.rs`, `data/*.ron` |
| 2 | `EquipmentSlot`, `equip_item`, `unequip_item`, `calculate_armor_class`, fix AC init and `equipped_count` | `src/domain/character.rs`, `src/domain/transactions.rs`, `src/domain/items/equipment_validation.rs` |
| 3 | Two-pass starting equipment flow, `CharacterDefinitionError::InvalidStartingEquipment`, data audit | `src/domain/character_definition.rs`, `data/test_campaign/data/characters.ron`, `campaigns/tutorial/data/characters.ron` |
| 4 | `EquipItemAction`, `UnequipItemAction`, Equip button, equipment strip, **E** shortcut | `src/game/systems/inventory_ui.rs` |
| 5 | Implementations doc, exhaustive coverage audit, final quality gates | `docs/explanation/implementations.md` |

---

## Architecture Compliance Checklist

- [ ] `ArmorClassification` match arms are exhaustive — no `_` wildcard
- [ ] `EquipmentSlot::for_item` match is exhaustive over all `ArmorClassification`
      variants
- [ ] Proficiency IDs (`"light_armor"`, `"medium_armor"`, etc.) are string
      values from `ProficiencyDatabase`, never hard-coded in callers
- [ ] RON data files use `classification: Helmet` / `Boots`, not tag-based
      workarounds
- [ ] No tests reference `campaigns/tutorial` — all use `data/test_campaign`
      (Implementation Rule 5)
- [ ] Helmet and Boots test fixture items exist in
      `data/test_campaign/data/items.ron`
- [ ] `equip_item` and `unequip_item` are pure-domain functions in
      `transactions.rs` with no Bevy dependencies
- [ ] `calculate_armor_class` only reads `equipment` and `item_db` — no
      mutable state
- [ ] AC recalculation is triggered inside `equip_item` and `unequip_item`,
      never in callers
- [ ] Starting equipment items are added to inventory and equip-validated via
      `equip_item` — `create_starting_equipment` is deleted
- [ ] `EquipItemAction` and `UnequipItemAction` follow the existing message
      pattern (`#[derive(Message)]`, registered in `InventoryPlugin::build`)
- [ ] All new public functions have `///` doc comments with runnable doctests
- [ ] SPDX `FileCopyrightText` and `License-Identifier` headers present in
      all modified `.rs` files
- [ ] `docs/explanation/implementations.md` updated after Phase 5 completion
