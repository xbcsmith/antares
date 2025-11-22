# Campaign Builder Bug Fixes - Implementation Guide

## Overview

This document provides step-by-step implementation instructions for fixing the three critical bugs in the Campaign Builder UI.

**CRITICAL**: These bugs MUST be fixed before proceeding with Phase 3B implementation.

**Related Documents**:
- `docs/how-to/test_campaign_builder_ui.md` - Testing procedures
- `docs/explanation/campaign_builder_completion_plan.md` - Implementation roadmap

---

## Bug #1: Items and Monsters Not Saved to Campaign

### Problem Statement

When users save a campaign via `File → Save` or `Ctrl+S`, the campaign metadata (campaign.ron) is saved, but the actual data files (items.ron, monsters.ron, spells.ron, etc.) are NOT automatically saved. This means:

- Users add items → Save campaign → Close → Reopen → Items are gone
- Users add monsters → Save campaign → Close → Reopen → Monsters are gone

### Root Cause

The `do_save_campaign()` function only saves the campaign metadata. It does NOT call the individual save functions for items, monsters, spells, quests, or dialogues.

**Location**: `sdk/campaign_builder/src/main.rs` lines 1105-1128

### Fix Implementation

**File**: `sdk/campaign_builder/src/main.rs`

**Step 1**: Locate the `do_save_campaign()` function (around line 1105)

**Step 2**: Replace the function with this corrected version:

```rust
fn do_save_campaign(&mut self) -> Result<(), CampaignError> {
    let path = self.campaign_path.as_ref().ok_or(CampaignError::NoPath)?;

    // CRITICAL FIX: Save all data files BEFORE saving campaign metadata
    // This ensures all content is persisted when user clicks "Save Campaign"

    // Track any save failures but continue (partial save is better than no save)
    let mut save_warnings = Vec::new();

    if let Err(e) = self.save_items() {
        save_warnings.push(format!("Items: {}", e));
    }

    if let Err(e) = self.save_spells() {
        save_warnings.push(format!("Spells: {}", e));
    }

    if let Err(e) = self.save_monsters() {
        save_warnings.push(format!("Monsters: {}", e));
    }

    // Save maps individually (they're saved per-map, not as a collection)
    for (idx, map) in self.maps.iter().enumerate() {
        if let Err(e) = self.save_map(map) {
            save_warnings.push(format!("Map {}: {}", idx, e));
        }
    }

    if let Err(e) = self.save_quests() {
        save_warnings.push(format!("Quests: {}", e));
    }

    if let Err(e) = self.save_dialogues_to_file() {
        save_warnings.push(format!("Dialogues: {}", e));
    }

    // Now save campaign metadata to RON format
    let ron_config = ron::ser::PrettyConfig::new()
        .struct_names(true)
        .enumerate_arrays(false)
        .depth_limit(4);

    let ron_string = ron::ser::to_string_pretty(&self.campaign, ron_config)?;

    // Write campaign metadata file
    fs::write(path, ron_string)?;

    self.unsaved_changes = false;

    // Update status message based on results
    if save_warnings.is_empty() {
        self.status_message = format!("✅ Campaign and all data saved to: {}", path.display());
    } else {
        self.status_message = format!(
            "⚠️ Campaign saved with warnings:\n{}",
            save_warnings.join("\n")
        );
    }

    // Update file tree if we have a campaign directory
    if let Some(dir) = self.campaign_dir.clone() {
        self.update_file_tree(&dir);
    }

    Ok(())
}
```

**Step 3**: Verify the fix compiles

```bash
cd sdk/campaign_builder
cargo check
```

**Expected**: No compilation errors.

**Step 4**: Run unit tests

```bash
cargo test --lib test_save_campaign
```

**Expected**: Tests pass (may need to update test expectations).

### Testing the Fix

**Manual Test**:

1. Run campaign builder: `cargo run --release`
2. Create new campaign
3. Go to Items tab → Add item "Test Sword"
4. Go to Monsters tab → Add monster "Test Goblin"
5. Click `File → Save Campaign` (or Ctrl+S)
6. **VERIFY**: Status message shows "✅ Campaign and all data saved"
7. Close application completely
8. Reopen application
9. Open the same campaign
10. Go to Items tab → **VERIFY**: "Test Sword" is present
11. Go to Monsters tab → **VERIFY**: "Test Goblin" is present

**Success Criteria**: Items and monsters persist after save/load cycle.

### Additional Validation

Check that files are actually written:

```bash
# After saving campaign
ls -lh test_campaign/
# Should show:
# campaign.ron (non-zero size)
# items.ron (non-zero size)
# monsters.ron (non-zero size)
# spells.ron (non-zero size)

# Verify RON content is valid
cat test_campaign/items.ron
# Should show valid RON array of items
```

---

## Bug #2: ID Clashes in Items and Monsters Tabs UI

### Problem Statement

When switching between Items, Monsters, and Spells tabs, combo boxes and dropdowns stop working, freeze, or behave erratically. This is caused by egui ID collisions.

### Root Cause

egui requires unique IDs for all interactive widgets. When using `ComboBox::from_label("text")` with the same label text in multiple locations, egui generates identical IDs, causing conflicts.

**Locations affected**:
- Items tab: `show_items_editor()` around line 2245
- Monsters tab: `show_monsters_editor()` around line 3529
- Spells tab: `show_spells_editor()` around line 3116

### Fix Implementation

**File**: `sdk/campaign_builder/src/main.rs`

**Step 1**: Fix Items Tab (around line 2245)

Locate this code in `show_items_editor()`:

```rust
egui::ComboBox::from_label("item_type_filter")
    .selected_text(...)
    .show_ui(ui, |ui| { ... });
```

Replace with:

```rust
egui::ComboBox::from_id_salt("items_filter_type_combo")
    .selected_text(
        self.items_filter_type
            .map(|f| f.as_str().to_string())
            .unwrap_or_else(|| "All Types".to_string()),
    )
    .show_ui(ui, |ui| {
        if ui
            .selectable_label(self.items_filter_type.is_none(), "All Types")
            .clicked()
        {
            self.items_filter_type = None;
        }
        for filter in ItemTypeFilter::all() {
            if ui
                .selectable_value(
                    &mut self.items_filter_type,
                    Some(filter),
                    filter.as_str(),
                )
                .clicked()
            {}
        }
    });
```

**Step 2**: Fix Spells Tab (around line 3116)

Locate in `show_spells_editor()`:

```rust
egui::ComboBox::from_label("School")
    .selected_text(...)
```

Replace with:

```rust
egui::ComboBox::from_id_salt("spells_school_filter_combo")
    .selected_text(...)
```

Locate the level filter combo box:

```rust
egui::ComboBox::from_label("Level")
    .selected_text(...)
```

Replace with:

```rust
egui::ComboBox::from_id_salt("spells_level_filter_combo")
    .selected_text(...)
```

**Step 3**: Fix Monsters Tab (around line 3529)

Search for all `ComboBox::from_label()` calls in `show_monsters_editor()` and related functions.

Replace each with unique ID salts:

```rust
// Example patterns to replace:
from_label("xxx") → from_id_salt("monsters_xxx_combo")
from_label("yyy") → from_id_salt("monsters_yyy_filter_combo")
```

**Step 4**: Fix Map Editor Tool Palette (if any combo boxes)

Check `sdk/campaign_builder/src/map_editor.rs` around line 978-1106.

The terrain and wall combo boxes should use:

```rust
// Terrain combo
egui::ComboBox::from_id_salt("map_terrain_palette_combo")
    .selected_text(...)

// Wall combo
egui::ComboBox::from_id_salt("map_wall_palette_combo")
    .selected_text(...)
```

**Step 5**: Verify the fix compiles

```bash
cd sdk/campaign_builder
cargo check
```

**Expected**: No compilation errors.

### Testing the Fix

**Manual Test**:

1. Run: `cargo run --release`
2. Open or create a campaign
3. Go to **Items** tab
4. Click "Item Type Filter" dropdown → Select "Weapon"
5. **VERIFY**: Dropdown works, "Weapon" is selected
6. Go to **Monsters** tab
7. Click any dropdown in Monsters tab
8. **VERIFY**: Dropdown works normally
9. Go back to **Items** tab
10. Click "Item Type Filter" dropdown again
11. **VERIFY**: Dropdown still works (no freeze, no crash)
12. Switch between tabs 5-10 times randomly
13. **VERIFY**: All dropdowns continue working

**Success Criteria**: No UI freezes, no crashes, all combo boxes work regardless of tab switching.

### Additional Validation

Add logging to confirm unique IDs:

```rust
// Temporary debug code
eprintln!("Items combo ID: items_filter_type_combo");
egui::ComboBox::from_id_salt("items_filter_type_combo")
    .selected_text(...)
```

Check console output - should see different IDs for different combos.

---

## Bug #3: Map Tools Selector - Terrain and Wall Reset Each Other

### Problem Statement

When editing a map, users select a terrain type (e.g., "Grass"), then select a wall type (e.g., "Normal Wall"). However, selecting the wall type causes the terrain selection to reset, and vice versa. Users cannot paint tiles with BOTH terrain and wall simultaneously.

### Root Cause

The `MapEditorState::current_tool` field is an enum that can only hold ONE tool at a time:

```rust
pub enum EditorTool {
    Select,
    PaintTerrain(TerrainType),  // Holds terrain
    PaintWall(WallType),        // Holds wall
    PlaceEvent,
    PlaceNpc,
    Fill,
    Erase,
}
```

When user selects a terrain, `current_tool = PaintTerrain(Grass)`. When they select a wall, `current_tool = PaintWall(Normal)`, which OVERWRITES the terrain selection.

### Design Decision

We need to separate terrain and wall selections from the tool selection. The tool should be "Paint Tile", and the terrain/wall choices should be separate state fields.

### Fix Implementation

**File**: `sdk/campaign_builder/src/map_editor.rs`

**Step 1**: Update `MapEditorState` struct (around line 222)

Locate:

```rust
pub struct MapEditorState {
    pub map: Map,
    pub metadata: MapMetadata,
    pub current_tool: EditorTool,
    pub selected_position: Option<(u32, u32)>,
    // ... rest
}
```

Add new fields:

```rust
pub struct MapEditorState {
    pub map: Map,
    pub metadata: MapMetadata,
    pub current_tool: EditorTool,
    pub selected_position: Option<(u32, u32)>,

    // NEW: Separate terrain and wall selections
    pub selected_terrain: TerrainType,
    pub selected_wall: WallType,

    undo_stack: UndoStack,
    // ... rest
}
```

**Step 2**: Update `MapEditorState::new()` (around line 255)

Add initialization for new fields:

```rust
pub fn new(map: Map, file_path: Option<PathBuf>) -> Self {
    Self {
        map,
        metadata: MapMetadata::default(),
        current_tool: EditorTool::Select,
        selected_position: None,

        // NEW: Initialize with sensible defaults
        selected_terrain: TerrainType::Ground,
        selected_wall: WallType::None,

        undo_stack: UndoStack::new(),
        has_changes: false,
        file_path,
        validation_errors: Vec::new(),
        show_grid: true,
        show_events: true,
        show_npcs: true,
        event_editor: None,
        npc_editor: None,
        show_metadata_editor: false,
    }
}
```

**Step 3**: Update `EditorTool` enum (around line 47)

Replace:

```rust
pub enum EditorTool {
    Select,
    PaintTerrain(TerrainType),  // Remove payload
    PaintWall(WallType),        // Remove payload
    PlaceEvent,
    PlaceNpc,
    Fill,
    Erase,
}
```

With:

```rust
pub enum EditorTool {
    Select,
    PaintTile,  // NEW: Unified painting tool (uses selected_terrain + selected_wall)
    PlaceEvent,
    PlaceNpc,
    Fill,
    Erase,
}
```

**Step 4**: Update `EditorTool::name()` and `icon()` methods (around line 66-89)

```rust
pub fn name(&self) -> &str {
    match self {
        EditorTool::Select => "Select",
        EditorTool::PaintTile => "Paint Tile",  // Updated
        EditorTool::PlaceEvent => "Place Event",
        EditorTool::PlaceNpc => "Place NPC",
        EditorTool::Fill => "Fill",
        EditorTool::Erase => "Erase",
    }
}

pub fn icon(&self) -> &str {
    match self {
        EditorTool::Select => "👆",
        EditorTool::PaintTile => "🖌️",  // Updated
        EditorTool::PlaceEvent => "🎯",
        EditorTool::PlaceNpc => "🧙",
        EditorTool::Fill => "🪣",
        EditorTool::Erase => "🧹",
    }
}
```

**Step 5**: Update painting methods (around line 302-322)

Replace `paint_terrain()` and `paint_wall()` with unified method:

```rust
/// Paints a tile with the currently selected terrain and wall
pub fn paint_tile(&mut self, position: (u32, u32)) {
    let old_tile = self.map.get_tile(position);

    let new_tile = Tile {
        terrain: self.selected_terrain,
        wall: self.selected_wall,
        ..old_tile.unwrap_or_default()
    };

    self.set_tile(position, new_tile);
}

// Keep these for backward compatibility / undo system
fn paint_terrain(&mut self, position: (u32, u32), terrain: TerrainType) {
    let old_tile = self.map.get_tile(position);
    let new_tile = Tile {
        terrain,
        ..old_tile.unwrap_or_default()
    };
    self.set_tile(position, new_tile);
}

fn paint_wall(&mut self, position: (u32, u32), wall: WallType) {
    let old_tile = self.map.get_tile(position);
    let new_tile = Tile {
        wall,
        ..old_tile.unwrap_or_default()
    };
    self.set_tile(position, new_tile);
}
```

**Step 6**: Update tool palette UI (around line 978-1106)

Replace the entire `show_tool_palette()` function:

```rust
fn show_tool_palette(&mut self, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label("Tools:");

        if ui
            .selectable_label(
                matches!(self.state.current_tool, EditorTool::Select),
                format!("{} Select", EditorTool::Select.icon()),
            )
            .clicked()
        {
            self.state.current_tool = EditorTool::Select;
        }

        if ui
            .selectable_label(
                matches!(self.state.current_tool, EditorTool::PaintTile),
                format!("{} Paint", EditorTool::PaintTile.icon()),
            )
            .clicked()
        {
            self.state.current_tool = EditorTool::PaintTile;
        }

        ui.separator();

        if ui
            .selectable_label(
                matches!(self.state.current_tool, EditorTool::PlaceEvent),
                format!("{} Event", EditorTool::PlaceEvent.icon()),
            )
            .clicked()
        {
            self.state.current_tool = EditorTool::PlaceEvent;
        }

        if ui
            .selectable_label(
                matches!(self.state.current_tool, EditorTool::PlaceNpc),
                format!("{} NPC", EditorTool::PlaceNpc.icon()),
            )
            .clicked()
        {
            self.state.current_tool = EditorTool::PlaceNpc;
        }

        ui.separator();

        if ui
            .selectable_label(
                matches!(self.state.current_tool, EditorTool::Erase),
                format!("{} Erase", EditorTool::Erase.icon()),
            )
            .clicked()
        {
            self.state.current_tool = EditorTool::Erase;
        }

        ui.separator();

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(self.state.can_redo(), egui::Button::new("↪ Redo"))
                .clicked()
            {
                self.state.redo();
            }

            if ui
                .add_enabled(self.state.can_undo(), egui::Button::new("↩ Undo"))
                .clicked()
            {
                self.state.undo();
            }
        });
    });

    // NEW: Separate row for terrain and wall selection (always visible)
    ui.horizontal(|ui| {
        ui.label("Terrain:");

        egui::ComboBox::from_id_salt("map_terrain_palette_combo")
            .selected_text(format!("{:?}", self.state.selected_terrain))
            .show_ui(ui, |ui| {
                for terrain in &[
                    TerrainType::Ground,
                    TerrainType::Grass,
                    TerrainType::Water,
                    TerrainType::Stone,
                    TerrainType::Dirt,
                    TerrainType::Forest,
                    TerrainType::Mountain,
                    TerrainType::Swamp,
                    TerrainType::Lava,
                ] {
                    ui.selectable_value(
                        &mut self.state.selected_terrain,
                        *terrain,
                        format!("{:?}", terrain),
                    );
                }
            });

        ui.separator();
        ui.label("Wall:");

        egui::ComboBox::from_id_salt("map_wall_palette_combo")
            .selected_text(format!("{:?}", self.state.selected_wall))
            .show_ui(ui, |ui| {
                for wall in &[
                    WallType::None,
                    WallType::Normal,
                    WallType::Door,
                    WallType::Torch,
                ] {
                    ui.selectable_value(
                        &mut self.state.selected_wall,
                        *wall,
                        format!("{:?}", wall),
                    );
                }
            });
    });

    // View options
    ui.horizontal(|ui| {
        ui.label("View:");
        ui.checkbox(&mut self.state.show_grid, "Grid");
        ui.checkbox(&mut self.state.show_events, "Events");
        ui.checkbox(&mut self.state.show_npcs, "NPCs");
    });
}
```

**Step 7**: Update grid click handler

In `MapGridWidget::ui()` or wherever tile clicks are handled, update to use `paint_tile()`:

```rust
// When user clicks a tile with PaintTile tool selected:
if self.state.current_tool == EditorTool::PaintTile {
    self.state.paint_tile(clicked_position);
}
```

**Step 8**: Verify the fix compiles

```bash
cd sdk/campaign_builder
cargo check
```

**Expected**: Compilation errors in tests that reference old enum variants. Update tests to use new API.

**Step 9**: Update tests

Find tests that reference `EditorTool::PaintTerrain` or `EditorTool::PaintWall`:

```rust
// OLD:
state.current_tool = EditorTool::PaintTerrain(TerrainType::Grass);

// NEW:
state.current_tool = EditorTool::PaintTile;
state.selected_terrain = TerrainType::Grass;
state.selected_wall = WallType::None;
```

Run tests:

```bash
cargo test --lib
```

**Expected**: All tests pass.

### Testing the Fix

**Manual Test**:

1. Run: `cargo run --release`
2. Open or create a campaign
3. Go to **Maps** tab
4. Click "➕ Create Map" (or edit existing)
5. Map editor opens
6. Select **"Grass"** from Terrain dropdown
7. Select **"Normal"** from Wall dropdown
8. **VERIFY**: Both selections remain active (check combo box displays)
9. Click on a tile in the grid
10. Select the tile and view inspector panel
11. **VERIFY**: Tile shows `terrain: Grass` AND `wall: Normal`
12. Change terrain to "Water" (keep wall as "Normal")
13. Paint another tile
14. **VERIFY**: New tile shows `terrain: Water` AND `wall: Normal`
15. Change wall to "Door" (keep terrain as "Water")
16. Paint another tile
17. **VERIFY**: Tile shows `terrain: Water` AND `wall: Door`

**Success Criteria**: Users can select both terrain and wall types independently, and painted tiles reflect BOTH selections.

### Additional Validation

Check undo/redo works:

1. Paint 5 tiles with different terrain/wall combinations
2. Click "Undo" 3 times
3. **VERIFY**: Last 3 tiles revert to previous state
4. Click "Redo" 2 times
5. **VERIFY**: Tiles repaint correctly

---

## Verification Checklist

After implementing all fixes, verify:

- [ ] Bug #1 Fixed: Items and monsters persist after save/load
- [ ] Bug #2 Fixed: No UI ID clashes when switching tabs
- [ ] Bug #3 Fixed: Terrain and wall selections don't reset each other
- [ ] All unit tests pass: `cargo test --lib`
- [ ] No clippy warnings: `cargo clippy --all-targets -- -D warnings`
- [ ] Code formatted: `cargo fmt --all`
- [ ] Manual test suite (from test_campaign_builder_ui.md) passes
- [ ] No regressions in existing functionality

---

## Rollout Plan

1. **Create feature branch**: `git checkout -b fix/campaign-builder-critical-bugs`
2. **Apply Bug #1 fix** → Test → Commit
3. **Apply Bug #2 fix** → Test → Commit
4. **Apply Bug #3 fix** → Test → Commit
5. **Run full test suite** (unit + manual)
6. **Update documentation** (`docs/explanation/implementations.md`)
7. **Create pull request** with test results
8. **Merge to main** after review

---

## Post-Fix Actions

After all fixes are merged:

1. Update `campaign_builder_completion_plan.md` to mark these issues resolved
2. Update `implementations.md` with fix details
3. Add regression tests to prevent these bugs from reoccurring
4. Continue with Phase 3B implementation (Items Editor Enhancement)

---

## Success Metrics

- Campaign save/load works 100% reliably
- No UI freezes or crashes during normal usage
- Map editor allows intuitive tile painting with terrain + wall
- User confidence in data persistence restored
- Ready to proceed with Phase 3B development

---

## Conclusion

These three bugs are CRITICAL and block user adoption. Once fixed and verified, the Campaign Builder becomes a reliable tool for content creation.

**NEXT STEP**: Apply these fixes, verify with manual testing, then proceed to Phase 3B implementation per the completion plan.
