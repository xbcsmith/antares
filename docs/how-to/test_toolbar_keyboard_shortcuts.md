# How to Test Toolbar Keyboard Shortcuts

**Version:** 1.0 | **Date:** 2025-01-28 | **Status:** Active

## Overview

This guide provides step-by-step instructions for testing the keyboard shortcuts
implemented in Phase 4 of the Campaign Builder UI completion plan. These shortcuts
enhance usability and provide power users with efficient keyboard navigation.

## Prerequisites

- Campaign Builder application running
- Test campaign directory with sample data
- Keyboard with standard modifier keys (Ctrl, Shift, Delete)
- Optional: High-DPI display for scaling tests

## Quick Reference Card

### EditorToolbar Shortcuts

| Action   | Shortcut      | Description                |
|----------|---------------|----------------------------|
| New      | Ctrl+N        | Create new entry           |
| Save     | Ctrl+S        | Save to campaign           |
| Load     | Ctrl+L        | Load from file             |
| Import   | Ctrl+Shift+I  | Import from RON text       |
| Export   | Ctrl+Shift+E  | Export to file             |
| Reload   | F5            | Reload from campaign       |

### ActionButtons Shortcuts

| Action     | Shortcut | Description              |
|------------|----------|--------------------------|
| Edit       | Ctrl+E   | Edit selected item       |
| Delete     | Delete   | Delete selected item     |
| Duplicate  | Ctrl+D   | Duplicate selected item  |

## Test Procedures

### Test Suite 1: EditorToolbar Keyboard Shortcuts

#### Test 1.1: New Entry (Ctrl+N)

**Purpose:** Verify Ctrl+N creates a new entry in all editors

**Steps:**

1. Open Campaign Builder
2. Navigate to Items Editor
3. Press `Ctrl+N`
4. **Expected:** Edit form appears for new item
5. Cancel the edit
6. Repeat for each editor:
   - Classes Editor
   - Races Editor
   - Spells Editor
   - Monsters Editor
   - Characters Editor
   - Maps Editor
   - Quests Editor
   - Dialogue Editor
   - Conditions Editor

**Pass Criteria:** Ctrl+N creates new entry in all 10 editors

---

#### Test 1.2: Save to Campaign (Ctrl+S)

**Purpose:** Verify Ctrl+S saves changes to campaign directory

**Steps:**

1. Open Items Editor
2. Create a new item with name "Test Sword"
3. Press `Ctrl+S`
4. **Expected:** Status message shows "Saved X items"
5. Close and reopen Campaign Builder
6. Open same campaign
7. **Expected:** "Test Sword" appears in items list

**Pass Criteria:** Data persists after save and reload

---

#### Test 1.3: Load from File (Ctrl+L)

**Purpose:** Verify Ctrl+L opens file picker dialog

**Steps:**

1. Open Items Editor
2. Press `Ctrl+L`
3. **Expected:** File picker dialog opens
4. **Expected:** Dialog filters for .ron files
5. Cancel the dialog
6. Verify no changes occurred

**Pass Criteria:** File dialog opens with correct filter

---

#### Test 1.4: Import RON (Ctrl+Shift+I)

**Purpose:** Verify Ctrl+Shift+I opens import dialog

**Steps:**

1. Open Items Editor
2. Press `Ctrl+Shift+I`
3. **Expected:** Import dialog modal opens
4. **Expected:** Text area is empty
5. Close dialog with Cancel button

**Pass Criteria:** Import dialog opens correctly

---

#### Test 1.5: Export to File (Ctrl+Shift+E)

**Purpose:** Verify Ctrl+Shift+E opens save file dialog

**Steps:**

1. Open Items Editor with at least 1 item
2. Press `Ctrl+Shift+E`
3. **Expected:** Save file dialog opens
4. **Expected:** Default filename is "items.ron"
5. Choose location and save
6. Open saved file in text editor
7. **Expected:** Valid RON format with item data

**Pass Criteria:** Export creates valid RON file

---

#### Test 1.6: Reload from Campaign (F5)

**Purpose:** Verify F5 reloads data from campaign directory

**Steps:**

1. Open Items Editor
2. Note current item count
3. Add a new item (don't save)
4. Note item count increased
5. Press `F5`
6. **Expected:** Item count returns to original
7. **Expected:** Status message shows "Reloaded from: [path]"
8. **Expected:** Unsaved item is gone

**Pass Criteria:** Reload discards unsaved changes

---

### Test Suite 2: ActionButtons Keyboard Shortcuts

#### Test 2.1: Edit Selected (Ctrl+E)

**Purpose:** Verify Ctrl+E starts editing selected item

**Steps:**

1. Open Items Editor
2. Click to select an item in the list
3. Press `Ctrl+E`
4. **Expected:** Right panel shows edit form for selected item
5. **Expected:** Form fields populated with item data
6. Cancel edit
7. Select different item
8. Press `Ctrl+E` again
9. **Expected:** Edit form shows newly selected item

**Pass Criteria:** Ctrl+E edits the correct item

---

#### Test 2.2: Delete Selected (Delete Key)

**Purpose:** Verify Delete key removes selected item

**Steps:**

1. Open Items Editor
2. Create test item "DELETE_ME"
3. Save to campaign
4. Select "DELETE_ME" in list
5. Press `Delete` key
6. **Expected:** Confirmation dialog appears (if implemented)
7. Confirm deletion
8. **Expected:** Item removed from list
9. **Expected:** Status indicates item deleted

**Pass Criteria:** Delete key removes selected item

---

#### Test 2.3: Duplicate Selected (Ctrl+D)

**Purpose:** Verify Ctrl+D duplicates selected item

**Steps:**

1. Open Items Editor
2. Select item "Longsword"
3. Press `Ctrl+D`
4. **Expected:** New item appears with name "Longsword (copy)"
5. **Expected:** New item has unique ID
6. **Expected:** All other properties match original
7. Edit duplicated item
8. **Expected:** Original remains unchanged

**Pass Criteria:** Ctrl+D creates independent copy

---

### Test Suite 3: Tooltip Verification

#### Test 3.1: Toolbar Button Tooltips

**Purpose:** Verify all toolbar buttons show shortcuts in tooltips

**Steps:**

1. Open any editor
2. Hover mouse over "➕ New" button
3. Wait 1 second
4. **Expected:** Tooltip shows "Create new entry (Ctrl+N)"
5. Repeat for each toolbar button:
   - 💾 Save → "(Ctrl+S)"
   - 📂 Load → "(Ctrl+L)"
   - 📥 Import → "(Ctrl+Shift+I)"
   - 📋 Export → "(Ctrl+Shift+E)"
   - 🔄 Reload → "(F5)"

**Pass Criteria:** All tooltips display correct shortcuts

---

#### Test 3.2: Action Button Tooltips

**Purpose:** Verify action buttons show shortcuts in tooltips

**Steps:**

1. Open Items Editor
2. Select an item
3. Hover over "✏️ Edit" button
4. **Expected:** Tooltip shows "Edit selected item (Ctrl+E)"
5. Hover over "🗑️ Delete" button
6. **Expected:** Tooltip shows "Delete selected item (Delete)"
7. Hover over "📋 Duplicate" button
8. **Expected:** Tooltip shows "Duplicate selected item (Ctrl+D)"

**Pass Criteria:** All action button tooltips correct

---

### Test Suite 4: Edge Cases and Conflicts

#### Test 4.1: No Item Selected

**Purpose:** Verify shortcuts behave correctly with no selection

**Steps:**

1. Open Items Editor
2. Ensure no item is selected in list
3. Press `Ctrl+E`
4. **Expected:** No action occurs (no item to edit)
5. Press `Delete`
6. **Expected:** No action occurs (no item to delete)
7. Press `Ctrl+D`
8. **Expected:** No action occurs (no item to duplicate)

**Pass Criteria:** Shortcuts gracefully handle no selection

---

#### Test 4.2: During Edit Mode

**Purpose:** Verify shortcuts work correctly during editing

**Steps:**

1. Open Items Editor
2. Press `Ctrl+N` to create new item
3. Type "Test" in name field
4. Press `Ctrl+S` (while still in edit mode)
5. **Expected:** Save action triggers
6. **Expected:** Campaign saved with current edit state

**Pass Criteria:** Shortcuts work during active editing

---

#### Test 4.3: Text Input Focus

**Purpose:** Verify shortcuts don't interfere with text input

**Steps:**

1. Open Items Editor
2. Press `Ctrl+N` to create new item
3. Click in "Name" text field
4. Type "Longsword"
5. Press `Ctrl+A` (standard select all)
6. **Expected:** Text in field is selected, NOT a new item created
7. Type "Shortsword" (replaces selected text)
8. **Expected:** Field now contains "Shortsword"

**Pass Criteria:** Standard text shortcuts work in input fields

---

#### Test 4.4: Search Field Focus

**Purpose:** Verify search field shortcuts work correctly

**Steps:**

1. Open Items Editor
2. Click in search field
3. Type "sword"
4. Press `Ctrl+A`
5. **Expected:** Text "sword" is selected
6. Press `Delete`
7. **Expected:** Text deleted, NOT item deleted

**Pass Criteria:** Search field doesn't trigger toolbar shortcuts

---

### Test Suite 5: High-DPI and Scaling

#### Test 5.1: Button Rendering at 1.5x Scale

**Purpose:** Verify buttons render correctly at 1.5x display scaling

**Steps:**

1. Set system display scaling to 150%
2. Launch Campaign Builder
3. Open Items Editor
4. Observe toolbar buttons
5. **Expected:** Buttons are crisp and clear
6. **Expected:** Emojis scale proportionally
7. **Expected:** Text is readable
8. Click each button
9. **Expected:** All buttons respond to clicks

**Pass Criteria:** UI scales cleanly to 1.5x

---

#### Test 5.2: Button Rendering at 2.0x Scale

**Purpose:** Verify buttons render correctly at 2.0x display scaling

**Steps:**

1. Set system display scaling to 200%
2. Launch Campaign Builder
3. Open Items Editor
4. Observe toolbar buttons
5. **Expected:** Buttons are crisp and clear
6. **Expected:** No pixelation or blurring
7. Test all keyboard shortcuts
8. **Expected:** Shortcuts work at 2.0x scale

**Pass Criteria:** UI scales cleanly to 2.0x

---

#### Test 5.3: Narrow Window Wrapping

**Purpose:** Verify toolbar wraps gracefully in narrow windows

**Steps:**

1. Open Campaign Builder
2. Resize window to minimum width (300px)
3. Open Items Editor
4. **Expected:** Toolbar buttons wrap to multiple rows
5. **Expected:** All buttons remain visible
6. **Expected:** No button clipping occurs
7. Test `Ctrl+N` shortcut
8. **Expected:** Shortcut works even when button wrapped

**Pass Criteria:** Toolbar wraps without losing functionality

---

### Test Suite 6: Cross-Platform Verification

#### Test 6.1: Windows Keyboard Shortcuts

**Platform:** Windows 10/11

**Steps:**

1. Test all EditorToolbar shortcuts (Suite 1)
2. Test all ActionButtons shortcuts (Suite 2)
3. **Expected:** All shortcuts work with Ctrl key

**Pass Criteria:** All shortcuts functional on Windows

---

#### Test 6.2: macOS Keyboard Shortcuts

**Platform:** macOS 12+

**Steps:**

1. Test all EditorToolbar shortcuts (Suite 1)
2. Use Cmd key instead of Ctrl
3. **Expected:** Cmd+N creates new entry
4. **Expected:** Cmd+S saves to campaign
5. Test all other shortcuts with Cmd
6. **Expected:** All shortcuts work with Cmd key

**Pass Criteria:** All shortcuts functional on macOS with Cmd

---

#### Test 6.3: Linux Keyboard Shortcuts

**Platform:** Ubuntu 22.04 or similar

**Steps:**

1. Test all EditorToolbar shortcuts (Suite 1)
2. Test all ActionButtons shortcuts (Suite 2)
3. Test on both X11 and Wayland
4. **Expected:** All shortcuts work on both display servers

**Pass Criteria:** All shortcuts functional on Linux

---

## Test Results Template

### Test Execution Record

**Date:** ____________
**Tester:** ____________
**Platform:** ____________
**Display Scaling:** ____________
**Campaign Builder Version:** ____________

| Test ID | Test Name                      | Result | Notes |
|---------|--------------------------------|--------|-------|
| 1.1     | New Entry (Ctrl+N)             | ☐ Pass ☐ Fail |       |
| 1.2     | Save to Campaign (Ctrl+S)      | ☐ Pass ☐ Fail |       |
| 1.3     | Load from File (Ctrl+L)        | ☐ Pass ☐ Fail |       |
| 1.4     | Import RON (Ctrl+Shift+I)      | ☐ Pass ☐ Fail |       |
| 1.5     | Export to File (Ctrl+Shift+E)  | ☐ Pass ☐ Fail |       |
| 1.6     | Reload from Campaign (F5)      | ☐ Pass ☐ Fail |       |
| 2.1     | Edit Selected (Ctrl+E)         | ☐ Pass ☐ Fail |       |
| 2.2     | Delete Selected (Delete)       | ☐ Pass ☐ Fail |       |
| 2.3     | Duplicate Selected (Ctrl+D)    | ☐ Pass ☐ Fail |       |
| 3.1     | Toolbar Button Tooltips        | ☐ Pass ☐ Fail |       |
| 3.2     | Action Button Tooltips         | ☐ Pass ☐ Fail |       |
| 4.1     | No Item Selected               | ☐ Pass ☐ Fail |       |
| 4.2     | During Edit Mode               | ☐ Pass ☐ Fail |       |
| 4.3     | Text Input Focus               | ☐ Pass ☐ Fail |       |
| 4.4     | Search Field Focus             | ☐ Pass ☐ Fail |       |
| 5.1     | Rendering at 1.5x Scale        | ☐ Pass ☐ Fail |       |
| 5.2     | Rendering at 2.0x Scale        | ☐ Pass ☐ Fail |       |
| 5.3     | Narrow Window Wrapping         | ☐ Pass ☐ Fail |       |

**Overall Pass Rate:** _____ / 17 tests passed

**Critical Issues Found:** (List any blocking issues)

**Minor Issues Found:** (List any non-blocking issues)

**Recommendations:** (Suggested improvements)

---

## Troubleshooting

### Shortcut Not Working

**Symptom:** Pressing keyboard shortcut has no effect

**Possible Causes:**

1. Another application intercepted the shortcut
2. Focus is in a text input field (expected behavior for some shortcuts)
3. Modal dialog is open (shortcuts may be disabled)
4. Platform-specific key mapping issue

**Solutions:**

1. Close other applications and retry
2. Click outside text fields before using shortcuts
3. Close any open dialogs
4. Try alternative shortcuts (e.g., menu buttons)

### Tooltip Not Appearing

**Symptom:** Hovering over button doesn't show tooltip

**Possible Causes:**

1. Hovering too briefly (tooltips have ~1 second delay)
2. System tooltip settings disabled
3. Mouse moved away before tooltip appeared

**Solutions:**

1. Hover for 2-3 seconds
2. Check system accessibility settings
3. Keep mouse steady while hovering

### Wrong Item Edited/Deleted

**Symptom:** Shortcut affects wrong item

**Possible Causes:**

1. Selection changed after pressing shortcut
2. Multiple items partially selected
3. UI state not updated

**Solutions:**

1. Ensure only one item is selected
2. Click item explicitly before using shortcut
3. Restart Campaign Builder if state corruption suspected

---

## Automated Testing Notes

While keyboard shortcuts cannot be easily unit tested in egui without a full
rendering context, the following integration tests should be added:

1. **Shortcut Registration Tests** - Verify shortcuts are registered
2. **Conflict Detection Tests** - Verify no shortcut conflicts
3. **Tooltip Content Tests** - Verify tooltip text matches shortcuts
4. **Button Label Tests** - Verify consistent button labels

See `sdk/campaign_builder/src/ui_helpers.rs` tests for documentation tests.

---

## Success Criteria

**Phase 4 is considered complete when:**

- [ ] All 17 tests pass on primary development platform
- [ ] No critical issues found
- [ ] Tooltips display correctly on all buttons
- [ ] Shortcuts work at 1.5x and 2.0x scaling
- [ ] Cross-platform verification complete (Windows, macOS, Linux)
- [ ] Documentation updated with keyboard shortcuts
- [ ] User feedback collected and addressed

---

## References

- Phase 4 Implementation: `docs/explanation/implementations.md`
- UI Helpers Source: `sdk/campaign_builder/src/ui_helpers.rs`
- Campaign Builder Plan: `docs/explanation/campaign_builder_ui_completion_plan.md`
