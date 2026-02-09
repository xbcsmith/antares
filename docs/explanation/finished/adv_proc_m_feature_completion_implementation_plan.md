# Advanced Procedural Meshes Feature Completion Implementation Plan

## Overview

This plan implements terrain-specific inspector controls, preset categorization UI, and tutorial map updates for the Campaign Builder's procedural mesh visualization system. The work spans three components:

1. **Game Engine** (`antares/src/domain/world/types.rs`): Extend `TileVisualMetadata` with terrain-specific fields
2. **Campaign Builder** (`antares/sdk/campaign_builder/src/map_editor.rs`): Add UI controls and preset palette
3. **Documentation** (`antares/docs/`): Create user guides and technical references

**Architecture Layer**: SDK Layer (Campaign Builder) + Domain Layer (TileVisualMetadata)

**Data Format**: RON (`.ron`) for all map data files per architecture.md Section 7.1

## Current State Analysis

### Existing Infrastructure

**File**: `antares/src/domain/world/types.rs`

- `TileVisualMetadata` struct (Lines 298-308) defines height, width_x, width_z, tint_color, scale
- Fields are `Option<f32>` or `Option<Color>` for override capability
- No terrain-specific fields (grass_density, tree_type, rock_variant, etc.)

**File**: `antares/sdk/campaign_builder/src/map_editor.rs`

- `MapEditorState` struct (Lines 924-934) manages editor state
- `VisualPreset` enum (Lines 307-317) defines presets (Default, ShortWall, TallWall, etc.)
- Inspector panel shows generic visual controls (Lines 1800-2000 approximate)
- No terrain-specific UI controls exist
- No preset categorization or filtering

**File**: `antares/data/campaigns/tutorial/maps/*.ron`

- Tutorial maps exist but use minimal `TileVisualMetadata`
- Current maps do not showcase terrain-specific features

### Identified Issues

**Issue 1**: `TileVisualMetadata` lacks terrain-specific fields

- **Location**: `antares/src/domain/world/types.rs:298-308`
- **Impact**: Cannot store grass_density, tree_type, rock_variant, water_flow_direction
- **Required**: Add optional fields with serde defaults

**Issue 2**: Inspector panel shows generic controls for all terrain types

- **Location**: `antares/sdk/campaign_builder/src/map_editor.rs` (inspector panel function)
- **Impact**: User sees irrelevant controls (tree_type dropdown on water tiles)
- **Required**: Context-sensitive UI based on `TerrainType`

**Issue 3**: No preset categorization or filtering

- **Location**: `antares/sdk/campaign_builder/src/map_editor.rs:307-317` (VisualPreset enum)
- **Impact**: All 20+ presets shown in flat list
- **Required**: Category enum (Walls, Nature, Water, Structures) with filter UI

**Issue 4**: Tutorial maps do not demonstrate terrain-specific features

- **Location**: `antares/data/campaigns/tutorial/maps/*.ron`
- **Impact**: New users don't see advanced mesh capabilities
- **Required**: Update 6 tutorial maps with varied terrain metadata

## Implementation Phases

### Phase 1: Extend TileVisualMetadata with Terrain-Specific Fields

**Objective**: Add optional terrain-specific fields to `TileVisualMetadata` struct

#### 1.1 Add Terrain-Specific Enums to Domain Layer

**File**: `antares/src/domain/world/types.rs`
**Location**: After existing enum definitions (before `TileVisualMetadata` struct)
**Action**: Add new enum definitions

**Code to Add**:

```rust
/// Grass density levels for terrain visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrassDensity {
    /// No grass blades (bare dirt)
    None,
    /// 10-20 blades per tile
    Low,
    /// 40-60 blades per tile
    Medium,
    /// 80-120 blades per tile
    High,
    /// 150+ blades per tile
    VeryHigh,
}

impl Default for GrassDensity {
    fn default() -> Self {
        Self::Medium
    }
}

/// Tree visual variants for forest tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TreeType {
    /// Deciduous tree (broad leaves)
    Oak,
    /// Coniferous tree (needle leaves)
    Pine,
    /// Dead/bare tree
    Dead,
    /// Palm tree
    Palm,
    /// Willow tree
    Willow,
}

impl Default for TreeType {
    fn default() -> Self {
        Self::Oak
    }
}

/// Rock visual variants for mountain/hill tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RockVariant {
    /// Smooth rounded boulders
    Smooth,
    /// Jagged sharp rocks
    Jagged,
    /// Layered sedimentary
    Layered,
    /// Crystalline formation
    Crystal,
}

impl Default for RockVariant {
    fn default() -> Self {
        Self::Smooth
    }
}

/// Water flow direction for river/stream tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaterFlowDirection {
    /// Still water (no flow)
    Still,
    /// Flowing north
    North,
    /// Flowing south
    South,
    /// Flowing east
    East,
    /// Flowing west
    West,
}

impl Default for WaterFlowDirection {
    fn default() -> Self {
        Self::Still
    }
}
```

**Validation**:

- Run `cargo check --all-targets --all-features`
- Expected: 0 errors
- Verify enums are exported in `antares/src/domain/world/mod.rs`

#### 1.2 Extend TileVisualMetadata Struct

**File**: `antares/src/domain/world/types.rs`
**Location**: Lines 298-340 (TileVisualMetadata struct definition)
**Action**: Add terrain-specific fields after existing fields

**Code to Add** (append to struct fields):

```rust
    /// Grass density for grassland/plains tiles (default: Medium)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grass_density: Option<GrassDensity>,

    /// Tree type for forest tiles (default: Oak)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_type: Option<TreeType>,

    /// Rock variant for mountain/hill tiles (default: Smooth)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rock_variant: Option<RockVariant>,

    /// Water flow direction for water tiles (default: Still)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub water_flow_direction: Option<WaterFlowDirection>,

    /// Foliage density multiplier (0.0 to 2.0, default: 1.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foliage_density: Option<f32>,

    /// Snow coverage percentage (0.0 to 1.0, default: 0.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snow_coverage: Option<f32>,
```

**Validation**:

- Run `cargo fmt --all`
- Run `cargo check --all-targets --all-features`
- Run `cargo clippy --all-targets --all-features -- -D warnings`
- Expected: 0 errors, 0 warnings

#### 1.3 Add Helper Methods to TileVisualMetadata

**File**: `antares/src/domain/world/types.rs`
**Location**: After `TileVisualMetadata` struct definition (in `impl TileVisualMetadata` block if exists, otherwise create new impl block)
**Action**: Add terrain-specific accessor methods

**Code to Add**:

```rust
impl TileVisualMetadata {
    /// Get grass density with fallback to default
    pub fn grass_density(&self) -> GrassDensity {
        self.grass_density.unwrap_or_default()
    }

    /// Get tree type with fallback to default
    pub fn tree_type(&self) -> TreeType {
        self.tree_type.unwrap_or_default()
    }

    /// Get rock variant with fallback to default
    pub fn rock_variant(&self) -> RockVariant {
        self.rock_variant.unwrap_or_default()
    }

    /// Get water flow direction with fallback to default
    pub fn water_flow_direction(&self) -> WaterFlowDirection {
        self.water_flow_direction.unwrap_or_default()
    }

    /// Get foliage density with fallback to 1.0
    pub fn foliage_density(&self) -> f32 {
        self.foliage_density.unwrap_or(1.0)
    }

    /// Get snow coverage with fallback to 0.0
    pub fn snow_coverage(&self) -> f32 {
        self.snow_coverage.unwrap_or(0.0)
    }

    /// Check if metadata has any terrain-specific overrides
    pub fn has_terrain_overrides(&self) -> bool {
        self.grass_density.is_some()
            || self.tree_type.is_some()
            || self.rock_variant.is_some()
            || self.water_flow_direction.is_some()
            || self.foliage_density.is_some()
            || self.snow_coverage.is_some()
    }
}
```

**Validation**:

- Run `cargo fmt --all`
- Run `cargo clippy --all-targets --all-features -- -D warnings`
- Run `cargo test --lib` (verify no existing tests break)
- Expected: All existing tests pass

#### 1.4 Unit Tests for New Fields

**File**: `antares/src/domain/world/types.rs`
**Location**: In `#[cfg(test)] mod tests` section at end of file
**Action**: Add serialization and default value tests

**Code to Add**:

```rust
#[cfg(test)]
mod terrain_metadata_tests {
    use super::*;

    #[test]
    fn test_grass_density_serialization() {
        let mut meta = TileVisualMetadata::default();
        meta.grass_density = Some(GrassDensity::High);

        let ron = ron::to_string(&meta).unwrap();
        assert!(ron.contains("grass_density"));
        assert!(ron.contains("High"));

        let deserialized: TileVisualMetadata = ron::from_str(&ron).unwrap();
        assert_eq!(deserialized.grass_density, Some(GrassDensity::High));
    }

    #[test]
    fn test_grass_density_default_not_serialized() {
        let meta = TileVisualMetadata::default();
        let ron = ron::to_string(&meta).unwrap();
        assert!(!ron.contains("grass_density"));
    }

    #[test]
    fn test_tree_type_accessor_defaults_to_oak() {
        let meta = TileVisualMetadata::default();
        assert_eq!(meta.tree_type(), TreeType::Oak);
    }

    #[test]
    fn test_has_terrain_overrides_returns_false_for_default() {
        let meta = TileVisualMetadata::default();
        assert!(!meta.has_terrain_overrides());
    }

    #[test]
    fn test_has_terrain_overrides_returns_true_when_set() {
        let mut meta = TileVisualMetadata::default();
        meta.grass_density = Some(GrassDensity::Low);
        assert!(meta.has_terrain_overrides());
    }

    #[test]
    fn test_foliage_density_clamps_in_valid_range() {
        let mut meta = TileVisualMetadata::default();
        meta.foliage_density = Some(1.5);
        assert_eq!(meta.foliage_density(), 1.5);
    }

    #[test]
    fn test_water_flow_direction_default_is_still() {
        let meta = TileVisualMetadata::default();
        assert_eq!(meta.water_flow_direction(), WaterFlowDirection::Still);
    }
}
```

**Validation**:

- Run `cargo test terrain_metadata_tests`
- Expected: 7 tests passed

#### 1.5 Update Domain Module Exports

**File**: `antares/src/domain/world/mod.rs`
**Location**: In public exports section
**Action**: Add new enum exports

**Code to Add**:

```rust
pub use types::{GrassDensity, TreeType, RockVariant, WaterFlowDirection};
```

**Validation**:

- Run `cargo check --all-targets --all-features`
- Verify enums are accessible from `antares::domain::world::{GrassDensity, TreeType, ...}`

#### 1.6 Deliverables

- [ ] `GrassDensity` enum added to `antares/src/domain/world/types.rs`
- [ ] `TreeType` enum added to `antares/src/domain/world/types.rs`
- [ ] `RockVariant` enum added to `antares/src/domain/world/types.rs`
- [ ] `WaterFlowDirection` enum added to `antares/src/domain/world/types.rs`
- [ ] 6 new optional fields added to `TileVisualMetadata` struct
- [ ] 7 helper methods implemented in `TileVisualMetadata` impl block
- [ ] 7 unit tests added and passing
- [ ] Enums exported in `antares/src/domain/world/mod.rs`
- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes (0 errors)
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes (0 warnings)
- [ ] `cargo test --lib` passes (all existing + new tests)

#### 1.7 Success Criteria

**Automated Verification**:

```bash
# Must all pass with 0 errors/warnings:
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test terrain_metadata_tests -- --nocapture
```

**Manual Verification**:

- [ ] `TileVisualMetadata` struct compiles with new fields
- [ ] RON serialization includes new fields when set
- [ ] RON serialization omits new fields when None (due to skip_serializing_if)
- [ ] Default values work correctly (Medium grass, Oak trees, Smooth rocks, Still water)
- [ ] `has_terrain_overrides()` correctly identifies when terrain fields are set

### Phase 2: Add Terrain-Specific Inspector Controls to Campaign Builder

**Objective**: Implement context-sensitive UI controls in map editor inspector panel

#### 2.1 Create TerrainEditorState Struct

**File**: `antares/sdk/campaign_builder/src/map_editor.rs`
**Location**: After `MapEditorState` struct definition (around line 960)
**Action**: Add new state struct for terrain-specific UI

**Code to Add**:

```rust
/// Terrain-specific editor state for inspector panel
/// Mirrors TileVisualMetadata terrain fields for UI binding
#[derive(Debug, Clone)]
pub struct TerrainEditorState {
    /// Grass density selection (for Grassland/Plains tiles)
    pub grass_density: GrassDensity,

    /// Tree type selection (for Forest tiles)
    pub tree_type: TreeType,

    /// Rock variant selection (for Mountain/Hill tiles)
    pub rock_variant: RockVariant,

    /// Water flow direction (for Water/River tiles)
    pub water_flow_direction: WaterFlowDirection,

    /// Foliage density slider value (0.0 to 2.0)
    pub foliage_density: f32,

    /// Snow coverage slider value (0.0 to 1.0)
    pub snow_coverage: f32,
}

impl Default for TerrainEditorState {
    fn default() -> Self {
        Self {
            grass_density: GrassDensity::Medium,
            tree_type: TreeType::Oak,
            rock_variant: RockVariant::Smooth,
            water_flow_direction: WaterFlowDirection::Still,
            foliage_density: 1.0,
            snow_coverage: 0.0,
        }
    }
}

impl TerrainEditorState {
    /// Load state from TileVisualMetadata
    pub fn from_metadata(metadata: &TileVisualMetadata) -> Self {
        Self {
            grass_density: metadata.grass_density(),
            tree_type: metadata.tree_type(),
            rock_variant: metadata.rock_variant(),
            water_flow_direction: metadata.water_flow_direction(),
            foliage_density: metadata.foliage_density(),
            snow_coverage: metadata.snow_coverage(),
        }
    }

    /// Apply state to TileVisualMetadata
    pub fn apply_to_metadata(&self, metadata: &mut TileVisualMetadata) {
        metadata.grass_density = Some(self.grass_density);
        metadata.tree_type = Some(self.tree_type);
        metadata.rock_variant = Some(self.rock_variant);
        metadata.water_flow_direction = Some(self.water_flow_direction);
        metadata.foliage_density = Some(self.foliage_density);
        metadata.snow_coverage = Some(self.snow_coverage);
    }

    /// Clear terrain-specific fields from metadata (set to None)
    pub fn clear_metadata(metadata: &mut TileVisualMetadata) {
        metadata.grass_density = None;
        metadata.tree_type = None;
        metadata.rock_variant = None;
        metadata.water_flow_direction = None;
        metadata.foliage_density = None;
        metadata.snow_coverage = None;
    }
}
```

**Validation**:

- Run `cargo fmt --all`
- Run `cargo check -p campaign_builder`
- Expected: 0 errors

#### 2.2 Add TerrainEditorState to MapEditorState

**File**: `antares/sdk/campaign_builder/src/map_editor.rs`
**Location**: Lines 924-960 (MapEditorState struct)
**Action**: Add new field to struct

**Code to Modify**:

```rust
pub struct MapEditorState {
    // ... existing fields ...
    pub selected_terrain: TerrainType,
    pub selected_wall_type: WallType,

    // ADD THIS FIELD:
    /// Terrain-specific editor state for inspector panel
    pub terrain_editor_state: TerrainEditorState,

    // ... remaining fields ...
}
```

**Also update `MapEditorState::new()` method** (add to initialization):

```rust
impl MapEditorState {
    pub fn new(map: Map, metadata: MapMetadata) -> Self {
        Self {
            // ... existing fields ...
            terrain_editor_state: TerrainEditorState::default(),
            // ... remaining fields ...
        }
    }
}
```

**Validation**:

- Run `cargo fmt --all`
- Run `cargo check -p campaign_builder`
- Expected: 0 errors

#### 2.3 Implement show_terrain_specific_controls() Function

**File**: `antares/sdk/campaign_builder/src/map_editor.rs`
**Location**: After inspector panel helper functions (around line 2000)
**Action**: Add new UI rendering function

**Code to Add**:

```rust
/// Show terrain-specific inspector controls based on selected tile's terrain type
///
/// Displays context-sensitive controls:
/// - Grassland/Plains: grass_density dropdown, foliage_density slider
/// - Forest: tree_type dropdown, foliage_density slider, snow_coverage slider
/// - Mountain/Hill: rock_variant dropdown, snow_coverage slider
/// - Water/Swamp: water_flow_direction dropdown
/// - Desert/Snow: snow_coverage slider
///
/// # Arguments
///
/// * `ui` - egui UI context
/// * `terrain_type` - The TerrainType of the selected tile
/// * `state` - Mutable reference to TerrainEditorState
///
/// # Returns
///
/// `true` if any control was modified, `false` otherwise
fn show_terrain_specific_controls(
    ui: &mut egui::Ui,
    terrain_type: TerrainType,
    state: &mut TerrainEditorState,
) -> bool {
    let mut changed = false;

    ui.heading("Terrain-Specific Settings");
    ui.separator();

    match terrain_type {
        TerrainType::Grassland | TerrainType::Plains => {
            ui.label("Grass Density:");
            let mut density_index = state.grass_density as usize;
            changed |= egui::ComboBox::from_id_source("grass_density")
                .selected_text(format!("{:?}", state.grass_density))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut density_index, 0, "None")
                        | ui.selectable_value(&mut density_index, 1, "Low")
                        | ui.selectable_value(&mut density_index, 2, "Medium")
                        | ui.selectable_value(&mut density_index, 3, "High")
                        | ui.selectable_value(&mut density_index, 4, "VeryHigh")
                })
                .inner
                .unwrap_or(false);

            if changed {
                state.grass_density = match density_index {
                    0 => GrassDensity::None,
                    1 => GrassDensity::Low,
                    2 => GrassDensity::Medium,
                    3 => GrassDensity::High,
                    4 => GrassDensity::VeryHigh,
                    _ => GrassDensity::Medium,
                };
            }

            ui.label("Foliage Density:");
            changed |= ui.add(egui::Slider::new(&mut state.foliage_density, 0.0..=2.0)
                .text("density")
                .step_by(0.1)).changed();
        }

        TerrainType::Forest => {
            ui.label("Tree Type:");
            let mut tree_index = state.tree_type as usize;
            changed |= egui::ComboBox::from_id_source("tree_type")
                .selected_text(format!("{:?}", state.tree_type))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut tree_index, 0, "Oak")
                        | ui.selectable_value(&mut tree_index, 1, "Pine")
                        | ui.selectable_value(&mut tree_index, 2, "Dead")
                        | ui.selectable_value(&mut tree_index, 3, "Palm")
                        | ui.selectable_value(&mut tree_index, 4, "Willow")
                })
                .inner
                .unwrap_or(false);

            if changed {
                state.tree_type = match tree_index {
                    0 => TreeType::Oak,
                    1 => TreeType::Pine,
                    2 => TreeType::Dead,
                    3 => TreeType::Palm,
                    4 => TreeType::Willow,
                    _ => TreeType::Oak,
                };
            }

            ui.label("Foliage Density:");
            changed |= ui.add(egui::Slider::new(&mut state.foliage_density, 0.0..=2.0)
                .text("density")
                .step_by(0.1)).changed();

            ui.label("Snow Coverage:");
            changed |= ui.add(egui::Slider::new(&mut state.snow_coverage, 0.0..=1.0)
                .text("coverage")
                .step_by(0.05)).changed();
        }

        TerrainType::Mountain | TerrainType::Hill => {
            ui.label("Rock Variant:");
            let mut rock_index = state.rock_variant as usize;
            changed |= egui::ComboBox::from_id_source("rock_variant")
                .selected_text(format!("{:?}", state.rock_variant))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut rock_index, 0, "Smooth")
                        | ui.selectable_value(&mut rock_index, 1, "Jagged")
                        | ui.selectable_value(&mut rock_index, 2, "Layered")
                        | ui.selectable_value(&mut rock_index, 3, "Crystal")
                })
                .inner
                .unwrap_or(false);

            if changed {
                state.rock_variant = match rock_index {
                    0 => RockVariant::Smooth,
                    1 => RockVariant::Jagged,
                    2 => RockVariant::Layered,
                    3 => RockVariant::Crystal,
                    _ => RockVariant::Smooth,
                };
            }

            ui.label("Snow Coverage:");
            changed |= ui.add(egui::Slider::new(&mut state.snow_coverage, 0.0..=1.0)
                .text("coverage")
                .step_by(0.05)).changed();
        }

        TerrainType::Water | TerrainType::Swamp => {
            ui.label("Water Flow Direction:");
            let mut flow_index = state.water_flow_direction as usize;
            changed |= egui::ComboBox::from_id_source("water_flow")
                .selected_text(format!("{:?}", state.water_flow_direction))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut flow_index, 0, "Still")
                        | ui.selectable_value(&mut flow_index, 1, "North")
                        | ui.selectable_value(&mut flow_index, 2, "South")
                        | ui.selectable_value(&mut flow_index, 3, "East")
                        | ui.selectable_value(&mut flow_index, 4, "West")
                })
                .inner
                .unwrap_or(false);

            if changed {
                state.water_flow_direction = match flow_index {
                    0 => WaterFlowDirection::Still,
                    1 => WaterFlowDirection::North,
                    2 => WaterFlowDirection::South,
                    3 => WaterFlowDirection::East,
                    4 => WaterFlowDirection::West,
                    _ => WaterFlowDirection::Still,
                };
            }
        }

        TerrainType::Snow | TerrainType::Desert => {
            ui.label("Snow Coverage:");
            changed |= ui.add(egui::Slider::new(&mut state.snow_coverage, 0.0..=1.0)
                .text("coverage")
                .step_by(0.05)).changed();
        }

        _ => {
            ui.label("No terrain-specific controls for this terrain type.");
        }
    }

    changed
}
```

**Validation**:

- Run `cargo fmt --all`
- Run `cargo check -p campaign_builder`
- Run `cargo clippy -p campaign_builder -- -D warnings`
- Expected: 0 errors, 0 warnings

#### 2.4 Integrate Terrain Controls into Inspector Panel

**File**: `antares/sdk/campaign_builder/src/map_editor.rs`
**Location**: Find the inspector panel rendering function (search for "Inspector" or "show_inspector_panel")
**Action**: Add terrain controls after existing visual metadata controls

**Locate existing code similar to**:

```rust
fn show_inspector_panel(ui: &mut egui::Ui, state: &mut MapEditorState, ...) {
    // ... existing visual controls (height, width, tint, scale) ...
}
```

**Add after existing visual controls**:

```rust
    // Existing visual controls here (height, width_x, width_z, tint_color, scale)
    // ...

    ui.separator();

    // ADD THIS SECTION:
    if let Some(pos) = state.selected_position {
        if let Some(tile) = state.map.get_tile(pos) {
            let terrain_type = tile.terrain;

            ui.heading("Terrain Properties");

            if show_terrain_specific_controls(ui, terrain_type, &mut state.terrain_editor_state) {
                // Apply changes to selected tile's visual metadata
                if let Some(metadata_map) = state.metadata.tile_visual_metadata.as_mut() {
                    let metadata = metadata_map.entry(pos).or_insert_with(TileVisualMetadata::default);
                    state.terrain_editor_state.apply_to_metadata(metadata);
                }
            }

            ui.separator();

            if ui.button("Clear Terrain Properties").clicked() {
                if let Some(metadata_map) = state.metadata.tile_visual_metadata.as_mut() {
                    if let Some(metadata) = metadata_map.get_mut(&pos) {
                        TerrainEditorState::clear_metadata(metadata);
                    }
                }
                state.terrain_editor_state = TerrainEditorState::default();
            }
        }
    }
```

**Validation**:

- Run `cargo fmt --all`
- Run `cargo check -p campaign_builder`
- Expected: 0 errors

#### 2.5 Add Terrain State Synchronization on Tile Selection

**File**: `antares/sdk/campaign_builder/src/map_editor.rs`
**Location**: Find tile selection logic (where `selected_position` is updated)
**Action**: Load terrain state when tile is selected

**Locate code similar to**:

```rust
if clicked_tile {
    state.selected_position = Some(new_position);
}
```

**Add synchronization after selection**:

```rust
if clicked_tile {
    state.selected_position = Some(new_position);

    // ADD THIS: Synchronize terrain editor state with selected tile
    if let Some(metadata_map) = state.metadata.tile_visual_metadata.as_ref() {
        if let Some(metadata) = metadata_map.get(&new_position) {
            state.terrain_editor_state = TerrainEditorState::from_metadata(metadata);
        } else {
            state.terrain_editor_state = TerrainEditorState::default();
        }
    }
}
```

**Validation**:

- Run `cargo fmt --all`
- Run `cargo check -p campaign_builder`
- Expected: 0 errors

#### 2.6 Deliverables

- [ ] `TerrainEditorState` struct added to `antares/sdk/campaign_builder/src/map_editor.rs`
- [ ] `from_metadata()`, `apply_to_metadata()`, `clear_metadata()` methods implemented
- [ ] `terrain_editor_state` field added to `MapEditorState`
- [ ] `show_terrain_specific_controls()` function implemented with all terrain type cases
- [ ] Terrain controls integrated into inspector panel
- [ ] Tile selection synchronizes `TerrainEditorState`
- [ ] "Clear Terrain Properties" button implemented
- [ ] `cargo fmt --all` passes
- [ ] `cargo check -p campaign_builder` passes (0 errors)
- [ ] `cargo clippy -p campaign_builder -- -D warnings` passes (0 warnings)

#### 2.7 Success Criteria

**Automated Verification**:

```bash
# Must all pass:
cargo fmt --all
cargo check -p campaign_builder
cargo clippy -p campaign_builder -- -D warnings
```

**Manual Verification** (start campaign_builder):

- [ ] Select a Grassland tile → Inspector shows "Grass Density" dropdown
- [ ] Select a Forest tile → Inspector shows "Tree Type" dropdown + sliders
- [ ] Select a Mountain tile → Inspector shows "Rock Variant" dropdown + snow slider
- [ ] Select a Water tile → Inspector shows "Water Flow Direction" dropdown
- [ ] Select a Desert/Snow tile → Inspector shows "Snow Coverage" slider only
- [ ] Changing dropdown/slider immediately updates tile visual metadata
- [ ] "Clear Terrain Properties" button resets all terrain fields to None
- [ ] Switching between tiles correctly loads their respective terrain properties

### Phase 3: Implement Preset Categorization & Palette UI

**Objective**: Add preset filtering by category with visual palette UI

#### 3.1 Add PresetCategory Enum

**File**: `antares/sdk/campaign_builder/src/map_editor.rs`
**Location**: Before `VisualPreset` enum definition (around line 300)
**Action**: Add category enum

**Code to Add**:

```rust
/// Categories for organizing visual presets in UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetCategory {
    /// All presets (no filter)
    All,
    /// Wall-related presets (ShortWall, TallWall, ThinWall, etc.)
    Walls,
    /// Nature presets (SmallTree, LargeTree, Bush, Boulder, etc.)
    Nature,
    /// Water/liquid presets (ShallowWater, DeepWater, Lava, etc.)
    Water,
    /// Structure presets (Pillar, Altar, Statue, etc.)
    Structures,
}

impl PresetCategory {
    /// Get all categories for UI iteration
    pub const fn all() -> &'static [PresetCategory] {
        &[
            PresetCategory::All,
            PresetCategory::Walls,
            PresetCategory::Nature,
            PresetCategory::Water,
            PresetCategory::Structures,
        ]
    }

    /// Get display name for UI
    pub const fn display_name(&self) -> &'static str {
        match self {
            PresetCategory::All => "All",
            PresetCategory::Walls => "Walls",
            PresetCategory::Nature => "Nature",
            PresetCategory::Water => "Water",
            PresetCategory::Structures => "Structures",
        }
    }
}
```

**Validation**:

- Run `cargo fmt --all`
- Run `cargo check -p campaign_builder`
- Expected: 0 errors

#### 3.2 Add category() Method to VisualPreset

**File**: `antares/sdk/campaign_builder/src/map_editor.rs`
**Location**: After `VisualPreset` enum definition (around line 370)
**Action**: Add impl block with categorization

**Code to Add**:

```rust
impl VisualPreset {
    /// Get the category for this preset
    pub const fn category(&self) -> PresetCategory {
        match self {
            VisualPreset::Default => PresetCategory::All,
            VisualPreset::ShortWall => PresetCategory::Walls,
            VisualPreset::TallWall => PresetCategory::Walls,
            VisualPreset::ThinWall => PresetCategory::Walls,
            VisualPreset::SmallTree => PresetCategory::Nature,
            VisualPreset::LargeTree => PresetCategory::Nature,
            VisualPreset::Bush => PresetCategory::Nature,
            VisualPreset::Boulder => PresetCategory::Nature,
            VisualPreset::ShallowWater => PresetCategory::Water,
            VisualPreset::DeepWater => PresetCategory::Water,
            VisualPreset::Lava => PresetCategory::Water,
            VisualPreset::Pillar => PresetCategory::Structures,
            VisualPreset::Altar => PresetCategory::Structures,
            VisualPreset::Statue => PresetCategory::Structures,
            // Add remaining presets to appropriate categories
            _ => PresetCategory::All,
        }
    }

    /// Filter presets by category
    pub fn by_category(category: PresetCategory) -> Vec<VisualPreset> {
        if category == PresetCategory::All {
            Self::all_presets()
        } else {
            Self::all_presets()
                .into_iter()
                .filter(|preset| preset.category() == category)
                .collect()
        }
    }

    /// Get all available presets
    pub fn all_presets() -> Vec<VisualPreset> {
        vec![
            VisualPreset::Default,
            VisualPreset::ShortWall,
            VisualPreset::TallWall,
            VisualPreset::ThinWall,
            VisualPreset::SmallTree,
            VisualPreset::LargeTree,
            VisualPreset::Bush,
            VisualPreset::Boulder,
            VisualPreset::ShallowWater,
            VisualPreset::DeepWater,
            VisualPreset::Lava,
            VisualPreset::Pillar,
            VisualPreset::Altar,
            VisualPreset::Statue,
            // Add all remaining preset variants here
        ]
    }
}
```

**Validation**:

- Run `cargo fmt --all`
- Run `cargo check -p campaign_builder`
- Expected: 0 errors

#### 3.3 Add preset_category_filter to MapEditorState

**File**: `antares/sdk/campaign_builder/src/map_editor.rs`
**Location**: Lines 924-960 (MapEditorState struct)
**Action**: Add filter field

**Code to Modify**:

```rust
pub struct MapEditorState {
    // ... existing fields ...
    pub terrain_editor_state: TerrainEditorState,

    // ADD THIS FIELD:
    /// Current preset category filter
    pub preset_category_filter: PresetCategory,

    // ... remaining fields ...
}
```

**Also update initialization**:

```rust
impl MapEditorState {
    pub fn new(map: Map, metadata: MapMetadata) -> Self {
        Self {
            // ... existing fields ...
            terrain_editor_state: TerrainEditorState::default(),
            preset_category_filter: PresetCategory::All, // ADD THIS
            // ... remaining fields ...
        }
    }
}
```

**Validation**:

- Run `cargo check -p campaign_builder`
- Expected: 0 errors

#### 3.4 Implement show_preset_palette() Function

**File**: `antares/sdk/campaign_builder/src/map_editor.rs`
**Location**: After `show_terrain_specific_controls()` function
**Action**: Add preset palette UI function

**Code to Add**:

```rust
/// Show visual preset palette with category filtering
///
/// Displays a grid of preset buttons organized by category with filter tabs
///
/// # Arguments
///
/// * `ui` - egui UI context
/// * `state` - Mutable reference to MapEditorState
///
/// # Returns
///
/// `Option<VisualPreset>` if a preset was clicked, None otherwise
fn show_preset_palette(ui: &mut egui::Ui, state: &mut MapEditorState) -> Option<VisualPreset> {
    let mut selected_preset = None;

    ui.heading("Visual Presets");

    // Category filter tabs
    ui.horizontal(|ui| {
        for category in PresetCategory::all() {
            if ui.selectable_label(
                state.preset_category_filter == *category,
                category.display_name()
            ).clicked() {
                state.preset_category_filter = *category;
            }
        }
    });

    ui.separator();

    // Preset grid (3 columns)
    let presets = VisualPreset::by_category(state.preset_category_filter);

    egui::Grid::new("preset_palette_grid")
        .num_columns(3)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            for (idx, preset) in presets.iter().enumerate() {
                if ui.button(format!("{:?}", preset)).clicked() {
                    selected_preset = Some(*preset);
                }

                // New row every 3 presets
                if (idx + 1) % 3 == 0 {
                    ui.end_row();
                }
            }
        });

    selected_preset
}
```

**Validation**:

- Run `cargo fmt --all`
- Run `cargo check -p campaign_builder`
- Run `cargo clippy -p campaign_builder -- -D warnings`
- Expected: 0 errors, 0 warnings

#### 3.5 Integrate Preset Palette into Inspector Panel

**File**: `antares/sdk/campaign_builder/src/map_editor.rs`
**Location**: In inspector panel function (after terrain controls section)
**Action**: Add preset palette UI

**Add after terrain controls section**:

```rust
    // After terrain controls section...

    ui.separator();

    // ADD THIS SECTION:
    if let Some(selected_preset) = show_preset_palette(ui, state) {
        // Apply selected preset to currently selected tile
        if let Some(pos) = state.selected_position {
            if let Some(metadata_map) = state.metadata.tile_visual_metadata.as_mut() {
                let metadata = metadata_map.entry(pos).or_insert_with(TileVisualMetadata::default);

                // Apply preset values to metadata
                match selected_preset {
                    VisualPreset::Default => {
                        *metadata = TileVisualMetadata::default();
                    }
                    VisualPreset::ShortWall => {
                        metadata.height = Some(1.5);
                    }
                    VisualPreset::TallWall => {
                        metadata.height = Some(3.5);
                    }
                    VisualPreset::ThinWall => {
                        metadata.width_z = Some(0.2);
                    }
                    VisualPreset::SmallTree => {
                        metadata.height = Some(2.0);
                        metadata.scale = Some(0.5);
                        metadata.tint_color = Some([0.2, 0.8, 0.2, 1.0]);
                    }
                    VisualPreset::LargeTree => {
                        metadata.height = Some(4.0);
                        metadata.scale = Some(1.5);
                        metadata.tint_color = Some([0.1, 0.6, 0.1, 1.0]);
                    }
                    // Add remaining preset applications...
                    _ => {}
                }
            }
        }
    }
```

**Validation**:

- Run `cargo fmt --all`
- Run `cargo check -p campaign_builder`
- Expected: 0 errors

#### 3.6 Deliverables

- [ ] `PresetCategory` enum added with `all()` and `display_name()` methods
- [ ] `category()` method added to `VisualPreset` impl block
- [ ] `by_category()` and `all_presets()` methods added to `VisualPreset`
- [ ] `preset_category_filter` field added to `MapEditorState`
- [ ] `show_preset_palette()` function implemented
- [ ] Preset palette integrated into inspector panel
- [ ] All presets categorized correctly (complete categorization in `category()` method)
- [ ] `cargo fmt --all` passes
- [ ] `cargo check -p campaign_builder` passes (0 errors)
- [ ] `cargo clippy -p campaign_builder -- -D warnings` passes (0 warnings)

#### 3.7 Success Criteria

**Automated Verification**:

```bash
cargo fmt --all
cargo check -p campaign_builder
cargo clippy -p campaign_builder -- -D warnings
```

**Manual Verification** (start campaign_builder):

- [ ] Inspector panel shows "Visual Presets" section with category tabs
- [ ] Clicking "Walls" tab shows only wall presets (ShortWall, TallWall, ThinWall)
- [ ] Clicking "Nature" tab shows only nature presets (SmallTree, LargeTree, Bush, Boulder)
- [ ] Clicking "All" tab shows all presets
- [ ] Clicking a preset button applies it to selected tile
- [ ] Preset grid displays in 3-column layout
- [ ] Category filter state persists when switching between tabs

### Phase 4: Update Tutorial Maps with Terrain Features

**Objective**: Update 6 tutorial maps to demonstrate terrain-specific features

#### 4.1 Analyze Current Tutorial Map Structure

**File**: `antares/data/campaigns/tutorial/maps/*.ron`
**Action**: Read all existing tutorial map files

**Command**:

```bash
ls -la antares/data/campaigns/tutorial/maps/
cat antares/data/campaigns/tutorial/maps/map_001_town_square.ron
# Review structure and existing TileVisualMetadata usage
```

**Expected Structure**:

```ron
(
    name: "Map Name",
    size: (width: 20, height: 20),
    tiles: {
        (x: 0, y: 0): (terrain: Grassland, wall: None),
        // ...
    },
    events: {
        // ...
    },
    metadata: (
        tile_visual_metadata: {
            (x: 5, y: 5): (
                height: Some(2.5),
                tint_color: Some([1.0, 0.5, 0.2, 1.0]),
                // New terrain fields will be added here
            ),
        },
    ),
)
```

**Validation**:

- Verify all 6 maps use RON format
- Identify maps with existing `TileVisualMetadata` usage
- Note which terrain types are present in each map

#### 4.2 Update Map 1 (Town Square) - Focus on Structures

**File**: `antares/data/campaigns/tutorial/maps/map_001_town_square.ron`
**Action**: Add visual metadata showcasing walls and structures

**Modifications**:

1. **Add varied wall heights** (Lines 100-120 approximate, in metadata section):

```ron
        tile_visual_metadata: {
            // Town walls - tall outer walls
            (x: 0, y: 0): (height: Some(3.5)),
            (x: 19, y: 0): (height: Some(3.5)),
            (x: 0, y: 19): (height: Some(3.5)),
            (x: 19, y: 19): (height: Some(3.5)),

            // Interior walls - short dividers
            (x: 10, y: 5): (height: Some(1.5)),
            (x: 10, y: 6): (height: Some(1.5)),

            // Decorative pillars at entrance
            (x: 9, y: 0): (
                height: Some(4.0),
                scale: Some(0.3),
                tint_color: Some([0.7, 0.7, 0.7, 1.0]),
            ),
        },
```

**Validation**:

- Run `cargo run -p campaign_builder`
- Load Map 1
- Verify walls render with different heights
- Verify RON syntax is valid (no parse errors)

#### 4.3 Update Map 2 (Forest Entrance) - Focus on Nature

**File**: `antares/data/campaigns/tutorial/maps/map_002_forest_entrance.ron`
**Action**: Add tree types and grass density variations

**Modifications**:

```ron
        tile_visual_metadata: {
            // Dense oak forest (north section)
            (x: 5, y: 2): (
                tree_type: Some(Oak),
                foliage_density: Some(1.8),
            ),
            (x: 6, y: 2): (
                tree_type: Some(Oak),
                foliage_density: Some(1.5),
            ),

            // Pine grove (east section)
            (x: 15, y: 8): (
                tree_type: Some(Pine),
                foliage_density: Some(1.2),
            ),
            (x: 16, y: 8): (
                tree_type: Some(Pine),
                foliage_density: Some(1.3),
            ),

            // Dead trees near dungeon entrance
            (x: 10, y: 18): (
                tree_type: Some(Dead),
                tint_color: Some([0.4, 0.3, 0.2, 1.0]),
            ),

            // Grassland with varying density
            (x: 2, y: 10): (grass_density: Some(Low)),
            (x: 3, y: 10): (grass_density: Some(Medium)),
            (x: 4, y: 10): (grass_density: Some(High)),
            (x: 5, y: 10): (grass_density: Some(VeryHigh)),
        },
```

**Validation**:

- Run `cargo run -p campaign_builder`
- Load Map 2
- Verify different tree types render distinctly
- Verify grass density variations are visible
- Verify RON syntax is valid

#### 4.4 Update Map 3 (Dungeon Level 1) - Focus on Rocks and Water

**File**: `antares/data/campaigns/tutorial/maps/map_003_dungeon_level_1.ron`
**Action**: Add rock variants and water flow

**Modifications**:

```ron
        tile_visual_metadata: {
            // Jagged cave walls
            (x: 1, y: 1): (rock_variant: Some(Jagged)),
            (x: 2, y: 1): (rock_variant: Some(Jagged)),

            // Crystal formations in treasure room
            (x: 15, y: 15): (
                rock_variant: Some(Crystal),
                tint_color: Some([0.5, 0.5, 1.0, 1.0]),
            ),

            // Layered sedimentary rocks
            (x: 8, y: 8): (rock_variant: Some(Layered)),

            // Underground river with flow direction
            (x: 10, y: 5): (water_flow_direction: Some(East)),
            (x: 11, y: 5): (water_flow_direction: Some(East)),
            (x: 12, y: 5): (water_flow_direction: Some(East)),
            (x: 13, y: 5): (water_flow_direction: Some(South)),
            (x: 13, y: 6): (water_flow_direction: Some(South)),
        },
```

**Validation**:

- Run `cargo run -p campaign_builder`
- Load Map 3
- Verify rock variants render correctly
- Verify water flow direction is applied
- Verify RON syntax is valid

#### 4.5 Update Maps 4, 5, 6 (Advanced Areas)

**Files**:

- `map_004_*.ron`
- `map_005_*.ron`
- `map_006_*.ron`

**Action**: Add mixed terrain features

**Map 4 Modifications** (Mountain Pass):

```ron
        tile_visual_metadata: {
            // Snowy mountain peaks
            (x: 5, y: 5): (
                rock_variant: Some(Smooth),
                snow_coverage: Some(0.8),
            ),
            (x: 6, y: 5): (
                rock_variant: Some(Smooth),
                snow_coverage: Some(0.9),
            ),

            // Snow-covered pines
            (x: 10, y: 10): (
                tree_type: Some(Pine),
                snow_coverage: Some(0.6),
            ),
        },
```

**Map 5 Modifications** (Desert Oasis):

```ron
        tile_visual_metadata: {
            // Palm trees around oasis
            (x: 10, y: 10): (tree_type: Some(Palm)),
            (x: 11, y: 10): (tree_type: Some(Palm)),

            // Still water in oasis pond
            (x: 10, y: 11): (water_flow_direction: Some(Still)),
        },
```

**Map 6 Modifications** (Swamp):

```ron
        tile_visual_metadata: {
            // Willow trees in swamp
            (x: 5, y: 5): (
                tree_type: Some(Willow),
                foliage_density: Some(0.8),
            ),

            // Murky swamp water (no flow)
            (x: 8, y: 8): (
                water_flow_direction: Some(Still),
                tint_color: Some([0.3, 0.4, 0.2, 0.8]),
            ),
        },
```

**Validation**:

- Run `cargo run -p campaign_builder` for each map
- Verify all terrain-specific features render correctly
- Verify RON syntax is valid for all maps

#### 4.6 Add Map Validation Script

**File**: `antares/scripts/validate_tutorial_maps.sh` (CREATE NEW)
**Action**: Create validation script

**Code**:

```bash
#!/bin/bash
# SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
# SPDX-License-Identifier: Apache-2.0

# Validate tutorial map RON syntax and metadata usage

set -e

echo "Validating tutorial maps..."

MAPS_DIR="antares/data/campaigns/tutorial/maps"

for map_file in "$MAPS_DIR"/*.ron; do
    echo "Checking $map_file..."

    # Check for RON syntax errors by attempting to parse
    # This requires campaign_builder to have a validation mode
    cargo run -p campaign_builder -- validate "$map_file" || {
        echo "ERROR: Failed to validate $map_file"
        exit 1
    }

    echo "✓ $map_file is valid"
done

echo "All tutorial maps validated successfully!"
```

**Make executable**:

```bash
chmod +x antares/scripts/validate_tutorial_maps.sh
```

**Validation**:

- Run `./antares/scripts/validate_tutorial_maps.sh`
- Expected: All maps pass validation

#### 4.7 Deliverables

- [ ] Map 1 (Town Square) updated with wall height variations
- [ ] Map 2 (Forest Entrance) updated with tree types and grass density
- [ ] Map 3 (Dungeon Level 1) updated with rock variants and water flow
- [ ] Map 4 updated with snow-covered mountains
- [ ] Map 5 updated with palm trees and oasis
- [ ] Map 6 updated with swamp features
- [ ] All maps use valid RON syntax
- [ ] All maps load without errors in campaign_builder
- [ ] Validation script created and passes

#### 4.8 Success Criteria

**Automated Verification**:

```bash
# All must pass:
./antares/scripts/validate_tutorial_maps.sh
cargo run -p campaign_builder -- validate antares/data/campaigns/tutorial/maps/map_001_town_square.ron
cargo run -p campaign_builder -- validate antares/data/campaigns/tutorial/maps/map_002_forest_entrance.ron
# ... repeat for all maps
```

**Manual Verification** (load each map in campaign_builder):

- [ ] Map 1: Tall outer walls, short interior walls, decorative pillars visible
- [ ] Map 2: Oak, Pine, and Dead trees render distinctly; grass density gradient visible
- [ ] Map 3: Jagged cave walls, crystal formations, water flows east then south
- [ ] Map 4: Snow-covered mountains and pines render correctly
- [ ] Map 5: Palm trees around oasis, still water
- [ ] Map 6: Willow trees, murky swamp water with tint
- [ ] All maps use terrain-specific fields correctly (no irrelevant fields on wrong terrain types)

### Phase 5: Testing & Documentation

**Objective**: Comprehensive testing and user-facing documentation

#### 5.1 Add Unit Tests for Terrain Fields

**File**: `antares/src/domain/world/types.rs`
**Location**: In `#[cfg(test)] mod tests` section
**Action**: Add comprehensive unit tests

**Code to Add**:

```rust
#[cfg(test)]
mod terrain_enum_tests {
    use super::*;

    #[test]
    fn test_grass_density_default_is_medium() {
        assert_eq!(GrassDensity::default(), GrassDensity::Medium);
    }

    #[test]
    fn test_tree_type_default_is_oak() {
        assert_eq!(TreeType::default(), TreeType::Oak);
    }

    #[test]
    fn test_rock_variant_default_is_smooth() {
        assert_eq!(RockVariant::default(), RockVariant::Smooth);
    }

    #[test]
    fn test_water_flow_default_is_still() {
        assert_eq!(WaterFlowDirection::default(), WaterFlowDirection::Still);
    }

    #[test]
    fn test_grass_density_serializes_to_ron() {
        let density = GrassDensity::High;
        let ron = ron::to_string(&density).unwrap();
        assert_eq!(ron, "High");
    }

    #[test]
    fn test_tree_type_deserializes_from_ron() {
        let ron = "Pine";
        let tree_type: TreeType = ron::from_str(ron).unwrap();
        assert_eq!(tree_type, TreeType::Pine);
    }

    #[test]
    fn test_rock_variant_round_trip_serialization() {
        let variant = RockVariant::Crystal;
        let ron = ron::to_string(&variant).unwrap();
        let deserialized: RockVariant = ron::from_str(&ron).unwrap();
        assert_eq!(deserialized, variant);
    }

    #[test]
    fn test_water_flow_all_variants_serialize() {
        let variants = vec![
            WaterFlowDirection::Still,
            WaterFlowDirection::North,
            WaterFlowDirection::South,
            WaterFlowDirection::East,
            WaterFlowDirection::West,
        ];

        for variant in variants {
            let ron = ron::to_string(&variant).unwrap();
            let deserialized: WaterFlowDirection = ron::from_str(&ron).unwrap();
            assert_eq!(deserialized, variant);
        }
    }
}

#[cfg(test)]
mod tile_visual_metadata_terrain_tests {
    use super::*;

    #[test]
    fn test_metadata_with_grass_density_serializes() {
        let mut meta = TileVisualMetadata::default();
        meta.grass_density = Some(GrassDensity::VeryHigh);

        let ron = ron::to_string(&meta).unwrap();
        assert!(ron.contains("grass_density"));
        assert!(ron.contains("VeryHigh"));
    }

    #[test]
    fn test_metadata_without_terrain_fields_is_minimal() {
        let meta = TileVisualMetadata::default();
        let ron = ron::to_string(&meta).unwrap();

        // Should not contain terrain-specific fields
        assert!(!ron.contains("grass_density"));
        assert!(!ron.contains("tree_type"));
        assert!(!ron.contains("rock_variant"));
        assert!(!ron.contains("water_flow_direction"));
    }

    #[test]
    fn test_metadata_accessors_return_defaults() {
        let meta = TileVisualMetadata::default();

        assert_eq!(meta.grass_density(), GrassDensity::Medium);
        assert_eq!(meta.tree_type(), TreeType::Oak);
        assert_eq!(meta.rock_variant(), RockVariant::Smooth);
        assert_eq!(meta.water_flow_direction(), WaterFlowDirection::Still);
        assert_eq!(meta.foliage_density(), 1.0);
        assert_eq!(meta.snow_coverage(), 0.0);
    }

    #[test]
    fn test_has_terrain_overrides_detects_grass_density() {
        let mut meta = TileVisualMetadata::default();
        assert!(!meta.has_terrain_overrides());

        meta.grass_density = Some(GrassDensity::Low);
        assert!(meta.has_terrain_overrides());
    }

    #[test]
    fn test_has_terrain_overrides_detects_tree_type() {
        let mut meta = TileVisualMetadata::default();
        meta.tree_type = Some(TreeType::Dead);
        assert!(meta.has_terrain_overrides());
    }

    #[test]
    fn test_has_terrain_overrides_detects_all_fields() {
        let mut meta = TileVisualMetadata::default();

        meta.grass_density = Some(GrassDensity::High);
        assert!(meta.has_terrain_overrides());

        meta.tree_type = Some(TreeType::Pine);
        assert!(meta.has_terrain_overrides());

        meta.rock_variant = Some(RockVariant::Jagged);
        assert!(meta.has_terrain_overrides());

        meta.water_flow_direction = Some(WaterFlowDirection::North);
        assert!(meta.has_terrain_overrides());

        meta.foliage_density = Some(0.5);
        assert!(meta.has_terrain_overrides());

        meta.snow_coverage = Some(0.8);
        assert!(meta.has_terrain_overrides());
    }

    #[test]
    fn test_foliage_density_bounds() {
        let mut meta = TileVisualMetadata::default();

        meta.foliage_density = Some(0.0);
        assert_eq!(meta.foliage_density(), 0.0);

        meta.foliage_density = Some(2.0);
        assert_eq!(meta.foliage_density(), 2.0);
    }

    #[test]
    fn test_snow_coverage_bounds() {
        let mut meta = TileVisualMetadata::default();

        meta.snow_coverage = Some(0.0);
        assert_eq!(meta.snow_coverage(), 0.0);

        meta.snow_coverage = Some(1.0);
        assert_eq!(meta.snow_coverage(), 1.0);
    }
}
```

**Validation**:

- Run `cargo test terrain_enum_tests`
- Run `cargo test tile_visual_metadata_terrain_tests`
- Expected: All tests pass (18 tests total)

#### 5.2 Add UI Integration Tests

**File**: `antares/sdk/campaign_builder/tests/terrain_ui_tests.rs` (CREATE NEW)
**Action**: Add UI state tests

**Code**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for terrain-specific UI controls

use antares::domain::world::{GrassDensity, TreeType, RockVariant, WaterFlowDirection, TileVisualMetadata};
use campaign_builder::map_editor::TerrainEditorState;

#[test]
fn test_terrain_editor_state_default_values() {
    let state = TerrainEditorState::default();

    assert_eq!(state.grass_density, GrassDensity::Medium);
    assert_eq!(state.tree_type, TreeType::Oak);
    assert_eq!(state.rock_variant, RockVariant::Smooth);
    assert_eq!(state.water_flow_direction, WaterFlowDirection::Still);
    assert_eq!(state.foliage_density, 1.0);
    assert_eq!(state.snow_coverage, 0.0);
}

#[test]
fn test_terrain_editor_state_loads_from_metadata() {
    let mut metadata = TileVisualMetadata::default();
    metadata.grass_density = Some(GrassDensity::High);
    metadata.tree_type = Some(TreeType::Pine);
    metadata.foliage_density = Some(1.5);

    let state = TerrainEditorState::from_metadata(&metadata);

    assert_eq!(state.grass_density, GrassDensity::High);
    assert_eq!(state.tree_type, TreeType::Pine);
    assert_eq!(state.foliage_density, 1.5);
}

#[test]
fn test_terrain_editor_state_applies_to_metadata() {
    let mut state = TerrainEditorState::default();
    state.grass_density = GrassDensity::VeryHigh;
    state.tree_type = TreeType::Dead;
    state.snow_coverage = 0.8;

    let mut metadata = TileVisualMetadata::default();
    state.apply_to_metadata(&mut metadata);

    assert_eq!(metadata.grass_density, Some(GrassDensity::VeryHigh));
    assert_eq!(metadata.tree_type, Some(TreeType::Dead));
    assert_eq!(metadata.snow_coverage, Some(0.8));
}

#[test]
fn test_clear_metadata_removes_terrain_fields() {
    let mut metadata = TileVisualMetadata::default();
    metadata.grass_density = Some(GrassDensity::High);
    metadata.tree_type = Some(TreeType::Oak);
    metadata.rock_variant = Some(RockVariant::Jagged);

    TerrainEditorState::clear_metadata(&mut metadata);

    assert_eq!(metadata.grass_density, None);
    assert_eq!(metadata.tree_type, None);
    assert_eq!(metadata.rock_variant, None);
}

#[test]
fn test_terrain_editor_state_preserves_non_terrain_metadata() {
    let mut metadata = TileVisualMetadata::default();
    metadata.height = Some(2.5);
    metadata.tint_color = Some([1.0, 0.5, 0.2, 1.0]);

    let state = TerrainEditorState::default();
    state.apply_to_metadata(&mut metadata);

    // Non-terrain fields should be preserved
    assert_eq!(metadata.height, Some(2.5));
    assert_eq!(metadata.tint_color, Some([1.0, 0.5, 0.2, 1.0]));
}
```

**Validation**:

- Run `cargo test --test terrain_ui_tests`
- Expected: 5 tests passed

#### 5.3 Add Preset Categorization Tests

**File**: `antares/sdk/campaign_builder/tests/preset_tests.rs` (CREATE NEW)
**Action**: Test preset categorization logic

**Code**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Tests for visual preset categorization

use campaign_builder::map_editor::{VisualPreset, PresetCategory};

#[test]
fn test_wall_presets_categorized_correctly() {
    assert_eq!(VisualPreset::ShortWall.category(), PresetCategory::Walls);
    assert_eq!(VisualPreset::TallWall.category(), PresetCategory::Walls);
    assert_eq!(VisualPreset::ThinWall.category(), PresetCategory::Walls);
}

#[test]
fn test_nature_presets_categorized_correctly() {
    assert_eq!(VisualPreset::SmallTree.category(), PresetCategory::Nature);
    assert_eq!(VisualPreset::LargeTree.category(), PresetCategory::Nature);
    assert_eq!(VisualPreset::Bush.category(), PresetCategory::Nature);
    assert_eq!(VisualPreset::Boulder.category(), PresetCategory::Nature);
}

#[test]
fn test_water_presets_categorized_correctly() {
    assert_eq!(VisualPreset::ShallowWater.category(), PresetCategory::Water);
    assert_eq!(VisualPreset::DeepWater.category(), PresetCategory::Water);
    assert_eq!(VisualPreset::Lava.category(), PresetCategory::Water);
}

#[test]
fn test_structure_presets_categorized_correctly() {
    assert_eq!(VisualPreset::Pillar.category(), PresetCategory::Structures);
    assert_eq!(VisualPreset::Altar.category(), PresetCategory::Structures);
    assert_eq!(VisualPreset::Statue.category(), PresetCategory::Structures);
}

#[test]
fn test_by_category_filters_correctly() {
    let wall_presets = VisualPreset::by_category(PresetCategory::Walls);

    assert!(wall_presets.contains(&VisualPreset::ShortWall));
    assert!(wall_presets.contains(&VisualPreset::TallWall));
    assert!(!wall_presets.contains(&VisualPreset::SmallTree));
}

#[test]
fn test_by_category_all_returns_all_presets() {
    let all_presets = VisualPreset::by_category(PresetCategory::All);
    let expected_count = VisualPreset::all_presets().len();

    assert_eq!(all_presets.len(), expected_count);
}

#[test]
fn test_preset_category_display_names() {
    assert_eq!(PresetCategory::Walls.display_name(), "Walls");
    assert_eq!(PresetCategory::Nature.display_name(), "Nature");
    assert_eq!(PresetCategory::Water.display_name(), "Water");
    assert_eq!(PresetCategory::Structures.display_name(), "Structures");
    assert_eq!(PresetCategory::All.display_name(), "All");
}
```

**Validation**:

- Run `cargo test --test preset_tests`
- Expected: 7 tests passed

#### 5.4 Create User Guide Documentation

**File**: `antares/docs/how-to/use_terrain_specific_controls.md` (CREATE NEW)
**Action**: Write user-facing guide

**Content**:

```markdown
# How to Use Terrain-Specific Controls in Map Editor

This guide explains how to use the terrain-specific visual controls in the Campaign Builder's map editor.

## Overview

The map editor provides context-sensitive controls based on the terrain type of the selected tile. This allows you to customize grass density, tree types, rock variants, water flow, and more.

## Prerequisites

- Campaign Builder installed and running
- A map loaded in the map editor
- Basic familiarity with tile selection and inspector panel

## Accessing Terrain Controls

1. **Select a tile**: Click on any tile in the map grid
2. **Open inspector panel**: The right sidebar shows tile properties
3. **Locate terrain controls**: Scroll to "Terrain-Specific Settings" section

The controls shown depend on the tile's terrain type.

## Terrain-Specific Controls by Type

### Grassland and Plains Tiles

**Controls Available**:

- **Grass Density**: Dropdown with options (None, Low, Medium, High, VeryHigh)
- **Foliage Density**: Slider from 0.0 to 2.0

**Example Use Case**: Create a gradient from sparse grass (Low) to lush meadow (VeryHigh)

**Steps**:

1. Select a grassland tile
2. Choose "Grass Density" → "VeryHigh"
3. Adjust "Foliage Density" slider to 1.5
4. Observe the tile update with denser grass rendering

### Forest Tiles

**Controls Available**:

- **Tree Type**: Dropdown (Oak, Pine, Dead, Palm, Willow)
- **Foliage Density**: Slider from 0.0 to 2.0
- **Snow Coverage**: Slider from 0.0 to 1.0

**Example Use Case**: Create a snowy pine forest

**Steps**:

1. Select a forest tile
2. Choose "Tree Type" → "Pine"
3. Adjust "Snow Coverage" slider to 0.8
4. Set "Foliage Density" to 1.2

### Mountain and Hill Tiles

**Controls Available**:

- **Rock Variant**: Dropdown (Smooth, Jagged, Layered, Crystal)
- **Snow Coverage**: Slider from 0.0 to 1.0

**Example Use Case**: Create crystal cave formations

**Steps**:

1. Select a mountain tile
2. Choose "Rock Variant" → "Crystal"
3. Use the "Tint Color" control (in general section) to add blue tint
4. Result: Crystalline formations with colored highlights

### Water and Swamp Tiles

**Controls Available**:

- **Water Flow Direction**: Dropdown (Still, North, South, East, West)

**Example Use Case**: Create a flowing river

**Steps**:

1. Select multiple water tiles in sequence
2. For first tile: "Water Flow Direction" → "East"
3. For next tiles: Continue "East" direction
4. For bend: Change to "South" direction
5. Result: Animated water flow along the river path

### Desert and Snow Tiles

**Controls Available**:

- **Snow Coverage**: Slider from 0.0 to 1.0

**Example Use Case**: Partial snow on desert peaks

**Steps**:

1. Select a desert tile (representing rocky terrain)
2. Adjust "Snow Coverage" to 0.3 for light dusting
3. Result: Desert terrain with snow highlights

## Using Visual Presets

Visual presets provide quick access to common configurations.

**Steps**:

1. Select a tile
2. Locate "Visual Presets" section in inspector panel
3. Click category tabs to filter presets (All, Walls, Nature, Water, Structures)
4. Click a preset button to apply it

**Example Presets**:

- **SmallTree**: Sets height=2.0, scale=0.5, green tint
- **TallWall**: Sets height=3.5
- **ShallowWater**: Sets reduced height for water tiles

## Clearing Terrain Properties

To reset terrain-specific settings to defaults:

1. Select the tile
2. Scroll to "Terrain-Specific Settings" section
3. Click **"Clear Terrain Properties"** button
4. All terrain fields reset to None (using default values)

## Best Practices

- **Use gradients**: Vary grass density across adjacent tiles for natural transitions
- **Mix tree types**: Combine Oak and Pine in same forest for variety
- **Match terrain context**: Use Palm trees only in desert/tropical maps
- **Flow consistency**: Ensure water flow directions form logical paths
- **Snow realism**: Higher altitudes should have higher snow_coverage values

## Troubleshooting

**Q: Controls don't show for my selected tile**

- Verify a tile is actually selected (highlighted in grid)
- Check the terrain type matches expected controls (e.g., Forest shows tree_type)

**Q: Changes don't apply to tile**

- Ensure you're modifying the selected tile (check position in inspector)
- Verify map is not read-only

**Q: Preset doesn't match my terrain type**

- Presets apply their values regardless of terrain type
- Manually adjust terrain-specific controls after applying preset

## See Also

- [Map Editor Reference](../reference/map_editor_reference.md)
- [Visual Metadata Format](../reference/visual_metadata_format.md)
- [Tutorial Map Examples](../../data/campaigns/tutorial/maps/)
```

**Validation**:

- Verify markdown syntax with `markdownlint` (if available)
- Read through for clarity and completeness

#### 5.5 Create Technical Reference Documentation

**File**: `antares/docs/reference/tile_visual_metadata_specification.md` (CREATE NEW)
**Action**: Write technical specification

**Content**:

````markdown
# TileVisualMetadata Specification

Technical reference for the `TileVisualMetadata` struct and terrain-specific fields.

## Overview

`TileVisualMetadata` provides optional override values for tile rendering in the game engine and campaign builder. All fields are `Option<T>` types, allowing selective overrides while maintaining default values.

**Module**: `antares::domain::world::types`

**Serialization**: RON format (`.ron` files)

## Struct Definition

```rust
pub struct TileVisualMetadata {
    // Generic visual properties
    pub height: Option<f32>,
    pub width_x: Option<f32>,
    pub width_z: Option<f32>,
    pub tint_color: Option<[f32; 4]>,
    pub scale: Option<f32>,

    // Terrain-specific properties (Phase 1 additions)
    pub grass_density: Option<GrassDensity>,
    pub tree_type: Option<TreeType>,
    pub rock_variant: Option<RockVariant>,
    pub water_flow_direction: Option<WaterFlowDirection>,
    pub foliage_density: Option<f32>,
    pub snow_coverage: Option<f32>,
}
```
````

## Terrain-Specific Enums

### GrassDensity

**Purpose**: Control grass blade count for grassland/plains terrain

**Variants**:

- `None`: No grass blades (bare dirt)
- `Low`: 10-20 blades per tile
- `Medium`: 40-60 blades per tile (default)
- `High`: 80-120 blades per tile
- `VeryHigh`: 150+ blades per tile

**RON Example**:

```ron
(
    grass_density: Some(High),
)
```

### TreeType

**Purpose**: Visual variant for forest terrain

**Variants**:

- `Oak`: Deciduous tree with broad leaves (default)
- `Pine`: Coniferous tree with needle leaves
- `Dead`: Bare tree without foliage
- `Palm`: Tropical palm tree
- `Willow`: Drooping willow tree

**RON Example**:

```ron
(
    tree_type: Some(Pine),
    foliage_density: Some(1.2),
)
```

### RockVariant

**Purpose**: Visual variant for mountain/hill terrain

**Variants**:

- `Smooth`: Rounded boulders (default)
- `Jagged`: Sharp, angular rocks
- `Layered`: Sedimentary rock layers
- `Crystal`: Crystalline formations

**RON Example**:

```ron
(
    rock_variant: Some(Crystal),
    tint_color: Some([0.5, 0.5, 1.0, 1.0]),
)
```

### WaterFlowDirection

**Purpose**: Animated water flow for rivers/streams

**Variants**:

- `Still`: No flow animation (default)
- `North`: Flowing northward
- `South`: Flowing southward
- `East`: Flowing eastward
- `West`: Flowing westward

**RON Example**:

```ron
(
    water_flow_direction: Some(East),
)
```

## Scalar Fields

### foliage_density

**Type**: `Option<f32>`

**Range**: 0.0 to 2.0

**Default**: 1.0

**Purpose**: Multiplier for foliage/vegetation density

**Example**:

```ron
(
    foliage_density: Some(1.5),  // 50% more foliage
)
```

### snow_coverage

**Type**: `Option<f32>`

**Range**: 0.0 to 1.0

**Default**: 0.0

**Purpose**: Percentage of tile covered in snow

**Example**:

```ron
(
    snow_coverage: Some(0.8),  // 80% snow coverage
)
```

## Serialization Behavior

### skip_serializing_if = "Option::is_none"

All fields use `#[serde(skip_serializing_if = "Option::is_none")]` attribute:

**None values are omitted** from serialized output:

```ron
// This:
TileVisualMetadata { grass_density: None, tree_type: None, ... }

// Serializes to:
()

// Not:
(grass_density: None, tree_type: None, ...)
```

**Some values are included**:

```ron
// This:
TileVisualMetadata { grass_density: Some(High), tree_type: None, ... }

// Serializes to:
(grass_density: Some(High))
```

## Helper Methods

### Accessor Methods with Defaults

```rust
impl TileVisualMetadata {
    pub fn grass_density(&self) -> GrassDensity;          // Default: Medium
    pub fn tree_type(&self) -> TreeType;                  // Default: Oak
    pub fn rock_variant(&self) -> RockVariant;            // Default: Smooth
    pub fn water_flow_direction(&self) -> WaterFlowDirection; // Default: Still
    pub fn foliage_density(&self) -> f32;                 // Default: 1.0
    pub fn snow_coverage(&self) -> f32;                   // Default: 0.0
}
```

**Usage**:

```rust
let metadata = TileVisualMetadata::default();
assert_eq!(metadata.grass_density(), GrassDensity::Medium);  // Returns default
```

### has_terrain_overrides()

```rust
pub fn has_terrain_overrides(&self) -> bool;
```

**Returns**: `true` if any terrain-specific field is `Some`, `false` otherwise

**Usage**:

```rust
let mut metadata = TileVisualMetadata::default();
assert!(!metadata.has_terrain_overrides());

metadata.grass_density = Some(GrassDensity::High);
assert!(metadata.has_terrain_overrides());
```

## Usage in Map Files

**File**: `antares/data/campaigns/{campaign_name}/maps/{map_name}.ron`

**Structure**:

```ron
(
    name: "Example Map",
    size: (width: 20, height: 20),
    tiles: {
        (x: 0, y: 0): (terrain: Grassland, wall: None),
        (x: 1, y: 0): (terrain: Forest, wall: None),
    },
    events: {},
    metadata: (
        tile_visual_metadata: {
            (x: 0, y: 0): (
                grass_density: Some(VeryHigh),
                foliage_density: Some(1.8),
            ),
            (x: 1, y: 0): (
                tree_type: Some(Oak),
                height: Some(3.5),
                tint_color: Some([0.2, 0.8, 0.2, 1.0]),
            ),
        },
    ),
)
```

## Validation Rules

1. **Optional Fields**: All fields can be `None` (valid default state)
2. **Range Validation**:
   - `foliage_density`: 0.0 ≤ value ≤ 2.0 (enforced at UI level)
   - `snow_coverage`: 0.0 ≤ value ≤ 1.0 (enforced at UI level)
3. **Enum Variants**: Must match defined enum values exactly
4. **RON Syntax**: Must be valid RON format (enforced by `ron` crate)

## See Also

- [Map Editor User Guide](../how-to/use_terrain_specific_controls.md)
- [RON Format Specification](https://github.com/ron-rs/ron)
- [Tutorial Map Examples](../../data/campaigns/tutorial/maps/)

````

**Validation**:
- Verify markdown syntax
- Ensure all code examples are valid

#### 5.6 Deliverables

- [ ] 18 unit tests added to `antares/src/domain/world/types.rs` and passing
- [ ] 5 UI integration tests added to `antares/sdk/campaign_builder/tests/terrain_ui_tests.rs` and passing
- [ ] 7 preset tests added to `antares/sdk/campaign_builder/tests/preset_tests.rs` and passing
- [ ] User guide created at `antares/docs/how-to/use_terrain_specific_controls.md`
- [ ] Technical reference created at `antares/docs/reference/tile_visual_metadata_specification.md`
- [ ] All documentation follows Diataxis framework (How-To vs Reference)
- [ ] All code examples in documentation are valid
- [ ] `cargo test` passes (all tests: existing + new)

#### 5.7 Success Criteria

**Automated Verification**:
```bash
# All tests must pass:
cargo test terrain_enum_tests
cargo test tile_visual_metadata_terrain_tests
cargo test --test terrain_ui_tests
cargo test --test preset_tests

# Total: 30 new tests passing
````

**Documentation Verification**:

- [ ] User guide includes screenshots/examples for all terrain types
- [ ] Technical reference includes all enum variants with descriptions
- [ ] All code blocks specify language (`rust` not generic)
- [ ] Markdown files use `lowercase_with_underscores.md` naming
- [ ] No broken internal links in documentation

## Cross-Phase Validation

### Architecture Compliance Checklist

Before marking implementation complete, verify:

- [ ] Consulted `antares/docs/reference/architecture.md` before modifying domain layer
- [ ] All new domain types added to `antares/src/domain/world/types.rs` (correct location)
- [ ] All new UI code added to `antares/sdk/campaign_builder/src/map_editor.rs` (correct layer)
- [ ] No domain layer dependencies on SDK layer (one-way dependency)
- [ ] RON format used exclusively for map data (no JSON/YAML)
- [ ] All enums are `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]`
- [ ] All optional fields use `#[serde(default, skip_serializing_if = "Option::is_none")]`
- [ ] Type aliases not required (GrassDensity is standalone enum, not alias)
- [ ] No magic numbers (all enums use named variants)
- [ ] SPDX copyright headers added to all new `.rs` files

### Quality Gates (All Phases)

**Run after EVERY phase completion**:

```bash
# Step 1: Format
cargo fmt --all

# Step 2: Compile check
cargo check --all-targets --all-features

# Step 3: Lint
cargo clippy --all-targets --all-features -- -D warnings

# Step 4: Test
cargo test --all-features

# Expected: 0 errors, 0 warnings, all tests pass
```

### Deliverable Handoff Criteria

Phase is complete when:

1. **Code Quality**: All 4 cargo commands pass with 0 errors/warnings
2. **Tests**: All new tests passing + existing tests still pass
3. **Documentation**: Phase-specific documentation updated in `docs/explanation/implementations.md`
4. **Architecture**: No deviations from architecture.md without explicit approval
5. **Manual Verification**: UI changes manually tested in campaign_builder (where applicable)

## Success Metrics (Feature Complete)

Feature is complete when ALL of the following are true:

**Domain Layer**:

- [ ] 4 new enums added (GrassDensity, TreeType, RockVariant, WaterFlowDirection)
- [ ] 6 new fields added to TileVisualMetadata
- [ ] 7 helper methods implemented
- [ ] 18 unit tests passing

**Campaign Builder**:

- [ ] TerrainEditorState struct implemented
- [ ] show_terrain_specific_controls() function renders correctly for all terrain types
- [ ] Preset categorization (PresetCategory enum + filtering) working
- [ ] show_preset_palette() function displays 3-column grid
- [ ] Inspector panel integrates both terrain controls and preset palette
- [ ] Tile selection synchronizes TerrainEditorState
- [ ] 12 integration tests passing

**Tutorial Maps**:

- [ ] All 6 tutorial maps updated with terrain-specific metadata
- [ ] Map validation script passes for all maps
- [ ] Maps showcase variety: walls, trees, rocks, water flow, snow

**Documentation**:

- [ ] User guide explains all terrain controls with examples
- [ ] Technical reference documents all enums and fields
- [ ] Documentation placed in correct Diataxis categories

**Quality**:

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes (0 errors)
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes (0 warnings)
- [ ] `cargo test --all-features` passes (30 new tests + all existing)
- [ ] Total test count: 30+ new tests

## Risk Mitigation

### Risk 1: RON Serialization Breaking Changes

**Risk**: Adding new fields to TileVisualMetadata breaks existing map files

**Mitigation**:

- All new fields are `Option<T>` with `#[serde(default)]`
- Existing maps without new fields deserialize with None values
- Backward compatibility maintained

**Validation**:

- Load existing maps before and after changes
- Verify no deserialization errors

### Risk 2: UI Performance with Many Controls

**Risk**: Inspector panel becomes slow with many dropdowns/sliders

**Mitigation**:

- Context-sensitive controls (only show relevant controls per terrain type)
- Maximum ~4 controls per terrain type
- egui handles this scale efficiently

**Validation**:

- Manually test UI responsiveness in campaign_builder

### Risk 3: Preset Categorization Incompleteness

**Risk**: Some presets not categorized correctly

**Mitigation**:

- Explicit categorization in `VisualPreset::category()` method
- Test coverage for all preset categories
- Default to `PresetCategory::All` for uncategorized presets

**Validation**:

- `cargo test preset_tests` verifies categorization

## Dependencies & Prerequisites

### Required Tools

- Rust toolchain (latest stable)
- `cargo-nextest` (for improved test running)
- Campaign Builder binary (`cargo run -p campaign_builder`)

### Crate Dependencies

**Already in Cargo.toml** (no additions required):

- `serde` (for serialization)
- `ron` (for RON format)
- `egui` (for UI rendering in campaign_builder)

### Phase Dependencies

**Sequential Requirements**:

1. Phase 1 must complete before Phase 2 (UI depends on domain types)
2. Phase 2 must complete before Phase 3 (preset UI depends on terrain controls)
3. Phase 4 can run in parallel with Phase 2/3 (map updates independent)
4. Phase 5 requires all previous phases complete

**Parallel Work Opportunities**:

- Phase 1 and Phase 4 can start simultaneously (domain work vs map updates)
- Documentation (Phase 5.4-5.5) can be drafted during Phase 2-3 implementation

## Timeline Summary

**Total Estimated Effort**: 12-16 agent work sessions

**Phase 1**: 2-3 sessions (domain layer + tests)
**Phase 2**: 3-4 sessions (UI controls + integration)
**Phase 3**: 2-3 sessions (preset categorization + palette)
**Phase 4**: 2-3 sessions (update 6 tutorial maps)
**Phase 5**: 3-4 sessions (testing + documentation)

**Critical Path**: Phase 1 → Phase 2 → Phase 5

## Open Questions for User Review

1. **Preset Completeness**: Do we have a complete list of all `VisualPreset` variants? The plan assumes SmallTree, LargeTree, Bush, Boulder, ShallowWater, DeepWater, Lava, Pillar, Altar, Statue exist - should we audit the actual enum?

2. **Additional Terrain Types**: Should we add controls for other terrain types (Cave, Road, Bridge)? Current plan covers Grassland, Forest, Mountain, Water, Desert, Snow.

3. **Validation Script Location**: Should `validate_tutorial_maps.sh` be in `antares/scripts/` or `antares/sdk/campaign_builder/scripts/`?

4. **Documentation Scope**: Do we need a migration guide for existing map authors, or is the user guide sufficient?

5. **Snow Coverage Semantics**: Should snow_coverage apply to ALL terrain types (e.g., snowy grassland), or only Mountain/Snow/Desert? Current plan allows it on Forest, Mountain, Desert, Snow.

6. **Water Flow Rendering**: Does the game engine already support rendering water flow animations, or is this a future feature? Plan assumes UI controls are sufficient.

7. **Foliage Density Range**: Is 0.0-2.0 the correct range, or should it be 0.0-1.0 (percentage-based)? Current plan uses 0.0-2.0 as multiplier.

8. **Inspector Panel Location**: Where exactly is the inspector panel rendering function in `map_editor.rs`? Plan needs approximate line number for integration point.

9. **Test Coverage Target**: Is >80% test coverage required, or is comprehensive unit testing of new functionality sufficient?

10. **Campaign Builder Validate Command**: Does campaign_builder currently support a `validate` subcommand, or do we need to add it for the map validation script?
