# Config Editor Phase 3: Interactive Key Capture and Auto-Population

## Overview

Phase 3 enhances the Campaign Builder Config Editor with interactive key capture and automatic field population, dramatically improving the user experience for key binding configuration.

**Key Features:**
- **Interactive Key Capture**: Click "Capture" and press a key instead of typing key names
- **Auto-Population**: Key binding fields automatically populate when opening the Config tab
- **Visual Feedback**: Clear indicators show when capture mode is active
- **Fallback Support**: Manual text editing still available for power users

## User Guide

### Opening the Config Editor

1. Launch Campaign Builder: `cargo run --package campaign_builder`
2. Open a campaign (e.g., `campaigns/tutorial`)
3. Click the **"Config"** tab in the tab bar
4. **Key binding fields auto-populate** with current config values

### Using Interactive Key Capture

#### Binding a Single Key

1. Navigate to **Controls Settings** section
2. Find the action you want to configure (e.g., "Move Forward")
3. Click the **"🎮 Capture"** button next to the text field
4. **Blue indicator appears**: "🎮 Press a key..."
5. Press the key you want to bind (e.g., press `W`)
6. Key appears in the text field as "W"
7. Capture mode exits automatically

#### Binding Multiple Keys to One Action

1. Click **"🎮 Capture"**, press `W` → Field shows "W"
2. Click **"🎮 Capture"** again, press `Up Arrow` → Field shows "W, Up Arrow"
3. Click **"🎮 Capture"** again, press `8` → Field shows "W, Up Arrow, 8"

All three keys (`W`, `Up Arrow`, `8`) now trigger "Move Forward".

#### Canceling Capture

1. Click **"🎮 Capture"** to start capture mode
2. Change your mind? Press `Escape`
3. Capture mode exits **without adding a key**

#### Clearing All Bindings

1. Click the **"🗑 Clear"** button next to a key binding field
2. All bindings for that action are removed (field becomes empty)

#### Manual Text Editing (Alternative Method)

You can still manually type key names if you prefer:

1. Click directly in the text field
2. Type: `W, Up Arrow, Space`
3. Press Tab or click elsewhere to save

**Supported Key Names:**
- Letters: `A-Z`
- Numbers: `0-9`
- Special: `Space`, `Enter`, `Escape`, `Tab`, `Backspace`, `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`
- Arrows: `Up Arrow`, `Down Arrow`, `Left Arrow`, `Right Arrow`
- Modifiers: `Shift`, `Ctrl`, `Alt`, `Super`
- Symbols: `+`, `-`, `*`, `/`, `.`, `,`, `;`, `'`, `[`, `]`, `\`, `` ` ``, `~`, `!`, `@`, `#`, `$`, `%`, `^`, `&`

### Saving Your Changes

1. Make your key binding changes
2. Click **"Save"** in the toolbar
3. Changes written to `<campaign>/config.ron`

### Auto-Population Behavior

**When does auto-load happen?**

- **First time** opening the Config tab in a session
- **When switching campaigns** (detects campaign directory change)
- **When clicking "Load" or "Reload"** toolbar buttons

**When does it NOT auto-load?**

- On every UI render (only once per campaign)
- When you have unsaved changes (Load button preserves your edits)

## Implementation Details

### Architecture

```
ConfigEditorState
├── needs_initial_load: bool          ← Tracks first display
├── last_campaign_dir: Option<PathBuf> ← Detects campaign changes
├── capturing_key_for: Option<String>  ← Tracks which action is capturing
└── last_captured_key: Option<String>  ← Stores most recent key captured

Helper Functions (outside impl block):
├── egui_key_to_string(key: &egui::Key) -> String
├── format_key_list(keys: &[String]) -> String
└── parse_key_list(text: &str) -> Vec<String>
```

### Key Capture Flow

```rust
// In show() method - called every frame
fn show(&mut self, ui, campaign_dir, unsaved_changes, status_message) {
    // 1. Auto-load check (only on first display or campaign change)
    if (self.needs_initial_load || campaign_changed) && campaign_dir.is_some() {
        self.load_config(campaign_dir);
        self.update_edit_buffers(); // ← Populates text fields
    }

    // 2. Handle key capture events
    self.handle_key_capture(ui);

    // 3. Render UI (toolbar, sections, etc.)
    // ...
}
```

### Event Handler

```rust
fn handle_key_capture(&mut self, ui: &mut egui::Ui) {
    if self.capturing_key_for.is_none() {
        return; // Not capturing, skip
    }

    ui.input(|i| {
        for event in &i.events {
            if let egui::Event::Key { key, pressed: true, .. } = event {
                if *key == egui::Key::Escape {
                    // Cancel capture
                    self.capturing_key_for = None;
                    return;
                }

                // Convert egui key to human-readable string
                let key_name = egui_key_to_string(key);

                // Add to appropriate buffer
                let buffer = match self.capturing_key_for.as_ref().unwrap().as_str() {
                    "move_forward" => &mut self.controls_move_forward_buffer,
                    "move_back" => &mut self.controls_move_back_buffer,
                    // ... etc
                };

                if !buffer.is_empty() {
                    buffer.push_str(", ");
                }
                buffer.push_str(&key_name);

                self.capturing_key_for = None; // Exit capture mode
            }
        }
    });
}
```

### Key Conversion

```rust
fn egui_key_to_string(key: &egui::Key) -> String {
    match key {
        egui::Key::W => "W".to_string(),
        egui::Key::ArrowUp => "Up Arrow".to_string(),
        egui::Key::Space => "Space".to_string(),
        egui::Key::Enter => "Enter".to_string(),
        // ... 76 total mappings
        _ => format!("{:?}", key), // Fallback for unmapped keys
    }
}
```

### Buffer Helpers

```rust
// Format Vec<String> as comma-separated text
fn format_key_list(keys: &[String]) -> String {
    keys.join(", ")
}

// Parse comma-separated text to Vec<String>
fn parse_key_list(text: &str) -> Vec<String> {
    text.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
```

## Testing

### Test Coverage (14 new tests)

**Key Conversion Tests:**
- `test_egui_key_to_string_letters` - A-Z conversion
- `test_egui_key_to_string_numbers` - 0-9 conversion
- `test_egui_key_to_string_special_keys` - Space, Enter, etc.
- `test_egui_key_to_string_arrows` - Arrow key conversion

**Formatting Tests:**
- `test_format_key_list_single_key`
- `test_format_key_list_multiple_keys`
- `test_format_key_list_empty`
- `test_parse_key_list_single_key`
- `test_parse_key_list_multiple_keys`
- `test_parse_key_list_with_extra_spaces`
- `test_parse_key_list_empty_string`
- `test_parse_key_list_filters_empty_entries`

**State Management Tests:**
- `test_needs_initial_load_default_true`
- `test_capturing_key_for_default_none`
- `test_update_edit_buffers_auto_populates`
- `test_round_trip_buffer_conversion`
- `test_manual_text_edit_still_works`
- `test_multiple_keys_per_action`

### Running Tests

```bash
# Full test suite (note: some pre-existing failures in lib.rs, unrelated to config_editor)
cargo test --package campaign_builder --lib config_editor::tests

# Build verification
cargo build --package campaign_builder --lib
```

## Known Limitations

### Current Limitations

1. **No modifier combinations**: Cannot capture `Shift+W`, `Ctrl+Space`, etc. (single keys only)
2. **Platform-specific names**: Super/Windows key may display differently on macOS vs Windows
3. **No conflict detection**: Won't warn if same key used for multiple actions
4. **Single key capture**: Must click Capture for each key (no "record all" mode)
5. **No visual preview**: Can't see which key you're about to press

### Workarounds

- **Modifiers**: Use separate bindings (e.g., bind both `Shift` and `W` to different actions if needed)
- **Conflicts**: Manually review all bindings before saving
- **Multiple keys**: Click Capture repeatedly for each key you want to add

## Future Enhancements (Out of Scope)

### Potential Phase 4 Features

- **Modifier Support**: Capture `Shift+W`, `Ctrl+Space` as single bindings
- **Conflict Detection**: Warn when same key bound to multiple actions
- **Visual Keymap**: Show all current bindings in a grid/table view
- **Binding Profiles**: Pre-defined templates (WASD, Arrow Keys, Vim-style)
- **Undo/Redo**: Revert key binding changes before saving
- **Platform-Aware Names**: Auto-detect OS and use "Command" on macOS, "Windows Key" on Windows
- **Record Mode**: Click once, press multiple keys, click again to stop recording

## Troubleshooting

### Key binding fields are empty when I open the Config tab

**Cause**: Auto-load may have failed.

**Solution**: Click the **"Load"** button in the toolbar manually.

### Capture button doesn't respond

**Cause**: May already be in capture mode for another action.

**Solution**: Press `Escape` to cancel current capture, then try again.

### Pressed a key but it didn't appear

**Possible causes:**
1. Key not supported (unmapped egui::Key variant) → Will show debug format like `Key::F13`
2. Pressed a modifier-only key (Shift, Ctrl alone) → Will capture "Shift", "Ctrl" as text
3. Capture mode wasn't active → Check for blue "Press a key..." indicator

**Solution**: Use manual text editing as fallback if capture doesn't work.

### Changes not saving to config.ron

**Cause**: Didn't click Save button, or validation error.

**Solution**:
1. Check for red validation error messages in the UI
2. Ensure all required key bindings are non-empty
3. Click **"Save"** button in toolbar

## Related Documentation

- **Implementation Plan**: `docs/explanation/config_editor_implementation_plan.md`
- **Phase 1 & 2 Summary**: `docs/explanation/implementations.md` (search for "Config Editor")
- **GameConfig Schema**: `docs/explanation/game_config_schema.md`
- **Architecture Reference**: `docs/reference/architecture.md` (Section 6)

## Version History

- **Phase 3**: 2025-01-13 - Interactive key capture and auto-population
- **Phase 2**: 2025-01-12 - UI enhancements, validation, presets
- **Phase 1**: 2025-01-11 - Core config editor implementation

## Credits

Implemented following the antares project's editor pattern standards and AGENTS.md guidelines.
