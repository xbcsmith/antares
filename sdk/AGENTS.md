# AGENTS.md - SDK Development Guidelines

**CRITICAL**: This file supplements the root `AGENTS.md` with rules that apply
specifically to code inside the `sdk/` directory. All root `AGENTS.md` rules
still apply. When a rule here conflicts with the root, the more specific rule
here wins.

Read the root `AGENTS.md` first, then read this file before touching anything
under `sdk/`.

---

## Scope

This file covers the `sdk/campaign_builder` crate, which implements the egui-
based Campaign Builder GUI application. The rules below exist because GUI code
has failure modes that pure game-logic code does not.

---

## egui Widget ID Rules (MANDATORY)

### Why This Matters

egui assigns every widget a hash-based ID derived from its label text and its
position in the widget tree. When the same widget type appears more than once
under the same parent — especially inside loops or `columns()` calls — the IDs
collide silently. There is no compiler error, no clippy warning, and no panic.
The symptom is subtle misbehaviour: scroll areas share position, combo boxes
show the wrong selection, click events fire on the wrong row, and collapsing
headers open each other.

This has already caused a production bug in `template_browser.rs`. Every rule
below exists to prevent a recurrence.

---

### Rule 1: Every Loop Body Must Use `push_id`

Any widget rendered inside a `for` loop shares its egui ID with the same widget
in every other iteration unless the loop body is wrapped with `ui.push_id`.

**WRONG:**

```antares/sdk/examples/wrong_egui_loop.rs#L1-7
// Every row's horizontal and selectable_label hash to the same value.
for (id, entry) in items {
    ui.horizontal(|ui| {
        ui.selectable_label(is_selected, &entry.name);
    });
}
```

**RIGHT:**

```antares/sdk/examples/right_egui_loop.rs#L1-9
// push_id scopes all child widget IDs under the unique item id.
for (id, entry) in items {
    ui.push_id(id, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_label(is_selected, &entry.name);
        });
    });
}
```

For grid layouts that iterate rows **and** cells, push_id twice — once for the
row index and once for the item ID:

```antares/sdk/examples/right_egui_grid.rs#L1-13
for (row_idx, row) in items.chunks(cols).enumerate() {
    ui.push_id(row_idx, |ui| {
        ui.horizontal(|ui| {
            for (item_id, entry) in row {
                ui.push_id(item_id, |ui| {
                    ui.vertical(|ui| {
                        // all widgets here are uniquely scoped
                    });
                });
            }
        });
    });
}
```

The key to choose as the `push_id` argument is the item's **stable unique
identifier** — the registry ID string, the database index, or an enum
discriminant. Never use the loop counter alone as the sole key if items can be
reordered, because the counter changes meaning between frames.

---

### Rule 2: Every `ScrollArea` Must Have a Distinct `id_salt`

`egui::ScrollArea` uses a fixed internal ID by default. Two scroll areas in the
same window — for example a gallery list on the left and a detail preview on
the right — will share scroll position unless each has a distinct salt.

**WRONG:**

```antares/sdk/examples/wrong_egui_scroll.rs#L1-5
// Both scroll areas hash to the same internal ID.
egui::ScrollArea::vertical().show(ui, |ui| { /* gallery */ });

egui::ScrollArea::vertical().show(ui, |ui| { /* preview */ }); // ❌ clashes
```

**RIGHT:**

```antares/sdk/examples/right_egui_scroll.rs#L1-7
egui::ScrollArea::vertical()
    .id_salt("gallery_scroll")
    .show(ui, |ui| { /* gallery */ });

egui::ScrollArea::vertical()
    .id_salt(format!("preview_scroll_{}", selected_id))
    .show(ui, |ui| { /* preview */ });
```

When the scroll area wraps a per-item preview, include the item's unique ID in
the salt so that switching items also resets the scroll position cleanly.

**Naming convention for scroll area salts:**

```text
"{panel_name}_scroll"                  — fixed panel, appears once per frame
"preview_scroll_{item_id}"             — per-item preview
"{panel_name}_scroll_{disambiguator}"  — when two fixed panels share a parent
```

---

### Rule 3: Every `ComboBox` Must Use `from_id_salt`

`egui::ComboBox::from_label` hashes the label string. Two combo boxes with the
same visible label (e.g. two "Category:" filters on different panels rendered in
the same frame) will fight over the same popup slot.

**WRONG:**

```antares/sdk/examples/wrong_egui_combo.rs#L1-5
egui::ComboBox::from_label("Category:")   // ❌
    .show_ui(ui, |ui| { /* ... */ });

egui::ComboBox::from_label("Category:")   // ❌ same hash
    .show_ui(ui, |ui| { /* ... */ });
```

**RIGHT:**

```antares/sdk/examples/right_egui_combo.rs#L1-8
egui::ComboBox::from_id_salt("creature_category_filter")
    .selected_text(category_text)
    .show_ui(ui, |ui| { /* ... */ });

egui::ComboBox::from_id_salt("item_category_filter")
    .selected_text(category_text)
    .show_ui(ui, |ui| { /* ... */ });
```

Use `from_id_salt` with a **globally unique snake_case string constant** for
every combo box in the campaign builder. The string must encode both the panel
it lives in and what it controls:

```text
"{panel}_{field}_filter"     e.g.  "creature_category_filter"
"{panel}_{field}_selector"   e.g.  "map_editor_tile_selector"
"{panel}_sort_order"         e.g.  "template_browser_sort_order"
```

If a combo box appears inside a loop, also wrap it in `push_id` so the loop
iteration provides additional disambiguation.

---

### Rule 4: `CollapsingHeader` Titles Must Be Unique Within Their Parent

`egui::CollapsingHeader` uses its title string as its ID. Two collapsing
headers with the same title string inside the same parent will open and close
together.

**WRONG:**

```antares/sdk/examples/wrong_egui_collapsing.rs#L1-7
for creature in creatures {
    // Every header is titled "Details" — all open/close together.
    egui::CollapsingHeader::new("Details")
        .show(ui, |ui| { ui.label(&creature.name); });
}
```

**RIGHT:**

```antares/sdk/examples/right_egui_collapsing.rs#L1-7
for creature in creatures {
    ui.push_id(creature.id, |ui| {
        egui::CollapsingHeader::new("Details")
            .show(ui, |ui| { ui.label(&creature.name); });
    });
}
```

---

### Rule 5: `egui::Grid` and `egui::plot::Plot` Must Have a Unique `id_salt`

Both types take an ID in their constructor. Use a descriptive string constant,
not an empty string or a generic name like `"grid"`.

**WRONG:**

```antares/sdk/examples/wrong_egui_grid_id.rs#L1-3
egui::Grid::new("grid").show(ui, |ui| { /* ... */ }); // ❌ too generic
egui::Grid::new("grid").show(ui, |ui| { /* ... */ }); // ❌ clashes above
```

**RIGHT:**

```antares/sdk/examples/right_egui_grid_id.rs#L1-3
egui::Grid::new("creature_register_asset_preview_grid").show(ui, |ui| { /* ... */ });
egui::Grid::new("creature_mesh_properties_grid").show(ui, |ui| { /* ... */ });
```

---

### Rule 6: `SidePanel`, `TopBottomPanel`, and `CentralPanel` Must Be Registered Every Frame

`egui::SidePanel`, `egui::TopBottomPanel`, and `egui::CentralPanel` are not
ordinary widgets — they are **layout reservations**. egui allocates their space
during the layout pass, which happens before any widget closures run. If a
panel is wrapped in a condition and that condition is false on a given frame,
egui does not reserve the space, and no amount of state mutation inside other
closures on the same frame can bring the panel back for that frame.

The classic failure pattern:

1. Panel is guarded by `if some_flag { egui::SidePanel::right(...).show_inside(...) }`
2. User clicks a row inside the central scroll area — the click fires inside a
   closure that runs _after_ the panel block was already skipped.
3. `some_flag` is now true, but the panel was not registered this frame.
4. The panel appears on the next frame — only if something else triggers a
   repaint. Without `request_repaint()` it may never appear at all.

This caused the Creature Editor right panel to be invisible after clicking a
creature row (fixed 2025).

**WRONG:**

```antares/sdk/examples/wrong_egui_panel_conditional.rs#L1-8
// Panel skipped when nothing is selected → invisible on the first click.
if self.selected_entry.is_some() {
    egui::SidePanel::right("detail_panel")
        .show_inside(ui, |ui| {
            self.show_detail(ui, creatures);
        });
}
```

**RIGHT:**

```antares/sdk/examples/right_egui_panel_unconditional.rs#L1-16
// Panel always registered. Empty state shown as a placeholder.
egui::SidePanel::right("detail_panel")
    .default_width(300.0)
    .show_inside(ui, |ui| {
        match self.selected_entry {
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("Select an item to see details.")
                            .weak()
                            .italics(),
                    );
                });
            }
            Some(idx) => {
                self.show_detail(ui, creatures, idx);
            }
        }
    });
```

**Rules that follow from this:**

- Never wrap a `SidePanel`, `TopBottomPanel`, or `CentralPanel` in a boolean
  condition whose value can change as a result of widget interactions on the
  same frame.
- If a panel should be "hidden", show a placeholder instead of skipping the
  call entirely.
- The only acceptable use of a boolean guard on a panel is one that changes
  between separate user-initiated navigation events (e.g. switching editor
  tabs), never as a reaction to a click inside a sibling closure on the same
  frame.

---

### Rule 7: Call `request_repaint()` Whenever Layout-Driving State Changes

egui is an immediate-mode GUI that only repaints when told to. If you mutate
state inside a widget closure (such as setting `selected_entry = Some(idx)`)
and that state controls what is shown elsewhere in the same frame, egui must
be told to schedule a new frame immediately so the change becomes visible.

Without `request_repaint()` the user may need to move the mouse or interact
with the window before the UI reflects the change, creating the appearance of
a broken or frozen panel.

**Call `request_repaint()` after every state mutation that:**

- Toggles a panel's content (e.g. selecting an item that populates a side panel)
- Changes which tab or mode is active
- Opens or closes a window
- Updates a value displayed in a different panel from where the interaction
  happened

**WRONG:**

```antares/sdk/examples/wrong_egui_repaint.rs#L1-5
if response.clicked() {
    self.selected_entry = Some(idx); // ❌ no repaint requested
}
```

**RIGHT:**

```antares/sdk/examples/right_egui_repaint.rs#L1-6
if response.clicked() {
    self.selected_entry = Some(idx);
    ui.ctx().request_repaint(); // ✓ next frame is scheduled immediately
}
```

The same applies to tab switches in the top-level `update()`:

```antares/sdk/examples/right_egui_repaint_tab.rs#L1-5
if ui.button("Creatures").clicked() {
    self.active_tab = EditorTab::Creatures;
    ui.ctx().request_repaint(); // ✓ tab content appears without mouse move
}
```

---

### Rule 8: `egui::Window` Titles Must Be Unique Across the Whole Frame

`egui::Window::new(title)` uses the title string as the window's identity. Two
windows with the same title rendered in the same `update()` call are treated as
the same window by egui.

Guard every window with a boolean flag on the app state so it is only rendered
when open, and ensure no two windows share a title string:

```antares/sdk/examples/right_egui_window.rs#L1-10
// In CampaignBuilderApp state:
//   show_creature_template_browser: bool
//   show_register_asset_dialog: bool  (on CreaturesEditorState)

if self.show_creature_template_browser {
    egui::Window::new("Creature Template Browser")  // unique title
        .show(ctx, |ui| { /* ... */ });
}
```

---

### Rule 9: Always Use `TwoColumnLayout` for List/Detail Splits

The project provides `TwoColumnLayout` in `ui_helpers.rs` precisely to avoid
the egui panel-ordering problems described in Rule 6. **Never replace it with a
raw `SidePanel::right().show_inside()`** for a list/detail or
list/preview layout inside an editor panel.

`TwoColumnLayout` uses `ui.horizontal + ui.vertical`, which are ordinary widget
calls evaluated in document order. Both columns see the same frame state.
`SidePanel` is a layout reservation evaluated before all widget closures — any
state set by a click in the left column is invisible to the right panel until
the next frame.

**WRONG:**

```antares/sdk/examples/wrong_two_column_raw_panel.rs#L1-12
// Raw SidePanel evaluated before the list's ScrollArea fires clicks.
// Right panel sees stale `selected_entry` on the click frame.
egui::SidePanel::right("preview_panel")
    .default_width(300.0)
    .show_inside(ui, |ui| {
        if let Some(idx) = self.selected_entry {
            self.show_preview(ui, &items[idx]);
        }
    });
egui::ScrollArea::vertical()
    .id_salt("items_list_scroll")
    .show(ui, |ui| { /* list with click handlers */ });
```

**RIGHT:**

```antares/sdk/examples/right_two_column_helper.rs#L1-18
// TwoColumnLayout evaluates both columns in order; right column always
// sees the selection state set by the left column on the same frame.
TwoColumnLayout::new("items_editor")
    .with_inspector_min_width(300.0)
    .show_split(
        ui,
        |left_ui| {
            // list with click handlers
            // set pending_selection here; apply after show_split
        },
        |right_ui| {
            match preview_snapshot {
                None => { right_ui.label("Select an item to preview."); }
                Some((idx, ref item)) => { self.show_preview(right_ui, item, idx); }
            }
        },
    );
```

**Decision rule:** If you are tempted to reach for `SidePanel::right` inside an
editor panel, stop and use `TwoColumnLayout` instead. The only acceptable use of
`SidePanel` in this codebase is in the top-level `update()` function in
`lib.rs` (the global sidebar and the central panel itself).

---

### Rule 10: Pre-Compute Shared State Before Multi-Closure Calls

`TwoColumnLayout::show_split` — and any function that accepts two or more
closures simultaneously — requires that **both closures are constructed at the
same time**. The Rust borrow checker therefore sees both captures simultaneously
and will reject code where one closure captures `self` by shared reference and
the other captures `self` by mutable reference, even if the borrows are of
disjoint fields (E0500).

The fix is always the same: **extract everything the immutable closure needs
into owned local variables before the call**. This also makes the data flow
explicit and easier to reason about.

**WRONG:**

```antares/sdk/examples/wrong_two_closure_borrow.rs#L1-14
// E0500: left closure borrows self.id_manager (shared),
// right closure borrows all of self (mutable).
TwoColumnLayout::new("creatures_registry").show_split(
    ui,
    |left_ui| {
        for &idx in &filtered_indices {
            let ok = self.id_manager.validate_id(id, cat).is_ok(); // ❌ shared borrow
            // ...
        }
    },
    |right_ui| {
        self.show_preview(right_ui, creature, idx); // ❌ mutable borrow
    },
);
```

**RIGHT:**

```antares/sdk/examples/right_two_closure_precompute.rs#L1-22
// Pre-compute everything the left closure needs from self before show_split.
let row_valid: Vec<bool> = filtered_indices
    .iter()
    .map(|&idx| {
        let cat = CreatureCategory::from_id(creatures[idx].id);
        self.id_manager.validate_id(creatures[idx].id, cat).is_ok() // ✓ computed before
    })
    .collect();
let preview_snapshot: Option<(usize, CreatureDefinition)> =
    self.selected_registry_entry
        .and_then(|idx| creatures.get(idx).map(|c| (idx, c.clone()))); // ✓ owned clone

TwoColumnLayout::new("creatures_registry").show_split(
    ui,
    |left_ui| {
        // uses row_valid (Vec<bool>) — no self borrow
    },
    |right_ui| {
        self.show_preview(right_ui, &preview_snapshot, idx); // ✓ only mutable borrow
    },
);
```

**General pattern for any multi-closure call:**

1. Identify every piece of `self` each closure needs to read.
2. For closures that only read data: extract to an owned `let` binding before
   the call (clone if necessary).
3. For the one closure that needs `&mut self`: keep the method call inside that
   closure only.
4. Collect mutations from all closures as deferred local variables (`pending_*`)
   and apply them after the multi-closure call returns.

**The deferred mutation pattern:**

```antares/sdk/examples/right_deferred_mutation.rs#L1-15
let mut pending_selection: Option<usize> = None;
let mut pending_selection_reset_confirm = false;

TwoColumnLayout::new("my_editor").show_split(
    ui,
    |left_ui| {
        if response.clicked() {
            pending_selection = Some(idx);           // ✓ deferred
            pending_selection_reset_confirm = true;  // ✓ deferred
            left_ui.ctx().request_repaint();
        }
    },
    |right_ui| { /* ... */ },
);

// Apply mutations after show_split — no active closure borrows.
if let Some(new_idx) = pending_selection {
    if pending_selection_reset_confirm {
        self.delete_confirm_pending = false;
    }
    self.selected_entry = Some(new_idx);
}
```

---

### Rule 11: Test Contracts, Not Implementation Constants

Tests that hardcode the numeric value of a constant — rather than asserting
the contract the constant enforces — become stale silently when the constant
changes. There is no compiler error and no clippy warning. The test simply
fails with a confusing number mismatch on the next person who changes the
range.

This caused three test failures in `creatures_manager` when ID ranges were
widened from 50-wide bands to 1000-wide bands: the production code was correct
but the tests still asserted the old boundary numbers.

**WRONG:**

```antares/sdk/examples/wrong_test_constant.rs#L1-8
#[test]
fn test_monster_id_range() {
    let range = CreatureCategory::Monsters.id_range();
    assert_eq!(range.start, 1);
    assert_eq!(range.end, 51);  // ❌ hardcodes the end value
                                //    silently wrong after any range change
}
```

**RIGHT:**

```antares/sdk/examples/right_test_constant.rs#L1-15
#[test]
fn test_monster_id_range() {
    let range = CreatureCategory::Monsters.id_range();

    // Assert the contract: Monsters start at 1 and do not overlap NPCs.
    assert_eq!(range.start, 1);
    let npc_range = CreatureCategory::Npcs.id_range();
    assert_eq!(range.end, npc_range.start, "Monster range must end where NPC range begins");

    // Assert boundary membership — these are the semantically meaningful checks.
    assert!(range.contains(&1),    "ID 1 must be a Monster");
    assert!(range.contains(&(range.end - 1)), "last Monster ID must be in range");
    assert!(!range.contains(&range.end),      "first NPC ID must not be a Monster");
}
```

**Rules that follow from this:**

- Assert **boundary membership** (`range.contains(&x)`) rather than the
  raw numeric endpoints.
- Assert **cross-category non-overlap** (`range.end == next_category.start`)
  rather than a hardcoded integer.
- When a constant represents a capacity limit (e.g. `MAX_ITEMS`), import and
  reference the constant in the test — do not repeat its numeric value:

```antares/sdk/examples/right_test_capacity_constant.rs#L1-7
#[test]
fn test_inventory_full_at_max() {
    // ✓ References the constant — survives any capacity change.
    for i in 0..Inventory::MAX_ITEMS {
        inventory.add(i).unwrap();
    }
    assert!(inventory.add(99).is_err());
}
```

---

### Rule 12: Never Put a Growing Button Row in `ui.horizontal`; Use `ui.horizontal_wrapped`

`ui.horizontal` allocates exactly as much width as is available and clips
anything that overflows. A toolbar that looks fine at 1400 px becomes unusable
at 1000 px because the rightmost buttons are simply invisible — no scrollbar,
no overflow indicator, no error. The compiler and clippy cannot detect this.

This caused the Creature Editor toolbar's "Register Asset" and "Browse
Templates" buttons to be invisible at standard window widths (fixed 2025).

**WRONG:**

```antares/sdk/examples/wrong_toolbar_overflow.rs#L1-10
// All buttons in one horizontal row — clips silently on narrow windows.
ui.horizontal(|ui| {
    let toolbar_action = EditorToolbar::new("creatures_toolbar").show(ui);
    egui::ComboBox::from_id_salt("category_filter").show_ui(ui, |ui| { /* … */ });
    ui.label("Category");
    egui::ComboBox::from_id_salt("sort_order").show_ui(ui, |ui| { /* … */ });
    ui.label("Sort");
    if ui.button("🔄 Revalidate").clicked() { /* … */ }
    if ui.button("📥 Register Asset").clicked() { /* … */ }  // ❌ invisible at 1000 px
    if ui.button("📋 Browse Templates").clicked() { /* … */ } // ❌ invisible at 1000 px
});
```

**RIGHT:**

```antares/sdk/examples/right_toolbar_wrapped.rs#L1-17
// Row 1: EditorToolbar on its own line — it is already compact and always fits.
let toolbar_action = EditorToolbar::new("creatures_toolbar")
    .with_search(&mut self.search_query)
    .with_total_count(creatures.len())
    .show(ui);

// Row 2: filters and registry actions on a wrapped row so they reflow rather
// than clip when the window is narrow.
ui.horizontal_wrapped(|ui| {
    egui::ComboBox::from_id_salt("category_filter").show_ui(ui, |ui| { /* … */ });
    ui.label("Category");
    ui.separator();
    egui::ComboBox::from_id_salt("sort_order").show_ui(ui, |ui| { /* … */ });
    ui.label("Sort");
    ui.separator();
    if ui.button("🔄 Revalidate").clicked() { /* … */ }
    if ui.button("📥 Register Asset").clicked() { /* … */ }
    if ui.button("📋 Browse Templates").clicked() { /* … */ }
});
```

**Rules that follow from this:**

- Use `ui.horizontal` **only** for a fixed, small number of widgets whose total
  width is guaranteed to fit (e.g. a pair of labels, or a single icon + text).
- Use `ui.horizontal_wrapped` for every toolbar row that contains more than
  two or three buttons, or that contains a `ComboBox` plus buttons.
- Split long toolbars across two rows when they contain logically distinct
  groups: put the primary `EditorToolbar` on row 1 (it manages its own width),
  and put filters, secondary actions, and contextual buttons on row 2 with
  `horizontal_wrapped`.
- Apply the same rule to the Save/Cancel/action row in every edit-mode panel.

**Where "Register Asset" (and other contextual actions) must appear:**

A feature that can be triggered from a toolbar must also be reachable from
every context where it is relevant — specifically:

- The registry-list toolbar (row 2).
- The registry-list **preview panel** (right column), next to Edit.
- The **edit-mode** Save/Cancel row, so the user can register while editing.

If a button exists in only one of these places, users who don't know the full
layout will be unable to find it.

---

## egui ID Audit Checklist

Run this checklist on every function you touch before marking any campaign
builder UI task complete. One missed `push_id` is enough to reintroduce the bug
class this section was written to prevent.

**Panels:**

- [ ] No `SidePanel`, `TopBottomPanel`, or `CentralPanel` is wrapped in a
      boolean condition whose value can change from widget interactions on the
      same frame
- [ ] Every panel that was previously conditional now shows a placeholder when
      its content is absent, rather than being skipped entirely
- [ ] Every state mutation that changes panel content or tab visibility is
      followed by `ui.ctx().request_repaint()`

**Loops:**

- [ ] Every `for` loop that renders widgets wraps its body in
      `ui.push_id(unique_key, ...)`
- [ ] Grid views that chunk rows use `push_id(row_idx, ...)` on the row **and**
      `push_id(item_id, ...)` on each cell

**Scroll areas:**

- [ ] Every `ScrollArea` has a distinct `.id_salt("...")`
- [ ] Per-item scroll areas include the item's ID in the salt
- [ ] No two scroll areas in the same function share a salt string

**Combo boxes:**

- [ ] Every `ComboBox` uses `from_id_salt("unique_string")`, not `from_label`
- [ ] Every salt string is unique across the entire crate (grep for it)

**Repaint:**

- [ ] Every click / toggle handler that mutates layout-driving state calls
      `ui.ctx().request_repaint()`
- [ ] Tab-switch handlers in `update()` call `ctx.request_repaint()`

**Other widgets:**

- [ ] Every `CollapsingHeader` inside a loop is wrapped in `push_id`
- [ ] Every `egui::Grid` and `egui::plot::Plot` has a descriptive unique ID
- [ ] No two `egui::Window::new(...)` calls share a title string in the same
      `update()` frame

**Toolbar layout:**

- [ ] Every toolbar row that contains more than two buttons uses
      `ui.horizontal_wrapped`, not `ui.horizontal`
- [ ] The `EditorToolbar` widget stands alone on its own row; filters and
      secondary action buttons go on a second `horizontal_wrapped` row
- [ ] The Save/Cancel row in every edit-mode panel uses `ui.horizontal_wrapped`
- [ ] Every contextual action (Register Asset, Browse Templates, etc.) that
      appears in the toolbar also appears in the preview panel and the edit-mode
      action row so it is reachable from every workflow entry point

---

## SDK-Specific Workflow Steps

The root `AGENTS.md` Golden Workflow applies in full. Insert the following step
between step 6 (add tests) and step 7 (cargo fmt) when working on any file
under `sdk/campaign_builder/src/`:

```text
6a. (Campaign Builder UI only) Run the egui ID audit checklist above:
      - No SidePanel / TopBottomPanel / CentralPanel is conditionally skipped
        when its content can change from same-frame interactions — use a
        placeholder instead
      - Every state mutation that drives panel content calls request_repaint()
      - Every loop body rendering widgets is wrapped in push_id
      - Every ScrollArea has a distinct id_salt
      - Every ComboBox uses from_id_salt with a globally unique string
      - Every Grid / Plot / CollapsingHeader in a loop has a unique ID
      - No two Windows share a title string in the same frame
      - List/detail or list/preview splits use TwoColumnLayout, not raw SidePanel
      - Any multi-closure call (show_split, etc.) pre-computes shared state into
        owned locals before the call; mutations are deferred and applied after
      - Toolbar rows with more than two buttons use horizontal_wrapped, not horizontal
      - Every contextual action present in the toolbar is also present in the
        preview panel and the edit-mode action row
```

Do not skip this step even for "small" changes. ID collisions are invisible to
the compiler and to the test suite; the only defence is the audit.

---

## SDK-Specific Validation Checklist

Add these items to the standard checklist from root `AGENTS.md` when reviewing
campaign builder UI code:

### egui Panel and Repaint Correctness

- [ ] No `SidePanel`, `TopBottomPanel`, or `CentralPanel` skipped by a boolean
      guard that can flip during same-frame widget interactions
- [ ] Panels with conditional content show a placeholder when content is absent
- [ ] Every click / selection handler that mutates layout-driving state calls
      `ui.ctx().request_repaint()`
- [ ] Every tab-switch in `update()` calls `ctx.request_repaint()`

### Layout and Architecture

- [ ] List/detail and list/preview layouts use `TwoColumnLayout`, not raw
      `SidePanel::right().show_inside()`
- [ ] Any function that accepts two or more closures simultaneously has all
      shared `self` reads extracted to owned `let` bindings before the call
- [ ] All state mutations inside closures use the deferred `pending_*` pattern
      and are applied after the multi-closure call returns
- [ ] Tests assert contracts and boundary membership, not hardcoded numeric
      constant values
- [ ] Toolbar rows with more than two buttons use `ui.horizontal_wrapped`
- [ ] Every contextual action in the toolbar is also in the preview panel and
      the edit-mode action row

### egui ID Uniqueness

- [ ] All `push_id` calls verified for every widget loop
- [ ] All `ScrollArea` instances have distinct `id_salt` values
- [ ] All `ComboBox` instances use `from_id_salt` with unique strings
- [ ] All `Grid` / `Plot` instances have unique ID strings
- [ ] All `CollapsingHeader` instances inside loops use `push_id`
- [ ] Grepped the crate for duplicate `id_salt` and `from_id_salt` strings —
      none found

---

## Living Document

This file is updated whenever a new bug class is found and fixed in the
campaign builder. When you fix a new category of bug, add a rule and an
example here before closing the task.

Last updated: 2025

### Bugs recorded in this file

| Date | File                  | Pattern                                                                                                                                                                                                                                                           | Rule             |
| ---- | --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| 2025 | `template_browser.rs` | widgets in loops without `push_id`; bare `ScrollArea::vertical()`; `ComboBox::from_label`                                                                                                                                                                         | Rules 1, 2, 3    |
| 2025 | `creatures_editor.rs` | `SidePanel::right` wrapped in `if selected.is_some()`; no `request_repaint()` on click; bare `ScrollArea::vertical()`; `ComboBox::from_label`                                                                                                                     | Rules 2, 3, 6, 7 |
| 2025 | `creatures_editor.rs` | `SidePanel::right.show_inside` used instead of `TwoColumnLayout`; registry list loop rows missing `push_id`; `self.id_manager` borrowed inside left closure conflicting with `&mut self` capture in right closure — fixed by pre-computing `row_valid: Vec<bool>` | Rules 1, 6       |
| 2025 | `creatures_editor.rs` | All toolbar controls in one `ui.horizontal` — "Register Asset" and "Browse Templates" clipped invisible at standard window widths; button not present in preview panel or edit-mode row                                                                           | Rule 12          |
