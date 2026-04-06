<!--
SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
SPDX-License-Identifier: Apache-2.0
-->

# SDK Fixes Implementation Plan

## Overview

Four UI/data bugs in the Campaign Builder SDK need to be resolved. Two fixes
are SDK-only layout or wiring issues; two require coordinated changes in both
the game-engine data model and the SDK editor. The plan is ordered from
simplest (pure UI rearrangement) to most complex (new game-engine fields with
matching loot mechanics), so each phase can be shipped and tested independently.

---

## Current State Analysis

### Existing Infrastructure

| Area                   | File(s)                                              | Notes                                                                                                                                                                                                                  |
| ---------------------- | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Map event data model   | `src/domain/world/types.rs`                          | `MapEvent::Container` has `id`, `name`, `description`, `items` – **no `gold` or `gems`**                                                                                                                               |
| Container event result | `src/domain/world/events.rs`                         | `EventResult::EnterContainer` carries `container_event_id`, `container_name`, `items` – **no gold/gems**                                                                                                               |
| Container loot UI      | `src/game/systems/container_inventory_ui.rs`         | Renders item list only; no currency row                                                                                                                                                                                |
| Stock template domain  | `src/domain/world/npc_runtime.rs`                    | `MerchantStockTemplate` has `id`, `entries`, magic fields – **no `description`**                                                                                                                                       |
| SDK map editor         | `sdk/campaign_builder/src/map_editor.rs`             | `EventEditorState` container branch has items UI; **no gold/gems fields**                                                                                                                                              |
| SDK map inspector      | `sdk/campaign_builder/src/map_editor.rs`             | `show_inspector_panel` renders the Event Editor at the **bottom** of the column, below Visual Properties, Terrain Settings, and Preset Palette                                                                         |
| SDK furniture editor   | `sdk/campaign_builder/src/furniture_editor.rs`       | `show_form` has Save and Cancel buttons only at the **bottom inside** a `ScrollArea`; **no Back button at the top**                                                                                                    |
| SDK stock templates    | `sdk/campaign_builder/src/stock_templates_editor.rs` | `StockTemplateEditBuffer::from_template` hardcodes `description: String::new()` with a comment "templates have no description field in the domain type"; `to_template` omits `description` from the constructed struct |

### Identified Issues

1. **Container Gold/Gems missing** – `MapEvent::Container` has no `gold` or
   `gems` fields; the SDK Container event editor has no UI for them; the
   game-engine container loot handler does not award currency when the player
   takes all.

2. **Event Editor placement wrong** – The Event Editor panel renders at the
   very bottom of the right-column inspector, below Visual Properties, Terrain
   Settings, and Preset Palette. It must appear immediately below the Event
   Details summary, not at the foot of the column.

3. **Furniture editor Back to List button missing** – `show_form` exposes
   only "✅ Save Definition" and "❌ Cancel" at the bottom of a `ScrollArea`.
   There is no navigation button at the top of the form, so authors must scroll
   to the bottom to escape the edit view.

4. **Stock Template description not loaded** – The `description` field exists
   in `StockTemplateEditBuffer` and is shown in the SDK edit view, but
   `from_template` always resets it to an empty string, and `to_template` never
   writes it back to the domain struct, so edits are silently discarded on every
   round-trip.

---

## Implementation Phases

---

### Phase 1: Pure SDK Layout Fixes

Fix the two bugs that require no game-engine data model changes: the missing
Back button on the Furniture form and the misplaced Event Editor in the
inspector.

#### 1.1 Furniture Editor – Add Back to List Button

**File:** `sdk/campaign_builder/src/furniture_editor.rs`

**Function:** `show_form` (L758)

Inside `show_form`, immediately after the heading and separator (before the
`ScrollArea` opens), add a `ui.horizontal_wrapped` row containing a
`"◀ Back to List"` button. When clicked, set `self.mode =
FurnitureEditorMode::List` and call `ui.ctx().request_repaint()`. Do **not**
remove the existing Cancel button at the bottom of the form; authors who have
scrolled to the bottom still need it. The top button is purely for quick
navigation.

The addition looks conceptually like:

```
ui.heading(…);
ui.separator();

// NEW: top-of-form navigation
ui.horizontal_wrapped(|ui| {
    if ui.button("◀ Back to List").clicked() {
        self.mode = FurnitureEditorMode::List;
        ui.ctx().request_repaint();
    }
});
ui.separator();

egui::ScrollArea::vertical()  // existing ScrollArea unchanged
```

#### 1.2 Map Editor Inspector – Move Event Editor to Event Details Position

**File:** `sdk/campaign_builder/src/map_editor.rs`

**Function:** `show_inspector_panel` (L4061)

The Event Editor block (currently rendered at ~L4581 after the visual/terrain
sections) must be moved to appear **inside** the selected-tile `ui.group`, right
after the horizontal row of `"✏️ Edit Event"` / `"🗑 Remove Event"` buttons
and before the `ui.separator()` that precedes Visual Properties. The final
layout order of the inspector column must be:

1. Map ID / Size / Name group
2. Selected tile info group
   - Position / Terrain / Wall info
   - NPC info + buttons (if present)
   - **Event Details summary + Edit/Remove buttons (if event present)**
   - **Event Editor panel (if `PlaceEvent` tool active)** ← moved here
3. Visual Properties group
4. Terrain-Specific Settings group
5. Visual Preset Palette group
6. NPC Placement editor (if `PlaceNpc` tool active) — unchanged position
7. Statistics group
8. Validation Errors group

Remove the existing `if matches!(editor.current_tool, EditorTool::PlaceEvent)`
block from its current location at the bottom and re-render it at the new
position described above. The inner `ui.group` wrapper and heading `"Event
Editor"` are preserved.

#### 1.3 Character Editor – Replace Starting Spells Collapsible Header with Flat Section Heading

**File:** `sdk/campaign_builder/src/characters_editor.rs`

**Problem:** `show_starting_spells_editor` wraps all its content in an
`egui::CollapsingHeader` (a collapsible dropdown), while every other section
of the Character Editor form uses a flat `ui.heading(...)` followed by
always-visible content. The collapsible is visually inconsistent and out of
place.

**Call site fix** — `show_character_form` (currently near L2038):

Replace the bare call:

```
ui.add_space(10.0);
self.show_starting_spells_editor(ui, spells, classes);
```

with the same heading-first pattern used by every adjacent section:

```
ui.add_space(10.0);
ui.heading("Starting Spells");

self.show_starting_spells_editor(ui, spells, classes);
```

**Function fix** — `show_starting_spells_editor`:

Remove the outer `egui::CollapsingHeader::new("📚 Starting Spells")` /
`.id_salt("starting_spells_header")` / `.default_open(false)` / `.show(ui,
|ui| { ... })` wrapper entirely. The function body (non-caster warning,
autocomplete picker, `ScrollArea` grid) becomes the direct content of the
function with no additional nesting level. The `id_salt` on the
`CollapsingHeader` is deleted; the `ScrollArea` and `Grid` already carry their
own `id_salt` / `id` values and are unaffected.

The resulting function structure is:

```
fn show_starting_spells_editor(
    &mut self,
    ui: &mut egui::Ui,
    available_spells: &[Spell],
    classes: &[ClassDefinition],
) {
    // Non-caster warning (unchanged logic)
    …

    // "Add Spell" autocomplete selector (unchanged)
    …

    ui.add_space(4.0);

    if self.buffer.starting_spells.is_empty() {
        ui.label(egui::RichText::new("No starting spells defined.").italics());
    } else {
        // ScrollArea + Grid (unchanged, id_salt values preserved)
        …
    }
}
```

No changes to `CharacterEditBuffer`, serialization, validation, or any other
file are required.

#### 1.4 Testing Requirements

- Add a unit test
  `test_event_editor_renders_before_visual_properties_section` in
  `map_editor.rs` `mod tests` verifying that `show_inspector_panel` does not
  crash and that the event editor state is still accessible after the
  refactor.
- Add a unit test `test_furniture_show_form_back_button_returns_to_list` in
  `furniture_editor_tests.rs` constructing a `FurnitureEditorState` in Edit
  mode, simulating a click on Back, and asserting the mode returns to `List`.
- Verify existing `characters_editor.rs` tests for `show_starting_spells_editor`
  (`test_non_caster_warning_detection`, `test_starting_spells_no_duplicate`,
  `test_starting_spells_remove_entry`, etc.) continue to pass unchanged — the
  logic is untouched so no test changes are needed.
- Run the full SDK test suite: `cargo nextest run --all-features -p
campaign_builder`.

#### 1.5 Deliverables

- [ ] `◀ Back to List` button added to top of furniture `show_form`
- [ ] Event Editor moved directly below Event Details in `show_inspector_panel`
- [ ] `egui::CollapsingHeader` removed from `show_starting_spells_editor`
- [ ] `ui.heading("Starting Spells")` added at the call site in `show_character_form`
- [ ] Unit tests added and passing
- [ ] `cargo fmt`, `cargo check`, `cargo clippy -D warnings` all clean

#### 1.6 Success Criteria

- Opening Furniture → Edit Furniture shows a Back to List button at the top
  of the form without scrolling.
- Clicking Back to List returns to the furniture list.
- Opening Maps → Edit Map → selecting a tile with an event and activating the
  Edit Event button shows the Event Editor immediately below the Event
  Details, not at the page footer.
- Opening Characters → Edit Character shows "Starting Spells" as a plain
  section heading (matching "Starting Equipment", "Starting Items",
  "Description", etc.) with the spell table and Add Spell picker always
  visible — no collapsible dropdown.

---

### Phase 2: Stock Template Description – Data Model + SDK Wire-up

Fix the round-trip bug that silently discards the `description` field of a
stock template when editing and saving.

#### 2.1 Game Engine – Add `description` to `MerchantStockTemplate`

**File:** `src/domain/world/npc_runtime.rs`

Add a `description` field to `MerchantStockTemplate` with `#[serde(default)]`
so existing RON files that omit `description` deserialise without error:

```rust
/// Optional human-readable description shown in the SDK editor.
///
/// Not used at runtime; purely an authoring aid.
#[serde(default)]
pub description: String,
```

All existing construction sites in the codebase that use struct literal syntax
must be updated to include `description: String::new()` (or the appropriate
value) to satisfy the compiler. Search for `MerchantStockTemplate {` across
`src/` and `sdk/` and add the field everywhere.

#### 2.2 SDK – Fix `from_template` to Load `description`

**File:** `sdk/campaign_builder/src/stock_templates_editor.rs`

**Function:** `StockTemplateEditBuffer::from_template` (L133)

Replace:

```rust
description: String::new(), // templates have no description field in the domain type
```

with:

```rust
description: template.description.clone(),
```

Remove the stale comment.

#### 2.3 SDK – Fix `to_template` to Persist `description`

**Function:** `StockTemplateEditBuffer::to_template` (L168)

In the `Ok(MerchantStockTemplate { … })` construction at the end of the
function, add:

```rust
description: self.description.clone(),
```

#### 2.4 Testing Requirements

- Extend `test_from_template_round_trips` in `stock_templates_editor.rs` to
  assert that a template with a non-empty description round-trips correctly
  through `from_template` → mutate buffer description → `to_template`.
- Add `test_stock_template_description_is_persisted` verifying that calling
  `from_template` on a template whose `description` is `"General goods shop"`
  produces a buffer whose `description` equals `"General goods shop"`.
- Add `test_stock_template_description_to_template` verifying that
  `to_template` includes the buffer's description in the returned struct.
- Update any existing tests that construct `MerchantStockTemplate` with struct
  literal syntax to include the new `description` field.
- Run `cargo nextest run --all-features`.

#### 2.5 Deliverables

- [ ] `description: String` field added to `MerchantStockTemplate` with
      `#[serde(default)]`
- [ ] All struct literal construction sites updated
- [ ] `from_template` reads `template.description`
- [ ] `to_template` writes `self.description` into the returned struct
- [ ] RON serialisation round-trip test passes (existing
      `test_load_from_file_round_trip` must continue to pass)
- [ ] New unit tests added and passing
- [ ] `cargo fmt`, `cargo check`, `cargo clippy -D warnings` all clean

#### 2.6 Success Criteria

- Opening Stock Templates → Edit Template on a template that already has a
  description shows the description pre-populated in the editor.
- Editing the description and saving round-trips the value correctly into the
  `.ron` data file.
- Loading the saved `.ron` file back into the editor shows the same
  description without loss.

---

### Phase 3: Container Gold and Gems – Full Stack Implementation

This is the largest change: new fields on the game-engine data model, updated
loot dispatch, container UI changes in the game, and matching editor UI in the
SDK.

#### 3.1 Game Engine – Add `gold` and `gems` to `MapEvent::Container`

**File:** `src/domain/world/types.rs`

Add two new fields to the `Container` variant of `MapEvent`. Both use
`#[serde(default)]` so all existing map RON files remain valid:

```rust
/// Gold coins placed in the container by the campaign author.
///
/// Added to the party's shared gold when the player takes all or
/// takes the currency individually from the container UI.
#[serde(default)]
gold: u32,

/// Gems placed in the container by the campaign author.
///
/// Added to the party's shared gems when taken.
#[serde(default)]
gems: u32,
```

Add `gold` and `gems` to every existing match arm and construction site that
destructs or constructs `MapEvent::Container { … }` throughout `src/`.

#### 3.2 Game Engine – Propagate Gold/Gems Through `EventResult`

**File:** `src/domain/world/events.rs`

Extend `EventResult::EnterContainer` with two new fields:

```rust
EnterContainer {
    container_event_id: String,
    container_name: String,
    items: Vec<crate::domain::character::InventorySlot>,
    /// Gold available to take from the container.
    gold: u32,
    /// Gems available to take from the container.
    gems: u32,
},
```

Update `trigger_event` at the `MapEvent::Container` arm to propagate the new
fields:

```rust
MapEvent::Container { id, name, items, gold, gems, .. } => {
    EventResult::EnterContainer {
        container_event_id: id.clone(),
        container_name: name.clone(),
        items: items.clone(),
        gold: *gold,
        gems: *gems,
    }
}
```

Update every match arm in `src/` that destructs `EventResult::EnterContainer`
to include the new fields (use `..` where the values are not yet consumed to
satisfy the compiler with minimal churn).

#### 3.3 Game Engine – Container Inventory State

**File:** `src/application/container_inventory_state.rs`

Locate `ContainerInventoryState` (or equivalent). Add:

```rust
/// Gold available in the open container.
pub gold: u32,
/// Gems available in the open container.
pub gems: u32,
```

In the code that populates `ContainerInventoryState` from
`EventResult::EnterContainer`, read the new `gold` and `gems` fields.

When the party executes "Take All", add the container's `gold` to
`GameState.party.gold` and `gems` to `GameState.party.gems`, then zero out
the container fields. When individual currency take is added to the UI, apply
the same write-back pattern used by item takes.

Write back the updated `gold`/`gems` values to `MapEvent::Container` on
container close using the same write-back path used for `items` today (search
for `write_back` or the existing container close handler in
`container_inventory_ui.rs`).

#### 3.4 Game Engine – Container Inventory UI

**File:** `src/game/systems/container_inventory_ui.rs`

In the container right panel, after the item list and before the action
buttons, add a currency row:

- If `gold > 0` show `💰 Gold: {gold}` with a `[Take Gold]` button.
- If `gems > 0` show `💎 Gems: {gems}` with a `[Take Gems]` button.
- `[Take All]` must also sweep currency (gold + gems) in addition to items.

Follow the existing `TakeItemAction` / `StashItemAction` message pattern.
Introduce `TakeCurrencyAction { gold: u32, gems: u32 }` as a new message type
and a corresponding handler that adds to `party.gold` / `party.gems` and
zeroes the container state fields.

#### 3.5 SDK – Extend `EventEditorState` for Gold and Gems

**File:** `sdk/campaign_builder/src/map_editor.rs`

Add to `EventEditorState` (near the `container_*` fields, ~L2021):

```rust
/// Gold coins in this container (displayed as a text field, parsed as u32).
pub container_gold: String,
/// Gems in this container (displayed as a text field, parsed as u32).
pub container_gems: String,
```

In `Default for EventEditorState`, initialise both to `"0".to_string()`.

#### 3.6 SDK – Wire Gold/Gems in `to_map_event` and `from_map_event`

**Function:** `EventEditorState::to_map_event` (L2177)

In the `EventType::Container` branch, parse `self.container_gold` and
`self.container_gems` as `u32` (default `0` on parse failure) and include them
in the constructed `MapEvent::Container { … }`.

**Function:** `EventEditorState::from_map_event` (L2417)

In the `MapEvent::Container { gold, gems, … }` destructuring arm, set:

```rust
container_gold: gold.to_string(),
container_gems: gems.to_string(),
```

#### 3.7 SDK – Add Gold/Gems UI to `show_event_editor` Container Branch

**Function:** `show_event_editor` (L4858)

Inside `EventType::Container =>` (currently at ~L5700 in the file), add two
numeric text-edit rows between the Container ID row and the item list, following
the same `ui.horizontal` + `TextEdit::singleline` pattern used for other
numeric fields:

```
ui.horizontal(|ui| {
    ui.label("💰 Gold:");
    ui.add(TextEdit::singleline(&mut event_editor.container_gold)
        .id_salt("container_evt_gold")
        .desired_width(80.0)
        .hint_text("0"));
});

ui.horizontal(|ui| {
    ui.label("💎 Gems:");
    ui.add(TextEdit::singleline(&mut event_editor.container_gems)
        .id_salt("container_evt_gems")
        .desired_width(80.0)
        .hint_text("0"));
});
```

Add tooltip hints explaining that these values are taken when the player
opens and empties the container.

#### 3.8 Testing Requirements

**Game engine tests** (`src/domain/world/events.rs` `mod tests`):

- `test_container_event_with_gold_returns_gold_in_result` – verify
  `EventResult::EnterContainer` carries the gold set on the event.
- `test_container_event_with_gems_returns_gems_in_result` – same for gems.
- `test_container_event_zero_currency_default` – verify a container with no
  currency fields (`#[serde(default)]`) returns `gold: 0, gems: 0`.

**Game engine tests** (`src/application/save_game.rs` or equivalent):

- Extend `test_save_load_preserves_container_items_after_partial_take` to
  verify that `gold` and `gems` also survive a save/load round-trip.

**SDK tests** (`sdk/campaign_builder/src/map_editor.rs` `mod tests`):

- `test_event_editor_state_to_container_with_gold_and_gems` – construct an
  `EventEditorState` with `container_gold = "50"`, `container_gems = "3"`,
  call `to_map_event`, assert the result is
  `MapEvent::Container { gold: 50, gems: 3, .. }`.
- `test_event_editor_state_from_container_with_gold_and_gems` – call
  `from_map_event` on a `MapEvent::Container { gold: 100, gems: 5, .. }` and
  assert the buffer strings are `"100"` and `"5"`.
- `test_event_editor_state_container_gold_gems_default_zero` – verify default
  state has `"0"` for both fields.

**SDK tests** (data/test_campaign integration):

- Ensure any test campaign container events in
  `data/test_campaign/data/` RON files that omit `gold`/`gems` still parse
  cleanly under `#[serde(default)]`.

Run `cargo nextest run --all-features` across both the root crate and the SDK
crate after each sub-step.

#### 3.9 Deliverables

- [ ] `gold: u32` and `gems: u32` added to `MapEvent::Container` with
      `#[serde(default)]`
- [ ] `EventResult::EnterContainer` carries `gold` and `gems`
- [ ] `trigger_event` propagates gold/gems from the map event
- [ ] `ContainerInventoryState` tracks gold/gems
- [ ] `TakeCurrencyAction` message + handler implemented
- [ ] `[Take Gold]`, `[Take Gems]` buttons in container UI
- [ ] `[Take All]` sweeps currency as well as items
- [ ] Container close writes back gold/gems to `MapEvent::Container`
- [ ] `EventEditorState` has `container_gold` and `container_gems`
- [ ] `to_map_event` and `from_map_event` handle gold/gems
- [ ] SDK Container event editor shows Gold and Gems input fields
- [ ] All new unit tests passing
- [ ] `cargo fmt`, `cargo check`, `cargo clippy -D warnings` all clean
- [ ] `data/test_campaign` RON files parse cleanly with new fields defaulting

#### 3.10 Success Criteria

- In the SDK, opening an Add Event → Container shows Gold and Gems input
  fields. Saving a container with gold=50, gems=3 round-trips through RON and
  reloads correctly in the editor.
- In-game, opening a container that has gold/gems shows the currency in the
  container right panel with Take Gold / Take Gems buttons.
- Take All also awards gold and gems to the party.
- A container with no gold/gems fields in its RON file loads without error and
  shows 0 for both.

---

## Implementation Order Summary

| Phase | Scope                                                                         | Files Changed                                                                                         |
| ----- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| 1     | Furniture Back to List + Event Editor position + Starting Spells flat heading | `furniture_editor.rs`, `map_editor.rs`, `characters_editor.rs`                                        |
| 2     | Stock Template description round-trip                                         | `npc_runtime.rs`, `stock_templates_editor.rs`                                                         |
| 3     | Container gold/gems (engine + SDK + loot UI)                                  | `types.rs`, `events.rs`, `container_inventory_state.rs`, `container_inventory_ui.rs`, `map_editor.rs` |

Each phase must pass all four quality gates before the next phase begins:

```text
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

## Notes

- All new fields that touch serialised data (`MapEvent::Container`,
  `MerchantStockTemplate`) MUST use `#[serde(default)]` to preserve backward
  compatibility with existing RON campaign files.
- Test data for automated tests lives in `data/test_campaign`, never in
  `campaigns/tutorial`.
- Any new `.ron` test fixture files go under
  `data/test_campaign/data/` as described in `AGENTS.md` Implementation Rule 5.
- No git operations; all commits are left to the user.
