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

### Phase 6: UI Integration for Advanced Features

**Status**: PLANNED

Integrate Phase 5 advanced features (variations, LOD, animations, templates) into Campaign Builder visual editor.

#### 6.1 Variation Editor UI

**File**: `sdk/campaign_builder/src/creatures_editor.rs`

Add variation creation and editing interface:

- "Create Variation" button in creature editor toolbar
- Variation panel showing:
  - Base creature selector (dropdown of existing creatures)
  - Name input field
  - Global scale override slider (0.1 - 10.0)
  - Per-mesh color override color pickers
  - Per-mesh scale override sliders (X, Y, Z)
- Preview shows variation applied to base in real-time
- "Apply Variation" button creates new creature from base + overrides
- Validation feedback for invalid mesh indices or negative scales

**New File**: `sdk/campaign_builder/src/variation_editor.rs`

- `VariationEditorState` struct with:
  - `base_creature_id: Option<CreatureId>`
  - `variation: CreatureVariation`
  - `preview_creature: Option<CreatureDefinition>`
- `render_variation_editor()` method
- `apply_variation_to_preview()` updates preview in real-time

#### 6.2 LOD Editor UI

**File**: `sdk/campaign_builder/src/creatures_editor.rs`

Add LOD level management:

- "Generate LOD Levels" button in mesh editor
- LOD configuration dialog:
  - Number of levels slider (1-5)
  - Auto-calculate distances checkbox (default: on)
  - Manual distance inputs if auto-calculate off
- LOD level preview dropdown:
  - "LOD0 (Full Detail)" - original mesh
  - "LOD1 (50%)" - first simplified level
  - "LOD2 (25%)" - second simplified level
  - etc.
- Triangle count display per LOD level
- Preview switches mesh based on LOD dropdown
- "Clear LOD Levels" button removes generated LODs

**Helper Functions**:

- `generate_and_preview_lods()` - calls `lod::generate_lod_levels()` and updates UI
- `render_lod_info_panel()` - shows LOD statistics table

#### 6.3 Animation Editor UI

**New File**: `sdk/campaign_builder/src/animation_editor.rs`

- `AnimationEditorState` struct:
  - `animation: AnimationDefinition`
  - `current_time: f32` - playback position
  - `playing: bool` - playback state
  - `selected_keyframe: Option<usize>`
- Timeline scrubber widget (0.0 to duration)
- Keyframe markers on timeline (clickable)
- Transport controls: Play/Pause, Stop, Loop toggle
- Keyframe editing panel:
  - Time input (seconds)
  - Mesh index selector
  - Transform inputs (translation, rotation, scale)
  - "Add Keyframe" button
  - "Delete Keyframe" button
- Animation properties:
  - Name input
  - Duration slider (0.1 - 60.0 seconds)
  - Looping checkbox
- Preview applies current animation frame to creature

**Integration**: Add "Animations" tab to creature editor

#### 6.4 Template Browser UI

**New File**: `sdk/campaign_builder/src/template_browser.rs`

- `TemplateBrowserState` struct:
  - `templates: Vec<CreatureDefinition>`
  - `selected_template: Option<CreatureId>`
  - `search_query: String`
  - `category_filter: Option<TemplateCategory>`
- Gallery view with template thumbnails (3x3 grid)
- Search bar (filters by name)
- Category filter buttons:
  - All, Humanoid, Quadruped, Dragon, Robot, Undead
- Template card shows:
  - Name
  - Thumbnail preview (static mesh render)
  - Triangle count
  - "Use Template" button
- "Use Template" creates copy with new ID in current campaign

**Template Loading**:

- Load templates from `data/creature_templates/*.ron` on startup
- Cache in memory for fast browsing

#### 6.5 Material Editor UI

**File**: `sdk/campaign_builder/src/creatures_editor.rs`

Add material editing to mesh properties panel:

- "Material" collapsing header in mesh editor
- Base color picker (RGBA)
- Metallic slider (0.0 - 1.0)
- Roughness slider (0.0 - 1.0)
- Emissive color picker (RGB, optional)
- Alpha mode dropdown: Opaque, Blend, Mask
- "Clear Material" button (sets to None)

#### 6.6 Texture Picker UI

**File**: `sdk/campaign_builder/src/creatures_editor.rs`

Add texture selection:

- "Texture" collapsing header in mesh editor
- File browser button: "Select Texture..."
- Shows relative path: `textures/dragon_scales.png`
- Thumbnail preview if texture loaded
- "Clear Texture" button (sets to None)
- Validates texture file exists in campaign directory

#### 6.7 Testing Requirements

**Unit Tests**:

- `test_variation_editor_state_initialization`
- `test_variation_apply_updates_preview`
- `test_lod_generation_ui_updates_levels`
- `test_animation_editor_keyframe_add_remove`
- `test_template_browser_loads_templates`
- `test_template_browser_search_filter`
- `test_material_editor_updates_mesh`
- `test_texture_picker_validates_path`

**Integration Tests**:

- `test_create_variation_from_ui`
- `test_generate_lods_and_preview`
- `test_animation_playback_in_preview`
- `test_use_template_creates_creature`

**Manual Testing**:

- Create color variation (blue dragon from base)
- Generate LOD levels and verify triangle counts
- Create simple bounce animation and preview
- Browse templates and create creature from template
- Apply materials and textures to mesh

#### 6.8 Deliverables

- [ ] `sdk/campaign_builder/src/variation_editor.rs` with variation UI
- [ ] `sdk/campaign_builder/src/animation_editor.rs` with animation timeline
- [ ] `sdk/campaign_builder/src/template_browser.rs` with gallery view
- [ ] LOD generation UI in creature editor
- [ ] Material editor UI in mesh properties
- [ ] Texture picker UI in mesh properties
- [ ] Animation tab in creature editor
- [ ] Template browser accessible from creature editor
- [ ] Real-time preview updates for all editors
- [ ] Documentation in `docs/how-to/use_creature_editor.md`

#### 6.9 Success Criteria

- User can create variations with 3 clicks: Select base → Adjust color → Apply
- LOD generation shows immediate feedback (triangle counts, preview)
- Animation timeline supports drag-and-drop keyframe editing
- Template browser loads and displays 10+ templates under 500ms
- Material changes update preview in real-time
- Texture picker validates files and shows thumbnails
- All UI tests pass
- User guide demonstrates each feature

---

### Phase 7: Game Engine Integration

**Status**: PLANNED

Integrate Phase 5 features into game engine for runtime use (texture loading, LOD switching, animation playback).

#### 7.1 Texture Loading System

**File**: `src/game/systems/creature_meshes.rs`

Extend `mesh_definition_to_bevy()` to handle textures:

- Check `mesh.texture_path` field
- If Some, load texture using Bevy asset server:
  - `asset_server.load(texture_path)`
  - Wait for texture to load or use placeholder
- Apply texture to material's `base_color_texture`
- Handle missing textures gracefully (error message, use default)

**New System**: `texture_loading_system`

```rust
fn texture_loading_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&CreatureVisual, &Handle<StandardMaterial>), Without<TextureLoaded>>,
) {
    // Load textures for creatures
}
```

**Marker Component**: `TextureLoaded` - prevents re-loading

#### 7.2 Material Application System

**File**: `src/game/systems/creature_meshes.rs`

Update material creation to use `MaterialDefinition`:

- Convert `MaterialDefinition` → Bevy `StandardMaterial`:
  - `base_color` → `StandardMaterial::base_color`
  - `metallic` → `StandardMaterial::metallic`
  - `roughness` → `StandardMaterial::perceptual_roughness`
  - `emissive` → `StandardMaterial::emissive`
  - `alpha_mode` → `StandardMaterial::alpha_mode`
- Apply to mesh material handle

**Function**: `material_definition_to_bevy(def: &MaterialDefinition) -> StandardMaterial`

#### 7.3 LOD Switching System

**New File**: `src/game/systems/lod.rs`

Implement automatic LOD level switching based on camera distance:

```rust
pub struct LodState {
    pub current_level: usize,
    pub mesh_handles: Vec<Handle<Mesh>>, // LOD0, LOD1, LOD2, etc.
    pub distances: Vec<f32>,
}

fn lod_switching_system(
    mut commands: Commands,
    camera_query: Query<&Transform, With<Camera>>,
    mut creature_query: Query<(&Transform, &mut LodState, &Children)>,
    mut mesh_query: Query<&mut Handle<Mesh>>,
) {
    // Calculate distance from camera to creature
    // Select appropriate LOD level based on distance
    // Swap mesh handle if LOD level changed
}
```

**Integration**: Add `LodState` component when spawning creatures with LOD levels

#### 7.4 Animation Playback System

**New File**: `src/game/systems/animation.rs`

Implement keyframe animation playback:

```rust
pub struct CreatureAnimation {
    pub definition: AnimationDefinition,
    pub current_time: f32,
    pub playing: bool,
}

fn animation_playback_system(
    time: Res<Time>,
    mut query: Query<(&mut CreatureAnimation, &Children)>,
    mut transform_query: Query<&mut Transform>,
) {
    // Update current_time
    // Sample animation at current_time
    // Apply transforms to child mesh entities
}
```

**Features**:

- Advance `current_time` by `delta_seconds`
- Loop or stop based on `animation.definition.looping`
- Use `animation.definition.sample(mesh_index, current_time)` for transforms
- Apply to corresponding mesh child entity

#### 7.5 Creature Spawning with Advanced Features

**File**: `src/game/systems/creature_spawning.rs` (or create if doesn't exist)

Update creature spawning to include:

- LOD state initialization if `creature.meshes[0].lod_levels.is_some()`
- Material application from `mesh.material`
- Texture loading from `mesh.texture_path`
- Animation component if creature has animations

**Spawn Function Signature**:

```rust
fn spawn_creature_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    creature: &CreatureDefinition,
    position: Vec3,
    animation: Option<AnimationDefinition>,
) -> Entity
```

#### 7.6 Performance Optimizations

**Instancing Support**:

- Multiple creatures with same visual use same mesh/material instances
- Bevy's built-in instancing for identical materials
- Track mesh cache by `CreatureId` to reuse assets

**Batching**:

- Group creatures by visual ID for draw call reduction
- Use Bevy's automatic batching

**Profiling Hooks**:

- Add `#[cfg(feature = "profiling")]` spans:
  - `lod_switching_system`
  - `animation_playback_system`
  - `texture_loading_system`

#### 7.7 Testing Requirements

**Unit Tests**:

- `test_material_definition_conversion`
- `test_lod_distance_calculation`
- `test_animation_time_advance`
- `test_texture_path_resolution`

**Integration Tests**:

- `test_spawn_creature_with_lod`
- `test_spawn_creature_with_animation`
- `test_spawn_creature_with_texture`
- `test_lod_switches_at_distance`

**Performance Tests**:

- Render 100 creatures with LOD - maintains 60 FPS
- Render 50 animated creatures - maintains 30 FPS
- Load 20 textured creatures - completes under 2 seconds

#### 7.8 Deliverables

- [ ] `src/game/systems/lod.rs` with LOD switching
- [ ] `src/game/systems/animation.rs` with animation playback
- [ ] Texture loading in `creature_meshes.rs`
- [ ] Material conversion in `creature_meshes.rs`
- [ ] Updated creature spawning with all features
- [ ] Performance profiling integration
- [ ] Instancing and batching optimizations
- [ ] Documentation in `docs/explanation/game_engine_integration.md`

#### 7.9 Success Criteria

- Creatures spawn with correct textures from campaign
- LOD switches automatically at specified distances
- Animations play smoothly at 60 FPS
- Materials render with PBR lighting
- 100 creatures render at 60+ FPS with LOD
- 50 animated creatures render at 30+ FPS
- Texture loading doesn't block gameplay
- All integration tests pass

---

### Phase 8: Content Creation & Templates

**Status**: PLANNED

Expand creature template library with diverse examples and create comprehensive content creation tutorials.

#### 8.1 Additional Creature Templates

**Directory**: `data/creature_templates/`

Create high-quality templates (RON format):

**Quadruped Template** (`quadruped.ron`):

- Body mesh (torso)
- Head mesh
- 4 leg meshes (front-left, front-right, back-left, back-right)
- Tail mesh
- Customizable: leg length, body size, tail length
- Template ID: 1001

**Dragon Template** (`dragon.ron`):

- Body mesh (elongated torso)
- Head mesh (with snout)
- Neck mesh
- 2 wing meshes (left, right)
- 4 leg meshes
- Tail mesh (long, segmented)
- Customizable: wing size, tail length, scale colors
- Template ID: 1002

**Robot Template** (`robot.ron`):

- Chassis mesh (boxy torso)
- Head mesh (cube with antenna)
- 2 arm meshes (segmented)
- 2 leg meshes (cylindrical)
- Modular design for easy customization
- Template ID: 1003

**Undead Template** (`undead.ron`):

- Skeletal structure (thin meshes)
- Skull head
- Exposed ribcage torso
- Bone arms and legs
- Ghostly color tint option
- Template ID: 1004

**Beast Template** (`beast.ron`):

- Muscular quadruped body
- Large jaw head
- Claws on feet
- Optional horns/spikes
- Template ID: 1005

#### 8.2 Example Creatures from Notes

**Directory**: `data/creature_examples/`

Import procedural creatures from `notes/procedural_meshes_complete/`:

- Parse existing creature definitions
- Convert to RON format
- Add metadata (category, tags, difficulty)
- Include:
  - Simple creatures (cube, pyramid, sphere characters)
  - Medium creatures (bipeds, quadrupeds)
  - Complex creatures (dragons, multi-part monsters)

**Migration Script**: `scripts/import_example_creatures.sh`

#### 8.3 Template Metadata System

**New File**: `src/domain/visual/template_metadata.rs`

```rust
pub struct TemplateMetadata {
    pub category: TemplateCategory,
    pub tags: Vec<String>,
    pub difficulty: Difficulty,
    pub author: String,
    pub description: String,
    pub thumbnail_path: Option<String>,
}

pub enum TemplateCategory {
    Humanoid,
    Quadruped,
    Dragon,
    Robot,
    Undead,
    Beast,
    Custom,
}

pub enum Difficulty {
    Beginner,
    Intermediate,
    Advanced,
}
```

**File Format**: Each template has companion `.meta.ron` file:

- `humanoid.ron` → `humanoid.meta.ron`
- Contains `TemplateMetadata`

#### 8.4 Content Creation Tutorials

**File**: `docs/how-to/create_creatures.md` (NEW)

Comprehensive tutorial covering:

1. **Getting Started**:

   - Opening Campaign Builder
   - Creating first creature from template
   - Understanding mesh structure

2. **Basic Customization**:

   - Changing colors
   - Adjusting scale
   - Modifying transforms

3. **Creating Variations**:

   - Color variants (blue/red dragon)
   - Size variants (young/ancient)
   - Combining variations

4. **Working with Meshes**:

   - Adding/removing meshes
   - Primitive generators (cube, sphere, cylinder)
   - Mesh validation and fixing issues

5. **Advanced Features**:

   - Generating LOD levels
   - Applying materials and textures
   - Creating simple animations

6. **Best Practices**:
   - Avoiding degenerate triangles
   - Proper normal orientation
   - UV mapping guidelines
   - Performance considerations

**File**: `docs/tutorials/creature_creation_quickstart.md` (NEW)

5-minute quickstart:

1. Load humanoid template
2. Change color to blue
3. Scale to 2x
4. Save as "Blue Giant"
5. Preview in game

#### 8.5 Template Gallery Documentation

**File**: `docs/reference/creature_templates.md` (NEW)

Reference documentation for all templates:

- Template ID table
- Category breakdown
- Mesh count per template
- Customization options
- Usage examples
- Preview images (when available)

#### 8.6 Testing Requirements

**Template Validation Tests**:

- `test_all_templates_load_successfully`
- `test_template_metadata_valid`
- `test_templates_pass_validation`
- `test_template_ids_unique`

**Example Creature Tests**:

- `test_example_creatures_load`
- `test_example_creatures_render`

**Tutorial Validation**:

- Walk through each tutorial manually
- Verify all steps work as documented
- Check all file paths and references

#### 8.7 Deliverables

- [ ] Quadruped template (`quadruped.ron`)
- [ ] Dragon template (`dragon.ron`)
- [ ] Robot template (`robot.ron`)
- [ ] Undead template (`undead.ron`)
- [ ] Beast template (`beast.ron`)
- [ ] Template metadata files (`.meta.ron`)
- [ ] Example creatures from notes (10+ creatures)
- [ ] `docs/how-to/create_creatures.md` tutorial
- [ ] `docs/tutorials/creature_creation_quickstart.md`
- [ ] `docs/reference/creature_templates.md` reference
- [ ] Template validation tests
- [ ] Gallery images/thumbnails (optional)

#### 8.8 Success Criteria

- 5+ diverse templates available
- Each template has complete metadata
- 10+ example creatures imported
- Tutorial guides beginner through first creature (under 10 minutes)
- Reference documentation covers all templates
- All templates pass validation
- Community can create creatures without developer help
- Templates cover 80% of common creature types

---

### Phase 9: Performance & Optimization

**Status**: PLANNED

Optimize rendering performance for large creature counts, improve LOD algorithms, and implement advanced batching.

#### 9.1 Advanced LOD Algorithms

**File**: `src/domain/visual/lod.rs`

Replace basic triangle decimation with edge collapse algorithm:

**New Function**: `simplify_mesh_edge_collapse(mesh: &MeshDefinition, target_count: usize) -> MeshDefinition`

- Build half-edge data structure
- Calculate quadric error metrics per vertex
- Iteratively collapse edges with lowest error
- Preserve mesh boundaries
- Update normals after collapse

**Quadric Error Metrics**:

- Measure geometric error introduced by collapse
- Prioritize collapses that preserve shape
- Much better quality than area-based decimation

**Benchmark**: Compare old vs new simplification quality and performance

#### 9.2 Mesh Instancing System

**File**: `src/game/systems/instancing.rs` (NEW)

Implement GPU instancing for identical creatures:

```rust
pub struct InstancedCreature {
    pub creature_id: CreatureId,
    pub instances: Vec<InstanceData>,
}

pub struct InstanceData {
    pub transform: Mat4,
    pub color_tint: Vec4,
}

fn instancing_system(
    query: Query<(&CreatureVisual, &Transform, &ColorTint)>,
    mut instanced: ResMut<InstancedCreatures>,
) {
    // Group creatures by ID
    // Build instance buffers
    // Submit instanced draw calls
}
```

**Target**: 1000+ identical creatures in single draw call

#### 9.3 Mesh Batching Optimization

**File**: `src/game/systems/batching.rs` (NEW)

Implement static batching for non-moving creatures:

- Combine meshes with same material
- Build single vertex buffer
- Reduce draw calls by 90% for static content
- Invalidate batch on creature movement

**Dynamic Batching**:

- Sort creatures by material/texture
- Minimize state changes
- Group by render layer

#### 9.4 LOD Distance Auto-Tuning

**File**: `src/game/systems/lod.rs`

Add adaptive LOD distance calculation:

- Monitor frame rate
- Increase LOD distances if FPS < target
- Decrease LOD distances if FPS > target (better quality)
- Per-creature importance (player nearby = higher detail)

**Config**: `LodConfig` resource with tuning parameters

#### 9.5 Texture Atlas Generation

**File**: `src/game/systems/texture_atlas.rs` (NEW)

Combine creature textures into atlases:

- Reduce texture switches
- Pack small textures (256x256) into 2048x2048 atlas
- Adjust UVs automatically
- Generate at campaign load time

**Benefits**: Fewer texture binds, better GPU cache usage

#### 9.6 Memory Optimization

**Mesh Compression**:

- Quantize vertex positions (16-bit floats)
- Compress normals (octahedral encoding)
- Pack UVs (16-bit)
- 50% memory reduction

**Lazy Loading**:

- Load creature meshes on-demand
- Unload distant creatures
- LRU cache for creature definitions

#### 9.7 Profiling Integration

**File**: `src/game/profiling.rs`

Add Tracy/puffin profiling support:

- Instrument all systems
- Track GPU timing
- Mesh memory usage
- Draw call counts

**Metrics Dashboard** (debug mode):

- FPS graph
- Draw call count
- Vertex count
- Texture memory
- Creature count by LOD level

#### 9.8 Performance Testing Suite

**File**: `tests/performance/creature_rendering.rs`

Automated performance tests:

- `test_render_100_creatures_60fps`
- `test_render_1000_instances_60fps`
- `test_lod_switching_overhead`
- `test_animation_performance`
- `test_texture_loading_time`

**Benchmarks** (criterion):

- LOD generation speed
- Mesh simplification quality/time
- Instancing vs individual draws

#### 9.9 Testing Requirements

**Unit Tests**:

- `test_edge_collapse_preserves_topology`
- `test_quadric_error_calculation`
- `test_instance_buffer_generation`
- `test_mesh_batching_combines_correctly`

**Performance Tests**:

- 1000 creatures @ 60 FPS (with instancing)
- 100 unique creatures @ 30 FPS (with LOD)
- Texture atlas generation < 5 seconds
- LOD switching < 1ms per frame

**Regression Tests**:

- Track performance metrics over time
- Alert if FPS drops > 10%
- Memory usage increase > 20%

#### 9.10 Deliverables

- [ ] `src/domain/visual/lod.rs` with edge collapse algorithm
- [ ] `src/game/systems/instancing.rs` with GPU instancing
- [ ] `src/game/systems/batching.rs` with static batching
- [ ] `src/game/systems/texture_atlas.rs` with atlas generation
- [ ] Adaptive LOD distance tuning
- [ ] Mesh compression implementation
- [ ] Lazy loading system
- [ ] Profiling instrumentation
- [ ] Performance test suite
- [ ] Benchmark suite
- [ ] Documentation in `docs/explanation/performance_optimization.md`

#### 9.11 Success Criteria

- 1000+ creatures render at 60 FPS (with instancing)
- 100 unique creatures render at 60 FPS (with LOD)
- Edge collapse LOD quality 2x better than decimation
- Texture atlas reduces texture switches by 80%
- Memory usage < 100MB for 1000 creatures
- LOD switching overhead < 0.5ms per frame
- All performance tests pass
- Profiling shows no bottlenecks

---

### Phase 10: Advanced Animation Systems

**Status**: PLANNED

Implement skeletal animation, blend trees, inverse kinematics, and procedural animation.

#### 10.1 Skeletal Hierarchy System

**File**: `src/domain/visual/skeleton.rs` (NEW)

Define skeletal structure:

```rust
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub root_bone: BoneId,
}

pub struct Bone {
    pub id: BoneId,
    pub name: String,
    pub parent: Option<BoneId>,
    pub rest_transform: Transform,
    pub inverse_bind_pose: Mat4,
}

pub type BoneId = usize;
```

**Skinning Data**:

- Attach meshes to bones via weights
- Multiple bones influence single vertex
- Max 4 influences per vertex (GPU standard)

#### 10.2 Skeletal Animation

**File**: `src/domain/visual/skeletal_animation.rs` (NEW)

Extend animation system for skeletons:

```rust
pub struct SkeletalAnimation {
    pub name: String,
    pub duration: f32,
    pub bone_tracks: HashMap<BoneId, Vec<BoneKeyframe>>,
    pub looping: bool,
}

pub struct BoneKeyframe {
    pub time: f32,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}
```

**Features**:

- Per-bone animation tracks
- Quaternion rotation (smooth interpolation)
- SLERP for rotations, LERP for position/scale

#### 10.3 Animation Blend Trees

**File**: `src/domain/visual/blend_tree.rs` (NEW)

Implement animation blending:

```rust
pub enum BlendNode {
    Clip(AnimationClip),
    Blend2D { x_param: String, y_param: String, samples: Vec<BlendSample> },
    Additive { base: Box<BlendNode>, additive: Box<BlendNode>, weight: f32 },
    LayeredBlend { layers: Vec<(Box<BlendNode>, f32)> },
}

pub struct BlendSample {
    pub position: Vec2,
    pub animation: AnimationClip,
}
```

**Use Cases**:

- Walk/run blending based on speed
- Aim offset (look left/right)
- Additive hit reactions
- Layered upper/lower body animations

#### 10.4 Inverse Kinematics (IK)

**File**: `src/game/systems/ik.rs` (NEW)

Implement two-bone IK solver:

```rust
pub struct IkChain {
    pub bones: [BoneId; 2],
    pub target: Vec3,
    pub pole_target: Option<Vec3>,
}

fn solve_two_bone_ik(chain: &IkChain, skeleton: &Skeleton) -> [Quat; 2]
```

**Constraints**:

- Joint angle limits
- Pole vector for elbow/knee direction
- Chain length preservation

**Use Cases**:

- Foot placement on uneven terrain
- Hand reaching for objects
- Head look-at targets

#### 10.5 Procedural Animation

**File**: `src/game/systems/procedural_animation.rs` (NEW)

Generate animations at runtime:

**Idle Breathing**:

- Sine wave on torso scale
- Subtle head bobbing

**Walk Cycle Generation**:

- Inverse pendulum leg motion
- Arm swing counter to legs
- Hip rotation

**Ragdoll Physics** (basic):

- Convert skeleton to physics bodies
- Apply forces
- Update bone transforms from physics

#### 10.6 Animation State Machine

**File**: `src/domain/visual/animation_state_machine.rs` (NEW)

```rust
pub struct AnimationStateMachine {
    pub states: HashMap<String, AnimationState>,
    pub transitions: Vec<Transition>,
    pub current_state: String,
    pub parameters: HashMap<String, f32>,
}

pub struct AnimationState {
    pub name: String,
    pub blend_tree: BlendNode,
}

pub struct Transition {
    pub from: String,
    pub to: String,
    pub condition: TransitionCondition,
    pub duration: f32,
}
```

**Example States**:

- Idle → Walk (when speed > 0.1)
- Walk → Run (when speed > 3.0)
- Any → Jump (when jump pressed)
- Jump → Fall (when velocity.y < 0)

#### 10.7 Animation Compression

**File**: `src/domain/visual/animation_compression.rs` (NEW)

Reduce animation memory:

- Quantize keyframe values
- Remove redundant keyframes (linear segments)
- Curve fitting (fewer keyframes, same motion)
- 70% size reduction

#### 10.8 Animation Editor UI

**File**: `sdk/campaign_builder/src/skeletal_animation_editor.rs` (NEW)

Visual skeleton editor:

- Bone hierarchy tree view
- 3D skeleton viewport
- Bone transform gizmos
- Skinning weight painting
- Animation timeline with all bone tracks
- Blend tree visual editor

#### 10.9 Testing Requirements

**Unit Tests**:

- `test_skeleton_hierarchy_traversal`
- `test_two_bone_ik_solver`
- `test_animation_blending`
- `test_state_machine_transitions`
- `test_animation_compression_roundtrip`

**Integration Tests**:

- `test_skeletal_animation_playback`
- `test_blend_tree_evaluation`
- `test_ik_chain_reaches_target`
- `test_procedural_walk_cycle`

**Performance Tests**:

- 50 skeletal animated creatures @ 60 FPS
- IK solver < 0.1ms per chain
- Blend tree evaluation < 1ms per creature

#### 10.10 Deliverables

- [ ] `src/domain/visual/skeleton.rs` with skeletal system
- [ ] `src/domain/visual/skeletal_animation.rs` with bone animations
- [ ] `src/domain/visual/blend_tree.rs` with blending
- [ ] `src/game/systems/ik.rs` with IK solver
- [ ] `src/game/systems/procedural_animation.rs` with generators
- [ ] `src/domain/visual/animation_state_machine.rs` with FSM
- [ ] `src/domain/visual/animation_compression.rs` with compression
- [ ] `sdk/campaign_builder/src/skeletal_animation_editor.rs` UI
- [ ] Example skeletal creatures with animations
- [ ] Documentation in `docs/explanation/skeletal_animation.md`
- [ ] Tutorial in `docs/how-to/create_skeletal_animations.md`

#### 10.11 Success Criteria

- Skeletal animations play smoothly on humanoid template
- Blend trees smoothly transition walk→run
- IK feet stick to ground on slopes
- Procedural walk cycle looks natural
- State machine handles 10+ states/transitions
- 50 skeletal creatures render at 60 FPS
- Animation compression reduces size by 60%+
- Skeleton editor allows visual bone editing
- All animation tests pass
- Tutorial demonstrates complete workflow

---

## Implementation Status Summary

| Phase    | Status       | Description                          |
| -------- | ------------ | ------------------------------------ |
| Phase 1  | ✅ COMPLETED | Core Domain Integration              |
| Phase 2  | ✅ COMPLETED | Game Engine Rendering                |
| Phase 3  | ✅ COMPLETED | Campaign Builder Visual Editor       |
| Phase 4  | ✅ COMPLETED | Content Pipeline Integration         |
| Phase 5  | ✅ COMPLETED | Advanced Features & Polish           |
| Phase 6  | 📋 PLANNED   | UI Integration for Advanced Features |
| Phase 7  | 📋 PLANNED   | Game Engine Integration              |
| Phase 8  | 📋 PLANNED   | Content Creation & Templates         |
| Phase 9  | 📋 PLANNED   | Performance & Optimization           |
| Phase 10 | 📋 PLANNED   | Advanced Animation Systems           |

## Conclusion

This plan provides a comprehensive, phased approach to implementing procedural mesh support in Antares. Phases 1-5 are complete, establishing the foundation for creature visuals, variations, LOD, materials, and animation keyframes.

Phases 6-10 build upon this foundation with:

- **Phase 6**: User-facing editors for all advanced features
- **Phase 7**: Runtime integration in the game engine
- **Phase 8**: Comprehensive content library and tutorials
- **Phase 9**: Performance optimization for large scenes
- **Phase 10**: Professional-grade skeletal animation

The plan respects Antares' existing architecture, leverages RON-based data-driven design, and integrates cleanly with both the game engine (Bevy ECS) and Campaign Builder SDK (egui). Each phase has clear deliverables, success criteria, and testing requirements to ensure quality at every step.

By following this roadmap, Antares will gain a powerful content creation tool that empowers campaign creators to build visually distinctive creatures, NPCs, and monsters without requiring 3D modeling software or external tools.
