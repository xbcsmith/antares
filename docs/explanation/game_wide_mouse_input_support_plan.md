<!-- SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Game-Wide Mouse Input Support Implementation Plan

## Overview

Mouse input currently works reliably only in the **Combat** screen and parts of
the **Inn Management** egui UI. Every other game mode — `Exploration`,
`Dialogue`/choice UI, `Menu`, `Inventory`, `MerchantInventory`,
`ContainerInventory` — has gaps ranging from missing click handlers on
individual widgets to complete absence of any mouse path. This plan delivers a
**phased, engine-wide** mouse input pass that:

1. Introduces a **shared `MouseActivation` utility**
   (`src/game/systems/mouse_input.rs`) that codifies the two-path activation
   model already proven in combat (`Interaction::Pressed` change-detection
   **plus** `just_pressed(Left) && Hovered` fallback).
2. Wires that utility into the two Bevy-UI-button systems (combat is already
   covered; menu needs the fallback).
3. Fixes the egui-based screens by completing the `response.clicked()` paths
   that are already partially scaffolded but not yet connected to actions.
4. Adds regression tests for each major mode so breakage is caught
   automatically.

All keyboard paths are preserved unchanged throughout. No architecture data
structures change and no new `GameMode` variants are introduced.

---

## Current State Analysis

### Existing Infrastructure

| System file                                                                    | Mode                      | Mouse today                                                                                                                                                                                                                                             |
| ------------------------------------------------------------------------------ | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/game/systems/combat.rs` — `combat_input_system`                           | `Combat`                  | ✅ Full dual-path: `Interaction::Pressed` + hovered-click fallback on `ActionButton`                                                                                                                                                                    |
| `src/game/systems/combat.rs` — `select_target`                                 | `Combat` target selection | ✅ Same dual-path on `EnemyCard`                                                                                                                                                                                                                        |
| `src/game/systems/inn_ui.rs` — `inn_ui_system`                                 | `InnManagement`           | ✅ egui `response.clicked()` on party/roster cards, Dismiss, Recruit, Swap, Exit                                                                                                                                                                        |
| `src/game/systems/recruitment_dialog.rs` — `show_recruitment_dialog`           | `Exploration` overlay     | ✅ egui `ui.button().clicked()` on Accept/Decline                                                                                                                                                                                                       |
| `src/game/systems/menu.rs` — `menu_button_interaction`                         | `Menu`                    | ⚠️ `Interaction::Pressed` fires but has no hovered-click fallback; sliders have no mouse path                                                                                                                                                           |
| `src/game/systems/dialogue_choices.rs` — `choice_input_system`                 | `Dialogue`                | ❌ Keyboard only; choice nodes have no `Button`/`Interaction` component                                                                                                                                                                                 |
| `src/game/systems/dialogue.rs` — `dialogue_input_system`                       | `Dialogue`                | ❌ Keyboard only; advance/dismiss not clickable                                                                                                                                                                                                         |
| `src/game/systems/inventory_ui.rs` — `inventory_ui_system`                     | `Inventory`               | ⚠️ egui UI; action buttons (Drop/Give) use `.clicked()` correctly. Slot grid uses `Sense::hover()` — cells do not capture clicks, requiring keyboard to select a slot before action buttons appear                                                      |
| `src/game/systems/merchant_inventory_ui.rs` — `merchant_inventory_ui_system`   | `MerchantInventory`       | ⚠️ egui UI; Buy/Sell action buttons use `.clicked()` correctly. Merchant stock rows use `Sense::click()` but `response.clicked()` is captured and then discarded — selection does not update state. Character slot grid also uses `Sense::hover()` only |
| `src/game/systems/container_inventory_ui.rs` — `container_inventory_ui_system` | `ContainerInventory`      | ⚠️ egui UI; Take/TakeAll/Stash action buttons use `.clicked()` correctly. Container item rows use `Sense::click()` but `_response` is ignored entirely — clicks do nothing. Character slot grid uses `Sense::hover()` only                              |

### Identified Issues

1. **No shared activation abstraction for Bevy-UI buttons.** Combat discovered
   and fixed the `Interaction::Pressed` reliability gap by adding a
   `just_pressed && Hovered` fallback. The menu system uses Bevy-UI buttons but
   lacks this fallback. Extracting the pattern to a shared helper prevents it
   being re-invented (or forgotten) on future Bevy-UI screens.

2. **Dialogue choice buttons are invisible to mouse.** `choice_input_system` is
   purely keyboard-driven. The spawned choice nodes carry no `Button` component
   and produce no `Interaction` events. The advance/dismiss path in
   `dialogue_input_system` is also keyboard-only.

3. **Menu sliders have no mouse path.** Settings volume sliders
   (`SettingSlider`) are adjusted only with Left/Right arrow keys in
   `handle_menu_keyboard`. There is no `response.drag_delta()` or
   `response.clicked()` path for mouse users.

4. **Inventory slot grids do not capture clicks.** In all three inventory
   screens the character slot grids are drawn with `Sense::hover()`, meaning
   egui does not deliver click events for them. A player cannot click a slot to
   select it — they must use arrow keys first before the action buttons appear.
   Changing to `Sense::click()` and wiring `response.clicked()` to selection
   state fixes this with minimal code change inside the existing egui render
   functions.

5. **Merchant stock row selection is scaffolded but disconnected.** The stock
   rows in `render_merchant_stock_panel` already use `Sense::click()` and check
   `response.clicked()` but the result is silently discarded. The selection
   state update (`ms.merchant_selected_slot = Some(i)`) is missing.

6. **Container item row clicks are fully ignored.** Container item rows use
   `Sense::click()` but store the response in `_response` (prefixed underscore)
   and never act on it. The selection state update
   (`cs.container_selected_slot = Some(i)`) is missing.

7. **No regression test coverage** for mouse activation outside combat. A
   future refactor can silently break mouse support with no failing tests.

8. **Exploration has no mouse-to-interact.** Clicking an NPC or interactive
   object tile does nothing. Lower priority than the menu/dialogue/inventory
   gaps, addressed in Phase 5.

---

## Implementation Phases

---

### Phase 1: Shared Mouse Activation Utility

**Goal:** Establish the canonical activation model for Bevy-UI buttons once, so
all subsequent phases that use Bevy-UI (combat, menu, dialogue choice UI) call a
single helper rather than duplicating the dual-path pattern.

#### 1.1 Create `src/game/systems/mouse_input.rs`

Add a new module alongside the other system files. It exposes:

- **`pub fn is_activated(interaction: &Interaction, interaction_ref:
Ref<Interaction>, mouse_just_pressed: bool) -> bool`** — returns `true` when
  either `Interaction::Pressed` changed this frame **or**
  `mouse_just_pressed && *interaction == Interaction::Hovered`. This is the
  exact logic already working in `combat_input_system` and `select_target`; it
  becomes the single source of truth for Bevy-UI button activation.

- **`pub fn mouse_just_pressed(mouse_buttons: Option<&ButtonInput<MouseButton>>) -> bool`** —
  a zero-boilerplate wrapper around
  `Option::is_some_and(|m| m.just_pressed(MouseButton::Left))` so callers do
  not repeat this pattern.

Both functions must be `#[inline]`, carry full `///` doc comments, and include
runnable doctests. Add the SPDX header as the first two lines of the file.

Register the module in `src/game/systems/mod.rs`.

#### 1.2 Refactor Combat to Use the New Helpers

Replace the three occurrences of the inline dual-path pattern in
`src/game/systems/combat.rs` (`combat_input_system` × 1,
`select_target` × 1, and the blocked-input guard × 1) with calls to
`mouse_input::is_activated` and `mouse_input::mouse_just_pressed`. Behaviour
must be byte-for-byte identical — this is a mechanical refactor. All existing
combat mouse tests must pass unchanged.

#### 1.3 Testing Requirements

Unit tests in `mouse_input.rs`:

- `test_is_activated_pressed_changed` — `Interaction::Pressed` changed →
  `true`.
- `test_is_activated_pressed_unchanged` — `Interaction::Pressed` not changed →
  `false`.
- `test_is_activated_hovered_with_mouse_press` — `Interaction::Hovered` +
  `mouse_just_pressed=true` → `true`.
- `test_is_activated_hovered_without_mouse_press` — `Interaction::Hovered` +
  `mouse_just_pressed=false` → `false`.
- `test_is_activated_none` — `Interaction::None` + any `mouse_just_pressed` →
  `false`.
- `test_mouse_just_pressed_none_resource` — `Option::None` → `false`.

#### 1.4 Deliverables

- [ ] `src/game/systems/mouse_input.rs` created with SPDX header,
      `is_activated`, `mouse_just_pressed`, and all unit tests listed in §1.3.
- [ ] `src/game/systems/mod.rs` declares the new `mouse_input` module.
- [ ] `combat_input_system` and `select_target` refactored to use helpers; all
      existing combat mouse tests still pass.
- [ ] `cargo fmt`, `cargo check`, `cargo clippy -D warnings`,
      `cargo nextest run` all green.

#### 1.5 Success Criteria

- `mouse_input` module compiles with zero warnings.
- All existing combat mouse tests pass unchanged.
- `grep -n "just_pressed(MouseButton" src/game/systems/combat.rs` returns zero
  results (all occurrences replaced by `mouse_input::mouse_just_pressed`).

---

### Phase 2: Menu Mouse Support (Buttons and Sliders)

**Goal:** Fix the menu system so every interactive element — navigation buttons
and settings sliders — is fully operable by mouse.

#### 2.1 Menu Button Hovered-Click Fallback

Update `menu_button_interaction` in `src/game/systems/menu.rs` to use the
Phase 1 helpers. The query signature changes to:

```text
Query<(&Interaction, Ref<Interaction>, &MenuButton), With<Button>>
```

Add `Option<Res<ButtonInput<MouseButton>>>` to the system parameter list so
`mouse_just_pressed` can be computed once per frame. Replace the bare
`*interaction == Interaction::Pressed` check with
`mouse_input::is_activated(interaction, interaction_ref, mjp)`.

The `update_button_colors` system already reads `Interaction::Hovered` for
visual hover feedback — no change needed there.

No changes to `handle_button_press` or `handle_menu_keyboard`.

#### 2.2 Settings Slider Mouse Support

The `SettingSlider` type in `src/game/systems/menu.rs` is currently driven only
by Left/Right arrow keys in `handle_menu_keyboard`. Add mouse support to the
slider Bevy-UI widgets:

- In `spawn_settings_menu`, change slider track nodes from `Sense::hover()` to
  include click detection. Because this is a Bevy-UI node tree (not egui), add
  a `SliderTrack` marker component carrying the slider identity
  (`VolumeType` or equivalent) to the track node.
- Add a new system `handle_slider_mouse` (registered in `MenuPlugin::build`)
  that queries `(&Interaction, Ref<Interaction>, &SliderTrack)` with
  `With<Button>`. On activation via `mouse_input::is_activated`, compute a
  normalised click position from the cursor position relative to the node's
  computed bounding rect and update the corresponding `SettingSlider` value in
  `GlobalState.config`.
- Also handle `Interaction::Hovered` while `ButtonInput<MouseButton>::pressed`
  (drag) so holding the mouse button and moving adjusts the slider continuously,
  matching the feel of a real slider widget. Use `CursorMoved` events or
  `Window::cursor_position` to read the current pointer position within the
  frame.

If the settings sliders are instead refactored to use egui (a simpler
implementation), use `egui::Slider` or `ui.add(egui::DragValue::...)` and wire
the returned value back into `GlobalState.config`. Either approach is
acceptable; the egui route is less code. The chosen approach must be documented
in the implementation summary.

#### 2.3 Testing Requirements

**Menu button tests:**

- `test_mouse_click_resume_button` — insert `Interaction::Pressed` on a Resume
  entity; verify `GlobalState` mode is no longer `Menu`.
- `test_mouse_hovered_click_save_button` — set `Interaction::Hovered` on a
  Save entity and `ButtonInput<MouseButton>::just_pressed(Left)`; verify
  `MenuState` submenu transitions to `SaveLoad`.
- `test_mouse_hover_does_not_dispatch_menu` — `Interaction::Hovered` alone must
  not trigger any action.

**Slider tests:**

- `test_slider_mouse_click_sets_value` — simulate a click at the centre of a
  slider track; verify the corresponding config volume value is updated.
- `test_slider_drag_updates_value` — simulate a held mouse button at two
  positions on the track; verify value changes between the two positions.

#### 2.4 Deliverables

- [ ] `menu_button_interaction` updated with hovered-click fallback using Phase
      1 helpers.
- [ ] `SliderTrack` marker component added; slider nodes upgraded to respond to
      mouse clicks and drags.
- [ ] `handle_slider_mouse` system registered in `MenuPlugin`.
- [ ] All tests from §2.3 implemented and passing.
- [ ] Quality gates green.

#### 2.5 Success Criteria

- Clicking any menu button in a headless test fires the correct action,
  identical to keyboard `Enter`.
- A player can adjust all audio settings sliders using mouse click and drag
  without touching the keyboard.
- Hovering alone never triggers any action.

---

### Phase 3: Dialogue Mouse Support

**Goal:** Allow mouse clicks to advance dialogue and select choices, making the
dialogue system fully operable without a keyboard.

#### 3.1 Dialogue Advance on Click

The `dialogue_input_system` in `src/game/systems/dialogue.rs` advances dialogue
via `Space`/`E`. Add a parallel mouse path: when the game is in
`GameMode::Dialogue` and a left mouse click is received, emit `AdvanceDialogue`.

Examine `src/game/systems/dialogue_visuals.rs` to determine whether the
dialogue text panel is a Bevy-UI node tree or an egui window:

- **If Bevy-UI:** Add a `Button` + `Interaction::None` component to the
  dialogue text container node spawned in `handle_start_dialogue`. In
  `dialogue_input_system`, add `Option<Res<ButtonInput<MouseButton>>>` and
  query the container node's `Interaction`; use `mouse_input::is_activated` to
  detect clicks.
- **If egui:** Add a transparent `egui::Button` or use `ui.interact(rect,
id, Sense::click()).clicked()` over the dialogue text area to detect clicks
  and emit `AdvanceDialogue`.

#### 3.2 Dialogue Choice Buttons

The choice UI spawned by `spawn_choice_ui` in
`src/game/systems/dialogue_choices.rs` creates `Text` / layout nodes without
`Button` components. To enable mouse clicks:

- Add a `ChoiceButton { pub choice_index: usize }` marker component (derived
  `Component`) to `dialogue_choices.rs`.
- In `spawn_choice_ui`, add `Button`, `Interaction::None`, and `ChoiceButton {
choice_index: i }` to each spawned choice row node.
- In `choice_input_system`, add a second query:
  `Query<(&Interaction, Ref<Interaction>, &ChoiceButton), With<Button>>` plus
  `Option<Res<ButtonInput<MouseButton>>>`. On activation via
  `mouse_input::is_activated`, immediately emit `SelectDialogueChoice {
choice_index }` and reset `ChoiceSelectionState` — matching the existing
  digit-key immediate-confirm path.

#### 3.3 Testing Requirements

- `test_mouse_click_advances_dialogue` — `Interaction::Pressed` on the dialogue
  text panel (or equivalent egui click simulation) emits `AdvanceDialogue`.
- `test_mouse_click_choice_dispatches_select` — `Interaction::Pressed` on a
  `ChoiceButton { choice_index: 1 }` entity emits
  `SelectDialogueChoice { choice_index: 1 }`.
- `test_mouse_hover_choice_does_not_select` — `Interaction::Hovered` alone on a
  `ChoiceButton` must not emit `SelectDialogueChoice`.
- `test_mouse_click_choice_resets_choice_state` — after click-selection,
  `ChoiceSelectionState::selected_index` resets to `0` and
  `choice_count` resets to `0`.

#### 3.4 Deliverables

- [ ] Dialogue text panel wired for advance-on-click (Bevy-UI or egui path,
      documented in implementation summary).
- [ ] `ChoiceButton` marker component added to `dialogue_choices.rs`.
- [ ] Choice nodes spawned with `Button`, `Interaction::None`, and
      `ChoiceButton`.
- [ ] `choice_input_system` extended with mouse activation loop.
- [ ] All tests from §3.3 implemented and passing.
- [ ] Quality gates green.

#### 3.5 Success Criteria

- Clicking the dialogue panel advances the conversation, identical to pressing
  `Space`.
- Clicking a choice immediately selects it, identical to pressing the
  corresponding digit key.
- Hovering alone never triggers any action.

---

### Phase 4: Inventory, Merchant, and Container Mouse Support

**Goal:** Complete the partially-scaffolded egui mouse paths in all three
inventory screens so a player can select items and trigger actions using only
mouse clicks, without needing to navigate with arrow keys first.

**Architecture note:** All three inventory screens are **egui-based** and must
remain so. The fixes in this phase work entirely within the existing egui render
functions by changing `Sense::hover()` to `Sense::click()` on slot cells and
connecting the already-allocated `response.clicked()` return values to state
updates.

#### 4.1 Inventory Slot Grid Click-to-Select

In `render_character_panel` in `src/game/systems/inventory_ui.rs`, the slot
grid cells are drawn via `egui::Painter` without allocating interactive
responses. To add click-to-select:

- Change the body allocation from `Sense::hover()` to create per-cell
  interactive rects using `ui.allocate_rect(cell_rect, Sense::click())`. Each
  allocation returns a `Response`; when `response.clicked()`, update
  `InvState.selected_slot = Some(slot_idx)` and
  `InventoryNavigationState.selected_slot_index = Some(slot_idx)`.
- Because `render_character_panel` currently returns only `Option<PanelAction>`,
  extend the return type to a small struct
  `CharacterPanelResult { action: Option<PanelAction>, clicked_slot: Option<usize> }`
  so the caller (`inventory_ui_system`) can apply the slot selection to
  `GlobalState` after the panel returns.
- A click on a slot that has an item should also advance to
  `NavigationPhase::ActionNavigation` immediately (matching the keyboard
  `Enter` path), so the action strip appears without a second click. A click on
  an empty slot selects it visually but stays in `SlotNavigation`.

The existing action strip buttons (Drop / Give) already use `egui::Button` with
`.clicked()` and work correctly — no changes needed there.

#### 4.2 Merchant Inventory: Connect Disconnected Stock Row Selection

In `render_merchant_stock_panel` in
`src/game/systems/merchant_inventory_ui.rs`, the stock rows already use
`Sense::click()` and check `response.clicked()`, but the result is discarded
after the comment "Mouse click on a stock row selects it". Fix this by:

- Returning the clicked stock row index from `render_merchant_stock_panel`
  (extend its return type from `Option<BuyAction>` to a struct
  `MerchantStockPanelResult { buy_action: Option<BuyAction>, clicked_row:
Option<usize> }`).
- In `merchant_inventory_ui_system`, when `clicked_row` is `Some(i)`, update
  `ms.merchant_selected_slot = Some(i)` and
  `nav_state.selected_slot_index = Some(i)`. If the row has content, also enter
  `NavigationPhase::ActionNavigation` so the Buy button appears immediately.

For the character sell panel, apply the same slot-cell click-to-select fix
described in §4.1 (the two panels share the same painter-drawn grid pattern):

- Change `render_character_sell_panel` to allocate per-cell click rects and
  return a `SellPanelResult { sell_action: Option<SellAction>, clicked_slot:
Option<usize> }`.
- Wire `clicked_slot` back to `ms.character_selected_slot` and
  `nav_state.selected_slot_index` in `merchant_inventory_ui_system`.

#### 4.3 Container Inventory: Connect Ignored Row Click Response

In `render_container_items_panel` in
`src/game/systems/container_inventory_ui.rs`, container item rows already use
`Sense::click()` but store the response as `_response` (ignored). Fix this by:

- Renaming `_response` to `response` and acting on `response.clicked()`: update
  `container_state.container_selected_slot = Some(i)` and
  `nav_state.selected_slot_index = Some(i)`. If the slot has content, also
  enter `NavigationPhase::ActionNavigation` so the Take/Take All buttons appear
  immediately.
- Propagate the clicked row index back through the return type of
  `render_container_items_panel` (extend `ContainerPanelResult` with a
  `Selected(usize)` variant, or use the existing `None` branch with a separate
  return field).

For the character stash panel, apply the same slot-cell click-to-select fix as
§4.1.

The existing Take, Take All, and Stash action buttons already use
`egui::Button` with `.clicked()` — no changes needed there.

#### 4.4 Testing Requirements

**Inventory:**

- `test_mouse_click_slot_with_item_enters_action_mode` — simulate
  `response.clicked()` on a slot that contains an item; verify
  `nav_state.phase == NavigationPhase::ActionNavigation` and
  `selected_slot_index == Some(slot_idx)`.
- `test_mouse_click_empty_slot_selects_only` — click an empty slot; verify
  selection moves but phase stays `SlotNavigation`.
- `test_mouse_click_drop_button_emits_action` — click Drop button after slot is
  selected; verify `DropItemAction` is emitted with correct indices.

**Merchant:**

- `test_mouse_click_stock_row_updates_selection` — simulate
  `response.clicked()` on stock row `i`; verify `ms.merchant_selected_slot ==
Some(i)`.
- `test_mouse_click_available_stock_row_enters_action_mode` — click an
  available stock row; verify `nav_state.phase == ActionNavigation`.
- `test_mouse_click_buy_button_emits_action` — click Buy button after row is
  selected; verify `BuyItemAction` is emitted.
- `test_mouse_click_character_slot_updates_sell_selection` — click a character
  inventory slot; verify selection updates and action mode is entered if item
  present.

**Container:**

- `test_mouse_click_container_row_updates_selection` — simulate
  `response.clicked()` on container row `i`; verify
  `cs.container_selected_slot == Some(i)`.
- `test_mouse_click_container_row_with_item_enters_action_mode` — click a row
  with an item; verify `nav_state.phase == ActionNavigation`.
- `test_mouse_click_take_button_emits_action` — click Take button; verify
  `TakeItemAction` is emitted.
- `test_mouse_click_take_all_emits_action` — click Take All button; verify
  `TakeAllAction` is emitted.

#### 4.5 Deliverables

- [ ] `render_character_panel` allocates per-cell click rects; return type
      extended to carry `clicked_slot`.
- [ ] `inventory_ui_system` wires `clicked_slot` to `InventoryState` and
      `InventoryNavigationState`.
- [ ] `render_merchant_stock_panel` return type extended; `merchant_inventory_ui_system`
      wires clicked row to merchant selection state.
- [ ] `render_character_sell_panel` per-cell click rects added; return type
      extended; wired in `merchant_inventory_ui_system`.
- [ ] `_response` renamed to `response` in container item rows; click wired to
      container selection state in `container_inventory_ui_system`.
- [ ] Character stash panel per-cell click rects added; wired in
      `container_inventory_ui_system`.
- [ ] All tests from §4.4 implemented and passing.
- [ ] Quality gates green.

#### 4.6 Success Criteria

- A player can click any inventory slot to select it and immediately see the
  action strip appear, with no arrow-key navigation required.
- A player can click a merchant stock row to select it and click Buy in a
  single mouse interaction flow.
- A player can click a container item row to select it and click Take in a
  single mouse interaction flow.
- All existing keyboard navigation tests for all three screens pass unchanged.

---

### Phase 5: Inn Management Keyboard/Mouse Parity Audit

**Goal:** Validate that the already-present egui mouse paths in `inn_ui.rs` are
complete and that a player can accomplish every inn action using mouse alone.

#### 5.1 Audit Current Inn Mouse Paths

Cross-check each action in `inn_action_system` against the keyboard-only
`inn_input_system`:

| Action                | Keyboard                         | Mouse today                                                                                        |
| --------------------- | -------------------------------- | -------------------------------------------------------------------------------------------------- |
| Select party member   | Arrow keys + Enter               | `response.clicked()` on card ✅                                                                    |
| Dismiss party member  | `D` key                          | `Dismiss` button inside card ✅                                                                    |
| Select roster member  | Arrow keys + Enter               | `response.clicked()` on card ✅                                                                    |
| Recruit roster member | `R` key                          | `Recruit` button inside card ✅                                                                    |
| Swap party ↔ roster   | `S` key (needs both selected)    | `Swap` button — only visible when `selected_party.or(nav_state.selected_party_index)` is `Some` ⚠️ |
| Exit inn              | `Escape` / `Enter` on Exit focus | Exit button ✅                                                                                     |

The key gap: `Swap` requires a party member to be pre-selected. The `Swap`
button is conditionally rendered only when
`selected_party.or(nav_state.selected_party_index)` is `Some`. A pure-mouse
user must click a party card first, then click Swap on a roster card.
Verify whether `inn_selection_system` sets `InnNavigationState::selected_party_index`
when processing a `SelectPartyMember` event arriving from a mouse click
(not just keyboard focus). If it does not, this is the gap to fix.

#### 5.2 Fix Any Gaps Found

If `inn_selection_system` only updates the mouse-facing
`InnManagementState::selected_party_slot` but not
`InnNavigationState::selected_party_index` on a mouse-originated
`SelectPartyMember` event, update it to set both fields. This ensures
keyboard Swap (`S` key) and mouse Swap button both see the same selection
regardless of input method.

Similarly ensure that clicking a roster card sets
`InnNavigationState::selected_roster_index` so keyboard `S` also sees the
mouse selection.

#### 5.3 Testing Requirements

- `test_mouse_only_swap_flow` — emit `SelectPartyMember { party_index: 0 }`
  (simulating a card click), then emit `InnSwapCharacters { party_index: 0,
roster_index: 0 }`; verify both characters trade positions in `GlobalState`
  and `InnNavigationState.selected_party_index == Some(0)` after the first
  event.
- `test_mouse_dismiss_then_recruit` — emit Dismiss on a party card, then Recruit
  on a roster card; verify party size changes correctly.
- `test_swap_button_visible_after_mouse_party_select` — after processing a
  `SelectPartyMember` event, verify
  `InnNavigationState.selected_party_index == Some(n)` so the Swap button
  renders.

#### 5.4 Deliverables

- [ ] Inn mouse/keyboard parity audit findings documented as an inline block
      comment in `inn_ui.rs` (not a separate file).
- [ ] Any selection sync gaps in `inn_selection_system` fixed.
- [ ] All tests from §5.3 implemented and passing.
- [ ] Quality gates green.

#### 5.5 Success Criteria

- A player can complete a full inn visit — dismiss a member, recruit a new
  character, swap two members, and exit — using mouse alone without touching
  the keyboard.

---

### Phase 6: Exploration Mouse-to-Interact

**Goal:** Allow the player to click on an NPC, door, or interactive tile in the
first-person exploration view to trigger the same interaction that the keyboard
`Interact` action produces.

#### 6.1 Adopt `MeshPickingPlugin` (Long-Term Correct Approach)

Bevy 0.17 ships `bevy::picking` including `MeshPickingPlugin`. Register
`MeshPickingPlugin` in the game app. Mark NPC billboard meshes and door tile
meshes with `PickingBehavior` so `Pointer<Click>` events are delivered to
them. This is the correct long-term solution: it works for any clickable object
at any screen position, handles depth ordering, and will compose naturally with
future visual improvements (3-D geometry, sprite billboards, etc.).

Add a system `handle_world_click` in `src/game/systems/input.rs` (or a new
`src/game/systems/exploration_mouse.rs`) that:

- Guards on `GameMode::Exploration` — ignores clicks in any other mode.
- Listens for `Pointer<Click>` events on entities that carry an `NpcMarker`,
  `TileCoord`, or `MapEventMarker` component.
- Routes the click through the same NPC/door/event checking logic already used
  by the keyboard `Interact` path in `handle_input`, emitting the same
  `MapEventTriggered` or `DoorOpenedEvent` messages. No duplication of
  interaction logic.

#### 6.2 Fallback: Centre-Screen Click Heuristic

If `MeshPickingPlugin` integration proves infeasible within this pass (e.g.
due to billboard mesh incompatibility or pick-ray configuration complexity),
implement a minimal fallback: when `ButtonInput<MouseButton>::just_pressed(Left)`
fires in `Exploration` mode and the cursor is within the centre third of the
window, treat it as an `Interact` key press on the tile directly ahead of the
party. This is the same tile the keyboard `Interact` action targets and requires
no ray-cast infrastructure.

Document clearly in `exploration_mouse.rs` which approach was implemented and
leave a `TODO` comment for upgrading to full picking if the fallback was chosen.

#### 6.3 Testing Requirements

- `test_world_click_npc_triggers_dialogue` — place an NPC in the tile ahead of
  the party; simulate a `Pointer<Click>` event (or `WorldClickEvent` for the
  fallback) on that entity/position; verify `MapEventTriggered` is emitted with
  an `NpcDialogue` event.
- `test_world_click_blocked_outside_exploration_mode` — same click in `Combat`
  or `Menu` mode must not emit `MapEventTriggered`.

#### 6.4 Deliverables

- [ ] `MeshPickingPlugin` registered (or fallback centre-click heuristic
      implemented and documented).
- [ ] `handle_world_click` system wired through existing interaction logic.
- [ ] Tests from §6.3 implemented and passing.
- [ ] Quality gates green.

#### 6.5 Success Criteria

- Clicking an NPC visible in the exploration view starts dialogue, identical to
  pressing `Interact` while adjacent.
- No change to keyboard-driven exploration behaviour.

---

### Phase 7: Regression Test Suite and Documentation

**Goal:** Collect one representative mouse regression test per major game mode
into a structured integration test file, and document the canonical mouse input
model for future contributors.

#### 7.1 Create `tests/mouse_input_regression.rs`

Add an integration test file covering one representative mouse interaction per
mode:

| Test name                                 | Mode                      | Verified behaviour                                                       |
| ----------------------------------------- | ------------------------- | ------------------------------------------------------------------------ |
| `regression_combat_action_button_click`   | `Combat`                  | `ActionButton::Attack` click dispatches into target selection            |
| `regression_combat_enemy_card_click`      | `Combat` target selection | `EnemyCard` click emits `AttackAction` with correct monster index        |
| `regression_menu_resume_click`            | `Menu`                    | Resume button click restores previous mode                               |
| `regression_menu_settings_slider_click`   | `Menu` (Settings)         | Slider click updates config volume                                       |
| `regression_dialogue_advance_click`       | `Dialogue`                | Panel click emits `AdvanceDialogue`                                      |
| `regression_dialogue_choice_click`        | `Dialogue` (choices)      | `ChoiceButton(1)` click emits `SelectDialogueChoice { choice_index: 1 }` |
| `regression_inventory_slot_click_selects` | `Inventory`               | Slot click with item enters `ActionNavigation`                           |
| `regression_inventory_drop_click`         | `Inventory`               | Drop button click emits `DropItemAction`                                 |
| `regression_merchant_stock_row_click`     | `MerchantInventory`       | Stock row click updates selection state                                  |
| `regression_merchant_buy_click`           | `MerchantInventory`       | Buy button click emits `BuyItemAction`                                   |
| `regression_container_row_click`          | `ContainerInventory`      | Container row click updates selection state                              |
| `regression_container_take_click`         | `ContainerInventory`      | Take button click emits `TakeItemAction`                                 |
| `regression_inn_swap_mouse_only`          | `InnManagement`           | Mouse-only select party + swap flow completes                            |
| `regression_exploration_click_interact`   | `Exploration`             | Click on NPC/tile-ahead emits `MapEventTriggered`                        |

Each test builds a minimal `App` with `MinimalPlugins` plus the relevant plugin
(no renderer required). Mouse input is injected by inserting
`ButtonInput::<MouseButton>` with `.press(MouseButton::Left)` for raw button
input and `Interaction::Pressed` on the target entity for Bevy-UI tests.

All test data must use `data/test_campaign` per Implementation Rule 5 — no
reference to `campaigns/tutorial`.

#### 7.2 Document the Canonical Mouse Input Model

Add a `/// # Mouse Input Model` section to the `mouse_input` module-level doc
comment explaining:

- The dual-path activation model and why both paths are necessary (platform
  reliability of `Interaction::Pressed` in Bevy 0.17).
- The rule for egui-based screens: use `response.clicked()` / `Sense::click()`
  — do **not** add Bevy `Button` + `Interaction` components to egui-managed
  nodes.
- The rule for Bevy-UI-button screens: add `Button` + `Interaction::None` to
  the spawned node, add `Option<Res<ButtonInput<MouseButton>>>` to the system
  signature, and call `mouse_input::is_activated`.
- How to add mouse support to a new system in three steps.

Update `docs/explanation/implementations.md` with a summary entry for this
feature.

#### 7.3 Deliverables

- [ ] `tests/mouse_input_regression.rs` with all 14 regression tests passing.
- [ ] `mouse_input.rs` module doc includes the Mouse Input Model section.
- [ ] `docs/explanation/implementations.md` updated.
- [ ] `cargo nextest run --all-features` green.

#### 7.4 Success Criteria

- All 14 regression tests pass.
- Running `grep -rn "just_pressed(MouseButton" src/` returns results only
  inside `mouse_input.rs` and in callers of `mouse_input::mouse_just_pressed`
  — no ad-hoc inline patterns remain anywhere else.
- Zero warnings from `cargo clippy --all-targets --all-features -- -D warnings`.

---

## Recommended Implementation Order

| Phase | Description                                        | Depends on    | Risk   | Effort |
| ----- | -------------------------------------------------- | ------------- | ------ | ------ |
| 1     | Shared `MouseActivation` utility + Combat refactor | —             | Low    | Small  |
| 2     | Menu buttons + slider mouse support                | Phase 1       | Medium | Medium |
| 3     | Dialogue advance + choice clicks                   | Phase 1       | Medium | Medium |
| 4     | Inventory / Merchant / Container egui fixes        | — (egui only) | Low    | Medium |
| 5     | Inn management parity audit + gap fix              | —             | Low    | Small  |
| 6     | Exploration click-to-interact                      | —             | Medium | Medium |
| 7     | Regression suite + documentation                   | Phases 1–6    | Low    | Small  |

Phases 1, 4, and 5 are independent of each other and can be developed in
parallel. Phase 2 and Phase 3 depend on the Phase 1 helper for Bevy-UI
activation. Phase 6 is fully independent. Phase 7 depends on all prior phases
being complete.
