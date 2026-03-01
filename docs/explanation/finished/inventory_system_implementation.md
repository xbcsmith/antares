<!-- SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Inventory System Implementation Plan

## Overview

This plan defines the full implementation of a unified inventory system used by
both Characters and NPCs (Merchants, Priests, Innkeepers, and other future role
types). The work spans the domain layer, application layer, game systems, SDK
tooling, data files, and tests.

The approach is:

1. Build one shared inventory ownership model in `src/domain/` that both
   Characters and NPCs compose.
2. Add role-specific merchant, priest, and innkeeper overlays on top of that
   shared core using composition, not duplication.
3. Wire explicit buy/sell/service transaction operations through the application
   and dialogue layers rather than ad-hoc mutations.
4. Extend data schemas, SDK validation, and save/load to reflect the new state.
5. Preserve all existing inn party-management behavior unless explicitly changed.

---

## Current State Analysis

### Existing Infrastructure

The following structures, modules, and files are already in place and MUST be
used as the foundation. Do NOT rewrite them; extend them.

#### Domain Layer (`src/domain/`)

| File                           | Relevant Content                                                                                                                                          |
| ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/character.rs`      | `Inventory` (MAX_ITEMS=6), `InventorySlot`, `Equipment` (MAX_EQUIPPED=7), `Character` struct with `inventory`, `equipment`, `gold`, `gems`, `food` fields |
| `src/domain/items/types.rs`    | `Item`, `ItemType`, `WeaponData`, `ArmorData`, `ConsumableData`, etc.                                                                                     |
| `src/domain/items/database.rs` | `ItemDatabase` - loads from RON, queries by `ItemId` (u8)                                                                                                 |
| `src/domain/types.rs`          | `ItemId = u8`, `InnkeeperId = String`, `CharacterId = usize`, `EventId = u16`                                                                             |
| `src/domain/world/npc.rs`      | `NpcDefinition` with `is_merchant: bool`, `is_innkeeper: bool`, `is_priest` is MISSING                                                                    |
| `src/domain/world/events.rs`   | `EventResult::EnterInn`, `EventResult::NpcDialogue`                                                                                                       |
| `src/domain/dialogue.rs`       | `DialogueAction::GiveItems`, `DialogueAction::TakeItems`, `DialogueAction::GiveGold`, `DialogueAction::TakeGold`                                          |

#### Application Layer (`src/application/`)

| File                           | Relevant Content                                                        |
| ------------------------------ | ----------------------------------------------------------------------- |
| `src/application/mod.rs`       | `GameState` with `party: Party`, `roster: Roster`; `InnManagementState` |
| `src/application/save_game.rs` | `SaveGame` serializes `GameState`; no NPC runtime state serialized yet  |
| `src/application/dialogue.rs`  | `DialogueState`, `DialogueAction` execution path                        |

#### Game Systems (`src/game/systems/`)

| File                           | Relevant Content                                                                                                                                           |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/game/systems/dialogue.rs` | `execute_action()` dispatches `DialogueAction` variants; currently handles `GiveItems`/`TakeItems`/`GiveGold`/`TakeGold` without inventory-owner targeting |
| `src/game/systems/events.rs`   | `handle_events()` dispatches `EventResult::EnterInn` and `EventResult::NpcDialogue`                                                                        |
| `src/game/systems/inn_ui.rs`   | Existing inn party-management UI (MUST be preserved)                                                                                                       |

#### SDK (`src/sdk/`)

| File                    | Relevant Content                                                                      |
| ----------------------- | ------------------------------------------------------------------------------------- |
| `src/sdk/database.rs`   | `NpcDatabase` with `merchants()`, `innkeepers()` filters; `ContentDatabase`           |
| `src/sdk/validation.rs` | `Validator` with `validate_innkeepers()`, `ValidationError::InnkeeperMissingDialogue` |

#### Data Files

| File                               | Description                                      |
| ---------------------------------- | ------------------------------------------------ |
| `data/npcs.ron`                    | Core NPC archetypes; no stock/service fields yet |
| `campaigns/tutorial/data/npcs.ron` | Tutorial NPCs; no stock/service fields yet       |
| `data/items.ron`                   | Item definitions (ItemId = u8)                   |

#### Existing Tests

| File                                                   | Description                      |
| ------------------------------------------------------ | -------------------------------- |
| `tests/innkeeper_party_management_integration_test.rs` | Inn party management integration |
| `tests/npc_dialogue_integration_test.rs`               | NPC dialogue integration         |

### Identified Issues

The following gaps exist in the current codebase that this plan must address:

1. **No merchant stock model.** `NpcDefinition` has `is_merchant: bool` but no
   fields for what items the merchant sells, their prices, or their currency
   reserves.

2. **No priest role or service catalog.** `NpcDefinition` has no `is_priest`
   flag and no service catalog concept (healing, condition curing, restoration).

3. **No innkeeper service catalog.** `InnManagementState` handles party roster
   only. Paid services (lodging costs, healing costs) have no representation.

4. **No shared inventory ownership abstraction.** `Inventory` and `Equipment`
   exist only on `Character`. NPCs have no inventory fields to hold items for
   sale or quest rewards.

5. **No transaction operations.** There are no `buy()`, `sell()`,
   `consume_service()` functions in the domain or application layer. Item
   transfers currently happen only through `DialogueAction::GiveItems` /
   `TakeItems` without any party-gold check, capacity check, or ownership
   targeting.

6. **No character targeting for item delivery.** `DialogueAction::GiveItems`
   gives items to the party without specifying which character receives them.
   Buy transactions must deliver the item to a selected party member.

7. **No NPC runtime state in save/load.** `SaveGame` serializes `GameState`
   which does not include NPC runtime inventory or service consumption records.

8. **No SDK/validation coverage for merchant or service fields.** `Validator`
   checks innkeeper dialogue but does not validate merchant stock item IDs,
   service catalog entries, or economy settings.

---

## Implementation Phases

### Phase 1: Shared Inventory Domain Model

Establish the shared ownership primitives that both Characters and NPCs will
compose. All new types must be placed in `src/domain/` and must use the type
aliases defined in `src/domain/types.rs` (`ItemId`, `CharacterId`, etc.).

#### 1.1 Add `is_priest` Field to `NpcDefinition`

**File:** `src/domain/world/npc.rs`

Add `pub is_priest: bool` to `NpcDefinition` alongside the existing
`is_merchant` and `is_innkeeper` fields. Annotate with `#[serde(default)]` to
preserve backward compatibility with existing RON files. Update the `merchant()`
and `innkeeper()` constructor methods to set `is_priest: false`. Add a new
`priest()` constructor method following the same pattern.

Update `NpcDatabase` in `src/sdk/database.rs`:

- Add `pub fn priests(&self) -> Vec<&NpcDefinition>` filtering on `is_priest`.

**Validation:** `cargo check` must pass. Existing NPC RON files must deserialize
without errors. `test_npc_definition_merchant` and
`test_npc_definition_innkeeper` must still pass.

#### 1.2 Create `src/domain/inventory.rs` - Shared Inventory Ownership Primitives

Create a new file `src/domain/inventory.rs`. Add it to `src/domain/mod.rs` with
`pub mod inventory;`.

Define the following types in this file, all with SPDX copyright header:

```
InventoryOwner - enum with variants: Character(CharacterId), Npc(NpcId)
```

Define `StockEntry`:

- `pub item_id: ItemId`
- `pub quantity: u8` (0 = sold out)
- `pub override_price: Option<u32>` (None = use item base_cost from ItemDatabase)

Define `MerchantStock`:

- `pub entries: Vec<StockEntry>` - current mutable runtime stock
- `pub restock_template: Option<String>` - optional template ID for restocking
- Implement `pub fn get_entry(&self, item_id: ItemId) -> Option<&StockEntry>`
- Implement `pub fn get_entry_mut(&mut self, item_id: ItemId) -> Option<&mut StockEntry>`
- Implement `pub fn decrement(&mut self, item_id: ItemId) -> bool` (returns false if out of stock)
- Implement `pub fn effective_price(&self, item_id: ItemId, base_cost: u32) -> u32`

Define `ServiceEntry`:

- `pub service_id: String` (e.g., `"heal_all"`, `"cure_poison"`, `"rest"`)
- `pub cost: u32` (in gold)
- `pub gem_cost: u32` (default 0)
- `pub description: String`

Define `ServiceCatalog`:

- `pub services: Vec<ServiceEntry>`
- Implement `pub fn get_service(&self, service_id: &str) -> Option<&ServiceEntry>`
- Implement `pub fn has_service(&self, service_id: &str) -> bool`

Define `NpcEconomySettings`:

- `pub buy_rate: f32` (multiplier applied to item base_cost when NPC buys from player; default 0.5)
- `pub sell_rate: f32` (multiplier applied to item base_cost when NPC sells to player; default 1.0)
- `pub max_buy_value: Option<u32>` (None = no cap)

All types must derive `Debug, Clone, Serialize, Deserialize, PartialEq`.
All types must have `///` doc comments on every public field.

**Validation:** `cargo check`, `cargo clippy -- -D warnings`, and
`cargo nextest run` must all pass.

#### 1.3 Extend `NpcDefinition` with Inventory and Service Fields

**File:** `src/domain/world/npc.rs`

Add the following optional fields to `NpcDefinition`, all annotated with
`#[serde(default)]`:

- `pub stock_template: Option<String>` - ID referencing a merchant stock
  template in campaign data (e.g., `"blacksmith_basic_stock"`). Used to
  initialize `MerchantStock` at runtime.
- `pub service_catalog: Option<crate::domain::inventory::ServiceCatalog>` -
  Inline service definitions for priest or innkeeper NPCs.
- `pub economy: Option<crate::domain::inventory::NpcEconomySettings>` - Per-NPC
  buy/sell rate overrides.

These fields are static definition data (what the NPC offers). Runtime mutable
state (current stock quantities) lives in `NpcRuntimeState` created in Phase 2.

**Validation:** `cargo check` must pass. All existing `test_npc_definition_*`
tests must pass. New fields must serialize and deserialize correctly in RON
round-trips.

#### 1.4 Testing Requirements for Phase 1

Write the following unit tests. Place them in the `mod tests` block of each
respective file.

**In `src/domain/inventory.rs`:**

- `test_merchant_stock_decrement_success` - decrement an entry with quantity > 0,
  assert returns true, quantity decremented by 1.
- `test_merchant_stock_decrement_out_of_stock` - decrement entry with quantity 0,
  assert returns false.
- `test_merchant_stock_decrement_nonexistent_item` - decrement item_id not in
  stock, assert returns false.
- `test_merchant_stock_effective_price_uses_override` - set override_price,
  assert effective_price returns override, not base_cost.
- `test_merchant_stock_effective_price_uses_base_cost` - no override, assert
  effective_price returns base_cost.
- `test_service_catalog_get_service_found` - add service, get by service_id, assert Some.
- `test_service_catalog_get_service_not_found` - get non-existent service_id, assert None.
- `test_service_catalog_has_service` - add service, assert has_service returns true.

**In `src/domain/world/npc.rs`:**

- `test_npc_definition_is_priest_defaults_false` - deserialize NPC from RON
  without `is_priest` field, assert `is_priest == false`.
- `test_npc_definition_priest_constructor` - call `NpcDefinition::priest()`,
  assert `is_priest == true`, `is_merchant == false`, `is_innkeeper == false`.
- `test_npc_definition_stock_template_defaults_none` - deserialize NPC from RON
  without `stock_template`, assert `stock_template == None`.
- `test_npc_definition_service_catalog_defaults_none` - deserialize NPC from RON
  without `service_catalog`, assert `service_catalog == None`.

#### 1.5 Deliverables

- [ ] `src/domain/inventory.rs` created with `StockEntry`, `MerchantStock`,
      `ServiceEntry`, `ServiceCatalog`, `NpcEconomySettings`, `InventoryOwner`
- [ ] `src/domain/mod.rs` updated with `pub mod inventory;`
- [ ] `src/domain/world/npc.rs` updated: `is_priest`, `stock_template`,
      `service_catalog`, `economy` added to `NpcDefinition`; `NpcDefinition::priest()`
      constructor added
- [ ] `src/sdk/database.rs` updated: `NpcDatabase::priests()` added
- [ ] All unit tests from Section 1.4 passing

#### 1.6 Success Criteria

- `cargo fmt --all` produces no output
- `cargo check --all-targets --all-features` reports zero errors
- `cargo clippy --all-targets --all-features -- -D warnings` reports zero warnings
- `cargo nextest run --all-features` passes all tests
- All existing NPC RON files (`data/npcs.ron`, `campaigns/tutorial/data/npcs.ron`)
  deserialize without errors (verified by `test_load_core_npcs_file` and
  `test_load_tutorial_npcs_file` in `src/sdk/database.rs`)

---

### Phase 2: NPC Runtime State and Transaction Operations

Add the mutable runtime state that tracks NPC stock during play, and add the
explicit domain-layer transaction functions that enforce all business rules.

#### 2.1 Create `src/domain/world/npc_runtime.rs` - NPC Runtime State

Create a new file `src/domain/world/npc_runtime.rs`. Register it in
`src/domain/world/mod.rs` as `pub mod npc_runtime;`.

Define `NpcRuntimeState`:

- `pub npc_id: NpcId` - which NPC this runtime state belongs to
- `pub stock: Option<crate::domain::inventory::MerchantStock>` - None if NPC is
  not a merchant or has no stock. Initialized from `stock_template` at session start.
- `pub services_consumed: Vec<String>` - list of `service_id` values consumed
  this session (for per-session limits, if needed)

Implement:

- `pub fn new(npc_id: NpcId) -> Self`
- `pub fn initialize_stock_from_template(template: &MerchantStockTemplate) -> Self` -
  creates runtime state with stock quantities copied from the template

Define `MerchantStockTemplate`:

- `pub id: String` - matches `NpcDefinition::stock_template`
- `pub entries: Vec<StockEntry>` - the template quantities, NOT mutable during play

Define `NpcRuntimeStore`:

- `npcs: HashMap<NpcId, NpcRuntimeState>` (private)
- Implement `pub fn new() -> Self`
- Implement `pub fn get(&self, npc_id: &NpcId) -> Option<&NpcRuntimeState>`
- Implement `pub fn get_mut(&mut self, npc_id: &NpcId) -> Option<&mut NpcRuntimeState>`
- Implement `pub fn insert(&mut self, state: NpcRuntimeState)`
- Implement `pub fn initialize_merchant(&mut self, npc: &NpcDefinition, templates: &MerchantStockTemplateDatabase)`

Define `MerchantStockTemplateDatabase`:

- `templates: HashMap<String, MerchantStockTemplate>` (private)
- Implement `pub fn new() -> Self`
- Implement `pub fn add(&mut self, template: MerchantStockTemplate)`
- Implement `pub fn get(&self, id: &str) -> Option<&MerchantStockTemplate>`
- Implement `pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ...>`

All types must derive `Debug, Clone, Serialize, Deserialize`. Types used only at
runtime (not serialized from data files) may omit `Deserialize` if appropriate,
but `NpcRuntimeState` MUST be fully serializable so it can be included in save
data.

#### 2.2 Create `src/domain/transactions.rs` - Transaction Operations

Create a new file `src/domain/transactions.rs`. Register it in
`src/domain/mod.rs` as `pub mod transactions;`.

Define `TransactionError` using `thiserror::Error`:

```
InsufficientGold { have: u32, need: u32 }
InsufficientGems { have: u32, need: u32 }
InventoryFull { character_id: CharacterId }
ItemNotInStock { item_id: ItemId }
OutOfStock { item_id: ItemId }
ItemNotInInventory { item_id: ItemId, character_id: CharacterId }
NpcNotFound { npc_id: String }
ServiceNotFound { service_id: String }
CharacterNotFound { character_id: CharacterId }
InvalidQuantity
```

Define the following pure functions. Each function takes only domain types as
arguments (no Bevy types). Each function returns `Result<T, TransactionError>`.

**`pub fn buy_item`**

Signature:

```
pub fn buy_item(
    party: &mut Party,
    character: &mut Character,
    character_id: CharacterId,
    npc_runtime: &mut NpcRuntimeState,
    npc_def: &NpcDefinition,
    item_id: ItemId,
    item_db: &ItemDatabase,
) -> Result<InventorySlot, TransactionError>
```

Logic (execute in this exact order):

1. Look up `item` in `item_db.get_item(item_id)`. If None, return
   `TransactionError::ItemNotInStock`.
2. Look up `entry` in `npc_runtime.stock.get_entry(item_id)`. If None, return
   `TransactionError::ItemNotInStock`.
3. If `entry.quantity == 0`, return `TransactionError::OutOfStock`.
4. Compute `price = entry.effective_price(item_id, item.base_cost as u32)`.
   Apply `npc_def.economy.sell_rate` if present (default 1.0). Round to nearest
   integer.
5. If `party.gold < price as u16`, return
   `TransactionError::InsufficientGold { have: party.gold as u32, need: price }`.
6. If `character.inventory.is_full()`, return
   `TransactionError::InventoryFull { character_id }`.
7. Deduct `party.gold -= price as u16`.
8. Call `npc_runtime.stock.decrement(item_id)`.
9. Construct `slot = InventorySlot { item_id, charges: item.max_charges }`.
10. Add slot to `character.inventory` (use `character.inventory.add_item(slot)`).
11. Return `Ok(slot)`.

**`pub fn sell_item`**

Signature:

```
pub fn sell_item(
    party: &mut Party,
    character: &mut Character,
    character_id: CharacterId,
    npc_runtime: &mut NpcRuntimeState,
    npc_def: &NpcDefinition,
    item_id: ItemId,
    item_db: &ItemDatabase,
) -> Result<u32, TransactionError>
```

Logic (execute in this exact order):

1. Find the slot index in `character.inventory.items` where
   `slot.item_id == item_id`. If not found, return
   `TransactionError::ItemNotInInventory { item_id, character_id }`.
2. Look up `item` in `item_db.get_item(item_id)`. If None, return
   `TransactionError::ItemNotInStock`.
3. Compute `sell_price`. Use `item.sell_cost` if non-zero, else use
   `item.base_cost / 2`. Apply `npc_def.economy.buy_rate` if present (default
   0.5). Round down to nearest integer. Minimum sell price is 1 gold.
4. Remove the slot from `character.inventory.items` at the found index.
5. Add `sell_price` to `party.gold` (clamp to `u16::MAX`).
6. If `npc_runtime.stock` is `Some`, add or increment the item back into
   `npc_runtime.stock.entries` (optional: only if NPC has existing entry for it).
7. Return `Ok(sell_price)`.

**`pub fn consume_service`**

Signature:

```
pub fn consume_service(
    party: &mut Party,
    targets: &mut Vec<&mut Character>,
    npc_runtime: &mut NpcRuntimeState,
    service_catalog: &ServiceCatalog,
    service_id: &str,
) -> Result<ServiceOutcome, TransactionError>
```

Define `ServiceOutcome`:

- `pub service_id: String`
- `pub gold_paid: u32`
- `pub gems_paid: u32`
- `pub characters_affected: Vec<CharacterId>`

Logic (execute in this exact order):

1. Look up `entry = service_catalog.get_service(service_id)`. If None, return
   `TransactionError::ServiceNotFound`.
2. Check `party.gold >= entry.cost as u16`. If not, return
   `TransactionError::InsufficientGold`.
3. Check `party.gems >= entry.gem_cost as u16`. If not, return
   `TransactionError::InsufficientGems`.
4. Deduct `party.gold -= entry.cost as u16`.
5. Deduct `party.gems -= entry.gem_cost as u16`.
6. Apply service effect to each character in `targets` based on `service_id`:
   - `"heal_all"` or `"heal"`: set `character.hp.current = character.hp.base`
   - `"restore_sp"`: set `character.sp.current = character.sp.base`
   - `"cure_poison"`: call `character.remove_condition(Condition::POISONED)`
   - `"cure_disease"`: call `character.remove_condition(Condition::DISEASED)`
   - `"cure_all"`: call `character.conditions.clear()`
   - `"resurrect"`: call `character.conditions.clear()`, set
     `character.hp.current = 1`
   - `"rest"`: set `character.hp.current = character.hp.base`,
     `character.sp.current = character.sp.base`, call `character.conditions.clear()`
   - Unrecognized `service_id`: no-op (character is unaffected; no error)
7. Record `service_id` in `npc_runtime.services_consumed`.
8. Return `Ok(ServiceOutcome { ... })`.

#### 2.3 Extend `GameState` with `NpcRuntimeStore`

**File:** `src/application/mod.rs`

Add `pub npc_runtime: NpcRuntimeStore` field to `GameState`. Import the type
from `crate::domain::world::npc_runtime::NpcRuntimeStore`.

Update `GameState::new()` and `GameState::default()` to initialize
`npc_runtime: NpcRuntimeStore::new()`.

Update `GameState::new_game()` / `GameState::initialize_roster()` to call
`npc_runtime.initialize_merchant(npc_def, templates)` for each NPC with a
non-None `stock_template`, once the `ContentDatabase` is available.

#### 2.4 Testing Requirements for Phase 2

**In `src/domain/transactions.rs`:**

- `test_buy_item_success` - party has gold, character has space, NPC has stock.
  Assert: gold decreased by price, item added to character inventory, stock
  quantity decremented.
- `test_buy_item_insufficient_gold` - party gold < price. Assert:
  `TransactionError::InsufficientGold`. Party gold unchanged. Character
  inventory unchanged. Stock unchanged.
- `test_buy_item_inventory_full` - character inventory at `Inventory::MAX_ITEMS`.
  Assert: `TransactionError::InventoryFull`. Party gold unchanged. Stock
  unchanged.
- `test_buy_item_out_of_stock` - item in NPC stock but quantity == 0. Assert:
  `TransactionError::OutOfStock`. Party gold unchanged. Character inventory
  unchanged.
- `test_buy_item_not_in_stock` - item_id not in NPC stock at all. Assert:
  `TransactionError::ItemNotInStock`.
- `test_sell_item_success` - item in character inventory, NPC has buy rate.
  Assert: item removed from inventory, gold added to party.
- `test_sell_item_not_in_inventory` - item_id not in character inventory. Assert:
  `TransactionError::ItemNotInInventory`.
- `test_sell_item_minimum_price` - item with sell_cost == 0, base_cost == 1.
  Assert: returned gold >= 1.
- `test_consume_service_heal_all_success` - party has gold, character has reduced
  HP. Assert: gold deducted, HP restored to base.
- `test_consume_service_insufficient_gold` - party gold < service cost. Assert:
  `TransactionError::InsufficientGold`. Party gold unchanged. Character HP
  unchanged.
- `test_consume_service_not_found` - service_id not in catalog. Assert:
  `TransactionError::ServiceNotFound`.
- `test_consume_service_resurrect` - dead character. Assert: conditions cleared,
  HP set to 1.

**In `src/domain/world/npc_runtime.rs`:**

- `test_npc_runtime_state_new` - assert npc_id set, stock None, consumed empty.
- `test_npc_runtime_store_insert_and_get` - insert state, retrieve by npc_id,
  assert Some.
- `test_npc_runtime_store_get_mut` - insert state, get mutable ref, modify,
  assert mutation persists.

#### 2.5 Deliverables

- [ ] `src/domain/inventory.rs` updated with `InventoryOwner`
- [ ] `src/domain/world/npc_runtime.rs` created with `NpcRuntimeState`,
      `NpcRuntimeStore`, `MerchantStockTemplate`, `MerchantStockTemplateDatabase`
- [ ] `src/domain/world/mod.rs` updated with `pub mod npc_runtime;`
- [ ] `src/domain/transactions.rs` created with `TransactionError`,
      `buy_item()`, `sell_item()`, `consume_service()`, `ServiceOutcome`
- [ ] `src/domain/mod.rs` updated with `pub mod transactions;`
- [ ] `src/application/mod.rs` updated: `GameState` has `npc_runtime` field
- [ ] All unit tests from Section 2.4 passing

#### 2.6 Success Criteria

- All four cargo quality gate commands pass with zero errors and zero warnings
- `test_buy_item_success` confirms party gold decrements and item appears in
  character inventory
- `test_consume_service_heal_all_success` confirms HP is restored after gold
  is deducted
- `test_save_and_load` in `src/application/save_game.rs` still passes (GameState
  serializes with new `npc_runtime` field)

---

### Phase 3: Dialogue Action and Application Integration

Wire the new transaction operations into the dialogue execution path. Replace the
current ad-hoc item-give logic with calls to the domain transaction functions,
and add new `DialogueAction` variants for merchant and service interactions.

#### 3.1 Add `DialogueAction` Variants for Transactions

**File:** `src/domain/dialogue.rs`

Add the following new variants to `DialogueAction`. Use `#[serde(default)]` on
optional sub-fields where needed:

```
BuyItem {
    item_id: ItemId,
    target_character_id: Option<CharacterId>,   // None = first available character
}

SellItem {
    item_id: ItemId,
    source_character_id: Option<CharacterId>,   // None = search all party members
}

OpenMerchant {
    npc_id: String,   // NpcId of the merchant
}

ConsumeService {
    service_id: String,
    target_character_ids: Vec<CharacterId>,     // empty = apply to whole party
}
```

Update `DialogueAction::description()` to return a human-readable string for
each new variant. This method is tested by existing tests and must not break.

#### 3.2 Update `execute_action()` in Game Systems to Call Transaction Functions

**File:** `src/game/systems/dialogue.rs`

In the `execute_action()` function, add match arms for the four new
`DialogueAction` variants:

**For `DialogueAction::BuyItem { item_id, target_character_id }`:**

1. Get `game_state` (mutable) from Bevy world resources.
2. Resolve `character_id`: use `target_character_id` if `Some`, else use index
   `0` if party is non-empty.
3. Get `npc_id` from `dialogue_state.speaker_npc_id`. If None, log a warning
   and return.
4. Get `npc_def` from `content_db.npcs.get_npc(&npc_id)`. If None, log error and
   return.
5. Get or create `npc_runtime` from `game_state.npc_runtime.get_mut(&npc_id)`.
6. Get mutable character from `game_state.roster.get_character_mut(character_id)`.
7. Call `domain::transactions::buy_item(...)`.
8. On `Ok(slot)`: log info message `"Bought item {item_id} for character
{character_id}"`.
9. On `Err(e)`: log warning message with error description. No state is mutated.

**For `DialogueAction::SellItem { item_id, source_character_id }`:**

1. Same NPC resolution pattern as BuyItem.
2. If `source_character_id` is None, iterate party members to find first
   character whose inventory contains `item_id`. If none found, log warning and
   return.
3. Call `domain::transactions::sell_item(...)`.
4. On `Ok(price)`: log info `"Sold item {item_id} for {price} gold"`.
5. On `Err(e)`: log warning with error description.

**For `DialogueAction::OpenMerchant { npc_id }`:**

This action transitions the game to a new `GameMode` for the shop UI. Because
the shop UI (Phase 4) does not exist yet, log an info message
`"OpenMerchant: {npc_id} - shop UI not yet implemented"` and return without
state change. This placeholder must be replaced in Phase 4.

**For `DialogueAction::ConsumeService { service_id, target_character_ids }`:**

1. Get `npc_id` from `dialogue_state.speaker_npc_id`.
2. Get `npc_def` and look up `service_catalog`. If None, log warning and return.
3. Get `npc_runtime` mutably from `game_state.npc_runtime`.
4. Resolve targets: if `target_character_ids` is empty, collect all party
   members as `Vec<&mut Character>`. Otherwise resolve by index.
5. Call `domain::transactions::consume_service(...)`.
6. On `Ok(outcome)`: log info with outcome details.
7. On `Err(e)`: log warning with error description.

#### 3.3 Extend `EventResult` and Event Handler for Merchant Entry

**File:** `src/domain/world/events.rs`

Add a new `EventResult` variant:

```
EnterMerchant {
    npc_id: crate::domain::world::NpcId,
}
```

**File:** `src/game/systems/events.rs`

In `handle_events()`, add a match arm for `EventResult::EnterMerchant`:

1. Look up the NPC by `npc_id` in content DB.
2. If NPC has a `dialogue_id`, fire `StartDialogue` event (same pattern as
   `NpcDialogue`).
3. Otherwise log info `"Merchant {npc_id} has no dialogue configured"`.

Do NOT add a new `GameMode::Shopping` in this phase. That is reserved for Phase 4.

#### 3.4 Testing Requirements for Phase 3

**In `src/game/systems/dialogue.rs` tests (or a new integration test file
`tests/transaction_dialogue_integration_test.rs`):**

- `test_buy_item_dialogue_action_deducts_gold` - set up game state with party
  gold, character with space, NPC with stock. Fire `DialogueAction::BuyItem`.
  Assert party gold decreased and item in character inventory.
- `test_buy_item_dialogue_action_insufficient_gold_no_mutation` - party has
  insufficient gold. Fire `DialogueAction::BuyItem`. Assert party gold unchanged
  and character inventory unchanged.
- `test_consume_service_dialogue_action_heals_party` - fire
  `DialogueAction::ConsumeService { service_id: "heal_all", target_character_ids: vec![] }`.
  Assert character HP restored.
- `test_consume_service_dialogue_action_insufficient_gold_no_mutation` - party
  has 0 gold, service costs > 0. Fire action. Assert HP unchanged, gold unchanged.
- `test_dialogue_action_description_buy_item` - assert
  `DialogueAction::BuyItem { item_id: 1, target_character_id: None }.description()`
  is not empty.

#### 3.5 Deliverables

- [ ] `src/domain/dialogue.rs` updated: `BuyItem`, `SellItem`, `OpenMerchant`,
      `ConsumeService` variants added to `DialogueAction`
- [ ] `src/game/systems/dialogue.rs` updated: `execute_action()` handles all
      four new variants
- [ ] `src/domain/world/events.rs` updated: `EventResult::EnterMerchant` added
- [ ] `src/game/systems/events.rs` updated: `EnterMerchant` match arm added
- [ ] All unit and integration tests from Section 3.4 passing

#### 3.6 Success Criteria

- All four cargo quality gate commands pass
- `test_buy_item_dialogue_action_deducts_gold` confirms end-to-end transaction
  through dialogue system
- `test_dialogue_action_description_buy_item` confirms `description()` coverage
- All existing dialogue tests in `src/game/systems/dialogue.rs` still pass

---

### Phase 4: Data Schema and SDK Updates

Extend the RON data schemas, SDK database loaders, and SDK validator to support
merchant stock templates, service catalogs, and NPC economy settings.

#### 4.1 Create `data/npc_stock_templates.ron`

Create the file `data/npc_stock_templates.ron` as the core data file for
`MerchantStockTemplateDatabase`.

Define at minimum these templates:

- `"blacksmith_basic"` - common weapons (club, dagger, short sword) and armor
  (leather, chain), quantities 2-5 each.
- `"general_store_basic"` - consumables (rations, healing herbs), quantities
  5-10 each.
- `"alchemist_basic"` - consumable potions, quantities 3-5 each.

All `item_id` values MUST reference valid IDs present in `data/items.ron`. Use
only items whose IDs are already defined there.

RON format follows the existing pattern in the project. Example entry:

```
(
    id: "blacksmith_basic",
    entries: [
        (item_id: 1, quantity: 3, override_price: None),
        (item_id: 2, quantity: 2, override_price: Some(15)),
    ],
)
```

#### 4.2 Create `campaigns/tutorial/data/npc_stock_templates.ron`

Create the campaign-specific stock template file for the tutorial campaign. This
file MAY override entries from `data/npc_stock_templates.ron` or define entirely
new templates scoped to the tutorial.

Define at minimum:

- `"tutorial_merchant_stock"` - a curated subset of items appropriate for a
  tutorial campaign, with low quantities to encourage resource management.

#### 4.3 Update `data/npcs.ron` and `campaigns/tutorial/data/npcs.ron`

Update the `base_merchant` NPC entry in `data/npcs.ron` to add:

```
stock_template: Some("blacksmith_basic"),
economy: Some((buy_rate: 0.5, sell_rate: 1.0, max_buy_value: None)),
```

Update the `base_priest` NPC entry in `data/npcs.ron` to add:

```
service_catalog: Some((services: [
    (service_id: "heal_all", cost: 50, gem_cost: 0, description: "Heal all party members to full HP"),
    (service_id: "cure_poison", cost: 25, gem_cost: 0, description: "Cure poison from one character"),
    (service_id: "cure_disease", cost: 75, gem_cost: 0, description: "Cure disease from one character"),
    (service_id: "resurrect", cost: 200, gem_cost: 1, description: "Resurrect a dead character"),
])),
```

Update the `base_innkeeper` NPC entry in `data/npcs.ron` to add:

```
service_catalog: Some((services: [
    (service_id: "rest", cost: 10, gem_cost: 0, description: "Rest the party, restoring HP and SP"),
])),
```

Update `campaigns/tutorial/data/npcs.ron` equivalently for the tutorial-specific
merchant and innkeeper NPCs.

#### 4.4 Add `MerchantStockTemplateDatabase` to `ContentDatabase`

**File:** `src/sdk/database.rs`

Add a new database struct `MerchantStockTemplateDatabase` following the same
pattern as existing databases (`ItemDatabase`, `NpcDatabase`, etc.):

- Private field `templates: HashMap<String, MerchantStockTemplate>`
- `pub fn new() -> Self`
- `pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, DatabaseError>`
- `pub fn get(&self, id: &str) -> Option<&MerchantStockTemplate>`
- `pub fn all_templates(&self) -> Vec<&MerchantStockTemplate>`
- `pub fn count(&self) -> usize`

Add `pub npc_stock_templates: MerchantStockTemplateDatabase` field to
`ContentDatabase`.

Update `ContentDatabase::load_core()` to load `data/npc_stock_templates.ron`.

Update `ContentDatabase::load_campaign()` to load campaign-specific
`npc_stock_templates.ron` (if present, otherwise use core data only).

Update `ContentDatabase::stats()` to include a `npc_stock_template_count` field
in `ContentStats`.

#### 4.5 Add SDK Validator Rules for Merchant and Service Data

**File:** `src/sdk/validation.rs`

Add two new `ValidationError` variants:

```
MissingStockTemplateItem {
    context: String,       // NPC ID
    item_id: ItemId,       // Item referenced in template
}

InvalidServiceId {
    context: String,       // NPC ID
    service_id: String,    // The unrecognized service ID
}
```

Add two new validation methods to `impl Validator`:

**`fn validate_merchant_stock(&self) -> Vec<ValidationError>`**

For every `NpcDefinition` where `is_merchant == true`:

- If `stock_template` is `Some(template_id)`, look it up in
  `db.npc_stock_templates.get(template_id)`.
- If template not found, emit `ValidationError::MissingItem` (reuse existing
  variant with context = NPC ID).
- For each `StockEntry` in the template, verify `db.items.has_item(&entry.item_id)`.
  If not found, emit `ValidationError::MissingStockTemplateItem`.

**`fn validate_service_catalogs(&self) -> Vec<ValidationError>`**

Define the set of known built-in service IDs as a constant:

```
const KNOWN_SERVICE_IDS: &[&str] = &[
    "heal_all", "heal", "restore_sp", "cure_poison", "cure_disease",
    "cure_all", "resurrect", "rest",
];
```

For every `NpcDefinition` where `service_catalog` is `Some`:

- For each `ServiceEntry` in the catalog, check whether `service_id` is in
  `KNOWN_SERVICE_IDS`.
- If not, emit `ValidationError::InvalidServiceId` with severity `Warning`
  (not `Error`) to allow custom service IDs for future extensibility.

Update `Validator::validate_all()` to call both new methods and include their
results.

#### 4.6 Testing Requirements for Phase 4

**In `src/sdk/database.rs` tests:**

- `test_merchant_stock_template_database_new` - assert empty database.
- `test_merchant_stock_template_database_load_from_file` - load
  `data/npc_stock_templates.ron`, assert count >= 3.
- `test_content_database_includes_npc_stock_templates` - load core content,
  assert `npc_stock_templates.count() > 0`.
- `test_base_merchant_has_stock_template` - load `data/npcs.ron`, get
  `base_merchant`, assert `stock_template == Some("blacksmith_basic")`.
- `test_base_priest_has_service_catalog` - load `data/npcs.ron`, get
  `base_priest`, assert `service_catalog` is `Some` with at least 4 entries.

**In `src/sdk/validation.rs` tests:**

- `test_validate_merchant_stock_valid` - NPC with valid stock_template referencing
  valid items, assert zero errors.
- `test_validate_merchant_stock_missing_template` - NPC references non-existent
  template, assert at least one error.
- `test_validate_merchant_stock_invalid_item` - template references item_id not
  in ItemDatabase, assert `ValidationError::MissingStockTemplateItem`.
- `test_validate_service_catalogs_known_ids` - service catalog with all known
  service IDs, assert zero warnings.
- `test_validate_service_catalogs_unknown_id` - service catalog with unknown
  service_id, assert one warning with severity `Warning` (not `Error`).

#### 4.7 Deliverables

- [ ] `data/npc_stock_templates.ron` created
- [ ] `campaigns/tutorial/data/npc_stock_templates.ron` created
- [ ] `data/npcs.ron` updated: `base_merchant`, `base_priest`, `base_innkeeper`
      have appropriate new fields
- [ ] `campaigns/tutorial/data/npcs.ron` updated equivalently
- [ ] `src/sdk/database.rs` updated: `MerchantStockTemplateDatabase` struct,
      `ContentDatabase::npc_stock_templates` field, loading in `load_core()` and
      `load_campaign()`
- [ ] `src/sdk/validation.rs` updated: two new `ValidationError` variants, two
      new validator methods, `validate_all()` updated
- [ ] All unit tests from Section 4.6 passing

#### 4.8 Success Criteria

- All four cargo quality gate commands pass
- `test_content_database_includes_npc_stock_templates` confirms RON files load
- `test_validate_merchant_stock_invalid_item` confirms cross-reference validation
  catches bad item IDs in stock templates
- `test_load_core_npcs_file` and `test_load_tutorial_npcs_file` (pre-existing
  tests) still pass with updated NPC RON files

---

### Phase 5: Save/Load Persistence

Extend `SaveGame` to persist NPC runtime inventory and service state so that
merchant stock consumed during a session is restored correctly on load.

#### 5.1 Add `NpcRuntimeStore` to `SaveGame` Serialization

**File:** `src/application/save_game.rs`

`GameState` already has the `npc_runtime: NpcRuntimeStore` field added in Phase 2. Because `SaveGame` serializes `GameState` verbatim, `NpcRuntimeStore` will be
included in the save file automatically — BUT only if `NpcRuntimeStore` and
`NpcRuntimeState` fully implement `Serialize` and `Deserialize`, which was
required in Phase 2.

Verify this end-to-end by running the existing `test_save_and_load` test after
Phase 2 changes. If it fails due to a missing `Serialize` impl on any new type,
add the missing derive.

No structural change to `SaveGame` itself is required.

#### 5.2 Handle Missing `npc_runtime` Field in Legacy Save Files

`NpcRuntimeStore` must have a `Default` implementation that produces an empty
store. Annotate the `npc_runtime` field in `GameState` with
`#[serde(default)]` so that existing save files without this field deserialize
correctly (they will get an empty `NpcRuntimeStore`, which will be re-initialized
from template data when the campaign loads).

**File:** `src/application/mod.rs`

Ensure `impl Default for NpcRuntimeStore` returns `NpcRuntimeStore::new()`.

Add `#[serde(default)]` to the `npc_runtime` field on `GameState`.

#### 5.3 Re-initialize NPC Runtime State on Load

When loading a save game, if the loaded `game_state.npc_runtime` is empty (e.g.,
from a legacy save), re-initialize merchant stock from templates after campaign
content is loaded.

**File:** `src/application/mod.rs`

Add a new method to `GameState`:

```
pub fn ensure_npc_runtime_initialized(&mut self, content: &ContentDatabase)
```

Logic:

- Iterate all NPCs in `content.npcs.all_npcs()`.
- For each NPC, if `npc_runtime.get(&npc.id)` is `None` AND `npc.stock_template`
  is `Some`, call `npc_runtime.initialize_merchant(npc, &content.npc_stock_templates)`.
- This is idempotent: calling it on a fully-initialized game state is a no-op.

Call this method from `GameState::load_campaign_content()` after the content
database is populated.

#### 5.4 Testing Requirements for Phase 5

**In `src/application/save_game.rs` tests (add to existing test module):**

- `test_save_load_preserves_npc_runtime_stock` - set up game state with NPC
  runtime stock, simulate a buy (decrement one item), save, load, assert the
  loaded state has the decremented quantity, not the original template quantity.
- `test_save_load_legacy_format_empty_npc_runtime` - load a save game that does
  NOT have a `npc_runtime` field in the RON. Assert deserialization succeeds and
  `npc_runtime` is empty (not an error).

**In `src/application/mod.rs` tests (add to existing test module):**

- `test_ensure_npc_runtime_initialized_populates_merchants` - create `GameState`
  with empty `npc_runtime`, call `ensure_npc_runtime_initialized()` with a
  `ContentDatabase` containing NPCs with `stock_template`. Assert that the NPC
  runtime states are now populated.
- `test_ensure_npc_runtime_initialized_is_idempotent` - call
  `ensure_npc_runtime_initialized()` twice. Assert second call does not
  overwrite existing runtime state (quantities remain at post-first-call values).

#### 5.5 Deliverables

- [ ] `src/domain/world/npc_runtime.rs`: `NpcRuntimeStore`, `NpcRuntimeState`
      implement `Serialize`, `Deserialize`, `Default`
- [ ] `src/application/mod.rs`: `GameState.npc_runtime` has `#[serde(default)]`;
      `GameState::ensure_npc_runtime_initialized()` implemented
- [ ] All unit tests from Section 5.4 passing

#### 5.6 Success Criteria

- All four cargo quality gate commands pass
- `test_save_load_preserves_npc_runtime_stock` confirms round-trip fidelity
- `test_save_load_legacy_format_empty_npc_runtime` confirms backward compatibility
- `test_save_and_load` (pre-existing) still passes

---

### Phase 6: Integration Tests and End-to-End Verification

Add end-to-end integration tests that exercise complete merchant, priest, and
innkeeper flows from game event through dialogue through transaction to state
persistence.

#### 6.1 Create `tests/merchant_transaction_integration_test.rs`

This file tests the complete merchant buy/sell flow end-to-end.

Required tests:

- `test_merchant_buy_flow_end_to_end` - Full flow:

  1. Create `GameState` with tutorial campaign content loaded.
  2. Set party gold to 100.
  3. Call `buy_item(...)` from `domain::transactions` directly (not through Bevy)
     with a valid item from the stock template.
  4. Assert: `party.gold < 100`, character inventory contains the item.
  5. Call `save`, call `load`, assert loaded state matches.

- `test_merchant_sell_flow_end_to_end` - Full flow:

  1. Add item to character inventory manually.
  2. Call `sell_item(...)`.
  3. Assert: item removed from inventory, `party.gold` increased.

- `test_merchant_buy_respects_inventory_limit` - fill character inventory to
  `Inventory::MAX_ITEMS`, attempt buy, assert `TransactionError::InventoryFull`,
  party gold unchanged.

- `test_merchant_stock_depletes_after_buy` - buy all quantity of one item,
  attempt to buy again, assert `TransactionError::OutOfStock`.

- `test_merchant_stock_persists_depletion_after_save_load` - buy one item,
  save, load, verify the item's quantity in NPC runtime stock is still
  decremented.

#### 6.2 Create `tests/priest_service_integration_test.rs`

This file tests the priest service flow end-to-end.

Required tests:

- `test_priest_heal_all_flow` - Character at partial HP, party has sufficient
  gold. Call `consume_service(...)` with `"heal_all"`. Assert HP restored, gold
  deducted.

- `test_priest_resurrect_flow` - Character is dead (has `Condition::DEAD`).
  Call `consume_service(...)` with `"resurrect"`. Assert conditions cleared, HP
  is 1, gold deducted.

- `test_priest_service_insufficient_gold` - Party gold < service cost. Assert
  `TransactionError::InsufficientGold`, no HP change, no gold change.

- `test_priest_service_save_load_preserves_state` - After a service is consumed,
  save and load, verify the party state is correctly restored.

#### 6.3 Create `tests/innkeeper_service_integration_test.rs`

This file tests the innkeeper service flow end-to-end WITHOUT breaking the
existing party management behavior tested in
`tests/innkeeper_party_management_integration_test.rs`.

Required tests:

- `test_innkeeper_rest_service_heals_party` - Party at partial HP/SP, party has
  gold. Call `consume_service(...)` with `"rest"`. Assert all party members have
  HP and SP at base, gold deducted.

- `test_innkeeper_rest_insufficient_gold` - Party gold is 0. Assert
  `TransactionError::InsufficientGold`.

- `test_existing_inn_party_management_unaffected` - Re-run the core scenario
  from `test_complete_inn_workflow` using the updated codebase. Assert it still
  passes with no regression.

#### 6.4 Quality Gate Validation

After all tests are written and pass individually, run the full quality gate
sequence:

```
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

All commands MUST produce zero errors and zero warnings.

#### 6.5 Deliverables

- [ ] `tests/merchant_transaction_integration_test.rs` created with 5 tests
- [ ] `tests/priest_service_integration_test.rs` created with 4 tests
- [ ] `tests/innkeeper_service_integration_test.rs` created with 3 tests
- [ ] All new integration tests passing
- [ ] All pre-existing tests still passing (zero regressions)

#### 6.6 Success Criteria

- `cargo nextest run --all-features` passes ALL tests including pre-existing
- Total test count is greater than before Phase 6 began
- `test_existing_inn_party_management_unaffected` explicitly confirms zero
  regression in existing inn flow
- `test_merchant_stock_persists_depletion_after_save_load` confirms complete
  data fidelity for the new systems

---

### Phase 7: Documentation Updates

Update implementation documentation to reflect the delivered inventory system.

#### 7.1 Update `docs/explanation/implementations.md`

Append a new section to `docs/explanation/implementations.md` summarizing:

- Overview of what was built (shared inventory primitives, NPC runtime state,
  transaction operations, dialogue integration, data schema, save/load)
- List of files created and modified
- Components implemented (with the struct/function names)
- Testing coverage (counts and descriptions)
- Architecture compliance notes (type aliases used, constants used, no
  architectural deviations)

Do NOT modify `docs/reference/architecture.md`. It is read-only source of truth.

Do NOT create any new documentation files other than updating
`docs/explanation/implementations.md`.

#### 7.2 Deliverables

- [ ] `docs/explanation/implementations.md` updated with inventory system
      implementation summary

#### 7.3 Success Criteria

- File uses `lowercase_with_underscores` naming (already satisfied)
- No emojis in documentation
- All code blocks in doc comments use the correct path annotation format
- `markdownlint` passes (if configured in project)

---

## Candidate Files Summary

### New Files

| File                                              | Phase | Purpose                             |
| ------------------------------------------------- | ----- | ----------------------------------- |
| `src/domain/inventory.rs`                         | 1     | Shared inventory primitives         |
| `src/domain/transactions.rs`                      | 2     | Transaction domain operations       |
| `src/domain/world/npc_runtime.rs`                 | 2     | NPC mutable runtime state           |
| `data/npc_stock_templates.ron`                    | 4     | Core merchant stock templates       |
| `campaigns/tutorial/data/npc_stock_templates.ron` | 4     | Tutorial stock templates            |
| `tests/merchant_transaction_integration_test.rs`  | 6     | Merchant integration tests          |
| `tests/priest_service_integration_test.rs`        | 6     | Priest service integration tests    |
| `tests/innkeeper_service_integration_test.rs`     | 6     | Innkeeper service integration tests |

### Modified Files

| File                                  | Phase | Change                                                                                 |
| ------------------------------------- | ----- | -------------------------------------------------------------------------------------- |
| `src/domain/mod.rs`                   | 1, 2  | Add `pub mod inventory;` and `pub mod transactions;`                                   |
| `src/domain/world/mod.rs`             | 2     | Add `pub mod npc_runtime;`                                                             |
| `src/domain/world/npc.rs`             | 1     | Add `is_priest`, `stock_template`, `service_catalog`, `economy` to `NpcDefinition`     |
| `src/domain/dialogue.rs`              | 3     | Add `BuyItem`, `SellItem`, `OpenMerchant`, `ConsumeService` to `DialogueAction`        |
| `src/domain/world/events.rs`          | 3     | Add `EventResult::EnterMerchant`                                                       |
| `src/application/mod.rs`              | 2, 5  | Add `npc_runtime` to `GameState`; add `ensure_npc_runtime_initialized()`               |
| `src/game/systems/dialogue.rs`        | 3     | Add match arms in `execute_action()` for new `DialogueAction` variants                 |
| `src/game/systems/events.rs`          | 3     | Add match arm for `EventResult::EnterMerchant`                                         |
| `src/sdk/database.rs`                 | 4     | Add `MerchantStockTemplateDatabase`, `ContentDatabase::npc_stock_templates`            |
| `src/sdk/validation.rs`               | 4     | Add `MissingStockTemplateItem`, `InvalidServiceId` variants; add two validator methods |
| `data/npcs.ron`                       | 4     | Update `base_merchant`, `base_priest`, `base_innkeeper` with new fields                |
| `campaigns/tutorial/data/npcs.ron`    | 4     | Update tutorial NPCs with new fields                                                   |
| `docs/explanation/implementations.md` | 7     | Append implementation summary                                                          |

---

## Architecture Constraints (MUST FOLLOW)

The following constraints apply to ALL code written in this plan. Violations
will cause rejection.

1. **Type aliases**: Use `ItemId`, `CharacterId`, `InnkeeperId`, `NpcId`
   (= `String`). Never use raw `u8`, `usize`, or `String` where a type alias
   exists.

2. **Constants**: Use `Inventory::MAX_ITEMS` and `Equipment::MAX_EQUIPPED`.
   Never hardcode `6` or `7`.

3. **AttributePair**: Never set `character.hp.base` or `character.stats.*.base`
   directly in transaction code. Use `character.hp.current` for runtime
   modifications.

4. **Error handling**: All functions that can fail return `Result<T, E>`.
   No `unwrap()` without a safety justification comment. No `expect()` without
   a descriptive message.

5. **RON format**: All new data files use `.ron` extension and RON syntax.
   No JSON or YAML for game content data.

6. **SPDX headers**: Every new `.rs` file begins with:

   ```
   // SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
   // SPDX-License-Identifier: Apache-2.0
   ```

7. **Doc comments**: Every public function, struct, enum, and module in new
   files must have `///` doc comments. Every public function must have an
   `# Examples` section with a compilable example.

8. **No new modules**: Do not create `src/utils/`, `src/helpers/`, or
   `src/common/` modules. All new code belongs in the modules specified in this
   plan.

9. **Existing inn flow**: The `InnManagementState`, `inn_ui.rs` system, and all
   behavior in `tests/innkeeper_party_management_integration_test.rs` MUST
   remain functionally unchanged.

10. **Party-level currency**: Gold, gems, and food are fields on `Party`, not on
    individual `Character`. All transaction functions debit/credit `party.gold`
    or `party.gems` directly.
