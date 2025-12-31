# Portrait Support Implementation Plan

## Overview

This plan adds comprehensive portrait support to the Campaign Builder SDK Character
Editor Tab. The implementation enables users to select character portraits through:

1. **Autocomplete text input** - Type-ahead suggestions from available portraits in
   `assets/portraits`
2. **Grid picker popup** - Visual grid browser for selecting portraits with thumbnail
   previews
3. **Preview panel integration** - Display the selected portrait image in the Character
   Preview panel

## Current State Analysis

### Existing Infrastructure

| Component | Location | Current State |
|-----------|----------|---------------|
| `portrait_id` field | [CharacterEditBuffer](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs#L90) | String field, parsed as `u8` |
| Portrait form input | [characters_editor.rs:1129-1134](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs#L1129-L1134) | Simple `TextEdit::singleline` |
| Preview panel | [show_character_preview](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs#L866-L908) | Shows `portrait_id` as text label |
| Asset Manager | [AssetType::Portrait](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/asset_manager.rs#L173-L174) | Defines portrait subdirectory as `assets/portraits` |
| Portrait assets | `campaigns/tutorial/assets/portraits/` | PNG files named `0.png`, `10.png`, `11.png`, etc. |
| Autocomplete patterns | [ui_helpers.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/ui_helpers.rs#L2247-L2313) | Established patterns like `autocomplete_item_selector` |

### Identified Issues

1. **No autocomplete** - Portrait ID field is plain text with no suggestions
2. **No visual preview** - User cannot see available portrait images
3. **No grid picker** - No popup to browse and select portraits visually
4. **Preview panel shows ID only** - No actual portrait image rendered

## Implementation Phases

### Phase 1: Core Portrait Discovery

Add capability to scan and enumerate available portrait assets from the campaign
directory.

#### 1.1 Create Portrait Discovery Function

**File: [ui_helpers.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/ui_helpers.rs)**

Add a new function to discover available portrait files:

```text
Location: After extract_monster_candidates function (around line 3400)
Function: extract_portrait_candidates(campaign_dir: Option<&PathBuf>) -> Vec<String>
```

**Logic:**
1. Check if `campaign_dir` is provided
2. Construct path: `campaign_dir/assets/portraits`
3. Scan directory for image files (`.png`, `.jpg`, `.jpeg`)
4. Extract portrait IDs from filenames (e.g., `0.png` → `"0"`), prioritizing `.png` files
5. Return sorted list of portrait ID strings (numeric sort)
6. Handle missing directories gracefully (return empty vec)

#### 1.2 Create Portrait Path Resolution Helper

**File: [ui_helpers.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/ui_helpers.rs)**

Add a helper to resolve portrait ID to full path:

```text
Function: resolve_portrait_path(campaign_dir: Option<&PathBuf>, portrait_id: &str) -> Option<PathBuf>
```

**Logic:**
1. Build path: `campaign_dir/assets/portraits/{portrait_id}.png` (prioritize PNG)
2. Check if file exists
3. Only try other formats if game engine supports them
4. Return `Some(path)` if found, `None` otherwise

#### 1.3 Testing Requirements

- Unit test for `extract_portrait_candidates` with mock directory
- Unit test for `resolve_portrait_path` with mock files
- Test edge cases: empty directory, no campaign dir, invalid files

#### 1.4 Deliverables

- [ ] `extract_portrait_candidates` function in `ui_helpers.rs`
- [ ] `resolve_portrait_path` function in `ui_helpers.rs`
- [ ] Unit tests for both functions

#### 1.5 Success Criteria

- Functions compile without errors
- Unit tests pass
- Discovery correctly enumerates portrait files

---

### Phase 2: Autocomplete Portrait Selector

Add an autocomplete widget for portrait selection following existing patterns.

#### 2.1 Create `autocomplete_portrait_selector` Function

**File: [ui_helpers.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/ui_helpers.rs)**

Add after existing autocomplete selectors (around line 3276):

```text
Function signature:
pub fn autocomplete_portrait_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_portrait_id: &mut String,
    available_portraits: &[String],
) -> bool
```

**Pattern:** Follow `autocomplete_condition_selector` as reference template:
- Use `AutocompleteInput` widget from `ui_helpers`
- Persist buffer state with `load_autocomplete_buffer` / `store_autocomplete_buffer`
- Show clear button when selection exists
- Return `true` if selection changed

#### 2.2 Add Export in Module

**File: [ui_helpers.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/ui_helpers.rs)**

Add to module public exports (at beginning of file in `use` statements and module documentation).

#### 2.3 Testing Requirements

- Unit test for autocomplete behavior
- Test clear button functionality

#### 2.4 Deliverables

- [ ] `autocomplete_portrait_selector` function
- [ ] Public export in module
- [ ] Unit tests

#### 2.5 Success Criteria

- Autocomplete widget shows suggestions from available portraits
- Selection persists correctly
- Clear button removes selection

---

### Phase 3: Portrait Grid Picker Popup

Add a popup window with a grid of portrait thumbnails for visual selection.

#### 3.1 Create Portrait Grid State

**File: [characters_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs)**

Add state fields to `CharactersEditorState`:

```text
Location: After line 58 (has_unsaved_changes field)
Add fields:
    /// Whether the portrait grid picker popup is open
    pub portrait_picker_open: bool,
    /// Cached portrait textures for grid display
    pub portrait_textures: HashMap<String, Option<egui::TextureHandle>>,
    /// Available portrait IDs (cached from directory scan)
    pub available_portraits: Vec<String>,
```

Initialize in `Default` impl:
- `portrait_picker_open: false`
- `portrait_textures: HashMap::new()`
- `available_portraits: Vec::new()`

#### 3.2 Create Portrait Loading Helper

**File: [characters_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs)**

Add method to load portrait texture:

```text
Function: load_portrait_texture(
    &mut self,
    ctx: &egui::Context,
    campaign_dir: Option<&PathBuf>,
    portrait_id: &str,
) -> Option<&egui::TextureHandle>
```

**Logic:**
1. Check if texture already cached in `portrait_textures`
2. If not cached, attempt to load image from file:
   - Resolve path using `resolve_portrait_path`
   - Load image bytes
   - Decode using `image` crate
   - Create `egui::ColorImage`
   - Register as `TextureHandle` with egui context
3. Store result in cache (even `None` for failed loads)
4. Return reference to cached texture

#### 3.3 Create Portrait Grid Picker Method

**File: [characters_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs)**

Add method to render grid picker popup:

```text
Function: show_portrait_grid_picker(
    &mut self,
    ctx: &egui::Context,
    campaign_dir: Option<&PathBuf>,
) -> Option<String>  // Returns selected portrait ID if user clicked one
```

**UI Structure:**
```
+----------------------------------+
|      Select Portrait      [X]   |
+----------------------------------+
| +----+ +----+ +----+ +----+     |
| |    | |    | |    | |    |     |
| | 0  | | 1  | | 2  | | 3  |     |
| +----+ +----+ +----+ +----+     |
| +----+ +----+ +----+ +----+     |
| |    | |    | |    | |    |     |
| | 4  | | 5  | | 6  | | 7  |     |
| +----+ +----+ +----+ +----+     |
|              ...                 |
+----------------------------------+
```

**Implementation:**
- Use `egui::Window` as modal popup
- Title: "Select Portrait"
- Add close button using `Window::title_bar(true)`
- Use `egui::Grid` with 4-5 columns for thumbnails
- Each cell:
  - Display portrait image (64x64 pixels)
  - Show portrait ID below image
  - Use `ui.image()` or placeholder if image not loaded
  - Make cell clickable with `ImageButton` or selectable container
- On click: return `Some(portrait_id)` and set `portrait_picker_open = false`
- On close: set `portrait_picker_open = false`

#### 3.4 Integrate Picker Button in Form

**File: [characters_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs)**

Modify `show_character_form` method (around line 1129-1134):

**Before:**
```rust
ui.label("Portrait ID:");
ui.add(
    egui::TextEdit::singleline(&mut self.buffer.portrait_id)
        .desired_width(60.0),
);
ui.end_row();
```

**After:**
```rust
ui.label("Portrait ID:");
ui.horizontal(|ui| {
    // Autocomplete input
    if autocomplete_portrait_selector(
        ui,
        "character_portrait",
        "",  // no additional label, already shown
        &mut self.buffer.portrait_id,
        &self.available_portraits,
    ) {
        // Selection changed via autocomplete
    }

    // Grid picker button
    if ui.button("🖼").on_hover_text("Browse portraits").clicked() {
        self.portrait_picker_open = true;
    }
});
ui.end_row();
```

#### 3.5 Render Picker Popup in `show` Method

**File: [characters_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs)**

In the main `show()` method, after the form rendering, add popup rendering:

```text
Location: After match on self.mode (around line 639)
```

**Code:**
```rust
// Show portrait grid picker if open
if self.portrait_picker_open {
    if let Some(selected_id) = self.show_portrait_grid_picker(ui.ctx(), campaign_dir) {
        self.buffer.portrait_id = selected_id;
        *unsaved_changes = true;
    }
}
```

#### 3.6 Scan Portraits on Campaign Open

Update `show()` method to refresh available portraits when campaign directory changes:

```text
Location: At start of show() method
```

**Logic:**
- Store previous campaign_dir in state
- If campaign_dir changed, rescan portraits:
  ```rust
  self.available_portraits = extract_portrait_candidates(campaign_dir);
  ```

#### 3.7 Testing Requirements

- Manual testing: verify popup opens and closes correctly
- Manual testing: verify clicking portrait selects it
- Manual testing: verify scrolling works for many portraits

#### 3.8 Deliverables

- [ ] State fields for portrait picker
- [ ] `load_portrait_texture` method
- [ ] `show_portrait_grid_picker` method
- [ ] Form integration with autocomplete + picker button
- [ ] Popup rendering in `show()` method
- [ ] Portrait scanning on campaign open

#### 3.9 Success Criteria

- Clicking 🖼 button opens portrait grid popup
- Grid displays available portraits as thumbnails
- Clicking a portrait closes popup and fills portrait_id field
- Autocomplete works alongside picker button

---

### Phase 4: Preview Panel Portrait Display

Show the selected portrait image in the Character Preview panel.

#### 4.1 Update `show_character_preview` Method

**File: [characters_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs)**

Modify the preview panel to show portrait image:

**Location:** [show_character_preview](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs#L866-L908)

**Changes:**
1. Add parameters: `ctx: &egui::Context`, `campaign_dir: Option<&PathBuf>`
2. Before the info grid, add portrait image display at **64x64 pixels**:

```rust
// Show portrait image at top of preview (64x64 standardized size)
let portrait_id = character.portrait_id.to_string();
if let Some(texture) = self.load_portrait_texture(ctx, campaign_dir, &portrait_id) {
    ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(64.0, 64.0)));
    ui.add_space(10.0);
}
```

3. Update call sites to pass additional parameters

#### 4.2 Update Method Signature and Call Sites

**File: [characters_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs)**

The `show_character_preview` is called from `show_list` (around line 821). Update:

1. Add `campaign_dir: Option<&PathBuf>` parameter to `show_list`
2. Pass `ui.ctx()` and `campaign_dir` to `show_character_preview`
3. Update `show()` method call to `show_list` to pass campaign_dir

Show an error indicator and fallback when portrait image cannot be loaded:

```rust
// In show_character_preview:
let portrait_id = character.portrait_id.to_string();
if let Some(texture) = self.load_portrait_texture(ctx, campaign_dir, &portrait_id) {
    ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(64.0, 64.0)));
} else {
    // Portrait not found - show ❌ indicator
    ui.vertical(|ui| {
        ui.add(
            egui::Label::new(
                egui::RichText::new("❌")
                    .size(32.0)
                    .color(egui::Color32::RED)
            )
        );
        ui.label(
            egui::RichText::new(format!("Portrait '{}' not found", portrait_id))
                .small()
                .color(egui::Color32::YELLOW)
        );
    });

    // If not already "0", try loading fallback portrait "0"
    if portrait_id != "0" {
        if let Some(fallback) = self.load_portrait_texture(ctx, campaign_dir, "0") {
            ui.add(egui::Image::new(fallback).fit_to_exact_size(egui::vec2(64.0, 64.0)));
        }
    }
}
ui.add_space(10.0);
```

#### 4.4 Testing Requirements

- Manual testing: verify portrait displays in preview panel
- Manual testing: verify placeholder shows for invalid portrait IDs
- Manual testing: verify image scales correctly

#### 4.5 Deliverables

- [ ] Updated `show_character_preview` with portrait display
- [ ] Updated method signatures and call sites
- [ ] Placeholder for missing portraits

#### 4.6 Success Criteria

- Character preview panel shows selected portrait image
- Placeholder displays for invalid/missing portraits
- Portrait renders at appropriate size (64-96px)

---

### Phase 5: Polish and Edge Cases

Final polish, error handling, and edge case fixes.

#### 5.1 Add Tooltip with Portrait Path

In the autocomplete selector, add tooltip showing full path on hover.

#### 5.2 Add Image Loading Error Handling

Wrap image loading in proper error handling with logging.

#### 5.3 Test All Character Editor Operations

Verify portrait support works with:
- New character creation
- Editing existing character
- Character list scrolling
- Save/load operations

#### 5.4 Testing Requirements

- Manual testing: complete workflow testing
- Verify no panics or crashes
- Run `cargo clippy` and `cargo test`

#### 5.5 Deliverables

- [ ] Tooltip enhancements
- [ ] Error handling improvements
- [ ] Full workflow testing completed
- [ ] Code quality checks pass

#### 5.6 Success Criteria

- All character operations work correctly with portrait support
- No compiler warnings or clippy errors
- Tests pass

---

## File Modification Summary

| File | Changes |
|------|---------|
| [ui_helpers.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/ui_helpers.rs) | Add `extract_portrait_candidates`, `resolve_portrait_path`, `autocomplete_portrait_selector` |
| [characters_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs) | Add portrait picker state, `load_portrait_texture`, `show_portrait_grid_picker`, update form and preview |

## Dependencies

- `egui::TextureHandle` for image display (already available via eframe/egui)
- `image` crate for image loading (check if already in dependencies, add if not)

## Design Decisions

1. **Portrait size** - Portraits are standardized at **64x64 pixels**
2. **Portrait format** - Support only what the game engine supports, prioritizing **PNG first**
3. **Fallback handling** - If a character references a non-existent portrait:
   - Default to portrait ID **"0"**
   - Show an **❌** indicator in the portrait preview to signal the issue
