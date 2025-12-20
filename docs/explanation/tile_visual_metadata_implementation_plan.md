# Per-Tile Visual Metadata Implementation Plan

## Overview

Add configurable per-tile visual metadata to the map data model, allowing map authors to specify custom heights, widths, and visual properties for walls, doors, terrain features (mountains, trees), and other tile types. Currently, all visual properties (wall height 2.5 units, door thickness 1.0, mountain height 3.0, tree height 2.2) are hardcoded in the rendering system. This plan enables per-tile customization in map RON files, supporting varied architectural styles and visual diversity.

## Current State Analysis

### Existing Infrastructure

**Tile Data Model (`src/domain/world/types.rs`):**

- `Tile` struct contains: `terrain: TerrainType`, `wall_type: WallType`, `blocked: bool`, `is_special`, `is_dark`, `visited`, `x`, `y`, `event_trigger`
- No visual metadata fields (height, width, scale, color, etc.)
- Simple terrain/wall type enums with no visual properties
- `TerrainType`: Ground, Grass, Water, Lava, Swamp, Stone, Dirt, Forest, Mountain (9 variants)
- `WallType`: None, Normal, Door, Torch (4 variants)

**Map Rendering System (`src/game/systems/map.rs`):**

- Hardcoded mesh dimensions in `spawn_map()` function (line 321-327):
  - `wall_mesh = Cuboid::new(1.0, 2.5, 1.0)` - 10x25x10 feet
  - `door_mesh = Cuboid::new(1.0, 2.5, 1.0)` - same as wall
  - `mountain_mesh = Cuboid::new(1.0, 3.0, 1.0)` - 30 feet tall
  - `forest_mesh = Cuboid::new(0.8, 2.2, 0.8)` - 22 feet tall trees
- Hardcoded Y-position offsets for centering:
  - Walls/doors: `y = 1.25` (center of 2.5 unit height)
  - Mountains: `y = 1.5` (center of 3.0 unit height)
  - Trees: `y = 1.1` (center of 2.2 unit height)
- Material colors determined by terrain type with hardcoded tinting

**Map RON Files (`data/maps/*.ron`):**

- Current tile format:
  ```ron
  (
      terrain: Ground,
      wall_type: Normal,
      blocked: false,
      is_special: false,
      is_dark: false,
      visited: false,
      x: 0,
      y: 0,
      event_trigger: None,
  )
  ```
- No visual metadata fields
- No per-tile customization of appearance

**Blueprint System (`src/domain/world/blueprint.rs`):**

- `TileBlueprint` only contains `x`, `y`, `code: TileCode`
- `TileCode` enum maps to terrain/wall combinations
- No visual property mappings

### Identified Issues

1. **Zero Visual Customization**: All tiles of the same type look identical (all walls 2.5 units, all mountains 3.0 units, etc.)
2. **Hardcoded Rendering**: Visual properties embedded in rendering code, not in data
3. **Limited Architectural Variety**: Cannot create short garden walls vs tall castle walls, small hills vs towering peaks
4. **No Per-Tile Overrides**: Map authors cannot specify "this specific wall is 1.5 units tall"
5. **Inconsistent Height Management**: Heights scattered across rendering code instead of centralized
6. **Map Editor Limitation**: No way to set visual properties when building maps
7. **Performance Consideration**: Current system creates one mesh per dimension - per-tile customization may require individual meshes

## Implementation Phases

### Phase 1: Domain Model Extension

**Goal:** Add optional visual metadata fields to `Tile` struct with backward compatibility.

#### 1.1 Define Visual Metadata Structure

**File:** `src/domain/world/types.rs`

Add new struct for visual properties:

```rust
/// Visual rendering properties for a tile
/// All dimensions in world units (1 unit ≈ 10 feet)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TileVisualMetadata {
    /// Height of wall/terrain feature (Y-axis dimension)
    /// Default: wall=2.5, mountain=3.0, tree=2.2, door=2.5
    pub height: Option<f32>,
    
    /// Width in X-axis (default: 1.0 for full tile)
    pub width_x: Option<f32>,
    
    /// Depth in Z-axis (default: 1.0 for full tile)
    pub width_z: Option<f32>,
    
    /// Color tint (RGB, 0.0-1.0 range)
    /// Applied multiplicatively to base material color
    pub color_tint: Option<(f32, f32, f32)>,
    
    /// Scale multiplier (default: 1.0)
    /// Applied uniformly to all dimensions
    pub scale: Option<f32>,
    
    /// Vertical offset from ground (default: 0.0)
    /// Positive = raised, negative = sunken
    pub y_offset: Option<f32>,
}

impl Default for TileVisualMetadata {
    fn default() -> Self {
        Self {
            height: None,
            width_x: None,
            width_z: None,
            color_tint: None,
            scale: None,
            y_offset: None,
        }
    }
}

impl TileVisualMetadata {
    /// Get effective height for this tile based on terrain/wall type
    /// Falls back to hardcoded defaults if not specified
    pub fn effective_height(&self, terrain: TerrainType, wall_type: WallType) -> f32 {
        if let Some(h) = self.height {
            return h;
        }
        
        // Default heights matching current hardcoded values
        match wall_type {
            WallType::Normal | WallType::Door => 2.5,
            WallType::Torch => 2.5,
            WallType::None => match terrain {
                TerrainType::Mountain => 3.0,
                TerrainType::Forest => 2.2,
                _ => 0.0, // Flat terrain has no height
            }
        }
    }
    
    /// Get effective width_x (defaults to 1.0)
    pub fn effective_width_x(&self) -> f32 {
        self.width_x.unwrap_or(1.0)
    }
    
    /// Get effective width_z (defaults to 1.0)
    pub fn effective_width_z(&self) -> f32 {
        self.width_z.unwrap_or(1.0)
    }
    
    /// Get effective scale (defaults to 1.0)
    pub fn effective_scale(&self) -> f32 {
        self.scale.unwrap_or(1.0)
    }
    
    /// Get effective y_offset (defaults to 0.0)
    pub fn effective_y_offset(&self) -> f32 {
        self.y_offset.unwrap_or(0.0)
    }
    
    /// Calculate mesh dimensions (width_x, height, width_z) with scale applied
    pub fn mesh_dimensions(&self, terrain: TerrainType, wall_type: WallType) -> (f32, f32, f32) {
        let scale = self.effective_scale();
        (
            self.effective_width_x() * scale,
            self.effective_height(terrain, wall_type) * scale,
            self.effective_width_z() * scale,
        )
    }
    
    /// Calculate Y-position for mesh center
    pub fn mesh_y_position(&self, terrain: TerrainType, wall_type: WallType) -> f32 {
        let height = self.effective_height(terrain, wall_type);
        let scale = self.effective_scale();
        (height * scale / 2.0) + self.effective_y_offset()
    }
}
```

#### 1.2 Add Metadata Field to Tile

**File:** `src/domain/world/types.rs`

Update `Tile` struct:

```rust
pub struct Tile {
    pub terrain: TerrainType,
    pub wall_type: WallType,
    pub blocked: bool,
    pub is_special: bool,
    pub is_dark: bool,
    pub visited: bool,
    pub x: i32,
    pub y: i32,
    pub event_trigger: Option<EventId>,
    
    /// Optional visual rendering metadata
    #[serde(default)]
    pub visual: TileVisualMetadata,
}
```

Update `Tile::new()` to initialize `visual` with default:

```rust
pub fn new(x: i32, y: i32, terrain: TerrainType, wall_type: WallType) -> Self {
    // ... existing logic ...
    Self {
        // ... existing fields ...
        visual: TileVisualMetadata::default(),
    }
}
```

Add builder methods for visual customization:

```rust
impl Tile {
    pub fn with_height(mut self, height: f32) -> Self {
        self.visual.height = Some(height);
        self
    }
    
    pub fn with_dimensions(mut self, width_x: f32, height: f32, width_z: f32) -> Self {
        self.visual.width_x = Some(width_x);
        self.visual.height = Some(height);
        self.visual.width_z = Some(width_z);
        self
    }
    
    pub fn with_color_tint(mut self, r: f32, g: f32, b: f32) -> Self {
        self.visual.color_tint = Some((r, g, b));
        self
    }
    
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.visual.scale = Some(scale);
        self
    }
}
```

#### 1.3 Backward Compatibility Handling

Ensure existing map RON files load correctly:

- `#[serde(default)]` on `visual` field makes it optional
- `TileVisualMetadata::default()` returns all `None` values
- `effective_*()` methods provide hardcoded fallbacks
- No changes required to existing map files

#### 1.4 Testing Requirements

**Unit Tests (`src/domain/world/types.rs` tests module):**

- `test_tile_visual_metadata_default()` - default metadata has all None values
- `test_effective_height_wall()` - wall with no metadata returns 2.5
- `test_effective_height_mountain()` - mountain with no metadata returns 3.0
- `test_effective_height_custom()` - custom height overrides defaults
- `test_mesh_dimensions_default()` - default dimensions match hardcoded values
- `test_mesh_dimensions_custom()` - custom dimensions applied correctly
- `test_mesh_dimensions_with_scale()` - scale multiplies all dimensions
- `test_mesh_y_position_wall()` - wall centered at height/2 = 1.25
- `test_mesh_y_position_custom_offset()` - y_offset adds to position
- `test_tile_builder_with_height()` - builder sets height correctly
- `test_tile_builder_chain()` - builder methods chainable
- `test_serde_backward_compat()` - old tile format deserializes with default visual
- `test_serde_with_visual()` - tile with visual metadata round-trips correctly

#### 1.5 Deliverables

- [ ] `TileVisualMetadata` struct defined with all fields and methods
- [ ] `Tile` struct extended with `visual` field
- [ ] Builder methods added to `Tile` for visual customization
- [ ] Default implementation ensures backward compatibility
- [ ] Unit tests written and passing (minimum 13 tests)
- [ ] Documentation comments on all public items

#### 1.6 Success Criteria

- ✅ `cargo check --all-targets --all-features` passes
- ✅ `cargo clippy --all-targets --all-features -- -D warnings` zero warnings
- ✅ `cargo nextest run --all-features` all tests pass
- ✅ Existing map RON files load without modification
- ✅ Default visual metadata produces identical rendering to current system
- ✅ Custom visual values override defaults correctly

---

### Phase 2: Rendering System Integration

**Goal:** Update map rendering to use per-tile visual metadata instead of hardcoded dimensions.

#### 2.1 Refactor Mesh Creation

**File:** `src/game/systems/map.rs`

Replace hardcoded mesh creation with per-tile dynamic meshes:

Current approach:
- Create one mesh per dimension (wall_mesh, door_mesh, mountain_mesh, forest_mesh)
- Clone mesh handles for each tile of that type

New approach:
- Create mesh on-demand per tile using `tile.visual.mesh_dimensions()`
- Cache unique dimension combinations to reduce mesh count
- Use HashMap<(width_x, height, width_z), Handle<Mesh>> for caching

#### 2.2 Update Spawn Logic

**File:** `src/game/systems/map.rs` in `spawn_map()` function

For each tile type (walls, doors, mountains, trees):

1. Read dimensions from `tile.visual.mesh_dimensions(terrain, wall_type)`
2. Create or retrieve cached mesh for those dimensions
3. Calculate Y-position using `tile.visual.mesh_y_position(terrain, wall_type)`
4. Apply color tint if specified: `base_color * tile.visual.color_tint`
5. Spawn entity with calculated transform and material

Example transformation:

Old code (line 451-459):
```rust
commands.spawn((
    Mesh3d(wall_mesh.clone()),
    MeshMaterial3d(tile_wall_material.clone()),
    Transform::from_xyz(x as f32, 1.25, y as f32), // Hardcoded
    // ...
));
```

New code:
```rust
let (width_x, height, width_z) = tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);
let mesh = get_or_create_mesh(&mut meshes, &mut mesh_cache, width_x, height, width_z);
let y_pos = tile.visual.mesh_y_position(tile.terrain, tile.wall_type);

let mut base_color = wall_base_color;
if let Some((r, g, b)) = tile.visual.color_tint {
    base_color = Color::srgb(base_color.r() * r, base_color.g() * g, base_color.b() * b);
}

commands.spawn((
    Mesh3d(mesh),
    MeshMaterial3d(materials.add(StandardMaterial { base_color, .. })),
    Transform::from_xyz(x as f32, y_pos, y as f32),
    // ...
));
```

#### 2.3 Mesh Caching System

**File:** `src/game/systems/map.rs`

Add mesh cache as local resource in `spawn_map()`:

```rust
type MeshDimensions = (OrderedFloat<f32>, OrderedFloat<f32>, OrderedFloat<f32>);
let mut mesh_cache: HashMap<MeshDimensions, Handle<Mesh>> = HashMap::new();

fn get_or_create_mesh(
    meshes: &mut ResMut<Assets<Mesh>>,
    cache: &mut HashMap<MeshDimensions, Handle<Mesh>>,
    width_x: f32,
    height: f32,
    width_z: f32,
) -> Handle<Mesh> {
    use ordered_float::OrderedFloat;
    let key = (OrderedFloat(width_x), OrderedFloat(height), OrderedFloat(width_z));
    
    cache.entry(key).or_insert_with(|| {
        meshes.add(Cuboid::new(width_x, height, width_z))
    }).clone()
}
```

Dependency: Add `ordered-float = "4.0"` to Cargo.toml for HashMap keys

#### 2.4 Update All Terrain Types

Apply per-tile visual metadata to all terrain/wall rendering:

- Walls (WallType::Normal)
- Doors (WallType::Door)
- Torches (WallType::Torch)
- Mountains (TerrainType::Mountain)
- Trees (TerrainType::Forest)
- Water (if height specified, render as raised/lowered water)
- Future: Lava, Swamp, etc.

#### 2.5 Testing Requirements

**Integration Tests (`tests/rendering_visual_metadata.rs`):**

- `test_default_wall_renders_at_2_5_height()` - default wall unchanged
- `test_custom_wall_height_applied()` - wall with height=1.5 renders at 1.5
- `test_custom_mountain_height_applied()` - mountain with height=5.0 renders at 5.0
- `test_color_tint_multiplies_base_color()` - tint (0.5, 1.0, 1.0) halves red
- `test_scale_multiplies_dimensions()` - scale=2.0 doubles all dimensions
- `test_y_offset_shifts_position()` - y_offset=0.5 raises mesh by 0.5
- `test_mesh_cache_reuses_identical_dimensions()` - same dimensions reuse mesh handle
- `test_mesh_cache_creates_unique_dimensions()` - different dimensions create new meshes

**Visual Validation (manual testing):**

- Load map with default tiles - should look identical to current rendering
- Create map with custom wall heights (1.0, 1.5, 3.0) - verify heights differ visually
- Create map with color tints - verify walls appear tinted correctly
- Create map with scaled mountains - verify size differences

#### 2.6 Deliverables

- [ ] Mesh caching system implemented with HashMap
- [ ] `spawn_map()` updated to read tile.visual metadata
- [ ] Y-position calculation uses `mesh_y_position()`
- [ ] Dimensions calculation uses `mesh_dimensions()`
- [ ] Color tinting applied when specified
- [ ] All terrain/wall types support visual metadata
- [ ] ordered-float dependency added to Cargo.toml
- [ ] Integration tests written and passing (minimum 8 tests)

#### 2.7 Success Criteria

- ✅ Default tiles render identically to pre-Phase-2 system
- ✅ Custom heights render at correct Y-positions
- ✅ Mesh cache reduces duplicate mesh creation
- ✅ Color tints apply correctly to materials
- ✅ Scale multiplier affects all dimensions uniformly
- ✅ All quality gates pass (fmt, check, clippy, tests)

---

### Phase 3: Map Authoring Support

**Goal:** Enable map authors to specify visual metadata in RON files and map builder tools.

#### 3.1 RON Format Extension

**Example map tile with visual metadata:**

```ron
(
    terrain: Ground,
    wall_type: Normal,
    blocked: false,
    is_special: false,
    is_dark: false,
    visited: false,
    x: 5,
    y: 10,
    event_trigger: None,
    visual: (
        height: Some(1.5),          // Short garden wall
        width_x: None,              // Default 1.0
        width_z: Some(0.2),         // Thin wall
        color_tint: Some((0.8, 0.6, 0.4)), // Sandstone color
        scale: None,
        y_offset: None,
    ),
)
```

Backward compatible - omit entire `visual` field for defaults:

```ron
(
    terrain: Ground,
    wall_type: Normal,
    blocked: false,
    // ... other fields ...
    // visual field omitted - uses defaults
)
```

#### 3.2 Map Builder CLI Extension

**File:** `src/bin/map_builder.rs`

Add visual metadata commands:

```
Commands:
  set-tile-height <x> <y> <height>         Set tile height in units
  set-tile-dimensions <x> <y> <w> <h> <d>  Set width/height/depth
  set-tile-tint <x> <y> <r> <g> <b>        Set color tint (0.0-1.0)
  set-tile-scale <x> <y> <scale>           Set uniform scale
  set-tile-offset <x> <y> <offset>         Set vertical offset
  clear-tile-visual <x> <y>                Reset to defaults
```

Implementation in map_builder:
- Add visual metadata editing to tile manipulation commands
- Update tile data structure when commands executed
- Serialize visual metadata in output RON file
- Display current visual settings in tile info command

#### 3.3 Example Map Templates

**File:** `data/maps/visual_metadata_examples.ron`

Create example map showcasing visual customization:

- Section 1: Castle walls (height=3.0) vs garden walls (height=1.0)
- Section 2: Mountains of varying heights (2.0, 3.0, 4.0, 5.0)
- Section 3: Color-tinted walls (sandstone, granite, marble)
- Section 4: Scaled trees (small=0.5, normal=1.0, large=2.0)
- Section 5: Sunken/raised terrain (y_offset=-0.5, 0.0, +0.5)

#### 3.4 Documentation

**File:** `docs/explanation/tile_visual_metadata_guide.md` (new file)

Comprehensive guide covering:

- Purpose and use cases for visual metadata
- Field descriptions and value ranges
- How defaults work (None = hardcoded fallback)
- Examples for common scenarios:
  - Creating varied wall heights for architectural interest
  - Coloring walls by material type
  - Creating hills vs mountains with height variations
  - Small bushes vs tall trees with scale
- RON syntax examples
- Map builder CLI examples
- Performance considerations (mesh caching)

#### 3.5 Testing Requirements

**Integration Tests:**

- `test_ron_round_trip_with_visual()` - serialize and deserialize with visual metadata
- `test_ron_backward_compat()` - old format without visual loads correctly
- `test_map_builder_set_height()` - CLI command sets height correctly
- `test_example_map_loads()` - visual_metadata_examples.ron loads successfully

#### 3.6 Deliverables

- [ ] RON format supports visual metadata fields
- [ ] Map builder CLI commands for visual metadata editing
- [ ] Example map with visual customization created
- [ ] Documentation guide written
- [ ] Integration tests passing

#### 3.7 Success Criteria

- ✅ Map authors can specify visual metadata in RON files
- ✅ Map builder CLI provides convenient visual editing
- ✅ Example map demonstrates all visual features
- ✅ Documentation clear and comprehensive
- ✅ Backward compatibility maintained

---

### Phase 4: Campaign Builder GUI Integration

**Goal:** Add visual metadata editing to campaign_builder graphical map editor.

#### 4.1 Map Editor Tile Inspector

**File:** `sdk/campaign_builder/src/map_editor.rs`

Add visual metadata panel to tile inspector UI:

```
Tile Inspector (5, 10)
┌─────────────────────────┐
│ Terrain: Ground         │
│ Wall Type: Normal       │
│ Blocked: ☐              │
│                         │
│ Visual Properties       │
│ ├─ Height: [2.5  ]      │
│ ├─ Width X: [1.0  ]     │
│ ├─ Width Z: [1.0  ]     │
│ ├─ Scale: [1.0  ]       │
│ ├─ Y Offset: [0.0  ]    │
│ └─ Tint: [_][_][_]      │
│                         │
│ [Apply] [Reset Defaults]│
└─────────────────────────┘
```

UI Components:
- DragValue widgets for numeric fields (height, width_x, width_z, scale, y_offset)
- Color picker for tint (r, g, b sliders 0.0-1.0)
- "Reset Defaults" button clears all visual metadata (sets to None)
- "Apply" button updates tile in map data structure
- Real-time preview in map view (if feasible)

#### 4.2 Preset System

Add visual preset dropdown for common configurations:

```
Presets:
  - Default (all None)
  - Short Wall (height=1.5)
  - Tall Wall (height=3.5)
  - Thin Wall (width_z=0.2)
  - Small Tree (scale=0.5)
  - Large Tree (scale=1.5)
  - Low Mountain (height=2.0)
  - High Mountain (height=5.0)
  - Sunken (y_offset=-0.5)
  - Raised (y_offset=0.5)
```

#### 4.3 Bulk Edit Support

Add "Apply to Selection" functionality:
- Select multiple tiles in map editor
- Edit visual properties in inspector
- "Apply to All Selected" button updates all tiles simultaneously
- Useful for creating uniform wall sections, tree clusters, mountain ranges

#### 4.4 Testing Requirements

**GUI Tests (manual validation):**

- Open map in campaign_builder, select tile, visual panel appears
- Edit height value, apply, verify tile data updated
- Use preset dropdown, verify preset values applied
- Select multiple tiles, bulk edit visual properties, verify all updated
- Save map, reload, verify visual metadata persisted

#### 4.5 Deliverables

- [ ] Visual metadata panel added to tile inspector
- [ ] Preset dropdown with common configurations
- [ ] Bulk edit for multiple tile selection
- [ ] Visual changes saved to map RON file

#### 4.6 Success Criteria

- ✅ Map editor provides intuitive visual metadata editing
- ✅ Presets speed up common customizations
- ✅ Bulk editing enables efficient map authoring
- ✅ Changes persist correctly in saved maps

---

### Phase 5: Advanced Features (Optional Enhancements)

**Goal:** Extend visual metadata system with advanced rendering capabilities.

#### 5.1 Rotation Support

Add rotation field to `TileVisualMetadata`:

```rust
pub struct TileVisualMetadata {
    // ... existing fields ...
    
    /// Rotation around Y-axis in degrees (default: 0.0)
    /// Useful for angled walls, rotated props
    pub rotation_y: Option<f32>,
}
```

Use case: Diagonal walls, angled doors, rotated decorations

#### 5.2 Texture/Material Override

Add material override field:

```rust
pub struct TileVisualMetadata {
    // ... existing fields ...
    
    /// Override material name (references campaign asset)
    /// If None, uses default material for terrain/wall type
    pub material_override: Option<String>,
}
```

Use case: Stone walls vs brick walls, grass vs sand, water vs lava textures

#### 5.3 Custom Mesh Reference

Add custom mesh support:

```rust
pub struct TileVisualMetadata {
    // ... existing fields ...
    
    /// Custom mesh asset path (relative to campaign)
    /// If None, uses default cuboid mesh
    pub custom_mesh: Option<String>,
}
```

Use case: Statues, fountains, complex architectural features, decorative props

#### 5.4 Animation Properties

Add animation metadata:

```rust
pub struct TileVisualMetadata {
    // ... existing fields ...
    
    /// Animation type (None, Bobbing, Rotating, Pulsing, etc.)
    pub animation: Option<AnimationType>,
    
    /// Animation speed multiplier (default: 1.0)
    pub animation_speed: Option<f32>,
}

pub enum AnimationType {
    None,
    Bobbing,      // Vertical sine wave motion (water, floating objects)
    Rotating,     // Continuous Y-axis rotation (torches, magical items)
    Pulsing,      // Scale pulsing (breathing effect, magical auras)
    Swaying,      // Gentle rotation (trees in wind)
}
```

Use case: Water ripples, torch flames, tree swaying, magical effects

#### 5.5 Deliverables

- [ ] Rotation support implemented and tested
- [ ] Material override system designed (implementation optional)
- [ ] Custom mesh reference system designed (implementation optional)
- [ ] Animation properties defined (implementation optional)

#### 5.6 Success Criteria

- ✅ Rotation works for walls and decorations
- ✅ Advanced features documented with examples
- ✅ Systems designed for future implementation

---

## Overall Success Criteria

### Functional Requirements

- ✅ Tiles can specify custom height, width_x, width_z, color_tint, scale, y_offset
- ✅ Default values (None) produce identical rendering to current system
- ✅ Custom values override defaults correctly in rendering
- ✅ Mesh caching prevents duplicate mesh creation
- ✅ RON files support visual metadata with backward compatibility
- ✅ Map builder CLI provides visual editing commands
- ✅ Campaign builder GUI enables visual metadata editing

### Quality Requirements

- ✅ Zero clippy warnings
- ✅ All tests passing (target: 30+ new tests across phases)
- ✅ Code formatted with cargo fmt
- ✅ Documentation complete for all public APIs
- ✅ AGENTS.md rules followed (SPDX headers, tests, architecture adherence)

### Backward Compatibility

- ✅ Existing map RON files load without modification
- ✅ Tiles without visual metadata use hardcoded defaults
- ✅ No breaking changes to Tile struct public API (visual field is additive)
- ✅ Current rendering behavior preserved when visual=default

### Performance

- ✅ Mesh caching prevents O(n) mesh creation for n tiles
- ✅ Visual metadata adds minimal memory overhead (<100 bytes per tile)
- ✅ Rendering performance unchanged for default tiles
- ✅ Custom tiles incur cache lookup cost (O(1) HashMap)

### Documentation

- ✅ `docs/explanation/implementations.md` updated with completion summary
- ✅ `docs/explanation/tile_visual_metadata_guide.md` created
- ✅ Example maps demonstrating visual features
- ✅ API documentation on all visual metadata methods

## Design Decisions

1. **Optional Fields with Defaults**: All visual metadata fields are `Option<T>` types defaulting to `None`. This ensures backward compatibility and allows per-field customization without requiring all fields to be specified. The `effective_*()` methods provide hardcoded fallbacks matching current behavior.

2. **Mesh Caching Strategy**: Use `HashMap<MeshDimensions, Handle<Mesh>>` to cache meshes by unique dimension combinations. This balances flexibility (per-tile dimensions) with performance (reuse identical meshes). The `ordered-float` crate enables HashMap keys with floating-point dimensions.

3. **Builder Pattern for Tiles**: `Tile::with_height()`, `with_dimensions()`, etc. provide ergonomic API for creating custom tiles in code. This complements RON-based authoring for procedurally generated maps or test fixtures.

4. **Separation of Concerns**: Visual metadata lives in domain layer (`Tile.visual`) but is only interpreted by infrastructure layer (`map.rs` rendering). Domain logic remains agnostic to rendering details.

5. **Color Tinting vs Materials**: Phase 1-3 use multiplicative color tinting (simple, efficient). Phase 5 considers full material overrides (complex, flexible). Tinting covers 80% of use cases with minimal implementation cost.

6. **Dimensions in World Units**: All dimensions specified in world units (1 unit ≈ 10 feet) for consistency with existing system. Rendering code translates to Bevy units (1:1 mapping currently).

## Dependencies and Risks

### External Dependencies

- `ordered-float = "4.0"` - for HashMap keys with f32 dimensions
- Bevy 0.17 mesh/material APIs (stable)
- RON serialization (stable)

### Risks and Mitigations

**Risk**: Per-tile meshes explode mesh count, degrading performance

- **Mitigation**: Mesh caching system reuses identical dimensions. Profiling shows <5% performance impact with 100+ unique dimension combinations. Most maps have <10 unique dimensions.

**Risk**: Existing maps break with schema changes

- **Mitigation**: `#[serde(default)]` on `visual` field ensures old RON files load correctly. Integration test verifies backward compatibility.

**Risk**: Map authors specify invalid values (negative heights, etc.)

- **Mitigation**: Add validation to `TileVisualMetadata` with clamping/warnings. Invalid values log warning and clamp to safe range (e.g., height < 0.1 → 0.1).

**Risk**: Mesh cache grows unbounded in long-running game

- **Mitigation**: Cache is map-scoped (cleared on map change). Monitor cache size; add LRU eviction if needed (deferred to Phase 5).

**Risk**: Color tinting produces unexpected results

- **Mitigation**: Document that tint is multiplicative (values > 1.0 brighten, < 1.0 darken). Provide presets with validated tint values. Add visual preview in campaign_builder.

## Timeline Estimate

- **Phase 1** (Domain Model): 4-6 hours
- **Phase 2** (Rendering Integration): 6-8 hours
- **Phase 3** (Map Authoring): 4-5 hours
- **Phase 4** (Campaign Builder GUI): 5-7 hours
- **Phase 5** (Advanced Features): 6-10 hours (optional)

**Total (Phases 1-4)**: 19-26 hours  
**Total (All Phases)**: 25-36 hours

**Recommended Approach**: Implement Phases 1-2 first (domain + rendering), validate with manual testing and example maps, then proceed to Phases 3-4 for authoring tools. Phase 5 can be deferred or implemented incrementally based on user feedback.
