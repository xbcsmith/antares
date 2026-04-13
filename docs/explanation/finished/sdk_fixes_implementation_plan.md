<!--
SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
SPDX-License-Identifier: Apache-2.0
-->

# SDK Fixes Implementation Plan

## Overview

Twelve UI/data bugs and gaps in the Campaign Builder SDK need to be resolved.
Some fixes are SDK-only layout or display issues; others require coordinated
changes in both the game-engine data model and the SDK editor. The plan is
ordered from simplest (pure UI rearrangement) to most complex (new game-engine
fields with matching loot mechanics and NPC dialog generation), so each phase
can be shipped and tested independently.

---

## Current State Analysis

### Existing Infrastructure

| Area                   | File(s)                                              | Notes                                                                                                                                                                                                       |
| ---------------------- | ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Map event data model   | `src/domain/world/types.rs`                          | `MapEvent::Container` has `id`, `name`, `description`, `items` – **no `gold` or `gems`**                                                                                                                    |
| Container event result | `src/domain/world/events.rs`                         | `EventResult::EnterContainer` carries `container_event_id`, `container_name`, `items` – **no gold/gems**                                                                                                    |
| Container loot UI      | `src/game/systems/container_inventory_ui.rs`         | Renders item list only; no currency row                                                                                                                                                                     |
| Stock template domain  | `src/domain/world/npc_runtime.rs`                    | `MerchantStockTemplate` has `id`, `entries`, magic fields – **no `description`**                                                                                                                            |
| SDK map editor         | `sdk/campaign_builder/src/map_editor.rs`             | `EventEditorState` container branch has items UI; **no gold/gems fields**; placed events (Container/Furniture) are not written back to the map RON on save                                                  |
| SDK map inspector      | `sdk/campaign_builder/src/map_editor.rs`             | `show_inspector_panel` renders the Event Editor at the **bottom** of the column, below Visual Properties, Terrain Settings, and Preset Palette                                                              |
| SDK furniture editor   | `sdk/campaign_builder/src/furniture_editor.rs`       | `show_form` has Save and Cancel buttons only at the **bottom inside** a `ScrollArea`; **no Back button at the top**                                                                                         |
| SDK stock templates    | `sdk/campaign_builder/src/stock_templates_editor.rs` | `StockTemplateEditBuffer::from_template` hardcodes `description: String::new()`; `to_template` omits `description`; display view shows id and entries only – **no description**                             |
| SDK characters editor  | `sdk/campaign_builder/src/characters_editor.rs`      | Starting Spells section uses a collapsible header; autocomplete ignores character class for disambiguation; `ScrollArea` is too short (only ~2 spells visible); display view omits starting spells entirely |
| SDK NPC editor         | `sdk/campaign_builder/src/npcs_editor.rs`            | "Create Merchant Dialog" button exists but fires no action; no default dialog is generated for merchants                                                                                                    |
| SDK validation         | `sdk/campaign_builder/src/validation.rs`             | NPC stock template references are resolved against an incomplete registry, causing valid templates to be flagged as unknown                                                                                 |
| SDK config editor      | `sdk/campaign_builder/src/config_editor.rs`          | Key Bindings section is missing the `Spellbook` binding (`[B]`)                                                                                                                                             |

### Identified Issues

1. **Container Gold/Gems missing** – `MapEvent::Container` has no `gold` or
   `gems` fields; the SDK Container event editor has no UI for them; the
   game-engine container loot handler does not award currency when the player
   takes all.

2. **Event Editor placement wrong** – The Event Editor panel renders at the
   very bottom of the right-column inspector, below Visual Properties, Terrain
   Settings, and Preset Palette. It must appear immediately below the Event
   Details summary, not at the foot of the column.

3. **Place Event Container/Furniture not saved to map RON** – Placing a
   Container or Furniture event on the map shows up in the editor session but is
   not written to the map `.ron` file on save. The entity is lost on the next
   load.

4. **Furniture editor Back to List button missing** – `show_form` exposes
   only "✅ Save Definition" and "❌ Cancel" at the bottom of a `ScrollArea`.
   There is no navigation button at the top of the form, so authors must scroll
   to the bottom to escape the edit view.

5. **Stock Template description not loaded or displayed** – The `description`
   field exists in `StockTemplateEditBuffer` and is shown in the SDK edit view,
   but `from_template` always resets it to an empty string, and `to_template`
   never writes it back to the domain struct. The display view also omits
   description entirely.

6. **NPC Editor Create Merchant Dialog does nothing** – When an NPC is
   designated as a Merchant and the "Create Merchant Dialog" button is clicked,
   no dialog is created. The NPC is left without a merchant dialog entry.

7. **Characters Display missing starting spells** – The character detail/display
   view does not list starting spells, making it impossible to review a
   character's spell loadout without entering the edit form.

8. **Characters Starting Spells Auto Complete ignores class** – When a
   Sorcerer character has a spell that shares its name with a Cleric spell (e.g.
   `Awaken`), the autocomplete always resolves it to the Cleric version, silently
   assigning the wrong spell.

9. **Characters Starting Spells area too small** – The `ScrollArea` that
   displays the selected starting spells list only shows ~2 rows before
   requiring a scroll. Authors cannot scan a character's full spell selection
   without excessive scrolling.

10. **Validation flags valid NPC Stock Templates as unknown** – NPC stock
    template references are not resolved against the full stock-template registry,
    so templates that exist in the data are falsely reported as unknown during
    validation.

11. **Config Editor missing Spellbook key binding** – The Key Bindings section
    of the Config Editor does not expose the `Spellbook` action binding (`[B]`),
    making it impossible to reconfigure that key through the SDK.

12. **Character Editor Starting Spells section uses inconsistent collapsible
    header** – The Starting Spells section wraps its content in an
    `egui::CollapsingHeader`, while every other character form section uses a
    flat `ui.heading(…)` with always-visible content.

---

## Implementation Phases

---

### Phase 1: Pure SDK Layout and Display Fixes

Fix all bugs that require no game-engine data model changes: the missing Back
button on the Furniture form, the misplaced Event Editor in the inspector, and
the three Character editor gaps (inconsistent collapsible header, wrong
autocomplete class resolution, undersized spell list, and missing display-view
spells).

---

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

```antares/sdk/campaign_builder/src/furniture_editor.rs#L1-14
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

---

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

---

#### 1.3 Character Editor – Replace Starting Spells Collapsible Header with Flat Section Heading

**File:** `sdk/campaign_builder/src/characters_editor.rs`

**Problem:** `show_starting_spells_editor` wraps all its content in an
`egui::CollapsingHeader` (a collapsible dropdown), while every other section
of the Character Editor form uses a flat `ui.heading(...)` followed by
always-visible content. The collapsible is visually inconsistent and out of
place.

**Call site fix** — `show_character_form` (currently near L2038):

Replace the bare call:

```antares/sdk/campaign_builder/src/characters_editor.rs#L1-4
ui.add_space(10.0);
self.show_starting_spells_editor(ui, spells, classes);
```

with the same heading-first pattern used by every adjacent section:

```antares/sdk/campaign_builder/src/characters_editor.rs#L1-5
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

```antares/sdk/campaign_builder/src/characters_editor.rs#L1-22
fn show_starting_spells_editor(
    &mut self,
    ui: &mut egui::Ui,
    available_spells: &[Spell],
    classes: &[ClassDefinition],
) {
    // Non-caster warning (unchanged logic)
    …

    // "Add Spell" autocomplete selector (unchanged — see §1.4 for class fix)
    …

    ui.add_space(4.0);

    if self.buffer.starting_spells.is_empty() {
        ui.label(egui::RichText::new("No starting spells defined.").italics());
    } else {
        // ScrollArea + Grid (see §1.5 for height increase)
        …
    }
}
```

No changes to `CharacterEditBuffer`, serialization, validation, or any other
file are required for this sub-step alone.

---

#### 1.4 Character Editor – Fix Starting Spells Auto Complete Class Filtering

**File:** `sdk/campaign_builder/src/characters_editor.rs`

**Problem:** The autocomplete picker inside `show_starting_spells_editor` builds
its candidate list from `available_spells` without filtering by the character's
class. When a spell name (e.g. `Awaken`) appears in both the Cleric and
Sorcerer spell lists, the autocomplete always resolves to the Cleric version
because Cleric spells are typically first in the slice. A Sorcerer character
therefore silently receives the Cleric variant.

**Fix:**

Before building the autocomplete candidate list, filter `available_spells` to
only those spells that belong to the class(es) the character can use. Use
`self.buffer.class` (or equivalent field) to determine the character's class,
then look up the matching `ClassDefinition` from the `classes` slice to obtain
that class's allowed spell disciplines.

```antares/sdk/campaign_builder/src/characters_editor.rs#L1-12
// Filter spells to those available to this character's class before
// populating the autocomplete candidate list.
let class_spells: Vec<&Spell> = available_spells
    .iter()
    .filter(|spell| {
        classes
            .iter()
            .find(|c| c.id == self.buffer.class)
            .map(|c| c.spell_disciplines.contains(&spell.discipline))
            .unwrap_or(false)
    })
    .collect();
```

Replace all references to `available_spells` inside the autocomplete block with
`class_spells`. The full `available_spells` slice is still passed to the
function signature unchanged; the filtering is purely local to the picker
widget.

If the character has no class assigned yet, fall back to showing all spells so
the picker remains functional during initial character creation.

---

#### 1.5 Character Editor – Increase Starting Spells Visible Area

**File:** `sdk/campaign_builder/src/characters_editor.rs`

**Function:** `show_starting_spells_editor`

**Problem:** The `ScrollArea` wrapping the selected starting-spells grid is
sized to show only ~2 rows before a scrollbar appears. Authors working with
casters that have 5–8 starting spells must scroll repeatedly to review their
selection.

**Fix:** Set an explicit minimum height on the `ScrollArea` so that at least 5
spell rows are visible without scrolling. Using the existing row height (
approximately `24.0` dp per row plus `4.0` dp spacing), a minimum height of
`145.0` dp shows exactly 5 rows. Prefer `min_scrolled_height` so the area
can still grow when the window is taller.

```antares/sdk/campaign_builder/src/characters_editor.rs#L1-5
egui::ScrollArea::vertical()
    .id_salt("starting_spells_scroll")
    .min_scrolled_height(145.0)   // ← show ~5 rows without scrolling
    .show(ui, |ui| {
```

No other changes to the `Grid` or row rendering are needed.

---

#### 1.6 Characters Display – Add Starting Spells Section

**File:** `sdk/campaign_builder/src/characters_editor.rs`

**Function:** `show_character_display` (or equivalent read-only detail panel)

**Problem:** The character detail/display view shows attributes, equipment, and
starting items but omits starting spells entirely.

**Fix:** After the "Starting Items" section in the display view, add a
"Starting Spells" section. Render it as a flat `ui.heading("Starting Spells")`
followed by either:

- A message `"No starting spells defined."` (italicised) when the list is
  empty, or
- A `egui::Grid` listing each spell's name and discipline (two columns), using
  `id_salt("display_starting_spells_grid")`.

The display view is read-only; no edit controls are needed. Pull spell names by
looking up each `SpellId` in the `available_spells` slice passed to the panel
(or add `available_spells: &[Spell]` to the function signature if it is not
already present).

```antares/sdk/campaign_builder/src/characters_editor.rs#L1-20
// Existing Starting Items section ends above here.

ui.add_space(10.0);
ui.heading("Starting Spells");
ui.separator();

if self.buffer.starting_spells.is_empty() {
    ui.label(egui::RichText::new("No starting spells defined.").italics());
} else {
    egui::Grid::new("display_starting_spells_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for spell_id in &self.buffer.starting_spells {
                let name = available_spells
                    .iter()
                    .find(|s| &s.id == spell_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or("(unknown)");
                ui.label(name);
                ui.end_row();
            }
        });
}
```

---

#### 1.7 Testing Requirements

- Add `test_event_editor_renders_before_visual_properties_section` in
  `map_editor.rs` `mod tests` verifying that `show_inspector_panel` does not
  crash and that the event editor state is still accessible after the refactor.
- Add `test_furniture_show_form_back_button_returns_to_list` in
  `furniture_editor.rs` `mod tests` constructing a `FurnitureEditorState` in
  Edit mode, simulating a click on Back, and asserting the mode returns to
  `List`.
- Add `test_starting_spells_autocomplete_uses_character_class` in
  `characters_editor.rs` `mod tests`: construct a `CharacterEditBuffer` with
  class `Sorcerer`, call the candidate-list builder with a spell list that
  includes both a Cleric `Awaken` and a Sorcerer `Awaken`, and assert only the
  Sorcerer variant is returned.
- Add `test_starting_spells_display_section_shows_spell_names` verifying that
  the display panel renders each spell's name from the provided `available_spells`
  slice.
- Verify existing `characters_editor.rs` tests for `show_starting_spells_editor`
  (`test_non_caster_warning_detection`, `test_starting_spells_no_duplicate`,
  `test_starting_spells_remove_entry`, etc.) continue to pass unchanged — the
  logic is untouched so no test changes are needed beyond the new ones above.
- Run the full SDK test suite: `cargo nextest run --all-features -p campaign_builder`.

#### 1.8 Deliverables

- [ ] `◀ Back to List` button added to top of furniture `show_form`
- [ ] Event Editor moved directly below Event Details in `show_inspector_panel`
- [ ] `egui::CollapsingHeader` removed from `show_starting_spells_editor`
- [ ] `ui.heading("Starting Spells")` added at the call site in `show_character_form`
- [ ] Starting Spells autocomplete filters candidates by character class
- [ ] Starting Spells `ScrollArea` minimum height set to show ~5 rows
- [ ] Starting Spells section added to `show_character_display`
- [ ] Unit tests added and passing
- [ ] `cargo fmt`, `cargo check`, `cargo clippy -D warnings` all clean

#### 1.9 Success Criteria

- Opening Furniture → Edit Furniture shows a **Back to List** button at the top
  of the form without scrolling.
- Clicking **Back to List** returns to the furniture list.
- Opening Maps → Edit Map → selecting a tile with an event and activating the
  Edit Event button shows the Event Editor immediately below the Event Details,
  not at the page footer.
- Opening Characters → Edit Character shows "Starting Spells" as a plain
  section heading (matching "Starting Equipment", "Starting Items",
  "Description", etc.) with the spell table and Add Spell picker always visible.
- The autocomplete for a Sorcerer character with a spell name shared by Cleric
  (e.g. `Awaken`) resolves to the Sorcerer variant.
- The Starting Spells `ScrollArea` shows at least 5 rows without scrolling.
- Opening Characters → Display Character shows the "Starting Spells" section
  listing all starting spell names.

---

### Phase 2: Stock Template Description – Data Model + SDK Wire-up + Display

Fix the round-trip bug that silently discards the `description` field of a
stock template when editing and saving, and ensure the description is also
visible in the display/list view.

---

#### 2.1 Game Engine – Add `description` to `MerchantStockTemplate`

**File:** `src/domain/world/npc_runtime.rs`

Add a `description` field to `MerchantStockTemplate` with `#[serde(default)]`
so existing RON files that omit `description` deserialise without error:

```antares/src/domain/world/npc_runtime.rs#L1-5
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

---

#### 2.2 SDK – Fix `from_template` to Load `description`

**File:** `sdk/campaign_builder/src/stock_templates_editor.rs`

**Function:** `StockTemplateEditBuffer::from_template` (L133)

Replace:

```antares/sdk/campaign_builder/src/stock_templates_editor.rs#L1-2
description: String::new(), // templates have no description field in the domain type
```

with:

```antares/sdk/campaign_builder/src/stock_templates_editor.rs#L1-2
description: template.description.clone(),
```

Remove the stale comment.

---

#### 2.3 SDK – Fix `to_template` to Persist `description`

**Function:** `StockTemplateEditBuffer::to_template` (L168)

In the `Ok(MerchantStockTemplate { … })` construction at the end of the
function, add:

```antares/sdk/campaign_builder/src/stock_templates_editor.rs#L1-2
description: self.description.clone(),
```

---

#### 2.4 SDK – Show Description in Stock Templates Display Screen

**File:** `sdk/campaign_builder/src/stock_templates_editor.rs`

**Function:** `show_template_display` (or the list-row / detail panel that
renders a read-only view of a `MerchantStockTemplate`)

**Problem:** The display/list view renders the template `id` and the count of
entries but never shows the `description`, so authors cannot verify what a
template is for without opening the edit form.

**Fix:** In the display panel, after the template ID heading add a description
row. If `description` is empty render an italicised placeholder
`"No description."` so the row is always present and scannable:

```antares/sdk/campaign_builder/src/stock_templates_editor.rs#L1-10
ui.label(egui::RichText::new("Description:").strong());
if template.description.is_empty() {
    ui.label(egui::RichText::new("No description.").italics().weak());
} else {
    ui.label(&template.description);
}
ui.end_row();
```

Ensure the `Grid` or layout that wraps this has `id_salt("stock_template_display_grid")`
or equivalent so it does not collide with other grids on the same frame.

---

#### 2.5 Testing Requirements

- Extend `test_from_template_round_trips` to assert that a template with a
  non-empty description round-trips correctly through `from_template` → mutate
  buffer description → `to_template`.
- Add `test_stock_template_description_is_persisted` verifying that
  `from_template` on a template whose `description` is `"General goods shop"`
  produces a buffer whose `description` equals `"General goods shop"`.
- Add `test_stock_template_description_to_template` verifying that `to_template`
  includes the buffer's description in the returned struct.
- Add `test_stock_template_display_shows_description` verifying that the display
  panel renders the description string when it is non-empty, and renders the
  placeholder when it is empty.
- Update any existing tests that construct `MerchantStockTemplate` with struct
  literal syntax to include the new `description` field.
- Run `cargo nextest run --all-features`.

#### 2.6 Deliverables

- [ ] `description: String` field added to `MerchantStockTemplate` with
      `#[serde(default)]`
- [ ] All struct literal construction sites updated
- [ ] `from_template` reads `template.description`
- [ ] `to_template` writes `self.description` into the returned struct
- [ ] Stock Templates display view renders the description (or placeholder)
- [ ] RON serialisation round-trip test passes (existing
      `test_load_from_file_round_trip` must continue to pass)
- [ ] New unit tests added and passing
- [ ] `cargo fmt`, `cargo check`, `cargo clippy -D warnings` all clean

#### 2.7 Success Criteria

- Opening Stock Templates → Edit Template on a template that already has a
  description shows the description pre-populated in the editor.
- Editing the description and saving round-trips the value correctly into the
  `.ron` data file.
- Loading the saved `.ron` file back into the editor shows the same description
  without loss.
- The Stock Templates list/display view shows the description (or "No
  description." placeholder) for every template without requiring the edit form
  to be opened.

---

### Phase 3: Container Gold and Gems + Place Event Map RON Save Fix

This phase covers two related map-event bugs: new currency fields for
containers (game engine + SDK), and the missing write-back that causes placed
Container and Furniture events to be lost when the map is saved.

---

#### 3.1 Game Engine – Add `gold` and `gems` to `MapEvent::Container`

**File:** `src/domain/world/types.rs`

Add two new fields to the `Container` variant of `MapEvent`. Both use
`#[serde(default)]` so all existing map RON files remain valid:

```antares/src/domain/world/types.rs#L1-14
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

---

#### 3.2 Game Engine – Propagate Gold/Gems Through `EventResult`

**File:** `src/domain/world/events.rs`

Extend `EventResult::EnterContainer` with two new fields:

```antares/src/domain/world/events.rs#L1-10
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

```antares/src/domain/world/events.rs#L1-9
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

---

#### 3.3 Game Engine – Container Inventory State

**File:** `src/application/container_inventory_state.rs`

Locate `ContainerInventoryState` (or equivalent). Add:

```antares/src/application/container_inventory_state.rs#L1-5
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

---

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

---

#### 3.5 SDK – Extend `EventEditorState` for Gold and Gems

**File:** `sdk/campaign_builder/src/map_editor.rs`

Add to `EventEditorState` (near the `container_*` fields, ~L2021):

```antares/sdk/campaign_builder/src/map_editor.rs#L1-5
/// Gold coins in this container (displayed as a text field, parsed as u32).
pub container_gold: String,
/// Gems in this container (displayed as a text field, parsed as u32).
pub container_gems: String,
```

In `Default for EventEditorState`, initialise both to `"0".to_string()`.

---

#### 3.6 SDK – Wire Gold/Gems in `to_map_event` and `from_map_event`

**Function:** `EventEditorState::to_map_event` (L2177)

In the `EventType::Container` branch, parse `self.container_gold` and
`self.container_gems` as `u32` (default `0` on parse failure) and include them
in the constructed `MapEvent::Container { … }`.

**Function:** `EventEditorState::from_map_event` (L2417)

In the `MapEvent::Container { gold, gems, … }` destructuring arm, set:

```antares/sdk/campaign_builder/src/map_editor.rs#L1-3
container_gold: gold.to_string(),
container_gems: gems.to_string(),
```

---

#### 3.7 SDK – Add Gold/Gems UI to `show_event_editor` Container Branch

**Function:** `show_event_editor` (L4858)

Inside `EventType::Container =>` (currently at ~L5700 in the file), add two
numeric text-edit rows between the Container ID row and the item list, following
the same `ui.horizontal` + `TextEdit::singleline` pattern used for other
numeric fields:

```antares/sdk/campaign_builder/src/map_editor.rs#L1-16
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

Add tooltip hints explaining that these values are taken when the player opens
and empties the container.

---

#### 3.8 SDK – Fix Place Event Container/Furniture Not Saved to Map RON

**File:** `sdk/campaign_builder/src/map_editor.rs`

**Problem:** When the author uses the "Place Event" tool to drop a Container or
Furniture event onto a map tile and then saves the map, the placed event is
visible in the in-editor tile state but is **not written to the `.ron` file**.
On the next load the tile appears empty. This is a missing write-back: the save
path serialises the map's existing event list but does not flush pending
`PlaceEvent` edits beforehand.

**Diagnosis steps:**

1. Locate the map save function (search for `save_map`, `write_map`, or
   `serialize_map` in `map_editor.rs`).
2. Confirm whether the placed Container/Furniture event is committed to
   `editor.map.events` (the canonical in-memory event list) at the point of save,
   or whether it remains only in `EventEditorState`.
3. If it remains only in `EventEditorState`: before serialising, call the same
   commit/apply logic that the "✅ Save Event" / "Apply" button uses to flush
   the pending `EventEditorState` into `editor.map.events`.

**Fix pattern:**

```antares/sdk/campaign_builder/src/map_editor.rs#L1-10
fn save_map(editor: &mut MapEditorState, …) -> Result<(), …> {
    // Flush any pending PlaceEvent edit before serialising.
    if matches!(editor.current_tool, EditorTool::PlaceEvent) {
        if let Some(tile_pos) = editor.selected_tile {
            commit_event_editor_to_map(editor, tile_pos);
        }
    }

    // Existing RON serialisation path unchanged below.
```

Implement `commit_event_editor_to_map` (or inline the logic if it already
exists under a different name) so it calls `to_map_event` on the current
`EventEditorState`, inserts/replaces the event at the target tile in
`editor.map.events`, and resets `editor.event_editor` to `Default::default()`.

Apply the same flush to the Furniture placement path if Furniture events
follow a separate code branch.

---

#### 3.9 Testing Requirements

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
- `test_place_event_container_commits_to_map_on_save` – construct a
  `MapEditorState` with `current_tool = PlaceEvent`, populate
  `EventEditorState` with a Container event, call `save_map`, and verify the
  saved `.ron` bytes contain the container event.
- `test_place_event_furniture_commits_to_map_on_save` – same for a Furniture
  event.

**SDK tests** (data/test_campaign integration):

- Ensure any test campaign container events in
  `data/test_campaign/data/` RON files that omit `gold`/`gems` still parse
  cleanly under `#[serde(default)]`.

Run `cargo nextest run --all-features` across both the root crate and the SDK
crate after each sub-step.

#### 3.10 Deliverables

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
- [ ] Placed Container/Furniture events are committed to the map before save
- [ ] Map RON file contains placed Container/Furniture events after save
- [ ] All new unit tests passing
- [ ] `cargo fmt`, `cargo check`, `cargo clippy -D warnings` all clean
- [ ] `data/test_campaign` RON files parse cleanly with new fields defaulting

#### 3.11 Success Criteria

- In the SDK, opening an Add Event → Container shows Gold and Gems input
  fields. Saving a container with gold=50, gems=3 round-trips through RON and
  reloads correctly in the editor.
- In-game, opening a container that has gold/gems shows the currency in the
  container right panel with Take Gold / Take Gems buttons.
- Take All also awards gold and gems to the party.
- A container with no gold/gems fields in its RON file loads without error and
  shows 0 for both.
- Placing a Container or Furniture event on a map tile and saving writes the
  event to the map `.ron` file; reloading the map restores the event on the
  same tile.

---

### Phase 4: NPC Editor – Create Merchant Dialog

When an NPC is designated as a Merchant, clicking "Create Merchant Dialog"
should automatically generate a sensible default dialog tree and assign it to
the NPC. Currently the button fires no action.

---

#### 4.1 SDK – Implement Create Merchant Dialog Action

**File:** `sdk/campaign_builder/src/npcs_editor.rs`

**Problem:** The button `"Create Merchant Dialog"` exists in the NPC edit form
but its `clicked()` branch is empty (or only sets a flag that is never acted
upon). No dialog is generated and the NPC is left without a dialog assignment.

**Fix:**

When the button is clicked, generate a minimal merchant dialog and insert it
into the campaign's dialog collection. The generated dialog must:

1. Have a unique `id` derived from the NPC's `id` (e.g.
   `format!("{}_merchant_dialog", npc.id)`).
2. Include a greeting node with text such as
   `"Welcome, traveller. Browse my wares."`.
3. Include at least one response branch:
   - `"Browse goods"` → triggers the merchant shop UI (use the existing
     `DialogAction::OpenShop` or equivalent action type).
   - `"Goodbye"` → ends the conversation.
4. Assign the new dialog's `id` to `npc.dialog_id` (or equivalent field) so
   the NPC is linked to it immediately.
5. Switch the NPC editor view to confirm the assignment (e.g. show the dialog
   id as a read-only label, or open the dialog editor focused on the new node).

```antares/sdk/campaign_builder/src/npcs_editor.rs#L1-20
if ui.button("Create Merchant Dialog").clicked() {
    let dialog_id = format!("{}_merchant_dialog", self.buffer.id);
    let dialog = DialogDefinition {
        id: dialog_id.clone(),
        nodes: vec![
            DialogNode {
                id: "greeting".to_string(),
                text: "Welcome, traveller. Browse my wares.".to_string(),
                responses: vec![
                    DialogResponse {
                        text: "Browse goods.".to_string(),
                        action: DialogAction::OpenShop,
                        next_node: None,
                    },
                    DialogResponse {
                        text: "Goodbye.".to_string(),
                        action: DialogAction::EndConversation,
                        next_node: None,
                    },
                ],
            },
        ],
    };
    campaign_data.dialogs.push(dialog);
    self.buffer.dialog_id = Some(dialog_id);
    ui.ctx().request_repaint();
}
```

Adapt the concrete types (`DialogDefinition`, `DialogNode`, `DialogResponse`,
`DialogAction`) to match the actual domain types in the codebase. Search
`src/domain/` for the dialog data structures before writing code.

---

#### 4.2 Testing Requirements

- Add `test_create_merchant_dialog_generates_dialog` – construct an
  `NpcEditBuffer` with `is_merchant = true`, simulate clicking the button, and
  assert that:
  - A `DialogDefinition` with id `"{npc_id}_merchant_dialog"` is present in
    the resulting campaign data.
  - The dialog has a greeting node with at least two response branches.
  - `buffer.dialog_id` equals the new dialog's id.
- Add `test_create_merchant_dialog_id_is_unique` – call the action twice on
  different NPCs and assert the two generated dialog ids differ.
- Run `cargo nextest run --all-features -p campaign_builder`.

#### 4.3 Deliverables

- [ ] `"Create Merchant Dialog"` button generates a `DialogDefinition` with
      greeting and at minimum Browse/Goodbye responses
- [ ] Generated dialog is inserted into campaign dialog collection
- [ ] NPC `dialog_id` field is assigned the new dialog id
- [ ] UI is repainted and shows the linked dialog id
- [ ] Unit tests added and passing
- [ ] `cargo fmt`, `cargo check`, `cargo clippy -D warnings` all clean

#### 4.4 Success Criteria

- Designating an NPC as a Merchant and clicking **Create Merchant Dialog**
  creates a dialog node visible in the Dialogs section of the Campaign Builder.
- The NPC editor shows the newly created dialog id linked to the NPC.
- The generated dialog has at least a greeting node, a "Browse goods" branch,
  and a "Goodbye" branch.
- The dialog is persisted to the campaign `.ron` files on the next campaign
  save.

---

### Phase 5: Validation – NPC Stock Templates

The validation subsystem flags NPC stock template references as unknown even
when the templates are defined in the campaign data. This produces false-positive
errors that obscure real validation problems.

---

#### 5.1 Investigate Validation Registry

**File:** `sdk/campaign_builder/src/validation.rs`

Locate the validation rule that checks NPC stock template references. Typical
implementation pattern:

```antares/sdk/campaign_builder/src/validation.rs#L1-8
// Builds a set of known template ids from the loaded campaign data,
// then checks each NPC's stock_template_id against it.
let known_templates: HashSet<&str> = campaign
    .stock_templates          // ← verify this field name
    .iter()
    .map(|t| t.id.as_str())
    .collect();
```

Confirm that the set is populated from the correct collection field. Common
failure modes:

- The validation pass runs before stock templates are loaded into the campaign
  data struct (ordering bug).
- The field name used to look up templates in the NPC struct differs from the
  field name used to build the known-ids set (naming mismatch).
- The registry is built from a stale snapshot rather than the live campaign
  data (cache invalidation bug).

---

#### 5.2 Fix the Registry Source

Once the root cause is confirmed, apply the minimal fix:

- If it is an **ordering bug**: move the stock-template registry build to before
  the NPC validation loop.
- If it is a **naming mismatch**: align the field name / id accessor on both
  sides of the lookup.
- If it is a **cache bug**: ensure the validation pass always reads from the
  current `CampaignData` snapshot and not from a cached intermediate.

---

#### 5.3 Testing Requirements

- Add `test_validation_known_stock_template_not_flagged` – build a minimal
  `CampaignData` with one `MerchantStockTemplate` and one NPC whose
  `stock_template_id` references it, run the validation pass, and assert no
  "unknown stock template" error is produced.
- Add `test_validation_unknown_stock_template_is_flagged` – same setup but
  with the NPC referencing a template id that does not exist, and assert the
  validation error is present.
- Run `cargo nextest run --all-features -p campaign_builder`.

#### 5.4 Deliverables

- [ ] Root cause of false-positive unknown-stock-template errors identified and
      documented in a code comment
- [ ] Validation registry correctly populated from live campaign data
- [ ] Valid stock template references produce no validation errors
- [ ] Invalid references still produce the appropriate error
- [ ] Unit tests added and passing
- [ ] `cargo fmt`, `cargo check`, `cargo clippy -D warnings` all clean

#### 5.5 Success Criteria

- Opening Validation in a campaign that has stock templates and NPCs referencing
  those templates shows **no** "unknown stock template" errors for valid
  references.
- Introducing an NPC with a genuinely missing template id still produces the
  validation error.
- No regressions in any other validation checks.

---

### Phase 6: Config Editor – Key Bindings Spellbook `[B]`

The Config Editor's Key Bindings section does not expose the `Spellbook` action
binding, preventing authors from remapping the `[B]` key through the SDK.

---

#### 6.1 Locate the Key Bindings Definition

**Files to check:**

- `sdk/campaign_builder/src/config_editor.rs` – the editor that renders the
  key-binding rows.
- `src/domain/config.rs` (or `src/application/config.rs`) – the `ControlsConfig`
  or equivalent struct that holds the key bindings.

Confirm that a `spellbook` (or `open_spellbook`) field exists in
`ControlsConfig`. If it does not exist, it must be added (see §6.2). If it
exists but is absent from the editor's binding list, proceed to §6.3.

---

#### 6.2 Game Engine – Add `spellbook` Binding to `ControlsConfig` (if missing)

**File:** `src/domain/config.rs` (or wherever `ControlsConfig` lives)

Add:

```antares/src/domain/config.rs#L1-6
/// Key binding that opens the Spellbook panel.
///
/// Default: `B`
#[serde(default = "default_spellbook_key")]
pub spellbook: KeyCode,
```

Add the corresponding `default_spellbook_key` function returning `KeyCode::B`
(or equivalent for the key library in use).

Update `data/test_campaign/config.ron` (the test fixture) to include the new
`spellbook` key so the test fixture stays valid. Per Implementation Rule 5,
`campaigns/tutorial/config.ron` is updated separately by the user as the live
campaign file.

---

#### 6.3 SDK – Add Spellbook Row to Key Bindings Editor

**File:** `sdk/campaign_builder/src/config_editor.rs`

**Function:** The function that renders the key-binding table rows (search for
`"key_bindings"` or the existing rows for `inventory`, `map`, `journal`, etc.)

Add a new row for `Spellbook` following the exact same pattern used by adjacent
rows:

```antares/sdk/campaign_builder/src/config_editor.rs#L1-9
// Spellbook
ui.label("Spellbook");
key_binding_widget(
    ui,
    &mut config_buffer.controls.spellbook,
    "key_binding_spellbook",
);
ui.end_row();
```

The `key_binding_widget` helper (or whatever the existing pattern uses) handles
rendering a clickable key-capture button and updating the binding in the buffer.

---

#### 6.4 Testing Requirements

- Add `test_config_editor_spellbook_key_binding_present` – construct a default
  `ConfigEditorBuffer` and assert `controls.spellbook` is `KeyCode::B`.
- Add `test_config_editor_spellbook_key_binding_roundtrips` – set
  `controls.spellbook = KeyCode::K`, serialise to RON, deserialise, and assert
  the binding is preserved.
- Verify that `data/test_campaign/config.ron` includes the `spellbook` key and
  that the existing `test_load_config_from_file` (or equivalent) continues to
  pass.
- Run `cargo nextest run --all-features`.

#### 6.5 Deliverables

- [ ] `spellbook` field present in `ControlsConfig` with `#[serde(default)]`
      and default `KeyCode::B`
- [ ] `data/test_campaign/config.ron` updated to include `spellbook` key
- [ ] Key Bindings editor in the SDK renders a `Spellbook` row
- [ ] Changing the binding in the editor round-trips correctly through RON
- [ ] Unit tests added and passing
- [ ] `cargo fmt`, `cargo check`, `cargo clippy -D warnings` all clean

#### 6.6 Success Criteria

- Opening Config Editor → Key Bindings shows a **Spellbook** row with the
  default key `B`.
- Remapping it to another key, saving, and reopening the config shows the new
  binding.
- Existing campaigns whose `config.ron` omits `spellbook` load without error
  and use the default `B` binding.

---

## Implementation Order Summary

| Phase | Scope                                                                                                  | Files Changed                                                                                         |
| ----- | ------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------- |
| 1     | Furniture Back button · Event Editor position · Starting Spells (heading, autocomplete, size, display) | `furniture_editor.rs`, `map_editor.rs`, `characters_editor.rs`                                        |
| 2     | Stock Template description round-trip + display view                                                   | `npc_runtime.rs`, `stock_templates_editor.rs`                                                         |
| 3     | Container gold/gems (engine + SDK + loot UI) + Place Event map RON save fix                            | `types.rs`, `events.rs`, `container_inventory_state.rs`, `container_inventory_ui.rs`, `map_editor.rs` |
| 4     | NPC Editor Create Merchant Dialog                                                                      | `npcs_editor.rs`, campaign dialog domain types                                                        |
| 5     | Validation – NPC Stock Templates false positives                                                       | `validation.rs`                                                                                       |
| 6     | Config Editor – Spellbook key binding                                                                  | `config.rs` (engine), `config_editor.rs` (SDK), `data/test_campaign/config.ron`                       |

Each phase must pass all four quality gates before the next phase begins:

```antares/antares#L1-6
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

## Notes

- All new fields that touch serialised data (`MapEvent::Container`,
  `MerchantStockTemplate`, `ControlsConfig`) MUST use `#[serde(default)]` to
  preserve backward compatibility with existing RON campaign files.
- Test data for automated tests lives in `data/test_campaign`, never in
  `campaigns/tutorial`.
- Any new `.ron` test fixture files go under `data/test_campaign/data/` as
  described in `AGENTS.md` Implementation Rule 5.
- `data/test_campaign/config.ron` must include ALL `ControlsConfig` keys
  (including `spellbook` after Phase 6) as noted in `AGENTS.md` Implementation
  Rule 5.
- No git operations; all commits are left to the user.
