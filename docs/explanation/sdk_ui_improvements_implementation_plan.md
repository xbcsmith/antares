# SDK UI Improvements Implementation Plan

## Overview

This plan outlines a phased approach to address critical issues in the Antares SDK Campaign Builder UI. The improvements focus on fixing broken functionality in the Quest Editor, enhancing the Classes Editor, and implementing consistent file I/O patterns across all editor tabs.

## Current State Analysis

### Existing Infrastructure

The Campaign Builder (`sdk/campaign_builder/src/`) uses egui for its UI and consists of:

- `main.rs` - Core application with all editor panels (~10,000 lines)
- `quest_editor.rs` - Quest editing state management
- `classes_editor.rs` - Class editing state management
- `dialogue_editor.rs` - Dialogue editing state management
- Various other supporting modules (map_editor, asset_manager, etc.)

The codebase already implements:

- RON-based serialization for all game data
- File dialog support via `rfd` crate
- ComboBox dropdowns for ID selection in objectives
- Import/Export functionality for some editors

### Identified Issues

#### Critical (Blocking Normal Use)

1. **Duplicate Stage Editor Call** - `show_quest_stages_editor` called twice at lines 5187-5190, causing UI ID clashes
2. **Cannot Add Objectives** - `selected_stage` is never set when viewing stages, blocking objective creation
3. **Save Quest Button Fails** - Quest persistence is broken

#### High Priority (UX Degradation)

4. **Quest ID Not Auto-Populated** - New quests don't receive auto-generated IDs
5. **Classes Tab Not Pre-Populated** - Classes don't load from campaign directory on startup
6. **Classes Missing Description Field** - No way to add class descriptions
7. **Classes Missing Starting Equipment** - No way to configure initial equipment from items data

#### Medium Priority (Consistency)

8. **Missing Load From File Buttons** - Items, Monsters, Maps, Quests, Classes tabs lack external file loading
9. **Missing Save To File Buttons** - Same tabs lack external file saving capability
10. **ScrollArea Not Expanding** - ScrollArea boxes don't fill available window space

## Implementation Phases

### Phase 1: Critical Quest Editor Fixes

**Goal**: Restore basic quest editing functionality

#### 1.1 Remove Duplicate Stage Editor Call

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: In `show_quest_form` (around line 5187-5190), remove the duplicate call to `show_quest_stages_editor(ui)`. The code currently has:

```text
// Stages editor
self.show_quest_stages_editor(ui);

// Stages editor   <-- DUPLICATE, REMOVE THIS
self.show_quest_stages_editor(ui);
```

Remove the second call and its comment.

#### 1.2 Fix Selected Stage Tracking for Objective Addition

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: In `show_quest_stages_editor`, when a stage collapsing header is clicked/expanded, set `selected_stage` to that stage's index. Modify the stage iteration loop to track which stage is expanded and update `self.quest_editor_state.selected_stage` accordingly.

**Implementation approach**:

- Add state tracking for which stage header was clicked
- When the collapsing header is opened, set `selected_stage = Some(stage_idx)`
- Ensure the "Add Objective" button in `show_quest_objectives_editor` can then work since `selected_stage` will be set

#### 1.3 Fix Quest ID Auto-Population

**File**: [`quest_editor.rs`](../../sdk/campaign_builder/src/quest_editor.rs)

**Change**: In `start_new_quest`, compute and set the next available ID in the quest buffer:

- Add a parameter or method to get the next available quest ID
- Set `self.quest_buffer.id` to the computed value as a string

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: Pass the next available ID when calling `start_new_quest()` or compute it inside the editor state.

#### 1.4 Testing Requirements

- Unit test: Verify `start_new_quest` populates buffer with unique ID
- Unit test: Verify `selected_stage` is set when stage is expanded
- Manual test: Create quest, add stages, add objectives - all should work
- Manual test: Save quest and reload - verify persistence

#### 1.5 Deliverables

- Fixed `show_quest_form` with single stage editor call
- Fixed stage selection tracking in `show_quest_stages_editor`
- Auto-populated quest IDs in `start_new_quest`
- All existing quest editor tests passing

#### 1.6 Success Criteria

- Can create a new quest with auto-generated ID
- Can add stages without UI ID clashes
- Can add objectives to any stage
- Quest Save button successfully persists changes

### Phase 2: Classes Editor Enhancements

**Goal**: Complete the Classes Editor with missing features

#### 2.1 Pre-Populate Classes from Campaign Directory

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: In `do_open_campaign`, after loading campaign metadata, call `load_classes()` to populate the classes editor state. This mirrors the pattern used for items, spells, and monsters:

```text
// After loading campaign.ron
self.load_items();
self.load_spells();
self.load_monsters();
self.load_classes();  // ADD THIS
```

#### 2.2 Add Description Field to Class Editor

**File**: [`domain/classes.rs`](../../src/domain/classes.rs)

**Change**: Add `description: String` field to `ClassDefinition`:

- Add `pub description: String` field after `name`
- Update any `ClassDefinition` constructors/builders to include description
- Update RON serialization tests if present

**File**: [`classes_editor.rs`](../../sdk/campaign_builder/src/classes_editor.rs)

**Change**: Add `description: String` field to `ClassEditBuffer`:

- Add field to struct
- Initialize to empty string in `Default` impl
- Populate from `ClassDefinition.description` in `start_edit_class`
- Save to `ClassDefinition.description` in `save_class`

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: In `show_class_form`, add description field in "Basic Info" group:

- Add multiline text edit for description after the name field

#### 2.3 Add Starting Equipment Configuration

**File**: [`domain/classes.rs`](../../src/domain/classes.rs)

**Change**: Add starting equipment fields to `ClassDefinition`:

- `pub starting_weapon_id: Option<ItemId>` - Default weapon for the class
- `pub starting_armor_id: Option<ItemId>` - Default armor for the class
- `pub starting_items: Vec<ItemId>` - Additional starting items (potions, tools, etc.)

**File**: [`classes_editor.rs`](../../sdk/campaign_builder/src/classes_editor.rs)

**Change**: Add starting equipment fields to `ClassEditBuffer`:

- `starting_weapon_id: String`
- `starting_armor_id: String`
- `starting_items: Vec<String>` (item IDs as strings for editing)

Update `start_edit_class` to populate these from `ClassDefinition`.
Update `save_class` to convert strings to `ItemId` and save to `ClassDefinition`.

#### 2.4 Improve Disablement Bit UI

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Problem**: The "Disablement Bit" field in the Classes editor is confusing to users. It's a raw numeric value with no explanation of what it represents or how it's used.

**Change**: In `show_class_form`, replace the raw text field with a more user-friendly UI:

- Add a tooltip/help text explaining the purpose: "This bit flag determines which items this class CANNOT use. Items with matching disablement flags will be restricted."
- Add a visual reference showing which bit corresponds to which class position
- Consider using a dropdown or checkbox-based selector instead of raw number

**UI Enhancement**:

```text
ui.group(|ui| {
    ui.horizontal(|ui| {
        ui.label("Item Restriction Bit:");
        ui.text_edit_singleline(&mut self.classes_editor_state.buffer.disablement_bit);
        ui.label("ℹ️").on_hover_text(
            "This bit flag (0-7) determines item restrictions.\n\
             Items can be flagged to disable usage by specific classes.\n\
             Bit 0 = Knight, Bit 1 = Paladin, Bit 2 = Archer, etc.\n\
             Example: A class with bit 2 cannot use items with disablement flag bit 2 set."
        );
    });

    // Show current bit meaning
    if let Ok(bit) = self.classes_editor_state.buffer.disablement_bit.parse::<u8>() {
        ui.label(format!("This class uses restriction bit position: {}", bit));
    }
});
```

**Alternative approach** - Replace with dropdown showing predefined positions:

```text
egui::ComboBox::from_id_salt("disablement_bit_selector")
    .selected_text(format!("Bit {} (Position {})", bit_value, bit_value))
    .show_ui(ui, |ui| {
        for i in 0..8 {
            ui.selectable_value(
                &mut self.classes_editor_state.buffer.disablement_bit,
                i.to_string(),
                format!("Bit {} - Class slot {}", i, i + 1),
            );
        }
    });
```

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: In `show_class_form`, add a "Starting Equipment" group with:

- ComboBox for weapon selection (populated from `self.items` filtered by `ItemTypeFilter::Weapon`)
- ComboBox for armor selection (populated from `self.items` filtered by `ItemTypeFilter::Armor`)
- List view with add/remove buttons for additional starting items
- Each item in list uses ComboBox populated from all `self.items`

#### 2.5 Testing Requirements

- Unit test: Verify classes load on campaign open
- Unit test: Verify class buffer populates all fields including new ones
- Manual test: Open campaign, verify classes tab shows loaded classes
- Manual test: Edit class, add description and equipment, save and reload
- Manual test: Disablement bit field shows helpful tooltip and is understandable

#### 2.6 Deliverables

- Classes auto-load when campaign opens
- Description field in class editor
- Starting equipment configuration with item dropdowns
- Improved disablement bit UI with tooltip and clear explanation
- Updated tests for new functionality

#### 2.7 Success Criteria

- Classes tab populated when campaign opens
- Can edit and save class descriptions
- Can configure starting equipment from available items
- Disablement bit field is clear and understandable to users
- All class editor tests passing

### Phase 3: Consistent File I/O Across All Tabs

**Goal**: Add Load From File and Save To File buttons to all content tabs

#### 3.1 Define Reusable File I/O Pattern

Create a consistent pattern for external file operations with merge/replace toggle:

**App State Addition** in [`main.rs`](../../sdk/campaign_builder/src/main.rs):

Add fields to `CampaignBuilderApp` for merge/replace behavior:

- `file_load_merge_mode: bool` - true = merge (default), false = replace

```text
// Load from external file pattern with merge/replace
ui.horizontal(|ui| {
    if ui.button("📂 Load from File").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RON", &["ron"])
            .pick_file()
        {
            if self.file_load_merge_mode {
                // Merge: Add new items, update existing by ID
                self.merge_items_from_file(&path);
            } else {
                // Replace: Clear existing and load fresh
                self.load_items_from_file(&path);
            }
        }
    }

    // Toggle for merge vs replace
    ui.checkbox(&mut self.file_load_merge_mode, "Merge");
    ui.label(if self.file_load_merge_mode { "(adds to existing)" } else { "(replaces all)" });
});

// Save to external file pattern
if ui.button("💾 Save to File").clicked() {
    if let Some(path) = rfd::FileDialog::new()
        .set_file_name("data.ron")
        .add_filter("RON", &["ron"])
        .save_file()
    {
        // Serialize and write data
    }
}
```

**Merge Logic**: When merging, iterate loaded items and:

- If item ID exists in current data, update it
- If item ID doesn't exist, append it
- Preserve items not present in loaded file

#### 3.2 Add File I/O to Items Tab

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: In `show_items_editor` toolbar section, add:

- "Load from File" button using `rfd::FileDialog::pick_file()`
- "Save to File" button using `rfd::FileDialog::save_file()`
- Helper methods `load_items_from_file(path)` and `save_items_to_file(path)`

#### 3.3 Add File I/O to Monsters Tab

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: In `show_monsters_editor`, add same pattern as Items.

#### 3.4 Add File I/O to Maps Tab

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: In `show_maps_editor`, add same pattern for map data.

#### 3.5 Add File I/O to Quests Tab

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: In `show_quests_editor`, add:

- "Load from File" button (in addition to existing Import RON)
- "Save to File" button for exporting to external location

#### 3.6 Add File I/O to Classes Tab

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: In `show_classes_editor`, add same pattern.

#### 3.7 Testing Requirements

- Manual test: Each tab can load RON file from arbitrary location
- Manual test: Each tab can save RON file to arbitrary location
- Manual test: Loaded data correctly populates editor
- Manual test: Saved data can be reloaded

#### 3.8 Deliverables

- All content tabs have "Load from File" button
- All content tabs have "Save to File" button
- Consistent error handling and status messages
- File type filters set to RON format

#### 3.9 Success Criteria

- Can load external data files into any editor tab
- Can export editor data to any location
- File dialogs use appropriate filters
- Status messages indicate success/failure

### Phase 4: ScrollArea Layout Improvements

**Goal**: Ensure ScrollArea components expand to fill available space

#### 4.1 Audit ScrollArea Usage

Review all `egui::ScrollArea` usages in `main.rs` and identify those that should expand:

- List panels (items, spells, monsters, quests, classes, dialogues)
- Preview panels
- Form content areas

#### 4.2 Apply Consistent ScrollArea Configuration

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: For each ScrollArea that should fill available space, update to use:

```text
egui::ScrollArea::vertical()
    .auto_shrink([false, false])  // Don't shrink to content
    .show(ui, |ui| {
        // content
    });
```

For panels in split layouts, ensure the parent panel uses `.fill()` or appropriate sizing.

#### 4.3 Fix Specific Problem Areas

Identified areas needing attention:

- Quest list panel (`show_quest_list`)
- Classes list panel (`show_classes_list`)
- Items list panel (`show_items_list`)
- Preview panels in all editors

#### 4.4 Testing Requirements

- Manual test: Resize window, verify scroll areas expand appropriately
- Manual test: Long lists scroll correctly within expanded area
- Manual test: No content clipping when window is small

#### 4.5 Deliverables

- All list ScrollAreas expand to fill panel height
- Preview ScrollAreas expand appropriately
- Consistent scroll behavior across all tabs

#### 4.6 Success Criteria

- ScrollAreas fill available space in all editors
- Lists remain scrollable at all window sizes
- No visual layout issues when resizing

### Phase 5: ID Management and Sorting

**Goal**: Ensure consistent ID auto-population, conflict detection, and sorted display across all editors

#### 5.1 Consistent ID Auto-Population

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: Ensure all "New" buttons auto-populate the ID field with the next available ID:

- Items: `next_available_item_id()` already exists - verify it's used in `show_items_editor` "Add Item" button
- Spells: `next_available_spell_id()` already exists - verify usage
- Monsters: `next_available_monster_id()` already exists - verify usage
- Maps: `next_available_map_id()` already exists - verify usage
- Quests: `next_available_quest_id()` already exists - ensure `start_new_quest` uses it (Phase 1 fix)
- Classes: Add `next_available_class_id()` method and use in `start_new_class`
- Dialogues: `next_available_dialogue_id()` already exists - verify usage

**Pattern for all editors**:

```text
if ui.button("➕ New [Type]").clicked() {
    let next_id = self.next_available_[type]_id();
    self.[type]_edit_buffer.id = next_id.to_string();
    // ... rest of new item setup
}
```

#### 5.2 ID Conflict Warning System

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: Add ID validation with warning display in all editor forms:

- When user edits an ID field, check for conflicts in real-time
- Display warning icon and message if ID already exists
- Allow save but show confirmation dialog on conflict

**Implementation approach**:

Add helper method for conflict detection:

```text
fn check_id_conflict<T, F>(items: &[T], current_idx: Option<usize>, new_id: &str, get_id: F) -> bool
where
    F: Fn(&T) -> String
{
    items.iter().enumerate().any(|(idx, item)| {
        Some(idx) != current_idx && get_id(item) == new_id
    })
}
```

**UI Pattern for ID fields**:

```text
ui.horizontal(|ui| {
    ui.label("ID:");
    let response = ui.text_edit_singleline(&mut self.[type]_edit_buffer.id);

    // Check for conflict
    let has_conflict = self.check_[type]_id_conflict(&self.[type]_edit_buffer.id);
    if has_conflict {
        ui.label("⚠️");
        ui.label(egui::RichText::new("ID already exists!").color(egui::Color32::YELLOW));
    }
});
```

**Save behavior with conflict**:

- If conflict detected on save, show confirmation dialog
- Dialog text: "An item with ID [X] already exists. This will overwrite it. Continue?"
- User can confirm (overwrite) or cancel

#### 5.3 Sorted Display by ID

**File**: [`main.rs`](../../sdk/campaign_builder/src/main.rs)

**Change**: Sort all lists by ID before display in list views:

**Items** - In `show_items_list`:

- After filtering, sort `filtered_items` by `item.id`

**Spells** - In `show_spells_list`:

- Sort filtered spells by `spell.id`

**Monsters** - In `show_monsters_list`:

- Sort filtered monsters by `monster.id`

**Maps** - In `show_maps_list`:

- Sort maps by `map.id` before iteration
- This specifically addresses the "maps listed out of order" issue

**Quests** - In `show_quest_list`:

- Sort `filtered_quests_cloned` by `quest.id`

**Classes** - In `show_classes_list`:

- Sort filtered classes by `class.id` (string comparison or parse to numeric)

**Dialogues** - In `show_dialogue_list`:

- Sort filtered dialogues by `dialogue.id`

**Implementation pattern**:

```text
// After filtering, before display
filtered_items.sort_by_key(|(_, item)| item.id);
```

For string IDs (like classes):

```text
filtered_classes.sort_by(|(_, a), (_, b)| {
    // Try numeric sort first, fall back to string sort
    match (a.id.parse::<u32>(), b.id.parse::<u32>()) {
        (Ok(a_num), Ok(b_num)) => a_num.cmp(&b_num),
        _ => a.id.cmp(&b.id),
    }
});
```

#### 5.4 Testing Requirements

- Unit test: `next_available_*_id()` returns correct next ID for each type
- Unit test: ID conflict detection correctly identifies duplicates
- Unit test: Sorted lists maintain correct order after operations
- Manual test: Create new item in each tab, verify ID is pre-populated
- Manual test: Edit ID to existing value, verify warning appears
- Manual test: Maps display in ID order (no longer confusing)

#### 5.5 Deliverables

- All "New" buttons auto-populate with next available ID
- ID fields show real-time conflict warnings
- Save with conflict shows confirmation dialog
- All list views sorted by ID
- `next_available_class_id()` method added

#### 5.6 Success Criteria

- Never need to guess the next ID - always pre-populated
- Clear visual warning when entering duplicate ID
- Can still use custom IDs with explicit conflict acknowledgment
- Maps and all other lists display in logical ID order
- Consistent ID management behavior across all editor tabs

## Implementation Order

**Recommended sequence**:

1. **Phase 1** - Critical fixes (blocking issues must be resolved first)
2. **Phase 5** - ID management (improves UX for all subsequent work)
3. **Phase 2** - Classes enhancements (independent of other phases)
4. **Phase 3** - File I/O consistency (can be done incrementally per tab)
5. **Phase 4** - Layout polish (lowest priority, cosmetic improvement)

## Risk Assessment

### Low Risk

- Duplicate stage editor removal (simple deletion)
- ScrollArea layout fixes (configuration changes only)
- Sorted list display (simple sort calls)

### Medium Risk

- File I/O additions (new functionality, but follows existing patterns)
- Quest ID auto-population (touches state management)
- ID auto-population consistency (existing patterns, needs verification)

### High Risk

- Classes description and starting equipment (requires domain model changes)
- Selected stage tracking fix (involves interaction between multiple components)
- ID conflict warning system (new UI patterns, confirmation dialogs)

## Design Decisions

The following decisions have been made for this implementation:

1. **Class Description**: A `description: String` field will be added to `ClassDefinition` in the domain model. This provides a place for class lore, gameplay tips, or other descriptive text.

2. **Starting Equipment Storage**: Starting equipment will be stored directly in `ClassDefinition` with:

   - `starting_weapon_id: Option<ItemId>` - Primary weapon
   - `starting_armor_id: Option<ItemId>` - Primary armor
   - `starting_items: Vec<ItemId>` - Additional items (consumables, tools, etc.)

3. **File I/O Merge vs Replace**: External file loading will:
   - **Merge by default** - New items added, existing items updated by ID match
   - **Replace option** - Checkbox toggle to switch to replace mode (clears existing data before loading)
   - UI shows current mode with descriptive text

## Dependencies

- `rfd` crate for file dialogs (already in use)
- `ron` crate for serialization (already in use)
- No new external dependencies required

## Testing Strategy

All changes must pass:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Manual verification checklist per phase provided in each phase section.
