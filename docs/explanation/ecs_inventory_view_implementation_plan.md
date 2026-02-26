# ECS Inventory View Implementation Plan

## Overview

This plan implements an inventory screen for the Antares turn-based RPG using
the project's established egui-based UI pattern (matching `inn_ui.rs`). The
domain types (`Inventory`, `InventorySlot`, `Equipment`) remain structurally
identical; each party member entity gets a `CharacterEntity` component and the
`PartyEntities` resource maps party indices to Bevy entities. On top of that
foundation, a new `GameMode::Inventory(InventoryState)` drives a full-screen
egui overlay that shows up to six character inventory panels simultaneously,
supports Tab-cycling between characters, and allows item drop and item transfer
between party members. The inventory key defaults to `"I"` and is fully
configurable via `ControlsConfig`.

**UI Framework Decision:** This plan uses `bevy_egui` (egui) for all inventory
UI rendering, matching the established pattern in `src/game/systems/inn_ui.rs`.
The `MenuPlugin` uses Bevy native UI (Node/Button), but `InnUiPlugin` uses
egui. The inventory system is more similar to the inn management UI in scope
and interaction model; therefore it follows the `InnUiPlugin` pattern.

---

## Current State Analysis

### Existing Infrastructure

| File                         | Relevant Content                                                                                                                                                                                                                                      |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/character.rs`    | `Inventory` (`MAX_ITEMS = 6`), `InventorySlot`, `Equipment` (`MAX_EQUIPPED = 6`); nested inside `Character` inside `Party` inside `GameState`                                                                                                         |
| `src/application/mod.rs`     | `GameMode` enum (derives `Debug, Clone, PartialEq, Serialize, Deserialize`) with `Exploration`, `Combat(CombatState)`, `Menu(MenuState)`, `Dialogue(DialogueState)`, `InnManagement(InnManagementState)`; `GameState::enter_menu()` pattern to follow |
| `src/application/menu.rs`    | `MenuState` + `MenuType` enum; `get_resume_mode()`, `set_submenu()`, `select_next()`, `select_previous()` navigation helpers; `previous_mode: Box<GameMode>` to break recursive size                                                                  |
| `src/game/systems/inn_ui.rs` | **Primary pattern to follow.** `InnUiPlugin` uses `bevy_egui::EguiContexts`, `add_message::<T>()`, chained systems `(inn_input_system, inn_selection_system, inn_ui_system, inn_selection_system, inn_action_system)`, `InnNavigationState` resource  |
| `src/game/systems/menu.rs`   | `MenuPlugin` with Bevy native UI (`Node`, `Button`, `MenuRoot` component, `menu_setup`/`menu_cleanup`); **do not follow for inventory**                                                                                                               |
| `src/game/systems/hud.rs`    | `HudPlugin`, `CharacterCard { party_index }`, `HUD_PANEL_HEIGHT`, `HUD_BOTTOM_GAP`; HUD uses Bevy native UI and must remain visible when inventory is open                                                                                            |
| `src/game/systems/input.rs`  | `GameAction` enum (6 variants), `KeyMap`, `InputConfigResource`, `toggle_menu_state` helper; `handle_input` blocks all input in `GameMode::Menu(_)` and `GameMode::Combat(_)`                                                                         |
| `src/sdk/game_config.rs`     | `ControlsConfig` with `move_forward`, `move_back`, `turn_left`, `turn_right`, `interact`, `menu` fields; `validate()` only checks `movement_cooldown >= 0`                                                                                            |
| `src/game/systems/mod.rs`    | Lists all system modules; `inn_ui` already registered                                                                                                                                                                                                 |
| `src/bin/antares.rs`         | `AntaresPlugin::build` registers `DialoguePlugin`, `QuestPlugin`, `InnUiPlugin`, `RecruitmentDialogPlugin`, `MenuPlugin`, `CombatPlugin`; `HudPlugin` and `InputPlugin` registered elsewhere                                                          |
| `src/domain/transactions.rs` | `buy_item`, `sell_item`, `consume_service`; pure domain functions; `sell_item` takes `party`, `character`, `character_id`, `npc_runtime`, `npc_def`, `item_id`, `item_db`                                                                             |
| `src/game/components/mod.rs` | Exports `billboard`, `combat`, `creature`, `dialogue`, `furniture`, `menu`, `performance`, `sprite` — **no `inventory` module yet**                                                                                                                   |

### Identified Issues

1. **No `GameMode::Inventory` variant.** The `GameMode` enum has five variants;
   there is no inventory state, no entry point, and no cleanup.

2. **`GameAction::Inventory` does not exist.** `ControlsConfig` has no
   `inventory` key field. `KeyMap::from_controls_config` does not map `"I"` to
   any action. `handle_input` has no branch for opening the inventory.

3. **`InventoryState` does not exist.** There is no
   `src/application/inventory_state.rs`, no `InventoryState` struct, and no
   `GameState::enter_inventory()` method.

4. **No inventory UI system.** There is no `src/game/systems/inventory_ui.rs`,
   no `InventoryPlugin`, and no egui rendering function for inventory panels.

5. **No `CharacterEntity` component or `PartyEntities` resource.** Party members
   have no Bevy `Entity` of their own; `CharacterCard` uses a `party_index: usize`
   lookup against `GlobalState`. These ECS types do not exist yet.

6. **No item action message types.** There are no `DropItemAction` or
   `TransferItemAction` message types for the inventory system.

7. **`src/game/components/inventory.rs` does not exist.** There is no component
   file for inventory-related ECS types.

8. **`Inventory`, `InventorySlot`, and `Equipment` do not derive `Component`.**
   They are plain domain structs.

---

## Implementation Phases

### Phase 1: ECS Foundation — Component Wrappers and Entity Spawning

Establish the minimum ECS surface area needed by later phases without changing
any domain logic, field definitions, or existing tests.

#### 1.1 Add `#[derive(Component)]` to `Inventory`, `InventorySlot`, and `Equipment`

**File:** `src/domain/character.rs`

**Exact change:** Add `Component` to the `#[derive(...)]` attribute on each of
the three structs. Import `bevy::prelude::Component` unconditionally at the top
of the file (matching the style of other files that import bevy directly;
`src/domain/character.rs` already uses `serde` derives without feature gates).

- `InventorySlot` already derives `Debug, Clone, Copy, PartialEq, Eq,
Serialize, Deserialize` — add `Component` after `Eq`.
- `Inventory` already derives `Debug, Clone, PartialEq, Serialize,
Deserialize` — add `Component` after `Deserialize`.
- `Equipment` already derives `Debug, Clone, PartialEq, Serialize,
Deserialize` — add `Component` after `Deserialize`.

**No field changes, no method changes, no constant changes.**

**Validation:** `cargo check --all-targets --all-features` passes. All existing
`test_inventory_*` and `test_equipment_*` tests still pass.

#### 1.2 Create `src/game/components/inventory.rs` with `CharacterEntity` and `PartyEntities`

**File:** `src/game/components/inventory.rs` (new file)

Add SPDX header as first two lines.

Define:

```rust
/// Links a Bevy entity to the party member at a given party index.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterEntity {
    pub party_index: usize,
}
```

Define a `PartyEntities` resource using `PARTY_MAX_SIZE` from
`crate::domain::character::PARTY_MAX_SIZE` (value is `6`):

```rust
/// Resource mapping party_index (0–PARTY_MAX_SIZE-1) to the Bevy Entity for that character.
#[derive(Resource, Debug)]
pub struct PartyEntities {
    pub entities: [Option<Entity>; PARTY_MAX_SIZE],
}

impl Default for PartyEntities {
    fn default() -> Self {
        Self {
            entities: [None; PARTY_MAX_SIZE],
        }
    }
}
```

Add `/// doc comment` to every public item. Include `use bevy::prelude::*` and
`use crate::domain::character::PARTY_MAX_SIZE`.

#### 1.3 Export `inventory` submodule from `src/game/components/mod.rs`

**File:** `src/game/components/mod.rs`

Add `pub mod inventory;` to the module list (alphabetical order: between
`furniture` and `menu`).

Add re-exports for `CharacterEntity` and `PartyEntities` at the bottom of the
`pub use` block:

```rust
pub use inventory::{CharacterEntity, PartyEntities};
```

#### 1.4 Spawn party member entities in `HudPlugin` startup

**File:** `src/game/systems/hud.rs`

Add a new startup system `setup_party_entities` to `HudPlugin::build`. Register
it with `app.add_systems(Startup, setup_party_entities)` alongside the existing
`setup_hud` registration.

`setup_party_entities` signature:

```rust
fn setup_party_entities(
    mut commands: Commands,
    global_state: Res<GlobalState>,
) {
```

Implementation:

1. Call `commands.init_resource::<PartyEntities>()`.
2. Iterate `0..PARTY_MAX_SIZE`.
3. For each `party_index`, spawn a Bevy entity with `CharacterEntity { party_index }`.
4. Store each `Entity` in a local `[Option<Entity>; PARTY_MAX_SIZE]` array.
5. Insert the populated `PartyEntities` resource via
   `commands.insert_resource(PartyEntities { entities })`.

These entities carry no mesh or transform — they are pure ECS identity nodes.
No existing `CharacterCard` or `CharacterPortrait` logic changes.

#### 1.5 Testing Requirements for Phase 1

All tests go in a `#[cfg(test)] mod tests` block within their respective files.
Use `bevy::prelude::World` for component derive tests (no full `App` needed for
basic component insertion).

- `test_inventory_component_derive` — create a Bevy `World`, spawn an entity
  with `Inventory::new()`, query it back, assert it exists. **File:**
  `src/domain/character.rs`.
- `test_inventory_slot_component_derive` — same for
  `InventorySlot { item_id: 1, charges: 3 }`. **File:** `src/domain/character.rs`.
- `test_equipment_component_derive` — same for `Equipment::new()`. **File:**
  `src/domain/character.rs`.
- `test_character_entity_component` — assert `CharacterEntity { party_index: 2 }`
  can be inserted into a `World` and queried back with matching `party_index`.
  **File:** `src/game/components/inventory.rs`.
- `test_party_entities_resource_default` — assert `PartyEntities::default()`
  has all `None` slots and `entities.len() == PARTY_MAX_SIZE`. **File:**
  `src/game/components/inventory.rs`.
- `test_party_entities_resource_init` — build a minimal `App`, call
  `app.init_resource::<PartyEntities>()`, verify resource exists with all `None`
  slots. **File:** `src/game/components/inventory.rs`.

#### 1.6 Deliverables

- [ ] `Inventory`, `InventorySlot`, `Equipment` derive `Component` in
      `src/domain/character.rs`
- [ ] `src/game/components/inventory.rs` created with `CharacterEntity` and
      `PartyEntities` (SPDX header, doc comments, tests)
- [ ] `src/game/components/mod.rs` updated to declare and re-export the
      `inventory` submodule
- [ ] `setup_party_entities` startup system added to `HudPlugin`; inserts
      `PartyEntities` resource with one entity per party slot
- [ ] All six unit tests from Section 1.5 passing

#### 1.7 Success Criteria

- `cargo clippy --all-targets --all-features -- -D warnings` reports zero
  warnings
- `cargo nextest run --all-features` passes all pre-existing tests plus new
  Phase 1 tests
- `PartyEntities` resource is accessible from any Bevy system via
  `Res<PartyEntities>`
- No domain struct field or method signatures changed

---

### Phase 2: Input and Mode Wiring

Add the `Inventory` key binding and `GameMode::Inventory(InventoryState)` so
that pressing `"I"` transitions the game into an inventory context without
touching the existing `Menu` mode or HUD.

#### 2.1 Add `inventory` key field to `ControlsConfig`

**File:** `src/sdk/game_config.rs`

Add the following field to `pub struct ControlsConfig` after the `menu` field:

```rust
/// Keys for opening the inventory screen
#[serde(default = "default_inventory_keys")]
pub inventory: Vec<String>,
```

Add the private default function immediately after the existing default
functions (follow the pattern of other `fn default_*` functions in the file):

```rust
fn default_inventory_keys() -> Vec<String> {
    vec!["I".to_string()]
}
```

Update `impl Default for ControlsConfig` to include:

```rust
inventory: default_inventory_keys(),
```

Update `ControlsConfig::validate()` to check that `inventory` is not empty:

```rust
if self.inventory.is_empty() {
    return Err(ConfigError::ValidationError(
        "inventory key list must not be empty".to_string(),
    ));
}
```

The `#[serde(default)]` annotation ensures existing RON campaign config files
without the `inventory` field deserialize without errors.

#### 2.2 Add `GameAction::Inventory` and wire `KeyMap`

**File:** `src/game/systems/input.rs`

Add the `Inventory` variant to `pub enum GameAction`:

```rust
/// Open or close the inventory screen
Inventory,
```

In `KeyMap::from_controls_config`, add a loop over `config.inventory` that maps
each key string to `GameAction::Inventory`. Place it after the existing `menu`
loop, following the exact same pattern:

```rust
// Map inventory keys
for key_str in &config.inventory {
    if let Some(key_code) = parse_key_code(key_str) {
        bindings.insert(key_code, GameAction::Inventory);
    } else {
        warn!("Invalid key code in inventory: {}", key_str);
    }
}
```

#### 2.3 Create `src/application/inventory_state.rs` with `InventoryState`

**File:** `src/application/inventory_state.rs` (new file)

Add SPDX header as first two lines.

Define `InventoryState` matching the `MenuState` pattern from
`src/application/menu.rs` exactly (same derives, same `Box<GameMode>` trick):

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryState {
    /// The GameMode active before the inventory was opened (for resume on close)
    pub previous_mode: Box<GameMode>,
    /// Which character panel is currently focused (0..party_size, wraps on Tab)
    pub focused_index: usize,
    /// Which party indices currently have open panels (1–6 panels visible)
    pub open_panels: Vec<usize>,
    /// Currently selected slot index within the focused panel (None = no selection)
    pub selected_slot: Option<usize>,
}
```

Implement the following methods with full `///` doc comments and `# Examples`
blocks:

- `pub fn new(previous_mode: GameMode) -> Self` — sets `focused_index = 0`,
  `open_panels = vec![0]`, `selected_slot = None`.
- `pub fn get_resume_mode(&self) -> GameMode` — returns
  `(*self.previous_mode).clone()`. Matches `MenuState::get_resume_mode`.
- `pub fn tab_next(&mut self, party_size: usize)` — if `party_size == 0`,
  return early. Advance `focused_index = (focused_index + 1) % party_size`. If
  `focused_index` is not in `open_panels` and `open_panels.len() <
PARTY_MAX_SIZE`, push it.
- `pub fn tab_prev(&mut self, party_size: usize)` — reverse direction with wrap:
  `if focused_index == 0 { focused_index = party_size - 1 } else {
focused_index -= 1 }`. Same panel-open logic as `tab_next`.
- `pub fn close_focused_panel(&mut self)` — remove `focused_index` from
  `open_panels`. If `open_panels` is now empty, re-add index `0` to avoid a
  state where the inventory is open but no panel is visible; the calling system
  should then transition back to `previous_mode`.
- `pub fn select_next_slot(&mut self, slot_count: usize)` — if `slot_count ==
0`, return. Set `selected_slot = Some((current + 1) % slot_count)` where
  `current = selected_slot.unwrap_or(0)`.
- `pub fn select_prev_slot(&mut self, slot_count: usize)` — reverse direction
  with wrap.

Implement `Default for InventoryState`:

```rust
impl Default for InventoryState {
    fn default() -> Self {
        Self::new(GameMode::Exploration)
    }
}
```

Add `use crate::application::GameMode;`, `use serde::{Deserialize, Serialize};`,
and `use crate::domain::character::PARTY_MAX_SIZE;` at the top.

#### 2.4 Register `inventory_state` module and add `GameMode::Inventory` variant

**File:** `src/application/mod.rs`

Add `pub mod inventory_state;` to the module declarations (after `pub mod
menu;`, in alphabetical order).

Add `Inventory` variant to `pub enum GameMode`:

```rust
/// Inventory management screen
Inventory(crate::application::inventory_state::InventoryState),
```

Place it between `InnManagement` and `Menu` (alphabetical order is acceptable;
match the existing ordering style of the enum).

Add `pub fn enter_inventory(&mut self)` to `impl GameState`, placed after
`enter_menu`:

```rust
/// Enters inventory mode, storing the current mode for resume on close.
pub fn enter_inventory(&mut self) {
    let prev = self.mode.clone();
    self.mode = GameMode::Inventory(
        crate::application::inventory_state::InventoryState::new(prev)
    );
}
```

#### 2.5 Wire `handle_input` to open/close the inventory

**File:** `src/game/systems/input.rs`

Add an import for `GameMode` at the top if not already present (it is used in
`toggle_menu_state`; check the existing imports).

In `handle_input`, add a branch for `GameAction::Inventory` using
`is_action_just_pressed`. Place this block **after** the existing `Menu` toggle
block and **before** the movement cooldown check, so it has the same priority as
the menu toggle:

```rust
// Check for inventory toggle ("I" key) — same priority as menu toggle.
if input_config
    .key_map
    .is_action_just_pressed(GameAction::Inventory, &keyboard_input)
{
    let game_state = &mut global_state.0;
    match &game_state.mode {
        crate::application::GameMode::Inventory(inv_state) => {
            // Close inventory: restore previous mode
            game_state.mode = inv_state.get_resume_mode();
            info!("Inventory closed: restored mode = {:?}", game_state.mode);
        }
        crate::application::GameMode::Menu(_)
        | crate::application::GameMode::Combat(_) => {
            // Do not open inventory from menu or combat mode
        }
        _ => {
            game_state.enter_inventory();
            info!("Inventory opened: mode = {:?}", game_state.mode);
        }
    }
    return; // Exit early after inventory toggle
}
```

Add the inventory mode to the existing input-blocking guard so that movement
input is suppressed while inventory is open. Find the block that reads:

```rust
if matches!(game_state.mode, crate::application::GameMode::Menu(_)) {
    return;
}
```

Extend it to also block on `Inventory`:

```rust
if matches!(
    game_state.mode,
    crate::application::GameMode::Menu(_)
        | crate::application::GameMode::Inventory(_)
) {
    return;
}
```

#### 2.6 Testing Requirements for Phase 2

All tests go in `#[cfg(test)] mod tests` blocks within their respective source
files unless specified otherwise.

- `test_controls_config_inventory_default` — assert
  `ControlsConfig::default().inventory == vec!["I".to_string()]`. **File:**
  `src/sdk/game_config.rs`.
- `test_controls_config_validate_empty_inventory_keys` — assert
  `ControlsConfig { inventory: vec![], ..Default::default() }.validate()`
  returns `Err(ConfigError::ValidationError(_))`. **File:**
  `src/sdk/game_config.rs`.
- `test_controls_config_validate_non_empty_inventory_keys` — assert
  `ControlsConfig::default().validate()` returns `Ok(())`. **File:**
  `src/sdk/game_config.rs`.
- `test_key_map_inventory_action` — build a `KeyMap` from default config; assert
  `key_map.get_action(KeyCode::KeyI) == Some(GameAction::Inventory)`. **File:**
  `src/game/systems/input.rs`.
- `test_inventory_state_new` — assert `focused_index == 0`, `open_panels ==
vec![0]`, `selected_slot == None`. **File:**
  `src/application/inventory_state.rs`.
- `test_inventory_state_tab_next_opens_panels` — call `tab_next` twice on a
  3-member party starting at index 0; assert `open_panels == vec![0, 1, 2]` and
  `focused_index == 2`. **File:** `src/application/inventory_state.rs`.
- `test_inventory_state_tab_next_wraps` — call `tab_next` on a 2-member party
  with `focused_index = 1`; assert `focused_index` wraps to `0`. **File:**
  `src/application/inventory_state.rs`.
- `test_inventory_state_tab_prev_wraps` — call `tab_prev` on a 2-member party
  with `focused_index = 0`; assert wraps to `1`. **File:**
  `src/application/inventory_state.rs`.
- `test_inventory_state_close_focused_panel` — open two panels (indices 0 and
  1), close the focused one; assert it is removed from `open_panels`. **File:**
  `src/application/inventory_state.rs`.
- `test_inventory_state_close_last_panel_keeps_one` — with only panel 0 open,
  call `close_focused_panel`; assert `open_panels` is not empty (it re-adds 0).
  **File:** `src/application/inventory_state.rs`.
- `test_inventory_state_select_next_slot` — with 6 slots and no selection,
  `select_next_slot(6)` sets `selected_slot = Some(1)`. **File:**
  `src/application/inventory_state.rs`.
- `test_game_mode_inventory_variant_constructable` — assert
  `GameMode::Inventory(InventoryState::default())` compiles and matches the
  `Inventory` variant. **File:** `src/application/mod.rs` (tests block).
- `test_enter_inventory_sets_mode` — create a `GameState`, call
  `enter_inventory()`, assert `matches!(game_state.mode,
GameMode::Inventory(_))`. **File:** `src/application/mod.rs` (tests block).
- `test_handle_input_i_opens_inventory` — Bevy `App` integration test mirroring
  the existing `test_escape_opens_and_closes_menu_via_button_input` test
  structure; press `KeyCode::KeyI`, run one update, assert mode is
  `GameMode::Inventory(_)`. **File:** `src/game/systems/input.rs`
  (`integration_tests` module).
- `test_handle_input_i_closes_inventory` — open inventory, press `KeyCode::KeyI`
  again, assert mode returns to `GameMode::Exploration`. **File:**
  `src/game/systems/input.rs` (`integration_tests` module).
- `test_movement_blocked_in_inventory_mode` — assert movement keys do not change
  party position while in `GameMode::Inventory(_)`. **File:**
  `src/game/systems/input.rs` (`combat_guard_tests` module or a new sibling
  module `inventory_guard_tests`).

#### 2.7 Deliverables

- [ ] `inventory` field added to `ControlsConfig` with `#[serde(default)]` and
      default value `["I"]`
- [ ] `ControlsConfig::validate()` rejects empty `inventory` list
- [ ] `GameAction::Inventory` added to enum in `src/game/systems/input.rs`
- [ ] `KeyMap::from_controls_config` maps `inventory` keys to
      `GameAction::Inventory`
- [ ] `src/application/inventory_state.rs` created with `InventoryState` and
      all navigation methods (SPDX header, doc comments, tests)
- [ ] `pub mod inventory_state;` declared in `src/application/mod.rs`
- [ ] `GameMode::Inventory(InventoryState)` variant added to `GameMode` enum
- [ ] `GameState::enter_inventory()` implemented in `src/application/mod.rs`
- [ ] `handle_input` opens/closes inventory on `GameAction::Inventory`
- [ ] Movement input blocked while in `GameMode::Inventory(_)`
- [ ] All sixteen tests from Section 2.6 passing

#### 2.8 Success Criteria

- Pressing `"I"` in-game transitions `GlobalState.0.mode` to
  `GameMode::Inventory(InventoryState { focused_index: 0, open_panels: [0], .. })`
- Pressing `"I"` again restores the previous mode
- All existing `test_escape_*` and `test_toggle_menu_state_*` tests still pass
- All existing `test_controls_config_*` tests still pass

---

### Phase 3: Inventory UI Panel Rendering

Build the egui overlay system that renders up to six character inventory panels,
following the `InnUiPlugin` pattern from `src/game/systems/inn_ui.rs` exactly.

#### 3.1 Create `src/game/systems/inventory_ui.rs`

**File:** `src/game/systems/inventory_ui.rs` (new file)

Add SPDX header as first two lines.

**Imports required:**

```rust
use crate::application::inventory_state::InventoryState;
use crate::application::GameMode;
use crate::domain::character::PARTY_MAX_SIZE;
use crate::game::resources::GlobalState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
```

**`InventoryPlugin`** — implement `Plugin` and register all systems in `fn build`.
Follow the `InnUiPlugin` pattern: use `add_message::<T>()` for action events,
`init_resource::<InventoryNavigationState>()` for navigation state, and chain
systems in order:

```rust
app.add_message::<DropItemAction>()
   .add_message::<TransferItemAction>()
   .init_resource::<InventoryNavigationState>()
   .add_systems(
       Update,
       (
           inventory_input_system,
           inventory_ui_system,
           inventory_action_system,
       )
           .chain(),
   );
```

**`InventoryNavigationState`** resource — mirrors `InnNavigationState`:

```rust
#[derive(Resource, Default, Debug)]
pub struct InventoryNavigationState {
    /// Index of the selected slot within the focused panel (None = header focused)
    pub selected_slot_index: Option<usize>,
    /// Which panel column has keyboard focus (maps to open_panels index, not party_index)
    pub focus_on_panel: usize,
}
```

**`inventory_input_system`** — reads `ButtonInput<KeyCode>` and
`InputConfigResource`. Runs every frame; only processes input when
`GlobalState.0.mode` is `GameMode::Inventory(_)`. Handles:

- `Tab` (no modifier) → call `inventory_state.tab_next(party_size)`, reset
  `selected_slot_index = None`.
- `Shift+Tab` → call `inventory_state.tab_prev(party_size)`, reset
  `selected_slot_index = None`.
- `ArrowUp` → call `inventory_state.select_prev_slot(slot_count)`.
- `ArrowDown` → call `inventory_state.select_next_slot(slot_count)`.
- `Escape` or configured `inventory` key (via `InputConfigResource`) → call
  `game_state.mode = inventory_state.get_resume_mode()` to close.

Where `slot_count = game_state.party.members[focused_index].inventory.items.len()`.
Perform bounds check before indexing into `party.members`.

**`inventory_ui_system`** — renders the egui inventory overlay. System signature:

```rust
fn inventory_ui_system(
    mut contexts: EguiContexts,
    mut global_state: ResMut<GlobalState>,
    nav_state: Res<InventoryNavigationState>,
)
```

Implementation steps:

1. Check `if let GameMode::Inventory(inv_state) = &global_state.0.mode` — if
   not inventory mode, return immediately.
2. Render using `egui::CentralPanel::default().show(ctx, |ui| { ... })` as the
   outer container.
3. Inside the panel: render a heading `"Inventory"` with a close hint
   `"[I] or [Esc] to close"`.
4. Render `ui.horizontal(|ui| { ... })` to lay out character panels side by side.
5. For each `party_index` in `inv_state.open_panels.clone()` (cloned to avoid
   borrow conflict), call the private helper `render_character_panel`.
6. Render a footer row showing focused character name and selected item details.

**`render_character_panel`** — private helper (not a system):

```rust
fn render_character_panel(
    ui: &mut egui::Ui,
    party_index: usize,
    is_focused: bool,
    selected_slot: Option<usize>,
    global_state: &GlobalState,
)
```

Implementation:

1. Bounds-check `party_index < global_state.0.party.members.len()`; return if
   out of bounds.
2. Get `character = &global_state.0.party.members[party_index]`.
3. Use `ui.push_id(party_index, |ui| { ... })` to scope all widgets — **this is
   mandatory** per `sdk/AGENTS.md` egui ID hygiene rules.
4. Inside `push_id`:
   - Render a `egui::Frame` with a distinct border color when `is_focused`
     (use `egui::Color32::YELLOW` for focused, `egui::Color32::DARK_GRAY` for
     unfocused).
   - Render character name as a heading.
   - Render `"Gold: {}"` with character's gold amount.
   - Render `"Items ({}/{})"` header with `items.len()` and `Inventory::MAX_ITEMS`.
   - For each slot index `0..Inventory::MAX_ITEMS`:
     - Use `ui.push_id(format!("slot_{}", slot_idx), |ui| { ... })` for each
       slot widget — **mandatory per egui ID rules**.
     - If `slot_idx < items.len()`: render slot as a filled button showing item
       ID (plain number, since item name lookup requires `GameContent` resource
       which can be passed as an additional parameter — see note below).
     - If `slot_idx >= items.len()`: render an empty dimmed label `"[empty]"`.
     - Highlight the selected slot with `egui::Color32::YELLOW` background when
       `Some(slot_idx) == selected_slot`.

**Note on item name display:** To display item names, `inventory_ui_system`
needs access to `Res<crate::application::resources::GameContent>`. Add it to
the system signature. Use `game_content.db().items.get_item(item_id)` to look
up names. If item lookup fails, display `"Item #{item_id}"` as a fallback.
Pass `game_content` through to `render_character_panel`.

#### 3.2 Add `pub mod inventory_ui;` to `src/game/systems/mod.rs`

**File:** `src/game/systems/mod.rs`

Add `pub mod inventory_ui;` in alphabetical order (between `inn_ui` and
`input`).

#### 3.3 Register `InventoryPlugin` in `src/bin/antares.rs`

**File:** `src/bin/antares.rs`

In `AntaresPlugin::build`, add:

```rust
app.add_plugins(antares::game::systems::inventory_ui::InventoryPlugin);
```

Place it after `InnUiPlugin` and before `MenuPlugin` (alphabetical / logical
order).

#### 3.4 Testing Requirements for Phase 3

- `test_inventory_ui_plugin_builds` — build a minimal `App` with
  `InventoryPlugin`, assert it does not panic. Pattern mirrors
  `test_inn_ui_plugin_builds` in `src/game/systems/inn_ui.rs`. **File:**
  `src/game/systems/inventory_ui.rs`.
- `test_inventory_navigation_state_default` — assert
  `InventoryNavigationState::default()` has `selected_slot_index = None` and
  `focus_on_panel = 0`. **File:** `src/game/systems/inventory_ui.rs`.
- `test_inventory_action_button_variants` — assert `DropItemAction` and
  `TransferItemAction` can be constructed without panic. (These are defined in
  Phase 4 but tested here since they are registered in `InventoryPlugin::build`.)
  **File:** `src/game/systems/inventory_ui.rs`.
- `test_render_character_panel_does_not_panic_empty_inventory` — construct a
  minimal `egui::Context`, create an `egui::Ui`, call `render_character_panel`
  with a character whose inventory is empty; assert no panic. **File:**
  `src/game/systems/inventory_ui.rs`.
- `test_render_character_panel_does_not_panic_full_inventory` — same as above
  but with `Inventory::MAX_ITEMS` slots filled. **File:**
  `src/game/systems/inventory_ui.rs`.

#### 3.5 Deliverables

- [ ] `src/game/systems/inventory_ui.rs` created with SPDX header, `InventoryPlugin`,
      `InventoryNavigationState`, `inventory_input_system`, `inventory_ui_system`,
      `inventory_action_system` (stub for Phase 4), `render_character_panel`
- [ ] `src/game/systems/mod.rs` updated with `pub mod inventory_ui;`
- [ ] `src/bin/antares.rs` registers `InventoryPlugin`
- [ ] All five tests from Section 3.4 passing

#### 3.6 Success Criteria

- Pressing `"I"` during exploration renders a visible egui `CentralPanel`
  showing character name, gold, and inventory slots for each open panel
- Focused panel has a yellow border; unfocused panels have a dark gray border
- Tab cycles focus through party members and opens additional panels (up to
  `PARTY_MAX_SIZE`)
- `Escape` or `"I"` closes the overlay and returns to prior mode
- HUD (Bevy native UI) remains visible and functional while inventory is open
  because egui renders in a separate pass that does not conflict with Bevy UI
- No egui widget ID collisions: every slot uses `push_id` scoped by
  `(party_index, slot_index)`

---

### Phase 4: Item Actions — Drop and Transfer

Wire the Drop and Transfer actions so that a selected item can be discarded from
the inventory or moved to another character.

#### 4.1 Define `DropItemAction` and `TransferItemAction` message types

**File:** `src/game/systems/inventory_ui.rs`

Add the following message types (using `#[derive(Message)]` matching the pattern
in `src/game/systems/inn_ui.rs`):

```rust
/// Emitted when the player confirms dropping a selected item.
#[derive(Message)]
pub struct DropItemAction {
    /// Party index of the character dropping the item (0..PARTY_MAX_SIZE)
    pub party_index: usize,
    /// Slot index within that character's inventory (0..Inventory::MAX_ITEMS)
    pub slot_index: usize,
}

/// Emitted when the player transfers an item to another character.
#[derive(Message)]
pub struct TransferItemAction {
    /// Party index of the source character
    pub from_party_index: usize,
    /// Slot index within the source character's inventory
    pub from_slot_index: usize,
    /// Party index of the target character (must differ from from_party_index)
    pub to_party_index: usize,
}
```

Both types are already registered with `add_message` in `InventoryPlugin::build`
from Phase 3 (they were stubs; now provide the full definitions).

#### 4.2 Extend `inventory_ui_system` to render item action buttons

**File:** `src/game/systems/inventory_ui.rs`

When `inv_state.selected_slot` is `Some(slot_idx)` and the focused character's
inventory has an item at that index, render an action row beneath the slot
listing in `render_character_panel`. The action row contains:

- A **"Drop"** button: when clicked, write a `DropItemAction { party_index,
slot_index: slot_idx }` via `MessageWriter<DropItemAction>`.
- For each other party member index `j` in `inv_state.open_panels` where `j !=
focused_index`: a **"Give to {name}"** button that writes
  `TransferItemAction { from_party_index: focused_index, from_slot_index:
slot_idx, to_party_index: j }`.

Pass `MessageWriter<DropItemAction>` and `MessageWriter<TransferItemAction>` as
additional parameters to `inventory_ui_system`. Use `ui.push_id("actions", |ui|
{ ... })` for the action row to prevent ID collisions.

Update `render_character_panel` to accept optional writer callbacks. Use Rust
closures or restructure the render function to return an `Option<PanelAction>`
enum that the calling system dispatches. The `PanelAction` enum approach keeps
the render helper free of `MessageWriter` generics:

```rust
enum PanelAction {
    Drop { party_index: usize, slot_index: usize },
    Transfer { from_party_index: usize, from_slot_index: usize, to_party_index: usize },
}
```

`render_character_panel` returns `Option<PanelAction>`. The calling system
(`inventory_ui_system`) matches on the return value and writes the appropriate
message.

#### 4.3 Implement `inventory_action_system`

**File:** `src/game/systems/inventory_ui.rs`

`inventory_action_system` handles both `DropItemAction` and `TransferItemAction`
messages in a single system.

System signature:

```rust
fn inventory_action_system(
    mut global_state: ResMut<GlobalState>,
    mut drop_reader: MessageReader<DropItemAction>,
    mut transfer_reader: MessageReader<TransferItemAction>,
)
```

**`DropItemAction` handling:**

1. For each `DropItemAction { party_index, slot_index }` in `drop_reader`:
2. Bounds-check: `party_index < party.members.len()`.
3. Bounds-check: `slot_index < party.members[party_index].inventory.items.len()`.
4. Call `party.members[party_index].inventory.remove_item(slot_index)`. This
   returns `Option<InventorySlot>`; log the dropped item if `Some`.
5. If mode is `GameMode::Inventory(inv_state)`, set
   `inv_state.selected_slot = None`.
6. Log: `info!("Dropped item from party[{}] slot {}", party_index, slot_index)`.
7. No world-entity spawn: dropped items disappear in Phase 4 (world-drop
   rendering is future work).

**`TransferItemAction` handling:**

1. For each `TransferItemAction { from_party_index, from_slot_index,
to_party_index }` in `transfer_reader`:
2. Bounds-check all three indices against `party.members.len()` and
   `inventory.items.len()`.
3. Check `party.members[to_party_index].inventory.has_space()`; if full, log a
   warning (`warn!("Transfer failed: target party[{}] inventory is full",
to_party_index)`) and return without mutation.
4. Remove `InventorySlot` from source: `party.members[from_party_index]
.inventory.remove_item(from_slot_index)`. Store the returned
   `Option<InventorySlot>`.
5. If `None`, log warning and return (item was already removed by a concurrent
   message — defensive check).
6. Call `party.members[to_party_index].inventory.add_item(slot.item_id,
slot.charges)`. This returns `Result<(), CharacterError>`; log any error and
   **if error**, put the item back into the source inventory to avoid item loss:
   `party.members[from_party_index].inventory.add_item(slot.item_id,
slot.charges).ok()`.
7. If `Ok(())`, log success and reset `selected_slot = None` in
   `InventoryState`.

**Note on mutable borrow split:** Rust forbids borrowing two elements of
`party.members` mutably at the same time via `[]`. Use index arithmetic with a
split or copy the `InventorySlot` value before obtaining the second mutable
borrow. The pattern: remove (returns owned `InventorySlot`), then add to target
— this works because the first borrow is released after `remove_item`.

#### 4.4 Testing Requirements for Phase 4

- `test_drop_item_action_removes_from_inventory` — create a `GameState` in
  `Inventory` mode with a character holding one item; emit `DropItemAction`;
  run one frame; assert `inventory.items` is empty and `selected_slot = None`.
  **File:** `src/game/systems/inventory_ui.rs`.
- `test_drop_item_action_invalid_index_no_panic` — emit `DropItemAction` with
  `slot_index = 99` (out of bounds); assert no panic and inventory unchanged.
  **File:** `src/game/systems/inventory_ui.rs`.
- `test_drop_item_invalid_party_index_no_panic` — emit `DropItemAction` with
  `party_index = 99`; assert no panic. **File:**
  `src/game/systems/inventory_ui.rs`.
- `test_transfer_item_character_to_character_success` — source has one item,
  target has space; emit `TransferItemAction`; assert item moved, source empty,
  target has one item, gold unchanged. **File:**
  `src/game/systems/inventory_ui.rs`.
- `test_transfer_item_target_inventory_full` — target at `Inventory::MAX_ITEMS`
  items; emit `TransferItemAction`; assert source inventory unchanged and target
  still full. **File:** `src/game/systems/inventory_ui.rs`.
- `test_transfer_item_no_item_at_source_slot` — emit `TransferItemAction` with
  `from_slot_index` beyond the source inventory length; assert no panic and no
  mutation. **File:** `src/game/systems/inventory_ui.rs`.
- `test_panel_action_drop_variant` — assert `PanelAction::Drop { party_index:
0, slot_index: 1 }` has the correct field values. **File:**
  `src/game/systems/inventory_ui.rs`.
- `test_panel_action_transfer_variant` — assert
  `PanelAction::Transfer { from_party_index: 0, from_slot_index: 0,
to_party_index: 1 }` has the correct field values. **File:**
  `src/game/systems/inventory_ui.rs`.

#### 4.5 Deliverables

- [ ] `DropItemAction` and `TransferItemAction` fully defined (not stubs)
- [ ] `PanelAction` enum defined and returned from `render_character_panel`
- [ ] Action buttons rendered in `inventory_ui_system` for Drop and Transfer
- [ ] `inventory_action_system` implemented: handles both action types with
      bounds checks, defensive item-loss guard, and proper logging
- [ ] All eight tests from Section 4.4 passing

#### 4.6 Success Criteria

- Selecting a slot and pressing the Drop button removes the item from the
  character's inventory and updates the panel to show the slot as `[empty]`
- Selecting a slot and pressing a Give button moves the item to the target
  character, provided their inventory has space
- No item loss occurs if `add_item` fails after `remove_item` (rollback logic)
- All pre-existing `test_sell_item_*` and `test_buy_item_*` domain tests still
  pass (no `transactions.rs` changes)

---

### Phase 5: Configuration, Data, and Documentation

Finalize configurable key bindings in RON data files and update
`docs/explanation/implementations.md`.

#### 5.1 Update `campaigns/tutorial/config.ron`

**File:** `campaigns/tutorial/config.ron`

Add `inventory: ["I"],` to the `controls: ControlsConfig(...)` block after the
`menu: ["Escape"],` line:

```
controls: ControlsConfig(
    move_forward: ["W", "ArrowUp"],
    move_back: ["S", "ArrowDown"],
    turn_left: ["A", "ArrowLeft"],
    turn_right: ["D", "ArrowRight"],
    interact: ["Space", "E"],
    menu: ["Escape"],
    inventory: ["I"],
    movement_cooldown: 0.2,
),
```

The `#[serde(default)]` annotation already ensures existing files without this
field deserialize correctly; this update makes the intent explicit and serves as
the canonical example.

#### 5.2 Check for additional campaign config files

**Action:** Search for any other `config.ron` files in the `campaigns/`
directory tree: `find campaigns/ -name "config.ron"`. At time of writing, only
`campaigns/tutorial/config.ron` exists (confirmed). If additional files are
discovered, apply the same `inventory: ["I"]` addition to each.

Also update `campaigns/config.template.ron` with the same `inventory: ["I"]`
addition so future campaigns created from the template include the field.

#### 5.3 Update `docs/explanation/implementations.md`

**File:** `docs/explanation/implementations.md`

Append a new top-level section `## ECS Inventory View` at the end of the file.
Follow the format of existing entries. Include:

- `### Overview` — what was built and why (2–4 sentences).
- `### Components Implemented` — table with columns `File`, `Type of Change`,
  `Description`. One row per file in the Candidate Files Summary below.
- `### Test Counts` — table with columns `File`, `New Tests`, `Test Names (brief)`.
- `### Architecture Compliance Notes` — bullet list confirming:
  - Surface ECS only: `Inventory`, `InventorySlot`, `Equipment` gained
    `#[derive(Component)]` with no field changes.
  - `GameMode` extended with `Inventory(InventoryState)` variant following
    existing `InnManagement(InnManagementState)` precedent.
  - UI uses `bevy_egui` matching `InnUiPlugin` pattern; not Bevy native UI.
  - `ControlsConfig.inventory` uses `#[serde(default)]`; existing RON files
    deserialize without modification.
  - `transactions.rs` not modified; item drops discard items without
    domain-layer involvement in Phase 4.
  - All `PARTY_MAX_SIZE` and `Inventory::MAX_ITEMS` constants used; no magic
    numbers.
  - SPDX headers on all new `.rs` files.

#### 5.4 Testing Requirements for Phase 5

- `test_tutorial_config_deserializes_with_inventory_key` — load
  `campaigns/tutorial/config.ron` as a `GameConfig` via
  `GameConfig::load_or_default`; assert
  `config.controls.inventory == vec!["I".to_string()]`. **File:**
  `src/sdk/game_config.rs` (tests block, after existing `test_load_valid_config_file`
  test — follow that test's pattern exactly).
- `test_controls_config_ron_roundtrip_includes_inventory` — serialize a
  `ControlsConfig` with `inventory: vec!["I".to_string(), "F1".to_string()]`
  to RON string, deserialize back, assert round-trip fidelity for the `inventory`
  field. **File:** `src/sdk/game_config.rs`.

#### 5.5 Deliverables

- [ ] `campaigns/tutorial/config.ron` updated with `inventory: ["I"]`
- [ ] `campaigns/config.template.ron` updated with `inventory: ["I"]`
- [ ] `docs/explanation/implementations.md` has `## ECS Inventory View` section
      appended
- [ ] All two tests from Section 5.4 passing

#### 5.6 Success Criteria

- `cargo fmt --all` produces no output
- `cargo check --all-targets --all-features` reports zero errors
- `cargo clippy --all-targets --all-features -- -D warnings` reports zero warnings
- `cargo nextest run --all-features` passes all tests including all pre-existing
  tests
- `"I"` key is present in `campaigns/tutorial/config.ron`

---

## Candidate Files Summary

### New Files

| File                                 | Phase | Purpose                                                                                                               |
| ------------------------------------ | ----- | --------------------------------------------------------------------------------------------------------------------- |
| `src/application/inventory_state.rs` | 2     | `InventoryState` struct and all navigation helpers                                                                    |
| `src/game/components/inventory.rs`   | 1     | `CharacterEntity` component, `PartyEntities` resource                                                                 |
| `src/game/systems/inventory_ui.rs`   | 3, 4  | `InventoryPlugin`, `InventoryNavigationState`, all inventory UI and action systems, `PanelAction` enum, message types |

### Modified Files

| File                                  | Phase | Change                                                                                                                |
| ------------------------------------- | ----- | --------------------------------------------------------------------------------------------------------------------- |
| `src/domain/character.rs`             | 1     | Add `Component` to `Inventory`, `InventorySlot`, `Equipment` derives                                                  |
| `src/game/components/mod.rs`          | 1     | Add `pub mod inventory;` and re-export `CharacterEntity`, `PartyEntities`                                             |
| `src/game/systems/hud.rs`             | 1     | Add `setup_party_entities` startup system to `HudPlugin`; insert `PartyEntities` resource                             |
| `src/application/mod.rs`              | 2     | Add `pub mod inventory_state;`; add `GameMode::Inventory(InventoryState)` variant; add `GameState::enter_inventory()` |
| `src/game/systems/input.rs`           | 2     | Add `GameAction::Inventory`; wire `KeyMap`; open/close inventory in `handle_input`; extend input-block guard          |
| `src/sdk/game_config.rs`              | 2, 5  | Add `inventory: Vec<String>` to `ControlsConfig` with `#[serde(default)]`; update `Default`; update `validate()`      |
| `src/game/systems/mod.rs`             | 3     | Add `pub mod inventory_ui;`                                                                                           |
| `src/bin/antares.rs`                  | 3     | Register `InventoryPlugin`                                                                                            |
| `campaigns/tutorial/config.ron`       | 5     | Add `inventory: ["I"]` to controls block                                                                              |
| `campaigns/config.template.ron`       | 5     | Add `inventory: ["I"]` to controls block                                                                              |
| `docs/explanation/implementations.md` | 5     | Append `## ECS Inventory View` summary section                                                                        |

---

## Architecture Constraints

All code written in this plan must comply with `AGENTS.md`. The following
constraints are especially relevant:

1. **Surface ECS only.** `Inventory`, `InventorySlot`, and `Equipment` gain
   `#[derive(Component)]` but no field changes and no method changes.
   `buy_item`, `sell_item`, and `consume_service` in
   `src/domain/transactions.rs` are not modified.

2. **Domain layer stays Bevy-free in logic.** `Component` is a marker trait
   with no runtime overhead; adding it does not introduce any Bevy execution
   dependency into the domain layer's pure functions or existing tests.

3. **egui for inventory UI, not Bevy native UI.** The inventory uses
   `bevy_egui` matching the `InnUiPlugin` pattern. The `MenuPlugin` uses Bevy
   native UI (`Node`, `Button`) — do not mix the two patterns in the inventory
   system.

4. **egui ID hygiene is mandatory.** Every loop body rendering widgets in egui
   must use `ui.push_id(unique_key, ...)`. Every `ScrollArea` must have
   `.id_salt("unique_string")`. Every `ComboBox` must use `from_id_salt`. The
   inventory panel loop uses `push_id(party_index, ...)` and each slot row uses
   `push_id(format!("slot_{}", slot_idx), ...)`. No exceptions.

5. **`serde(default)` for all new config fields.** `ControlsConfig.inventory`
   must have `#[serde(default = "default_inventory_keys")]` so existing RON
   campaign config files without the field continue to deserialize without
   errors.

6. **No `unwrap()` without justification.** All fallible operations use
   `Result<T, E>` with `?` or explicit `match`/`if let`. Bounds checks precede
   all `party.members[index]` accesses. Item-loss rollback is implemented in
   `inventory_action_system`.

7. **SPDX headers.** Every new `.rs` file begins with:

   ```
   // SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
   // SPDX-License-Identifier: Apache-2.0
   ```

8. **Type aliases and constants.** Use `ItemId` (not `u32`), `PARTY_MAX_SIZE`
   (not `6`), `Inventory::MAX_ITEMS` (not `6`), `Equipment::MAX_EQUIPPED` (not
   `6`) throughout. No magic numbers.

9. **`GameMode::Inventory` serialization.** `GameMode` already derives
   `Serialize, Deserialize` (confirmed in `src/application/mod.rs`).
   `InventoryState` must also derive `Serialize, Deserialize` so the full
   `GameMode` enum remains serializable (required for save/load).

10. **`Box<GameMode>` for `previous_mode`.** `InventoryState.previous_mode`
    must be `Box<GameMode>` to break the recursive type size dependency, exactly
    as `MenuState.previous_mode` does in `src/application/menu.rs`.

11. **No new modules beyond this plan.** Do not create `src/utils/`,
    `src/helpers/`, or any other module not listed in the Candidate Files
    Summary. All new code goes in the files listed.

12. **Test naming convention.** All test functions must follow the pattern
    `test_{function_or_feature}_{condition}_{expected_result}`.

---

## Quality Gate Checklist

Run the following commands in order after completing each phase. All must pass
before proceeding to the next phase.

```
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Expected results for each command:**

| Command                                                    | Expected output                         |
| ---------------------------------------------------------- | --------------------------------------- |
| `cargo fmt --all`                                          | No output (all files already formatted) |
| `cargo check --all-targets --all-features`                 | `Finished` with 0 errors                |
| `cargo clippy --all-targets --all-features -- -D warnings` | `Finished` with 0 warnings              |
| `cargo nextest run --all-features`                         | All tests pass; 0 failures              |
