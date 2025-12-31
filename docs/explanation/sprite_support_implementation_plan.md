# Tile Sprite Support Implementation Plan

## Overview

Add sprite-based visual rendering for tiles, enabling map authors to replace default 3D mesh representations (cuboid walls, floor planes) with 2D billboard sprites. This extends the tile visual metadata system (prerequisite: `tile_visual_metadata_implementation_plan.md`) to support texture-based visuals for walls, doors, terrain features, and decorative elements. Sprites render as billboards facing the camera, providing classic RPG visual style with efficient texture atlas-based rendering.

## Prerequisites

> [!IMPORTANT]
> Complete **Phases 1-2** of [tile_visual_metadata_implementation_plan.md](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/tile_visual_metadata_implementation_plan.md) before beginning this plan. Specifically:
> - `TileVisualMetadata` struct must exist in `src/domain/world/types.rs`
> - Rendering system in `src/game/systems/map.rs` must support per-tile visual metadata
> - Mesh caching infrastructure must be operational

## Current State Analysis

### Existing Infrastructure

**Tile Data Model (`src/domain/world/types.rs`):**

- `Tile` struct has 9 fields: `terrain`, `wall_type`, `blocked`, `is_special`, `is_dark`, `visited`, `x`, `y`, `event_trigger`
- After tile visual metadata plan: will have `visual: TileVisualMetadata` field
- No sprite/texture reference fields currently exist
- `TerrainType` enum: Ground, Grass, Water, Lava, Swamp, Stone, Dirt, Forest, Mountain
- `WallType` enum: None, Normal, Door, Torch

**Map Rendering System (`src/game/systems/map.rs`):**

- `spawn_map()` function creates 3D meshes for all tile types
- Uses `Mesh3d`, `MeshMaterial3d`, `StandardMaterial` for 3D rendering
- Meshes are hardcoded `Cuboid` and `Plane3d` primitives
- No sprite or texture atlas infrastructure exists
- Materials use solid colors with roughness properties

**Asset Directory Structure:**

- `assets/porrtraits/` - character portraits (existing, note: directory misspelled)
- No sprite sheet or tile texture directories exist
- Campaign assets in `campaigns/{name}/data/` directory structure

**Dependencies (`Cargo.toml`):**

- `bevy = { version = "0.17", default-features = true }` - includes sprite rendering
- No `bevy_sprite3d` or texture atlas dependencies
- No image processing crates for sprite generation

### Technology Decision: PNG vs SVG

**Recommendation: Use PNG sprite sheets with `TextureAtlas`**

| Aspect | PNG Sprites | SVG |
|--------|-------------|-----|
| Bevy Support | First-class, highly optimized | Community crates only (`bevy_svg`) |
| Performance | Pre-rasterized, GPU-ready | Requires tessellation on load |
| Batching | Automatic sprite batching | Mesh-based, less efficient |
| Tooling | Standard game dev workflow | Limited editor support |
| Animation | Frame-based, TextureAtlas indexed | Complex path morphing |
| Scalability | Fixed resolution (mipmaps help) | Infinite (at runtime cost) |

The tile visual metadata system's `scale` field handles size variation.
Pre-render sprites at 128x128 or 128x256 pixels, use mipmaps for quality
at different distances.

### Identified Issues

1. **No Sprite Infrastructure**: No texture loading, atlas management, or sprite rendering code exists
2. **3D-Only Rendering**: Current system only uses 3D meshes, no 2D sprite capability
3. **No Billboard Support**: No mechanism to make 2D sprites face the camera in 3D world
4. **No Asset Pipeline**: No tooling to create or manage sprite sheets
5. **No SDK Integration**: Map editor cannot select/preview sprites for tiles
6. **Missing Dependency**: Need `bevy_sprite3d` crate for 3D world sprite rendering

## Implementation Phases

### Phase 1: Sprite Metadata Extension

**Goal:** Extend `TileVisualMetadata` with optional sprite reference fields.

#### 1.1 Define Sprite Reference Structure

**File:** `src/domain/world/types.rs`

Add sprite configuration to the visual metadata struct (after tile_visual_metadata plan):

- Create `SpriteReference` struct with:
  - `sheet_path: String` - path to sprite sheet image (relative to campaign or global assets)
  - `sprite_index: u32` - index within texture atlas grid (0-indexed)
  - `animation: Option<SpriteAnimation>` - optional animation configuration

- Create `SpriteAnimation` struct with:
  - `frames: Vec<u32>` - frame indices in animation order
  - `fps: f32` - frames per second (default: 8.0)
  - `looping: bool` - whether animation loops (default: true)

#### 1.2 Extend TileVisualMetadata

**File:** `src/domain/world/types.rs`

Add sprite field to the existing `TileVisualMetadata` struct:

- Add `#[serde(default)]` `sprite: Option<SpriteReference>` field
- This field replaces default mesh rendering when set
- When None, uses existing 3D mesh rendering

#### 1.3 Add Sprite Helper Methods

**File:** `src/domain/world/types.rs`

Add helper methods to `TileVisualMetadata`:

- `uses_sprite() -> bool` - returns true if sprite rendering is enabled
- `sprite_sheet_path() -> Option<&str>` - get sprite sheet path
- `sprite_index() -> Option<u32>` - get sprite index
- `has_animation() -> bool` - check if sprite has animation config

#### 1.4 Builder Methods for Sprites

**File:** `src/domain/world/types.rs`

Add builder methods to `Tile`:

- `with_sprite(sheet_path: &str, sprite_index: u32) -> Self` - set static sprite
- `with_animated_sprite(sheet_path: &str, frames: Vec<u32>, fps: f32, looping: bool) -> Self` - set animated sprite

#### 1.5 Testing Requirements

**Unit Tests (`src/domain/world/types.rs` tests module):**

- `test_sprite_reference_serialization()` - SpriteReference round-trips through RON
- `test_sprite_animation_defaults()` - SpriteAnimation defaults are correct (fps=8, looping=true)
- `test_tile_visual_uses_sprite()` - `uses_sprite()` returns true when sprite set
- `test_tile_visual_no_sprite()` - `uses_sprite()` returns false when sprite is None
- `test_sprite_sheet_path_accessor()` - `sprite_sheet_path()` returns correct path
- `test_tile_with_sprite_builder()` - `with_sprite()` sets sprite correctly
- `test_tile_with_animated_sprite_builder()` - `with_animated_sprite()` sets animation
- `test_backward_compat_no_sprite_field()` - old RON without sprite field loads correctly

#### 1.6 Deliverables

- [ ] `SpriteReference` struct defined with sheet_path, sprite_index, animation
- [ ] `SpriteAnimation` struct defined with frames, fps, looping
- [ ] `TileVisualMetadata.sprite` field added with `#[serde(default)]`
- [ ] Helper methods added: `uses_sprite()`, `sprite_sheet_path()`, `sprite_index()`, `has_animation()`
- [ ] Builder methods added: `with_sprite()`, `with_animated_sprite()`
- [ ] Unit tests written and passing (minimum 8 tests)

#### 1.7 Success Criteria

- ✅ `cargo check --all-targets --all-features` passes
- ✅ `cargo clippy --all-targets --all-features -- -D warnings` zero warnings
- ✅ `cargo nextest run --all-features` all tests pass
- ✅ Existing tile RON files without sprite field load correctly
- ✅ New tile RON files with sprite field serialize/deserialize correctly

---

### Phase 2: Sprite Asset Infrastructure

**Goal:** Set up sprite sheet loading, texture atlas management, and asset pipeline.

#### 2.1 Add bevy_sprite3d Dependency

**File:** `Cargo.toml`

Add dependency for 3D world sprite rendering:

- Add `bevy_sprite3d = "3.0"` (compatible with Bevy 0.17)

#### 2.2 Create Sprite Asset Loader

**File:** `src/game/resources/sprite_assets.rs` (new file)

Create resource to manage sprite textures and atlases:

- `SpriteSheetConfig` struct with: path, columns, rows, tile_width, tile_height
- `SpriteAssets` resource (HashMap-based cache) with:
  - `get_or_load(path, config, asset_server, atlas_layouts)` - lazy load and cache
  - `contains(path) -> bool` - check if sheet is loaded
  - `clear()` - clear all cached sheets

#### 2.3 Create Sprite Sheet Registry

**File:** `data/sprite_sheets.ron` (new file)

Define sprite sheet configurations in a registry file with entries for:

- `walls` - 4x4 grid, 128x256 pixels (stone, brick, wood variants)
- `doors` - 4x2 grid, 128x256 pixels (open, closed, locked variants)
- `terrain` - 8x8 grid, 128x128 pixels (floor tiles)
- `trees` - 4x4 grid, 128x256 pixels (vegetation)
- `decorations` - 8x8 grid, 64x64 pixels (small objects)

#### 2.4 Add Sprite Registry Data Structure

**File:** `src/domain/world/sprites.rs` (new file)

Define the registry structure:

- `SpriteSheetEntry` struct with: id, path, columns, rows, tile_width, tile_height, description
- `SpriteSheetRegistry` struct with:
  - `load_from_file(path) -> Result<Self>` - load from RON
  - `get(id) -> Option<&SpriteSheetEntry>` - find by ID
  - `sprite_count(id) -> Option<u32>` - get total sprites in sheet

#### 2.5 Create Directory Structure

Create asset directories (sprite images will be hand-crafted later):

- `assets/sprites/walls.png` - 4x4 grid (512x1024)
- `assets/sprites/doors.png` - 4x2 grid (512x512)
- `assets/sprites/terrain.png` - 8x8 grid (1024x1024)
- `assets/sprites/trees.png` - 4x4 grid (512x1024)
- `assets/sprites/decorations.png` - 8x8 grid (512x512)

#### 2.6 Testing Requirements

**Unit Tests:**

- `test_sprite_sheet_config_default()` - default config has 4x4 grid, 64x64 tiles
- `test_sprite_assets_get_or_load()` - loading a sheet caches it correctly
- `test_sprite_assets_contains()` - `contains()` returns true after loading
- `test_sprite_sheet_registry_load()` - registry loads from RON successfully
- `test_sprite_sheet_registry_get()` - `get()` finds sheet by ID
- `test_sprite_count()` - `sprite_count()` returns columns * rows

#### 2.7 Deliverables

- [ ] `bevy_sprite3d = "3.0"` added to Cargo.toml
- [ ] `SpriteAssets` resource created with `get_or_load()`, `contains()`, `clear()`
- [ ] `SpriteSheetRegistry` struct created with `load_from_file()`, `get()`, `sprite_count()`
- [ ] `data/sprite_sheets.ron` registry file created
- [ ] `assets/sprites/` directory created with structure documented
- [ ] Unit tests written and passing (minimum 6 tests)

#### 2.8 Success Criteria

- ✅ `bevy_sprite3d` compiles and links correctly
- ✅ Sprite sheet registry loads without errors
- ✅ `SpriteAssets` caches loaded textures correctly
- ✅ All quality gates pass

---

### Phase 3: Sprite Rendering Integration

**Goal:** Update map rendering system to render sprites for tiles with sprite metadata.

#### 3.1 Add Sprite3d Plugin

**File:** `src/game/mod.rs` or `src/game/plugins.rs`

Add the `bevy_sprite3d::Sprite3dPlugin` to the game plugin set.

#### 3.2 Create Sprite Rendering Components

**File:** `src/game/systems/map.rs`

Add new components:

- `TileSprite` component: `sheet_path: String`, `sprite_index: u32`
- `AnimatedTileSprite` component: `frames: Vec<u32>`, `fps: f32`, `looping: bool`, `current_frame: usize`, `timer: f32`

#### 3.3 Modify spawn_map for Hybrid Rendering

**File:** `src/game/systems/map.rs`

Update `spawn_map()` to:

1. Check `tile.visual.uses_sprite()` for each tile
2. If true: call `spawn_sprite_tile()` to create Sprite3d billboard entity
3. If false: call existing mesh spawning code

`spawn_sprite_tile()` should:
- Get texture and atlas handles from `SpriteAssets`
- Create `Sprite3d` bundle with billboard settings (`double_sided: true`)
- Set `pixels_per_metre: 128.0` (1 world unit = 128 pixels base)
- Position at tile coordinates using `tile.visual.mesh_y_position()`
- Apply scale from `tile.visual.effective_scale()`
- Tag with `MapEntity`, `TileCoord`, `TileSprite`

#### 3.4 Add Sprite Animation System

**File:** `src/game/systems/map.rs`

Create `animate_tile_sprites` system:

- Query for `(TextureAtlas, AnimatedTileSprite)` entities
- Increment timer by `time.delta_secs()`
- When timer exceeds `1.0 / fps`, advance to next frame
- Handle looping vs non-looping animation
- Update `TextureAtlas.index` to current frame

#### 3.5 Testing Requirements

**Integration Tests:**

- `test_sprite_tile_spawns_sprite3d()` - tile with sprite renders as Sprite3d, not mesh
- `test_mesh_tile_spawns_mesh()` - tile without sprite renders as mesh
- `test_sprite_scale_applied()` - sprite scale matches tile.visual.scale
- `test_sprite_y_offset_applied()` - sprite position includes y_offset
- `test_animated_sprite_updates()` - animated sprite changes frames over time
- `test_animation_loops_correctly()` - looping animation resets to frame 0
- `test_sprite_and_mesh_coexist()` - map with mixed tiles renders both types

#### 3.6 Deliverables

- [ ] `Sprite3dPlugin` added to game plugins
- [ ] `TileSprite` component created
- [ ] `spawn_tile_sprites()` or modified `spawn_map()` handles sprite tiles
- [ ] `AnimatedTileSprite` component and `animate_tile_sprites()` system created
- [ ] Hybrid rendering (mesh + sprite) works correctly
- [ ] Integration tests written and passing (minimum 7 tests)

#### 3.7 Success Criteria

- ✅ Tiles with sprite metadata render as billboards facing camera
- ✅ Tiles without sprite metadata render as 3D meshes (unchanged behavior)
- ✅ Sprite scale and positioning use TileVisualMetadata values
- ✅ Animated sprites cycle through frames correctly
- ✅ Performance acceptable with 100+ sprite tiles

---

### Phase 4: Sprite Asset Creation Guide

**Goal:** Document sprite creation workflow and provide starter assets.

#### 4.1 Sprite Creation Guide

**File:** `docs/tutorials/creating_sprites.md` (new file)

Create comprehensive guide covering:

- Sprite sheet specifications (dimensions, grid sizes, formats)
- Recommended formats: PNG-24 with alpha transparency
- Creating sprites with GIMP, Aseprite, Inkscape
- Registering sprite sheets in `data/sprite_sheets.ron`
- Using sprites in map RON definitions
- Animation configuration examples

Hand-craft sprite sheets with the following specifications:

| File | Grid | Sprite Size | Sheet Size | Content |
|------|------|-------------|------------|---------|
| `walls.png` | 4x4 | 128x256 | 512x1024 | Stone, brick, wood, damaged walls |
| `doors.png` | 4x2 | 128x256 | 512x512 | Wooden/iron doors (open/closed) |
| `terrain.png` | 8x8 | 128x128 | 1024x1024 | Stone, grass, dirt, water floors |
| `trees.png` | 4x4 | 128x256 | 512x1024 | Deciduous, conifer, dead, magical |

#### 4.3 Testing Requirements

**Validation Tests:**

- `test_starter_sprite_sheets_exist()` - all documented sheets exist in assets/sprites/
- `test_sprite_registry_matches_files()` - registry entries match actual files
- `test_sprite_dimensions_valid()` - sprites match documented dimensions

#### 4.4 Deliverables

- [ ] `docs/tutorials/creating_sprites.md` guide created
- [ ] `assets/sprites/walls.png` hand-crafted (512x1024, 4x4 grid)
- [ ] `assets/sprites/doors.png` hand-crafted (512x512, 4x2 grid)
- [ ] `assets/sprites/terrain.png` hand-crafted (1024x1024, 8x8 grid)
- [ ] `assets/sprites/trees.png` hand-crafted (512x1024, 4x4 grid)
- [ ] Registry in `data/sprite_sheets.ron` matches actual assets

#### 4.5 Success Criteria

- ✅ Documentation complete and accurate
- ✅ All registered sprite sheets exist and load correctly
- ✅ Sample sprites render correctly in-game
- ✅ Guide provides clear steps for creating new sprites

---

### Phase 5: Campaign Builder SDK Integration

**Goal:** Add sprite selection and preview to the Campaign Builder map editor.

#### 5.1 Add Sprite Browser Panel

**File:** `sdk/campaign_builder/src/map_editor.rs`

Add `SpriteBrowserState` struct:

- `selected_sheet: Option<String>` - currently selected sprite sheet
- `selected_sprite: Option<u32>` - currently selected sprite index
- `registry: Option<SpriteSheetRegistry>` - loaded registry
- `preview_textures: HashMap<String, egui::TextureHandle>` - preview cache

Add `show_sprite_browser()` method:

- ComboBox to select sprite sheet from registry
- Grid view of sprites in selected sheet
- Click to select sprite

#### 5.2 Add Sprite Field to Tile Inspector

**File:** `sdk/campaign_builder/src/map_editor.rs`

Extend tile inspector panel:

- Show current sprite setting (sheet/index or "None")
- "Browse..." button to open sprite browser
- "Clear" button to remove sprite
- Preview image of selected sprite
- Integration with existing visual properties (height, scale, etc.)

#### 5.3 Add Sprite Preview in Map View

**File:** `sdk/campaign_builder/src/map_editor.rs`

Modify map grid rendering:

- For tiles with sprites: draw sprite texture at tile position
- Use UV coordinates to select correct sprite from atlas
- For tiles without sprites: draw colored rectangle (existing behavior)

#### 5.4 Testing Requirements

**GUI Validation (Manual):**

- Open map editor, select tile, sprite browser appears
- Select sprite from browser, preview updates
- Apply sprite to tile, tile data updated
- Save map, reload, sprite setting persisted
- Clear sprite returns to mesh rendering

#### 5.5 Deliverables

- [ ] `SpriteBrowserState` struct with sprite sheet/sprite selection
- [ ] Sprite browser panel with grid view of available sprites
- [ ] Sprite field in tile inspector with preview
- [ ] Map view shows sprite previews instead of color blocks
- [ ] Sprite settings persist correctly in saved maps

#### 5.6 Success Criteria

- ✅ Map editor provides intuitive sprite selection
- ✅ Sprite preview accurate and helpful
- ✅ Changes persist correctly in saved maps
- ✅ Existing maps without sprites continue to function

---

### Phase 6: Advanced Features (Optional)

**Goal:** Add advanced sprite features based on user feedback.

#### 6.1 Sprite Layering

Support multiple sprites per tile:

- Add `decoration_sprites: Vec<SpriteReference>` field to `TileVisualMetadata`
- Render base sprite, then overlay decorations
- Use case: floor tile + rug + furniture

#### 6.2 Procedural Sprite Selection

Auto-select sprites based on terrain/wall type:

- Create `SpriteAutoMapping` struct mapping terrain/wall types to sprite ranges
- Auto-randomize sprite index within range for variety
- Use case: varied stone floor tiles without manual assignment

#### 6.3 Sprite Material Properties

Add material overrides for sprite rendering:

- Add `emissive: Option<f32>` for glow effects
- Add `alpha: Option<f32>` for transparency override
- Use case: glowing magical tiles, semi-transparent water

#### 6.4 Deliverables

- [ ] Sprite layering system designed (implementation optional)
- [ ] Procedural sprite selection system designed (implementation optional)
- [ ] Sprite material properties designed (implementation optional)

---

## Overall Success Criteria

### Functional Requirements

- ✅ Tiles can specify sprite instead of default mesh rendering
- ✅ Sprites render as billboards facing the camera
- ✅ Sprite animations cycle through frames correctly
- ✅ Sprite scale and offset use TileVisualMetadata values
- ✅ Sprite sheets load and cache efficiently
- ✅ Campaign Builder allows sprite selection and preview

### Quality Requirements

- ✅ Zero clippy warnings
- ✅ All tests passing (target: 25+ new tests across phases)
- ✅ Code formatted with cargo fmt
- ✅ Documentation complete for all public APIs
- ✅ AGENTS.md rules followed (SPDX headers, tests, architecture adherence)

### Backward Compatibility

- ✅ Existing map RON files without sprite field load correctly
- ✅ Tiles without sprite metadata use 3D mesh rendering
- ✅ No breaking changes to Tile struct public API
- ✅ Current rendering behavior preserved when sprite=None

### Performance

- ✅ Sprite atlas batching minimizes draw calls
- ✅ Texture caching prevents redundant loads
- ✅ Hybrid rendering (mesh + sprite) performs well
- ✅ Animation system uses delta time efficiently

## Dependencies

### External Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `bevy_sprite3d` | 3.0 | Billboard sprite rendering in 3D world |
| `bevy` | 0.17 | Core engine (existing) |

### Internal Dependencies

| Component | Dependency |
|-----------|------------|
| Phase 1 | Tile Visual Metadata Plan (Phases 1-2 complete) |
| Phase 2 | Phase 1 complete |
| Phase 3 | Phase 2 complete, bevy_sprite3d added |
| Phase 4 | Phase 3 complete |
| Phase 5 | Phase 4 complete |

## Risks and Mitigations

**Risk**: `bevy_sprite3d` incompatible with Bevy 0.17

- **Mitigation**: Verify crate compatibility before starting Phase 2. Alternative: implement custom billboard sprite rendering using Bevy's built-in `Sprite` with Transform facing camera.

**Risk**: Sprite rendering performance degrades with many tiles

- **Mitigation**: Texture atlas batching should handle 1000+ sprites. If issues arise, implement frustum culling for off-screen sprites.

**Risk**: PNG sprites don't scale well at close camera distances

- **Mitigation**: Generate mipmaps for sprite textures. Alternatively, provide multiple resolution sprite sheets.

**Risk**: Existing maps break with schema changes

- **Mitigation**: `#[serde(default)]` on sprite field ensures old RON files load correctly.

## Timeline Estimate

- **Phase 1** (Sprite Metadata): 3-4 hours
- **Phase 2** (Asset Infrastructure): 4-5 hours
- **Phase 3** (Rendering Integration): 6-8 hours
- **Phase 4** (Asset Creation Guide): 3-4 hours
- **Phase 5** (SDK Integration): 5-7 hours
- **Phase 6** (Advanced Features): 4-8 hours (optional)

**Total (Phases 1-5)**: 21-28 hours
**Total (All Phases)**: 25-36 hours

**Recommended Approach**: Implement Phases 1-3 first to establish working sprite rendering. Phase 4 (documentation + starter assets) can be done in parallel with Phase 3. Phase 5 (SDK) follows once core rendering is stable. Phase 6 is optional based on user feedback.
