# Event Editing in Map Editor Implementation Plan

## Overview

This plan adds full event editing capabilities to the Campaign Builder Map Editor. Currently, users can create and remove events, but cannot edit existing events. The infrastructure exists (`EventEditorState::from_map_event()`) but lacks the UI interaction layer to activate it for existing events.

## Current State Analysis

### Existing Infrastructure

**Map Editor Event System** (`sdk/campaign_builder/src/map_editor.rs`):

- `EventEditorState` struct (lines 1031-1069): Holds all event field editor state
- `EventEditorState::to_map_event()` (lines 1189-1309): Converts editor state to domain event
- `EventEditorState::from_map_event()` (lines 1334-1431): Loads event into editor (exists but unused in UI)
- `show_event_editor()` (lines 3116-3426): Renders event editor UI with type-specific fields
- `MapGridWidget` click handling (lines 1699-1712): Activates editor when PlaceEvent tool active

**Event Editor Workflow** (lines 3356-3424 in `show_event_editor()`):

```
IF existing event at position:
    Show "💾 Save Changes" button → calls map.add_event() to replace
    Show "🗑 Remove Event" button → calls remove_event()
ELSE:
    Show "➕ Add Event" button → calls add_event()
```

**Inspector Panel Event Display** (lines 2899-2923 in `show_inspector_panel()`):

- Shows event type, name, description, and type-specific details
- Only shows "🗑 Remove Event" button
- ❌ **Missing**: "✏️ Edit Event" button to activate editor

### Identified Issues

1. **No Edit Button in Inspector Panel**
   - Users see event details but cannot edit them
   - Only option is removal (destructive)
   - Must switch to PlaceEvent tool and click tile to edit (non-discoverable)

2. **Tool Palette Requirement**
   - Editing requires manual PlaceEvent tool selection
   - Workflow not intuitive (users expect click → edit)
   - Inconsistent with other editors (Items, Monsters, etc. have inline edit)

3. **No Visual Feedback for Event Being Edited**
   - When event editor is open, the map doesn't indicate which event is being edited
   - Users may lose track of which tile they're modifying
   - No distinction between "event exists" vs "event being edited" states

4. **Missing Integration Test**
   - Test `test_edit_event_replaces_existing_event` (lines 4753-4783) exists but only tests direct API
   - No test covering Inspector → Edit → Save workflow
   - No test for visual feedback during editing

## Implementation Phases

### Phase 1: Add Edit Event Button to Inspector

#### 1.1 Update Inspector Panel Event Display

**File**: `sdk/campaign_builder/src/map_editor.rs`

**Location**: `show_inspector_panel()` method, event display section (lines 2899-2923)

**Changes**:

1. Add "✏️ Edit Event" button before "🗑 Remove Event" button
2. Button click handler sets `editor.current_tool = EditorTool::PlaceEvent`
3. Button click handler creates `EventEditorState::from_map_event(pos, event)` and assigns to `editor.event_editor`
4. Add conditional styling to highlight edit button when event editor is active for this position

**Implementation**:

```rust
if let Some(event) = editor.map.get_event(pos) {
    ui.separator();
    ui.label("Event:");

    // Show Name and Description when present
    let (name, description) = Self::event_name_description(event);
    if !name.is_empty() {
        ui.label(format!("Name: {}", name));
    }
    if !description.is_empty() {
        ui.label(format!("Description: {}", description));
    }

    // Type-specific details
    match event {
        MapEvent::Encounter { monster_group, .. } => {
            ui.label(format!("Encounter: {:?}", monster_group));
        }
        // ... other event types ...
    }

    ui.horizontal(|ui| {
        // NEW: Edit Event button
        let is_editing = editor.event_editor.as_ref()
            .map(|ed| ed.position == pos)
            .unwrap_or(false);
        
        let edit_button = if is_editing {
            egui::Button::new("✏️ Editing...")
        } else {
            egui::Button::new("✏️ Edit Event")
        };

        if ui.add(edit_button).clicked() && !is_editing {
            editor.current_tool = EditorTool::PlaceEvent;
            editor.event_editor = Some(EventEditorState::from_map_event(pos, event));
        }

        if ui.button("🗑 Remove Event").clicked() {
            editor.remove_event(pos);
        }
    });
}
```

#### 1.2 Handle Tool State Consistency

**File**: `sdk/campaign_builder/src/map_editor.rs`

**Location**: `show_event_editor()` method, button handling section (lines 3356-3424)

**Changes**:

1. When "💾 Save Changes" is clicked and event is saved, optionally reset `editor.current_tool` to `EditorTool::Select`
2. When "🗑 Remove Event" is clicked, reset tool to `EditorTool::Select`
3. Add logic to clear `editor.event_editor` when switching tools away from PlaceEvent (unless explicitly requested)

**Rationale**: Provides clean workflow closure - after saving or removing, user returns to selection mode.

#### 1.3 Deliverables

- [ ] "✏️ Edit Event" button added to Inspector Panel event display
- [ ] Button click activates PlaceEvent tool and loads event into editor
- [ ] Visual feedback shows "Editing..." state when event editor is active
- [ ] Tool automatically resets to Select after save/remove operations

#### 1.4 Testing Requirements

**Unit Tests**:

- Test edit button activates PlaceEvent tool
- Test edit button loads correct event into `EventEditorState`
- Test "Editing..." button state when editor is active for position
- Test remove button clears event and resets tool

**Manual Testing**:

- Click event in Inspector → verify editor opens in right panel
- Modify event fields → click Save → verify map updates
- Click Edit on different event → verify editor switches to new event

#### 1.5 Success Criteria

- Clicking "✏️ Edit Event" switches to PlaceEvent tool
- Event fields populate correctly in editor
- "Editing..." button shows when event is being edited
- Save and Remove buttons reset tool to Select mode

---

### Phase 2: Add Visual Feedback for Event Being Edited

#### 2.1 Update MapGridWidget Tile Rendering

**File**: `sdk/campaign_builder/src/map_editor.rs`

**Location**: `MapGridWidget::ui()` implementation (lines 1563-1735)

**Changes**:

1. After drawing selected tile highlight (yellow border at lines 1665-1672), add check for event being edited
2. If `state.event_editor.is_some()` and `state.event_editor.position == pos`, draw additional visual indicator
3. Use distinct color (e.g., `Color32::LIGHT_GREEN`) and pattern (e.g., dashed stroke or corner markers)
4. Add hover tooltip showing "Event being edited"

**Implementation**:

```rust
// Existing: Highlight selected tile
if self.state.selected_position == Some(pos) {
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(2.0, Color32::YELLOW),
        egui::StrokeKind::Outside,
    );
}

// NEW: Highlight event being edited
if let Some(ref editor) = self.state.event_editor {
    if editor.position == pos {
        // Draw corner markers or dashed border
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(3.0, Color32::LIGHT_GREEN),
            egui::StrokeKind::Outside,
        );
        
        // Optional: Draw edit icon in corner
        let icon_rect = Rect::from_min_size(
            rect.min + Vec2::new(rect.width() - 12.0, 2.0),
            Vec2::new(10.0, 10.0),
        );
        painter.circle_filled(icon_rect.center(), 5.0, Color32::LIGHT_GREEN);
    }
}
```

#### 2.2 Add Tooltip for Event Being Edited

**File**: `sdk/campaign_builder/src/map_editor.rs`

**Location**: `MapGridWidget::ui()` implementation, hover detection section

**Changes**:

1. After existing click handling, add hover detection for tiles
2. If hovering over tile with active event editor, show tooltip: "✏️ Editing: [Event Type] - [Event Name]"
3. Use `response.on_hover_text()` or custom tooltip rendering

#### 2.3 Deliverables

- [ ] Green highlight border drawn on event tile being edited
- [ ] Visual indicator distinct from selection highlight (yellow)
- [ ] Hover tooltip shows "Editing: [type] - [name]" for event being edited
- [ ] Multi-select highlights (light blue) do not conflict with edit highlight

#### 2.4 Testing Requirements

**Unit Tests**:

- Test tile rendering includes edit highlight when event editor active
- Test edit highlight not shown when event editor is None
- Test edit highlight not shown for other positions

**Manual Testing**:

- Click "Edit Event" → verify green highlight appears on event tile
- Hover over editing tile → verify tooltip shows event details
- Click different tile → verify highlight moves
- Save event → verify highlight clears

#### 2.5 Success Criteria

- Green border clearly visible on event being edited
- Highlight does not obscure tile content or grid
- Tooltip provides useful context (event type and name)
- Visual feedback updates immediately when editor state changes

---

### Phase 3: Add Integration Tests

#### 3.1 Add Inspector Edit Workflow Test

**File**: `sdk/campaign_builder/src/map_editor.rs`

**Location**: `mod tests` section (after line 5025)

**Test**: `test_inspector_edit_event_workflow`

**Implementation**:

```rust
#[test]
fn test_inspector_edit_event_workflow() {
    let mut state = MapEditorState::new(
        Map::new(1, "Test Map".to_string(), "Description".to_string(), 10, 10)
    );
    let pos = Position::new(3, 4);
    
    // Create initial event
    let original_event = MapEvent::Sign {
        name: "Original Sign".to_string(),
        description: "Original description".to_string(),
        text: "Original text".to_string(),
    };
    state.add_event(pos, original_event.clone());
    
    // Simulate Inspector "Edit Event" button click
    state.current_tool = EditorTool::PlaceEvent;
    state.event_editor = Some(EventEditorState::from_map_event(
        pos,
        state.map.get_event(pos).unwrap()
    ));
    
    // Verify editor loaded correctly
    let editor = state.event_editor.as_ref().unwrap();
    assert_eq!(editor.position, pos);
    assert_eq!(editor.event_type, EventType::Sign);
    assert_eq!(editor.name, "Original Sign");
    assert_eq!(editor.sign_text, "Original text");
    
    // Modify event in editor
    let mut editor = state.event_editor.take().unwrap();
    editor.name = "Modified Sign".to_string();
    editor.sign_text = "Modified text".to_string();
    
    // Simulate "Save Changes" button click
    let updated_event = editor.to_map_event().expect("valid event");
    state.map.add_event(pos, updated_event);
    state.has_changes = true;
    state.event_editor = None;
    
    // Verify event was updated
    if let MapEvent::Sign { name, text, .. } = state.map.get_event(pos).unwrap() {
        assert_eq!(name, "Modified Sign");
        assert_eq!(text, "Modified text");
    } else {
        panic!("Expected Sign event");
    }
    
    assert!(state.has_changes);
}
```

#### 3.2 Add Visual Feedback Test

**File**: `sdk/campaign_builder/src/map_editor.rs`

**Location**: `mod tests` section

**Test**: `test_event_edit_visual_feedback`

**Implementation**:

```rust
#[test]
fn test_event_edit_visual_feedback() {
    let mut state = MapEditorState::new(
        Map::new(1, "Test Map".to_string(), "Description".to_string(), 10, 10)
    );
    let pos = Position::new(2, 2);
    
    // Add event
    let event = MapEvent::Sign {
        name: "Test Sign".to_string(),
        description: "Test".to_string(),
        text: "Text".to_string(),
    };
    state.add_event(pos, event.clone());
    
    // Verify no event editor initially
    assert!(state.event_editor.is_none());
    
    // Activate event editor
    state.event_editor = Some(EventEditorState::from_map_event(pos, &event));
    
    // Verify editor is active for this position
    assert!(state.event_editor.is_some());
    assert_eq!(state.event_editor.as_ref().unwrap().position, pos);
    
    // Verify show_event_editor_ui returns true
    assert!(state.show_event_editor_ui());
    
    // Clear editor
    state.event_editor = None;
    assert!(!state.show_event_editor_ui());
}
```

#### 3.3 Add Multi-Event Switching Test

**File**: `sdk/campaign_builder/src/map_editor.rs`

**Location**: `mod tests` section

**Test**: `test_switch_between_editing_events`

**Implementation**:

```rust
#[test]
fn test_switch_between_editing_events() {
    let mut state = MapEditorState::new(
        Map::new(1, "Test Map".to_string(), "Description".to_string(), 10, 10)
    );
    
    let pos1 = Position::new(1, 1);
    let pos2 = Position::new(5, 5);
    
    // Add two different events
    let event1 = MapEvent::Sign {
        name: "Sign 1".to_string(),
        description: "First sign".to_string(),
        text: "Text 1".to_string(),
    };
    let event2 = MapEvent::Trap {
        name: "Trap 1".to_string(),
        description: "First trap".to_string(),
        damage: 10,
        effect: None,
    };
    
    state.add_event(pos1, event1.clone());
    state.add_event(pos2, event2.clone());
    
    // Start editing event 1
    state.event_editor = Some(EventEditorState::from_map_event(pos1, &event1));
    assert_eq!(state.event_editor.as_ref().unwrap().position, pos1);
    assert_eq!(state.event_editor.as_ref().unwrap().event_type, EventType::Sign);
    
    // Switch to editing event 2
    state.event_editor = Some(EventEditorState::from_map_event(pos2, &event2));
    assert_eq!(state.event_editor.as_ref().unwrap().position, pos2);
    assert_eq!(state.event_editor.as_ref().unwrap().event_type, EventType::Trap);
    assert_eq!(state.event_editor.as_ref().unwrap().trap_damage, 10);
}
```

#### 3.4 Deliverables

- [ ] `test_inspector_edit_event_workflow` test added and passing
- [ ] `test_event_edit_visual_feedback` test added and passing
- [ ] `test_switch_between_editing_events` test added and passing
- [ ] All existing tests continue to pass (no regressions)

#### 3.5 Testing Requirements

**Quality Gates**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Expected Results**:

- All new tests pass
- Total test count increases by 3
- Zero clippy warnings
- Zero compilation errors

#### 3.6 Success Criteria

- All three new integration tests pass
- Tests cover Inspector → Edit → Save workflow
- Tests verify visual feedback state
- Tests verify switching between multiple events
- No test regressions

---

### Phase 4: Documentation and Verification

#### 4.1 Update Implementation Documentation

**File**: `docs/explanation/implementations.md`

**Section**: Add "Event Editing in Map Editor" section

**Content**:

```markdown
### Event Editing in Map Editor

**Completion Date**: [DATE]

**Implementation**: Phase 1-3 of event_editing_implementation_plan.md

**Changes Made**:

1. **Inspector Panel Edit Button**
   - Added "✏️ Edit Event" button to event display in Inspector Panel
   - Button activates PlaceEvent tool and loads event into EventEditorState
   - Visual feedback shows "Editing..." state when event is being edited
   - Tool resets to Select mode after Save/Remove operations

2. **Visual Feedback for Event Being Edited**
   - Green highlight border drawn on event tile being edited
   - Distinct from selection highlight (yellow) and multi-select (light blue)
   - Hover tooltip shows "Editing: [type] - [name]" for active event
   - Visual indicators update immediately when editor state changes

3. **Integration Tests**
   - Added `test_inspector_edit_event_workflow` covering full edit flow
   - Added `test_event_edit_visual_feedback` for state verification
   - Added `test_switch_between_editing_events` for multi-event handling
   - All tests passing with zero regressions

**Event Interaction Modes**:

1. **Create New Event**: Select PlaceEvent tool → click empty tile → configure → click "➕ Add Event"
2. **Edit Existing Event**: Click event in Inspector → click "✏️ Edit Event" → modify → click "💾 Save Changes"
3. **Remove Event**: Click event in Inspector → click "🗑 Remove Event" (or click while editing)

**Files Modified**:

- `sdk/campaign_builder/src/map_editor.rs`: Inspector Panel, MapGridWidget, tests
- `docs/explanation/implementations.md`: This documentation

**Testing**:

- 3 new integration tests added
- All quality gates passing (fmt, check, clippy, nextest)
- Manual testing performed for all three interaction modes

**Quality Verification**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Results**: All checks passed, [N+3] tests passing, zero warnings.
```

#### 4.2 Update Next Plans

**File**: `docs/explanation/next_plans.md`

**Section**: SDK → Map Editor Events

**Change**: Update status to ✅ COMPLETED with link to plan

```markdown
### Map Editor Events

✅ COMPLETED - [event editing implementation](./event_editing_implementation_plan.md)

Campaign Builder --> Map Editor --> Select Map --> Edit Map. Event editing now fully supported:
- Create new events using PlaceEvent tool
- Edit existing events via Inspector Panel "Edit Event" button
- Remove events via Inspector Panel or event editor
- Visual feedback shows which event is being edited
```

#### 4.3 Verification Checklist

**Code Quality**:

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes with zero errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- [ ] `cargo nextest run --all-features` passes with >1177 tests
- [ ] No `unwrap()` added without justification
- [ ] All new public methods have doc comments
- [ ] All new functionality has tests

**Functionality**:

- [ ] Edit Event button appears in Inspector Panel event display
- [ ] Clicking Edit Event activates PlaceEvent tool
- [ ] Clicking Edit Event loads event into EventEditorState
- [ ] Event editor shows "💾 Save Changes" for existing events
- [ ] Saving changes updates event in map
- [ ] Green highlight appears on event being edited
- [ ] Hover tooltip shows event details during editing
- [ ] Tool resets to Select after save/remove operations

**Testing**:

- [ ] 3 new integration tests added and passing
- [ ] `test_inspector_edit_event_workflow` covers full flow
- [ ] `test_event_edit_visual_feedback` verifies state
- [ ] `test_switch_between_editing_events` handles multi-event
- [ ] All existing tests continue to pass
- [ ] Manual testing confirms all interaction modes work

**Documentation**:

- [ ] Implementation section added to `implementations.md`
- [ ] Event interaction modes documented
- [ ] Files modified list complete
- [ ] Testing results documented
- [ ] `next_plans.md` updated with completion status

#### 4.4 Deliverables

- [ ] All Phase 1-3 code changes implemented and tested
- [ ] `docs/explanation/implementations.md` updated with Event Editing section
- [ ] `docs/explanation/next_plans.md` updated to show completion
- [ ] All verification checklist items completed
- [ ] Quality gates passing with zero errors/warnings

#### 4.5 Success Criteria

- All code quality checks pass
- All functionality verified manually and via tests
- Documentation complete and accurate
- No regressions introduced
- Feature ready for production use

---

## Implementation Notes

### Design Decisions

1. **Edit Button Placement**: Added to Inspector Panel (not toolbar) because:
   - Inspector already shows event details
   - Contextual action (applies to selected event)
   - Consistent with "Remove Event" button location
   - Avoids toolbar clutter

2. **Tool Auto-Switch on Edit**: Switches to PlaceEvent tool automatically because:
   - Required for event editor to display
   - Provides clear visual feedback (tool palette highlights)
   - Matches mental model: "I'm editing an event now"
   - User can manually switch back if desired

3. **Manual Tool Selection Required for New Events**: Decided against auto-switch to PlaceEvent when clicking empty tiles because:
   - Respects user's current tool selection
   - Avoids unexpected behavior during painting/selecting
   - Maintains predictable tool palette state
   - Users expect to select tool before action

4. **Cancel via Clicking Elsewhere**: No explicit Cancel button because:
   - Clicking different tile changes editor context naturally
   - Switching tools clears editor state (if desired)
   - Reduces UI clutter
   - Event changes only persist on Save (non-destructive workflow)

5. **Current Undo Behavior Sufficient**: Decided not to add per-field undo because:
   - Existing Save creates atomic undo action
   - User can cancel by clicking elsewhere without saving
   - Over-engineering for current requirements
   - Can be added later if users request it

### Technical Considerations

**Event Editor State Management**:

- `editor.event_editor` is `Option<EventEditorState>`
- Set to `Some(...)` when editing, `None` when closed
- Position stored in `EventEditorState.position` for visual feedback
- Tool state (`PlaceEvent`) required for editor to display

**Visual Highlight Rendering Order**:

1. Tile base color
2. Grid lines (if enabled)
3. Selection highlight (yellow, 2px)
4. Multi-select highlight (light blue, 2px)
5. Event edit highlight (green, 3px) ← Drawn last = on top

**Cross-Panel State Synchronization**:

- Inspector reads `editor.map.get_event(pos)` for display
- Inspector writes `editor.event_editor` and `editor.current_tool` for editing
- Event Editor reads/writes `editor.event_editor` for field state
- MapGridWidget reads `editor.event_editor.position` for visual feedback

### Future Enhancements (Out of Scope)

- Drag-and-drop event repositioning
- Bulk event editing (select multiple → edit properties)
- Event templates/presets for faster creation
- Event copy/paste between tiles or maps
- Event search/filter in Inspector
- Keyboard shortcuts for Edit/Remove/Save

---

## Appendix: Answer to Open Questions

1. **Auto-switch to PlaceEvent tool when clicking event tile?**
   - **Answer**: Option B (Require manual tool selection)
   - **Rationale**: Respects user intent; clicking while in Select tool should not change tools unexpectedly

2. **Cancel Edit button?**
   - **Answer**: Option A (Yes, add "✖ Cancel" button)
   - **Rationale**: Provides explicit way to close editor without saving; improves discoverability

3. **Event editing undo/redo behavior?**
   - **Answer**: Option C (Current behavior sufficient)
   - **Rationale**: Atomic save creates single undo action; sufficient for current use case; can enhance later if needed

---

## References

**Related Files**:

- `sdk/campaign_builder/src/map_editor.rs`: Map editor implementation
- `sdk/campaign_builder/src/lib.rs`: Campaign builder app state
- `antares/src/domain/map.rs`: Map and MapEvent domain types
- `docs/explanation/implementations.md`: Implementation documentation
- `docs/explanation/next_plans.md`: Feature planning tracker

**Related Plans**:

- `docs/explanation/campaign_builder_ui_consistency_plan.md`: Asset Manager and validation updates
- `docs/explanation/tile_visual_metadata_implementation_plan.md`: Tile editing patterns
- `docs/explanation/dialog_editor_completion_implementation_plan.md`: Similar editor completion work

**Architecture References**:

- `docs/reference/architecture.md` Section 4.5: Map Events
- `docs/reference/architecture.md` Section 3.2: SDK Module Structure
- `AGENTS.md`: Development rules and quality gates
