# Inventory UI Text List Refactor Implementation Plan

## Overview

Replace the pixel-painted 8×8 icon-grid character inventory panels across three
screens with scrollable text-row lists matching the existing style used by the
merchant stock and container-items panels. Add a Single/Multi view toggle so the
player can collapse to a full-width single-character panel (with the complete
equipment strip clearly readable) or expand back to the all-characters grid view.
Simultaneously simplify the 2D grid keyboard navigation in all three screens to
linear up/down movement. The change is purely cosmetic and layout-level: every
action struct, event writer, game-logic system, equipment strip, and
action-button strip remains untouched throughout all phases.

---

## Current State Analysis

### Existing Infrastructure

| File                                                                                                | Role                                     | Lines |
| --------------------------------------------------------------------------------------------------- | ---------------------------------------- | ----- |
| [`src/application/inventory_state.rs`](../../../src/application/inventory_state.rs)                 | `InventoryState` + `tab_next`/`tab_prev` | 535   |
| [`src/game/systems/inventory_ui.rs`](../../../src/game/systems/inventory_ui.rs)                     | Main character inventory screen          | 6 659 |
| [`src/game/systems/merchant_inventory_ui.rs`](../../../src/game/systems/merchant_inventory_ui.rs)   | Merchant buy/sell screen                 | 2 419 |
| [`src/game/systems/container_inventory_ui.rs`](../../../src/game/systems/container_inventory_ui.rs) | Container take/stash screen              | 2 870 |
| [`src/game/systems/inventory_ui_common.rs`](../../../src/game/systems/inventory_ui_common.rs)       | Shared layout constants and nav types    | 184   |

The merchant **stock** panel (`render_merchant_stock_panel`, L946–L1209) and the
container **items** panel (`render_container_items_panel`, L1171–L1516) already
use the desired text-row pattern — scrollable `ScrollArea::vertical` with one row
per item, selection highlighted with a yellow border. This is the target style.

The **equipment strip** inside `render_character_panel` (L1416–L1547) already
renders equipped item names as text in painted cells and is **not changing** in
this refactor. It is explicitly preserved in all phases. In Single view it
benefits automatically by receiving the full panel width.

The **unequip keyboard flow** in `handle_equip_flow` (L613–L714) is also
preserved unchanged: `ArrowUp` from slot 0 enters the equipment strip,
`ArrowLeft`/`ArrowRight` cycles slots, `Enter` unequips, `ArrowDown` returns to
the item list.

### Identified Issues

1. **8×8 icon grid in `render_character_panel`** (L1549–L1647) — geometric
   silhouettes with no item names visible until a slot is selected. With 4–6
   party panels on screen each panel is ≈1/3 width × 1/2 height, making icons
   unreadable and navigation confusing.

2. **No single-character focus mode** — the inventory always distributes space
   across all open panels. There is no way to collapse to one full-width panel to
   clearly read the equipment strip and item list for a single character.

3. **Number keys 1-6 are unbound** in `inventory_input_system`. Tab cycles focus
   but progressively accumulates panels; there is no direct character-select
   shortcut.

4. **Identical icon-grid body duplicated twice** — `render_character_sell_panel`
   in `merchant_inventory_ui.rs` (L771–L829) and `render_character_stash_panel`
   in `container_inventory_ui.rs` (L1007–L1069) are near-identical copies.

5. **2D grid keyboard navigation in all three files** — `handle_grid_navigation`
   (L718–L831), the `MerchantFocus::Left` branch (L400–L428), and the
   `ContainerFocus::Left` branch (L529–L557) all use column/row arithmetic
   (`±1` left/right, `±SLOT_COLS` up/down) that only makes sense for a grid.

6. **`SLOT_COLS` shared constant** in `inventory_ui_common.rs` (L29–L30) is used
   exclusively for 2D grid rendering and navigation arithmetic, becoming dead
   code after the refactor.

7. **`paint_item_silhouette` / `paint_item_silhouette_pub`** (L1842–L2026 in
   `inventory_ui.rs`) — ~185 lines of geometric drawing code that becomes
   entirely unreferenced after the three grids are replaced.

---

## Implementation Phases

---

### Phase 1: Core Foundation — View Mode State and Navigation Model

Add the `InventoryViewMode` enum and `Single`/`Multi` view toggle to
`InventoryState`, bind number keys 1-6 in the inventory input system, and update
`inventory_ui_system` to render either one or all panels based on the active
mode. This phase touches only the state and input layers; no rendering changes
are made to the panel body yet.

#### 1.1 Add `InventoryViewMode` to `inventory_state.rs`

In [`src/application/inventory_state.rs`](../../../src/application/inventory_state.rs),
add a `pub enum InventoryViewMode { Multi, Single }` with `Default = Multi` and
the standard `Debug`, `Clone`, `PartialEq`, `Eq` derives.

Add a `pub view_mode: InventoryViewMode` field to `InventoryState` (L41–L75),
defaulting to `Multi`.

#### 1.2 Add View-Transition Methods to `InventoryState`

Add two new pub methods with full `///` doc comments and doctests:

- **`enter_single_view(character_index: usize)`** — sets `view_mode = Single`,
  sets `focused_index = character_index`, clears `selected_slot = None`.
  Does **not** modify `open_panels` (the multi-view panel set is preserved for
  when the player returns to Multi mode).

- **`enter_multi_view(party_size: usize)`** — sets `view_mode = Multi`,
  rebuilds `open_panels` as `(0..party_size).collect()` so all party members are
  visible, keeps `focused_index` unchanged.

#### 1.3 Update `tab_next` / `tab_prev` Semantics

Modify `tab_next` and `tab_prev` (L149–L198) so that when called while
`view_mode == Single`, they first call `enter_multi_view(party_size)` before
advancing `focused_index`. This means pressing **Tab from Single view expands
back to Multi view** and then cycles focus — one gesture, clear semantic.

When called while `view_mode == Multi`, behaviour is unchanged: cycle focus and
add the new panel to `open_panels` if not already present.

#### 1.4 Bind Number Keys 1-6 in `inventory_input_system`

In `inventory_input_system` (L859–L991), add a number-key check block
(before the Tab handling) that maps `Digit1`–`Digit6` to `enter_single_view(N)`:
clear `nav_state.selected_slot_index`, `nav_state.focused_action_index`,
`nav_state.selected_equip_slot`, and reset `nav_state.phase` to
`NavigationPhase::SlotNavigation`. Guard with a bounds check against
`party_size`.

#### 1.5 Update `inventory_ui_system` for View Mode

In `inventory_ui_system` (L1212–L1319), after cloning `inv_state`, derive the
effective panels list for `render_item_grid`:

- `InventoryViewMode::Multi` → pass `inv_state.open_panels` as now.
- `InventoryViewMode::Single` → pass `&[inv_state.focused_index]` so
  `render_item_grid` allocates the full available space to one panel.

No other changes to `render_item_grid` are needed — when given a single-element
slice the existing `cols = 1`, `rows = 1` path already produces a full-width,
full-height panel.

#### 1.6 Update Hint Text

In `render_equipment_panel` (L997–L1046), update the `SlotNavigation` hint
strings to reflect both modes:

- **Multi view hint**: `"1-6: single view   Tab: cycle   ↑↓: navigate   Enter: select   E: equip   U: use   Esc/I: close"`
- **Single view hint**: `"1-6: switch char   Tab: all chars   ↑↓: navigate   Enter: select   E: equip   U: use   Esc/I: close"`

Pass the `view_mode` into `render_equipment_panel` so it can choose the correct
hint string.

#### 1.7 Testing Requirements

Add tests to `inventory_state.rs`:

- `test_enter_single_view_sets_mode_and_clears_slot`
- `test_enter_multi_view_restores_all_panels`
- `test_tab_next_from_single_view_enters_multi_and_advances`
- `test_tab_prev_from_single_view_enters_multi_and_retreats`
- `test_number_key_in_inventory_enters_single_view` (Bevy app test in `inventory_ui.rs`)

Run all four quality gates.

#### 1.8 Deliverables

- [x] `InventoryViewMode` enum added to `inventory_state.rs`
- [x] `view_mode` field added to `InventoryState`
- [x] `enter_single_view` and `enter_multi_view` methods added with doc comments and doctests
- [x] `tab_next` / `tab_prev` expand to Multi when called from Single
- [x] Number keys 1-6 bound in `inventory_input_system`; trigger `enter_single_view`
- [x] `inventory_ui_system` passes correct panel slice based on `view_mode`
- [x] Hint text updated for both view modes
- [x] All new state-transition tests pass
- [x] All four quality gates pass with zero errors and zero warnings

#### 1.9 Success Criteria

Pressing `1` from the multi-panel inventory collapses to that character's full-width
panel. Pressing `Tab` from the single view expands back to all party members.
`cargo nextest run --all-features` passes with all tests green.

---

### Phase 2: Main Character Inventory — Text List + Linear Navigation

Replace the 8×8 icon grid with a scrollable text list in `render_character_panel`
and simplify `handle_grid_navigation` to linear movement. The equipment strip,
action strip, and all keyboard unequip/equip flows are preserved exactly.

#### 2.1 Replace Grid Body with Scrollable Text Rows

In [`inventory_ui.rs`](../../../src/game/systems/inventory_ui.rs), replace the
grid-rendering body inside `render_character_panel` (L1549–L1647) with a
`ScrollArea::vertical` (id salt: `"char_inv_scroll_{party_index}"`) containing
one row per occupied inventory slot. Each row renders:

- Slot index (right-aligned, dim)
- Item name (primary text, white when occupied)
- Item type category tag, e.g. `[Weapon]`, `[Potion]`, `[Ammo]` (dim, right column)
- Charge annotation `✨N` for wand/staff items with remaining charges

Empty rows beyond `items.len()` are not rendered — the list length equals the
actual number of items carried, not `MAX_ITEMS`. An "(empty)" placeholder is
shown when `items` is empty.

Row selection highlight (yellow border + amber fill) follows the pattern in
`render_merchant_stock_panel` (L1064–L1083). The `body_h` calculation
(L1387–L1388) is unchanged because the equipment strip, header, and action
reserve heights are all preserved.

#### 2.2 Simplify `handle_grid_navigation` to Linear

In `handle_grid_navigation` (L718–L831), replace all 2D column/row arithmetic
with linear `±1` movement wrapping at `character.inventory.items.len()`. Items
beyond the actual inventory count are never selectable.

The `ArrowUp` from slot 0 (or from no selection) → enter equipment strip
special case (L795–L808) is **preserved exactly**: setting
`nav_state.selected_equip_slot = Some(EquipmentSlot::Weapon)` and clearing
`nav_state.selected_slot_index`.

`ArrowLeft` and `ArrowRight` are **removed from slot navigation** — they have
no meaning in a linear list and are already consumed by `handle_equip_flow`
when the equipment strip is focused.

#### 2.3 Remove Local Silhouette References

Delete the `ITEM_SILHOUETTE_COLOR` constant (L67–L68) and the two calls to the
private `paint_item_silhouette` inside `render_character_panel` (L1613 and
L1634). Do **not** delete `paint_item_silhouette_pub` yet — it is still called
by the Merchant (Phase 3) and Container (Phase 4) character sub-panels.

#### 2.4 Testing Requirements

Update `test_render_character_panel_does_not_panic_empty_inventory` (L3078–L3118)
and `test_render_character_panel_does_not_panic_full_inventory` (L3127–L3176) to
match the new text-list panel structure. All other tests are unaffected.

Run all four quality gates.

#### 2.5 Deliverables

- [ ] `render_character_panel` body replaced with `ScrollArea` text rows
- [ ] `handle_grid_navigation` uses linear `±1` navigation; `ArrowUp` from top → equipment strip preserved
- [ ] `ArrowLeft`/`ArrowRight` removed from slot navigation; `handle_equip_flow` retains them for equipment cycling
- [ ] `ITEM_SILHOUETTE_COLOR` constant deleted
- [ ] Calls to `paint_item_silhouette` removed from `render_character_panel`
- [ ] `test_render_character_panel_does_not_panic_empty_inventory` updated
- [ ] `test_render_character_panel_does_not_panic_full_inventory` updated
- [ ] All four quality gates pass with zero errors and zero warnings

#### 2.6 Success Criteria

`cargo nextest run --all-features` passes. In both Single and Multi view, the
character inventory body shows a scrollable text list of item names. The equipment
strip above it continues to display all 7 slots with names and the unequip
keyboard/mouse flow is unchanged.

---

### Phase 3: Merchant Character Panel — Text List + Linear Navigation

Replace the icon grid in `render_character_sell_panel` with a text list and
simplify the `MerchantFocus::Left` navigation branch to linear movement.

#### 3.1 Replace Grid Body with Text Rows

In [`merchant_inventory_ui.rs`](../../../src/game/systems/merchant_inventory_ui.rs),
replace the grid body inside `render_character_sell_panel` (L771–L829) with a
`ScrollArea::vertical` (id salt: `"merch_char_inv_scroll"`) text-row list. Row
content: slot index, item name, and type tag. Do **not** add sell-price
annotations here — sell price is only relevant when interacting with the merchant
and is already shown in the action strip (L831–L900) when a slot is selected.

#### 3.2 Remove `paint_item_silhouette_pub` Call

Remove the call to `paint_item_silhouette_pub` (L815) from
`render_character_sell_panel`.

#### 3.3 Simplify `MerchantFocus::Left` Navigation

In `merchant_inventory_input_system` (L400–L428), replace the `MerchantFocus::Left`
2D navigation branch with linear `±1` movement wrapping at `items.len()`,
matching the already-linear `MerchantFocus::Right` branch (L430–L454) in the
same function. `ArrowLeft`/`ArrowRight` move `±1` row for discoverability,
identical to the merchant stock panel behaviour.

#### 3.4 Testing Requirements

Confirm `test_buy_item_action_via_click_matches_keyboard_action` (L2126–L2190),
`test_sell_item_action_via_click_matches_keyboard_action` (L2196–L2239), and
`test_merchant_keyboard_navigation_phase_transitions` (L2345–L2418) pass.

Run all four quality gates.

#### 3.5 Deliverables

- [ ] `render_character_sell_panel` body replaced with `ScrollArea` text rows (no sell-price column)
- [ ] `paint_item_silhouette_pub` call removed from `merchant_inventory_ui.rs`
- [ ] `MerchantFocus::Left` navigation branch converted to linear `±1`
- [ ] All four quality gates pass with zero errors and zero warnings

#### 3.6 Success Criteria

`cargo nextest run --all-features` passes. The merchant character sell panel
shows a text list visually consistent with the stock panel beside it.

---

### Phase 4: Container Character Panel — Text List + Linear Navigation

Replace the icon grid in `render_character_stash_panel` with a text list and
simplify the `ContainerFocus::Left` navigation branch to linear movement.

#### 4.1 Replace Grid Body with Text Rows

In [`container_inventory_ui.rs`](../../../src/game/systems/container_inventory_ui.rs),
replace the grid body inside `render_character_stash_panel` (L1007–L1069) with a
`ScrollArea::vertical` (id salt: `"cont_char_inv_scroll"`) text-row list. Row
content: slot index, item name, and type tag. After this change
`paint_item_silhouette_pub` has **zero callers** in the entire codebase.

#### 4.2 Remove `paint_item_silhouette_pub` Call

Remove the call to `paint_item_silhouette_pub` (L1052) from
`render_character_stash_panel`.

#### 4.3 Simplify `ContainerFocus::Left` Navigation

In `container_inventory_input_system` (L529–L557), replace the
`ContainerFocus::Left` 2D navigation branch with linear `±1` movement wrapping
at `items.len()`, matching the already-linear `ContainerFocus::Right` branch
(L558–L582) in the same function.

#### 4.4 Testing Requirements

Confirm `test_take_item_action_via_click_removes_from_container` (L2441–L2494),
`test_stash_item_action_via_click_adds_to_container` (L2688–L2739), and
`test_container_keyboard_navigation_phase_transitions` (L2750–L2869) pass.

Run all four quality gates.

#### 4.5 Deliverables

- [ ] `render_character_stash_panel` body replaced with `ScrollArea` text rows
- [ ] `paint_item_silhouette_pub` call removed from `container_inventory_ui.rs`
- [ ] `ContainerFocus::Left` navigation branch converted to linear `±1`
- [ ] All four quality gates pass with zero errors and zero warnings

#### 4.6 Success Criteria

`cargo nextest run --all-features` passes. The container character stash panel
shows a text list visually consistent with the container items panel beside it.

---

### Phase 5: Dead Code Cleanup and Documentation

Remove all symbols that became unreferenced across Phases 2–4 and update
project documentation.

#### 5.1 Delete the Silhouette Engine

In [`inventory_ui.rs`](../../../src/game/systems/inventory_ui.rs), delete:

- Private `paint_item_silhouette` function (L1866–L2026, ~160 lines)
- Public re-export `paint_item_silhouette_pub` (L1842–L1850)

Both are unreferenced after Phases 2–4. `cargo clippy -D warnings` will flag
them as dead code if any were accidentally missed.

#### 5.2 Remove the Shared Grid Constant

In [`inventory_ui_common.rs`](../../../src/game/systems/inventory_ui_common.rs),
delete the `SLOT_COLS` constant (L29–L30). Verify zero remaining references with
a project-wide search before deleting.

#### 5.3 Documentation Update

Append a summary of the complete refactor to
[`docs/explanation/implementations.md`](implementations.md), covering: what was
replaced, what was preserved (equipment strip, all action/logic systems), the
new Single/Multi view model, line-count deltas per file, and the key-binding
changes.

#### 5.4 Testing Requirements

Run the full `cargo nextest run --all-features` suite. Confirm `cargo check
--all-targets --all-features` emits zero warnings (dead-code lints would surface
any missed deletions).

#### 5.5 Deliverables

- [ ] `paint_item_silhouette` deleted
- [ ] `paint_item_silhouette_pub` deleted
- [ ] `SLOT_COLS` removed from `inventory_ui_common.rs`
- [ ] Zero references to `SLOT_COLS` or `paint_item_silhouette` remain in codebase
- [ ] `docs/explanation/implementations.md` updated
- [ ] All four quality gates pass with zero errors and zero warnings

#### 5.6 Success Criteria

`cargo check --all-targets --all-features` emits zero warnings.
`cargo nextest run --all-features` passes. Project-wide search for
`paint_item_silhouette` and `SLOT_COLS` returns no matches.

---

## Key Design Decisions

### View Toggle Semantics

| Key         | Multi view                                         | Single view                         |
| ----------- | -------------------------------------------------- | ----------------------------------- |
| `1`–`6`     | Collapse to Single, focus character N              | Switch to character N (stay Single) |
| `Tab`       | Cycle focus, add panel to `open_panels` (existing) | Expand to Multi, then advance focus |
| `Shift+Tab` | Cycle focus backward (existing)                    | Expand to Multi, then retreat focus |
| `Esc` / `I` | Close inventory (existing)                         | Close inventory (existing)          |

### Equipment Strip — Preserved Unchanged

The equipment strip (L1416–L1547) already renders item names as text and
supports full keyboard unequip navigation (`ArrowUp` → strip, `←→` cycle slots,
`Enter` unequip, `ArrowDown` return to list). It is **not modified** in any
phase. In Single view it naturally receives the full panel width, improving
readability without any code changes.

### What Does Not Change

All action structs (`DropItemAction`, `TransferItemAction`, `UseItemExplorationAction`,
`EquipItemAction`, `UnequipItemAction`), all action-processing systems
(`inventory_action_system`, `handle_use_item_action_exploration`, equip/unequip
logic), the per-panel action button strip, the `NavigationPhase` enum, and all
`handle_equip_flow` keyboard logic are **untouched throughout all phases**.

---

## Summary: Scope by File

| File                                         | Phase(s) | Lines Removed | Lines Added | Net      |
| -------------------------------------------- | -------- | ------------- | ----------- | -------- |
| `src/application/inventory_state.rs`         | 1        | ~5            | ~80         | +75      |
| `src/game/systems/inventory_ui.rs`           | 1, 2, 5  | ~445          | ~110        | −335     |
| `src/game/systems/merchant_inventory_ui.rs`  | 3        | ~60           | ~50         | −10      |
| `src/game/systems/container_inventory_ui.rs` | 4        | ~65           | ~50         | −15      |
| `src/game/systems/inventory_ui_common.rs`    | 5        | ~3            | 0           | −3       |
| **Total**                                    |          | **~578**      | **~290**    | **−288** |
