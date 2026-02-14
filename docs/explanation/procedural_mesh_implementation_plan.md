# Procedural Mesh System Implementation Plan

## Overview

This plan outlines the phased implementation of user-configurable procedural meshes for both the Antares game engine and Campaign Builder SDK. The system will allow content creators to define custom creatures, NPCs, and objects using RON-based mesh definitions that can be visually edited, validated, and exported as part of campaigns.

**Key Features:**

- User-defined procedural meshes via RON configuration files
- Visual mesh editor integrated into Campaign Builder
- Template-based mesh generation (primitives: cube, sphere, cylinder, pyramid)
- Multi-mesh creatures with hierarchical composition
- Automatic normal calculation and mesh validation
- Integration with existing Bevy ECS rendering pipeline
- Campaign packaging and distribution support

## Current State Analysis

### Existing Infrastructure

**Game Engine (Bevy-based):**

- `src/game/systems/procedural_meshes.rs` - Existing procedural mesh system for environmental objects (trees, furniture, structures)
- `src/game/components/` - Bevy ECS component definitions
- `src/game/systems/actor.rs` - Actor rendering system
- Bevy 0.15+ rendering pipeline with mesh caching

**Campaign Builder SDK:**

- `sdk/campaign_builder/src/` - egui-based content editor
- Multiple specialized editors: `monsters_editor.rs`, `npc_editor.rs`, `characters_editor.rs`
- `sdk/campaign_builder/src/lib.rs` - Main application with tab-based UI
- Campaign packaging and validation infrastructure

**Domain Layer:**

- `src/domain/combat/database.rs` - MonsterDefinition structures
- `src/domain/character_definition.rs` - CharacterDefinition structures
- `src/domain/types.rs` - Type aliases (ItemId, SpellId, MonsterId, etc.)
- RON-based serialization for all content types

**SDK Infrastructure:**

- `src/sdk/campaign_loader.rs` - Campaign content loading
- `src/sdk/database.rs` - ContentDatabase for game content
- `src/sdk/validation.rs` - Content validation framework
- `src/sdk/campaign_packager.rs` - Campaign export/packaging

### Identified Issues

1. **No unified creature visual system** - Monsters and NPCs lack visual mesh definitions
2. **Visual content creation gap** - No tool for creating custom creature appearances
3. **Missing creature catalog** - No database of visual creature definitions
4. **No mesh validation** - No system to validate procedural mesh integrity
5. **Duplicate mesh generation** - Environmental mesh system not reusable for creatures
6. **Limited NPC customization** - NPCs can't have custom appearances beyond basic sprites

## Implementation Phases

### Phase 1: Core Domain Integration

#### 1.1 Foundation Work

**Add core mesh types to domain layer:**

Create `src/domain/visual/mod.rs`:

- `MeshDefinition` struct (vertices, normals, uvs, indices, color)
- `CreatureDefinition` struct (name, meshes, scale, visual properties)
- `MeshTransform` struct (translation, rotation, scale per mesh)
- Mesh validation functions (`validate_vertices`, `validate_indices`, `validate_normals`)

Create `src/domain/types.rs` additions:

- Add `CreatureId` type alias (newtype pattern around `u32`)
- Add `MeshId` type alias for internal mesh references

Create `src/domain/visual/mesh_validation.rs`:

- `fn validate_mesh_definition(mesh: &MeshDefinition) -> Result<(), ValidationError>`
- Check vertex count > 0
- Check indices are valid (within vertex bounds)
- Check triangle count (indices % 3 == 0)
- Validate normal count matches vertex count (if provided)
- Validate UV count matches vertex count (if provided)
- Check for degenerate triangles

#### 1.2 Add Creature Database

Create `src/domain/visual/creature_database.rs`:

- `CreatureDatabase` struct with HashMap storage
- `fn add_creature(creature: CreatureDefinition) -> Result<CreatureId, DatabaseError>`
- `fn get_creature(id: CreatureId) -> Option<&CreatureDefinition>`
- `fn all_creatures() -> impl Iterator<Item = &CreatureDefinition>`
- `fn remove_creature(id: CreatureId) -> Option<CreatureDefinition>`
- `fn load_from_ron(path: &Path) -> Result<Vec<CreatureDefinition>, LoadError>`
- Support for creature variants (color swaps, scale changes)

#### 1.3 Integrate with ContentDatabase

Update `src/sdk/database.rs`:

- Add `pub creatures: CreatureDatabase` field to `ContentDatabase`
- Update `ContentDatabase::new()` to initialize creature database
- Update `ContentDatabase::load_campaign()` to load `creatures/` directory
- Add creature validation to `ContentDatabase::validate()`

Update `src/sdk/campaign_loader.rs`:

- Add `creatures/` directory scanning to `load_campaign()`
- Parse all `.ron` files as `CreatureDefinition`
- Validate creatures during campaign load
- Add to `Campaign` struct: `pub creature_files: Vec<String>`

#### 1.4 Link MonsterDefinition to Visuals

Update `src/domain/combat/database.rs`:

- Add `pub visual_id: Option<CreatureId>` field to `MonsterDefinition`
- Update `MonsterDefinition::to_monster()` to copy visual_id
- Add Serde `#[serde(default)]` attribute for backwards compatibility

Update `src/domain/combat/monster.rs`:

- Add `pub visual_id: Option<CreatureId>` field to `Monster` struct
- Initialize visual_id in `Monster::new()` to None
- Add setter method: `pub fn set_visual(&mut self, visual_id: CreatureId)`

Add to monster RON files (example):

```ron
MonsterDefinition(
    id: 1,
    name: "Red Dragon",
    visual_id: Some(42), // References dragon_base creature
    // ... rest of stats
)
```

#### 1.5 Testing Requirements

**Unit Tests (`src/domain/visual/tests.rs`):**

- `test_mesh_definition_serialization` - Round-trip RON serialization
- `test_validate_mesh_valid_cube` - Valid cube mesh passes validation
- `test_validate_mesh_invalid_indices` - Invalid indices rejected
- `test_validate_mesh_empty_vertices` - Empty vertices rejected
- `test_creature_database_add_retrieve` - Add/retrieve creatures
- `test_creature_database_duplicate_id` - Reject duplicate IDs
- `test_mesh_normal_auto_calculation` - Auto-calculate flat normals
- `test_creature_definition_multi_mesh` - Multi-mesh creatures work

**Integration Tests (`tests/creature_loading.rs`):**

- `test_load_creature_from_ron_file` - Load example creature
- `test_load_campaign_with_creatures` - Campaign loader finds creatures
- `test_invalid_creature_file_error` - Graceful error handling
- `test_creature_database_in_content_db` - Database integration

**Monster-Visual Linking Tests (`src/domain/combat/tests.rs`):**

- `test_monster_definition_with_visual_id` - Visual ID serializes correctly
- `test_monster_definition_without_visual_id` - None is default (backwards compatible)
- `test_monster_set_visual` - Can update visual after creation
- `test_monster_to_monster_copies_visual_id` - Conversion preserves visual_id

#### 1.6 Deliverables

- [ ] `src/domain/visual/mod.rs` with core types
- [ ] `src/domain/visual/mesh_validation.rs` with validation logic
- [ ] `src/domain/visual/creature_database.rs` with storage/retrieval
- [ ] Updated `src/domain/types.rs` with CreatureId type alias
- [ ] Updated `src/sdk/database.rs` with creature database field
- [ ] Updated `src/sdk/campaign_loader.rs` with creature loading
- [ ] Updated `src/domain/combat/database.rs` with visual_id field
- [ ] Updated `src/domain/combat/monster.rs` with visual_id support
- [ ] Unit tests with >80% coverage
- [ ] Integration tests for campaign loading
- [ ] Monster-visual linking tests
- [ ] Documentation in `docs/explanation/implementations.md`

#### 1.7 Success Criteria

- `cargo check --all-targets --all-features` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo nextest run --all-features` passes (100% tests pass)
- RON creature files load without errors
- Creature validation catches invalid meshes
- ContentDatabase contains creatures after campaign load
- MonsterDefinition RON files can include optional visual_id field
- Existing monster RON files without visual_id still load (backwards compatible)

---

### Phase 2: Game Engine Rendering

#### 2.1 Bevy ECS Components

Create `src/game/components/creature.rs`:

- `#[derive(Component)] CreatureVisual` - Links entity to CreatureDefinition
- `#[derive(Component)] MeshPart` - Represents one mesh in multi-mesh creature
- `#[derive(Component)] CreatureAnimationState` - Future: animation playback
- Fields: `creature_id: CreatureId`, `mesh_index: usize`, `material_override: Option<Handle<StandardMaterial>>`

Update `src/game/components/mod.rs`:

- Add `pub mod creature;`
- Re-export creature components

#### 2.2 Mesh Generation System

Create `src/game/systems/creature_meshes.rs`:

- `fn mesh_definition_to_bevy(mesh: &MeshDefinition) -> Mesh`
- Converts domain `MeshDefinition` to `bevy::render::mesh::Mesh`
- Insert vertices as `Mesh::ATTRIBUTE_POSITION`
- Insert normals as `Mesh::ATTRIBUTE_NORMAL` (auto-calculate if missing)
- Insert UVs as `Mesh::ATTRIBUTE_UV_0` (if provided)
- Insert indices as `Indices::U32`

Create helper functions:

- `fn calculate_flat_normals(vertices: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]>`
- `fn calculate_smooth_normals(vertices: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]>`
- `fn create_material_from_color(color: [f32; 4]) -> StandardMaterial`

#### 2.3 Creature Spawning System

Create `src/game/systems/creature_spawning.rs`:

- `pub fn spawn_creature(commands: &mut Commands, creature_def: &CreatureDefinition, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, position: Vec3) -> Entity`
- Create parent entity with `CreatureVisual` component
- Create child entity for each mesh in `creature_def.meshes`
- Apply `Transform` with scale from `creature_def.scale`
- Apply color from `mesh.color` or default material
- Return parent entity ID

Add Bevy system:

- `fn creature_spawning_system(mut commands: Commands, query: Query<(Entity, &SpawnCreatureRequest)>, creatures: Res<GameContent>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>)`
- Reads `SpawnCreatureRequest` component (creature_id, position)
- Spawns creature using `spawn_creature` function
- Removes `SpawnCreatureRequest` component after spawning

#### 2.4 Mesh Caching Integration

Update `src/game/systems/procedural_meshes.rs`:

- Add `creature_meshes: HashMap<(CreatureId, usize), Handle<Mesh>>` to `ProceduralMeshCache`
- Add `fn get_or_create_creature_mesh(&mut self, creature_id: CreatureId, mesh_index: usize, mesh_def: &MeshDefinition, meshes: &mut Assets<Mesh>) -> Handle<Mesh>`
- Cache creature meshes to avoid regeneration
- Add `fn clear_creature_cache(&mut self)` for hot-reloading

#### 2.5 Monster-Visual Spawning Integration

Create `src/game/systems/monster_rendering.rs`:

- `fn spawn_monster_with_visual(commands: &mut Commands, monster: &Monster, creature_db: &CreatureDatabase, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, position: Vec3) -> Entity`
- Check if `monster.visual_id.is_some()`
- If visual_id present, look up `CreatureDefinition` from database
- Spawn creature visual using `spawn_creature()` function
- Attach `MonsterMarker` component to link visual to combat entity
- If no visual_id, fallback to default representation (billboard/sprite)

Add Bevy system integration:

- `fn monster_visual_update_system(query: Query<(&Monster, &Transform, &Children), Changed<Monster>>, creatures: Res<GameContent>)`
- Update creature visuals when monster state changes
- Apply damage effects, condition indicators
- Handle monster death (fade out, remove visual)

#### 2.6 Testing Requirements

**Unit Tests (`src/game/systems/tests/creature_tests.rs`):**

- `test_mesh_definition_to_bevy_vertices` - Vertices converted correctly
- `test_mesh_definition_to_bevy_normals_auto` - Auto-calculated normals
- `test_mesh_definition_to_bevy_color` - Material color applied
- `test_calculate_flat_normals_cube` - Flat normal calculation
- `test_spawn_creature_creates_hierarchy` - Parent/child entities created
- `test_spawn_creature_scale_applied` - Scale transform applied
- `test_creature_mesh_cache_prevents_duplication` - Cache works

**Monster-Visual Integration Tests (`src/game/systems/tests/monster_rendering_tests.rs`):**

- `test_spawn_monster_with_visual_id` - Monster with visual_id spawns creature
- `test_spawn_monster_without_visual_id` - Monster without visual_id uses fallback
- `test_monster_visual_lookup_invalid_id` - Invalid visual_id handled gracefully
- `test_multiple_monsters_share_visual` - Multiple monsters can use same CreatureDefinition

**Visual Tests (manual with test campaign):**

- Spawn simple cube creature - renders correctly
- Spawn multi-mesh creature (robot) - all parts visible
- Spawn creature with custom colors - colors match definition
- Spawn scaled creature - scale applied correctly
- Spawn 100 identical creatures - performance acceptable
- Spawn monster with visual_id - creature visual appears
- Spawn two different monsters with same visual_id - both use same mesh (cached)

#### 2.7 Deliverables

- [ ] `src/game/components/creature.rs` with Bevy components
- [ ] `src/game/systems/creature_meshes.rs` with mesh conversion
- [ ] `src/game/systems/creature_spawning.rs` with spawning logic
- [ ] `src/game/systems/monster_rendering.rs` with monster-visual integration
- [ ] Updated `src/game/systems/procedural_meshes.rs` with creature cache
- [ ] Updated `src/game/systems/mod.rs` with system registration
- [ ] Unit tests with >80% coverage
- [ ] Monster-visual integration tests
- [ ] Test campaign with example creatures
- [ ] Test monsters with visual_id references
- [ ] Documentation in `docs/explanation/implementations.md`

#### 2.8 Success Criteria

- Creatures spawn and render in game
- Multi-mesh creatures show all parts correctly
- Materials/colors match RON definitions
- Mesh caching reduces draw calls for repeated creatures
- No visual artifacts (z-fighting, incorrect normals)
- Frame rate stable with 50+ creatures on screen
- Monsters with visual_id reference spawn with correct creature visual
- Monsters without visual_id use fallback rendering (don't crash)
- Multiple monsters can share same visual (mesh cache working)

---

### Phase 3: Campaign Builder Visual Editor

#### 3.1 Creature Editor UI

Create `sdk/campaign_builder/src/creature_editor.rs`:

- `pub struct CreatureEditorState` with fields:
  - `current_creature: Option<CreatureDefinition>`
  - `selected_mesh_index: Option<usize>`
  - `mesh_templates: Vec<MeshTemplate>`
  - `export_path: String`
  - `camera_distance: f32`
  - `camera_angle: (f32, f32)` (pitch, yaw)
  - `show_wireframe: bool`
  - `show_normals: bool`

UI Layout (egui):

- **Left Panel (200px):** Mesh list, add/remove/duplicate buttons
- **Center Panel:** 3D preview viewport (using Bevy rendering)
- **Right Panel (300px):** Selected mesh properties editor
- **Bottom Panel (150px):** Export/import buttons, validation status

Mesh List View:

- Show all meshes in `current_creature.meshes`
- Selectable list with mesh names
- Buttons: "Add Primitive", "Duplicate", "Delete"
- Drag-to-reorder support (future enhancement)

#### 3.2 Mesh Property Editor

Right panel sections:

**Mesh Info Section:**

- Text input: Mesh name
- Color picker: `mesh.color` (RGBA)
- Checkbox: "Auto-calculate normals"

**Primitive Generation Section:**

- Dropdown: Primitive type (Cube, Sphere, Cylinder, Pyramid)
- Size/radius/height sliders
- Segment count slider (for sphere/cylinder)
- Button: "Generate Primitive" - replaces current mesh

**Transform Section:**

- Translation: X, Y, Z sliders (-5.0 to 5.0)
- Rotation: Pitch, Yaw, Roll sliders (0° to 360°)
- Scale: X, Y, Z sliders (0.1 to 5.0)

**Vertex Editor Section (Advanced):**

- Table showing vertices (read-only for now)
- Vertex count display
- Triangle count display
- Button: "Export to JSON" for external editing

**Validation Section:**

- Display validation errors in red
- Display warnings in yellow
- Display success message in green
- Button: "Validate Mesh"

#### 3.3 3D Preview Integration

Create `sdk/campaign_builder/src/preview_renderer.rs`:

- Embed minimal Bevy app for preview rendering
- Render to egui texture using `egui_render_to_texture`
- Camera controls: click-drag to rotate, scroll to zoom
- Display creature at origin with grid background
- Toggle wireframe overlay
- Toggle normal visualization (debug lines)

Preview Controls:

- Mouse drag: Rotate camera
- Mouse scroll: Zoom in/out
- Right-click drag: Pan camera
- Reset button: Return to default view
- Background color picker

#### 3.4 Template/Primitive Generators

Create `sdk/campaign_builder/src/mesh_templates.rs`:

- `pub fn create_cube(size: f32, color: [f32; 4]) -> MeshDefinition`
- `pub fn create_sphere(radius: f32, segments: u32, color: [f32; 4]) -> MeshDefinition`
- `pub fn create_cylinder(radius: f32, height: f32, segments: u32, color: [f32; 4]) -> MeshDefinition`
- `pub fn create_pyramid(base: f32, height: f32, color: [f32; 4]) -> MeshDefinition`
- `pub fn create_cone(radius: f32, height: f32, segments: u32, color: [f32; 4]) -> MeshDefinition`

Creature Templates:

- `pub fn template_humanoid() -> CreatureDefinition` - Basic humanoid (head, torso, limbs)
- `pub fn template_quadruped() -> CreatureDefinition` - Four-legged creature
- `pub fn template_dragon() -> CreatureDefinition` - Dragon base (body, wings, tail)
- `pub fn template_robot() -> CreatureDefinition` - Simple robot (cubes)

#### 3.5 Integration with Main SDK

Update `sdk/campaign_builder/src/lib.rs`:

Add to `EditorTab` enum:

```rust
Creatures, // New tab
```

Add to `CampaignBuilderApp` struct:

```rust
creature_editor_state: CreatureEditorState,
```

Add tab UI:

```rust
ui.selectable_value(&mut state.active_tab, EditorTab::Creatures, "🦖 Creatures");
```

Render creature editor when tab active:

```rust
EditorTab::Creatures => render_creature_editor(ui, &mut state.creature_editor_state),
```

#### 3.6 Testing Requirements

**Unit Tests (`sdk/campaign_builder/tests/creature_editor_tests.rs`):**

- `test_create_cube_template_valid` - Cube generation works
- `test_create_sphere_segments` - Sphere segment count correct
- `test_creature_editor_add_mesh` - Add mesh to creature
- `test_creature_editor_remove_mesh` - Remove mesh from creature
- `test_creature_editor_duplicate_mesh` - Duplicate mesh
- `test_mesh_templates_all_valid` - All templates validate

**Integration Tests (`sdk/campaign_builder/tests/creature_editor_integration.rs`):**

- `test_creature_editor_ui_renders` - UI renders without panic
- `test_export_creature_to_ron` - Export creates valid RON file
- `test_import_creature_from_ron` - Import loads correctly
- `test_validation_catches_errors` - Invalid meshes flagged

**Manual UI Tests:**

- Open creature editor tab - UI displays correctly
- Add cube primitive - Preview shows cube
- Change color - Preview updates color
- Add second mesh - Both meshes visible
- Export creature - RON file created
- Import creature - Creature loads in editor
- Rotate camera - Camera responds smoothly

#### 3.7 Deliverables

- [ ] `sdk/campaign_builder/src/creature_editor.rs` with main editor UI
- [ ] `sdk/campaign_builder/src/mesh_templates.rs` with primitive generators
- [ ] `sdk/campaign_builder/src/preview_renderer.rs` with 3D preview
- [ ] Updated `sdk/campaign_builder/src/lib.rs` with Creatures tab
- [ ] Unit tests with >80% coverage
- [ ] Integration tests for editor functionality
- [ ] User documentation in `docs/how-to/create_creatures.md`
- [ ] Documentation in `docs/explanation/implementations.md`

#### 3.8 Success Criteria

- Creatures tab visible in Campaign Builder
- Can create new creature from scratch
- Can add/remove/edit meshes
- Primitive generators create valid meshes
- 3D preview renders creatures correctly
- Export produces valid RON files
- Import loads exported files without errors
- Validation catches mesh errors before export

---

### Phase 4: Content Pipeline Integration

#### 4.1 Campaign Loading Updates

Update `src/sdk/campaign_loader.rs`:

- Add `fn load_creatures(campaign_path: &Path) -> Result<Vec<CreatureDefinition>, CampaignError>`
- Scan `{campaign_path}/creatures/` directory
- Load all `.ron` files
- Validate each creature definition
- Return Vec of loaded creatures

Update `Campaign` struct:

```rust
pub creatures: Vec<CreatureDefinition>,
```

Update `CampaignLoader::load_campaign()`:

- Call `load_creatures()` and populate `campaign.creatures`
- Add creature validation to campaign validation step
- Log creature count during load

#### 4.2 Validation Framework

Update `src/sdk/validation.rs`:

- Add `fn validate_creature(creature: &CreatureDefinition) -> Vec<ValidationError>`
- Check creature name not empty
- Check scale > 0.0
- Check health > 0.0
- Check speed >= 0.0
- Validate each mesh in `creature.meshes`
- Check for mesh name collisions within creature

Create `src/sdk/creature_validation.rs`:

- `fn validate_mesh_topology(mesh: &MeshDefinition) -> Result<(), TopologyError>`
- Check no degenerate triangles (zero area)
- Check consistent winding order (all CCW or all CW)
- Check manifold edges (each edge shared by exactly 2 triangles)
- Warn on isolated vertices (not referenced by any triangle)

Add validation to Campaign Builder:

- Run validation on creature save
- Display validation results in UI
- Block export if validation fails (with override option)

#### 4.3 Export/Import Functionality

Update `sdk/campaign_builder/src/packager.rs`:

- Add creatures to campaign package
- Create `creatures/` directory in package
- Copy all creature RON files
- Update `campaign.ron` with creature file list
- Validate creatures before packaging

Add import functionality:

- `fn import_creature_from_file(path: &Path) -> Result<CreatureDefinition, ImportError>`
- `fn import_creature_library(dir: &Path) -> Result<Vec<CreatureDefinition>, ImportError>`
- Support batch import from directory
- Support importing from other campaigns

#### 4.4 Asset Management

Create `sdk/campaign_builder/src/creature_assets.rs`:

- `struct CreatureAssetManager` - Manages creature file I/O
- `fn save_creature(creature: &CreatureDefinition, path: &Path) -> Result<(), IoError>`
- `fn load_creature(path: &Path) -> Result<CreatureDefinition, IoError>`
- `fn list_creatures(campaign_dir: &Path) -> Result<Vec<String>, IoError>`
- `fn delete_creature(campaign_dir: &Path, creature_name: &str) -> Result<(), IoError>`

Add asset browser UI:

- Display all creatures in campaign
- Thumbnail previews (future enhancement)
- Search/filter by name
- Sort by name/date modified
- Duplicate creature (copy RON file with new name)

#### 4.5 Testing Requirements

**Unit Tests (`src/sdk/tests/creature_pipeline_tests.rs`):**

- `test_load_creatures_from_campaign` - Campaign loader finds creatures
- `test_validate_creature_invalid_scale` - Validation catches bad scale
- `test_validate_mesh_degenerate_triangle` - Topology validation works
- `test_export_creature_creates_file` - Export writes RON file
- `test_import_creature_loads_correctly` - Import reads RON file

**Integration Tests (`sdk/campaign_builder/tests/pipeline_integration.rs`):**

- `test_full_creature_workflow` - Create, save, load, export, import
- `test_campaign_with_creatures_packages_correctly` - Packaging includes creatures
- `test_creature_validation_prevents_export` - Validation blocks bad exports
- `test_batch_import_creatures` - Import multiple creatures at once

#### 4.6 Deliverables

- [ ] Updated `src/sdk/campaign_loader.rs` with creature loading
- [ ] Updated `src/sdk/validation.rs` with creature validation
- [ ] New `src/sdk/creature_validation.rs` with topology checks
- [ ] Updated `sdk/campaign_builder/src/packager.rs` with creature packaging
- [ ] New `sdk/campaign_builder/src/creature_assets.rs` with asset management
- [ ] Unit tests with >80% coverage
- [ ] Integration tests for full pipeline
- [ ] Documentation in `docs/explanation/implementations.md`

#### 4.7 Success Criteria

- Campaigns load creatures automatically
- Creature validation runs on campaign load
- Invalid creatures logged with clear error messages
- Campaign packaging includes all creature files
- Exported campaigns contain creatures
- Imported campaigns load creatures correctly
- Asset browser shows all campaign creatures

---

### Phase 5: Advanced Features & Polish

#### 5.1 Creature Variations System

Create `src/domain/visual/creature_variations.rs`:

- `struct CreatureVariation` - Defines override parameters
  - `base_creature_id: CreatureId`
  - `name: String`
  - `scale_override: Option<f32>`
  - `mesh_color_overrides: HashMap<usize, [f32; 4]>`
  - `mesh_scale_overrides: HashMap<usize, Vec3>`
- `fn apply_variation(base: &CreatureDefinition, variation: &CreatureVariation) -> CreatureDefinition`
- Allows creating color variants (blue dragon, red dragon) from base
- Allows scale variants (young dragon, ancient dragon) from base

Add variation UI to creature editor:

- Button: "Create Variation" - derive new creature from current
- Variation editor panel - edit override parameters
- Preview shows variation applied to base

#### 5.2 LOD (Level of Detail) Support

Add LOD fields to `MeshDefinition`:

```rust
pub lod_levels: Option<Vec<MeshDefinition>>, // Simplified versions
pub lod_distances: Option<Vec<f32>>, // Distance thresholds
```

Create mesh simplification function:

- `fn simplify_mesh(mesh: &MeshDefinition, target_triangle_count: usize) -> MeshDefinition`
- Reduce triangle count while preserving silhouette
- Generate LOD0 (full detail), LOD1 (50%), LOD2 (25%), LOD3 (billboard)

Add LOD system to game:

- `fn creature_lod_system(query: Query<(&CreatureVisual, &Transform, &Children)>, camera: Query<&Transform, With<Camera>>, mut meshes: Query<&mut Handle<Mesh>>)`
- Calculate distance from camera to creature
- Swap mesh handles based on distance thresholds
- Update mesh cache to store LOD levels

#### 5.3 Material & Texture Support

Extend `MeshDefinition`:

```rust
pub material: Option<MaterialDefinition>,
pub texture_path: Option<String>,
```

Create `MaterialDefinition`:

```rust
pub struct MaterialDefinition {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: Option<[f32; 3]>,
    pub alpha_mode: AlphaMode, // Opaque, Blend, Mask
}
```

Add texture loading:

- Scan campaign `textures/` directory
- Load textures as Bevy `Image` assets
- Apply to materials via `base_color_texture`
- Add texture picker to creature editor UI

#### 5.4 Animation Keyframes (Foundation)

Create `src/domain/visual/animation.rs`:

```rust
pub struct AnimationDefinition {
    pub name: String,
    pub duration: f32,
    pub keyframes: Vec<Keyframe>,
    pub looping: bool,
}

pub struct Keyframe {
    pub time: f32,
    pub mesh_index: usize,
    pub transform: MeshTransform,
}
```

Add animation editor UI:

- Timeline scrubber
- Keyframe markers
- Add/remove keyframes
- Preview animation playback

Animation playback system:

- `fn creature_animation_system(time: Res<Time>, query: Query<(&CreatureAnimation, &Children)>, transforms: Query<&mut Transform>)`
- Interpolate between keyframes
- Apply transforms to child mesh entities
- Support blend trees (future)

#### 5.5 Creature Library/Templates

Create pre-built creature library:

- `data/creature_templates/` directory in SDK
- Humanoid template (customizable height, proportions)
- Quadruped template (customizable leg length, body size)
- Dragon template (customizable wing size, tail length)
- Robot template (modular parts)
- Undead template (skeletal structure)

Add template browser UI:

- Gallery view with thumbnails
- Search by category (humanoid, beast, monster, undead)
- "Create from Template" button
- Templates can be customized after creation

Catalog Integration:

- Load example creatures from `notes/procedural_meshes_complete/`
- Import into SDK templates directory
- Add metadata (category, tags, difficulty)

#### 5.6 Testing Requirements

**Unit Tests:**

- `test_creature_variation_color_override` - Color variation works
- `test_creature_variation_scale_override` - Scale variation works
- `test_mesh_simplification_reduces_triangles` - LOD generation works
- `test_material_definition_serialization` - Materials save/load
- `test_animation_keyframe_interpolation` - Animation interpolates correctly

**Integration Tests:**

- `test_lod_system_switches_meshes` - LOD switches at correct distances
- `test_texture_loading_from_campaign` - Textures load and apply
- `test_animation_playback_smooth` - Animations play smoothly
- `test_template_library_loads` - Templates load from SDK directory

**Performance Tests:**

- Render 100 creatures with LOD - maintains 60 FPS
- Render 1000 creatures with instancing - maintains 30 FPS
- Load 50 creatures from campaign - loads under 2 seconds

#### 5.7 Deliverables

- [ ] `src/domain/visual/creature_variations.rs` with variation system
- [ ] LOD support in `MeshDefinition` and rendering pipeline
- [ ] Material/texture support in mesh definitions
- [ ] Animation foundation in `src/domain/visual/animation.rs`
- [ ] Template library in `data/creature_templates/`
- [ ] Updated creature editor with variation/LOD/animation UIs
- [ ] Performance optimizations for large creature counts
- [ ] Example creatures from notes imported as templates
- [ ] Documentation in `docs/explanation/implementations.md`
- [ ] User guide in `docs/how-to/create_creatures.md`

#### 5.8 Success Criteria

- Creature variations create new creatures from bases
- LOD system improves performance with distant creatures
- Textures load and apply correctly to creatures
- Animation keyframes can be created and played back
- Template library provides starting points for creators
- Example creatures (dragon, skeleton, etc.) available as templates
- Performance acceptable with 100+ creatures on screen
- All quality gates pass (fmt, check, clippy, tests)

---

## Cross-Cutting Concerns

### Documentation Updates

**Files to create/update:**

- `docs/how-to/create_creatures.md` - Tutorial for creating custom creatures
- `docs/reference/creature_ron_format.md` - RON file format specification
- `docs/explanation/procedural_mesh_architecture.md` - System architecture
- `docs/explanation/implementations.md` - Implementation summary
- `README.md` - Add procedural mesh feature to feature list

### Data File Organization

**Campaign structure:**

```
campaigns/{campaign_name}/
├── campaign.ron
├── creatures/           # New directory
│   ├── heroes/
│   │   ├── knight.ron
│   │   └── wizard.ron
│   ├── monsters/
│   │   ├── goblin.ron
│   │   └── dragon.ron
│   └── npcs/
│       ├── innkeeper.ron
│       └── merchant.ron
├── textures/            # Optional, Phase 5
│   └── creature_skins/
└── ... (other campaign content)
```

### Performance Considerations

**Mesh caching strategy:**

- Cache generated Bevy meshes in `ProceduralMeshCache`
- Key: `(CreatureId, MeshIndex, LODLevel)`
- Clear cache on creature hot-reload
- Limit cache size (LRU eviction if > 1000 meshes)

**Rendering optimizations:**

- Use GPU instancing for duplicate creatures
- Frustum culling for off-screen creatures
- LOD switching based on camera distance
- Batch material changes to reduce state switches

### Validation Strategy

**Mesh validation levels:**

1. **Basic** - Vertices/indices not empty, indices in bounds
2. **Topology** - No degenerate triangles, manifold edges
3. **Quality** - Normal consistency, UV coverage, aspect ratios
4. **Performance** - Triangle count limits, vertex count limits

**Validation timing:**

- On mesh creation in editor (Basic)
- On creature save (Basic + Topology)
- On campaign export (All levels)
- On game load (Basic only, for performance)

### Error Handling

**Error types:**

- `MeshValidationError` - Mesh fails validation checks
- `CreatureLoadError` - Failed to load creature RON file
- `MeshGenerationError` - Failed to generate Bevy mesh
- `ExportError` - Failed to export creature to file

**Error presentation:**

- Editor: Red text with clear error message
- CLI validator: Detailed error with file/line number
- Game: Log warning and skip creature (don't crash)
- Campaign Builder: Validation panel with fixable issues highlighted

### Backwards Compatibility

**Not applicable** - This is a new feature with no existing data to migrate.

**Future compatibility:**

- Version field in `CreatureDefinition` for future schema changes
- RON format allows adding new optional fields without breaking old files
- LOD/animation/texture fields all optional for backwards compatibility

---

## Migration Path

**Phase 1 → Phase 2:**

- Domain types available for game systems to use
- No migration needed, additive changes only

**Phase 2 → Phase 3:**

- Game can spawn creatures from RON files
- Editor can create creatures for game to use

**Phase 3 → Phase 4:**

- Creatures editable in Campaign Builder
- Campaigns can include creature assets

**Phase 4 → Phase 5:**

- Basic workflow complete and tested
- Advanced features built on solid foundation

---

## Risk Mitigation

**Risk: Performance degradation with many creatures**

- Mitigation: Implement LOD and instancing early
- Fallback: Limit creature count per scene in campaign validation

**Risk: Editor 3D preview adds complexity to SDK**

- Mitigation: Use existing Bevy rendering, minimal custom code
- Fallback: Start with 2D preview (wireframe) if 3D too complex

**Risk: RON files become too large for complex creatures**

- Mitigation: Add compression option for large meshes
- Fallback: Support binary format for complex creatures (future)

**Risk: Mesh validation too strict, rejects valid meshes**

- Mitigation: Make validation levels configurable
- Fallback: Allow validation override with warning

**Risk: Integration with existing monster/NPC systems unclear**

- Mitigation: Keep visual system separate from game logic
- Fallback: `CreatureVisual` is optional component, existing systems work without it

---

## Design Decisions

### 1. Creature-to-Monster Linking: Separate Systems with References

**Decision:** Keep `CreatureDefinition` (visual) and `MonsterDefinition` (game logic) as separate systems linked by reference.

**Rationale:**

- `MonsterDefinition` (existing in `src/domain/combat/database.rs`): Contains game logic - stats, HP, AC, attacks, AI behavior (flee_threshold, can_advance, can_regenerate), resistances, loot tables, combat mechanics
- `CreatureDefinition` (new): Contains ONLY visual data - meshes, vertices, colors, scale, rendering information
- These are fundamentally different concerns that should remain decoupled

**Implementation:**

Add to `MonsterDefinition`:

```rust
pub visual_id: Option<CreatureId>, // References the 3D visual appearance
```

Add to NPC-related definitions (when created):

```rust
pub visual_id: Option<CreatureId>, // NPCs also reference creature visuals
```

**Benefits:**

- **Reusability:** Red Dragon and Blue Dragon monsters use same visual with color override, different stats
- **Flexibility:** Goblin Warrior and Goblin Shaman share visual, different combat abilities
- **Cross-system:** Friendly Innkeeper NPC and Hostile Bandit Monster can use same humanoid visual
- **Variants:** Easy to create stat variations without duplicating mesh data
- **Artist workflow:** Visual assets managed independently from game balance

**Examples:**

- `red_dragon` monster (visual_id: `dragon_base`, stats: fire_breath, high_hp)
- `blue_dragon` monster (visual_id: `dragon_base`, stats: ice_breath, medium_hp)
- `innkeeper` NPC (visual_id: `humanoid_merchant`, behavior: shop_keeper)
- `dying_goblin` NPC (visual_id: `goblin_wounded`, behavior: quest_giver)

**Implementation Notes:**

- Phase 1 adds `visual_id` to `MonsterDefinition`
- Phase 2 implements monster rendering with visual lookup
- NPC visual integration follows same pattern (add `visual_id` to NPC definitions when implemented)
- Visual system is agnostic to entity type (works for monsters, NPCs, player characters, etc.)

### 2. Animation System Scope: Simple Keyframe Transforms

**Decision:** Implement keyframe-based transform animations in Phase 5, defer skeletal animation to future work.

**Rationale:** Simple keyframe transforms are sufficient for low-poly aesthetic and easier to implement/edit.

### 3. Editor Preview Renderer: Separate Embedded Bevy App

**Decision:** Use isolated Bevy application embedded in SDK for preview rendering.

**Rationale:** SDK should not depend on game binary, keeps concerns separated and simplifies development.

---

## Implementation Order Summary

**Recommended implementation sequence:**

1. **Week 1-2: Phase 1** - Core domain types, validation, database
2. **Week 3-4: Phase 2** - Game rendering, spawning, caching
3. **Week 5-7: Phase 3** - Campaign Builder editor UI
4. **Week 8-9: Phase 4** - Content pipeline, packaging, validation
5. **Week 10-12: Phase 5** - Advanced features (LOD, materials, templates)

**Quick wins for early validation:**

- Implement Phase 1.1-1.3 first (core types + database)
- Create simple test creature RON file
- Validate it loads into `ContentDatabase`
- Proves architecture before investing in rendering/UI

**Parallel work opportunities:**

- Phase 2 (rendering) and Phase 3 (editor) can be developed in parallel
- One developer on game systems, one on SDK UI
- Integration point: Both use same domain types from Phase 1

---

## Success Metrics

**Technical metrics:**

- 100% test coverage on mesh validation functions
- Creature spawning <10ms per creature (uncached)
- Mesh cache hit rate >90% in typical gameplay
- Editor UI responsive <16ms frame time
- Campaign load time <5s for 100 creatures

**User experience metrics:**

- Can create simple creature (cube robot) in <5 minutes
- Can create complex creature (dragon) in <30 minutes
- Validation errors clear and actionable
- 3D preview renders at >30 FPS
- Export/import round-trip preserves all data

**Content creator metrics:**

- 10+ example creatures in template library
- Documentation covers all editor features
- Tutorial creates creature from scratch to in-game
- Community can share creature files easily

---

## Appendix: Example Creature RON Files

**Simple Cube Robot (Phase 1 deliverable):**

```ron
CreatureDefinition(
    name: "SimpleRobot",
    scale: 1.0,
    meshes: [
        MeshDefinition(
            name: "body",
            vertices: [
                [-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5],
                [-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [-0.5, 0.5, -0.5],
            ],
            normals: None,
            uvs: None,
            indices: [
                0, 1, 2, 2, 3, 0,
                1, 5, 6, 6, 2, 1,
                7, 6, 5, 5, 4, 7,
                4, 0, 3, 3, 7, 4,
                4, 5, 1, 1, 0, 4,
                3, 2, 6, 6, 7, 3,
            ],
            color: Some([0.3, 0.5, 0.8, 1.0]),
        ),
    ],
)
```

**Complex Dragon (Phase 5 reference):**
See `notes/procedural_meshes_complete/red_dragon.ron` for full example.

---

## Conclusion

This plan provides a comprehensive, phased approach to implementing procedural mesh support in Antares. By following the outlined phases, the project will gain a powerful content creation tool that empowers campaign creators to build visually distinctive creatures, NPCs, and monsters without requiring 3D modeling software.

The plan respects Antares' existing architecture, leverages RON-based data-driven design, and integrates cleanly with both the game engine (Bevy ECS) and Campaign Builder SDK (egui). Each phase has clear deliverables, success criteria, and testing requirements to ensure quality at every step.
