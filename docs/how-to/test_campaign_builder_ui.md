# How to Test Campaign Builder UI

## Overview

This guide provides comprehensive testing procedures for the Campaign Builder UI,
including manual test cases, automated test setup, and validation procedures.

**Related Documents**:
- `docs/explanation/campaign_builder_completion_plan.md` - Implementation roadmap
- `sdk/campaign_builder/QUICKSTART.md` - Getting started guide

## Known Critical Bugs (Must Fix First)

### Bug 1: Items and Monsters Not Saved to Campaign

**Root Cause**: Save operations call individual `save_items()` and `save_monsters()`
functions, but these are NOT automatically called when the campaign is saved via
`do_save_campaign()`. The campaign metadata is saved, but the actual data files are not.

**Location**: `sdk/campaign_builder/src/main.rs:1105-1128`

**Fix Required**:

```rust
fn do_save_campaign(&mut self) -> Result<(), CampaignError> {
    let path = self.campaign_path.as_ref().ok_or(CampaignError::NoPath)?;

    // NEW: Save all data files BEFORE saving campaign metadata
    if let Err(e) = self.save_items() {
        self.status_message = format!("Warning: Failed to save items: {}", e);
    }
    if let Err(e) = self.save_spells() {
        self.status_message = format!("Warning: Failed to save spells: {}", e);
    }
    if let Err(e) = self.save_monsters() {
        self.status_message = format!("Warning: Failed to save monsters: {}", e);
    }
    if let Err(e) = self.save_quests() {
        self.status_message = format!("Warning: Failed to save quests: {}", e);
    }
    if let Err(e) = self.save_dialogues_to_file() {
        self.status_message = format!("Warning: Failed to save dialogues: {}", e);
    }

    // Serialize campaign metadata to RON format
    let ron_config = ron::ser::PrettyConfig::new()
        .struct_names(true)
        .enumerate_arrays(false)
        .depth_limit(4);

    let ron_string = ron::ser::to_string_pretty(&self.campaign, ron_config)?;
    fs::write(path, ron_string)?;

    self.unsaved_changes = false;
    self.status_message = format!("Campaign and all data saved to: {}", path.display());

    // Update file tree
    if let Some(dir) = self.campaign_dir.clone() {
        self.update_file_tree(&dir);
    }

    Ok(())
}
```

**Test Case**:
1. Create new campaign
2. Add 2-3 items
3. Add 1-2 monsters
4. Save campaign (Ctrl+S or File → Save)
5. Close campaign builder
6. Reopen campaign
7. **VERIFY**: Items and monsters appear in their respective tabs

---

### Bug 2: ID Clashes in Items and Monsters Tabs UI

**Root Cause**: egui requires unique IDs for interactive widgets. When using
`ComboBox::from_label()` with the same label text in multiple places, egui
generates ID collisions.

**Location**:
- `sdk/campaign_builder/src/main.rs:2245-2288` (Items editor)
- Similar pattern in Monsters editor at lines 3529-3572

**Fix Required**: Use `ComboBox::from_id_salt()` with unique salt strings instead
of `from_label()`.

```rust
// WRONG (causes ID clash):
egui::ComboBox::from_label("🎨 Terrain")
    .selected_text(...)
    .show_ui(ui, |ui| { ... });

// CORRECT (unique ID per combo box):
egui::ComboBox::from_id_salt("items_type_filter_combo")
    .selected_text(...)
    .show_ui(ui, |ui| { ... });
```

**Locations to Fix**:

1. **Items Editor** (`show_items_editor`, line ~2245):
   - Change `from_label("item_type_filter")` → `from_id_salt("items_filter_type_combo")`

2. **Monsters Editor** (`show_monsters_editor`):
   - Any combo boxes need unique IDs: `from_id_salt("monsters_xxx_combo")`

3. **Spells Editor** (`show_spells_editor`, line ~3116):
   - Change `from_label()` → `from_id_salt("spells_school_filter_combo")`
   - Change `from_label()` → `from_id_salt("spells_level_filter_combo")`

**Test Case**:
1. Open Items tab
2. Click on "Item Type Filter" dropdown → should work
3. Switch to Monsters tab
4. Click on any dropdown in Monsters tab
5. Switch back to Items tab
6. **VERIFY**: Item Type Filter still works correctly (no UI freeze/crash)

---

### Bug 3: Map Tools Selector - Terrain and Wall Reset Each Other

**Root Cause**: The `current_tool` field in `MapEditorState` uses an enum that can
only hold one tool at a time. When selecting a terrain from the Terrain combo box,
it sets `current_tool = EditorTool::PaintTerrain(terrain)`. When selecting a wall
type, it sets `current_tool = EditorTool::PaintWall(wall)`, overwriting the terrain.

**Location**: `sdk/campaign_builder/src/map_editor.rs:978-1106`

**Design Issue**: The current design treats "Paint Terrain" and "Paint Wall" as
mutually exclusive tools, but users expect to select BOTH a terrain type AND a
wall type, then paint tiles with that combination.

**Fix Options**:

**Option A: Separate Fields (Recommended)**

```rust
pub struct MapEditorState {
    pub map: Map,
    pub current_tool: EditorTool,  // Select, PlaceEvent, PlaceNpc, Fill, Erase
    pub selected_terrain: TerrainType,  // NEW: Current terrain for painting
    pub selected_wall: WallType,        // NEW: Current wall for painting
    // ... rest of fields
}

// Update EditorTool enum to remove terrain/wall data:
pub enum EditorTool {
    Select,
    PaintTile,  // NEW: Combines terrain + wall painting
    PlaceEvent,
    PlaceNpc,
    Fill,
    Erase,
}

// Update paint methods:
pub fn paint_tile(&mut self, position: (u32, u32)) {
    let tile = Tile {
        terrain: self.selected_terrain,
        wall: self.selected_wall,
        ..Default::default()
    };
    self.set_tile(position, tile);
}
```

**Option B: Radio Button Groups (Alternative)**

Keep current design but make it clear that Terrain and Wall are separate tools:

```rust
// In show_tool_palette:
ui.horizontal(|ui| {
    ui.label("Paint Mode:");

    if ui.radio_value(&mut paint_mode, PaintMode::Terrain, "🎨 Terrain").clicked() {
        // Show terrain selector below
    }

    if ui.radio_value(&mut paint_mode, PaintMode::Wall, "🧱 Wall").clicked() {
        // Show wall selector below
    }
});

// Show selector based on mode
match paint_mode {
    PaintMode::Terrain => { /* terrain combo box */ }
    PaintMode::Wall => { /* wall combo box */ }
}
```

**Recommended**: Use Option A, as it matches user expectations (select both terrain
and wall, then paint).

**Test Case After Fix**:
1. Open Maps tab
2. Create or edit a map
3. Select "Grass" from Terrain combo box
4. Select "Normal" from Wall combo box
5. Click on map grid
6. **VERIFY**: Tile shows both Grass terrain AND Normal wall (not one or the other)

---

## Manual Testing Checklist

### Pre-Testing Setup

```bash
# Build campaign builder
cd sdk/campaign_builder
cargo build --release

# Run campaign builder
cargo run --release

# Or install and run
cargo install --path .
campaign_builder
```

### Test Suite 1: Campaign Lifecycle

**Test 1.1: Create New Campaign**

- [ ] Click File → New Campaign
- [ ] Enter campaign name: "Test Campaign"
- [ ] Enter author: "Test User"
- [ ] Set difficulty to "Normal"
- [ ] **VERIFY**: Campaign metadata appears in Metadata tab
- [ ] **VERIFY**: Status message shows "New campaign created"

**Test 1.2: Save Campaign**

- [ ] Click File → Save Campaign (or Ctrl+S)
- [ ] Choose directory: `test_campaigns/test_01`
- [ ] **VERIFY**: Status shows "Campaign saved to: [path]"
- [ ] **VERIFY**: File tree shows campaign.ron file
- [ ] **VERIFY**: Directory contains: campaign.ron, items.ron, spells.ron, monsters.ron

**Test 1.3: Load Campaign**

- [ ] Click File → Open Campaign
- [ ] Select `test_campaigns/test_01/campaign.ron`
- [ ] **VERIFY**: Campaign metadata loads correctly
- [ ] **VERIFY**: File tree populates
- [ ] **VERIFY**: No validation errors appear

**Test 1.4: Close and Reopen**

- [ ] Add 2-3 items (see Test 2.1)
- [ ] Save campaign
- [ ] Close application
- [ ] Reopen application
- [ ] Open same campaign
- [ ] **VERIFY**: Items persist correctly
- [ ] **VERIFY**: Monsters persist correctly
- [ ] **VERIFY**: All metadata unchanged

---

### Test Suite 2: Items Editor

**Test 2.1: Add Weapon Item**

- [ ] Go to Items tab
- [ ] Click "➕ Add Item"
- [ ] Set Name: "Longsword"
- [ ] Set Base Cost: 100
- [ ] Set Sell Cost: 50
- [ ] Select "⚔️ Weapon" type
- [ ] Set Damage: 1d8+0
- [ ] Set Hands Required: 1
- [ ] Click "💾 Save"
- [ ] **VERIFY**: Item appears in list
- [ ] **VERIFY**: Item ID is unique (check validation panel)

**Test 2.2: Add Armor Item**

- [ ] Click "➕ Add Item"
- [ ] Set Name: "Leather Armor"
- [ ] Select "🛡️ Armor" type
- [ ] Set AC Bonus: 2
- [ ] Set Weight: 10
- [ ] Click "💾 Save"
- [ ] **VERIFY**: Item appears in list with correct icon

**Test 2.3: Edit Existing Item**

- [ ] Click on "Longsword" in list
- [ ] Change Base Cost to 150
- [ ] Click "💾 Save"
- [ ] **VERIFY**: Item updates in list
- [ ] **VERIFY**: unsaved_changes flag set to true

**Test 2.4: Delete Item**

- [ ] Select "Leather Armor"
- [ ] Click "🗑️ Delete"
- [ ] **VERIFY**: Item removed from list
- [ ] **VERIFY**: Item count decreases

**Test 2.5: Filter Items**

- [ ] Add 5+ items of different types
- [ ] Set filter to "Weapon"
- [ ] **VERIFY**: Only weapons show in list
- [ ] Click "Cursed" filter
- [ ] Mark one item as cursed
- [ ] **VERIFY**: Only cursed items show

**Test 2.6: Search Items**

- [ ] Enter "sword" in search box
- [ ] **VERIFY**: Only items with "sword" in name appear
- [ ] Clear search
- [ ] **VERIFY**: All items reappear

**Test 2.7: ID Uniqueness Validation**

- [ ] Add item with ID 1
- [ ] Manually create items.ron with duplicate ID 1
- [ ] Reload campaign
- [ ] **VERIFY**: Validation error appears in Validation tab
- [ ] **VERIFY**: Error message indicates duplicate ID

---

### Test Suite 3: Monsters Editor

**Test 3.1: Add Basic Monster**

- [ ] Go to Monsters tab
- [ ] Click "➕ Add Monster"
- [ ] Set Name: "Goblin"
- [ ] Set Level: 1
- [ ] Set HP: 10
- [ ] Set AC: 12
- [ ] Click "💾 Save"
- [ ] **VERIFY**: Monster appears in list

**Test 3.2: Add Monster Attacks**

- [ ] Edit "Goblin"
- [ ] Click "Add Attack"
- [ ] Set Attack Name: "Short Sword"
- [ ] Set Damage: 1d6
- [ ] Set Hit Bonus: +2
- [ ] Click "💾 Save"
- [ ] **VERIFY**: Attack shows in monster preview

**Test 3.3: Add Monster Loot**

- [ ] Edit "Goblin"
- [ ] Add loot entry: Item ID 1, Drop Chance 0.5
- [ ] Click "💾 Save"
- [ ] **VERIFY**: Loot table displays correctly

**Test 3.4: Monster XP Calculation**

- [ ] Create monster with:
   - Level: 5
   - HP: 50
   - AC: 15
   - 2 attacks
- [ ] **VERIFY**: XP value calculates automatically
- [ ] **VERIFY**: XP appears in monster preview panel

---

### Test Suite 4: Spells Editor

**Test 4.1: Add Spell**

- [ ] Go to Spells tab
- [ ] Click "➕ Add Spell"
- [ ] Set Name: "Fireball"
- [ ] Set School: Sorcerer
- [ ] Set Level: 3
- [ ] Set SP Cost: 15
- [ ] Set Context: Combat
- [ ] Set Target: Enemy Group
- [ ] Click "💾 Save"
- [ ] **VERIFY**: Spell appears in list

**Test 4.2: Filter Spells by School**

- [ ] Add spells from different schools
- [ ] Set school filter to "Cleric"
- [ ] **VERIFY**: Only Cleric spells show
- [ ] Clear filter
- [ ] **VERIFY**: All spells reappear

**Test 4.3: Filter Spells by Level**

- [ ] Set level filter to "1-3"
- [ ] **VERIFY**: Only levels 1-3 spells show
- [ ] Set to "4-7"
- [ ] **VERIFY**: Only levels 4-7 spells show

---

### Test Suite 5: Maps Editor

**Test 5.1: Create New Map**

- [ ] Go to Maps tab
- [ ] Click "➕ Create Map"
- [ ] Set Width: 20
- [ ] Set Height: 20
- [ ] Set Name: "Test Dungeon"
- [ ] Click "Create"
- [ ] **VERIFY**: Map editor opens
- [ ] **VERIFY**: Grid displays correctly

**Test 5.2: Paint Terrain (After Bug Fix)**

- [ ] Select "Grass" from Terrain dropdown
- [ ] Click on map tiles
- [ ] **VERIFY**: Tiles turn to Grass terrain
- [ ] Select "Water" from Terrain dropdown
- [ ] Paint more tiles
- [ ] **VERIFY**: New tiles become Water

**Test 5.3: Paint Walls (After Bug Fix)**

- [ ] Select "Normal" from Wall dropdown
- [ ] Click on tiles
- [ ] **VERIFY**: Walls appear on tiles
- [ ] Select "Door" from Wall dropdown
- [ ] Click on tiles
- [ ] **VERIFY**: Doors appear

**Test 5.4: Combined Terrain + Wall (After Bug Fix)**

- [ ] Select "Stone" terrain
- [ ] Select "Torch" wall
- [ ] Click on tile
- [ ] **VERIFY**: Tile shows BOTH stone terrain AND torch wall
- [ ] Undo (Ctrl+Z)
- [ ] **VERIFY**: Tile reverts to previous state

**Test 5.5: Place Events**

- [ ] Select "🎯 Event" tool
- [ ] Click on tile
- [ ] Select event type: "Encounter"
- [ ] Add monster IDs
- [ ] Click "Save Event"
- [ ] **VERIFY**: Event icon appears on tile
- [ ] **VERIFY**: Event shows in inspector panel

**Test 5.6: Undo/Redo**

- [ ] Paint 5 tiles
- [ ] Click Undo 3 times
- [ ] **VERIFY**: Last 3 tiles revert
- [ ] Click Redo 2 times
- [ ] **VERIFY**: 2 tiles repaint

**Test 5.7: Save Map**

- [ ] Click "💾 Save" in map editor
- [ ] Return to Maps list
- [ ] **VERIFY**: Map appears in list
- [ ] **VERIFY**: Map file exists in campaign directory

---

### Test Suite 6: Quests Editor

**Test 6.1: Create Quest**

- [ ] Go to Quests tab
- [ ] Click "➕ Add Quest"
- [ ] Set Name: "Kill 5 Goblins"
- [ ] Set Description: "Defeat goblin raiders"
- [ ] Set Quest Giver: "Village Elder"
- [ ] Click "💾 Save"
- [ ] **VERIFY**: Quest appears in list

**Test 6.2: Add Quest Stages**

- [ ] Edit quest
- [ ] Click "Add Stage"
- [ ] Set Stage Name: "Find Goblin Camp"
- [ ] Set Description: "Locate the raiders"
- [ ] Click "Add Stage" again
- [ ] Set Stage Name: "Defeat Goblins"
- [ ] **VERIFY**: Both stages show in stages list

**Test 6.3: Add Quest Objectives**

- [ ] Select stage "Defeat Goblins"
- [ ] Click "Add Objective"
- [ ] Set Type: "Kill Monsters"
- [ ] Set Target: Monster ID 1 (Goblin)
- [ ] Set Count: 5
- [ ] Click "Save"
- [ ] **VERIFY**: Objective displays correctly

**Test 6.4: Quest Validation**

- [ ] Create quest with invalid monster ID
- [ ] Run validation
- [ ] **VERIFY**: Error shows in validation panel
- [ ] **VERIFY**: Error indicates missing monster reference

---

### Test Suite 7: Dialogues Editor

**Test 7.1: Create Dialogue Tree**

- [ ] Go to Dialogues tab
- [ ] Click "➕ Add Dialogue"
- [ ] Set Tree ID: "elder_greeting"
- [ ] Set NPC Name: "Village Elder"
- [ ] Click "💾 Save"
- [ ] **VERIFY**: Dialogue appears in list

**Test 7.2: Add Dialogue Nodes**

- [ ] Edit dialogue tree
- [ ] Click "Add Node"
- [ ] Set Node ID: "node_1"
- [ ] Set Text: "Greetings, adventurer!"
- [ ] Click "Add Choice"
- [ ] Set Choice Text: "Hello"
- [ ] Set Next Node: "node_2"
- [ ] **VERIFY**: Choice appears in node

**Test 7.3: Dialogue Tree Validation**

- [ ] Create node with next_node pointing to non-existent node
- [ ] Run validation
- [ ] **VERIFY**: Error shows "Broken dialogue link"

---

### Test Suite 8: Asset Manager

**Test 8.1: Browse Assets**

- [ ] Go to Assets tab
- [ ] Click "Icons" category
- [ ] **VERIFY**: List of icon files appears
- [ ] Click "Sprites" category
- [ ] **VERIFY**: List of sprite files appears

**Test 8.2: Asset Reference Tracking**

- [ ] Click "Scan References" button
- [ ] **VERIFY**: Scan completes without errors
- [ ] **VERIFY**: Used/unused asset counts display

**Test 8.3: Unreferenced Assets**

- [ ] Add unused icon file to assets/icons/
- [ ] Run reference scan
- [ ] **VERIFY**: Icon appears in "Unreferenced" list

---

### Test Suite 9: Validation Panel

**Test 9.1: Empty Campaign Validation**

- [ ] Create new empty campaign
- [ ] Go to Validation tab
- [ ] **VERIFY**: Shows "✅ No validation errors"

**Test 9.2: ID Conflict Detection**

- [ ] Create 2 items with same ID
- [ ] Run validation
- [ ] **VERIFY**: Error shows "Duplicate item ID: X"
- [ ] **VERIFY**: Severity is "Error" (red)

**Test 9.3: Missing Reference Detection**

- [ ] Create quest referencing non-existent monster
- [ ] Run validation
- [ ] **VERIFY**: Error shows "Quest references missing monster ID: X"

**Test 9.4: Balance Warnings**

- [ ] Create monster with Level 1 but 1000 HP
- [ ] Run advanced validation
- [ ] **VERIFY**: Warning shows "Monster HP unusually high for level"

---

### Test Suite 10: Export and Packaging

**Test 10.1: Export Campaign**

- [ ] Click File → Export Campaign
- [ ] Select output directory
- [ ] Click "Export"
- [ ] **VERIFY**: Status shows "Campaign exported to [path]"
- [ ] **VERIFY**: Exported directory contains all .ron files

**Test 10.2: Package for Distribution**

- [ ] Click Tools → Package Campaign
- [ ] Set package name
- [ ] Click "Create Package"
- [ ] **VERIFY**: .zip or .tar.gz file created
- [ ] Extract package
- [ ] **VERIFY**: All files present and valid RON format

---

## Automated Testing

### Unit Tests

Run existing unit tests:

```bash
cd sdk/campaign_builder
cargo test --lib
```

**Expected**: All tests pass. Current test count: 100+ tests.

### Integration Tests

Create integration test file: `sdk/campaign_builder/tests/integration_test.rs`

```rust
use std::fs;
use std::path::PathBuf;

#[test]
fn test_campaign_save_load_roundtrip() {
    let temp_dir = tempfile::tempdir().unwrap();
    let campaign_path = temp_dir.path().join("test_campaign.ron");

    // Create campaign
    let mut app = CampaignBuilderApp::default();
    app.campaign.name = "Test Campaign".to_string();
    app.campaign_path = Some(campaign_path.clone());
    app.campaign_dir = Some(temp_dir.path().to_path_buf());

    // Add test data
    app.items.push(/* test item */);
    app.monsters.push(/* test monster */);

    // Save campaign
    app.do_save_campaign().expect("Save should succeed");

    // Verify files exist
    assert!(campaign_path.exists());
    assert!(temp_dir.path().join("items.ron").exists());
    assert!(temp_dir.path().join("monsters.ron").exists());

    // Load campaign
    let mut app2 = CampaignBuilderApp::default();
    app2.do_open_campaign(&campaign_path).expect("Load should succeed");

    // Verify data matches
    assert_eq!(app2.campaign.name, "Test Campaign");
    assert_eq!(app2.items.len(), 1);
    assert_eq!(app2.monsters.len(), 1);
}

#[test]
fn test_items_persist_after_save() {
    // Similar to above but specifically tests Bug #1
}

#[test]
fn test_no_ui_id_clashes() {
    // Test that combo boxes have unique IDs
    // This would require egui test harness
}
```

### Performance Tests

Test with large datasets:

```bash
# Generate large campaign
cd sdk/campaign_builder/tests
cargo run --bin generate_large_campaign -- --items 1000 --monsters 500

# Load and verify performance
cargo run --release --bin campaign_builder -- --load test_large_campaign.ron

# Measure load time
time cargo run --release --bin campaign_builder -- --load test_large_campaign.ron
```

**Expected**: Load time < 2 seconds for 1000 items + 500 monsters.

---

## Regression Testing

After fixing bugs, re-run full test suite:

### Regression Checklist

- [ ] All unit tests pass: `cargo test --lib`
- [ ] All integration tests pass: `cargo test --test '*'`
- [ ] Manual Test Suite 1 (Campaign Lifecycle) passes
- [ ] Manual Test Suite 2 (Items Editor) passes
- [ ] Manual Test Suite 3 (Monsters Editor) passes
- [ ] Manual Test Suite 4 (Spells Editor) passes
- [ ] Manual Test Suite 5 (Maps Editor) passes
- [ ] Manual Test Suite 6 (Quests Editor) passes
- [ ] Manual Test Suite 7 (Dialogues Editor) passes
- [ ] Manual Test Suite 8 (Asset Manager) passes
- [ ] Manual Test Suite 9 (Validation Panel) passes
- [ ] Manual Test Suite 10 (Export/Package) passes

---

## Bug Verification Testing

After applying fixes, verify each bug is resolved:

### Bug 1 Verification: Items/Monsters Persist

```bash
# Test script
cd sdk/campaign_builder

# 1. Create campaign
cargo run --release

# (In UI: Create campaign, add 3 items, add 2 monsters, save, exit)

# 2. Verify files exist
ls -la test_campaign/
# Should see: items.ron, monsters.ron with non-zero size

# 3. Check RON file contents
cat test_campaign/items.ron
# Should see item definitions

# 4. Reload campaign
cargo run --release

# (In UI: Open campaign)
# VERIFY: Items tab shows 3 items
# VERIFY: Monsters tab shows 2 monsters
```

### Bug 2 Verification: No UI ID Clashes

```bash
# Manual test
cargo run --release

# (In UI)
# 1. Go to Items tab → Click Item Type Filter dropdown → Select "Weapon"
# 2. Go to Monsters tab → Click any dropdown
# 3. Go back to Items tab → Click Item Type Filter dropdown again
# VERIFY: No crash, no freeze, dropdown works normally
```

### Bug 3 Verification: Terrain + Wall Both Apply

```bash
# Manual test
cargo run --release

# (In UI)
# 1. Go to Maps tab → Create new map
# 2. Select "Grass" from Terrain dropdown
# 3. Select "Normal" from Wall dropdown
# 4. Click on a tile
# 5. Select tile and view inspector panel
# VERIFY: Tile shows terrain=Grass AND wall=Normal (not just one)
```

---

## Continuous Integration Setup

Add GitHub Actions workflow: `.github/workflows/campaign_builder_tests.yml`

```yaml
name: Campaign Builder Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable

      - name: Run unit tests
        run: cd sdk/campaign_builder && cargo test --lib

      - name: Run integration tests
        run: cd sdk/campaign_builder && cargo test --test '*'

      - name: Check formatting
        run: cd sdk/campaign_builder && cargo fmt --all -- --check

      - name: Run clippy
        run: cd sdk/campaign_builder && cargo clippy --all-targets -- -D warnings

      - name: Build release
        run: cd sdk/campaign_builder && cargo build --release
```

---

## Test Data Management

### Create Test Campaigns

```bash
# Minimal campaign (fast testing)
sdk/campaign_builder/test_data/minimal/
├── campaign.ron
├── items.ron (3 items)
├── monsters.ron (2 monsters)
└── spells.ron (5 spells)

# Medium campaign (realistic testing)
sdk/campaign_builder/test_data/medium/
├── campaign.ron
├── items.ron (50 items)
├── monsters.ron (30 monsters)
├── spells.ron (40 spells)
└── maps/ (5 maps)

# Large campaign (stress testing)
sdk/campaign_builder/test_data/large/
├── campaign.ron
├── items.ron (500 items)
├── monsters.ron (200 monsters)
├── spells.ron (100 spells)
└── maps/ (20 maps)
```

### Test Data Generator

Create: `sdk/campaign_builder/tests/generate_test_data.rs`

```rust
//! Generates test campaigns of various sizes

use antares::domain::items::Item;
use antares::domain::monsters::Monster;

fn generate_test_campaign(
    name: &str,
    num_items: usize,
    num_monsters: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Generate items
    let items: Vec<Item> = (0..num_items)
        .map(|i| Item {
            id: i as u32,
            name: format!("Test Item {}", i),
            // ... rest of fields
        })
        .collect();

    // Generate monsters
    let monsters: Vec<Monster> = (0..num_monsters)
        .map(|i| Monster {
            id: i as u32,
            name: format!("Test Monster {}", i),
            // ... rest of fields
        })
        .collect();

    // Write to RON files
    // ...

    Ok(())
}
```

---

## Test Metrics and Goals

### Current State (Before Fixes)

- Unit test count: ~100
- Unit test pass rate: 100%
- Integration test count: 0
- Manual test coverage: ~40%
- Known bugs: 3 critical

### Target State (After Fixes)

- Unit test count: ~150 (add 50 tests for new functionality)
- Unit test pass rate: 100%
- Integration test count: 10+ (save/load, persistence, validation)
- Manual test coverage: 90%
- Known bugs: 0 critical

### Success Criteria

- [ ] All 3 critical bugs fixed and verified
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] All manual test suites pass
- [ ] No regressions in existing functionality
- [ ] Campaign save/load works 100% reliably
- [ ] UI responsive with 1000+ items
- [ ] No memory leaks (valgrind clean)

---

## Next Steps

1. **Immediate (Week 1)**:
   - Fix Bug #1: Items/Monsters persistence
   - Fix Bug #2: UI ID clashes
   - Fix Bug #3: Terrain/Wall selector
   - Verify all bugs fixed with manual tests

2. **Short-term (Week 2)**:
   - Add integration tests for save/load
   - Set up CI pipeline
   - Create test data generator
   - Run full regression test suite

3. **Medium-term (Week 3-4)**:
   - Continue Phase 3B implementation (Items Editor enhancements)
   - Add automated UI tests (if egui test harness available)
   - Performance profiling and optimization

4. **Long-term (Phase 4+)**:
   - Complete Quest and Dialogue editors
   - Full asset manager implementation
   - Export and packaging features

---

## Conclusion

This testing guide provides comprehensive coverage for Campaign Builder UI validation.
The three critical bugs MUST be fixed before proceeding with Phase 3B implementation.

After fixes are verified, follow the campaign_builder_completion_plan.md for
continued development phases.

**Key Takeaway**: Test early, test often. Manual testing catches UI issues that
unit tests miss. Integration tests catch persistence bugs.
