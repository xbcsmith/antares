# Campaign Builder UI Completion Plan

## Overview

This plan completes Tasks 8.6, 8.7, and 8.8 from Phase 8 of the Phase 6 Cleanup Plan, ensuring full UI parity with CLI tools and consistent editor layouts across the Campaign Builder. The primary goal is to make the UI capable of doing everything the CLIs can do and more, while maintaining visual consistency and usability.

## Current State Analysis

### Existing Infrastructure

**Completed Editors (Reference Implementations):**

- ✅ Monsters Editor - Full two-column layout, toolbar, action buttons
- ✅ Items Editor - Complete CRUD operations, consistent UI
- ✅ Spells Editor - Full editing capabilities via UI
- ✅ Classes Editor - Updated in Phase 3 with proficiency system
- ✅ Dialogues Editor - Enhanced preview and list display (Task 8.3) - Verified working correctly
- 🔨 Characters Editor - Uses TwoColumnLayout but ActionButtons in wrong panel (needs fix)
- ✅ Quest Editor - Uses TwoColumnLayout component

**Shared UI Components Available:**

- `TwoColumnLayout` - Standard left/right panel layout
- `EditorToolbar` - Consistent toolbar with New/Save/Load/Import/Export
- `ActionButtons` - Standard Edit/Delete/Duplicate action buttons
- `ItemAction` enum - Standard action types

**CLI Tools Present:**

- `src/bin/race_editor.rs` - Full race CRUD with interactive prompts
- Other CLI editors may exist for conditions, quests, etc.

### Identified Issues

**1. Races Editor Issue (Critical)**

- SDK validation panel displays: "Races configured via races_file - use Race Editor CLI to manage"
- This message is INCORRECT - the Races Editor UI is fully functional
- Line 803-806 in `sdk/campaign_builder/src/main.rs` needs update
- User perception issue: UI appears incomplete when it's actually feature-complete

**2. Layout Consistency Gaps**

- Conditions Editor ✅ - Verified working correctly, uses TwoColumnLayout and follows standard pattern
- Dialogues Editor ✅ - Verified working correctly, no changes needed
- Quest Editor ✅ - Already uses TwoColumnLayout
- Characters Editor 🔨 - ActionButtons (Edit/Delete/Duplicate/Export) are in left panel list instead of right panel display
- Maps Editor ✅ - Follows pattern correctly, just has horizontal padding bug causing right panel cutoff at default width

**3. Toolbar Consistency**

- Some editors may have verbose button labels ("New Item" vs "New")
- Need to verify all use EditorToolbar component consistently
- Keyboard shortcuts not documented in tooltips

**4. Feature Parity with CLI**

- Race Editor CLI provides: List, Add, Edit, Delete, Preview, Save
- Races Editor UI provides: List (with search), Add, Edit, Delete, Save, Load, Export
- UI actually has MORE features (search filter, visual preview, merge mode)
- Items, Spells, and Monsters already have working Import with merge logic that can be reused
- Need to verify other editors have feature parity

## Implementation Phases

### Phase 1: Fix Races Editor Messaging (Critical - Quick Win)

**Objective:** Remove incorrect "use CLI" message and verify Races Editor UI is fully functional.

#### 1.1 Update Validation Panel Message

**File to modify:** `sdk/campaign_builder/src/main.rs`

**Changes:**

- Line 803-806: Replace CLI reference with accurate UI status message
- Change from: "Races configured via races_file - use Race Editor CLI to manage"
- Change to: "Races loaded and editable in Races Editor tab"
- OR better: Check if races are actually loaded and provide helpful status
- Consider showing race count: "N races loaded from races_file"

#### 1.2 Verify Races Editor Feature Completeness

**File to review:** `sdk/campaign_builder/src/races_editor.rs`

**Verification checklist:**

- [ ] New race creation works (start_new_race)
- [ ] Edit existing race works (start_edit_race)
- [ ] Delete race works (delete_race)
- [ ] Save to file works (save_to_file)
- [ ] Load from file works (load_from_file)
- [ ] Search/filter works (filtered_races)
- [ ] All race fields editable (StatModifiers, Resistances, SizeCategory, special_abilities, proficiencies, incompatible_item_tags)
- [ ] Validation prevents invalid data (empty id/name, duplicate ids)

#### 1.3 Add Missing Features (If Any)

**Required features:**

- Import functionality using existing pattern from Items/Spells/Monsters editors (reuse merge logic)
- Proficiency picker UI with STANDARD_PROFICIENCY_IDS constants from race_editor CLI
- Item tag picker UI with STANDARD_ITEM_TAGS constants from race_editor CLI

**Optional enhancements:**

- Preview mode like the CLI has (preview_race function)

#### 1.4 Testing Requirements

**Manual tests:**

- Create new race with all fields populated
- Edit existing race and verify changes saved
- Delete race and verify removal
- Load external .ron file with merge on/off
- Export races to new file
- Verify search filter works
- Test validation messages for invalid input

**Automated tests:**

- Existing tests in `races_editor.rs` lines 1079-1303 already cover:
  - State creation, new race, save, validation, delete, cancel, filtering
- Add test for next_available_race_id if not present
- Add test for import once implemented

#### 1.5 Deliverables

- [ ] Updated validation message in main.rs
- [ ] Verified all CRUD operations work
- [ ] Import feature implemented using Items/Spells/Monsters pattern
- [ ] Proficiency picker implemented with checkboxes/multiselect
- [ ] Item tag picker implemented with checkboxes/multiselect
- [ ] Preview mode added (optional enhancement)
- [ ] Manual test checklist completed
- [ ] All automated tests pass

#### 1.6 Success Criteria

- Assets/validation panel no longer tells users to use CLI
- Races Editor UI can do everything CLI can do
- New features documented in implementations.md
- Zero compiler warnings or errors
- User can confidently edit races entirely within UI

---

### Phase 2: Editor Layout Consistency Audit

**Objective:** Verify all editors follow the established pattern and identify gaps.

#### 2.1 Define Reference Pattern

**Standard Editor Pattern (from quality_of_life_improvements.md):**

```
┌────────────────────────────────────────────────────────────────┐
│ 📋 Editor Heading                                              │
├────────────────────────────────────────────────────────────────┤
│ [EditorToolbar with New/Save/Load/Import/Export]               │
│ [Search filter] [Merge mode checkbox] [Count]                  │
├──────────────┬─────────────────────────────────────────────────┤
│ LEFT PANEL   │ RIGHT PANEL                                     │
│              │  [ActionButtons: Edit/Delete/Duplicate/Export]  │
│ - Item list  │ - Details/form for selected item                │
│ - Scrollable │ - OR empty state prompt                         │
│ - Selectable │ - OR creation form                              │
│              │ - OR creation form                              │
│              │                                                 │
│              │                                                 │
└──────────────┴─────────────────────────────────────────────────┘

```

**Required Components:**

- Uses `TwoColumnLayout::new(id).show()` or `.show_split()`
- Uses `EditorToolbar::new(name).show()` for toolbar
- Uses `ActionButtons::new().with_edit().with_delete().show()` for item actions
- Left panel: item list with selection state
- Right panel: detail view or edit form
- Proper mode handling (List/Creating/Editing)

#### 2.2 Audit Each Editor

**Create checklist for each editor:**

| Editor     | TwoColumnLayout | EditorToolbar | ActionButtons | Mode Enum | Form | Notes                   |
| ---------- | --------------- | ------------- | ------------- | --------- | ---- | ----------------------- |
| Monsters   | ✅              | ✅            | ✅            | ✅        | ✅   | Reference               |
| Items      | ✅              | ✅            | ✅            | ✅        | ✅   | Reference               |
| Spells     | ✅              | ✅            | ✅            | ✅        | ✅   | Reference               |
| Classes    | ✅              | ✅            | ✅            | ✅        | ✅   | Updated Phase 3         |
| Races      | ✅              | ✅            | ✅            | ✅        | ✅   | Verify Phase 1          |
| Characters | ✅              | ✅            | 🔨            | ✅        | ✅   | **Buttons wrong panel** |
| Dialogues  | ✅              | ✅            | ✅            | ✅        | ✅   | **Verified working**    |
| Quest      | ✅              | ✅            | ✅            | ✅        | ✅   | Already uses components |
| Conditions | ✅              | ✅            | ✅            | ✅        | ✅   | **Verified working**    |
| Maps       | ✅              | ✅            | ✅            | ✅        | ✅   | **Padding fix needed**  |

**Files to audit:**

- `sdk/campaign_builder/src/conditions_editor.rs` - ✅ Verified working correctly, no changes needed
- `sdk/campaign_builder/src/dialogues_editor.rs` - ✅ Verified working correctly, no changes needed
- `sdk/campaign_builder/src/characters_editor.rs` - 🔨 Move ActionButtons from left panel to right panel
- `sdk/campaign_builder/src/map_editor.rs` - Fix horizontal padding causing right panel cutoff

#### 2.3 Document Gaps

**For each non-compliant editor, document:**

- What components are missing
- What custom UI code could be replaced with shared components
- Estimated effort (small/medium/large)
- Breaking changes if any

#### 2.4 Testing Requirements

**Manual verification:**

- Open each editor tab
- Verify visual consistency
- Check toolbar buttons present and working
- Verify two-column layout renders properly
- Test action buttons (Edit/Delete/Duplicate)

#### 2.5 Deliverables

- [ ] Completed audit checklist for all editors
- [ ] Conditions Editor verified as working correctly (no changes needed)
- [ ] Dialogues Editor verified as working correctly (no changes needed)
- [ ] Characters Editor button placement issue documented
- [ ] Maps Editor padding issue documented
- [ ] Priority list for Phase 3 updates

#### 2.6 Success Criteria

- Clear understanding of which editors need updates
- Documented pattern deviations with reasons
- Plan for Phase 3 work

---

### Phase 3: Fix Characters Editor & Maps Editor

**Objective:** Move ActionButtons to correct panel in Characters Editor and fix Maps Editor horizontal padding. Conditions and Dialogues Editors already verified working.

#### 3.1 Verified Editors - No Changes Needed

**Conditions Editor:** `sdk/campaign_builder/src/conditions_editor.rs`

**Status:** ✅ Manually verified working correctly

- Uses TwoColumnLayout properly
- Uses EditorToolbar component
- Uses ActionButtons component in correct location
- Mode enum implemented correctly
- Form layout consistent with standard pattern
- Search filter integrated

**Dialogues Editor:** `sdk/campaign_builder/src/dialogue_editor.rs`

**Status:** ✅ Manually verified working correctly

- Uses TwoColumnLayout properly
- Uses EditorToolbar component
- Uses ActionButtons component in correct location
- Enhanced preview and list display (Task 8.3)
- Follows standard pattern correctly

**Conclusion:** No edits required for Conditions or Dialogues Editors.

#### 3.2 Fix Characters Editor ActionButtons Placement

**File:** `sdk/campaign_builder/src/characters_editor.rs`

**Issue:** ActionButtons (Edit/Delete/Duplicate) are in left panel list (lines 817-825) instead of right panel preview.

**Current behavior:**

- Left panel shows character list with inline ActionButtons after each character name
- Right panel shows character preview but no action buttons

**Expected behavior (matching Items/Spells/Monsters editors):**

- Left panel shows character list (selectable labels only)
- Right panel shows character preview WITH ActionButtons at top

**Fix approach:**

1. **Remove ActionButtons from left panel** (line 817-825 in `show_list` method)

   - Keep only the selectable label with character name/race/class
   - Remove the horizontal UI block with ActionButtons

2. **Add ActionButtons to right panel** (in `show_character_preview` method)

   - Add ActionButtons after character heading, similar to Items Editor pattern
   - Place buttons between heading and separator
   - Handle Edit/Delete/Duplicate actions in right panel context

3. **Reference implementation:** Check `items_editor.rs` lines 467-477 for correct pattern

**Testing:**

- Verify buttons appear in right panel when character selected
- Test Edit/Delete/Duplicate actions still work
- Verify left panel is cleaner with just selectable list items

#### 3.3 Fix Maps Editor Horizontal Padding

**File:** `sdk/campaign_builder/src/map_editor.rs`

**Issue:** Right panel gets cut off at default UI width due to horizontal padding miscalculation.

**Root cause:** Lines 1571-1601 compute left column width with complex logic involving:

- `total_width`, `sep_margin`, `inspector_min_width`
- `compute_left_column_width()` helper function
- Display config ratios

**Fix approach:**

- Review padding/margin calculations in TwoColumnLayout usage
- Verify `sep_margin = 12.0` is appropriate
- Ensure `inspector_min_width` leaves enough space for right panel content
- Test at various window widths (narrow, default, wide)
- May need to adjust `display_config.left_column_max_ratio` or minimum widths

#### 3.4 Apply Fixes

**For Characters Editor:**

- Move ActionButtons from left panel (remove lines 817-825) to right panel (add to show_character_preview)
- Follow Items Editor pattern for button placement
- Test all button actions work correctly

**For Maps Editor:**

- Fix padding/margin calculations in TwoColumnLayout usage
- Test at multiple window widths
- Verify grid rendering unaffected
- Update tests if needed
- Run quality checks

#### 3.5 Testing Requirements

**Characters Editor testing:**

- Verify ActionButtons appear in right panel only
- Test Edit button loads character into edit form
- Test Delete button removes character
- Test Duplicate button creates copy with new ID
- Verify left panel is cleaner without inline buttons
- Test selection still works correctly

**Maps Editor testing:**

- Test at narrow, default, and wide window widths
- Verify right panel inspector not cut off
- Verify grid rendering works correctly
- Test zoom controls still function
- Verify left/right panel separation visible

**General testing:**

- Manual test all CRUD operations in both editors
- Verify toolbar actions work
- Verify layout renders correctly on various window sizes
- Test with empty state
- Test with many items (scrolling)

**Regression testing:**

- Ensure existing functionality not broken
- Verify saved files still load correctly
- Test merge mode behavior
- Verify validation still works

#### 3.6 Deliverables

- [ ] Conditions Editor verified working (no changes)
- [ ] Dialogues Editor verified working (no changes)
- [ ] Characters Editor ActionButtons moved to right panel
- [ ] Maps Editor horizontal padding fixed
- [ ] All editors confirmed to follow standard pattern
- [ ] Tests pass for Characters and Maps Editor changes
- [ ] Documentation updated

#### 3.7 Success Criteria

- Visual consistency across all applicable editors
- User can switch between editors and see familiar patterns
- No functional regressions
- All quality checks pass

---

### Phase 4: Toolbar Consistency (Task 8.7)

**Objective:** Ensure all editor toolbars use consistent buttons, labels, and shortcuts.

#### 4.1 Audit Toolbar Implementation

**Files to check:** All `*_editor.rs` files in `sdk/campaign_builder/src/`

**Verification checklist per editor:**

- [ ] Uses `EditorToolbar` component
- [ ] Has standard buttons: ➕New, 💾Save, 📂Load, 📥Import, 📋Export
- [ ] Button labels are concise (not "New Item", just "New")
- [ ] Toolbar scales properly to window width
- [ ] Merge checkbox present for Load action
- [ ] Search filter integrated where appropriate

**Common issues to fix:**

- Verbose labels: "New Monster" → "New"
- Missing buttons: Some editors may not have Import/Export
- Inconsistent order: Buttons should be in standard order
- Custom toolbar code instead of EditorToolbar component

#### 4.2 Standardize Button Labels

**Pattern to enforce:**

- "➕ New" (not "New Item", "Add New", etc.)
- "💾 Save" (not "Save All", "Save File")
- "📂 Load" (not "Open", "Load File")
- "📥 Import" (not "Import Data")
- "📋 Export" (not "Export Data", "Save As")

**Update EditorToolbar component if needed:**

- File: `sdk/campaign_builder/src/ui_helpers.rs`
- Ensure standard labels are used
- Consider making labels configurable but with sensible defaults

#### 4.3 Add Keyboard Shortcuts

**Standard shortcuts to implement:**

- Ctrl+N / Cmd+N → New
- Ctrl+S / Cmd+S → Save
- Ctrl+O / Cmd+O → Load
- Ctrl+E / Cmd+E → Export
- Ctrl+Delete / Cmd+Delete → Delete selected

**Implementation approach:**

- Add keyboard shortcut handling to EditorToolbar
- Display shortcuts in button tooltips: "New (Ctrl+N)"
- Handle platform differences (Ctrl on Windows/Linux, Cmd on macOS)
- Ensure shortcuts don't conflict with system shortcuts

**Files to modify:**

- `sdk/campaign_builder/src/ui_helpers.rs` - Add shortcut handling to EditorToolbar
- Individual `*_editor.rs` files - Wire up shortcut actions if not using EditorToolbar

#### 4.4 Toolbar Scaling

**Ensure toolbar buttons:**

- Don't overflow on narrow windows
- Use wrapping or horizontal scroll if needed
- Maintain readable size on all screen sizes
- Consider collapsing to icon-only on very narrow screens

#### 4.5 Testing Requirements

**Manual testing:**

- Test each editor toolbar
- Verify all buttons present and working
- Test keyboard shortcuts in each editor
- Test on various window sizes
- Verify tooltips show shortcuts

**Accessibility testing:**

- Verify keyboard navigation works
- Ensure screen reader announces buttons correctly
- Test high contrast mode compatibility

#### 4.6 Deliverables

- [ ] All editors use EditorToolbar component
- [ ] Consistent button labels across all editors
- [ ] Keyboard shortcuts implemented
- [ ] Shortcuts documented in tooltips
- [ ] Toolbar scales properly
- [ ] Documentation updated with keyboard shortcuts

#### 4.7 Success Criteria

- User sees identical toolbar pattern in every editor
- Muscle memory for shortcuts works across editors
- Toolbar usable on both large and small screens
- Zero custom toolbar implementations (except Maps if justified)

---

### Phase 5: Comprehensive Testing and Documentation (Task 8.8)

**Objective:** Thoroughly test all editors and document the completed UI improvements.

#### 5.1 Manual Testing Protocol

**Create testing checklist for EACH editor:**

**Layout and Visual Consistency:**

- [ ] Editor uses two-column layout (left: list, right: details/form)
- [ ] Toolbar present at top with standard buttons
- [ ] Action buttons present for selected items
- [ ] Visual spacing and padding consistent with other editors
- [ ] Text sizes and fonts consistent
- [ ] Icons and emojis used consistently

**Functionality Testing:**

- [ ] New: Creates new item with fresh ID
- [ ] Edit: Loads item data into form, saves changes
- [ ] Delete: Removes item after confirmation
- [ ] Save: Writes to campaign file successfully
- [ ] Load: Opens file dialog, loads data (with merge option)
- [ ] Import: Imports data from external file
- [ ] Export: Exports to external file with file dialog
- [ ] Search/Filter: Filters list correctly

**Edge Cases:**

- [ ] Works with empty list (shows helpful prompt)
- [ ] Works with 1 item
- [ ] Works with 100+ items (scrolling, performance)
- [ ] Handles invalid data gracefully (validation messages)
- [ ] Prevents duplicate IDs
- [ ] Handles file load errors (shows error message)
- [ ] Handles file save errors (shows error message)
- [ ] Canceling edit restores previous state

**Integration Testing:**

- [ ] Auto-loads when campaign selected
- [ ] Shows in assets panel correctly
- [ ] Validation panel shows relevant checks
- [ ] Changes mark campaign as unsaved
- [ ] Switching tabs preserves editor state

#### 5.2 Automated Testing

**Add integration tests if missing:**

**Test file:** `sdk/campaign_builder/tests/editor_integration_tests.rs` (create if needed)

**Test scenarios:**

- Create item, verify it appears in list
- Edit item, verify changes persist
- Delete item, verify removal
- Load/save round-trip preserves data
- Merge mode combines data correctly
- Search filter works correctly
- Validation catches invalid data

**Run existing test suites:**

```bash
cargo test --all-features
cargo test --package campaign-builder
cargo test --lib races_editor
cargo test --lib conditions_editor
# etc for each editor
```

#### 5.3 Asset and Validation Panel Testing

**Asset Panel Tests:**

- [ ] All campaign data files show as "📁 Data File" (not "Unused")
- [ ] Referenced assets show as "✅ Referenced"
- [ ] Truly unreferenced files show as "⚠️ Unreferenced"
- [ ] Asset counts accurate for all types
- [ ] File size calculations correct
- [ ] References list shows correct items (max 5 + "and N more")

**Validation Panel Tests:**

- [ ] Overall status badge correct (✅/⚠️/❌)
- [ ] Error/warning/info counts accurate
- [ ] Total checks count displayed
- [ ] Category grouping works
- [ ] Re-validate button refreshes results
- [ ] Context-aware tips show correctly
- [ ] No "use CLI" messages remain anywhere

#### 5.4 Screenshot Documentation

**Capture screenshots for each editor showing:**

- Empty state (when no items)
- List view with multiple items
- Edit/create form view
- Toolbar buttons
- Action buttons
- Search/filter in use
- Validation messages (if applicable)

**Organize screenshots:**

```
docs/images/editors/
  ├── monsters_editor_empty.png
  ├── monsters_editor_list.png
  ├── monsters_editor_form.png
  ├── items_editor_empty.png
  ├── items_editor_list.png
  ├── items_editor_form.png
  ├── races_editor_empty.png
  ├── races_editor_list.png
  ├── races_editor_form.png
  └── ... (etc for all editors)
```

#### 5.5 Update Documentation

**Files to update:**

**1. `docs/explanation/implementations.md`:**

- Add Phase 8.6, 8.7, 8.8 sections
- Document Races Editor UI parity with CLI
- Document layout consistency updates
- Document toolbar standardization
- Document keyboard shortcuts
- List all files modified

**2. `docs/explanation/phase6_cleanup_plan.md`:**

- Mark Task 8.6 substasks as completed
- Mark Task 8.7 substasks as completed
- Mark Task 8.8 substasks as completed
- Update Phase 8 status to ✅ COMPLETED

**3. Create/update `docs/how-to/campaign_builder_guide.md`:**

- Document standard editor pattern
- Explain toolbar buttons
- List keyboard shortcuts
- Show example screenshots
- Explain search/filter usage
- Document merge mode behavior

**4. Update `README.md` (if applicable):**

- Mention Campaign Builder UI improvements
- Note UI/CLI feature parity
- Link to campaign builder guide

#### 5.6 Deliverables

- [ ] Completed manual testing checklist for all editors
- [ ] Integration tests added/verified
- [ ] Screenshots captured and organized
- [ ] implementations.md updated
- [ ] phase6_cleanup_plan.md updated
- [ ] campaign_builder_guide.md created/updated
- [ ] README.md updated (if applicable)

#### 5.7 Success Criteria

- All editors pass manual testing checklist
- All automated tests pass (cargo test --all-features)
- Zero "use CLI" messages in UI
- Documentation accurately reflects current state
- Screenshots show consistent, professional UI
- User can confidently use UI for all campaign editing tasks

---

## Execution Strategy

### Recommended Order

1. **Phase 1 (Immediate - 2-4 hours):** Fix Races Editor messaging, implement Import with existing merge pattern, add proficiency/tag pickers
2. **Phase 2 (Quick - 1-2 hours):** Audit remaining editors (Conditions and Dialogues Editors already verified working)
3. **Phase 3 (Small - 2-3 hours):** Fix Characters Editor button placement and Maps Editor padding (Conditions/Dialogues need no changes)
4. **Phase 4 (Medium - 4-6 hours):** Standardize toolbars and add keyboard shortcuts
5. **Phase 5 (Thorough - 6-10 hours):** Comprehensive testing and documentation

**Total estimated effort:** 15-25 hours

### Quality Gates

**After each phase:**

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

**All must pass before proceeding to next phase.**

### Rollback Plan

- Each phase should be in separate git branch
- Commit after each substask completion
- If quality gates fail, fix or rollback specific commits
- Keep main/trunk branch always in working state

### Success Metrics

**User Experience:**

- User never sees "use CLI" messages
- User sees consistent patterns across all editors
- User can accomplish all tasks within UI
- User can learn one editor pattern and apply to all

**Technical Quality:**

- Zero compiler warnings
- Zero clippy warnings
- 100% test pass rate
- All editors use shared components where applicable

**Documentation:**

- All changes documented in implementations.md
- Campaign Builder user guide complete
- Screenshots up to date
- Keyboard shortcuts documented

## Resolved Questions

1. **Maps Editor Scope:** ✅ Maps Editor already follows standard pattern correctly. Only needs horizontal padding fix for right panel cutoff.

2. **Import Implementation:** ✅ Implement Import for Races Editor using existing merge logic pattern from Items/Spellers/Monsters editors.

3. **Proficiency/Tag Pickers:** ✅ Add visual pickers with checkboxes/multiselect for better UX using STANDARD_PROFICIENCY_IDS and STANDARD_ITEM_TAGS constants.

4. **Conditions Editor:** ✅ Manually verified working correctly. Uses all shared components properly. No changes needed.

5. **Dialogues Editor:** ✅ Manually verified working correctly. Enhanced in Task 8.3. No changes needed.

6. **Characters Editor:** ✅ Identified ActionButtons placement issue - buttons should be in right panel, not left panel list.

## Open Questions

1. **Keyboard Shortcuts:** Which shortcuts are most important? Recommend: New (Ctrl+N), Save (Ctrl+S), Delete (Delete key), Cancel (Esc) as minimum viable set.

2. **Preview Mode:** Should Races Editor have a read-only preview mode like CLI has? Recommend as optional enhancement, not required for feature parity.

## Dependencies

- All Phase 8 work depends on shared UI components being stable
- Keyboard shortcuts require egui input handling capabilities
- Screenshot documentation requires running UI in test mode
- Integration tests require campaign-builder test framework

## Risks and Mitigations

**Risk 1: Breaking existing functionality**

- **Mitigation:** Comprehensive testing before/after each change
- **Mitigation:** Git branches per phase for easy rollback

**Risk 2: Maps Editor padding fix breaks grid rendering**

- **Mitigation:** Test thoroughly at multiple window widths
- **Mitigation:** Ensure zoom and grid calculations unaffected by padding changes

**Risk 3: Keyboard shortcuts conflict with system shortcuts**

- **Mitigation:** Test on all platforms (Windows, macOS, Linux)
- **Mitigation:** Make shortcuts configurable

**Risk 4: Performance issues with large datasets**

- **Mitigation:** Test with 100+ items per editor
- **Mitigation:** Implement virtual scrolling if needed

## Conclusion

This plan provides a clear path to completing Tasks 8.6, 8.7, and 8.8, ensuring the Campaign Builder UI is consistent, feature-complete, and superior to CLI tools. By following the phased approach and quality gates, we ensure each improvement is solid before proceeding to the next.

The immediate priority (Phase 1) removes user-facing messaging that incorrectly tells users to use CLI tools, which is critical for user confidence in the UI. Subsequent phases build on this foundation to create a polished, professional campaign editing experience.
