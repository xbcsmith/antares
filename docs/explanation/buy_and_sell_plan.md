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

| Component | Location | Status |
|-----------|----------|--------|
| `buy_item()` transaction | `src/domain/transactions.rs` | ✅ Implemented and tested |
| `sell_item()` transaction | `src/domain/transactions.rs` | ✅ Implemented and tested |
| `MerchantStock`, `StockEntry`, `NpcEconomySettings` | `src/domain/inventory.rs` | ✅ Implemented and tested |
| `MerchantInventoryState`, `MerchantFocus` | `src/application/merchant_inventory_state.rs` | ✅ Implemented and tested |
| `ContainerInventoryState`, `ContainerFocus` | `src/application/container_inventory_state.rs` | ✅ Implemented and tested |
| `GameMode::MerchantInventory(_)` | `src/application/mod.rs` | ✅ Variant exists |
| `GameMode::ContainerInventory(_)` | `src/application/mod.rs` | ✅ Variant exists |
| `GameState::enter_merchant_inventory()` | `src/application/mod.rs` | ✅ Implemented and tested |
| `GameState::enter_container_inventory()` | `src/application/mod.rs` | ✅ Implemented and tested |
| `MerchantInventoryPlugin` (UI + input + action systems) | `src/game/systems/merchant_inventory_ui.rs` | ✅ Implemented |
| `ContainerInventoryPlugin` (UI + input + action systems) | `src/game/systems/container_inventory_ui.rs` | ✅ Implemented |
| `NpcDefinition::is_merchant` flag | `src/domain/world/npc.rs` | ✅ Implemented |
| `NpcRuntimeStore`, `NpcRuntimeState` | `src/domain/world/npc_runtime.rs` | ✅ Implemented |
| `ensure_npc_runtime_initialized()` | `src/application/mod.rs` | ✅ Implemented |
| `DialogueAction::OpenMerchant` | `src/domain/dialogue.rs` | ✅ Variant exists (stub) |
| `EventResult::EnterMerchant` | `src/domain/world/events.rs` | ✅ Variant exists |
| `handle_event_result` for `EnterMerchant` | `src/game/systems/events.rs` | ✅ Routes to dialogue |
| Tutorial merchant NPCs and stock templates | `campaigns/tutorial/data/` | ✅ Data present |
| Test campaign merchant NPCs and stock templates | `data/test_campaign/data/` | ✅ Data present |

### What Is Missing (Gaps This Plan Closes)

| Gap | Phase |
|-----|-------|
| `DialogueAction::OpenMerchant` handler calls `enter_merchant_inventory()` instead of logging a stub | Phase 1 |
| Pressing `I` while in `GameMode::Dialogue` with a merchant NPC opens `GameMode::MerchantInventory` | Phase 1 |
| `I` key in non-merchant dialogue is ignored (no mode change) | Phase 1 |
| Player feedback (game log message) when buy/sell fails (insufficient gold, inventory full, out of stock, cursed item) | Phase 2 |
| Price display in merchant stock panel (shows item cost before confirming buy) | Phase 2 |
| Party gold display in merchant UI header | Phase 2 |
| Container interaction: `E` on a container map event enters `GameMode::ContainerInventory` | Phase 3 |
| Container items persist after partial take (map event state written back on close) | Phase 3 |
| Container empty state: panel shows "Empty" text when container has no items | Phase 3 |
| Mouse click support for Buy, Sell, Take, Take All, Stash action buttons | Phase 4 |
| Tutorial merchant dialogue node wires `OpenMerchant` action to open shop | Phase 5 |
| `data/test_campaign` merchant dialogue mirrors tutorial wiring | Phase 5 |
| `docs/explanation/implementations.md` updated | Phase 5 |

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

| Failure case | Log message |
|---|---|
| `InsufficientGold { have, need }` | `"Not enough gold. Need {need} gp, have {have} gp."` |
| `InventoryFull { character_id }` | `"Inventory is full. Drop an item to make room."` |
| `OutOfStock { item_id }` | `"The merchant is out of stock for that item."` |
| `ItemNotInInventory` | `"You do not have that item."` |
| Cursed item sell attempt | `"That item is cursed and cannot be sold."` |

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

None. All required modules already exist.

### Modified Files

| File | Phase | Nature of Change |
|------|-------|------------------|
| `src/game/systems/dialogue.rs` | 1 | Wire `OpenMerchant` to `enter_merchant_inventory` |
| `src/application/dialogue.rs` | 1 | Add `npc_id: Option<NpcId>` to `DialogueState` (if absent) |
| `src/game/systems/input.rs` | 1 | `I` key in `Dialogue` mode opens merchant inventory for merchant NPCs |
| `src/game/systems/merchant_inventory_ui.rs` | 2, 4 | Gold display, price display, error feedback, mouse support |
| `src/domain/character/equipment.rs` | 2 | `is_item_equipped(item_id) -> bool` (if absent) |
| `src/game/systems/container_inventory_ui.rs` | 3, 4 | Write-back on close, empty state, mouse support |
| `src/domain/world/events.rs` | 3 | `MapEvent::Container` variant (if absent) |
| `src/game/systems/events.rs` | 3 | Wire `E` on container to `enter_container_inventory` |
| `data/test_campaign/data/maps/map_1.ron` | 3 | Add test container event |
| `data/test_campaign/data/dialogues.ron` | 5 | Wire `OpenMerchant` action |
| `campaigns/tutorial/data/dialogues.ron` | 5 | Wire `OpenMerchant` action for both tutorial merchants |
| `src/application/save_game.rs` | 5 | Stock and container persistence tests |
| `docs/explanation/implementations.md` | 5 | Implementation summary section |

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
