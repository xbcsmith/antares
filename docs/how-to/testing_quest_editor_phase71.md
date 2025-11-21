# Testing Quest Editor - Phase 7.1

Manual test checklist for the Quest Stage & Objective Editing UI features implemented in Phase 7.1.

## Prerequisites

- Campaign Builder built and running: `cargo run --release --package campaign_builder --bin campaign-builder`
- A campaign loaded (or create a new one via File → New Campaign)
- At least one quest created in the Quests tab

## Test Suite

### Test 1: Stage Editing Modal

**Purpose**: Verify stage editing workflow with modal dialog

- [ ] Navigate to Quests tab
- [ ] Select a quest from the list
- [ ] Expand a quest stage by clicking its header
- [ ] Click **✏️** button next to the stage name
- [ ] **Expected**: Modal dialog opens titled "Edit Stage"
- [ ] **Expected**: Form shows Stage Number, Name, Description fields pre-filled with current values
- [ ] **Expected**: "Require all objectives" checkbox reflects current state
- [ ] Modify the stage name (e.g., append " - Edited")
- [ ] Modify the description
- [ ] Toggle the "Require all objectives" checkbox
- [ ] Click **✅ Save**
- [ ] **Expected**: Modal closes
- [ ] **Expected**: Stage name in list updates to show new value
- [ ] **Expected**: "● Unsaved changes" appears in status bar

### Test 2: Stage Deletion

**Purpose**: Verify stage removal works correctly

- [ ] Expand a quest with at least 2 stages
- [ ] Note the number of stages before deletion
- [ ] Click **🗑️** button on the second stage
- [ ] **Expected**: Stage removed from list immediately
- [ ] **Expected**: Quest now has one fewer stage
- [ ] **Expected**: "● Unsaved changes" indicator appears
- [ ] Remaining stages still display correctly

### Test 3: Objective Editing Modal

**Purpose**: Verify objective editing workflow

- [ ] Expand a stage that has at least one objective
- [ ] Click **✏️** button next to an objective
- [ ] **Expected**: Modal dialog opens titled "Edit Objective"
- [ ] **Expected**: Objective Type dropdown shows current type pre-selected
- [ ] **Expected**: Form fields below show data for that objective type
- [ ] **Example**: For "Kill Monsters" - Monster ID and Quantity fields visible
- [ ] Modify one of the fields (e.g., change quantity from 5 to 10)
- [ ] Click **✅ Save**
- [ ] **Expected**: Modal closes
- [ ] **Expected**: Objective description updates in list
- [ ] **Expected**: "● Unsaved changes" appears

### Test 4: Objective Type Conversion

**Purpose**: Verify dynamic form updates when changing objective type

- [ ] Edit an objective with type "Kill Monsters"
- [ ] Note the fields: Monster ID, Quantity
- [ ] Change the Objective Type dropdown to "Collect Items"
- [ ] **Expected**: Form fields update instantly to show Item ID, Quantity
- [ ] **Expected**: Previous Monster ID value is cleared
- [ ] Fill in Item ID: `42`, Quantity: `5`
- [ ] Click **✅ Save**
- [ ] **Expected**: Objective now shows "Collect 5 of Item 42" (or similar)
- [ ] Edit the same objective again
- [ ] **Expected**: Type shows "Collect Items" and fields show Item ID: 42, Quantity: 5

### Test 5: All Objective Types

**Purpose**: Verify all 7 objective types render correct form fields

#### Kill Monsters

- [ ] Select "Kill Monsters" → Fields: Monster ID, Quantity appear
- [ ] Fill in values → Save → Objective description displays correctly

#### Collect Items

- [ ] Select "Collect Items" → Fields: Item ID, Quantity appear
- [ ] Fill in values → Save → Description correct

#### Reach Location

- [ ] Select "Reach Location" → Fields: Map ID, X, Y, Radius appear
- [ ] Fill in values → Save → Description correct

#### Talk To NPC

- [ ] Select "Talk To NPC" → Fields: NPC ID, Map ID appear
- [ ] Fill in values → Save → Description correct

#### Deliver Item

- [ ] Select "Deliver Item" → Fields: Item ID, NPC ID, Quantity appear
- [ ] Fill in values → Save → Description correct

#### Escort NPC

- [ ] Select "Escort NPC" → Fields: NPC ID, Map ID, Destination X, Destination Y appear
- [ ] Fill in values → Save → Description correct

#### Custom Flag

- [ ] Select "Custom Flag" → Fields: Flag Name (text), Required Value (checkbox) appear
- [ ] Fill in values → Save → Description correct

### Test 6: Objective Deletion

**Purpose**: Verify objective removal and list renumbering

- [ ] Expand a stage with at least 3 objectives
- [ ] Note the objective numbers (1., 2., 3., etc.)
- [ ] Click **🗑️** button on the second objective
- [ ] **Expected**: Objective removed immediately
- [ ] **Expected**: Remaining objectives renumbered (1., 2., ...)
- [ ] **Expected**: No gaps in numbering
- [ ] **Expected**: "● Unsaved changes" appears

### Test 7: Modal Cancel Behavior

**Purpose**: Verify canceling discards changes

- [ ] Edit a stage → Make changes → Click **❌ Cancel**
- [ ] **Expected**: Modal closes
- [ ] **Expected**: Changes NOT applied (stage name unchanged)
- [ ] Edit an objective → Change type and fill fields → Click **❌ Cancel**
- [ ] **Expected**: Modal closes
- [ ] **Expected**: Objective unchanged (still has original type and data)

### Test 8: Persistence Verification

**Purpose**: Verify changes persist after save/reload cycle

- [ ] Make multiple edits:
  - [ ] Edit 2 different stages
  - [ ] Edit 3 different objectives (change types on at least one)
  - [ ] Delete 1 stage
  - [ ] Delete 1 objective
- [ ] Note all changes made
- [ ] Click **File → Save** (or Ctrl+S)
- [ ] **Expected**: "Campaign saved" message or similar feedback
- [ ] **Expected**: "● Unsaved changes" disappears
- [ ] Close Campaign Builder completely
- [ ] Reopen Campaign Builder
- [ ] Click **File → Open** → Select the same campaign.ron file
- [ ] Navigate to Quests tab
- [ ] Select the edited quest
- [ ] Expand stages and objectives
- [ ] **Expected**: ALL changes persisted correctly
  - [ ] Stage edits present
  - [ ] Objective edits present (including type conversions)
  - [ ] Deleted items still gone

### Test 9: Error Handling

**Purpose**: Verify graceful handling of edge cases

- [ ] Try editing with no quest selected (should not crash)
- [ ] Try clicking Edit on a stage, then Delete on same stage → Modal behavior?
- [ ] Open multiple modals by clicking Edit on stage AND objective → Only one should be active
- [ ] Fill objective form with invalid data (e.g., text in numeric fields) → Save → What happens?
  - **Note**: Current implementation stores as strings, so this may not error
- [ ] Close modal with X button (if available) → Should behave like Cancel

### Test 10: Usability & UI/UX

**Purpose**: Verify user-friendly interface

- [ ] Hover over ✏️ buttons → **Expected**: Tooltip shows "Edit Stage" or "Edit Objective"
- [ ] Hover over 🗑️ buttons → **Expected**: Tooltip shows "Delete Stage" or "Delete Objective"
- [ ] Hover over ➕ button → **Expected**: Tooltip shows "Add Objective"
- [ ] Modal dialogs are clearly titled
- [ ] Modal dialogs are appropriately sized (not too small, not too large)
- [ ] Form fields are labeled clearly
- [ ] Save/Cancel buttons are obvious and well-positioned
- [ ] No UI glitches (overlapping text, cutoff fields, etc.)

## Expected Results

✅ **All checkboxes above should pass**

If any test fails:

1. Note which test failed
2. Describe what happened vs. what was expected
3. Check console output for errors: `RUST_LOG=debug cargo run --package campaign_builder --bin campaign-builder`
4. Report issue with test number and description

## Known Limitations (Phase 7.1)

These are expected and not test failures:

- ❌ No undo/redo for quest edits (planned for future phase)
- ❌ No confirmation dialog for delete operations (delete is immediate)
- ❌ Objective Add button (➕) may not be fully wired yet (backend method exists)
- ❌ No visual indicator showing which item is currently being edited
- ❌ No keyboard shortcuts (Enter to save, Esc to cancel)
- ❌ Field validation is minimal (strings accepted everywhere)
- ❌ Cannot reorder stages or objectives yet (must delete and recreate)

## Performance Expectations

The quest editor should remain responsive with:

- ✅ 10+ quests in a campaign
- ✅ 5+ stages per quest
- ✅ 10+ objectives per stage
- ✅ Rapid edit/save cycles (modal opens quickly)

If performance is sluggish, note:

- Number of quests in campaign
- Number of stages/objectives
- Hardware specs (CPU, RAM, GPU/integrated graphics)

## Reporting Issues

When reporting issues found during testing:

1. **Test Number**: Which test failed (e.g., "Test 4: Objective Type Conversion")
2. **Steps to Reproduce**: Exact steps taken
3. **Expected**: What should have happened
4. **Actual**: What actually happened
5. **Environment**: OS, Rust version, egui version
6. **Logs**: Console output if errors present

## Success Criteria

Phase 7.1 is considered fully functional if:

- ✅ All 10 tests pass without errors
- ✅ Changes persist after save/reload
- ✅ All 7 objective types work correctly
- ✅ Modal dialogs open/close smoothly
- ✅ UI is responsive and user-friendly
- ✅ No crashes or panics during testing

---

**Testing completed by**: ******\_\_\_******
**Date**: ******\_\_\_******
**Test result**: ☐ Pass ☐ Fail (with notes)
**Notes**:
