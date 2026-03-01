# Buy and Sell Plan

Phased implementation plan for merchant buy/sell and container interaction in
the Antares game engine.

---

## Overview

This plan covers two related but distinct interaction systems:

1. **Merchant Trade** — When the player is in `GameMode::Dialogue` with a
   merchant NPC and presses `I`, a split-screen inventory opens. The left panel
   shows the active character's inventory with a **Sell** action button. The
   right panel shows the merchant's stock with a **Buy** action button. Number
   keys `1`–`6` switch the active character. `TAB` toggles panel focus. `ESC`
   returns to dialogue.

2. **Container Interaction** — When the player faces a container tile event
   (chest, barrel, hole-in-the-wall, etc.) and presses `E`, a split-screen
   inventory opens. The left panel shows the active character's inventory with a
   **Stash** action button. The right panel shows the container's contents with
   **Take** and **Take All** action buttons. Navigation is identical to merchant
   trade.

### Current State (Before This Plan)

The following infrastructure already exists and **must not be re-implemented**:

| Component                                                | Location                                       | Status                    |
| -------------------------------------------------------- | ---------------------------------------------- | ------------------------- |
| `buy_item()` transaction                                 | `src/domain/transactions.rs`                   | ✅ Implemented and tested |
| `sell_item()` transaction                                | `src/domain/transactions.rs`                   | ✅ Implemented and tested |
| `MerchantStock`, `StockEntry`, `NpcEconomySettings`      | `src/domain/inventory.rs`                      | ✅ Implemented and tested |
| `MerchantInventoryState`, `MerchantFocus`                | `src/application/merchant_inventory_state.rs`  | ✅ Implemented and tested |
| `ContainerInventoryState`, `ContainerFocus`              | `src/application/container_inventory_state.rs` | ✅ Implemented and tested |
| `GameMode::MerchantInventory(_)`                         | `src/application/mod.rs`                       | ✅ Variant exists         |
| `GameMode::ContainerInventory(_)`                        | `src/application/mod.rs`                       | ✅ Variant exists         |
| `GameState::enter_merchant_inventory()`                  | `src/application/mod.rs`                       | ✅ Implemented and tested |
| `GameState::enter_container_inventory()`                 | `src/application/mod.rs`                       | ✅ Implemented and tested |
| `MerchantInventoryPlugin` (UI + input + action systems)  | `src/game/systems/merchant_inventory_ui.rs`    | ✅ Implemented            |
| `ContainerInventoryPlugin` (UI + input + action systems) | `src/game/systems/container_inventory_ui.rs`   | ✅ Implemented            |
| `NpcDefinition::is_merchant` flag                        | `src/domain/world/npc.rs`                      | ✅ Implemented            |
| `NpcRuntimeStore`, `NpcRuntimeState`                     | `src/domain/world/npc_runtime.rs`              | ✅ Implemented            |
| `ensure_npc_runtime_initialized()`                       | `src/application/mod.rs`                       | ✅ Implemented            |
| `DialogueAction::OpenMerchant`                           | `src/domain/dialogue.rs`                       | ✅ Variant exists (stub)  |
| `EventResult::EnterMerchant`                             | `src/domain/world/events.rs`                   | ✅ Variant exists         |
| `handle_event_result` for `EnterMerchant`                | `src/game/systems/events.rs`                   | ✅ Routes to dialogue     |
| Tutorial merchant NPCs and stock templates               | `campaigns/tutorial/data/`                     | ✅ Data present           |
| Test campaign merchant NPCs and stock templates          | `data/test_campaign/data/`                     | ✅ Data present           |

### What Is Missing (Gaps This Plan Closes)

| Gap                                                                                                                   | Phase   |
| --------------------------------------------------------------------------------------------------------------------- | ------- |
| `DialogueAction::OpenMerchant` handler calls `enter_merchant_inventory()` instead of logging a stub                   | Phase 1 |
| Pressing `I` while in `GameMode::Dialogue` with a merchant NPC opens `GameMode::MerchantInventory`                    | Phase 1 |
| `I` key in non-merchant dialogue is ignored (no mode change)                                                          | Phase 1 |
| Player feedback (game log message) when buy/sell fails (insufficient gold, inventory full, out of stock, cursed item) | Phase 2 |
| Price display in merchant stock panel (shows item cost before confirming buy)                                         | Phase 2 |
| Party gold display in merchant UI header                                                                              | Phase 2 |
| Container interaction: `E` on a container map event enters `GameMode::ContainerInventory`                             | Phase 3 |
| Container items persist after partial take (map event state written back on close)                                    | Phase 3 |
| Container empty state: panel shows "Empty" text when container has no items                                           | Phase 3 |
| Mouse click support for Buy, Sell, Take, Take All, Stash action buttons                                               | Phase 4 |
| Tutorial merchant dialogue node wires `OpenMerchant` action to open shop                                              | Phase 5 |
| `data/test_campaign` merchant dialogue mirrors tutorial wiring                                                        | Phase 5 |
| `docs/explanation/implementations.md` updated                                                                         | Phase 5 |
| Merchant stock replenished to template quantities each in-game day                                                    | Phase 6 |
| Sold-out items reappear at dawn after the player rests or advances time past midnight                                 | Phase 6 |
| Each merchant carries a small rotating slot of random magic items refreshed every `magic_refresh_days`                | Phase 6 |
| Magic item pool and rotation count defined per-template in `npc_stock_templates.ron`                                  | Phase 6 |
| `GameState::advance_time` triggers restock check; no Bevy system clock dependency                                     | Phase 6 |
| Restock and magic-slot state persists across save/load                                                                | Phase 6 |

---

## Architecture Constraints

Before writing any code, re-read the following architecture sections:

- Section 4.3 (`Inventory`, `InventorySlot`, `Equipment`) — slot limits and
  `MAX_ITEMS` constant
- Section 4.6 (`ItemId`, `CharacterId`, `NpcId` type aliases) — use these,
  never raw `u32`/`usize` for domain types
- Section 4.9 (`CampaignLoader`) — data files use RON format only
- Section 12.7 (Inn and Save System) — `NpcRuntimeStore` is serialised into
  save data; stock changes from buying/selling must persist
- Section 12.11 (Item System Details) — cursed items cannot be sold

**Rules to verify before submitting each phase:**

- [ ] No raw `u32` used where `ItemId`/`CharacterId` type aliases apply
- [ ] `Inventory::MAX_ITEMS` constant used, never hardcoded `64` or other literal
- [ ] RON format for all new data files (never JSON or YAML for game data)
- [ ] `///` doc comments on every new public function, struct, enum
- [ ] All test data uses `data/test_campaign`, never `campaigns/tutorial`
- [ ] `cargo fmt --all` → no output
- [ ] `cargo check --all-targets --all-features` → 0 errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` → 0 warnings
- [ ] `cargo nextest run --all-features` → 0 failures

---

## Phase 1: Wire `OpenMerchant` Dialogue Action and `I`-Key Entry Point

**Goal:** Pressing `I` during dialogue with a merchant NPC opens
`GameMode::MerchantInventory`. The `OpenMerchant` dialogue action now drives
the same transition instead of logging a stub.

### 1.1 Wire `DialogueAction::OpenMerchant` in `execute_action`

**File:** `src/game/systems/dialogue.rs`

Find the `DialogueAction::OpenMerchant { npc_id }` arm in `execute_action`.
Replace the placeholder `info!` / `game_log.add` stub with a call to
`game_state.enter_merchant_inventory(npc_id.clone(), npc_name)`.

The NPC display name must be looked up from the content database before the
call. The lookup path is:

```antares/src/game/systems/dialogue.rs#L1265-1274
DialogueAction::OpenMerchant { npc_id } => {
    let npc_name = db
        .npcs
        .get_npc(npc_id)
        .map(|n| n.name.clone())
        .unwrap_or_else(|| npc_id.clone());
    game_state.ensure_npc_runtime_initialized(&db.content);
    game_state.enter_merchant_inventory(npc_id.clone(), npc_name);
}
```

**Important:** `ensure_npc_runtime_initialized` must be called before
`enter_merchant_inventory` so that the merchant's `MerchantStock` is populated
from its template the first time the player opens the shop. On subsequent
visits the store has already been initialised and the call is a no-op (existing
behaviour of that method).

The `db` and `game_state` parameters are already available in `execute_action`;
no new parameters are needed.

#### 1.1.1 Update the existing stub test

The test `test_open_merchant_dialogue_action_no_state_change` currently asserts
that the mode does **not** change (because the action was a stub). Replace it
with a test that asserts the mode transitions to `GameMode::MerchantInventory`:

Test name: `test_open_merchant_dialogue_action_enters_merchant_inventory`

```antares/src/game/systems/dialogue.rs#L2851-2874
#[test]
fn test_open_merchant_dialogue_action_enters_merchant_inventory() {
    let db = make_merchant_db();
    let mut game_state = make_game_state_with_merchant(0);
    game_state.mode = GameMode::Dialogue(merchant_dialogue_state());

    execute_action(
        &DialogueAction::OpenMerchant {
            npc_id: "merchant_tom".to_string(),
        },
        &mut game_state,
        &db,
        None,
        None,
        None,
    );

    assert!(
        matches!(game_state.mode, GameMode::MerchantInventory(_)),
        "OpenMerchant must transition mode to MerchantInventory"
    );
}
```

Add a second test that verifies a missing NPC does **not** panic and does not
change mode (graceful degradation):

Test name: `test_open_merchant_dialogue_action_unknown_npc_no_panic`

### 1.2 Add `I`-Key Handler for `GameMode::Dialogue`

**File:** `src/game/systems/input.rs`

The existing `handle_input` function already handles the `GameAction::Inventory`
action. The current match arm for that action explicitly ignores
`GameMode::Dialogue` (it falls into the `_` branch which calls
`game_state.enter_inventory()`). The change needed is:

- When the current mode is `GameMode::Dialogue(_)` **and** the NPC being spoken
  to `is_merchant == true`, call `game_state.enter_merchant_inventory(...)`.
- When the current mode is `GameMode::Dialogue(_)` and the NPC is **not** a
  merchant, do nothing (return without opening inventory or merchant screen).
- The existing behaviour for all other modes is unchanged.

The NPC identity must be read from `GlobalState::dialogue_state` (the
`DialogueState::npc_id` field, if present) and looked up in the content
database via `game_content: Option<Res<GameContent>>` which must be added to
the `handle_input` system parameters.

```antares/docs/explanation/buy_and_sell_plan.md#L1-1
// Pseudocode only — see candidate file section for real location
GameMode::Inventory(inv_state) => {
    // existing close-inventory path — unchanged
}
GameMode::Dialogue(dialogue_state) => {
    // NEW: check if NPC is a merchant
    if let Some(npc_id) = &dialogue_state.npc_id {
        if let Some(content) = game_content.as_deref() {
            if let Some(npc_def) = content.db().npcs.get_npc(npc_id) {
                if npc_def.is_merchant {
                    game_state.ensure_npc_runtime_initialized(&content.db());
                    game_state.enter_merchant_inventory(
                        npc_id.clone(),
                        npc_def.name.clone(),
                    );
                }
                // Non-merchant NPC: ignore I key press
            }
        }
    }
    return;
}
GameMode::Menu(_) | GameMode::Combat(_) => {
    // existing ignore path — unchanged
}
_ => {
    // existing open-inventory path — unchanged
    game_state.enter_inventory();
}
```

**Note:** If `DialogueState` does not currently carry an `npc_id` field, add
`pub npc_id: Option<NpcId>` to `DialogueState` in
`src/application/dialogue.rs`. Populate it in `handle_start_dialogue` in
`src/game/systems/dialogue.rs` when the dialogue is attached to an NPC entity.
The field defaults to `None` so existing serialised dialogue states and tests
are unaffected (use `#[serde(default)]` on the field).

#### 1.2.1 Verify `DialogueState::npc_id` exists

Read `src/application/dialogue.rs` before writing any code. If `npc_id` is
already present, skip the field addition. Do not add it twice.

### 1.3 Testing Requirements for Phase 1

All tests use `data/test_campaign`, not `campaigns/tutorial`.

**New tests in `src/game/systems/dialogue.rs`:**

- `test_open_merchant_dialogue_action_enters_merchant_inventory` — covered above
- `test_open_merchant_dialogue_action_unknown_npc_no_panic` — graceful
  degradation when NPC ID is not in content DB

**New integration tests in `src/game/systems/input.rs`:**

- `test_handle_input_i_in_dialogue_with_merchant_opens_merchant_inventory` —
  set up Bevy app with merchant NPC, put game into `Dialogue` mode with that
  NPC, press `I`, assert mode is `MerchantInventory`
- `test_handle_input_i_in_dialogue_with_non_merchant_does_not_open_inventory` —
  put game into `Dialogue` with a non-merchant NPC, press `I`, assert mode
  remains `Dialogue`
- `test_handle_input_i_in_dialogue_with_no_npc_id_does_nothing` — dialogue
  state has `npc_id: None`, press `I`, assert mode is unchanged

### 1.4 Deliverables

- [ ] `src/game/systems/dialogue.rs` — `OpenMerchant` arm calls
      `enter_merchant_inventory`
- [ ] `src/game/systems/input.rs` — `I` key in `Dialogue` mode opens merchant
      inventory only for merchant NPCs
- [ ] `src/application/dialogue.rs` — `DialogueState::npc_id` field added if
      missing
- [ ] Stub test replaced / new tests added and passing
- [ ] All four quality gates pass

### 1.5 Success Criteria

1. Starting a dialogue with a merchant NPC and pressing `I` transitions the
   game to `GameMode::MerchantInventory`.
2. Starting a dialogue with a non-merchant NPC and pressing `I` does nothing.
3. A `DialogueAction::OpenMerchant` node in a dialogue tree transitions the
   game to `GameMode::MerchantInventory` when executed.
4. No regressions in existing dialogue, input, or merchant inventory tests.

---

## Phase 2: Merchant UI — Price Display, Gold Feedback, and Error Feedback

**Goal:** The merchant trade screen shows item prices and current party gold.
Failed buy/sell attempts produce a visible feedback message (game log entry
and/or an inline status label in the UI panel) rather than a silent `warn!`.

### 2.1 Show Party Gold in the Merchant UI Header

**File:** `src/game/systems/merchant_inventory_ui.rs`

The header bar of the merchant UI already shows the character name and
merchant name. Extend it to display the party's current gold total:

```text
┌─────────────────────────────────────────────────────────────────────┐
│  Merchant Trade: [Character Name] ←→ [Merchant Name]   Gold: 1,234  │
└─────────────────────────────────────────────────────────────────────┘
```

Read `global_state.0.party.gold` in `merchant_inventory_ui_system` and format
it with thousands-separator grouping (use a helper `fn format_gold(g: u32) -> String`
defined at module scope).

### 2.2 Show Item Price in the Merchant Stock Panel

**File:** `src/game/systems/merchant_inventory_ui.rs`

In `render_merchant_stock_panel`, each stock row currently shows the item name
and quantity. Add a third column showing the buy price:

```text
│  [Item Name]                Qty: 3       Price: 45 gp              │
```

The price is computed by `MerchantStock::effective_price(item_id, item.base_cost)`
already available on the `StockEntry`. Retrieve the `base_cost` from
`game_content` (the `ItemDatabase`). If the item is not found in the database,
display `"? gp"`.

### 2.3 Show Sell Price in the Character Inventory Panel

**File:** `src/game/systems/merchant_inventory_ui.rs`

When a character inventory slot is highlighted in sell mode, show the expected
sell price below the action button strip:

```text
│  [ Sell ]   Sell value: 12 gp                                       │
```

Compute sell price using the same formula as `sell_item()` in
`src/domain/transactions.rs`:

1. `sell_cost` if non-zero, otherwise `base_cost / 2`
2. Multiplied by `npc_def.economy.buy_rate` (default 0.5), rounded down,
   minimum 1

The NPC definition is retrieved from `game_content.db().npcs.get_npc(npc_id)`.

### 2.4 Game Log Feedback for Failed Transactions

**File:** `src/game/systems/merchant_inventory_ui.rs`

In `merchant_inventory_action_system`, the buy and sell handlers currently
emit only `warn!` on failure. Add game log entries for every failure case:

| Failure case                      | Log message                                          |
| --------------------------------- | ---------------------------------------------------- |
| `InsufficientGold { have, need }` | `"Not enough gold. Need {need} gp, have {have} gp."` |
| `InventoryFull { character_id }`  | `"Inventory is full. Drop an item to make room."`    |
| `OutOfStock { item_id }`          | `"The merchant is out of stock for that item."`      |
| `ItemNotInInventory`              | `"You do not have that item."`                       |
| Cursed item sell attempt          | `"That item is cursed and cannot be sold."`          |

The `game_log` resource is already imported in other game systems. Add
`Option<ResMut<GameLog>>` to `merchant_inventory_action_system`'s parameters.

#### 2.4.1 Cursed Item Sell Guard

Before calling `sell_item()`, check `item.is_cursed`. If the item is cursed
and currently equipped, reject the sell with the cursed-item message. If the
item is in inventory but not equipped (unusual for a cursed item, but possible
as loot), allow the sell (the curse only applies to the equip/unequip cycle per
architecture Section 12.11).

Add the check:

```antares/docs/explanation/buy_and_sell_plan.md#L1-1
// In merchant_inventory_action_system sell branch, before calling sell_item():
if let Some(item_def) = game_content.db().items.get_item(item_id) {
    if item_def.is_cursed {
        // Check if it is currently equipped
        let is_equipped = character.equipment.is_item_equipped(item_id);
        if is_equipped {
            if let Some(ref mut log) = game_log {
                log.add("That item is cursed and cannot be removed or sold.");
            }
            continue;
        }
    }
}
```

If `Equipment::is_item_equipped(item_id: ItemId) -> bool` does not exist, add
it to `src/domain/character/equipment.rs` (or wherever `Equipment` is
implemented). The method checks all seven equipped slots.

### 2.5 Testing Requirements for Phase 2

**New unit tests in `src/game/systems/merchant_inventory_ui.rs`:**

- `test_format_gold_thousands_separator` — `format_gold(1234)` returns `"1,234"`
- `test_format_gold_zero` — `format_gold(0)` returns `"0"`

**New integration tests in `src/game/systems/merchant_inventory_ui.rs`:**

- `test_buy_action_insufficient_gold_adds_game_log_entry` — buy with 0 gold,
  verify game log contains expected message
- `test_buy_action_inventory_full_adds_game_log_entry` — fill inventory to
  `Inventory::MAX_ITEMS`, attempt buy, verify log message
- `test_sell_action_cursed_equipped_item_rejected` — equip a cursed item,
  attempt sell, verify the item remains equipped and log contains cursed message

### 2.6 Deliverables

- [ ] `src/game/systems/merchant_inventory_ui.rs` — gold display in header
- [ ] `src/game/systems/merchant_inventory_ui.rs` — price column in stock panel
- [ ] `src/game/systems/merchant_inventory_ui.rs` — sell-value preview in
      character panel
- [ ] `src/game/systems/merchant_inventory_ui.rs` — game log feedback on
      transaction failure
- [ ] Cursed-item sell guard (with `Equipment::is_item_equipped` if needed)
- [ ] All four quality gates pass

### 2.7 Success Criteria

1. Party gold is visible in the merchant UI header at all times.
2. Each stock entry shows a buy price before the player confirms.
3. When the player highlights a character inventory slot, the expected sell
   price is displayed.
4. Attempting to buy without enough gold produces a readable game log message.
5. Attempting to sell a cursed equipped item is rejected with a message.

---

## Phase 3: Container Interaction — `E`-Key Entry and State Persistence

**Goal:** Pressing `E` while facing a container tile event opens
`GameMode::ContainerInventory`. Container contents are written back to the map
event when the screen is closed, so partial takes persist within a session.

### 3.1 Audit `MapEvent` for Container Variant

**File:** `src/domain/world/events.rs` (or wherever `MapEvent` is defined)

Verify that a container variant (`MapEvent::Container` or similar) already
exists. If it does not, add:

```antares/docs/explanation/buy_and_sell_plan.md#L1-1
/// A container (chest, barrel, hole-in-the-wall, etc.) with takeable items.
Container {
    /// Unique identifier for this container instance (used as `container_event_id`).
    id: String,
    /// Display name shown in the right-panel header.
    name: String,
    /// Current items inside the container.
    items: Vec<InventorySlot>,
},
```

The `id` field is the `container_event_id` stored in `ContainerInventoryState`.
This is the key used to write the updated item list back to the map event on
close.

If the variant already exists (perhaps named differently), use the existing name
exactly. Do not add a duplicate.

### 3.2 Wire `E`-Key Container Interaction in `handle_events`

**File:** `src/game/systems/events.rs`

When `handle_events` processes a `MapEvent::Container { id, name, items }` (or
the equivalent existing variant), call:

```antares/docs/explanation/buy_and_sell_plan.md#L1-1
game_state.enter_container_inventory(id.clone(), name.clone(), items.clone());
```

This mirrors how `EventResult::EnterInn` triggers `enter_inn_management()`.

### 3.3 Persist Container State on Close

**File:** `src/game/systems/container_inventory_ui.rs`

When the player closes the container screen (presses `Esc` in
`container_inventory_input_system`), write the updated item list from
`ContainerInventoryState::items` back to the corresponding `MapEvent::Container`
in the current map:

1. Get `container_event_id` from the current `ContainerInventoryState`.
2. Find the matching `MapEvent` in `game_state.world.get_current_map_mut()`.
3. Replace its `items` field with `container_state.items.clone()`.
4. Restore previous mode.

The write-back must happen **before** mode is restored so that the updated
items are available for future interactions in the same session.

A helper function `fn write_container_items_back(game_state: &mut GameState, container_event_id: &str, items: Vec<InventorySlot>)`
can be added to the same file to keep the close handler readable.

### 3.4 Empty Container Display

**File:** `src/game/systems/container_inventory_ui.rs`

In `container_inventory_ui_system`, when `container_state.is_empty()` is
`true`, display a centred label `"(Empty)"` in the right panel body instead of
a slot grid. The **Take** and **Take All** buttons must be greyed out (disabled)
when the container is empty.

The `Take All` button is already wired to `TakeAllAction` in the action system.
Add a guard: if `is_empty()`, do not emit the action.

### 3.5 Container Map Data in `data/test_campaign`

Add at least one container map event to `data/test_campaign/data/maps/map_1.ron`
for integration testing. The container must have a non-empty `items` list so
that take and stash operations can be exercised without needing to set up state
programmatically.

Example RON fragment (adapt to the exact `MapEvent` field names used in the
project):

```antares/data/test_campaign/data/maps/map_1.ron#L1-1
(
    position: (x: 3, y: 3),
    event: Container(
        id: "test_chest_001",
        name: "Old Chest",
        items: [
            (item_id: 1, charges: 0),
            (item_id: 2, charges: 0),
        ],
    ),
),
```

### 3.6 Testing Requirements for Phase 3

All tests use `data/test_campaign`.

**New tests in `src/game/systems/container_inventory_ui.rs`:**

- `test_close_container_writes_items_back_to_map_event` — take one item,
  close screen, assert map event `items` list has one fewer entry
- `test_take_all_empties_container_and_writes_back` — take all, close, assert
  map event `items` is empty
- `test_stash_item_adds_to_container_and_writes_back` — stash one item, close,
  assert map event `items` list has one additional entry
- `test_empty_container_disables_take_button` — verify `TakeAllAction` not
  emitted when container is empty

**New integration test in `src/game/systems/events.rs`:**

- `test_container_map_event_enters_container_inventory_mode` — trigger a
  `MapEvent::Container`, assert game mode becomes `ContainerInventory`

### 3.7 Deliverables

- [ ] `MapEvent::Container` variant verified/added
- [ ] `src/game/systems/events.rs` — E key on container enters
      `ContainerInventory` mode
- [ ] `src/game/systems/container_inventory_ui.rs` — write-back on close
- [ ] `src/game/systems/container_inventory_ui.rs` — empty container display
      and disabled Take All
- [ ] `data/test_campaign/data/maps/map_1.ron` — at least one test container
- [ ] All four quality gates pass

### 3.8 Success Criteria

1. Pressing `E` while facing a container tile opens the split-screen container
   inventory.
2. After taking items and closing, re-interacting with the container shows the
   reduced contents.
3. An empty container shows `"(Empty)"` and the Take/Take All buttons are
   greyed out.
4. Stashing an item into a container persists the item on close.

---

## Phase 4: Mouse Support for Buy, Sell, Take, Take All, and Stash

**Goal:** All action buttons in the merchant and container inventory screens are
clickable with the mouse. Mouse hover highlights the button. Mouse click
executes the action identically to pressing `Enter` when the button is focused.

### 4.1 Merchant UI Mouse Support

**File:** `src/game/systems/merchant_inventory_ui.rs`

The egui `Button` widget already returns a `Response`. Add `.on_hover_cursor`
and check `response.clicked()` to fire the corresponding action writer.

Pattern for the **Buy** button in `render_merchant_stock_panel`:

```antares/docs/explanation/buy_and_sell_plan.md#L1-1
let response = ui.add(egui::Button::new("Buy").fill(BUY_COLOR));
if response.clicked() {
    if let Some(stock_idx) = merchant_state.merchant_selected_slot {
        buy_writer.write(BuyItemAction {
            npc_id: merchant_state.npc_id.clone(),
            stock_index: stock_idx,
            character_index: merchant_state.active_character_index,
        });
    }
}
```

Apply the same pattern to the **Sell** button in `render_character_sell_panel`.

Also wire mouse clicks on stock-row entries and inventory-slot cells to set
`merchant_selected_slot` / `character_selected_slot` directly on click, so the
player can click a row then click Buy/Sell in one pass instead of requiring
keyboard navigation to reach the row first.

### 4.2 Container UI Mouse Support

**File:** `src/game/systems/container_inventory_ui.rs`

Apply identical `response.clicked()` patterns to:

- **Take** button → emit `TakeItemAction`
- **Take All** button → emit `TakeAllAction`
- **Stash** button → emit `StashItemAction`

Wire mouse click on container-slot rows to set
`ContainerInventoryState::container_selected_slot`.

Wire mouse click on character-slot cells to set
`ContainerInventoryState::character_selected_slot`.

### 4.3 Shared Hover Highlight Colour

Both UIs use `SELECT_HIGHLIGHT_COLOR` (yellow) for keyboard-focused items.
Re-use the same constant for mouse-hovered rows/cells. When a row is both
keyboard-focused and mouse-hovered, the highlight colour is unchanged (no
visual conflict).

### 4.4 Testing Requirements for Phase 4

Bevy's egui test harness does not simulate mouse clicks at the widget level.
Cover mouse logic with unit tests on the helper functions and rely on the
existing keyboard-path integration tests to confirm that the same action writers
are fired correctly.

**New tests in `src/game/systems/merchant_inventory_ui.rs`:**

- `test_buy_item_action_via_click_matches_keyboard_action` — construct a
  `BuyItemAction` directly and send it through the action system; verify the
  same outcome as the keyboard path (gold deducted, inventory updated)
- `test_sell_item_action_via_click_matches_keyboard_action` — same pattern for
  sell

**New tests in `src/game/systems/container_inventory_ui.rs`:**

- `test_take_item_action_via_click_removes_from_container`
- `test_take_all_action_via_click_empties_container`
- `test_stash_item_action_via_click_adds_to_container`

### 4.5 Deliverables

- [ ] `src/game/systems/merchant_inventory_ui.rs` — Buy/Sell buttons clickable;
      stock rows and inventory cells clickable to set selection
- [ ] `src/game/systems/container_inventory_ui.rs` — Take/Take All/Stash
      buttons clickable; rows and cells clickable to set selection
- [ ] All four quality gates pass

### 4.6 Success Criteria

1. Clicking a stock row in the merchant panel sets it as the active selection.
2. Clicking the **Buy** button executes a buy for the selected row.
3. Clicking an inventory slot in the character panel sets it as the active
   selection.
4. Clicking the **Sell** button executes a sell for the selected slot.
5. Container Take, Take All, and Stash buttons all respond to mouse click.

---

## Phase 5: Tutorial Data Wiring, Save Persistence, and Documentation

**Goal:** The tutorial campaign merchant NPC dialogue nodes wire the
`OpenMerchant` action correctly. Bought/sold stock changes survive a save/load
cycle. Documentation is updated.

### 5.1 Wire `OpenMerchant` in Tutorial Merchant Dialogues

**File:** `campaigns/tutorial/data/dialogues.ron`

The tutorial merchant dialogue trees have a "Buy/Sell" choice branch but the
`actions` field on that choice is likely empty or absent. Add an
`OpenMerchant { npc_id: "tutorial_merchant_town" }` action to the choice node
that represents "I'd like to buy something" (or equivalent).

Repeat for `tutorial_merchant_town2` using its own `npc_id`.

Verify the dialogue IDs match those referenced in `campaigns/tutorial/data/npcs.ron`.

**Do NOT modify `campaigns/tutorial` for testing purposes.** The test campaign
at `data/test_campaign` is the fixture for all tests.

### 5.2 Wire `OpenMerchant` in Test Campaign Dialogues

**File:** `data/test_campaign/data/dialogues.ron`

Mirror the tutorial wiring: add `OpenMerchant { npc_id: "tutorial_merchant_town" }`
to the matching choice node in the test campaign merchant dialogue.

This is the only change allowed to `data/test_campaign` data files in this
phase beyond what Phase 3 required.

### 5.3 Verify Save/Load Preserves NPC Stock

**File:** `src/application/save_game.rs`

`NpcRuntimeStore` is already serialised via `#[serde(default)]` on
`GameState::npc_runtime`. Verify the round-trip with a targeted test:

Test name: `test_save_load_preserves_merchant_stock_after_buy`

1. Create a `GameState` with a merchant having 3 units of item 1.
2. Buy 1 unit (reduces stock to 2).
3. Serialise to RON with `save_game`.
4. Deserialise with `load_game`.
5. Assert the loaded state has 2 units of item 1 in the merchant stock.

If this test already exists and passes, document that it was verified (no
duplication needed).

### 5.4 Update `docs/explanation/implementations.md`

Add a new section summarising the buy/sell implementation:

- What was already present before this plan
- What each phase added
- File locations for all new/modified systems
- Known limitations (e.g. no per-character sell-price negotiation, no merchant
  "haggles" mechanic)

### 5.5 Testing Requirements for Phase 5

- `test_save_load_preserves_merchant_stock_after_buy` in
  `src/application/save_game.rs` (new or verified)
- `test_save_load_preserves_container_items_after_partial_take` in
  `src/application/save_game.rs` (new) — verify that partial container take
  survives save/load within the same map

### 5.6 Deliverables

- [ ] `campaigns/tutorial/data/dialogues.ron` — `OpenMerchant` action wired
      for both tutorial merchants
- [ ] `data/test_campaign/data/dialogues.ron` — `OpenMerchant` action wired
      for test merchant
- [ ] `src/application/save_game.rs` — stock persistence test added/verified
- [ ] `docs/explanation/implementations.md` — updated with buy/sell summary
- [ ] All four quality gates pass

### 5.7 Success Criteria

1. In the tutorial campaign, selecting the "buy/sell" choice in a merchant
   dialogue opens the merchant inventory screen.
2. Items bought from a merchant reduce the stock count and the reduction
   persists after saving and loading.
3. Items sold to a merchant appear in the merchant's stock (if the NPC already
   stocks that item type) and that change persists after save/load.
4. `docs/explanation/implementations.md` accurately describes the implemented
   system.

---

## Candidate Files Summary

### New Files

| File                                                 | Phase | Description                                                              |
| ---------------------------------------------------- | ----- | ------------------------------------------------------------------------ |
| `sdk/campaign_builder/src/stock_templates_editor.rs` | 7     | Full CRUD editor for `MerchantStockTemplate`; load/save to RON; 16 tests |

### Modified Files

| File                                              | Phase   | Nature of Change                                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------------------------------- | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/game/systems/dialogue.rs`                    | 1       | Wire `OpenMerchant` to `enter_merchant_inventory`                                                                                                                                                                                                                                                                                                                                                  |
| `src/application/dialogue.rs`                     | 1       | Add `npc_id: Option<NpcId>` to `DialogueState` (if absent)                                                                                                                                                                                                                                                                                                                                         |
| `src/game/systems/input.rs`                       | 1       | `I` key in `Dialogue` mode opens merchant inventory for merchant NPCs                                                                                                                                                                                                                                                                                                                              |
| `src/game/systems/merchant_inventory_ui.rs`       | 2, 4    | Gold display, price display, error feedback, mouse support                                                                                                                                                                                                                                                                                                                                         |
| `src/domain/character/equipment.rs`               | 2       | `is_item_equipped(item_id) -> bool` (if absent)                                                                                                                                                                                                                                                                                                                                                    |
| `src/game/systems/container_inventory_ui.rs`      | 3, 4    | Write-back on close, empty state, mouse support                                                                                                                                                                                                                                                                                                                                                    |
| `src/domain/world/events.rs`                      | 3, 7    | `MapEvent::Container` variant (Phase 3); used by SDK container event editor (Phase 7)                                                                                                                                                                                                                                                                                                              |
| `src/game/systems/events.rs`                      | 3       | Wire `E` on container to `enter_container_inventory`                                                                                                                                                                                                                                                                                                                                               |
| `data/test_campaign/data/maps/map_1.ron`          | 3       | Add test container event                                                                                                                                                                                                                                                                                                                                                                           |
| `data/test_campaign/data/dialogues.ron`           | 5       | Wire `OpenMerchant` action                                                                                                                                                                                                                                                                                                                                                                         |
| `campaigns/tutorial/data/dialogues.ron`           | 5       | Wire `OpenMerchant` action for both tutorial merchants                                                                                                                                                                                                                                                                                                                                             |
| `src/application/save_game.rs`                    | 5, 6    | Stock and container persistence tests (Phase 5); restock persistence tests (Phase 6)                                                                                                                                                                                                                                                                                                               |
| `docs/explanation/implementations.md`             | 5, 6, 7 | Implementation summary sections                                                                                                                                                                                                                                                                                                                                                                    |
| `src/domain/world/npc_runtime.rs`                 | 6       | `MerchantStockTemplate` gains `magic_item_pool` and `magic_refresh_days`; `NpcRuntimeState` gains `last_restock_day` and `magic_slots`; `restock_daily` and `restock_magic_slots` methods                                                                                                                                                                                                          |
| `src/domain/inventory.rs`                         | 6       | `MerchantStock` gains `restock_daily` helper                                                                                                                                                                                                                                                                                                                                                       |
| `src/application/mod.rs`                          | 6       | `advance_time` calls `npc_runtime.tick_restock`                                                                                                                                                                                                                                                                                                                                                    |
| `data/test_campaign/data/npc_stock_templates.ron` | 6       | Add `magic_item_pool` and `magic_refresh_days` to test templates                                                                                                                                                                                                                                                                                                                                   |
| `campaigns/tutorial/data/npc_stock_templates.ron` | 6       | Add `magic_item_pool` and `magic_refresh_days` to tutorial templates                                                                                                                                                                                                                                                                                                                               |
| `sdk/campaign_builder/src/lib.rs`                 | 7       | Add `pub mod stock_templates_editor`; add `StockTemplates` to `EditorTab`; add `stock_templates_editor_state`, `stock_templates`, `stock_templates_file` fields to `CampaignBuilderApp`; add `load_stock_templates`, `save_stock_templates_to_file`; render `StockTemplates` tab; extend `validate_npc_ids` and `validate_campaign`; add `stock_templates_file` to `CampaignMetadata`; 6 new tests |
| `sdk/campaign_builder/src/npc_editor.rs`          | 7       | Add `stock_template` to `NpcEditBuffer`; add `available_stock_templates` and `requested_template_edit` to `NpcEditorState`; extend `show_edit_view` Merchant section with stock-template drop-down and cross-navigation; update `save_npc` and `start_edit_npc`; 5 new tests                                                                                                                       |
| `sdk/campaign_builder/src/map_editor.rs`          | 7       | Add `Container` to `EventType`; add `EVENT_COLOR_CONTAINER` constant; add container fields to `EventEditorState`; wire `to_map_event` / `from_map_event`; render container item-list editor in `show_event_editor`; 8 new tests                                                                                                                                                                    |

---

## Quality Gate Checklist (Run After Every Phase)

```text
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

Expected results:

```text
✅ cargo fmt         → No output
✅ cargo check       → "Finished" with 0 errors
✅ cargo clippy      → "Finished" with 0 warnings
✅ cargo nextest run → all tests pass; 0 failed
```

**IF ANY GATE FAILS, STOP AND FIX BEFORE PROCEEDING TO THE NEXT PHASE.**

---

## Phase 6: Daily Restock and Magic Item Rotation

**Goal:** Merchant NPCs replenish their regular stock once per in-game day and
carry a small rotating slot of random magic items that refreshes on a
configurable cadence (default: every 7 days). Both the restock day and the
current magic-item slots are persisted in `NpcRuntimeState` so they survive
save/load cycles. No new `GameMode` variant or Bevy system is introduced; all
logic is pure-Rust domain code driven by the existing `advance_time` call.

---

### 6.1 Extend `MerchantStockTemplate` with Magic-Item Pool Fields

**File:** `src/domain/world/npc_runtime.rs`

Add two new `#[serde(default)]` fields to `MerchantStockTemplate` so that
existing `.ron` files deserialise without change:

```antares/src/domain/world/npc_runtime.rs#L85-95
pub struct MerchantStockTemplate {
    pub id: String,
    pub entries: Vec<TemplateStockEntry>,

    /// Pool of item IDs that may appear in the merchant's magic-item slots.
    ///
    /// At each magic refresh the engine picks `magic_slot_count` distinct
    /// items at random from this list. Duplicates in the list act as
    /// weighted entries (a doubled entry is twice as likely to be chosen).
    /// If the pool is empty, no magic slots are generated.
    #[serde(default)]
    pub magic_item_pool: Vec<ItemId>,

    /// How many random magic items appear in the shop at once.
    ///
    /// Defaults to 0 (no magic slots). Values above 0 activate the rotation.
    #[serde(default)]
    pub magic_slot_count: u8,

    /// Number of in-game days between magic-item slot refreshes.
    ///
    /// Defaults to 7. Must be ≥ 1; values of 0 are treated as 1.
    #[serde(default = "default_magic_refresh_days")]
    pub magic_refresh_days: u32,
}

fn default_magic_refresh_days() -> u32 { 7 }
```

**Important:** `magic_item_pool` contains `ItemId` values (`u32` type alias).
The pool is **definition data** (never mutated). The live magic items are held
in `NpcRuntimeState::magic_slots` (see §6.2).

---

### 6.2 Extend `NpcRuntimeState` with Restock Tracking Fields

**File:** `src/domain/world/npc_runtime.rs`

Add three new `#[serde(default)]` fields to `NpcRuntimeState`:

```antares/src/domain/world/npc_runtime.rs#L155-175
pub struct NpcRuntimeState {
    pub npc_id: NpcId,
    pub stock: Option<MerchantStock>,
    pub services_consumed: Vec<String>,

    /// The in-game day on which `stock` was last fully restocked from its
    /// template.  `0` means "never restocked this session" (forces an
    /// immediate restock on the first `tick_restock` call, which is the
    /// desired behaviour for a fresh or legacy-loaded save).
    #[serde(default)]
    pub last_restock_day: u32,

    /// Current magic-item slots: item IDs chosen at the last magic refresh.
    ///
    /// Each entry represents one unit of that magic item available for
    /// purchase. Entries are removed as items are bought (via the normal
    /// `MerchantStock` path — magic slots are injected into `stock.entries`
    /// at refresh time; see §6.3).
    #[serde(default)]
    pub magic_slots: Vec<ItemId>,

    /// The in-game day on which `magic_slots` was last refreshed.
    /// `0` means "never refreshed" (forces a refresh on first tick).
    #[serde(default)]
    pub last_magic_refresh_day: u32,
}
```

All three fields default to `0` / empty so deserialising a pre-Phase-6 save
produces the "never ticked" sentinel values, which cause an immediate restock
on the next `advance_time` call — correct behaviour.

---

### 6.3 Add `restock_daily` and `refresh_magic_slots` to `NpcRuntimeState`

**File:** `src/domain/world/npc_runtime.rs`

#### 6.3.1 `restock_daily`

````antares/src/domain/world/npc_runtime.rs#L1-1
/// Replenishes all regular stock entries back to the quantities defined in
/// `template`.
///
/// This method replaces each `StockEntry` quantity with the corresponding
/// template entry quantity.  Any entry whose `item_id` is not present in the
/// template (e.g. an item the player sold *to* the merchant) is left
/// unchanged — the merchant keeps what they were given.
///
/// # Arguments
///
/// * `template` - The template this merchant was initialised from.
///
/// # Examples
///
/// ```
/// use antares::domain::world::npc_runtime::{
///     NpcRuntimeState, MerchantStockTemplate, TemplateStockEntry,
/// };
///
/// let template = MerchantStockTemplate {
///     id: "basic".to_string(),
///     entries: vec![TemplateStockEntry { item_id: 1, quantity: 5, override_price: None }],
///     magic_item_pool: vec![],
///     magic_slot_count: 0,
///     magic_refresh_days: 7,
/// };
///
/// let mut state = NpcRuntimeState::initialize_stock_from_template(
///     "merchant_bob".to_string(), &template,
/// );
/// // Buy all stock
/// state.stock.as_mut().unwrap().entries[0].quantity = 0;
///
/// state.restock_daily(&template);
/// assert_eq!(state.stock.as_ref().unwrap().get_entry(1).unwrap().quantity, 5);
/// ```
pub fn restock_daily(&mut self, template: &MerchantStockTemplate) {
    let Some(stock) = self.stock.as_mut() else { return };
    for tmpl_entry in &template.entries {
        match stock.get_entry_mut(tmpl_entry.item_id) {
            Some(entry) => entry.quantity = tmpl_entry.quantity,
            None => stock.entries.push(StockEntry {
                item_id: tmpl_entry.item_id,
                quantity: tmpl_entry.quantity,
                override_price: tmpl_entry.override_price,
            }),
        }
    }
}
````

#### 6.3.2 `refresh_magic_slots`

````antares/src/domain/world/npc_runtime.rs#L1-1
/// Replaces the merchant's random magic-item slots with a freshly chosen
/// selection drawn from `template.magic_item_pool`.
///
/// **Selection algorithm**
///
/// 1. Remove any existing magic-slot entries from `stock.entries` (identified
///    by matching their `item_id` against the old `self.magic_slots` list).
/// 2. Choose `template.magic_slot_count` item IDs at random from
///    `template.magic_item_pool` without replacement within a single draw
///    (but duplicates in the pool increase selection probability).
/// 3. Add one `StockEntry` (quantity = 1, no price override) per chosen item
///    to `stock.entries`.
/// 4. Update `self.magic_slots` with the newly chosen item IDs.
///
/// A deterministic `seed` is accepted so tests can produce reproducible
/// results without depending on OS randomness.
///
/// # Arguments
///
/// * `template` - The template defining the magic-item pool and slot count.
/// * `seed`     - PRNG seed for reproducible selection (use `game_time.day`
///   combined with a stable NPC-specific value in production).
///
/// # Examples
///
/// ```
/// use antares::domain::world::npc_runtime::{
///     NpcRuntimeState, MerchantStockTemplate, TemplateStockEntry,
/// };
///
/// let template = MerchantStockTemplate {
///     id: "wizard_shop".to_string(),
///     entries: vec![],
///     magic_item_pool: vec![100, 101, 102, 103, 104],
///     magic_slot_count: 2,
///     magic_refresh_days: 7,
/// };
///
/// let mut state = NpcRuntimeState::new("wizard_zara".to_string());
/// state.stock = Some(antares::domain::inventory::MerchantStock::new());
/// state.refresh_magic_slots(&template, 42);
///
/// assert_eq!(state.magic_slots.len(), 2);
/// assert_eq!(
///     state.stock.as_ref().unwrap().entries.len(),
///     2,
///     "one stock entry per magic slot"
/// );
/// ```
pub fn refresh_magic_slots(&mut self, template: &MerchantStockTemplate, seed: u64) {
    let Some(stock) = self.stock.as_mut() else { return };

    // Step 1 — remove stale magic-slot entries
    let old_slots = std::mem::take(&mut self.magic_slots);
    for old_id in &old_slots {
        stock.entries.retain(|e| e.item_id != *old_id);
    }

    // Step 2 — pick new items from pool using a minimal LCG PRNG
    let count = template.magic_slot_count as usize;
    let pool = &template.magic_item_pool;
    if count == 0 || pool.is_empty() {
        return;
    }

    let mut rng = seed;
    let mut chosen: Vec<ItemId> = Vec::with_capacity(count);
    let mut available: Vec<ItemId> = pool.clone();

    for _ in 0..count {
        if available.is_empty() {
            break;
        }
        // LCG step: constants from Knuth TAOCP Vol.2
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let idx = (rng >> 33) as usize % available.len();
        chosen.push(available.remove(idx));
    }

    // Step 3 — add stock entries for chosen items
    for &item_id in &chosen {
        stock.entries.push(StockEntry {
            item_id,
            quantity: 1,
            override_price: None,
        });
    }

    // Step 4 — record new slots
    self.magic_slots = chosen;
}
````

**Why a hand-rolled LCG?** The project uses no external RNG crate and the
selection does not need cryptographic quality. The LCG is seeded with
`game_time.day ^ npc_id_hash` in production (see §6.4) so different merchants
on different days produce distinct selections. Tests supply an explicit seed.

---

### 6.4 Add `tick_restock` to `NpcRuntimeStore`

**File:** `src/domain/world/npc_runtime.rs`

````antares/src/domain/world/npc_runtime.rs#L1-1
/// Advances the restock clock for all merchants in the store.
///
/// Call this once per `advance_time` invocation, passing the **new**
/// `GameTime` (after the time advance has been applied).
///
/// For each NPC that has an active `MerchantStock`:
///
/// 1. **Daily restock** — if `new_day > last_restock_day`, replenish all
///    regular stock entries back to their template quantities.
/// 2. **Magic-slot refresh** — if the number of days elapsed since
///    `last_magic_refresh_day` meets or exceeds `template.magic_refresh_days`,
///    replace the magic slots with a freshly seeded selection.
///
/// Both operations are skipped for NPCs without a `restock_template`
/// or whose template cannot be found in `templates`.
///
/// # Arguments
///
/// * `new_time`  - The game time **after** the time advance.
/// * `templates` - The loaded template database.
///
/// # Examples
///
/// ```
/// use antares::domain::types::GameTime;
/// use antares::domain::world::npc_runtime::{
///     NpcRuntimeStore, NpcRuntimeState, MerchantStockTemplate,
///     MerchantStockTemplateDatabase, TemplateStockEntry,
/// };
/// use antares::domain::world::npc::NpcDefinition;
///
/// let mut templates = MerchantStockTemplateDatabase::new();
/// templates.add(MerchantStockTemplate {
///     id: "daily_shop".to_string(),
///     entries: vec![TemplateStockEntry { item_id: 1, quantity: 3, override_price: None }],
///     magic_item_pool: vec![],
///     magic_slot_count: 0,
///     magic_refresh_days: 7,
/// });
///
/// let mut merchant = NpcDefinition::merchant("bob", "Bob", "bob.png");
/// merchant.stock_template = Some("daily_shop".to_string());
///
/// let mut store = NpcRuntimeStore::new();
/// store.initialize_merchant(&merchant, &templates);
///
/// // Deplete stock
/// store.get_mut(&"bob".to_string()).unwrap()
///     .stock.as_mut().unwrap().entries[0].quantity = 0;
///
/// // Advance to day 2
/// let day2 = GameTime::new(2, 6, 0);
/// store.tick_restock(&day2, &templates);
///
/// // Stock should be replenished
/// assert_eq!(
///     store.get(&"bob".to_string()).unwrap()
///         .stock.as_ref().unwrap().get_entry(1).unwrap().quantity,
///     3
/// );
/// ```
pub fn tick_restock(
    &mut self,
    new_time: &crate::domain::types::GameTime,
    templates: &MerchantStockTemplateDatabase,
) {
    let new_day = new_time.day;

    // Collect NPC IDs to avoid borrowing self while iterating
    let npc_ids: Vec<NpcId> = self.npcs.keys().cloned().collect();

    for npc_id in npc_ids {
        // Borrow as mutable for this NPC only
        let state = match self.npcs.get_mut(&npc_id) {
            Some(s) => s,
            None => continue,
        };

        // Only process NPCs that have stock
        let template_id = match state.stock.as_ref().and_then(|s| s.restock_template.clone()) {
            Some(id) => id,
            None => continue,
        };

        let template = match templates.get(&template_id) {
            Some(t) => t.clone(),
            None => continue,
        };

        // --- Daily restock ---
        if new_day > state.last_restock_day {
            state.restock_daily(&template);
            state.last_restock_day = new_day;
        }

        // --- Magic-slot refresh ---
        if template.magic_slot_count > 0 && !template.magic_item_pool.is_empty() {
            let refresh_interval = template.magic_refresh_days.max(1);
            let days_since_refresh = new_day.saturating_sub(state.last_magic_refresh_day);
            if days_since_refresh >= refresh_interval
                || state.last_magic_refresh_day == 0
            {
                // Build a deterministic seed from the day and NPC ID
                let npc_hash: u64 = npc_id
                    .bytes()
                    .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
                let seed = (new_day as u64).wrapping_mul(2654435761).wrapping_add(npc_hash);
                state.refresh_magic_slots(&template, seed);
                state.last_magic_refresh_day = new_day;
            }
        }
    }
}
````

---

### 6.5 Wire `tick_restock` into `GameState::advance_time`

**File:** `src/application/mod.rs`

The existing `advance_time` method ticks active spell durations. Extend it to
also call `npc_runtime.tick_restock` after the time advance. The
`ContentDatabase` is needed for the template lookup; it is obtained from the
`GameState::campaign` field's loaded content, but since `advance_time` does not
currently take a content parameter the simplest correct design is to add an
optional `templates` parameter:

````antares/src/application/mod.rs#L1-1
/// Advances game time by the specified number of minutes.
///
/// After advancing, active spell durations are ticked and merchant NPC stock
/// is restocked / magic slots are rotated if a new in-game day has begun.
///
/// # Arguments
///
/// * `minutes`   - Number of in-game minutes to advance.
/// * `templates` - Template database used to replenish merchant stock.
///   Pass `None` in contexts where the content is not available (e.g.
///   headless unit tests that do not load campaign data); restocking is
///   silently skipped in that case.
///
/// # Examples
///
/// ```
/// use antares::application::GameState;
/// use antares::domain::world::npc_runtime::MerchantStockTemplateDatabase;
///
/// let mut state = GameState::new();
/// let templates = MerchantStockTemplateDatabase::new();
/// state.advance_time(60, Some(&templates));
/// assert_eq!(state.time.minute, 0);
/// assert_eq!(state.time.hour, 1);
/// ```
pub fn advance_time(
    &mut self,
    minutes: u32,
    templates: Option<&crate::domain::world::npc_runtime::MerchantStockTemplateDatabase>,
) {
    self.time.advance_minutes(minutes);
    for _ in 0..minutes {
        self.active_spells.tick();
    }
    if let Some(tmpl) = templates {
        self.npc_runtime.tick_restock(&self.time, tmpl);
    }
}
````

**Call-site audit:** Every call to `advance_time` in the codebase must be
updated to pass `Some(&content.npc_stock_templates)` when a `GameContent`
resource is available, or `None` where it is not (tests, headless tools). Use
`grep -rn 'advance_time'` to find all call sites before submitting.

---

### 6.6 Update `npc_stock_templates.ron` Data Files

#### 6.6.1 `data/test_campaign/data/npc_stock_templates.ron`

Add `magic_item_pool`, `magic_slot_count`, and `magic_refresh_days` to the
existing `"tutorial_merchant_stock"` template so the full rotation path is
exercised by integration tests:

```antares/data/test_campaign/data/npc_stock_templates.ron#L1-1
(
    id: "tutorial_merchant_stock",
    entries: [
        (item_id: 1, quantity: 2, override_price: None),
        (item_id: 2, quantity: 2, override_price: None),
        (item_id: 3, quantity: 1, override_price: None),
        (item_id: 20, quantity: 2, override_price: None),
        (item_id: 23, quantity: 2, override_price: None),
        (item_id: 50, quantity: 3, override_price: None),
    ],
    magic_item_pool: [101, 102, 103, 104, 105],
    magic_slot_count: 2,
    magic_refresh_days: 7,
),
```

The `"tutorial_blacksmith_stock"` template keeps `magic_slot_count: 0` (no
magic items) to verify the disabled-rotation path.

#### 6.6.2 `campaigns/tutorial/data/npc_stock_templates.ron`

Apply the same field additions to the production tutorial templates. Use the
same magic item IDs as the test campaign (they reference items that already
exist in `campaigns/tutorial/data/items.ron`). `magic_slot_count` values must
not exceed the size of `magic_item_pool`. Both fields default to `0`/`[]` via
`#[serde(default)]` so no existing gameplay is affected until you explicitly
set `magic_slot_count > 0`.

**Verify** that the IDs in `magic_item_pool` resolve to items with
`is_magical() == true` in the item database. Non-magical items in the pool are
accepted by the engine but should be avoided by data authors (document this
constraint in the RON file comments).

---

### 6.7 Testing Requirements for Phase 6

All tests live in the files modified by this phase. All test data uses
`data/test_campaign`, never `campaigns/tutorial`.

#### `src/domain/world/npc_runtime.rs` — new unit tests

| Test name                                                  | Verifies                                                                            |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `test_restock_daily_restores_depleted_quantities`          | After buying out an item, `restock_daily` brings quantities back to template values |
| `test_restock_daily_preserves_non_template_items`          | Items sold _to_ the merchant (not in template) are not removed by `restock_daily`   |
| `test_restock_daily_noop_on_no_stock`                      | Calling `restock_daily` on an NPC with `stock: None` does not panic                 |
| `test_refresh_magic_slots_populates_correct_count`         | After `refresh_magic_slots`, `magic_slots.len() == magic_slot_count`                |
| `test_refresh_magic_slots_entries_added_to_stock`          | `stock.entries` gains one entry per slot, each with `quantity == 1`                 |
| `test_refresh_magic_slots_removes_old_slots`               | Calling `refresh_magic_slots` twice removes the first set before adding the second  |
| `test_refresh_magic_slots_noop_when_pool_empty`            | `magic_slot_count > 0` but empty pool → no slots added, no panic                    |
| `test_refresh_magic_slots_capped_by_pool_size`             | `magic_slot_count` larger than pool size → slots capped to pool length              |
| `test_refresh_magic_slots_reproducible_with_same_seed`     | Same seed → same selection                                                          |
| `test_refresh_magic_slots_different_seed_different_result` | Different seeds → different selections (probabilistic; use a pool of ≥5 items)      |
| `test_tick_restock_triggers_on_new_day`                    | `tick_restock` on day 2 when `last_restock_day == 1` restocks quantities            |
| `test_tick_restock_no_restock_same_day`                    | `tick_restock` on day 1 when `last_restock_day == 1` does not change quantities     |
| `test_tick_restock_updates_last_restock_day`               | After `tick_restock`, `last_restock_day == new_time.day`                            |
| `test_tick_restock_magic_refresh_on_interval`              | After `magic_refresh_days` days `tick_restock` calls `refresh_magic_slots`          |
| `test_tick_restock_magic_no_refresh_before_interval`       | Before interval is reached magic slots are unchanged                                |
| `test_tick_restock_initial_zero_day_forces_restock`        | `last_restock_day == 0` forces a restock even on day 1                              |
| `test_tick_restock_skips_npc_without_template`             | NPCs with `stock: None` or no `restock_template` are silently skipped               |

#### `src/application/mod.rs` — updated tests

Update `test_advance_time_ticks_spells` to pass `None` for `templates` (the
new parameter). Verify the test still passes.

Add:

| Test name                                        | Verifies                                                                        |
| ------------------------------------------------ | ------------------------------------------------------------------------------- |
| `test_advance_time_triggers_restock`             | `advance_time` with a non-`None` templates arg and a day boundary calls restock |
| `test_advance_time_no_restock_without_templates` | `advance_time(None)` does not panic and does not alter stock                    |

#### `src/application/save_game.rs` — new test

| Test name                                              | Verifies                                                                                                                                 |
| ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `test_save_load_preserves_restock_day_and_magic_slots` | Serialise a `GameState` with `last_restock_day == 3` and `magic_slots: [101, 102]`; deserialise; assert both fields round-trip correctly |

---

### 6.8 Deliverables

- [ ] `src/domain/world/npc_runtime.rs` — `MerchantStockTemplate` extended;
      `NpcRuntimeState` extended; `restock_daily`, `refresh_magic_slots`, and
      `tick_restock` implemented and documented
- [ ] `src/application/mod.rs` — `advance_time` signature updated; all
      existing call sites patched; new tests added
- [ ] `data/test_campaign/data/npc_stock_templates.ron` — magic-item fields
      added to `"tutorial_merchant_stock"` template
- [ ] `campaigns/tutorial/data/npc_stock_templates.ron` — magic-item fields
      added to production templates
- [ ] All unit tests listed in §6.7 implemented and passing
- [ ] All four quality gates pass

---

### 6.9 Success Criteria

1. After the party rests overnight (or `advance_time` crosses a day boundary),
   a merchant that had sold-out items now shows full stock again.
2. A merchant configured with `magic_slot_count: 2` shows exactly 2 magic
   items in their shop, chosen from the `magic_item_pool`.
3. After `magic_refresh_days` in-game days have elapsed the magic items change
   to a new selection drawn from the pool.
4. Saving and loading preserves `last_restock_day`, `magic_slots`, and
   `last_magic_refresh_day` so stock and magic slots do not reset unexpectedly
   on load.
5. A merchant with `magic_slot_count: 0` or an empty `magic_item_pool` shows
   no magic items and no errors are logged.
6. All existing tests (including Phases 1–5) continue to pass — the `None`
   sentinel for `advance_time` templates preserves backward-compatible
   behaviour.

---

## Architecture Compliance Checklist

- [ ] `ItemId`, `NpcId`, `CharacterId` type aliases used — no raw `u32`/`usize`
      for domain identifiers
- [ ] `Inventory::MAX_ITEMS` constant referenced — never hardcoded numeric limit
- [ ] `Equipment::MAX_EQUIPPED` constant referenced — never hardcoded `7`
- [ ] All test data in `data/test_campaign` — no test references
      `campaigns/tutorial`
- [ ] RON format for all data files — no JSON or YAML for game content
- [ ] `///` doc comments on every new public function, struct, enum
- [ ] `AttributePair` pattern used if any stat modification is added
- [ ] `NpcRuntimeStore` serialised via existing `#[serde(default)]` — no
      parallel serialisation path introduced
- [ ] No architectural deviations from `docs/reference/architecture.md`

### Phase 7 — SDK-Specific Checklist

- [ ] Every loop in new egui code uses `ui.push_id` with a stable unique key
      (item ID string, template ID, or loop index combined with item ID)
- [ ] Every `ScrollArea` in new egui code has a distinct `id_salt` string
- [ ] Every `ComboBox` in new egui code uses `ComboBox::from_id_salt` (never
      `from_label`)
- [ ] No `SidePanel`, `TopBottomPanel`, or `CentralPanel` is gated by a
      same-frame boolean guard — show a placeholder widget instead
- [ ] Every layout-driving state mutation (mode switches, selection changes)
      calls `ui.ctx().request_repaint()`
- [ ] `stock_templates_editor.rs` follows the two-column list+preview layout
      pattern established by `npc_editor.rs` and `items_editor.rs`
- [ ] `NpcEditBuffer::stock_template` serialises to/from `NpcDefinition::stock_template`
      without data loss (`None` ↔ empty string, `Some(id)` ↔ non-empty string)
- [ ] `EventEditorState` container fields default to `vec![]` / `false` /
      `String::new()` so existing non-container events are not affected
- [ ] `EventType::Container` is included in `EventType::all()` so the event
      type picker shows it in the Map Editor inspector
- [ ] `validate_npc_ids` and `validate_campaign` cross-checks run as part of
      the existing `validate_campaign` call path — no new validation entry
      point introduced

---

## Phase 7: Campaign Builder — Stock Template and Container Item Editor

**Goal:** The Campaign Builder SDK gains two new authoring surfaces:

1. **Stock Template Editor** — a dedicated tab (`StockTemplates`) under
   `sdk/campaign_builder` where content authors can create, edit, and delete
   `MerchantStockTemplate` entries in `npc_stock_templates.ron`. The NPC editor
   gains a stock-template assignment drop-down so authors can link a merchant
   NPC to a template without hand-editing RON files.

2. **Container Event Item Editor** — the Map Editor's `EventEditorState` gains
   a `Container` event type. In the event inspector panel authors can compose
   the container's initial item list by picking from the campaign item database.
   The editor serialises to and from `MapEvent::Container` in
   `src/domain/world/events.rs` (added by Phase 3).

Both surfaces follow all SDK `AGENTS.md` egui rules: every loop uses
`push_id`, every `ScrollArea` has a unique `id_salt`, and every `ComboBox`
uses `from_id_salt`.

---

### 7.1 New File: `sdk/campaign_builder/src/stock_templates_editor.rs`

**File:** `sdk/campaign_builder/src/stock_templates_editor.rs`

This module owns all state and rendering for the stock-template authoring
surface. It mirrors the structure and conventions of the existing
`npc_editor.rs` — a `StockTemplatesEditorState` struct, a
`StockTemplateEditBuffer` for form fields, a `StockTemplatesEditorMode` enum,
and a `show()` entry point that returns `bool` (`true` = needs save).

#### 7.1.1 `StockTemplatesEditorMode`

```
pub enum StockTemplatesEditorMode {
    List,
    Add,
    Edit,
}
```

#### 7.1.2 `StockTemplateEditBuffer`

One buffer field per editable property of `MerchantStockTemplate`.
All fields are `String` (parsed on save) except `entries` and
`magic_item_pool`, which use structured sub-buffers:

```
pub struct StockTemplateEditBuffer {
    /// Template ID (e.g. "blacksmith_basic_stock")
    pub id: String,

    /// Human-readable description shown in the editor list
    pub description: String,

    /// Editable list of regular stock entries
    pub entries: Vec<TemplateEntryBuffer>,

    /// Item IDs in the magic-item rotation pool (one per line in the UI)
    pub magic_item_pool: Vec<String>,

    /// Number of magic slots shown at once (stringified u8)
    pub magic_slot_count: String,

    /// Days between magic-slot refreshes (stringified u32)
    pub magic_refresh_days: String,
}

pub struct TemplateEntryBuffer {
    /// Item ID (stringified u8 / ItemId)
    pub item_id: String,
    /// Restock quantity (stringified u8)
    pub quantity: String,
    /// Optional price override (empty = use item base_cost)
    pub override_price: String,
}
```

Conversion helpers `StockTemplateEditBuffer::from_template` and
`StockTemplateEditBuffer::to_template` handle parse/validate and produce
descriptive `Vec<String>` error lists identical to the pattern in
`npc_editor.rs` `validate_edit_buffer`.

#### 7.1.3 `StockTemplatesEditorState`

```
pub struct StockTemplatesEditorState {
    pub templates: Vec<MerchantStockTemplate>,
    pub selected_template: Option<usize>,
    pub mode: StockTemplatesEditorMode,
    pub edit_buffer: StockTemplateEditBuffer,
    pub search_filter: String,
    pub has_unsaved_changes: bool,
    pub validation_errors: Vec<String>,
    pub last_campaign_dir: Option<PathBuf>,   // #[serde(skip)]
    pub last_templates_file: Option<String>,  // #[serde(skip)]

    /// Item database snapshot for item-name lookups in the entry list.
    /// Refreshed from the caller on every `show()` call.
    pub available_items: Vec<Item>,           // #[serde(skip)]
}
```

#### 7.1.4 `StockTemplatesEditorState::show`

Signature:

```
pub fn show(
    &mut self,
    ui: &mut egui::Ui,
    available_items: &[Item],
    campaign_dir: Option<&PathBuf>,
    templates_file: &str,
) -> bool
```

**List view** — a two-column layout. Left column: a `ScrollArea` with
`id_salt("stock_tmpl_list_scroll")` listing each template as a
`selectable_label`. Clicking a row selects it and populates a read-only
preview in the right column. The toolbar (using `EditorToolbar::new`) provides
**New**, **Edit** (opens the selected template in edit mode), **Delete** (with
a confirmation), and **Export** buttons.

**Edit / Add view** — a `ScrollArea` with `id_salt("stock_tmpl_edit_scroll")`
containing three `ui.group` sections:

1. **Identity** — `id` (`TextEdit` with `id_salt("stmpl_edit_id")`) and
   `description` (`TextEdit` with `id_salt("stmpl_edit_description")`).

2. **Regular Stock Entries** — a header row labelling the three columns
   (`Item`, `Qty`, `Price Override`) followed by one row per entry rendered
   inside a `ScrollArea` with `id_salt("stmpl_entries_scroll")`. Each row
   is wrapped in `ui.push_id(format!("stmpl_entry_{}", i), ...)`. The `Item`
   cell uses a `ComboBox::from_id_salt(format!("stmpl_item_sel_{}", i))`
   populated from `available_items` (showing `id - name`). **Add Entry** and
   per-row **✕** (remove) buttons mutate the buffer. An **↑** / **↓** pair
   on each row allows reordering — both wrapped in
   `push_id(format!("stmpl_move_{}", i), ...)`.

3. **Magic Item Rotation** — three fields:
   - `magic_slot_count` (`DragValue` clamped 0–255, `id_salt("stmpl_magic_count")`)
   - `magic_refresh_days` (`DragValue` clamped 1–365, `id_salt("stmpl_magic_days")`)
   - `magic_item_pool` — a `ScrollArea` with `id_salt("stmpl_magic_pool_scroll")`
     listing pool entries. Each entry is a
     `ComboBox::from_id_salt(format!("stmpl_pool_item_{}", i))` populated from
     `available_items` filtered to `item.is_magical() == true`. An **Add to Pool**
     button appends a blank entry; each row has a **✕** remove button. Both
     the list and the buttons use `push_id(format!("stmpl_pool_{}", i), ...)`.

Below the groups: a validation-error panel (same style as `npc_editor.rs`) and
an action button strip — **⬅ Back**, **💾 Save**, **❌ Cancel** — where Save
calls `validate_edit_buffer`, then on success persists to the RON file at
`campaign_dir.join(templates_file)` using `ron::ser::to_string_pretty`.

#### 7.1.5 Load / Save helpers

```
pub fn load_from_file(&mut self, path: &Path) -> Result<(), String>
pub fn save_to_file(&self, path: &Path) -> Result<(), String>
```

Both use `ron` (de)serialisation over `Vec<MerchantStockTemplate>`. Errors
produce `String` messages that the caller forwards to the status bar, matching
the pattern in `CampaignBuilderApp::load_npcs` / `save_npcs_to_file`.

#### 7.1.6 Validation rules

| Field                       | Rule                                                                           |
| --------------------------- | ------------------------------------------------------------------------------ |
| `id`                        | Non-empty; matches `[a-z0-9_]+`; unique among existing templates (in add mode) |
| `entries[*].item_id`        | Parseable as `ItemId` (`u8`); non-zero                                         |
| `entries[*].quantity`       | Parseable as `u8`; ≥ 1                                                         |
| `entries[*].override_price` | Empty or parseable as `u32`                                                    |
| `magic_slot_count`          | Parseable as `u8`                                                              |
| `magic_refresh_days`        | Parseable as `u32`; ≥ 1 (treat 0 as 1 with a warning)                          |
| `magic_item_pool[*]`        | Parseable as `ItemId`; warning if item not found in `available_items`          |
| `magic_slot_count`          | Warning if `magic_slot_count > magic_item_pool.len()`                          |

---

### 7.2 Wire `StockTemplatesEditorState` into `CampaignBuilderApp`

**File:** `sdk/campaign_builder/src/lib.rs`

#### 7.2.1 Add `pub mod stock_templates_editor`

Add to `lib.rs` module declarations (alphabetically after `spells_editor`):

```
pub mod stock_templates_editor;
```

#### 7.2.2 Add `StockTemplates` to `EditorTab`

```
enum EditorTab {
    // … existing variants …
    StockTemplates,   // ← new, placed between Proficiencies and Assets
}
```

Update `EditorTab::name`:

```
EditorTab::StockTemplates => "Stock Templates",
```

#### 7.2.3 Add fields to `CampaignBuilderApp`

```
/// Stock template editor state (NPC restock definitions)
stock_templates_editor_state: StockTemplatesEditorState,

/// Loaded stock templates
stock_templates: Vec<MerchantStockTemplate>,

/// Filename for npc_stock_templates.ron relative to campaign data dir
stock_templates_file: String,
```

Default values:

```
stock_templates_editor_state: StockTemplatesEditorState::default(),
stock_templates: Vec::new(),
stock_templates_file: "data/npc_stock_templates.ron".to_string(),
```

#### 7.2.4 Add `CampaignMetadata::stock_templates_file` field

```
/// Relative path to the NPC stock templates RON file
#[serde(default = "default_stock_templates_file")]
pub stock_templates_file: String,
```

```
fn default_stock_templates_file() -> String {
    "data/npc_stock_templates.ron".to_string()
}
```

This mirrors the pattern of `default_proficiencies_file` and
`default_creatures_file` already in `CampaignMetadata`.

#### 7.2.5 Add `load_stock_templates` and `save_stock_templates_to_file`

```
fn load_stock_templates(&mut self) {
    if let Some(dir) = &self.campaign_dir {
        let path = dir.join(&self.campaign.stock_templates_file);
        match self.stock_templates_editor_state.load_from_file(&path) {
            Ok(()) => {
                self.stock_templates =
                    self.stock_templates_editor_state.templates.clone();
                self.status_message =
                    format!("Loaded {} stock templates", self.stock_templates.len());
            }
            Err(e) => {
                self.status_message =
                    format!("Warning: could not load stock templates: {}", e);
            }
        }
    }
}

fn save_stock_templates_to_file(&mut self) {
    if let Some(dir) = &self.campaign_dir {
        let path = dir.join(&self.campaign.stock_templates_file);
        match self.stock_templates_editor_state.save_to_file(&path) {
            Ok(()) => {
                self.status_message = "Stock templates saved.".to_string();
                self.unsaved_changes = false;
            }
            Err(e) => {
                self.status_message =
                    format!("Error saving stock templates: {}", e);
            }
        }
    }
}
```

Call `load_stock_templates()` inside `do_open_campaign` after
`load_npcs()` (the two files are typically loaded together).
Call `save_stock_templates_to_file()` inside `do_save_campaign` after
`save_npcs_to_file()`.

#### 7.2.6 Render the `StockTemplates` tab in `eframe::App::update`

Inside the `match self.active_tab` arm (which already dispatches to each
editor panel), add:

```
EditorTab::StockTemplates => {
    let needs_save = self.stock_templates_editor_state.show(
        ui,
        &self.items,
        self.campaign_dir.as_ref(),
        &self.campaign.stock_templates_file,
    );
    if needs_save {
        self.stock_templates =
            self.stock_templates_editor_state.templates.clone();
        self.unsaved_changes = true;
    }
}
```

---

### 7.3 Extend `NpcEditBuffer` with `stock_template` Field

**File:** `sdk/campaign_builder/src/npc_editor.rs`

#### 7.3.1 Add field to `NpcEditBuffer`

```
/// ID of the stock template this merchant uses (empty = no template)
pub stock_template: String,
```

Default: `String::new()`.

#### 7.3.2 Extend `show_edit_view` Merchant section

In `show_edit_view`, the existing **Faction & Roles** group already renders
`is_merchant` and `is_innkeeper` checkboxes. Immediately after the
`is_merchant` checkbox, add a conditional block:

```
if self.edit_buffer.is_merchant {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Stock Template:");
        egui::ComboBox::from_id_salt("npc_edit_stock_template_select")
            .selected_text(if self.edit_buffer.stock_template.is_empty() {
                "None (no stock)".to_string()
            } else {
                self.edit_buffer.stock_template.clone()
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(
                        self.edit_buffer.stock_template.is_empty(),
                        "None (no stock)",
                    )
                    .clicked()
                {
                    self.edit_buffer.stock_template.clear();
                }
                for tmpl in &self.available_stock_templates {
                    ui.push_id(&tmpl.id, |ui| {
                        if ui
                            .selectable_label(
                                self.edit_buffer.stock_template == tmpl.id,
                                &tmpl.id,
                            )
                            .on_hover_text(format!(
                                "{} entries, {} magic slots",
                                tmpl.entries.len(),
                                tmpl.magic_slot_count
                            ))
                            .clicked()
                        {
                            self.edit_buffer.stock_template = tmpl.id.clone();
                        }
                    });
                }
            });
        if !self.edit_buffer.stock_template.is_empty() {
            if ui.small_button("✏ Edit template").clicked() {
                // Signal the caller to navigate to the Stock Templates tab
                // and open this template for editing.
                self.requested_template_edit =
                    Some(self.edit_buffer.stock_template.clone());
            }
        }
    });
}
```

#### 7.3.3 Add `available_stock_templates` and `requested_template_edit` to `NpcEditorState`

```
/// Stock templates available for merchant assignment (populated by caller)
#[serde(skip)]
pub available_stock_templates: Vec<MerchantStockTemplate>,

/// Set by the UI when the user clicks "✏ Edit template"; consumed by
/// `CampaignBuilderApp` to switch tab and open the named template.
#[serde(skip)]
pub requested_template_edit: Option<String>,
```

#### 7.3.4 Thread `available_stock_templates` from `CampaignBuilderApp`

In `CampaignBuilderApp::update`, before calling
`self.npc_editor_state.show(...)`, set:

```
self.npc_editor_state.available_stock_templates =
    self.stock_templates.clone();
```

After `show()` returns, consume `requested_template_edit`:

```
if let Some(tmpl_id) =
    self.npc_editor_state.requested_template_edit.take()
{
    self.active_tab = EditorTab::StockTemplates;
    self.stock_templates_editor_state
        .open_template_for_edit(&tmpl_id);
}
```

Add the helper `open_template_for_edit` to `StockTemplatesEditorState`:

```
pub fn open_template_for_edit(&mut self, id: &str) {
    if let Some(idx) = self.templates.iter().position(|t| t.id == id) {
        self.selected_template = Some(idx);
        self.edit_buffer =
            StockTemplateEditBuffer::from_template(&self.templates[idx]);
        self.mode = StockTemplatesEditorMode::Edit;
    }
}
```

#### 7.3.5 Persist `stock_template` through `save_npc` / `start_edit_npc`

In `save_npc`, map `edit_buffer.stock_template` →
`NpcDefinition::stock_template`:

```
npc.stock_template = if self.edit_buffer.stock_template.is_empty() {
    None
} else {
    Some(self.edit_buffer.stock_template.clone())
};
```

In `start_edit_npc`, populate `edit_buffer.stock_template`:

```
self.edit_buffer.stock_template =
    npc.stock_template.clone().unwrap_or_default();
```

---

### 7.4 Add `Container` Event Type to the Map Editor

**File:** `sdk/campaign_builder/src/map_editor.rs`

#### 7.4.1 Add `Container` variant to `EventType`

```
pub enum EventType {
    Encounter,
    Treasure,
    Teleport,
    Trap,
    Sign,
    NpcDialogue,
    RecruitableCharacter,
    EnterInn,
    Furniture,
    Container,   // ← new
}
```

Update all `match` arms that are exhaustive over `EventType`:

- `name()` → `"Container"`
- `icon()` → `"📦"`
- `color()` → a new constant `EVENT_COLOR_CONTAINER` (teal:
  `Color32::from_rgb(0, 180, 160)`)
- `all()` → append `EventType::Container`

Add the new constant at module top alongside the existing colour constants:

```
const EVENT_COLOR_CONTAINER: egui::Color32 =
    egui::Color32::from_rgb(0, 180, 160);
```

#### 7.4.2 Add container fields to `EventEditorState`

```
/// Items in the container's initial inventory (item IDs)
pub container_items: Vec<ItemId>,

/// Freeform input buffer for adding an item to the container list
pub container_item_input: String,

/// Whether the container starts locked
pub container_locked: bool,

/// Optional description override shown when the container is opened
pub container_description: String,
```

Defaults: `container_items: vec![]`, `container_item_input: String::new()`,
`container_locked: false`, `container_description: String::new()`.

#### 7.4.3 Wire `Container` in `EventEditorState::to_map_event`

```
EventType::Container => {
    Ok(MapEvent::Container {
        name: self.name.clone(),
        description: if self.container_description.is_empty() {
            self.description.clone()
        } else {
            self.container_description.clone()
        },
        items: self.container_items.clone(),
        locked: self.container_locked,
    })
}
```

#### 7.4.4 Wire `Container` in `EventEditorState::from_map_event`

```
MapEvent::Container { name, description, items, locked } => {
    s.event_type = EventType::Container;
    s.name = name.clone();
    s.description = description.clone();
    s.container_items = items.clone();
    s.container_locked = *locked;
}
```

#### 7.4.5 Render the Container fields in `show_event_editor`

**File:** `sdk/campaign_builder/src/lib.rs`
**Function:** `CampaignBuilderApp::show_event_editor`

After the existing `EventType::Furniture` block, add a `Container` block:

```
EventType::Container => {
    ui.separator();
    ui.heading("📦 Container");

    ui.checkbox(
        &mut state.container_locked,
        "🔒 Starts Locked",
    );

    ui.horizontal(|ui| {
        ui.label("Description (optional override):");
    });
    ui.add(
        egui::TextEdit::multiline(&mut state.container_description)
            .desired_rows(2)
            .id_salt("container_evt_desc"),
    );

    ui.separator();
    ui.label("📦 Initial Items:");

    // Item add row
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("container_evt_add_item")
            .selected_text(if state.container_item_input.is_empty() {
                "Select item…".to_string()
            } else {
                state.container_item_input.clone()
            })
            .show_ui(ui, |ui| {
                for item in &self.items {
                    ui.push_id(item.id, |ui| {
                        let label = format!("{} - {}", item.id, item.name);
                        if ui
                            .selectable_label(
                                state.container_item_input
                                    == item.id.to_string(),
                                &label,
                            )
                            .clicked()
                        {
                            state.container_item_input = item.id.to_string();
                        }
                    });
                }
            });

        if ui.button("➕ Add").clicked() {
            if let Ok(id) =
                state.container_item_input.trim().parse::<ItemId>()
            {
                state.container_items.push(id);
                state.container_item_input.clear();
            }
        }
    });

    // Item list
    egui::ScrollArea::vertical()
        .id_salt("container_evt_items_scroll")
        .max_height(180.0)
        .show(ui, |ui| {
            let mut remove_idx: Option<usize> = None;
            for (i, &item_id) in
                state.container_items.iter().enumerate()
            {
                ui.push_id(format!("container_item_{}", i), |ui| {
                    ui.horizontal(|ui| {
                        let item_name = self
                            .items
                            .iter()
                            .find(|it| it.id == item_id)
                            .map(|it| it.name.as_str())
                            .unwrap_or("(unknown)");
                        ui.label(format!(
                            "{}.  {} — {}",
                            i + 1,
                            item_id,
                            item_name
                        ));
                        if ui
                            .small_button("✕")
                            .on_hover_text("Remove item")
                            .clicked()
                        {
                            remove_idx = Some(i);
                        }
                    });
                });
            }
            if let Some(idx) = remove_idx {
                state.container_items.remove(idx);
            }
            if state.container_items.is_empty() {
                ui.label(
                    egui::RichText::new("  (empty — container starts with no items)")
                        .weak(),
                );
            }
        });
}
```

`self.items` is the `Vec<Item>` already held by `CampaignBuilderApp`.

---

### 7.5 Validation: `validate_npc_ids` Extension

**File:** `sdk/campaign_builder/src/lib.rs`

Extend the existing `validate_npc_ids` function (or add a sibling
`validate_stock_template_refs`) to cross-check every merchant NPC's
`stock_template` field against the loaded `self.stock_templates` list:

```
// In validate_npc_ids (or validate_stock_template_refs):
for npc in &self.npc_editor_state.npcs {
    if let Some(ref tmpl_id) = npc.stock_template {
        if !self.stock_templates.iter().any(|t| &t.id == tmpl_id) {
            errors.push(ValidationResult {
                severity: ValidationSeverity::Error,
                category: "NPCs".to_string(),
                asset_id: npc.id.clone(),
                message: format!(
                    "NPC '{}' references unknown stock template '{}'",
                    npc.id, tmpl_id
                ),
            });
        }
    }
}
```

Add a parallel check in `validate_campaign` that validates every template's
item IDs against `self.items`:

```
for tmpl in &self.stock_templates {
    for entry in &tmpl.entries {
        if !self.items.iter().any(|it| it.id == entry.item_id) {
            errors.push(ValidationResult {
                severity: ValidationSeverity::Warning,
                category: "StockTemplates".to_string(),
                asset_id: tmpl.id.clone(),
                message: format!(
                    "Template '{}' references unknown item_id {}",
                    tmpl.id, entry.item_id
                ),
            });
        }
    }
    for &pool_id in &tmpl.magic_item_pool {
        if !self.items.iter().any(|it| it.id == pool_id) {
            errors.push(ValidationResult {
                severity: ValidationSeverity::Warning,
                category: "StockTemplates".to_string(),
                asset_id: tmpl.id.clone(),
                message: format!(
                    "Template '{}' magic pool references unknown item_id {}",
                    tmpl.id, pool_id
                ),
            });
        }
    }
}
```

---

### 7.6 Testing Requirements for Phase 7

All tests live in the SDK crate test modules (`mod tests` inside each modified
file). No Bevy app is needed; tests operate on plain structs.

#### `sdk/campaign_builder/src/stock_templates_editor.rs` — unit tests

| Test name                                                  | Verifies                                                                                      |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `test_stock_templates_editor_state_default`                | Default state has empty `templates`, `List` mode, no selection                                |
| `test_stock_template_edit_buffer_default`                  | Default buffer has empty strings and empty vecs                                               |
| `test_from_template_round_trips`                           | `from_template` → `to_template` produces an identical `MerchantStockTemplate`                 |
| `test_to_template_validates_empty_id`                      | Empty `id` produces a non-empty `validation_errors` list                                      |
| `test_to_template_validates_invalid_item_id`               | Non-numeric `item_id` in an entry produces a validation error                                 |
| `test_to_template_validates_zero_quantity`                 | `quantity == "0"` produces a validation error                                                 |
| `test_to_template_validates_invalid_override_price`        | Non-numeric non-empty `override_price` produces a validation error                            |
| `test_to_template_validates_magic_slot_count_exceeds_pool` | `magic_slot_count` > pool length produces a warning string                                    |
| `test_to_template_validates_magic_refresh_days_zero`       | `magic_refresh_days == "0"` is treated as `1` with a warning                                  |
| `test_add_entry_appends_to_buffer`                         | Pushing a `TemplateEntryBuffer` to `edit_buffer.entries` is reflected in `to_template` output |
| `test_remove_entry_shrinks_list`                           | Removing index 1 from a 3-entry buffer leaves 2 entries                                       |
| `test_reorder_entry_up`                                    | Swapping index 1 with index 0 reverses the first two entries                                  |
| `test_load_from_file_round_trip`                           | `save_to_file` then `load_from_file` on a temp path produces equal `templates`                |
| `test_load_from_file_missing_path_returns_error`           | Loading a non-existent path returns `Err` without panic                                       |
| `test_open_template_for_edit_sets_edit_mode`               | `open_template_for_edit("foo")` sets `mode = Edit` and `edit_buffer.id == "foo"`              |
| `test_open_template_for_edit_unknown_id_noop`              | Calling with an unknown ID does not change `mode` or `selected_template`                      |

#### `sdk/campaign_builder/src/npc_editor.rs` — updated tests

| Test name                                           | Verifies                                                                                                                    |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `test_npc_edit_buffer_stock_template_default_empty` | Default `NpcEditBuffer` has `stock_template == ""`                                                                          |
| `test_save_npc_merchant_with_stock_template`        | `save_npc` in add mode with `is_merchant=true, stock_template="blacksmith"` sets `npc.stock_template == Some("blacksmith")` |
| `test_save_npc_merchant_no_template`                | `save_npc` with `is_merchant=true, stock_template=""` sets `npc.stock_template == None`                                     |
| `test_start_edit_npc_populates_stock_template`      | `start_edit_npc` on an NPC with `stock_template: Some("wizard_shop")` sets `edit_buffer.stock_template == "wizard_shop"`    |
| `test_requested_template_edit_set_on_click`         | Directly setting `requested_template_edit = Some("foo")` is `Some("foo")` (field is readable)                               |

#### `sdk/campaign_builder/src/map_editor.rs` — updated tests

| Test name                                                | Verifies                                                                                                          |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `test_event_type_container_name`                         | `EventType::Container.name() == "Container"`                                                                      |
| `test_event_type_container_icon`                         | `EventType::Container.icon() == "📦"`                                                                             |
| `test_event_type_all_includes_container`                 | `EventType::all()` slice contains `EventType::Container`                                                          |
| `test_event_editor_state_to_container_empty_items`       | `to_map_event` with `Container` type and empty `container_items` produces `MapEvent::Container { items: [], .. }` |
| `test_event_editor_state_to_container_with_items`        | `container_items: [1, 2, 3]` → `MapEvent::Container { items: [1, 2, 3], .. }`                                     |
| `test_event_editor_state_to_container_locked`            | `container_locked: true` → `MapEvent::Container { locked: true, .. }`                                             |
| `test_event_editor_state_from_container`                 | `from_map_event` on a `MapEvent::Container` round-trips all fields correctly                                      |
| `test_event_editor_state_container_description_override` | Non-empty `container_description` is used as the event description; empty falls back to `description`             |

#### `sdk/campaign_builder/src/lib.rs` — updated tests

| Test name                                                 | Verifies                                                                                                                           |
| --------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `test_validate_npc_ids_detects_unknown_stock_template`    | An NPC with `stock_template: Some("nonexistent")` and no matching template in `stock_templates` produces an `Error` result         |
| `test_validate_npc_ids_valid_stock_template_passes`       | An NPC with `stock_template: Some("basic_stock")` where `basic_stock` exists in `stock_templates` produces no error for that check |
| `test_validate_campaign_warns_unknown_item_in_template`   | A template with `entry.item_id = 255` (not in `self.items`) produces a `Warning`                                                   |
| `test_validate_campaign_warns_unknown_item_in_magic_pool` | A template with `magic_item_pool: [254]` (not in `self.items`) produces a `Warning`                                                |
| `test_editor_tab_stock_templates_name`                    | `EditorTab::StockTemplates.name() == "Stock Templates"`                                                                            |
| `test_campaign_metadata_default_stock_templates_file`     | `CampaignMetadata::default().stock_templates_file == "data/npc_stock_templates.ron"`                                               |

---

### 7.7 Candidate File Changes

| File                                                 | Nature of Change                                                                                                                                                                                                                                                                                                                                                                                       |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `sdk/campaign_builder/src/stock_templates_editor.rs` | **New file** — `StockTemplatesEditorState`, `StockTemplateEditBuffer`, `TemplateEntryBuffer`, full `show()` implementation, load/save helpers, unit tests                                                                                                                                                                                                                                              |
| `sdk/campaign_builder/src/lib.rs`                    | Add `pub mod stock_templates_editor`; add `StockTemplates` to `EditorTab`; add `stock_templates_editor_state`, `stock_templates`, `stock_templates_file` fields to `CampaignBuilderApp`; add `load_stock_templates`, `save_stock_templates_to_file`; render `StockTemplates` tab; extend `validate_npc_ids` and `validate_campaign`; add `stock_templates_file` to `CampaignMetadata`; add 6 new tests |
| `sdk/campaign_builder/src/npc_editor.rs`             | Add `stock_template` to `NpcEditBuffer`; add `available_stock_templates` and `requested_template_edit` to `NpcEditorState`; extend `show_edit_view` Merchant section; update `save_npc` and `start_edit_npc`; add 5 new tests                                                                                                                                                                          |
| `sdk/campaign_builder/src/map_editor.rs`             | Add `Container` to `EventType`; add `EVENT_COLOR_CONTAINER` constant; add container fields to `EventEditorState`; wire `to_map_event` / `from_map_event`; add 8 new tests                                                                                                                                                                                                                              |

---

### 7.8 Deliverables

- [ ] `sdk/campaign_builder/src/stock_templates_editor.rs` — new file; full
      CRUD UI for `MerchantStockTemplate`; load/save to RON; all 16 unit tests
      passing
- [ ] `sdk/campaign_builder/src/lib.rs` — `StockTemplates` tab wired; metadata
      field added; validation cross-checks added; `CampaignBuilderApp` fields
      and helpers added; 6 new tests passing
- [ ] `sdk/campaign_builder/src/npc_editor.rs` — `stock_template` field in
      buffer; merchant stock-template drop-down in edit view; `✏ Edit template`
      cross-navigation; 5 new tests passing
- [ ] `sdk/campaign_builder/src/map_editor.rs` — `Container` event type with
      item list editor; `EVENT_COLOR_CONTAINER`; 8 new tests passing
- [ ] All four quality gates pass (including `sdk/` targets):
      `cargo fmt --all`, `cargo check --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings`,
      `cargo nextest run --all-features`
- [ ] All egui ID rules from `sdk/AGENTS.md` satisfied: every loop
      `push_id`, every `ScrollArea` has `id_salt`, every `ComboBox` uses
      `from_id_salt`, no `SidePanel`/`CentralPanel` skipped by same-frame guard

---

### 7.9 Success Criteria

1. A content author can open the Campaign Builder, navigate to the
   **Stock Templates** tab, and create a new `MerchantStockTemplate` with
   regular entries and a magic-item rotation pool without editing any RON file
   by hand. The template is saved to `npc_stock_templates.ron` on **💾 Save**.

2. In the **NPCs** tab, selecting `is_merchant` reveals a **Stock Template**
   drop-down listing all templates from the current campaign. Selecting a
   template and saving writes `stock_template: Some("template_id")` to
   `npcs.ron`. Clicking **✏ Edit template** navigates directly to that template
   in the **Stock Templates** tab.

3. In the **Maps** tab, placing a **Container** (📦) event opens an inspector
   panel where the author can pick items from the campaign item database to
   populate the container's initial contents and toggle the locked flag. Saving
   the map writes a correct `MapEvent::Container { … }` entry to the map's
   RON file.

4. The **Validation** panel reports an `Error` for any merchant NPC whose
   `stock_template` references a template ID that does not exist in
   `npc_stock_templates.ron`, and a `Warning` for any template that references
   an item ID not present in `items.ron`.

5. All new egui widgets use unique `id_salt` / `push_id` scopes; the Campaign
   Builder produces no duplicate-ID misbehaviour (scroll areas do not share
   position, combo boxes show correct selections).

6. All existing tests (Phases 1–6 and all pre-existing SDK tests) continue to
   pass — Phase 7 introduces no breaking changes to existing domain types or
   existing SDK editor state structures.
