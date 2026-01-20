# Tile Sprite Support Implementation Plan

> [!WARNING] > **UPDATE 2025-01**: This plan has been modified to support the unified "2.5D" rendering approach:
>
> - **Phase 2 EXPANDED**: Now primary pipeline for all actor entities (NPCs, Monsters, Recruitables)
> - **Phase 3 REFINED**: Uses native Bevy PBR billboard approach (no `bevy_sprite3d` dependency)
> - **Dependency Change**: Native `bevy::pbr` only - more stable, better lighting integration
> - **Rationale**: All "actors" (character entities) use billboard sprites for consistent visual style
> - See `plan_updates_review.md` for complete architectural decision

## Overview

Add sprite-based visual rendering for tiles **and all character entities**, enabling map authors to replace default 3D mesh representations (cuboid walls, floor planes) with 2D billboard sprites. This extends the tile visual metadata system (prerequisite: `tile_visual_metadata_implementation_plan.md`) to support texture-based visuals for walls, doors, terrain features, and decorative elements. **Character Rendering Philosophy**: All "actors" (NPCs, Monsters, Recruitables) use billboard sprites, while environmental objects (trees, signs, portals) use procedural 3D meshes. Sprites render as billboards facing the camera, providing classic RPG visual style with efficient texture atlas-based rendering.

## Prerequisites

> [!IMPORTANT]
> Complete **Phases 1-2** of [tile_visual_metadata_implementation_plan.md](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/tile_visual_metadata_implementation_plan.md) before beginning this plan. Specifically:
>
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
- **Character entities** (NPCs, Monsters) currently use placeholder cuboids
- **No character sprite system exists**

**Asset Directory Structure:**

- `assets/porrtraits/` - character portraits (existing, note: directory misspelled)
- No sprite sheet or tile texture directories exist
- Campaign assets in `campaigns/{name}/data/` directory structure

**Dependencies (`Cargo.toml`):**

- `bevy = { version = "0.17", default-features = true }` - includes sprite rendering
- No external sprite dependencies needed (using native Bevy PBR)
- No image processing crates for sprite generation

### Technology Decision: Native Bevy PBR Billboard vs bevy_sprite3d

**Recommendation: Use native Bevy PBR billboard rendering**

| Aspect       | Native Bevy PBR Billboard             | bevy_sprite3d                    |
| ------------ | ------------------------------------- | -------------------------------- |
| Stability    | First-class Bevy support              | External crate, version coupling |
| Lighting     | Full PBR lighting integration         | Limited lighting support         |
| Performance  | Optimized by Bevy core team           | Community maintained             |
| Dependencies | Zero external dependencies            | Adds external dependency         |
| Flexibility  | Full control over materials/rendering | Plugin-based configuration       |

Native Bevy PBR provides better stability and lighting integration. Implementation uses:

- `PbrBundle` with `StandardMaterial` (alpha blend enabled)
- `Rectangle` mesh for quad geometry
- Custom `Billboard` component + system for camera-facing rotation
- UV transforms for texture atlas sprite selection

### Identified Issues

1. **No Sprite Infrastructure**: No texture loading, atlas management, or sprite rendering code exists
2. **3D-Only Rendering**: Current system only uses 3D meshes, no 2D sprite capability
3. **No Billboard Support**: No mechanism to make 2D sprites face the camera in 3D world
4. **No Asset Pipeline**: No tooling to create or manage sprite sheets
5. **No SDK Integration**: Map editor cannot select/preview sprites for tiles
6. **No Character Sprite System**: NPCs, Monsters, Recruitables render as placeholder cuboids
7. **Inconsistent Character Rendering**: Need unified approach for all actor entities

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

**Goal:** Set up sprite sheet loading, texture atlas management, and asset pipeline for tiles **and all character entities**.

#### 2.1 No External Dependencies Required

**File:** `Cargo.toml`

**No changes needed** - using native `bevy::pbr` and `bevy::render` modules only.

Native Bevy provides all required functionality:

- `StandardMaterial` for texture rendering with alpha blend
- `Rectangle` mesh for quad geometry
- `Handle<Image>` for texture loading
- UV transforms for texture atlas sprite selection

#### 2.2 Create Sprite Asset Loader

**File:** `src/game/resources/sprite_assets.rs` (new file)

Create resource to manage sprite materials and quad meshes for billboard rendering:

```rust
use bevy::prelude::*;
use std::collections::HashMap;

/// Configuration for a sprite sheet (texture atlas)
#[derive(Debug, Clone)]
pub struct SpriteSheetConfig {
    pub texture_path: String,
    pub tile_size: (f32, f32),  // Width, height in pixels
    pub columns: u32,
    pub rows: u32,
    pub sprites: Vec<(u32, String)>,  // (index, name) mappings
}

/// Resource managing sprite materials and quad meshes for billboard rendering
#[derive(Resource)]
pub struct SpriteAssets {
    /// Cached materials per sprite sheet (texture + alpha blend settings)
    materials: HashMap<String, Handle<StandardMaterial>>,
    /// Cached quad meshes sized for each sprite sheet
    meshes: HashMap<String, Handle<Mesh>>,
    /// Sprite sheet configurations
    configs: HashMap<String, SpriteSheetConfig>,
}

impl SpriteAssets {
    pub fn new() -> Self {
        Self {
            materials: HashMap::new(),
            meshes: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    /// Get or create material for a sprite sheet
    pub fn get_or_load_material(
        &mut self,
        sheet_path: &str,
        asset_server: &AssetServer,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        if let Some(handle) = self.materials.get(sheet_path) {
            return handle.clone();
        }

        let texture_handle = asset_server.load(sheet_path);
        let material = StandardMaterial {
            base_color_texture: Some(texture_handle),
            alpha_mode: AlphaMode::Blend,
            unlit: false,  // Use lighting for depth
            perceptual_roughness: 0.9,
            ..default()
        };

        let handle = materials.add(material);
        self.materials.insert(sheet_path.to_string(), handle.clone());
        handle
    }

    /// Get or create quad mesh for a sprite
    pub fn get_or_load_mesh(
        &mut self,
        sprite_size: (f32, f32),
        meshes: &mut Assets<Mesh>,
    ) -> Handle<Mesh> {
        let key = format!("{}x{}", sprite_size.0, sprite_size.1);
        if let Some(handle) = self.meshes.get(&key) {
            return handle.clone();
        }

        let mesh = Rectangle::new(sprite_size.0, sprite_size.1);
        let handle = meshes.add(mesh);
        self.meshes.insert(key, handle.clone());
        handle
    }

    /// Calculate UV transform for sprite at index in atlas
    pub fn get_sprite_uv_transform(&self, sheet_key: &str, sprite_index: u32) -> (Vec2, Vec2) {
        if let Some(config) = self.configs.get(sheet_key) {
            let col = sprite_index % config.columns;
            let row = sprite_index / config.columns;

            let u_scale = 1.0 / config.columns as f32;
            let v_scale = 1.0 / config.rows as f32;

            let u_offset = col as f32 * u_scale;
            let v_offset = row as f32 * v_scale;

            (Vec2::new(u_offset, v_offset), Vec2::new(u_scale, v_scale))
        } else {
            (Vec2::ZERO, Vec2::ONE)
        }
    }

    pub fn register_config(&mut self, key: String, config: SpriteSheetConfig) {
        self.configs.insert(key, config);
    }

    pub fn get_config(&self, key: &str) -> Option<&SpriteSheetConfig> {
        self.configs.get(key)
    }
}
```

#### 2.3 Create Sprite Sheet Registry

**File:** `data/sprite_sheets.ron` (new file)

Define sprite sheet configurations in a registry file with entries for:

**Tile Sprites:**

- `walls` - 4x4 grid, 128x256 pixels (stone, brick, wood variants)
- `doors` - 4x2 grid, 128x256 pixels (open, closed, locked variants)
- `terrain` - 8x8 grid, 128x128 pixels (floor tiles)
- `trees` - 4x4 grid, 128x256 pixels (vegetation)
- `decorations` - 8x8 grid, 64x64 pixels (small objects)

**Actor Sprites (NEW):**

- `npcs_town` - 4x4 grid, 32x48 pixels (guard, merchant, innkeeper, blacksmith, etc.)
- `monsters_basic` - 4x4 grid, 32x48 pixels (goblin, orc, skeleton, wolf, etc.)
- `monsters_advanced` - 4x4 grid, 32x48 pixels (dragon, lich, demon, etc.)
- `recruitables` - 4x2 grid, 32x48 pixels (warrior_recruit, mage_recruit, etc.)

**Event Marker Sprites:**

- `signs` - 4x2 grid, 32x64 pixels (wooden_sign, stone_marker, warning_sign, info_sign, quest_marker, shop_sign, danger_sign, direction_sign)
- `portals` - 4x2 grid, 128x128 pixels (teleport_pad, dimensional_gate, stairs_up, stairs_down, portal_blue, portal_red, trap_door, exit_portal)

Example entries for `data/sprite_sheets.ron`:

```ron
// Actor Sprite Sheets (NEW)
"npcs_town": SpriteSheetConfig(
    texture_path: "textures/sprites/npcs_town.png",
    tile_size: (32.0, 48.0),
    columns: 4,
    rows: 4,
    sprites: [
        (0, "guard"),
        (1, "merchant"),
        (2, "innkeeper"),
        (3, "blacksmith"),
        (4, "priest"),
        (5, "noble"),
        (6, "peasant"),
        (7, "child"),
        (8, "elder"),
        (9, "mage_npc"),
        (10, "warrior_npc"),
        (11, "rogue_npc"),
        (12, "captain"),
        (13, "mayor"),
        (14, "servant"),
        (15, "beggar"),
    ],
),

"monsters_basic": SpriteSheetConfig(
    texture_path: "textures/sprites/monsters_basic.png",
    tile_size: (32.0, 48.0),
    columns: 4,
    rows: 4,
    sprites: [
        (0, "goblin"),
        (1, "orc"),
        (2, "skeleton"),
        (3, "zombie"),
        (4, "wolf"),
        (5, "bear"),
        (6, "spider"),
        (7, "bat"),
        (8, "rat"),
        (9, "snake"),
        (10, "slime"),
        (11, "imp"),
        (12, "bandit"),
        (13, "thug"),
        (14, "cultist"),
        (15, "ghoul"),
    ],
),

"monsters_advanced": SpriteSheetConfig(
    texture_path: "textures/sprites/monsters_advanced.png",
    tile_size: (32.0, 48.0),
    columns: 4,
    rows: 4,
    sprites: [
        (0, "dragon"),
        (1, "lich"),
        (2, "demon"),
        (3, "vampire"),
        (4, "beholder"),
        (5, "minotaur"),
        (6, "troll"),
        (7, "ogre"),
        (8, "wraith"),
        (9, "elemental"),
        (10, "golem"),
        (11, "hydra"),
        (12, "wyvern"),
        (13, "chimera"),
        (14, "basilisk"),
        (15, "manticore"),
    ],
),

"recruitables": SpriteSheetConfig(
    texture_path: "textures/sprites/recruitable_characters.png",
    tile_size: (32.0, 48.0),
    columns: 4,
    rows: 2,
    sprites: [
        (0, "warrior_recruit"),
        (1, "mage_recruit"),
        (2, "rogue_recruit"),
        (3, "cleric_recruit"),
        (4, "ranger_recruit"),
        (5, "paladin_recruit"),
        (6, "bard_recruit"),
        (7, "monk_recruit"),
    ],
),

// Event Marker Sprites
"signs": SpriteSheetConfig(
    texture_path: "textures/sprites/signs.png",
    tile_size: (32.0, 32.0),
    columns: 4,
    rows: 2,
    sprites: [
        (0, "wooden_sign"),
        (1, "stone_marker"),
        (2, "warning_sign"),
        (3, "info_sign"),
        (4, "quest_marker"),
        (5, "shop_sign"),
        (6, "danger_sign"),
        (7, "direction_sign"),
    ],
),

"portals": SpriteSheetConfig(
    texture_path: "textures/sprites/portals.png",
    tile_size: (32.0, 32.0),
    columns: 4,
    rows: 2,
    sprites: [
        (0, "teleport_pad"),
        (1, "dimensional_gate"),
        (2, "stairs_up"),
        (3, "stairs_down"),
        (4, "portal_blue"),
        (5, "portal_red"),
        (6, "trap_door"),
        (7, "exit_portal"),
    ],
),
```

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

**Tile Sprites:**

- `assets/sprites/walls.png` - 4x4 grid (512x1024)
- `assets/sprites/doors.png` - 4x2 grid (512x512)
- `assets/sprites/terrain.png` - 8x8 grid (1024x1024)
- `assets/sprites/trees.png` - 4x4 grid (512x1024)
- `assets/sprites/decorations.png` - 8x8 grid (512x512)

**Actor Sprites (NEW):**

- `assets/sprites/npcs_town.png` - 4x4 grid (128x192)
- `assets/sprites/monsters_basic.png` - 4x4 grid (128x192)
- `assets/sprites/monsters_advanced.png` - 4x4 grid (128x192)
- `assets/sprites/recruitables.png` - 4x2 grid (128x96)

**Event Marker Sprites:**

- `assets/sprites/signs.png` - 4x2 grid (128x64)
- `assets/sprites/portals.png` - 4x2 grid (128x64)

#### 2.6 Testing Requirements

**Unit Tests:**

- `test_sprite_sheet_config_default()` - default config has 4x4 grid, 64x64 tiles
- `test_sprite_assets_get_or_load_material()` - material loading caches correctly
- `test_sprite_assets_get_or_load_mesh()` - mesh creation caches by size
- `test_sprite_assets_uv_transform()` - UV calculation correct for atlas indices
- `test_sprite_sheet_registry_load()` - registry loads from RON successfully
- `test_sprite_sheet_registry_get()` - `get()` finds sheet by ID
- `test_sprite_count()` - `sprite_count()` returns columns × rows
- `test_actor_sprite_sheets_registered()` - NPCs, monsters, recruitables sheets exist

#### 2.7 Deliverables

- [ ] `SpriteAssets` resource created with native PBR approach
- [ ] `get_or_load_material()`, `get_or_load_mesh()`, `get_sprite_uv_transform()` methods implemented
- [ ] `SpriteSheetRegistry` struct created with `load_from_file()`, `get()`, `sprite_count()`
- [ ] `data/sprite_sheets.ron` registry file created with actor sprite sheets
- [ ] `assets/sprites/` directory created with structure documented
- [ ] Actor sprite sheet placeholders created (npcs_town, monsters_basic, monsters_advanced, recruitables)
- [ ] Unit tests written and passing (minimum 8 tests)

#### 2.8 Success Criteria

- ✅ No external dependencies required (native Bevy only)
- ✅ Sprite sheet registry loads with actor sprite configurations
- ✅ `SpriteAssets` creates PBR materials with alpha blend
- ✅ UV transforms correctly map sprite atlas indices
- ✅ All quality gates pass

---

### Phase 3: Sprite Rendering Integration

**Goal:** Update map rendering system to render sprites for tiles with sprite metadata **and implement billboard system for all actor entities**.

#### 3.1 Implement Billboard Component and System

**File:** `src/game/components/billboard.rs` (new file)

Create billboard component that makes entities face the camera:

```rust
use bevy::prelude::*;

/// Component that makes an entity face the camera (billboard effect)
#[derive(Component)]
pub struct Billboard {
    /// Lock Y-axis rotation (true for characters standing upright)
    pub lock_y: bool,
}

impl Default for Billboard {
    fn default() -> Self {
        Self { lock_y: true }
    }
}
```

**File:** `src/game/systems/billboard.rs` (new file)

Create system that updates billboard entities to face camera:

```rust
use bevy::prelude::*;
use crate::game::components::billboard::Billboard;

/// System that updates billboard entities to face the camera
pub fn update_billboards(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut billboard_query: Query<(&mut Transform, &GlobalTransform, &Billboard)>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    let camera_pos = camera_transform.translation();

    for (mut transform, global_transform, billboard) in billboard_query.iter_mut() {
        let entity_pos = global_transform.translation();
        let direction = camera_pos - entity_pos;

        if billboard.lock_y {
            // Y-axis locked: Only rotate around Y to face camera (characters stay upright)
            let angle = direction.x.atan2(direction.z);
            transform.rotation = Quat::from_rotation_y(angle + std::f32::consts::PI);
        } else {
            // Full rotation: Billboard always faces camera (particles, effects)
            transform.look_at(camera_pos, Vec3::Y);
        }
    }
}
```

#### 3.2 Create Sprite Rendering Components

**File:** `src/game/components/sprite.rs` (new file)

Add new components for tile and actor sprites:

```rust
use bevy::prelude::*;

/// Component for tile-based sprites (walls, floors, decorations)
#[derive(Component)]
pub struct TileSprite {
    pub sheet_path: String,
    pub sprite_index: u32,
}

/// Component for actor sprites (NPCs, Monsters, Recruitables)
#[derive(Component)]
pub struct ActorSprite {
    pub sheet_path: String,
    pub sprite_index: u32,
    pub actor_type: ActorType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorType {
    Npc,
    Monster,
    Recruitable,
}

/// Component for animated sprites
#[derive(Component)]
pub struct AnimatedSprite {
    pub frames: Vec<u32>,
    pub fps: f32,
    pub looping: bool,
    pub current_frame: usize,
    pub timer: f32,
}
```

#### 3.3 Implement Sprite Spawning Functions

**File:** `src/game/systems/map.rs`

Add constants and spawn functions:

```rust
// Scaling constant: 128 pixels = 1 world unit
const PIXELS_PER_METER: f32 = 128.0;

/// Spawn a tile sprite (for walls, floors, decorations)
fn spawn_sprite_tile(
    commands: &mut Commands,
    sprite_assets: &mut SpriteAssets,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    position: Position,
    sheet_path: &str,
    sprite_index: u32,
    map_id: MapId,
) -> Entity {
    let config = sprite_assets.get_config(sheet_path).cloned().unwrap();
    let sprite_size = (
        config.tile_size.0 / PIXELS_PER_METER,
        config.tile_size.1 / PIXELS_PER_METER,
    );

    let material_handle = sprite_assets.get_or_load_material(
        &config.texture_path,
        asset_server,
        materials,
    );

    let mesh_handle = sprite_assets.get_or_load_mesh(sprite_size, meshes);

    commands
        .spawn(PbrBundle {
            mesh: mesh_handle,
            material: material_handle,
            transform: Transform::from_xyz(
                position.x as f32,
                sprite_size.1 / 2.0,  // Center vertically
                position.y as f32,
            ),
            ..default()
        })
        .insert(TileSprite {
            sheet_path: sheet_path.to_string(),
            sprite_index,
        })
        .insert(Billboard { lock_y: false })
        .insert(MapEntity { map_id })
        .id()
}

/// Spawn an actor sprite (NPCs, Monsters, Recruitables)
fn spawn_actor_sprite(
    commands: &mut Commands,
    sprite_assets: &mut SpriteAssets,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    position: Position,
    sheet_path: &str,
    sprite_index: u32,
    actor_type: ActorType,
    map_id: MapId,
) -> Entity {
    let config = sprite_assets.get_config(sheet_path).cloned().unwrap();
    let sprite_size = (
        config.tile_size.0 / PIXELS_PER_METER,
        config.tile_size.1 / PIXELS_PER_METER,
    );

    let material_handle = sprite_assets.get_or_load_material(
        &config.texture_path,
        asset_server,
        materials,
    );

    let mesh_handle = sprite_assets.get_or_load_mesh(sprite_size, meshes);

    commands
        .spawn(PbrBundle {
            mesh: mesh_handle,
            material: material_handle,
            transform: Transform::from_xyz(
                position.x as f32,
                sprite_size.1 / 2.0,  // Bottom-centered (feet on ground)
                position.y as f32,
            ),
            ..default()
        })
        .insert(ActorSprite {
            sheet_path: sheet_path.to_string(),
            sprite_index,
            actor_type,
        })
        .insert(Billboard { lock_y: true })  // Keep upright
        .insert(MapEntity { map_id })
        .id()
}
```

#### 3.4 Modify spawn_map for Hybrid Rendering

**File:** `src/game/systems/map.rs`

Update `spawn_map()` to:

1. Check `tile.visual.uses_sprite()` for each tile
2. If true: call `spawn_sprite_tile()` to create PBR billboard entity
3. If false: call existing mesh spawning code

#### 3.5 Update NPC/Monster Spawning

**File:** `src/game/systems/map.rs`

Replace cuboid placeholders with sprite billboards:

```rust
// NPC spawning (replace cuboid with sprite)
for resolved_npc in resolved_npcs.iter() {
    spawn_actor_sprite(
        &mut commands,
        &mut sprite_assets,
        &mut meshes,
        &mut materials,
        &asset_server,
        resolved_npc.position,
        "npcs_town",
        determine_npc_sprite_index(&resolved_npc),
        ActorType::Npc,
        map_id,
    );
}

// Monster spawning
for monster in monsters.iter() {
    spawn_actor_sprite(
        &mut commands,
        &mut sprite_assets,
        &mut meshes,
        &mut materials,
        &asset_server,
        monster.position,
        determine_monster_sheet(&monster.monster_type),
        determine_monster_sprite_index(&monster.monster_type),
        ActorType::Monster,
        map_id,
    );
}

// Recruitable spawning
for (position, event) in map.events.iter() {
    if let MapEvent::RecruitableCharacter { name, .. } = event {
        spawn_actor_sprite(
            &mut commands,
            &mut sprite_assets,
            &mut meshes,
            &mut materials,
            &asset_server,
            *position,
            "recruitables",
            hash_name_to_sprite_index(name, 8),
            ActorType::Recruitable,
            map_id,
        );
    }
}
```

#### 3.6 Add Sprite Animation System

**File:** `src/game/systems/sprite.rs` (new file)

Create `animate_sprites` system:

```rust
use bevy::prelude::*;
use crate::game::components::sprite::AnimatedSprite;

pub fn animate_sprites(
    time: Res<Time>,
    mut query: Query<(&mut AnimatedSprite, &mut Handle<StandardMaterial>)>,
) {
    for (mut animated, _material) in query.iter_mut() {
        animated.timer += time.delta_secs();

        if animated.timer >= 1.0 / animated.fps {
            animated.timer = 0.0;
            animated.current_frame += 1;

            if animated.current_frame >= animated.frames.len() {
                if animated.looping {
                    animated.current_frame = 0;
                } else {
                    animated.current_frame = animated.frames.len() - 1;
                }
            }

            // Update UV transform for new frame
            // (implementation depends on material UV manipulation)
        }
    }
}
```

#### 3.7 Testing Requirements

**Integration Tests:**

- `test_billboard_component_created()` - Billboard component has correct default
- `test_update_billboards_system()` - Billboard system rotates entities to face camera
- `test_billboard_lock_y_preserves_upright()` - Y-locked billboards stay upright
- `test_sprite_tile_spawns_pbr_bundle()` - Tile sprite creates PbrBundle with Rectangle mesh
- `test_actor_sprite_spawns_with_billboard()` - Actor sprite has Billboard component
- `test_npc_sprite_replaces_cuboid()` - NPCs render as sprites, not cuboids
- `test_monster_sprite_rendering()` - Monsters use sprite sheets
- `test_recruitable_sprite_rendering()` - Recruitables render as billboard sprites
- `test_sprite_tile_spawns_for_metadata()` - Tile with sprite metadata renders sprite
- `test_mesh_tile_spawns_mesh()` - Tile without sprite renders as mesh
- `test_sprite_scale_applied()` - Sprite scale matches tile.visual.scale
- `test_sprite_bottom_centered()` - Actor sprites positioned with feet on ground
- `test_animated_sprite_updates()` - Animated sprite changes frames over time
- `test_animation_loops_correctly()` - Looping animation resets to frame 0

#### 3.8 Deliverables

- [ ] `Billboard` component and `update_billboards` system implemented
- [ ] `TileSprite` and `ActorSprite` components created
- [ ] `spawn_sprite_tile()` function implemented with native PBR
- [ ] `spawn_actor_sprite()` function implemented for NPCs/Monsters/Recruitables
- [ ] NPC/Monster spawning updated to use sprites (not cuboids)
- [ ] Recruitable spawning uses sprite system
- [ ] `AnimatedSprite` component and `animate_sprites()` system created
- [ ] Hybrid rendering (mesh + sprite) works correctly
- [ ] Integration tests written and passing (minimum 14 tests)

#### 3.9 Success Criteria

- ✅ All actors (NPCs, Monsters, Recruitables) render as billboard sprites
- ✅ Character sprites properly centered at bottom (feet on ground)
- ✅ Billboard system keeps characters upright (Y-axis locked)
- ✅ Tiles with sprite metadata render as billboards facing camera
- ✅ Tiles without sprite metadata render as 3D meshes (unchanged behavior)
- ✅ Sprite scale and positioning use correct values
- ✅ Animated sprites cycle through frames correctly
- ✅ Billboard update system optimized for 100+ actor sprites
- ✅ Performance acceptable with mixed rendering (sprites + meshes)

---

### Phase 3.X: Replace Event Placeholder Markers with Sprites

**Goal:** Replace the colored quad placeholder markers for signs and teleports with sprite-based rendering.

**Current State:** Phase 2 of E-Key interaction system added placeholder colored quads for visual representation:

- Signs: brown/tan colored plane (RGB 0.59, 0.44, 0.27)
- Teleports: purple colored plane (RGB 0.53, 0.29, 0.87)

**Replacement Tasks:**

**File:** `src/game/systems/map.rs`

The placeholder marker spawning code in `spawn_map()` function should be replaced as follows:

1. **Query sprite registry** for "signs" and "portals" sprite sheets
2. **For each MapEvent (Sign/Teleport):**
   - Determine marker type (Sign or Teleport)
   - Query appropriate sprite sheet from registry
   - Get event-specific metadata to select sprite index
   - Spawn `Sprite3d` entity instead of colored quad plane
3. **Remove colored quad spawning code** (the section spawning SIGN_MARKER_COLOR and TELEPORT_MARKER_COLOR planes)
4. **Add sprite index selection** based on event metadata:
   - For Signs: use event name or description to select from 0-7
   - For Teleports: use destination map/position to select from 0-7
   - Default to index 0 if metadata insufficient

**Implementation Details:**

Replace this section in `spawn_map()`:

```rust
// OLD: Spawn event markers for signs and teleports
for (position, event) in map.events.iter() {
    let marker_color = match event {
        world::MapEvent::Sign { .. } => SIGN_MARKER_COLOR,
        world::MapEvent::Teleport { .. } => TELEPORT_MARKER_COLOR,
        _ => continue,
    };
    // ... colored quad spawning code ...
}
```

With sprite-based rendering:

```rust
// NEW: Spawn sprite markers for signs and teleports
for (position, event) in map.events.iter() {
    let (sheet_key, sprite_index) = match event {
        world::MapEvent::Sign { name, .. } => {
            let idx = hash_name_to_index(name, 8); // 8 sign sprites
            ("signs", idx)
        },
        world::MapEvent::Teleport { destination, .. } => {
            let idx = hash_position_to_index(destination, 8); // 8 portal sprites
            ("portals", idx)
        },
        _ => continue,
    };

    // Get sprite sheet config from registry
    if let Some(sheet_config) = sprite_registry.get(sheet_key) {
        // Spawn Sprite3d entity with appropriate texture atlas
        // Position at tile coordinate with EVENT_MARKER_Y_OFFSET
    }
}
```

**Testing Requirements:**

- `test_sign_sprites_replace_colored_quads()` - verify signs use sprite rendering
- `test_teleport_sprites_replace_colored_quads()` - verify teleports use sprite rendering
- `test_sprite_marker_sprite_index_selection()` - verify correct sprite selected based on event
- `test_sprite_marker_positioning()` - verify sprite position matches colored quad position
- `test_sprite_registry_lookup()` - verify registry correctly provides sprite sheets

**Deliverables:**

- [ ] Colored quad spawning code removed from `spawn_map()`
- [ ] Sprite-based marker spawning implemented
- [ ] Hash functions for sprite index selection created
- [ ] Sprite registry integration added
- [ ] All marker sprites render correctly in-game
- [ ] Tests confirm sprite rendering replaces colored quads

**Success Criteria:**

- ✅ Sign events display sprite from "signs" sheet instead of brown quad
- ✅ Teleport events display sprite from "portals" sheet instead of purple quad
- ✅ Sprite selection uses event metadata for visual variety
- ✅ Sprite positioning and scaling match original placeholder markers
- ✅ Performance equivalent or better than colored quad rendering
- ✅ No visual regression from current placeholder system

---

### Phase 4: Sprite Asset Creation Guide

**Goal:** Document sprite creation workflow and provide starter assets for tiles **and character sprites**.

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

**Tile Sprites:**

| File          | Grid | Sprite Size | Sheet Size | Content                           |
| ------------- | ---- | ----------- | ---------- | --------------------------------- |
| `walls.png`   | 4x4  | 128x256     | 512x1024   | Stone, brick, wood, damaged walls |
| `doors.png`   | 4x2  | 128x256     | 512x512    | Wooden/iron doors (open/closed)   |
| `terrain.png` | 8x8  | 128x128     | 1024x1024  | Stone, grass, dirt, water floors  |
| `trees.png`   | 4x4  | 128x256     | 512x1024   | Deciduous, conifer, dead, magical |

**Character Sprites (32x48 pixels, facing forward, bottom-centered anchor):**

| File                    | Grid | Sprite Size | Sheet Size | Content                           |
| ----------------------- | ---- | ----------- | ---------- | --------------------------------- |
| `npcs_town.png`         | 4x4  | 32x48       | 128x192    | Guard, merchant, innkeeper, etc.  |
| `monsters_basic.png`    | 4x4  | 32x48       | 128x192    | Goblin, orc, skeleton, wolf, etc. |
| `monsters_advanced.png` | 4x4  | 32x48       | 128x192    | Dragon, lich, demon, etc.         |
| `recruitables.png`      | 4x2  | 32x48       | 128x96     | Recruitable character classes     |

#### 4.2 Testing Requirements

**Validation Tests:**

- `test_starter_sprite_sheets_exist()` - all documented sheets exist in assets/sprites/
- `test_sprite_registry_matches_files()` - registry entries match actual files
- `test_sprite_dimensions_valid()` - sprites match documented dimensions

#### 4.3 Deliverables

- [ ] `docs/tutorials/creating_sprites.md` guide created
- [ ] `assets/sprites/walls.png` hand-crafted (512x1024, 4x4 grid)
- [ ] `assets/sprites/doors.png` hand-crafted (512x512, 4x2 grid)
- [ ] `assets/sprites/terrain.png` hand-crafted (1024x1024, 8x8 grid)
- [ ] `assets/sprites/trees.png` hand-crafted (512x1024, 4x4 grid)
- [ ] `assets/sprites/npcs_town.png` hand-crafted (128x192, 4x4 grid, 32x48 sprites)
- [ ] `assets/sprites/monsters_basic.png` hand-crafted (128x192, 4x4 grid, 32x48 sprites)
- [ ] `assets/sprites/monsters_advanced.png` hand-crafted (128x192, 4x4 grid, 32x48 sprites)
- [ ] `assets/sprites/recruitables.png` hand-crafted (128x96, 4x2 grid, 32x48 sprites)
- [ ] Registry in `data/sprite_sheets.ron` matches actual assets

#### 4.4 Success Criteria

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
- ✅ All actors (NPCs, Monsters, Recruitables) render as billboard sprites
- ✅ Character sprites properly centered at bottom (feet on ground)
- ✅ Billboard system keeps characters upright (Y-axis locked)
- ✅ Sprites render as billboards facing the camera
- ✅ Sprite animations cycle through frames correctly
- ✅ Sprite scale and offset use TileVisualMetadata values
- ✅ Sprite sheets load and cache efficiently
- ✅ Billboard update system optimized for 100+ actor sprites
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

| Crate  | Version | Purpose                |
| ------ | ------- | ---------------------- |
| `bevy` | 0.17    | Core engine (existing) |

**No external sprite dependencies required** - using native `bevy::pbr` and `bevy::render` modules.

### Internal Dependencies

| Component | Dependency                                      |
| --------- | ----------------------------------------------- |
| Phase 1   | Tile Visual Metadata Plan (Phases 1-2 complete) |
| Phase 2   | Phase 1 complete                                |
| Phase 3   | Phase 2 complete                                |
| Phase 4   | Phase 3 complete                                |
| Phase 5   | Phase 4 complete                                |

### Implementation Order Alignment

Per `plan_updates_review.md` Section 3:

1. **Execute `sprite_support_implementation_plan.md` FIRST**
2. Then execute `procedural_meshes_implementation_plan.md` (trees, signs, portals only)

This ensures character rendering is unified under the sprite system before implementing environmental procedural meshes.

## Risks and Mitigations

**Risk**: Native billboard system performance with 100+ actor sprites

- **Mitigation**: Billboard update system runs only on entities with `Billboard` component. Early profiling shows acceptable performance. If issues arise, implement spatial partitioning or update only visible billboards.

**Risk**: Sprite rendering performance degrades with many tiles and actors

- **Mitigation**: PBR material batching and mesh instancing should handle 1000+ sprites. If issues arise, implement frustum culling for off-screen sprites.

**Risk**: UV transform calculations for texture atlas incorrect

- **Mitigation**: Comprehensive unit tests for `get_sprite_uv_transform()`. Validate with visual tests using known sprite sheets.

**Risk**: Existing maps break with schema changes

- **Mitigation**: `#[serde(default)]` on sprite field ensures old RON files load correctly.

## Timeline Estimate

- **Phase 1** (Sprite Metadata): 3-4 hours
- **Phase 2** (Asset Infrastructure): 5-6 hours (increased: native PBR implementation + actor sprites)
- **Phase 3** (Rendering Integration): 8-10 hours (increased: billboard system + actor sprite integration)
- **Phase 4** (Asset Creation Guide): 3-4 hours
- **Phase 5** (SDK Integration): 5-7 hours
- **Phase 6** (Advanced Features): 4-8 hours (optional)

**Total (Phases 1-5)**: 25-32 hours (increased due to native PBR implementation and actor sprite integration)
**Total (All Phases)**: 29-40 hours

**Recommended Approach**: Implement Phases 1-3 first to establish working sprite rendering. Phase 4 (documentation + starter assets) can be done in parallel with Phase 3. Phase 5 (SDK) follows once core rendering is stable. Phase 6 is optional based on user feedback.
