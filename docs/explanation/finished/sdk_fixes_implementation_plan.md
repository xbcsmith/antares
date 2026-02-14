# Procedural Mesh Feature Completion Implementation Plan

## Overview

This plan addresses missing deliverables from previous implementation plans and fixes broken Campaign Builder SDK functionality for the Map Editor visual properties system. The work focuses on:

1. **SDK Map Editor Fixes**: Terrain-Specific Settings, Visual Properties Apply button, and Visual Preset buttons not applying changes correctly
2. **Multi-select Support**: Ensuring all visual property operations respect tile multi-selection
3. **State Reset Behavior**: Implementing proper reset of editor states after apply/navigation actions

## Current State Analysis

### Existing Infrastructure

**Map Editor Components** ([map_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/map_editor.rs)):
- `TerrainEditorState` (lines 1168-1258): Correctly implements `from_metadata()`, `apply_to_metadata()`, `clear_metadata()`
- `VisualEditorState`: Handles visual properties (height, scale, rotation, color)
- `PresetCategory` enum (lines 305-344): Implemented with All, General, Nature, Water, Structures categories
- `show_terrain_specific_controls()` (lines 5010-5200): Returns `bool` when controls change
- `show_preset_palette()` (lines 5214-5256): Returns `Option<VisualPreset>` when preset clicked
- `show_visual_metadata_editor()` (lines 4816-4990): Apply button and visual controls
- Multi-select support: `selected_tiles: Vec<Position>` in `MapEditorState`
- 90+ existing unit tests (lines 5450-7500+)

**Tutorial Campaign Maps** ([maps](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/campaigns/tutorial/data/maps)):
- 6 map files exist: `map_1.ron` through `map_6.ron`
- Maps are functional but may need `TileVisualMetadata` updates

### Identified Issues

| Issue | Location | Problem | Impact |
|-------|----------|---------|--------|
| Terrain controls ignore multi-select | Lines 3868-3880 | Only applies to single `pos`, not `selected_tiles` | Terrain settings don't work for bulk edits |
| Preset palette ignores multi-select | Lines 3898-3907 | Only applies to single `pos`, not `selected_tiles` | Preset buttons don't work for bulk edits |
| Apply button incomplete | Lines 4954-4957 | Only applies `visual_editor` state, not `terrain_editor_state` | Terrain-specific settings never saved |
| No reset after Apply | Line 4957 | Editor state persists after applying | Confusing UX, stale values |
| No reset after Back to list | Not implemented | Editor state persists after navigation | Stale state when returning to editor |

## Implementation Phases

### Phase 1: Fix Apply Button to Include Terrain State

**Goal**: Ensure the "Apply" button merges both `visual_editor` and `terrain_editor_state` into tile metadata.

#### 1.1 Modify Apply Button Handler

**File**: [map_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/map_editor.rs) (lines 4954-4957)

**Current behavior**:
```rust
if ui.button(&apply_text).clicked() {
    let visual_metadata = editor.visual_editor.to_metadata();
    editor.apply_visual_metadata_to_selection(&visual_metadata);
}
```

**Required changes**:
1. After applying `visual_editor.to_metadata()`, also apply `terrain_editor_state` to each tile
2. Create helper method `apply_terrain_state_to_selection()` in `MapEditorState`
3. Call both methods when Apply button clicked

#### 1.2 Add Helper Method to MapEditorState

Add new method `apply_terrain_state_to_selection(&mut self)`:
- If `selected_tiles` is not empty, apply `terrain_editor_state` to each selected tile
- If `selected_tiles` is empty but `selected_position` exists, apply to that single tile
- Ensure `has_changes = true` is set

#### 1.3 Testing Requirements

- [ ] Add test `test_apply_button_includes_terrain_state`
- [ ] Add test `test_apply_terrain_to_multiple_tiles`

Run tests:
```bash
cargo test --package campaign_builder --lib map_editor::tests::test_apply_button_includes_terrain_state
cargo test --package campaign_builder --lib map_editor::tests::test_apply_terrain_to_multiple_tiles
```

#### 1.4 Deliverables

- [ ] `apply_terrain_state_to_selection()` method added
- [ ] Apply button handler updated to call both visual and terrain apply methods
- [ ] Unit tests passing

#### 1.5 Success Criteria

- Changing "Grass Density" dropdown and clicking Apply modifies selected tile's metadata
- Changing "Tree Type" dropdown and clicking Apply modifies selected tile's metadata
- All terrain-specific settings (water flow, rock variant, snow coverage, foliage density) are persisted

---

### Phase 2: Fix Multi-Select for Terrain Controls

**Goal**: Terrain-Specific Settings should respect multi-selection when Apply button is clicked.

#### 2.1 Modify Terrain Controls Integration

**File**: [map_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/map_editor.rs) (lines 3868-3880)

**Current behavior**: Immediately applies changes to single tile when dropdown changes

**Required changes**:
1. Remove immediate apply on control change (lines 3873-3879)
2. Let changes accumulate in `terrain_editor_state`
3. Changes are applied when user clicks "Apply" button (handled in Phase 1)

**Alternative**: Keep immediate apply but iterate over `selected_tiles` instead of single `pos`

**Recommended approach**: Accumulate changes in state, apply on Apply button click. This matches UX pattern of visual properties.

#### 2.2 Show Selection Count in Terrain Section

Add label showing "N tiles selected" when multi-select is active, similar to Visual Properties section.

#### 2.3 Testing Requirements

- [ ] Add test `test_terrain_controls_multi_select`
- [ ] Add test `test_terrain_controls_single_select_fallback`

Run tests:
```bash
cargo test --package campaign_builder --lib map_editor::tests::test_terrain_controls_multi_select
```

#### 2.4 Deliverables

- [ ] Terrain controls accumulate in state instead of immediate apply
- [ ] Selection count label added to Terrain-Specific Settings section
- [ ] Unit tests passing

#### 2.5 Success Criteria

- Select 3 forest tiles, change Tree Type to "Pine", click Apply → all 3 tiles have TreeType::Pine
- Select 5 grass tiles, change Grass Density to "High", click Apply → all 5 tiles have GrassDensity::High

---

### Phase 3: Fix Multi-Select for Visual Presets

**Goal**: Visual Preset palette buttons should apply to all selected tiles.

#### 3.1 Modify Preset Palette Integration

**File**: [map_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/map_editor.rs) (lines 3898-3907)

**Current behavior**: Applies preset to single `pos`

**Required changes**:
1. Check if `selected_tiles` is not empty
2. If multi-select active, iterate over all selected tiles and apply preset
3. If single tile selected, apply to that tile only

#### 3.2 Update show_preset_palette for Immediate Apply Mode

The current preset palette returns `Option<VisualPreset>`. Keep this pattern but handle multi-select at the call site.

#### 3.3 Testing Requirements

- [ ] Add test `test_preset_palette_multi_select`
- [ ] Add test `test_preset_palette_single_tile`

Run tests:
```bash
cargo test --package campaign_builder --lib map_editor::tests::test_preset_palette_multi_select
```

#### 3.4 Deliverables

- [ ] Preset palette applies to all selected tiles when multi-select is active
- [ ] Unit tests passing

#### 3.5 Success Criteria

- Select 4 wall tiles, click "Tall Wall" preset → all 4 tiles have height=3.0
- Select 2 tiles, click "Small Tree" preset → both tiles have tree_type=Oak, scale=0.5

---

### Phase 4: Implement State Reset Behavior

**Goal**: Reset visual properties and terrain state after Apply button and when navigating back to map list.

#### 4.1 Reset After Apply Button

**File**: [map_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/map_editor.rs) (line 4957)

**Required changes**:
1. After applying visual metadata, call `visual_editor.reset()`
2. After applying terrain state, reset `terrain_editor_state = TerrainEditorState::default()`

#### 4.2 Reset on Back to List

Find the "Back to list" button handler and add reset calls:
1. Reset `visual_editor`
2. Reset `terrain_editor_state`
3. Clear `selected_tiles`
4. Clear `selected_position`

#### 4.3 Add Reset Methods

If not existing, add `VisualEditorState::reset()` method (already exists per line 4962).
Verify `TerrainEditorState::default()` can be used for reset.

#### 4.4 Testing Requirements

- [ ] Add test `test_state_reset_after_apply`
- [ ] Add test `test_state_reset_on_back_to_list`

Run tests:
```bash
cargo test --package campaign_builder --lib map_editor::tests::test_state_reset_after_apply
cargo test --package campaign_builder --lib map_editor::tests::test_state_reset_on_back_to_list
```

#### 4.5 Deliverables

- [ ] Apply button resets both visual and terrain editor states
- [ ] Back to list resets all editor states and clears selections
- [ ] Unit tests passing

#### 4.6 Success Criteria

- After clicking Apply, all checkboxes unchecked and values reset to defaults
- After clicking Back to list, returning to same map shows fresh editor state

---

## Verification Plan

### Automated Tests

Run the full map_editor test suite to ensure no regressions:

```bash
cargo test --package campaign_builder --lib map_editor::
```

Run specific new tests:

```bash
cargo test --package campaign_builder --lib map_editor::tests::test_apply_button_includes_terrain_state
cargo test --package campaign_builder --lib map_editor::tests::test_terrain_controls_multi_select
cargo test --package campaign_builder --lib map_editor::tests::test_preset_palette_multi_select
cargo test --package campaign_builder --lib map_editor::tests::test_state_reset_after_apply
cargo test --package campaign_builder --lib map_editor::tests::test_state_reset_on_back_to_list
```

### Manual Verification

**Test 1: Terrain-Specific Settings Apply**
1. Run Campaign Builder: `cargo run --bin campaign_builder`
2. Open Map Editor tab, select a map, click Edit
3. Select a Forest tile on the map
4. In "Terrain-Specific Settings" section, change "Tree Type" dropdown to "Pine"
5. Click "Apply" button
6. Save map
7. **Expected**: Tile's visual metadata now has `tree_type: Pine`

**Test 2: Multi-Select Terrain Apply**
1. Enable "Multi-Select Mode" button
2. Click on 3 different Grass tiles to select them
3. Change "Grass Density" to "High"
4. Click "Apply"
5. **Expected**: All 3 tiles now have `grass_density: High`

**Test 3: Visual Preset Multi-Select**
1. Enable "Multi-Select Mode"
2. Select 2 wall tiles
3. In "Visual Presets" palette, click "Tall Wall"
4. **Expected**: Both tiles immediately get `height: 3.0`

**Test 4: Reset After Apply**
1. Select a tile, enable Height checkbox, set to 2.5
2. Click Apply
3. **Expected**: Height checkbox is unchecked, value reset to default

**Test 5: Reset on Back to List**
1. Edit a map, select a tile, change some visual properties
2. Click "Back to list" button
3. Select the same map again, click Edit
4. **Expected**: All visual property controls are at default values

---

## Implementation Timeline

| Phase | Estimated Time | Dependencies |
|-------|---------------|--------------|
| Phase 1: Apply Button Fix | 1-2 hours | None |
| Phase 2: Terrain Multi-Select | 1-2 hours | Phase 1 |
| Phase 3: Preset Multi-Select | 1 hour | None |
| Phase 4: State Reset | 1 hour | Phase 1, 2 |

**Total Estimated Time**: 4-6 hours

---

## Files Modified

| File | Changes |
|------|---------|
| [map_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/map_editor.rs) | Apply button, terrain controls, preset palette, reset behavior |

---

## Decisions Made

1. **Immediate vs Deferred Apply for Terrain Controls**: ✅ **Deferred apply** - Terrain dropdown changes accumulate in state, applied when Apply button is clicked. Consistent with visual properties UX.

2. **Reset Behavior Scope**: ✅ **Reset only editor fields, keep tile selection** - This supports iterative editing workflows.

3. **Tutorial Map Updates**: ✅ **Deferred to separate plan** - Previous attempts broke maps; requires dedicated careful approach.
