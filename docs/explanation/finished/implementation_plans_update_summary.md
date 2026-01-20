# Implementation Plans Update Summary

**Date**: 2025-01-XX
**Purpose**: Document required updates to align Procedural Meshes and Sprite Support implementation plans with the unified "2.5D" rendering approach.

---

## Overview

This document summarizes the changes made to both implementation plans to establish a clear separation:

- **3D Procedural World**: Trees, Signs, Portals (static environmental objects)
- **2D Sprite Characters**: NPCs, Monsters, Recruitables (all actor entities)

**Key Principle**: All "actors" (entities representing characters) use billboard sprites. All static environmental objects use procedural 3D meshes.

---

## Changes to `procedural_meshes_implementation_plan.md`

### UPDATE Header Added

Prepended update notice explaining:
- Phase 2 (NPC Procedural Representation) **REMOVED**
- Phase 3 (Event Markers) **MODIFIED** - Removed recruitable markers
- Phase 4 (Performance) **MODIFIED** - Removed NPC/recruitable cache entries
- Rationale: Establishes "2.5D" aesthetic

### Phase 2: NPC Procedural Representation

**Status**: **REMOVED ENTIRELY**

**Rationale**: NPCs will be rendered as sprites, not geometry.

**Removed Content**:
- `spawn_npc()` function specification
- NPC dimension constants (body radius, height, etc.)
- NPC color constants (body color, head color)
- Integration steps for `spawn_map_markers`
- NPC procedural generation tests (4 tests)
- All references to Capsule3d body and Sphere head

### Phase 3: Event Marker Procedural Meshes

**Status**: **MODIFIED**

**Changes**:

1. **Removed Section 3.3**: "Implement Recruitable Character Procedural Generation"
   - Deleted `spawn_recruitable_marker()` function
   - Removed recruitable dimension constants
   - Removed recruitable color constants (glowing green cylinder)

2. **Updated Section 3.4**: "Integrate Event Marker Spawning"
   - Changed match statement to only handle `Sign` and `Teleport`
   - Added comment: "RecruitableCharacter rendering handled by sprite system"
   - Removed `spawn_recruitable_marker()` call

3. **Updated Section 3.5**: "Testing Requirements"
   - Removed `test_recruitable_constants_valid()`
   - Removed `test_spawn_recruitable_marker_creates_entity()`
   - Test count reduced from 15 to 9 total

4. **Updated Section 3.6**: "Deliverables"
   - Removed recruitable marker deliverable
   - Updated test count to 5 (down from 7)

### Phase 4: Performance Optimization and Polish

**Status**: **MODIFIED**

**Changes to Section 4.1** (ProceduralMeshCache):

**Removed Fields**:
```rust
// REMOVED:
npc_body: Option<Handle<Mesh>>,
npc_head: Option<Handle<Mesh>>,
recruitable_cylinder: Option<Handle<Mesh>>,
```

**Kept Fields**:
```rust
// KEPT:
tree_trunk: Option<Handle<Mesh>>,
tree_foliage: Option<Handle<Mesh>>,
portal_torus: Option<Handle<Mesh>>,
sign_post: Option<Handle<Mesh>>,
sign_board: Option<Handle<Mesh>>,
```

### Documentation Updates

**Section 5.1**: Updated implementation summary to reflect:
- Only Trees, Portals, and Signs implemented
- 9 tests passing (not 15)
- No NPC procedural meshes
- Reference to sprite_support_implementation_plan.md for character rendering

**Appendix**: Updated test coverage section:
- Total unit tests: 9 (down from 15)
- Removed all NPC-related tests
- Removed recruitable-related tests

---

## Changes to `sprite_support_implementation_plan.md`

### UPDATE Header Added

Prepended update notice explaining:
- Phase 2 **EXPANDED** - Now primary pipeline for all actors
- Phase 3 **REFINED** - Native Bevy PBR billboard approach
- Dependency change: No `bevy_sprite3d`, using native `bevy::pbr`
- Rationale: Native Bevy PBR provides better stability and lighting

### Overview Section

**Updated** to include:
- "Add sprite-based visual rendering for tiles **and all character entities**"
- "**Character Rendering Philosophy**: All 'actors' use billboard sprites"
- Emphasis on NPCs, Monsters, Recruitables

### Current State Analysis

**Added** to Identified Issues:
- "No Character Sprite System"
- "Inconsistent Character Rendering"

**Technology Decision Section**:

**Changed from**: "PNG vs SVG"

**Changed to**: "Native Bevy PBR Billboard vs bevy_sprite3d"

**New Comparison Table**:

| Aspect       | Native Bevy PBR Billboard             | bevy_sprite3d                    |
|--------------|---------------------------------------|----------------------------------|
| Stability    | First-class Bevy support              | External crate, version coupling |
| Lighting     | Full PBR lighting integration         | Limited lighting support         |
| Performance  | Optimized by Bevy core team           | Community maintained             |
| Dependencies | Zero external dependencies            | Adds external dependency         |
| Flexibility  | Full control over materials/rendering | Plugin-based configuration       |

### Phase 2: Sprite Asset Infrastructure

**Status**: **EXPANDED**

**Section 2.1** - Changed from "Add bevy_sprite3d Dependency" to "No External Dependencies Required"
- **Removed**: `bevy_sprite3d = "3.0"` dependency
- **Using**: Native `bevy::pbr` and `bevy::render`

**Section 2.2** - "Create Sprite Asset Loader" **REDESIGNED**:

**New Data Structures**:
```rust
/// Resource managing sprite materials and quad meshes for billboard rendering
pub struct SpriteAssets {
    /// Cached materials per sprite sheet (texture + alpha blend settings)
    materials: HashMap<String, Handle<StandardMaterial>>,
    /// Cached quad meshes sized for each sprite sheet
    meshes: HashMap<String, Handle<Mesh>>,
    /// Sprite sheet configurations
    configs: HashMap<String, SpriteSheetConfig>,
}
```

**New Methods**:
- `get_or_load_material()` - Returns `Handle<StandardMaterial>` with alpha blend
- `get_or_load_mesh()` - Returns `Handle<Mesh>` (Rectangle quad)
- `get_sprite_uv_transform()` - Calculate UV offset/scale for atlas

**Material Settings**:
- `alpha_mode: AlphaMode::Blend`
- `unlit: false` (uses lighting)
- `perceptual_roughness: 0.9`

**Section 2.3** - "Create Sprite Sheet Registry" **EXPANDED**:

**Added Actor Sprite Sheets**:
```ron
"npcs_town": SpriteSheetConfig(
    texture_path: "textures/sprites/npcs_town.png",
    tile_size: (32.0, 48.0),
    columns: 4,
    rows: 4,
    sprites: [(0, "guard"), (1, "merchant"), ...]
),

"monsters_basic": SpriteSheetConfig(
    texture_path: "textures/sprites/monsters_basic.png",
    tile_size: (32.0, 48.0),
    columns: 4,
    rows: 4,
    sprites: [(0, "goblin"), (1, "orc"), ...]
),

"monsters_advanced": SpriteSheetConfig(...),

"recruitables": SpriteSheetConfig(
    texture_path: "textures/sprites/recruitable_characters.png",
    tile_size: (32.0, 48.0),
    columns: 4,
    rows: 2,
    sprites: [(0, "warrior_recruit"), (1, "mage_recruit"), ...]
),
```

**Section 2.5** - "Create Directory Structure" **EXPANDED**:

**Added Actor Sprite Assets**:
- `assets/sprites/npcs_town.png` - 4x4 grid (128x192)
- `assets/sprites/monsters_basic.png` - 4x4 grid (128x192)
- `assets/sprites/monsters_advanced.png` - 4x4 grid (128x192)
- `assets/sprites/recruitables.png` - 4x2 grid (128x96)

### Phase 3: Sprite Rendering Integration

**Status**: **REFINED** (Native Billboard System)

**Section 3.1** - Changed from "Add Sprite3d Plugin" to "Implement Billboard Component and System"

**New Billboard Component**:
```rust
/// Component that makes an entity face the camera (billboard effect)
#[derive(Component)]
pub struct Billboard {
    /// Lock Y-axis rotation (true for characters standing upright)
    pub lock_y: bool,
}
```

**New Billboard System**:
```rust
/// System that updates billboard entities to face the camera
pub fn update_billboards(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut billboard_query: Query<(&mut Transform, &GlobalTransform, &Billboard)>,
) {
    // Rotate entities to face camera
    // lock_y=true: Only Y-axis rotation (characters stay upright)
    // lock_y=false: Full rotation (particles, effects)
}
```

**Section 3.2** - "Create Sprite Rendering Components" **EXPANDED**:

**New Components**:
- `TileSprite`: For tile-based sprites
- `ActorSprite`: For character sprites (NPCs, Monsters, Recruitables)
- `AnimatedSprite`: For animated entities

**ActorType enum**:
```rust
pub enum ActorType {
    Npc,
    Monster,
    Recruitable,
}
```

**Section 3.3** - "Implement Sprite Spawning Functions" **NEW**:

**Constants**:
```rust
const PIXELS_PER_METER: f32 = 128.0;  // 128 pixels = 1 world unit
```

**Spawn Functions**:

1. `spawn_sprite_tile()` - For terrain/wall/decoration tiles
2. `spawn_actor_sprite()` - **NEW** - For NPCs, Monsters, Recruitables

**spawn_actor_sprite() specification**:
```rust
fn spawn_actor_sprite(
    commands: &mut Commands,
    sprite_assets: &mut SpriteAssets,
    position: Position,
    sheet_path: &str,
    sprite_index: u32,
    actor_type: ActorType,
    map_id: MapId,
) -> Entity {
    // Create PbrBundle with:
    // - Rectangle mesh (quad)
    // - StandardMaterial (alpha blend, texture)
    // - Transform (bottom-centered, scaled by PIXELS_PER_METER)
    // - Billboard { lock_y: true }
    // - ActorSprite component
}
```

**Key Implementation Details**:
- Sprites use `PbrBundle` (not external Sprite3d)
- Mesh: `Rectangle` sized to sprite dimensions
- Material: `StandardMaterial` with texture and alpha blend
- Transform: Bottom-centered (`y = height / (2.0 * PIXELS_PER_METER)`)
- Billboard: Y-locked to keep upright

**Section 3.5** - "Update NPC/Monster Spawning" **NEW**:

**NPC Spawning**:
```rust
// Replace cuboid placeholders with sprite billboards
for resolved_npc in resolved_npcs.iter() {
    spawn_actor_sprite(
        &mut commands,
        &mut sprite_assets,
        resolved_npc.position,
        "npcs_town",
        determine_npc_sprite_index(&resolved_npc),
        ActorType::Npc,
        map_id,
    );
}
```

**Monster Spawning**:
```rust
// Spawn monster sprites
for monster in monsters.iter() {
    spawn_actor_sprite(
        &mut commands,
        &mut sprite_assets,
        monster.position,
        determine_monster_sheet(&monster.monster_type),
        determine_monster_sprite_index(&monster.monster_type),
        ActorType::Monster,
        map_id,
    );
}
```

**Recruitable Spawning**:
```rust
// Recruitables are also actors - use sprite system
for (position, event) in map.events.iter() {
    if let MapEvent::RecruitableCharacter { name, .. } = event {
        spawn_actor_sprite(
            &mut commands,
            &mut sprite_assets,
            *position,
            "recruitables",
            hash_name_to_sprite_index(name, 8),
            ActorType::Recruitable,
            map_id,
        );
    }
}
```

**Section 3.7** - "Testing Requirements" **EXPANDED**:

**Added Tests** (14 total, up from 7):
- `test_billboard_component_created()`
- `test_update_billboards_system()`
- `test_billboard_lock_y_preserves_upright()`
- `test_sprite_tile_spawns_pbr_bundle()`
- `test_actor_sprite_spawns_with_billboard()`
- `test_npc_sprite_replaces_cuboid()`
- `test_monster_sprite_rendering()`
- `test_recruitable_sprite_rendering()`
- Plus original 7 tile sprite tests

### Phase 4: Sprite Asset Creation Guide

**Updated** to include actor sprite guidelines:
- Character sprites facing forward
- Bottom-centered anchor point
- 32x48 pixel character sprites

**Added sprite sheets**:
- `npcs_town.png`
- `monsters_basic.png`
- `monsters_advanced.png`
- `recruitables.png`

### Overall Success Criteria

**Added**:
- ✅ All actors (NPCs, Monsters, Recruitables) render as billboard sprites
- ✅ Character sprites properly centered at bottom (feet on ground)
- ✅ Billboard system keeps characters upright (Y-axis locked)
- ✅ Billboard update system optimized for 100+ actor sprites

### Dependencies

**Changed**:
- **Removed**: `bevy_sprite3d` external dependency
- **Using**: Native `bevy::pbr` and `bevy::render` only

### Timeline Estimate

**Updated**:
- Phase 2: 5-6 hours (up from 4-5) - More complexity with native PBR
- Phase 3: 8-10 hours (up from 6-8) - Actor sprite integration
- Total: 25-32 hours (up from 21-28)

### Implementation Order

**Added Section**: "Implementation Order Alignment"

```
Per plan_updates_review.md Section 3:

1. Execute sprite_support_implementation_plan.md FIRST
2. Then execute procedural_meshes_implementation_plan.md (trees, signs, portals only)

This ensures character rendering is unified under the sprite system
before implementing environmental procedural meshes.
```

---

## Conflict Resolution

| Feature          | Old (Procedural) | Old (Sprites)    | **New Unified Approach**                |
|------------------|------------------|------------------|-----------------------------------------|
| Terrain/Floor    | 3D Mesh          | N/A              | **3D Mesh** (Existing Map System)       |
| Trees            | Procedural       | N/A              | **Procedural Mesh** (Cylinder + Sphere) |
| Signs/Portals    | Procedural       | Sprite           | **Procedural Mesh** (Torus, Post+Board) |
| NPCs             | Capsule + Sphere | Sprite           | **Sprite** (PbrBundle + Billboard)      |
| Monsters         | N/A              | Sprite           | **Sprite** (PbrBundle + Billboard)      |
| Recruitables     | Glowing Cylinder | Sprite           | **Sprite** (PbrBundle + Billboard)      |

---

## Verification Checklist

After implementing both plans, verify:

### Procedural Meshes Plan

- [ ] Trees render with brown trunk and green foliage
- [ ] Signs render with post and board structure
- [ ] Portals render as purple glowing torus rings
- [ ] **No NPC procedural meshes exist**
- [ ] **No recruitable glowing cylinders exist**
- [ ] ProceduralMeshCache has 5 fields (tree×2, portal×1, sign×2)
- [ ] 9 unit tests pass

### Sprite Support Plan

- [ ] Billboard component and system implemented
- [ ] NPCs render as billboard sprites (not cuboids)
- [ ] Monsters render with sprite sheets
- [ ] Recruitables render as billboard sprites
- [ ] All actor sprites face camera
- [ ] All actor sprites stay upright (Y-axis locked)
- [ ] Sprite materials use PbrBundle with alpha blend
- [ ] No `bevy_sprite3d` dependency in Cargo.toml
- [ ] 14+ actor-related tests pass

### Integration

- [ ] NPCs spawn as sprites in `spawn_map_markers()`
- [ ] Monsters spawn as sprites
- [ ] Recruitables spawn as sprites (not in procedural_meshes)
- [ ] Trees spawn as procedural meshes
- [ ] Signs spawn as procedural meshes
- [ ] Portals spawn as procedural meshes
- [ ] No conflicts between systems
- [ ] Both rendering approaches coexist on same map

---

## File Modification Summary

### Files Modified in Procedural Meshes Plan

- `docs/explanation/procedural_meshes_implementation_plan.md` - Updated
- `src/game/systems/procedural_meshes.rs` - Will NOT include NPC/recruitable functions
- `src/game/systems/map.rs` - Sign/Portal spawning only

### Files Modified in Sprite Support Plan

- `docs/explanation/sprite_support_implementation_plan.md` - Updated
- `src/game/resources/sprite_assets.rs` - New, native PBR approach
- `src/game/components/billboard.rs` - New
- `src/game/systems/map.rs` - Actor sprite spawning
- `data/sprite_sheets.ron` - Includes actor entries
- `Cargo.toml` - NO changes (no external dependencies)

### New Assets Required

**Procedural**: None (pure geometry)

**Sprites**:
- `assets/sprites/npcs_town.png`
- `assets/sprites/monsters_basic.png`
- `assets/sprites/monsters_advanced.png`
- `assets/sprites/recruitables.png`

---

## References

- `plan_updates_review.md` - Original change request document
- `procedural_meshes_implementation_plan.md` - Updated plan
- `sprite_support_implementation_plan.md` - Updated plan
- `docs/reference/architecture.md` - Architecture reference
- `AGENTS.md` - Development rules

---

**End of Update Summary**
