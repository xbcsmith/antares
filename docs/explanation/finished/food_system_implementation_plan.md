# Food System Implementation Plan

## Overview

Convert the game's food system from an isolated numeric counter (`Party.food` and `Character.food`) to an inventory-based item system. Instead of the party starting with a fixed amount of invisible food, "Food Ration" (and other types of food) becomes a standard consumable item. When characters rest, the game will search their inventories for food items and consume them. Merchants and Innkeepers will be able to sell food items via standard inventory stock.

## Current State Analysis

### Existing Infrastructure

- **Food Counters**: `Character.food` (u8) tracks current food per character. `party.food` (u32) exists but is deprecated in favor of individual food tracking.
- **Rest System**: `food_needed_to_rest(party)` requires 1 ration per living member. `consume_food(party, amount)` deducts directly from the counters.
- **Characters**: Start with `starting_food` defined in their campaign definitions/initialization.
- **Item System**: `ItemType::Consumable(ConsumableData)` exists with `ConsumableEffect`. Items are loaded via `ItemDatabase`.
- **Merchants**: Use `MerchantStock` populated from stock templates (defined in campaign data).

### Identified Issues

- Food is an abstract number, separating it from the game's economy and inventory mechanics.
- There is no way to obtain more food because it is not an item; merchants cannot sell it.
- A party needs food to rest, but cannot acquire it, leading to eventual permanent starvation.

## Implementation Phases

### Phase 1: Core Item Foundation

Add food to the item system so it can exist in the world and be managed in inventories.

#### 1.1 Foundation Work

**[MODIFY] `src/domain/items/types.rs`**
1. Add a new `IsFood(u8)` variant to `ConsumableEffect` representing the number of rations this item provides (almost always 1).

**[CREATE/MODIFY] `data/items.ron` (or campaign equivalent)**
1. Create a "Food Ration" item definition in the default `ItemDatabase` utilizing the new `ConsumableEffect::IsFood(1)`.

#### 1.2 Testing Requirements

- Unit test verifying `ItemType::Consumable` with `IsFood` serializes/deserializes correctly.
- Ensure `ItemDatabase` loads without errors with the new standard food items included.

#### 1.3 Deliverables

- [ ] `ConsumableEffect::IsFood` variant added
- [ ] Base food items added to `items.ron`
- [ ] Serialization tests passed

#### 1.4 Success Criteria

Food rationing is expressible as standard consumable items in the `ItemDatabase`.

### Phase 2: Rest System Migration

Update the rest system and resource managers to consume items of type `IsFood` from character inventories instead of decrementing abstract numeric counters.

#### 2.1 Feature Work

**[MODIFY] `src/domain/resources.rs`**
1. Update `food_needed_to_rest(party)` to remain exactly as is: 1 ration required per living member.
2. Rewrite `consume_food(party, amount_per_member)`. Instead of `character.food -= amount`, it must iterate through `character.inventory`, find items where `effect == ConsumableEffect::IsFood`, and decrement their quantity via `inventory.remove_item()`.
3. Handle pooling/sharing: If `Character A` has no food items but `Character B` has extra, `B`'s inventory should provide the food for `A`.
4. Rewrite `check_starvation(party)`. It should sum all `IsFood` values across all party members' inventories.

**[MODIFY] `src/domain/character.rs` & `src/domain/character_definition.rs`**
1. Deprecate and remove `food` from `Character` and `Party`.
2. Modify character initialization: instead of setting `character.food = starting_food`, grant the `starting_food` quantity of the "Food Ration" item directly into the character's `Inventory`.

**[MODIFY] `src/application/save_game.rs` and `mod.rs`**
1. Remove all legacy forced assignments of food (e.g., `state.party.food = 20;`).

#### 2.2 Testing requirements

- Replace `test_consume_food` with tests that initialize a party, add `IsFood` items to inventories, call `consume_food()`, and assert items were removed from the correct inventory slots.
- Test edge cases where one character feeds another.

#### 2.3 Deliverables

- [ ] `consume_food()` rewritten to use inventory items
- [ ] `Character.food` numeric field removed
- [ ] Initial character setup yields food items
- [ ] `consume_food` tests updated

#### 2.4 Success Criteria

Resting consumes food items from the party's inventories. Party without food items is correctly blocked from resting.

### Phase 3: Merchant and Innkeeper Integration

Make food purchasable in the world.

#### 3.1 Configuration Updates

**[MODIFY] Data Files (`data/` or campaign data)**
1. Update `general_goods` and `innkeeper` stock templates to include the "Food Ration" `ItemId` with standard quantities and pricing.
2. Add "Food Ration" to drop tables if appropriate.

#### 3.2 Testing requirements

- Verify that loading a stock template with the food item correctly populates a merchant's inventory.

#### 3.3 Deliverables

- [ ] Merchant stock templates updated with food items

#### 3.4 Success Criteria

Players can interact with merchants and natively buy food rations into their inventory using gold.

### Phase 4: UI and SDK Editor Updates

Ensure the map/campaign builder supports the new food items.

#### 4.1 Feature Work

**[MODIFY] `sdk/campaign_builder/src/`**
1. Check `Items Editor` to ensure the new `IsFood` variant displays correctly in dropdowns when editing `ConsumableEffect`.
2. Add a `ration_value: u8` field to the UI when `IsFood` is selected.

#### 4.2 Deliverables

- [ ] SDK Items Editor supports `IsFood` variant

#### 4.3 Success Criteria

Campaign developers can create novel food items (e.g., "Elven Bread", "Roast Beef") in the Items Editor.
