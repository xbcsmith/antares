# Config Editor Phase 3: Interactive Key Capture and Auto-Population - PLAN

## Overview

Phase 3 enhances the Config Editor with interactive key capture functionality and proper auto-population of key binding fields. This addresses critical UX issues where users must TYPE key names (error-prone) and text fields don't show current config values when the tab opens.

## Problem Statement

### Current Issues (Phase 2)

1. **No Auto-Population**: When opening the Config tab, key binding text fields are empty even though the config has values
2. **Manual Key Entry**: Users must type key names like "W, Up Arrow" which is:
   - Error-prone (typos, wrong capitalization, invalid names)
   - Unintuitive (most users expect to press a key to bind it)
   - Requires memorizing the exact key name format
3. **Unused Fields**: `capturing_key_for` and `last_captured_key` fields were added in Phase 2 but never utilized
4. **Poor UX**: No visual feedback for capture state, no way to easily clear bindings

### User Experience Goal

Users should:
- See current key bindings immediately when opening Config tab
- Click a "Capture" button and press a key to bind it
- See visual feedback during capture ("Press a key...")
- Have the option to manually edit text as a fallback
- Easily clear unwanted bindings

## Implementation Phases

### Phase 3.1: Key Capture Foundation

#### File: `sdk/campaign_builder/src/config_editor.rs`

**Add Interactive Key Capture System**:

1. **Event Handler Method**:
   ```rust
   fn handle_key_capture(&mut self, ui: &egui::Ui, action_id: &str) -> bool
   ```
   - Process `ui.input(|i| i.events.iter())` for keyboard events
   - Filter for `egui::Event::Key { key, pressed: true, .. }`
   - Convert egui::Key to human-readable string
   - Handle special keys: Escape (cancel), Backspace (clear)
   - Return true if key was captured

2. **State Management**:
   - Use existing `capturing_key_for: Option<String>` to track which action is active
   - Use existing `last_captured_key: Option<String>` to store most recent capture
   - Add `needs_initial_load: bool` flag to trigger auto-population once

3. **Key Conversion Logic**:
   - Map egui::Key enum variants to display names
   - Examples: `Key::W` → "W", `Key::ArrowUp` → "Up Arrow", `Key::Space` → "Space"
   - Handle all letter keys (A-Z), number keys (0-9), special keys, arrows, modifiers

**Testing**:
- Test key capture state transitions (idle → capturing → captured → idle)
- Test Escape cancels without binding
- Test Backspace clears current binding
- Test all key types convert correctly

### Phase 3.2: UI Integration for Key Capture

#### File: `sdk/campaign_builder/src/config_editor.rs`

**Enhance `show_controls_section()` Method**:

1. **Add Capture Buttons**:
   - Place "🎮 Capture" button next to each key binding text field
   - On click: Set `self.capturing_key_for = Some(action_id)`
   - Show different button text/color when capturing for that action

2. **Visual Feedback**:
   ```
   Normal state:    [Text field with "W, Up Arrow"] [Capture]
   Capturing state: [🎮 Press a key...             ] [Cancel]
   Captured state:  [W, Up Arrow, Space           ] [Capture] (brief green flash)
   ```

3. **Clear Functionality**:
   - Add "Clear" button to remove all bindings for an action
   - Confirm before clearing if multiple keys are bound

4. **Layout**:
   ```rust
   ui.horizontal(|ui| {
       ui.label("Move Forward:");
       
       // Text field - editable, shows current bindings
       let text_response = ui.text_edit_singleline(&mut self.controls_move_forward_buffer);
       
       // Capture button - starts interactive capture
       if ui.button(if self.capturing_key_for == Some("move_forward") { 
           "🎮 Press a key..." 
       } else { 
           "Capture" 
       }).clicked() {
           self.capturing_key_for = Some("move_forward".to_string());
       }
       
       // Clear button - removes all bindings
       if ui.button("Clear").clicked() {
           self.controls_move_forward_buffer.clear();
       }
       
       // Handle key capture if active
       if self.capturing_key_for == Some("move_forward") {
           if let Some(key_name) = self.handle_key_capture(ui, "move_forward") {
               // Append to buffer (comma-separated)
               if !self.controls_move_forward_buffer.is_empty() {
                   self.controls_move_forward_buffer.push_str(", ");
               }
               self.controls_move_forward_buffer.push_str(&key_name);
               self.capturing_key_for = None;
               *unsaved_changes = true;
           }
       }
   });
   ```

**Testing**:
- Test capture button click activates capture mode
- Test visual feedback shows correct state
- Test captured key appears in text field
- Test multiple captures append correctly (comma-separated)
- Test clear button removes all bindings
- Test manual text editing still works

### Phase 3.3: Auto-Population on Load

#### File: `sdk/campaign_builder/src/config_editor.rs`

**Fix Initialization Flow**:

1. **Add `needs_initial_load` Flag**:
   ```rust
   pub struct ConfigEditorState {
       // ... existing fields ...
       pub needs_initial_load: bool,
   }
   ```

2. **Modify `show()` Method**:
   ```rust
   pub fn show(&mut self, ui: &mut egui::Ui, campaign_dir: Option<&PathBuf>, ...) {
       // Check if this is first render with a campaign directory
       if campaign_dir.is_some() && self.needs_initial_load {
           self.load_config(campaign_dir);
           self.needs_initial_load = false;
       }
       
       // ... rest of show() logic ...
   }
   ```

3. **Ensure `update_edit_buffers()` Called**:
   - Already called in `load_config()` ✓
   - Already called in "Reset to Defaults" ✓
   - Verify it's called on Reload ✓

4. **Handle Campaign Directory Changes**:
   - Track last campaign directory
   - When directory changes, set `needs_initial_load = true`
   - Prevents re-loading on every render

**Testing**:
- Test text fields populate when Config tab first opened
- Test text fields populate when campaign directory changes
- Test auto-load doesn't trigger on every render
- Test auto-load respects unsaved changes warning

### Phase 3.4: Key Name Conversion Utilities

#### File: `sdk/campaign_builder/src/config_editor.rs`

**Add Helper Functions**:

```rust
/// Convert egui::Key to human-readable display name
fn egui_key_to_string(key: egui::Key) -> Option<String> {
    use egui::Key;
    match key {
        // Letters
        Key::A => Some("A".to_string()),
        Key::B => Some("B".to_string()),
        // ... all letters ...
        Key::Z => Some("Z".to_string()),
        
        // Numbers
        Key::Num0 => Some("0".to_string()),
        Key::Num1 => Some("1".to_string()),
        // ... all numbers ...
        
        // Special keys
        Key::Space => Some("Space".to_string()),
        Key::Enter => Some("Enter".to_string()),
        Key::Escape => None, // Special: cancels capture
        Key::Tab => Some("Tab".to_string()),
        Key::Backspace => None, // Special: clears binding
        
        // Arrows
        Key::ArrowUp => Some("Up Arrow".to_string()),
        Key::ArrowDown => Some("Down Arrow".to_string()),
        Key::ArrowLeft => Some("Left Arrow".to_string()),
        Key::ArrowRight => Some("Right Arrow".to_string()),
        
        // Modifiers
        // Note: Modifiers are typically handled separately in egui
        
        _ => None, // Unsupported or special-case key
    }
}

/// Parse comma-separated key list for validation
fn parse_key_list(text: &str) -> Vec<String> {
    text.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Format key list as comma-separated text
fn format_key_list(keys: &[String]) -> String {
    keys.join(", ")
}
```

**Testing**:
- Test all letter keys convert correctly (A-Z)
- Test all number keys convert correctly (0-9)
- Test special keys (Space, Enter, Tab, etc.)
- Test arrow keys convert with "Arrow" suffix
- Test Escape returns None (signals cancel)
- Test Backspace returns None (signals clear)
- Test parse/format round-trip preserves keys

### Phase 3.5: Testing Requirements

#### New Tests to Add

1. **Key Conversion Tests** (5 tests):
   - `test_egui_key_to_string_letters()` - A-Z conversion
   - `test_egui_key_to_string_numbers()` - 0-9 conversion
   - `test_egui_key_to_string_special_keys()` - Space, Enter, Tab, etc.
   - `test_egui_key_to_string_arrows()` - Arrow key conversion
   - `test_egui_key_to_string_escape_backspace()` - Returns None

2. **Capture State Tests** (4 tests):
   - `test_key_capture_state_transitions()` - idle → capturing → captured
   - `test_escape_cancels_capture()` - Escape doesn't bind
   - `test_backspace_clears_binding()` - Backspace clears buffer
   - `test_multiple_key_capture()` - Multiple captures append correctly

3. **Auto-Population Tests** (3 tests):
   - `test_auto_population_on_first_load()` - Fields populate on init
   - `test_auto_population_respects_campaign_change()` - Re-loads on directory change
   - `test_auto_population_not_on_every_render()` - Only loads once

4. **Integration Tests** (2 tests):
   - `test_manual_text_edit_still_works()` - Fallback editing works
   - `test_clear_button_removes_bindings()` - Clear functionality

**Total New Tests**: 14 tests
**Total Tests After Phase 3**: 30 (Phase 1-2) + 14 (Phase 3) = 44 tests

### Phase 3.6: Documentation Updates

#### Files to Update

1. **`sdk/campaign_builder/src/config_editor.rs`**:
   - Add doc comments for new methods (`handle_key_capture`, `egui_key_to_string`, etc.)
   - Update examples showing interactive capture workflow

2. **`docs/explanation/implementations.md`**:
   - Add Phase 3 section documenting key capture and auto-population
   - Include usage examples and testing results

3. **`docs/explanation/config_editor_phase3_summary.md`** (NEW):
   - Comprehensive guide to Phase 3 implementation
   - User workflow examples
   - Technical decisions and architecture notes

## Deliverables Checklist

- [ ] `handle_key_capture()` method implemented
- [ ] `egui_key_to_string()` conversion function
- [ ] `parse_key_list()` and `format_key_list()` helpers
- [ ] "Capture" buttons added to all 6 key binding fields
- [ ] Visual feedback for capture state (blue highlight, "Press a key...")
- [ ] "Clear" buttons for removing bindings
- [ ] `needs_initial_load` flag and auto-population logic
- [ ] Campaign directory change detection
- [ ] 14 new tests (all passing)
- [ ] Documentation updates (3 files)
- [ ] Manual verification completed

## Success Criteria

### Functional Requirements

- ✅ Key binding text fields auto-populate when Config tab opens
- ✅ Clicking "Capture" button enables interactive key capture
- ✅ Pressing any supported key adds it to the binding
- ✅ Key names display as human-readable text (e.g., "Up Arrow" not "ArrowUp")
- ✅ Escape key cancels capture without binding
- ✅ Backspace clears the current binding
- ✅ Multiple keys can be captured for one action (comma-separated)
- ✅ Manual text editing still works as fallback
- ✅ Clear button removes all bindings for an action
- ✅ Visual feedback clearly shows capture state

### Non-Functional Requirements

- ✅ Zero regression in Phase 1 & 2 functionality (30 tests still passing)
- ✅ All 14 new tests passing
- ✅ `cargo check` and `cargo clippy` pass without errors
- ✅ Code follows existing patterns (consistent with Phase 1-2)
- ✅ Full documentation with examples
- ✅ SPDX headers on all files

## Risk Analysis

### Risk: egui Event Handling Limitations

**Issue**: egui may not expose all key events or may handle some keys specially (e.g., Tab for focus navigation)

**Mitigation**:
- Provide manual text editing as fallback
- Document which keys can/cannot be captured
- Test extensively with different key types
- Consider alternative: Button-based key selection (dropdown of common keys)

### Risk: Auto-Population Conflicts with Unsaved Changes

**Issue**: Auto-loading config might overwrite user's unsaved edits

**Mitigation**:
- Only auto-load on first render with campaign directory
- Track `needs_initial_load` flag carefully
- Respect `unsaved_changes` flag
- Show confirmation if auto-load would discard changes

### Risk: Platform-Specific Key Name Differences

**Issue**: Key names may differ across platforms (e.g., "Super" vs "Command" on macOS)

**Mitigation**:
- Use egui's platform-agnostic key names
- Document platform differences in tooltips
- Test on multiple platforms if possible
- Allow both "Ctrl" and "Control" as valid names

## User Workflow Examples

### Example 1: Binding a Single Key

1. User opens Config tab → Text fields auto-populate with current bindings
2. User clicks "Capture" button next to "Move Forward"
3. UI shows "🎮 Press a key..." in blue
4. User presses W key
5. Text field updates to show "W"
6. User clicks Save
7. Config saved with `move_forward: ["W"]`

### Example 2: Binding Multiple Keys

1. User clicks "Capture" next to "Move Forward" (currently shows "W")
2. UI shows "🎮 Press a key..."
3. User presses Up Arrow
4. Text field updates to show "W, Up Arrow"
5. User clicks "Capture" again
6. User presses Numpad 8
7. Text field shows "W, Up Arrow, 8"
8. User clicks Save

### Example 3: Clearing and Rebinding

1. User sees "Move Forward" is bound to "W, Up Arrow, Space"
2. User clicks "Clear" button
3. Text field becomes empty
4. User clicks "Capture"
5. User presses W
6. Text field shows "W"
7. User clicks Save

### Example 4: Manual Text Editing (Fallback)

1. User wants to bind "Shift+W" but interactive capture doesn't support modifiers
2. User types "Shift, W" directly in text field
3. Validator accepts both key names
4. User clicks Save
5. Binding works in game

## Testing Strategy

### Unit Tests (14 new tests)

- Key conversion utilities (5 tests)
- Capture state management (4 tests)
- Auto-population logic (3 tests)
- Integration with existing features (2 tests)

### Manual Testing

1. **Auto-Population**:
   - Open Config tab → Verify text fields show current bindings
   - Switch campaigns → Verify fields update to new campaign's bindings
   - Reload page → Verify auto-load only happens once

2. **Interactive Capture**:
   - Click Capture → Verify visual feedback
   - Press various keys → Verify correct names appear
   - Press Escape → Verify capture cancels
   - Press Backspace → Verify binding clears

3. **Multi-Key Binding**:
   - Capture multiple keys → Verify comma-separated list
   - Mix captured and typed keys → Verify both work

4. **Edge Cases**:
   - Capture while text field has focus → Verify events don't conflict
   - Rapid captures → Verify state doesn't get confused
   - Click Capture then click elsewhere → Verify capture cancels gracefully

### Regression Testing

- Run all 30 Phase 1-2 tests → Verify 100% passing
- Test all Phase 1-2 features → Verify no broken functionality
- Test save/load/reload → Verify data integrity

## Architecture Compliance

### Follows Existing Patterns

- ✅ Uses egui widgets consistently (Button, TextEdit, visual feedback)
- ✅ State management in ConfigEditorState struct
- ✅ Helper methods for complex logic (conversion, parsing)
- ✅ Proper error handling with validation
- ✅ Clear separation of concerns (UI, state, conversion, validation)

### Rust Best Practices

- ✅ Type-safe key conversion (egui::Key → String)
- ✅ Proper Option handling for special keys (Escape, Backspace)
- ✅ Comprehensive testing (44 total tests)
- ✅ Full documentation with examples
- ✅ SPDX headers on all files

## Related Documentation

- [egui Input Handling](https://docs.rs/egui/latest/egui/struct.Context.html#method.input) - Event capture API
- [egui::Key enum](https://docs.rs/egui/latest/egui/enum.Key.html) - All key variants
- [config_editor_implementation_plan.md](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/config_editor_implementation_plan.md) - Overall plan
- [implementations.md](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/implementations.md) - Implementation history

## Summary

Phase 3 transforms the Config Editor's key binding UX from error-prone manual text entry to intuitive interactive capture. Users can press keys to bind them, see immediate visual feedback, and still have the option to manually edit text if needed. Auto-population ensures the UI always shows current config values, eliminating confusion about what's actually bound.

**Status**: READY FOR IMPLEMENTATION
**Estimated Effort**: Medium (2-3 development sessions)
**Complexity**: Medium (egui event handling, state management)
**Risk Level**: Low (good fallbacks, comprehensive testing)
