# Creature Editor UX Fixes Implementation Plan

## Overview

This plan addresses five distinct UX gaps and bugs in the Campaign Builder's
creature editing workflow. The issues were identified by tracing the actual
runtime behavior of `sdk/campaign_builder/src/creatures_editor.rs` and
`sdk/campaign_builder/src/lib.rs` against the documented workflow in
`docs/how-to/create_creatures.md`.

The problems range from stale documentation to a silent data-loss bug and a
completely unwired template-browser component. Phases are ordered by risk and
user impact. Phases 1 through 3 are the highest priority because they block any
meaningful registry management. Phase 4 adds the missing import workflow. Phase
5 wires the already-built template browser.

---

## Current State Analysis

### Existing Infrastructure

| Component                    | Location                                                 | Status                                                                                                 |
| ---------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Creatures editor state       | `creatures_editor.rs`                                    | Struct present; core bugs documented below                                                             |
| Registry list (list mode)    | `creatures_editor::show_registry_mode()`                 | Renders; no right-column preview; no edit/delete/register actions                                      |
| Asset editor (edit/add mode) | `creatures_editor::show_edit_mode()`                     | Add works; Edit silently discards all changes (see Issue 2)                                            |
| `open_for_editing()` method  | `creatures_editor.rs`                                    | Correct implementation; never called from the registry double-click handler                            |
| `save_creatures()`           | `lib.rs`                                                 | Regenerates `creatures.ron` and all individual `.ron` files from `self.creatures` Vec on campaign save |
| `load_creatures()`           | `lib.rs`                                                 | Loads `creatures.ron` registry, then reads each referenced `.ron` file into `self.creatures` Vec       |
| Creature asset manager       | `creature_assets.rs`                                     | Built, file I/O for individual `.ron` files; not wired into any UI action                              |
| Template browser component   | `template_browser.rs`                                    | Fully built and tested; never instantiated in `CampaignBuilderApp`                                     |
| Creature template registry   | `creature_templates.rs` `initialize_template_registry()` | 24 templates built and tested; never called from any UI                                                |
| Template metadata types      | `template_metadata.rs`                                   | `TemplateRegistry` / `TemplateEntry` types available                                                   |
| Main template browser dialog | `lib.rs::show_template_browser_dialog()`                 | Covers Item / Monster / Quest / Dialogue / Map only; no Creature category                              |
| Tools menu                   | `lib.rs::update()`                                       | Has "Template Browser"; no "Creature Editor" entry                                                     |
| How-to guide                 | `docs/how-to/create_creatures.md`                        | Describes `Tools -> Creature Editor` (does not exist) and incorrect panel layout                       |

### Identified Issues

#### Issue 1 -- Documentation Mismatch

`create_creatures.md` Step 2 instructs users to navigate to
`Tools -> Creature Editor`. That menu entry does not exist. The Tools menu
contains "Template Browser", "Validate Campaign", "Advanced Validation Report",
"Balance Statistics", "Test Play", "Export Campaign", and "Preferences" — no
Creature Editor. The only entry point to creature editing is the `Creatures`
tab in the left sidebar.

The panel description that follows is also wrong. It describes three panels
(Template Browser / Preview / Properties), which reflects an early design
sketch. The actual layout in `show_edit_mode()` is: Mesh List (left) |
3D Preview placeholder (center) | Mesh Properties (right), with a
Creature-Level Properties strip at the bottom.

#### Issue 2 -- Silent Data-Loss Bug: Editing Registered Creatures is Completely Broken

This is the most severe issue. The double-click handler in
`show_registry_mode()` transitions to Edit mode as follows:

```rust
if response.double_clicked() {
    self.mode = CreaturesEditorMode::Edit;
    self.edit_buffer = creature.clone();
    self.selected_mesh_index = None;
    self.mesh_edit_buffer = None;
    self.mesh_transform_buffer = None;
    self.preview_dirty = true;
    // BUG: self.selected_creature is NOT set
}
```

It copies the creature into `edit_buffer` and switches `mode` to `Edit`, but
never sets `self.selected_creature = Some(idx)`. The field remains `None`.

In `show_edit_mode()`, the Save button code for Edit mode is:

```rust
CreaturesEditorMode::Edit => {
    if let Some(idx) = self.selected_creature {  // None -- entire block skipped
        if idx < creatures.len() {
            creatures[idx] = self.edit_buffer.clone();
            *unsaved_changes = true;
            result_message = Some(format!("Updated creature: ...", ...));
        }
    }
    // Falls through silently; mode resets to List; no data written
}
```

Because `selected_creature` is `None`, the block is skipped entirely. The mode
resets to `List` with no error message and no change to the `creatures` Vec.
All edits are silently discarded. The user sees the editor close normally and
has no indication the save failed.

The same `selected_creature` guard governs `ItemAction::Delete` and
`ItemAction::Duplicate` in `show_edit_mode()`. Both are also silently broken
for any creature entered via double-click from the registry.

The correct implementation, `open_for_editing()`, does set
`self.selected_creature = Some(index)`, but it is never called from the
double-click handler.

#### Issue 3 -- No Preview in Registry List Mode

`show_registry_mode()` renders a full-width flat list. Single-clicking a
creature sets `selected_registry_entry = Some(idx)` but nothing is rendered in
response. There is no right-column preview. Users cannot inspect a creature's
properties, see its mesh count, or access Edit/Delete/Duplicate actions without
blindly double-clicking into the broken edit flow described in Issue 2.

#### Issue 4 -- No Way to Register an Existing Creature Asset `.ron` File

There is no UI anywhere in the Campaign Builder to take a `.ron` file that
already exists on disk and add it as an entry in `creatures.ron`. The only way
to get a creature into the registry is:

1. Click New in the Creatures tab toolbar (`ToolbarAction::New` -> Add mode).
2. Fill in all fields.
3. Click Save (which pushes `edit_buffer` to `self.creatures`).
4. Save the campaign, which triggers `save_creatures()` to regenerate
   `creatures.ron` and write the individual `.ron` file.

There is no import-from-file, no register-by-path, and no way to reference a
`.ron` file that was created outside the Campaign Builder (e.g. hand-edited or
copied from another campaign).

#### Issue 5 -- Creature Templates Not Shown in Template Browser

The Tools menu Template Browser dialog (`show_template_browser_dialog()`) uses
`templates::TemplateCategory` which has five variants: `Item`, `Monster`,
`Quest`, `Dialogue`, `Map`. There is no `Creature` variant in this enum and no
creature-related content in `TemplateManager`.

A completely separate, fully tested creature template system exists:

- `template_browser.rs` -- `TemplateBrowserState` with grid/list view,
  category filter, complexity filter, tag search, and preview panel.
- `creature_templates.rs` -- `initialize_template_registry()` returning a
  `TemplateRegistry` with 24 production-ready creature templates.
- `template_metadata.rs` -- `TemplateRegistry`, `TemplateEntry`,
  `TemplateCategory` (Humanoid / Creature / Undead / Robot / Primitive),
  `Complexity` types.

`CampaignBuilderApp` has no field of type `TemplateBrowserState` and never
calls `initialize_template_registry()`. The entire subsystem is unreachable
from the running application.

---

## Implementation Phases

### Phase 1: Fix Documentation and Add Tools Menu Entry

#### 1.1 Add `Tools -> Creature Editor` Menu Entry

In `sdk/campaign_builder/src/lib.rs` inside the `Tools` menu block in
`impl eframe::App for CampaignBuilderApp::update()`, add a new button before
the first separator:

```rust
if ui.button("🐉 Creature Editor").clicked() {
    self.active_tab = EditorTab::Creatures;
    ui.close();
}
ui.separator();
```

This makes the navigation path the documentation already describes actually
work, and it is consistent with how other tabs are activated from the Tools
menu.

#### 1.2 Update `create_creatures.md` Getting Started Section

In `docs/how-to/create_creatures.md`, replace the "Opening the Campaign
Builder" subsection with accurate content:

- **Path A (via Tools menu):** `Tools -> Creature Editor` switches the active
  panel to the Creatures editor inside an already-open campaign.
- **Path B (direct tab):** Click the `Creatures` tab in the left sidebar.

Replace the three-panel description with the correct description of registry
mode (flat list with toolbar) and note that the three-panel layout (Mesh List /
3D Preview / Mesh Properties) only appears after opening an individual
creature for editing.

#### 1.3 Testing Requirements

- Add a unit test in `lib.rs` mod tests confirming the `EditorTab::Creatures`
  tab name returns `"Creatures"` (guards against regression during refactor).
- Manual smoke test: open app, click `Tools -> Creature Editor`, confirm the
  Creatures tab activates.

#### 1.4 Deliverables

- [ ] `sdk/campaign_builder/src/lib.rs` -- new "Creature Editor" button in
      Tools menu
- [ ] `docs/how-to/create_creatures.md` -- corrected Getting Started section

#### 1.5 Success Criteria

- `Tools` menu contains a "Creature Editor" entry that navigates to
  `EditorTab::Creatures`.
- Documentation accurately describes the navigation paths and actual panel
  layout.
- All quality gates pass with no new failures.

---

### Phase 2: Fix the Silent Data-Loss Bug in Edit Mode

This is the highest-priority code change. It must be fixed before any other
registry work because every subsequent phase depends on the edit workflow
being correct.

#### 2.1 Replace the Double-Click Handler in `show_registry_mode()`

In `sdk/campaign_builder/src/creatures_editor.rs`, inside the
`show_registry_mode()` function, replace the `double_clicked()` block:

**Current (broken):**

```rust
if response.double_clicked() {
    self.mode = CreaturesEditorMode::Edit;
    self.edit_buffer = creature.clone();
    self.selected_mesh_index = None;
    self.mesh_edit_buffer = None;
    self.mesh_transform_buffer = None;
    self.preview_dirty = true;
}
```

**Fixed:**

```rust
if response.double_clicked() {
    let file_name = format!(
        "assets/creatures/{}.ron",
        creature.name.to_lowercase().replace(' ', "_")
    );
    self.open_for_editing(creatures_slice, idx, &file_name);
}
```

`open_for_editing()` already sets `self.selected_creature = Some(index)`,
`self.edit_buffer = creature.clone()`, `self.mode = Edit`, and
`self.preview_dirty = true` correctly.

Note: `show_registry_mode()` currently takes `creatures: &mut [CreatureDefinition]`
(a slice). `open_for_editing()` also takes a slice, so no signature change is
required.

#### 2.2 Verify `ItemAction::Delete` and `ItemAction::Duplicate` Guards

With `selected_creature` now correctly set via `open_for_editing()`, the
existing `if let Some(idx) = self.selected_creature` guards in
`show_edit_mode()` for Delete and Duplicate will function correctly. No
additional changes are required for those paths.

#### 2.3 Testing Requirements

Add tests in `creatures_editor.rs` mod tests:

- `test_double_click_sets_selected_creature` -- simulate double-click index,
  confirm `selected_creature == Some(idx)` and `mode == Edit`.
- `test_edit_mode_save_updates_creature` -- open via `open_for_editing()`,
  modify `edit_buffer.name`, click Save, confirm `creatures[idx].name` is
  updated.
- `test_edit_mode_save_without_selected_creature_is_noop` -- directly set
  `mode = Edit` without calling `open_for_editing()`, click Save, confirm
  `creatures` Vec is unchanged (tests the guard itself, not the bug path).
- `test_delete_from_edit_mode_removes_creature` -- open via
  `open_for_editing()`, trigger Delete, confirm creature is removed from Vec.
- `test_duplicate_from_edit_mode_adds_creature` -- open via
  `open_for_editing()`, trigger Duplicate, confirm Vec length increases by one.

#### 2.4 Deliverables

- [ ] `sdk/campaign_builder/src/creatures_editor.rs` -- double-click handler
      replaced with `open_for_editing()` call
- [ ] Five new regression tests covering the previously broken paths

#### 2.5 Success Criteria

- Double-clicking a registered creature, making edits, and clicking Save
  updates the creature in `self.creatures` Vec.
- Delete and Duplicate from edit mode function correctly for creatures entered
  via double-click.
- No silent data discards. All existing tests pass.

---

### Phase 3: Preview Panel in Registry List Mode

#### 3.1 Redesign `show_registry_mode()` Layout

Refactor `show_registry_mode()` from a single full-width area to a two-column
layout using `egui::SidePanel::right("registry_preview_panel")` with
`default_width(300.0)` and `resizable(true)`. The right panel is shown only
when `self.selected_registry_entry.is_some()`.

If nesting a `SidePanel` inside `CentralPanel` causes rendering issues, fall
back to `ui.columns(2, ...)`, which is already proven in
`template_browser.rs::show()`.

#### 3.2 Implement `show_registry_preview_panel()`

Create a new private method on `CreaturesEditorState`:

```rust
fn show_registry_preview_panel(
    &mut self,
    ui: &mut egui::Ui,
    creatures: &mut Vec<CreatureDefinition>,
    campaign_dir: &Option<PathBuf>,
    unsaved_changes: &mut bool,
) -> Option<String>
```

The panel renders for the creature at `selected_registry_entry`. Content:

- Creature **name** as a heading.
- **ID** with category color from `CreatureCategory::from_id(creature.id).color()`.
- **Category** display name.
- **Scale** value.
- **Color tint** as a read-only colored swatch using `ui.painter()` or
  `egui::color_picker` in read-only mode.
- **Mesh count** with a collapsible list of mesh names and vertex counts.
- Derived **file path** using the same slug logic as `save_creatures()`:
  `assets/creatures/{slug}.ron`.

Action buttons:

- **"✏ Edit"** -- calls `self.open_for_editing(creatures, idx, &file_name)`.
  This is the primary action and should be the most prominent button.
- **"📋 Duplicate"** -- creates a copy with the next available ID (via
  `self.next_available_id(creatures)`), appends it to `creatures`, sets
  `*unsaved_changes = true`.
- **"🗑 Delete"** -- two-step confirmation to prevent accidental deletion (see
  Section 3.3).

#### 3.3 Two-Step Delete Confirmation

Add a field to `CreaturesEditorState`:

```rust
pub registry_delete_confirm_pending: bool,
```

Initialize to `false` in `Default`. When Delete is first clicked, set the flag
to `true` and change the button label to "⚠ Confirm Delete". A second click
executes the deletion; a "Cancel" button clears the flag without deleting.
Reset the flag whenever `selected_registry_entry` changes.

Update `back_to_registry()` to also reset `registry_delete_confirm_pending =
false`.

#### 3.4 Testing Requirements

Add tests in `creatures_editor.rs` mod tests:

- `test_registry_preview_not_shown_when_no_selection` -- confirm preview logic
  is skipped when `selected_registry_entry` is `None`.
- `test_registry_delete_confirm_flag_resets_on_selection_change` -- confirm
  `registry_delete_confirm_pending` resets when a different creature is
  selected.
- `test_registry_preview_edit_button_transitions_to_edit_mode` -- confirm
  clicking Edit in the preview panel calls `open_for_editing()` and sets
  `mode == Edit` with `selected_creature` set correctly.
- `test_registry_preview_duplicate_appends_creature` -- confirm Duplicate adds
  one creature to the Vec with a new ID.

#### 3.5 Deliverables

- [ ] `sdk/campaign_builder/src/creatures_editor.rs` -- `show_registry_mode()`
      refactored to two-column layout
- [ ] `sdk/campaign_builder/src/creatures_editor.rs` -- new
      `show_registry_preview_panel()` method
- [ ] `sdk/campaign_builder/src/creatures_editor.rs` -- `registry_delete_confirm_pending`
      field added to struct and `Default` impl
- [ ] `back_to_registry()` updated to reset the new field
- [ ] Four new unit tests

#### 3.6 Success Criteria

- Selecting any creature in the list renders name, ID, category, scale, color
  tint, and mesh count in the right column within one frame.
- Edit button opens the creature in the three-panel asset editor.
- Delete with two-step confirmation removes the creature from the Vec.
- Duplicate appends a new creature with the next available ID.
- All existing tests pass.

---

### Phase 4: Register Existing Creature Asset `.ron` File

#### 4.1 Add Register Asset State Fields

Add the following fields to `CreaturesEditorState`:

```rust
pub show_register_asset_dialog: bool,
pub register_asset_path_buffer: String,
pub register_asset_validated_creature: Option<CreatureDefinition>,
pub register_asset_error: Option<String>,
```

Initialize all to `false` / empty / `None` in `Default`.

#### 4.2 "Register Asset" Button in Toolbar

In `show_registry_mode()`, add a "📥 Register Asset" button to the toolbar
`ui.horizontal` block beside the existing "🔄 Revalidate" button. When clicked,
set `self.show_register_asset_dialog = true`.

#### 4.3 Implement the Register Asset Dialog

Create a new private method on `CreaturesEditorState`:

```rust
fn show_register_asset_dialog_window(
    &mut self,
    ctx: &egui::Context,
    creatures: &mut Vec<CreatureDefinition>,
    campaign_dir: &Option<PathBuf>,
    unsaved_changes: &mut bool,
) -> Option<String>
```

Render an `egui::Window` titled "Register Creature Asset" containing:

- A labeled text field bound to `self.register_asset_path_buffer`. The label
  should clarify that the path is relative to the campaign directory
  (e.g. `assets/creatures/goblin.ron`).
- A **"Validate"** button that:
  1. Resolves the full path: `campaign_dir.join(&self.register_asset_path_buffer)`.
  2. Reads the file with `std::fs::read_to_string`.
  3. Parses the contents with `ron::from_str::<CreatureDefinition>`.
  4. Checks that `creature.id` is not already present in `creatures`.
  5. On success, stores the parsed creature in
     `self.register_asset_validated_creature` and clears
     `self.register_asset_error`.
  6. On any failure, sets `self.register_asset_error` with a message
     including the specific cause (file not found, parse error with
     line/column, or ID already registered).
- A **"Register"** button, enabled only when
  `register_asset_validated_creature.is_some()`. When clicked, appends the
  validated creature to `creatures`, sets `*unsaved_changes = true`, clears
  all dialog state, and returns a success status string.
- A **"Cancel"** button that clears all dialog state without modifying
  `creatures`.
- An error label in `egui::Color32::RED` displayed when
  `self.register_asset_error.is_some()`.
- A success summary displayed when `register_asset_validated_creature.is_some()`
  (creature name, ID, mesh count) so users can confirm they are registering the
  right file before committing.

Call `show_register_asset_dialog_window()` from `show_registry_mode()` when
`self.show_register_asset_dialog` is `true`, passing `ui.ctx()`.

#### 4.4 ID Conflict Handling

The Validate step must check both ID range validity and duplicate IDs:

- Call `self.id_manager.validate_id(creature.id, category)` to detect range
  violations. Surface as: `"ID 5000 is outside the valid range for category
Monsters (1--999). Use a Monsters ID."`.
- Check `creatures.iter().any(|c| c.id == creature.id)` to detect duplicates.
  Surface as: `"ID 42 is already registered to 'Goblin'. Edit that creature or
choose a file with a unique ID."`.

#### 4.5 Path Normalization

Normalize path separators before joining with `campaign_dir`. Replace `\\`
with `/` and trim leading slashes. Document in the dialog label that paths must
use forward slashes and be relative to the campaign directory.

#### 4.6 Testing Requirements

Add tests in `creatures_editor.rs` mod tests:

- `test_register_asset_dialog_initial_state` -- all dialog fields default to
  empty/false/None.
- `test_register_asset_validate_duplicate_id_sets_error` -- confirm
  `register_asset_error` is set when a creature with the same ID is already in
  the Vec.
- `test_register_asset_register_button_disabled_before_validation` -- confirm
  the Register button logic is gated on `register_asset_validated_creature`.
- `test_register_asset_cancel_does_not_modify_creatures` -- confirm the Vec is
  unchanged after Cancel.
- `test_register_asset_success_appends_creature` -- provide a valid
  `CreatureDefinition` serialized to RON, confirm it is appended to `creatures`
  and `*unsaved_changes` is set.

#### 4.7 Deliverables

- [ ] `sdk/campaign_builder/src/creatures_editor.rs` -- four new state fields
- [ ] `sdk/campaign_builder/src/creatures_editor.rs` -- "Register Asset" button
      in toolbar
- [ ] `sdk/campaign_builder/src/creatures_editor.rs` -- `show_register_asset_dialog_window()`
      method with validate/register/cancel flow
- [ ] Five new unit tests

#### 4.8 Success Criteria

- A user can type a relative path to an existing `.ron` file, validate it, see
  a summary of its contents, and register it into the campaign's creature list
  in one workflow without leaving the Campaign Builder.
- ID conflicts and parse errors are surfaced with actionable messages before
  any modification to the Vec.
- Cancelling the dialog leaves the creature list unchanged.

---

### Phase 5: Wire Creature Template Browser into the Campaign Builder

This phase wires the fully built but completely disconnected creature template
system into `CampaignBuilderApp`.

#### 5.1 Add Creature Template Browser State to `CampaignBuilderApp`

In `sdk/campaign_builder/src/lib.rs`, add to the `CampaignBuilderApp` struct:

```rust
creature_template_registry: template_metadata::TemplateRegistry,
creature_template_browser_state: template_browser::TemplateBrowserState,
show_creature_template_browser: bool,
```

Initialize in `Default::default()`:

```rust
creature_template_registry: creature_templates::initialize_template_registry(),
creature_template_browser_state: template_browser::TemplateBrowserState::new(),
show_creature_template_browser: false,
```

#### 5.2 Add "Creature Templates..." Entry to the Tools Menu

In the Tools menu block in `update()`, add beneath the existing
"📋 Template Browser..." button:

```rust
if ui.button("🐉 Creature Templates...").clicked() {
    self.show_creature_template_browser = true;
    ui.close();
}
```

#### 5.3 Implement `show_creature_template_browser_dialog()`

Add a new private method to `CampaignBuilderApp`:

```rust
fn show_creature_template_browser_dialog(&mut self, ctx: &egui::Context)
```

The method:

1. Collects template references:
   `let entries: Vec<&TemplateEntry> = self.creature_template_registry.all_templates();`
2. Opens an `egui::Window` titled "🐉 Creature Template Browser" with
   `resizable(true)` and `default_size([900.0, 600.0])`.
3. Delegates rendering to
   `self.creature_template_browser_state.show(ui, &entries)`.
4. Handles the returned `Option<TemplateBrowserAction>`:
   - `TemplateBrowserAction::CreateNew(template_id)`: call
     `self.creature_template_registry.generate(&template_id)` to produce a
     `CreatureDefinition`, assign the next available ID using
     `self.creatures_editor_state.id_manager.suggest_next_id(category)`,
     push the creature onto `self.creatures`, switch to
     `EditorTab::Creatures`, open the editor with
     `self.creatures_editor_state.open_for_editing(...)`, set
     `self.unsaved_changes = true`, and set a status message.
   - `TemplateBrowserAction::ApplyToCurrent(template_id)`: if the creatures
     editor is in `Edit` mode, call `generate()` and copy the mesh data into
     `self.creatures_editor_state.edit_buffer`, set `preview_dirty = true`, and
     set a status message. If not in Edit mode, set a status message explaining
     that a creature must be open in the editor first.
5. Guard the call in `update()`:
   `if self.show_creature_template_browser { self.show_creature_template_browser_dialog(ctx); }`

#### 5.4 Surface Templates from Within the Creatures Editor

Add a sentinel constant to `creatures_editor.rs`:

```rust
pub const OPEN_CREATURE_TEMPLATES_SENTINEL: &str =
    "__campaign_builder::open_creature_templates__";
```

Add a "📋 Browse Templates" button to the toolbar in `show_registry_mode()`
and to the action row in `show_edit_mode()`. When clicked, return
`Some(OPEN_CREATURE_TEMPLATES_SENTINEL.to_string())`.

In the `EditorTab::Creatures` match arm in `update()`, detect the sentinel:

```rust
if let Some(msg) = self.creatures_editor_state.show(...) {
    if msg == creatures_editor::OPEN_CREATURE_TEMPLATES_SENTINEL {
        self.show_creature_template_browser = true;
    } else {
        self.status_message = msg;
    }
}
```

This pattern is consistent with the existing `requested_open_npc` mechanism in
the Maps editor.

#### 5.5 Testing Requirements

Add tests in `lib.rs` mod tests:

- `test_creature_template_browser_defaults_to_hidden` -- confirm
  `show_creature_template_browser` is `false` after `Default::default()`.
- `test_creature_template_registry_non_empty_on_default` -- confirm
  `creature_template_registry` has templates after initialization.
- `test_creature_template_sentinel_sets_show_flag` -- simulate the sentinel
  return value and confirm `show_creature_template_browser` is set to `true`.

Confirm all existing `template_browser.rs` tests continue to pass unchanged.

#### 5.6 Deliverables

- [ ] `sdk/campaign_builder/src/lib.rs` -- three new fields on
      `CampaignBuilderApp` and `Default` initialization
- [ ] `sdk/campaign_builder/src/lib.rs` -- "Creature Templates..." menu entry
      in Tools menu
- [ ] `sdk/campaign_builder/src/lib.rs` -- `show_creature_template_browser_dialog()`
      method
- [ ] `sdk/campaign_builder/src/lib.rs` -- dialog call guarded in `update()`
      dialogs block
- [ ] `sdk/campaign_builder/src/lib.rs` -- sentinel detection in
      `EditorTab::Creatures` match arm
- [ ] `sdk/campaign_builder/src/creatures_editor.rs` -- `OPEN_CREATURE_TEMPLATES_SENTINEL`
      constant
- [ ] `sdk/campaign_builder/src/creatures_editor.rs` -- "Browse Templates"
      button in registry toolbar and edit mode action row
- [ ] Three new unit tests in `lib.rs` mod tests

#### 5.7 Success Criteria

- `Tools -> Creature Templates...` opens the full-featured grid/list browser
  with all 24 registered creature templates, category filter, complexity
  filter, and preview panel functional.
- "Create New" on a template creates a creature, registers it in
  `self.creatures`, switches to the Creatures tab, and opens it in the
  three-panel editor ready to customize.
- "Apply to Current" while a creature is open in edit mode replaces its mesh
  data without discarding the creature's ID or name.
- "Browse Templates" button inside the Creatures tab toolbar opens the same
  dialog.
- The existing "Template Browser" (Items / Monsters / Quests / Dialogues /
  Maps) is unaffected.

---

## Cross-Cutting Concerns

### Error Handling

All new file I/O in Phase 4 must use `Result<T, E>` propagation. No `unwrap()`
or `expect()` without justification. The dialog's validate path must never
panic and must always surface the failure to the user via
`self.register_asset_error`.

### Code Quality Gates

Before marking any phase complete all four gates must pass:

```
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

The campaign_builder test suite currently has 1,759 passing tests. That count
must not decrease.

### Documentation Updates

After all phases are complete, update `docs/explanation/implementations.md`
with a new row in the implementation status table summarizing the fixes.

---

## Implementation Order Summary

| Phase | Primary File                    | Depends On                                    | Risk       | Effort |
| ----- | ------------------------------- | --------------------------------------------- | ---------- | ------ |
| 1     | `lib.rs`, `create_creatures.md` | None                                          | Low        | Small  |
| 2     | `creatures_editor.rs`           | None                                          | Low        | Small  |
| 3     | `creatures_editor.rs`           | Phase 2 (uses `open_for_editing()` correctly) | Low-Medium | Medium |
| 4     | `creatures_editor.rs`           | Phase 2                                       | Medium     | Medium |
| 5     | `lib.rs`, `creatures_editor.rs` | Phases 2 and 3                                | Medium     | Large  |

Phases 1 and 2 are independent and can be implemented simultaneously. Phase 2
must land before Phase 3 and Phase 5 so that the Edit button in the preview
panel and the "Create from Template" flow both use a working edit path. Phase 4
is independent of Phase 3 and Phase 5 and can be implemented in parallel with
them.

---

## Risk Mitigation

### Phase 2 Regression Risk

The double-click fix is a one-line behavioral change. The main risk is that
existing tests were written against the broken behavior and implicitly expected
`selected_creature` to remain `None`. The new tests described in Section 2.3
explicitly verify both the corrected behavior and the guard behavior, providing
a clear regression baseline.

### Phase 3 Layout Nesting Risk

Introducing `egui::SidePanel::right()` inside the area already owned by
`egui::CentralPanel` can produce layout conflicts depending on the egui version
in use. The fallback is `ui.columns(2, ...)` which is already used in
`template_browser.rs::show()` and known to work in the project's egui version.
Test this early in Phase 3 before building the full preview panel content.

### Phase 4 Path Portability Risk

User-typed paths will vary by operating system. Normalize all input by
replacing `\\` with `/` and stripping leading slashes before joining with
`campaign_dir`. The dialog label should state explicitly that paths are
relative to the campaign root and use forward slashes.

### Phase 5 Sentinel String Risk

Using a magic sentinel string is simple but fragile. Defining it as a `pub
const` in `creatures_editor.rs` and referencing the constant (not the literal)
in `lib.rs` prevents typo bugs and makes the intent clear to future readers.

---

## Success Metrics

The following criteria collectively define this plan as complete:

1. `docs/how-to/create_creatures.md` accurately describes every navigation path
   that works in the running application.
2. Double-clicking a registered creature, editing it, and clicking Save
   persistently updates the creature in `self.creatures` with no silent
   discard.
3. Delete and Duplicate in edit mode function correctly for creatures entered
   via double-click from the registry list.
4. Selecting any creature in the registry list renders a preview in a right
   column within one frame, with Edit, Duplicate, and Delete actions
   accessible without entering the full three-panel editor.
5. A user can register an existing `.ron` file that already exists on disk into
   the campaign's creature registry by typing a path, validating it, and
   clicking Register — without leaving the Campaign Builder and without risking
   silent failure.
6. `Tools -> Creature Templates...` opens the `TemplateBrowserState` window
   displaying all 24 built-in creature templates with category filter,
   complexity filter, and preview panel functional.
7. Creating a creature from a template navigates the user to the creature in
   the three-panel editor, ready to customize, with a correctly set
   `selected_creature` index so Save, Delete, and Duplicate all work.
8. All four `cargo` quality gates pass with zero warnings or failures and the
   test count does not decrease from 1,759.
