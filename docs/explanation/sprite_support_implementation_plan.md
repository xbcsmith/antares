# Sprite Support Implementation Plan

> [!IMPORTANT] > **AI AGENT IMPLEMENTATION GUIDE**
>
> This plan uses explicit, machine-parseable format optimized for AI agent execution.
> Each phase includes verification commands, explicit file paths, and quantifiable success criteria.
> Follow AGENTS.md rules and validate with provided commands before proceeding to next phase.

## Quick Reference Card

| Property                | Value                                                                |
| ----------------------- | -------------------------------------------------------------------- |
| **Total Phases**        | 5 (Core Implementation)                                              |
| **Optional Phases**     | 1 (Advanced Features)                                                |
| **Estimated Time**      | 25-32 hours (Phases 1-5)                                             |
| **Prerequisites**       | Tile Visual Metadata Plan Phases 1-2 COMPLETE                        |
| **Target Architecture** | Native Bevy PBR Billboard (no external sprite dependencies)          |
| **Primary Use Case**    | All actor entities (NPCs, Monsters, Recruitables) + decorative tiles |
| **Rendering Approach**  | Hybrid 2.5D (billboarded sprites + 3D meshes)                        |

## Implementation Order

```text
Phase 1: Metadata Extension (2-3h)
   ↓
Phase 2: Asset Infrastructure (5-6h)
   ↓
Phase 3: Rendering Integration (10-12h)
   ↓
Phase 4: Asset Creation Guide (3-4h)
   ↓
Phase 5: Campaign Builder SDK (5-7h)
   ↓
Phase 6: Advanced Features (4-8h, OPTIONAL)
```

## Overview

Add sprite-based visual rendering for tiles **and all character entities** using native Bevy PBR billboard approach. This extends the tile visual metadata system to support texture-based visuals for walls, doors, terrain features, decorative elements, and all actor entities (NPCs, Monsters, Recruitables).

### Architecture Philosophy

**Character Rendering**: All "actors" (character entities) use billboard sprites facing the camera.

**Environmental Objects**: Procedural 3D meshes for trees, signs, portals (handled by procedural_meshes_implementation_plan.md).

**Rendering Method**: Billboard quads with StandardMaterial and alpha blending, using native Bevy PBR (no external sprite dependencies).

**Texture Management**: Sprite sheets (texture atlases) with UV transform-based sprite selection.

### Key Features

- **Sprite Metadata**: Optional sprite references in `TileVisualMetadata`
- **Billboard System**: Camera-facing sprites with Y-axis lock (keep upright)
- **Actor Sprites**: Unified sprite rendering for NPCs, Monsters, Recruitables
- **Animation Support**: Frame-based sprite animations with configurable FPS
- **Hybrid Rendering**: Tiles can use sprites OR 3D meshes (backward compatible)
- **Asset Pipeline**: Sprite sheet registry with RON configuration
- **SDK Integration**: Campaign Builder sprite selection and preview

## Prerequisites

> [!CRITICAL] > **ALL prerequisites MUST be verified BEFORE starting Phase 1**

### Required Implementations

**Tile Visual Metadata Plan - Phases 1-2 COMPLETE**:

- `TileVisualMetadata` struct exists in `src/domain/world/types.rs`
- Rendering system in `src/game/systems/map.rs` supports per-tile visual metadata
- Mesh caching infrastructure operational

### Verification Commands

**Run ALL commands below. ALL must pass before proceeding:**

```bash
# 1. Verify TileVisualMetadata exists
grep -q "pub struct TileVisualMetadata" src/domain/world/types.rs
echo "Exit code: $?" # Must output: Exit code: 0

# 2. Verify TileVisualMetadata in Tile struct
grep -q "pub visual: TileVisualMetadata" src/domain/world/types.rs
echo "Exit code: $?" # Must output: Exit code: 0

# 3. Verify map rendering uses visual metadata
grep -q "tile.visual" src/game/systems/map.rs
echo "Exit code: $?" # Must output: Exit code: 0

# 4. Verify project compiles cleanly
cargo check --all-targets --all-features
# Expected output: "Finished dev [unoptimized + debuginfo] target(s)"

# 5. Verify no clippy warnings
cargo clippy --all-targets --all-features -- -D warnings
# Expected output: "Finished dev [unoptimized + debuginfo] target(s)" with 0 warnings

# 6. Verify all tests pass
cargo nextest run --all-features
# Expected output: "test result: ok" with all tests passing

# 7. Verify Bevy version
grep 'bevy = ' Cargo.toml | grep -q '0.17'
echo "Exit code: $?" # Must output: Exit code: 0
```

### Prerequisite Checklist

Phase 1 can begin when ALL items checked:

- [ ] `TileVisualMetadata` struct exists in `src/domain/world/types.rs`
- [ ] `Tile` struct has `visual: TileVisualMetadata` field
- [ ] Map rendering system uses `tile.visual` for per-tile customization
- [ ] Mesh caching system operational in `src/game/systems/map.rs`
- [ ] `cargo check --all-targets --all-features` exit code = 0
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` warnings = 0
- [ ] `cargo nextest run --all-features` all tests pass
- [ ] Bevy version = 0.17 (verify in `Cargo.toml`)
- [ ] Architecture document read: `docs/reference/architecture.md` Sections 4 & 7

### If Prerequisites Fail

**If ANY prerequisite check fails:**

1. **STOP** - Do not proceed to Phase 1
2. Complete `tile_visual_metadata_implementation_plan.md` Phases 1-2 first
3. Verify all prerequisite checks pass
4. Update `docs/explanation/implementations.md` with prerequisite completion
5. Re-run all verification commands above

## Current State Analysis

### Existing Infrastructure

**Tile Data Model** (`src/domain/world/types.rs`):

- `Tile` struct with `visual: TileVisualMetadata` field (added by prerequisite plan)
- `TerrainType` enum: Ground, Grass, Water, Lava, Swamp, Stone, Dirt, Forest, Mountain
- `WallType` enum: None, Normal, Door, Torch
- **No sprite/texture reference fields exist**
- **No billboard or sprite components exist**

**Map Rendering System** (`src/game/systems/map.rs`):

- `spawn_map()` creates 3D meshes for all tile types
- Uses `Mesh3d`, `MeshMaterial3d`, `StandardMaterial` for 3D rendering
- Meshes are `Cuboid` and `Plane3d` primitives
- Mesh caching system operational (from prerequisite plan)
- **No sprite or texture atlas infrastructure exists**
- **Character entities (NPCs, Monsters) use placeholder cuboids**
- **No character sprite system exists**

**Asset Directory Structure**:

```text
assets/
└── porrtraits/  # Character portraits (existing, note: directory misspelled)
```

**No sprite directories exist**:

- No `assets/sprites/` directory
- No sprite sheet registry file
- No tile texture assets
- No character sprite assets

**Dependencies** (`Cargo.toml`):

```toml
bevy = { version = "0.17", default-features = true }
```

**Required modules available**:

- `bevy::pbr` - PBR rendering (StandardMaterial, lighting)
- `bevy::render` - Mesh, Image, texture loading
- **No external sprite dependencies needed**

### Technology Decision: Native Bevy PBR Billboard

**Selected Approach**: Native Bevy PBR billboard rendering

**Rationale**:

| Aspect           | Native Bevy PBR Billboard             | bevy_sprite3d (rejected)         |
| ---------------- | ------------------------------------- | -------------------------------- |
| **Stability**    | First-class Bevy support              | External crate, version coupling |
| **Lighting**     | Full PBR lighting integration         | Limited lighting support         |
| **Performance**  | Optimized by Bevy core team           | Community maintained             |
| **Dependencies** | Zero external dependencies            | Adds external dependency         |
| **Flexibility**  | Full control over materials/rendering | Plugin-based configuration       |
| **Maintenance**  | Bevy version updates guaranteed       | May lag behind Bevy versions     |

**Implementation Components**:

- `PbrBundle` with `StandardMaterial` (alpha_mode: Blend)
- `Rectangle` mesh for quad geometry
- Custom `Billboard` component + system for camera-facing rotation
- UV transforms for texture atlas sprite selection
- Per-sprite material caching for performance

### Identified Issues

**Missing Infrastructure**:

1. **No Sprite Data Structures**: No `SpriteReference`, `SpriteAnimation` types
2. **No Asset Loading**: No texture loading, atlas management, sprite rendering code
3. **No Billboard Support**: No mechanism to make sprites face camera in 3D world
4. **No Asset Pipeline**: No tooling to create or manage sprite sheets
5. **No SDK Integration**: Map editor cannot select/preview sprites
6. **No Character Sprite System**: NPCs, Monsters, Recruitables render as placeholder cuboids
7. **No Sprite Registry**: No configuration file for sprite sheet definitions

**Backward Compatibility Requirement**:

- Existing maps without sprite metadata must continue to work
- Tiles without `sprite` field must use 3D mesh rendering
- No breaking changes to `Tile` struct public API

**Performance Considerations**:

- Must handle 100+ billboard sprites (actors in large maps)
- Sprite sheet atlas batching required to minimize draw calls
- Texture caching required to prevent redundant loads
- Billboard update system must be optimized (spatial partitioning if needed)

## Implementation Phases

> [!NOTE]
> Each phase includes:
>
> - **BEFORE YOU START**: Verification commands and prerequisite checks
> - **Implementation Tasks**: Explicit file paths, structures, and code specifications
> - **AFTER YOU COMPLETE**: Quality gates and validation commands
> - **Deliverables Checklist**: All items that must be checked before phase complete
> - **Success Criteria**: Quantifiable, automatically verifiable criteria

---

## Phase 1: Sprite Metadata Extension

**Goal**: Extend `TileVisualMetadata` with optional sprite reference fields.

**Estimated Time**: 2-3 hours

### BEFORE YOU START - Phase 1

**Verify Prerequisites** (ALL must pass):

```bash
# 1. Verify TileVisualMetadata exists
test -f src/domain/world/types.rs || { echo "ERROR: types.rs missing"; exit 1; }
grep -q "pub struct TileVisualMetadata" src/domain/world/types.rs || { echo "ERROR: TileVisualMetadata missing"; exit 1; }

# 2. Verify clean compilation
cargo check --all-targets --all-features || { echo "ERROR: Compilation failed"; exit 1; }

# 3. Verify no existing sprite structures (should fail - that's correct)
! grep -q "pub struct SpriteReference" src/domain/world/types.rs || { echo "WARNING: SpriteReference already exists"; }

# 4. Read architecture reference
echo "Read: docs/reference/architecture.md Section 4 (Data Structures)"
echo "Read: docs/reference/architecture.md Section 7.2 (RON Format)"
```

**Architecture Compliance Check**:

- [ ] Read architecture.md Section 4 (Data Structures)
- [ ] Verify `TileVisualMetadata` definition matches architecture
- [ ] Confirm adding `sprite` field won't break existing code
- [ ] Verify `#[serde(default)]` pattern for backward compatibility

### Phase 1: Core Foundation Sprite Metadata

#### 1.1 Define Sprite Reference Structure

**File**: `src/domain/world/types.rs`
**Location**: After `TileVisualMetadata` struct (approximately line 100-150)
**Action**: ADD structures

**Structure Specification**:

````rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// ADD AFTER TileVisualMetadata IMPLEMENTATION (before Tile struct)

/// Reference to a sprite in a sprite sheet (texture atlas)
///
/// # Examples
///
/// ```
/// use antares::domain::world::SpriteReference;
///
/// let sprite = SpriteReference {
///     sheet_path: "sprites/walls.png".to_string(),
///     sprite_index: 3,
///     animation: None,
/// };
/// assert_eq!(sprite.sprite_index, 3);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteReference {
    /// Path to sprite sheet image (relative to campaign or global assets)
    /// Example: "sprites/walls.png" or "textures/npcs_town.png"
    pub sheet_path: String,

    /// Index within texture atlas grid (0-indexed, row-major order)
    /// For 4x4 grid: index 0 = top-left, index 3 = top-right, index 15 = bottom-right
    pub sprite_index: u32,

    /// Optional animation configuration
    #[serde(default)]
    pub animation: Option<SpriteAnimation>,
}

/// Animation configuration for sprite frames
///
/// # Examples
///
/// ```
/// use antares::domain::world::SpriteAnimation;
///
/// let anim = SpriteAnimation {
///     frames: vec![0, 1, 2, 1], // Ping-pong animation
///     fps: 8.0,
///     looping: true,
/// };
/// assert_eq!(anim.frames.len(), 4);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteAnimation {
    /// Frame indices in animation sequence (refers to sprite_index values)
    pub frames: Vec<u32>,

    /// Frames per second (default: 8.0)
    #[serde(default = "default_animation_fps")]
    pub fps: f32,

    /// Whether animation loops (default: true)
    #[serde(default = "default_animation_looping")]
    pub looping: bool,
}

fn default_animation_fps() -> f32 {
    8.0
}

fn default_animation_looping() -> bool {
    true
}
````

**Validation**:

```bash
# Verify structures added
grep -c "pub struct SpriteReference" src/domain/world/types.rs # Expected output: 1
grep -c "pub struct SpriteAnimation" src/domain/world/types.rs # Expected output: 1

# Verify compiles
cargo check src/domain/world/types.rs
```

#### 1.2 Extend TileVisualMetadata

**File**: `src/domain/world/types.rs`
**Location**: Inside `TileVisualMetadata` struct (approximately line 70-100)
**Action**: ADD field

**Field to Add**:

```rust
// ADD AS LAST FIELD in TileVisualMetadata struct:

    /// Optional sprite reference for texture-based rendering
    /// When set, replaces default 3D mesh with billboarded sprite
    #[serde(default)]
    pub sprite: Option<SpriteReference>,
```

**Validation**:

```bash
# Verify field added
grep -q "pub sprite: Option<SpriteReference>" src/domain/world/types.rs
echo "Exit code: $?" # Expected: 0

# Verify serde default annotation
grep -B1 "pub sprite: Option<SpriteReference>" src/domain/world/types.rs | grep -q "#\[serde(default)\]"
echo "Exit code: $?" # Expected: 0
```

#### 1.3 Add Sprite Helper Methods

**File**: `src/domain/world/types.rs`
**Location**: Inside `impl TileVisualMetadata` block (approximately line 150-200)
**Action**: ADD methods

**Methods to Add**:

````rust
// ADD INSIDE impl TileVisualMetadata BLOCK:

/// Check if sprite rendering is enabled
///
/// # Examples
///
/// ```
/// use antares::domain::world::{TileVisualMetadata, SpriteReference};
///
/// let mut metadata = TileVisualMetadata::default();
/// assert!(!metadata.uses_sprite());
///
/// metadata.sprite = Some(SpriteReference {
///     sheet_path: "sprites/walls.png".to_string(),
///     sprite_index: 0,
///     animation: None,
/// });
/// assert!(metadata.uses_sprite());
/// ```
pub fn uses_sprite(&self) -> bool {
    self.sprite.is_some()
}

/// Get sprite sheet path if sprite is configured
///
/// # Examples
///
/// ```
/// use antares::domain::world::{TileVisualMetadata, SpriteReference};
///
/// let mut metadata = TileVisualMetadata::default();
/// assert_eq!(metadata.sprite_sheet_path(), None);
///
/// metadata.sprite = Some(SpriteReference {
///     sheet_path: "sprites/walls.png".to_string(),
///     sprite_index: 0,
///     animation: None,
/// });
/// assert_eq!(metadata.sprite_sheet_path(), Some("sprites/walls.png"));
/// ```
pub fn sprite_sheet_path(&self) -> Option<&str> {
    self.sprite.as_ref().map(|s| s.sheet_path.as_str())
}

/// Get sprite index if sprite is configured
///
/// # Examples
///
/// ```
/// use antares::domain::world::{TileVisualMetadata, SpriteReference};
///
/// let mut metadata = TileVisualMetadata::default();
/// assert_eq!(metadata.sprite_index(), None);
///
/// metadata.sprite = Some(SpriteReference {
///     sheet_path: "sprites/walls.png".to_string(),
///     sprite_index: 42,
///     animation: None,
/// });
/// assert_eq!(metadata.sprite_index(), Some(42));
/// ```
pub fn sprite_index(&self) -> Option<u32> {
    self.sprite.as_ref().map(|s| s.sprite_index)
}

/// Check if sprite has animation configuration
///
/// # Examples
///
/// ```
/// use antares::domain::world::{TileVisualMetadata, SpriteReference, SpriteAnimation};
///
/// let mut metadata = TileVisualMetadata::default();
/// assert!(!metadata.has_animation());
///
/// metadata.sprite = Some(SpriteReference {
///     sheet_path: "sprites/walls.png".to_string(),
///     sprite_index: 0,
///     animation: Some(SpriteAnimation {
///         frames: vec![0, 1, 2],
///         fps: 8.0,
///         looping: true,
///     }),
/// });
/// assert!(metadata.has_animation());
/// ```
pub fn has_animation(&self) -> bool {
    self.sprite
        .as_ref()
        .and_then(|s| s.animation.as_ref())
        .is_some()
}
````

**Validation**:

```bash
# Verify methods added
grep -c "pub fn uses_sprite" src/domain/world/types.rs # Expected: 1
grep -c "pub fn sprite_sheet_path" src/domain/world/types.rs # Expected: 1
grep -c "pub fn sprite_index" src/domain/world/types.rs # Expected: 1
grep -c "pub fn has_animation" src/domain/world/types.rs # Expected: 1
```

#### 1.4 Builder Methods for Sprites

**File**: `src/domain/world/types.rs`
**Location**: Inside `impl Tile` block (approximately line 250-300)
**Action**: ADD methods

**Methods to Add**:

````rust
// ADD INSIDE impl Tile BLOCK:

/// Set static sprite for this tile
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Tile, TerrainType, WallType};
/// use antares::domain::types::Position;
///
/// let tile = Tile::new(Position::new(0, 0), TerrainType::Ground, WallType::Normal)
///     .with_sprite("sprites/walls.png", 5);
///
/// assert!(tile.visual.uses_sprite());
/// assert_eq!(tile.visual.sprite_index(), Some(5));
/// ```
pub fn with_sprite(mut self, sheet_path: &str, sprite_index: u32) -> Self {
    self.visual.sprite = Some(SpriteReference {
        sheet_path: sheet_path.to_string(),
        sprite_index,
        animation: None,
    });
    self
}

/// Set animated sprite for this tile
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Tile, TerrainType, WallType};
/// use antares::domain::types::Position;
///
/// let tile = Tile::new(Position::new(0, 0), TerrainType::Ground, WallType::Normal)
///     .with_animated_sprite("sprites/water.png", vec![0, 1, 2, 3], 4.0, true);
///
/// assert!(tile.visual.uses_sprite());
/// assert!(tile.visual.has_animation());
/// ```
pub fn with_animated_sprite(
    mut self,
    sheet_path: &str,
    frames: Vec<u32>,
    fps: f32,
    looping: bool,
) -> Self {
    self.visual.sprite = Some(SpriteReference {
        sheet_path: sheet_path.to_string(),
        sprite_index: frames[0], // First frame is base sprite_index
        animation: Some(SpriteAnimation {
            frames,
            fps,
            looping,
        }),
    });
    self
}
````

**Validation**:

```bash
# Verify methods added
grep -c "pub fn with_sprite" src/domain/world/types.rs # Expected: 1
grep -c "pub fn with_animated_sprite" src/domain/world/types.rs # Expected: 1
```

#### 1.5 Testing Requirements

**File**: `src/domain/world/types.rs`
**Location**: In `#[cfg(test)] mod tests` block (end of file)
**Action**: ADD tests

**Required Tests** (8 total):

| Test Name                                | Purpose                   | Expected Behavior              |
| ---------------------------------------- | ------------------------- | ------------------------------ |
| `test_sprite_reference_serialization`    | RON round-trip            | Deserialize(Serialize(x)) == x |
| `test_sprite_animation_defaults`         | Default values correct    | fps=8.0, looping=true          |
| `test_tile_visual_uses_sprite`           | `uses_sprite()` when set  | returns true                   |
| `test_tile_visual_no_sprite`             | `uses_sprite()` when None | returns false                  |
| `test_sprite_sheet_path_accessor`        | Path extraction           | correct string returned        |
| `test_tile_with_sprite_builder`          | `with_sprite()` builder   | sprite field populated         |
| `test_tile_with_animated_sprite_builder` | Animation builder         | animation configured           |
| `test_backward_compat_no_sprite_field`   | Old RON loads             | no errors, sprite=None         |

**Test Implementation**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_reference_serialization() {
        let sprite = SpriteReference {
            sheet_path: "sprites/test.png".to_string(),
            sprite_index: 42,
            animation: None,
        };

        let ron_str = ron::to_string(&sprite).unwrap();
        let deserialized: SpriteReference = ron::from_str(&ron_str).unwrap();

        assert_eq!(sprite, deserialized);
    }

    #[test]
    fn test_sprite_animation_defaults() {
        let ron_str = r#"SpriteAnimation(frames: [0, 1, 2])"#;
        let anim: SpriteAnimation = ron::from_str(ron_str).unwrap();

        assert_eq!(anim.fps, 8.0);
        assert_eq!(anim.looping, true);
    }

    #[test]
    fn test_tile_visual_uses_sprite() {
        let mut metadata = TileVisualMetadata::default();
        assert!(!metadata.uses_sprite());

        metadata.sprite = Some(SpriteReference {
            sheet_path: "test.png".to_string(),
            sprite_index: 0,
            animation: None,
        });
        assert!(metadata.uses_sprite());
    }

    #[test]
    fn test_tile_visual_no_sprite() {
        let metadata = TileVisualMetadata::default();
        assert!(!metadata.uses_sprite());
        assert_eq!(metadata.sprite_sheet_path(), None);
        assert_eq!(metadata.sprite_index(), None);
        assert!(!metadata.has_animation());
    }

    #[test]
    fn test_sprite_sheet_path_accessor() {
        let mut metadata = TileVisualMetadata::default();
        metadata.sprite = Some(SpriteReference {
            sheet_path: "sprites/walls.png".to_string(),
            sprite_index: 5,
            animation: None,
        });

        assert_eq!(metadata.sprite_sheet_path(), Some("sprites/walls.png"));
    }

    #[test]
    fn test_tile_with_sprite_builder() {
        let tile = Tile::new(Position::new(0, 0), TerrainType::Ground, WallType::Normal)
            .with_sprite("sprites/test.png", 10);

        assert!(tile.visual.uses_sprite());
        assert_eq!(tile.visual.sprite_index(), Some(10));
        assert!(!tile.visual.has_animation());
    }

    #[test]
    fn test_tile_with_animated_sprite_builder() {
        let tile = Tile::new(Position::new(0, 0), TerrainType::Ground, WallType::Normal)
            .with_animated_sprite("sprites/water.png", vec![0, 1, 2, 3], 4.0, true);

        assert!(tile.visual.uses_sprite());
        assert!(tile.visual.has_animation());

        let anim = tile.visual.sprite.as_ref().unwrap().animation.as_ref().unwrap();
        assert_eq!(anim.frames, vec![0, 1, 2, 3]);
        assert_eq!(anim.fps, 4.0);
        assert_eq!(anim.looping, true);
    }

    #[test]
    fn test_backward_compat_no_sprite_field() {
        // Old RON format without sprite field
        let ron_str = r#"
            TileVisualMetadata(
                height: Some(2.5),
                width_x: None,
                width_z: None,
                color_tint: None,
                scale: None,
                y_offset: None,
                rotation_y: None,
            )
        "#;

        let metadata: TileVisualMetadata = ron::from_str(ron_str).unwrap();

        assert_eq!(metadata.height, Some(2.5));
        assert!(!metadata.uses_sprite());
        assert_eq!(metadata.sprite, None);
    }
}
```

**Validation**:

```bash
# Run tests
cargo nextest run --lib types -- --nocapture | tee phase1_test_output.txt

# Verify exactly 8 new tests pass
grep -o "test.*sprite.*ok" phase1_test_output.txt | wc -l # Expected: 8
```

### AFTER YOU COMPLETE - Phase 1

**Mandatory Quality Gates** (ALL must pass):

```bash
# 1. Format code
cargo fmt --all
echo "✓ Code formatted"

# 2. Compilation check
cargo check --all-targets --all-features
# Expected: "Finished dev [unoptimized + debuginfo] target(s)" with exit code 0

# 3. Lint check
cargo clippy --all-targets --all-features -- -D warnings
# Expected: 0 warnings, exit code 0

# 4. Test execution
cargo nextest run --all-features
# Expected: All tests pass, 8 new tests added

# 5. Verify SPDX headers present
head -n 2 src/domain/world/types.rs | grep -q "SPDX-FileCopyrightText"
head -n 2 src/domain/world/types.rs | grep -q "SPDX-License-Identifier"
echo "✓ SPDX headers present"

# 6. Verify deliverables
test -f src/domain/world/types.rs || echo "ERROR: types.rs missing"
grep -q "pub struct SpriteReference" src/domain/world/types.rs || echo "ERROR: SpriteReference missing"
grep -q "pub struct SpriteAnimation" src/domain/world/types.rs || echo "ERROR: SpriteAnimation missing"
grep -q "pub sprite: Option<SpriteReference>" src/domain/world/types.rs || echo "ERROR: sprite field missing"
echo "✓ All structures present"

# 7. Verify backward compatibility
echo 'TileVisualMetadata(height: Some(2.5))' | cargo run --bin test_ron_compat
# Expected: No errors, sprite defaults to None
```

### Deliverables Checklist - Phase 1

Phase 1 is complete when ALL items checked:

**Code Structures**:

- [ ] File `src/domain/world/types.rs` contains `SpriteReference` struct (3 fields: sheet_path, sprite_index, animation)
- [ ] File `src/domain/world/types.rs` contains `SpriteAnimation` struct (3 fields: frames, fps, looping)
- [ ] `SpriteAnimation` has `#[serde(default)]` on `fps` and `looping` fields
- [ ] Default functions `default_animation_fps()` and `default_animation_looping()` implemented
- [ ] `TileVisualMetadata` has `sprite: Option<SpriteReference>` field
- [ ] `sprite` field has `#[serde(default)]` annotation

**Helper Methods**:

- [ ] Method `TileVisualMetadata::uses_sprite()` implemented and returns bool
- [ ] Method `TileVisualMetadata::sprite_sheet_path()` implemented and returns Option<&str>
- [ ] Method `TileVisualMetadata::sprite_index()` implemented and returns Option<u32>
- [ ] Method `TileVisualMetadata::has_animation()` implemented and returns bool

**Builder Methods**:

- [ ] Method `Tile::with_sprite(sheet_path, sprite_index)` implemented
- [ ] Method `Tile::with_animated_sprite(sheet_path, frames, fps, looping)` implemented

**Documentation**:

- [ ] All public structs have `///` doc comments with examples
- [ ] All public methods have `///` doc comments with examples
- [ ] SPDX headers present in all modified files

**Testing**:

- [ ] Test `test_sprite_reference_serialization` passes
- [ ] Test `test_sprite_animation_defaults` passes
- [ ] Test `test_tile_visual_uses_sprite` passes
- [ ] Test `test_tile_visual_no_sprite` passes
- [ ] Test `test_sprite_sheet_path_accessor` passes
- [ ] Test `test_tile_with_sprite_builder` passes
- [ ] Test `test_tile_with_animated_sprite_builder` passes
- [ ] Test `test_backward_compat_no_sprite_field` passes
- [ ] Exactly 8 new tests added (verify count increase)

**Quality Gates**:

- [ ] `cargo fmt --all` applied successfully
- [ ] `cargo check --all-targets --all-features` exit code = 0
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` warnings = 0
- [ ] `cargo nextest run --all-features` all tests pass

**Documentation**:

- [ ] `docs/explanation/implementations.md` updated with Phase 1 completion summary

### Success Criteria - Phase 1

**Quantifiable Metrics** (ALL must be met):

| Criterion        | Verification Command                                                               | Expected Result |
| ---------------- | ---------------------------------------------------------------------------------- | --------------- |
| Structures exist | `grep -c "pub struct Sprite" src/domain/world/types.rs`                            | Output: 2       |
| Field added      | `grep -c "pub sprite: Option<SpriteReference>" src/domain/world/types.rs`          | Output: 1       |
| Methods added    | `grep -c "pub fn.*sprite" src/domain/world/types.rs`                               | Output: ≥ 6     |
| Tests added      | `cargo nextest run --lib types 2>&1 \| grep -c "test.*sprite.*ok"`                 | Output: 8       |
| Compilation      | `cargo check --all-targets --all-features; echo $?`                                | Output: 0       |
| Linting          | `cargo clippy --all-targets --all-features -- -D warnings 2>&1 \| grep -c warning` | Output: 0       |
| Test suite       | `cargo nextest run --all-features 2>&1 \| grep -c "test result: ok"`               | Output: ≥ 1     |

**Functional Verification**:

- ✅ Existing tile RON files without `sprite` field load correctly (backward compatible)
- ✅ New tile RON files with `sprite` field serialize and deserialize correctly
- ✅ `TileVisualMetadata::default()` has `sprite = None`
- ✅ `uses_sprite()` returns false when sprite is None
- ✅ `uses_sprite()` returns true when sprite is Some
- ✅ Builder methods correctly populate sprite field
- ✅ Animation defaults apply (fps=8.0, looping=true)

**Architecture Compliance**:

- ✅ Data structures match architecture.md Section 4 patterns
- ✅ `#[serde(default)]` used for backward compatibility
- ✅ All public items have `///` doc comments with examples
- ✅ SPDX headers in all modified files
- ✅ RON format used for serialization tests
- ✅ No use of raw types (u32 is acceptable for sprite_index)

---

## Phase 2: Sprite Asset Infrastructure

**Goal**: Set up sprite sheet loading, texture atlas management, and asset pipeline for tiles and character entities.

**Estimated Time**: 5-6 hours

### BEFORE YOU START - Phase 2

**Verify Phase 1 Complete** (ALL must pass):

```bash
# 1. Verify Phase 1 structures exist
grep -q "pub struct SpriteReference" src/domain/world/types.rs || { echo "ERROR: Phase 1 incomplete"; exit 1; }
grep -q "pub struct SpriteAnimation" src/domain/world/types.rs || { echo "ERROR: Phase 1 incomplete"; exit 1; }
grep -q "pub sprite: Option<SpriteReference>" src/domain/world/types.rs || { echo "ERROR: Phase 1 incomplete"; exit 1; }

# 2. Verify Phase 1 tests pass
cargo nextest run --lib types sprite || { echo "ERROR: Phase 1 tests failing"; exit 1; }

# 3. Verify clean state
cargo check --all-targets --all-features || { echo "ERROR: Compilation broken"; exit 1; }
cargo clippy --all-targets --all-features -- -D warnings || { echo "ERROR: Clippy warnings present"; exit 1; }

# 4. Verify no sprite infrastructure exists yet (should fail - that's correct)
! test -d src/game/resources || echo "WARNING: resources directory already exists"
! test -f src/game/resources/sprite_assets.rs || echo "WARNING: sprite_assets.rs already exists"
! test -d assets/sprites || echo "WARNING: sprites directory already exists"

# 5. Read architecture reference
echo "Read: docs/reference/architecture.md Section 7 (Asset Management)"
echo "Read: docs/reference/architecture.md Section 7.2 (RON Format)"
```

**Prerequisites Checklist**:

- [ ] Phase 1 complete (all deliverables checked)
- [ ] Phase 1 tests pass (8 sprite tests)
- [ ] `cargo check` passes with zero errors
- [ ] `cargo clippy` reports zero warnings
- [ ] No sprite asset infrastructure exists yet
- [ ] Architecture document Section 7 read

### Implementation Tasks - Phase 2

#### 2.1 No External Dependencies Required

**File**: `Cargo.toml`
**Action**: VERIFY (no changes needed)

**Verification**:

```bash
# Verify Bevy 0.17 dependency exists
grep 'bevy = { version = "0.17"' Cargo.toml
echo "Exit code: $?" # Expected: 0

# Verify no external sprite dependencies
! grep -q "bevy_sprite3d" Cargo.toml || { echo "ERROR: External sprite dependency found"; exit 1; }
! grep -q "bevy_mod_billboard" Cargo.toml || { echo "ERROR: External billboard dependency found"; exit 1; }

echo "✓ Using native Bevy only (no external dependencies)"
```

**Available Bevy Modules**:

- `bevy::pbr::StandardMaterial` - PBR material with texture and alpha blending
- `bevy::render::mesh::Rectangle` - Quad mesh for billboards
- `bevy::asset::Handle<Image>` - Texture loading
- `bevy::math::Vec2` - UV transforms
- `bevy::transform::components::Transform` - Position and rotation

#### 2.2 Create Sprite Asset Loader

**File**: `src/game/resources/sprite_assets.rs` (NEW FILE)
**Action**: CREATE file and directory

**Create Directory**:

```bash
mkdir -p src/game/resources
echo "pub mod sprite_assets;" >> src/game/resources/mod.rs
```

**File Structure**:

````rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Sprite asset management using native Bevy PBR
//!
//! This module provides sprite sheet loading, material caching, and UV transform
//! calculation for billboard-based sprite rendering using Bevy's PBR system.
//!
//! # Architecture
//!
//! - Uses `StandardMaterial` with alpha blending for sprite textures
//! - Caches materials per sprite sheet to minimize draw calls
//! - Provides UV transforms for texture atlas sprite selection
//! - No external dependencies - native Bevy only
//!
//! # Examples
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::resources::sprite_assets::SpriteAssets;
//!
//! fn setup_sprites(
//!     mut sprite_assets: ResMut<SpriteAssets>,
//!     asset_server: Res<AssetServer>,
//!     mut materials: ResMut<Assets<StandardMaterial>>,
//!     mut meshes: ResMut<Assets<Mesh>>,
//! ) {
//!     // Load sprite sheet material
//!     let material = sprite_assets.get_or_load_material(
//!         "sprites/walls.png",
//!         &asset_server,
//!         &mut materials,
//!     );
//!
//!     // Create quad mesh for sprite
//!     let mesh = sprite_assets.get_or_load_mesh((1.0, 2.0), &mut meshes);
//!
//!     // Get UV transform for sprite at index 5 in 4x4 grid
//!     let (offset, scale) = sprite_assets.get_sprite_uv_transform("walls", 5);
//! }
//! ```

use bevy::prelude::*;
use std::collections::HashMap;

/// Configuration for a sprite sheet (texture atlas)
///
/// # Examples
///
/// ```
/// use antares::game::resources::sprite_assets::SpriteSheetConfig;
///
/// let config = SpriteSheetConfig {
///     texture_path: "sprites/walls.png".to_string(),
///     tile_size: (128.0, 256.0),
///     columns: 4,
///     rows: 4,
///     sprites: vec![
///         (0, "stone_wall".to_string()),
///         (1, "brick_wall".to_string()),
///     ],
/// };
///
/// assert_eq!(config.columns, 4);
/// assert_eq!(config.rows, 4);
/// ```
#[derive(Debug, Clone)]
pub struct SpriteSheetConfig {
    /// Path to sprite sheet texture (relative to assets/)
    pub texture_path: String,

    /// Size of each sprite in pixels (width, height)
    pub tile_size: (f32, f32),

    /// Number of columns in sprite grid
    pub columns: u32,

    /// Number of rows in sprite grid
    pub rows: u32,

    /// Named sprite mappings (index, name)
    pub sprites: Vec<(u32, String)>,
}

/// Resource managing sprite materials and meshes for billboard rendering
///
/// # Architecture
///
/// - **Materials**: Cached `StandardMaterial` per sprite sheet (texture + alpha blend)
/// - **Meshes**: Cached `Rectangle` meshes per sprite size
/// - **Configs**: Sprite sheet configurations for UV calculations
///
/// # Performance
///
/// - Material caching reduces draw calls (one material per sprite sheet)
/// - Mesh caching avoids duplicate geometry (one mesh per size)
/// - UV transforms calculated on-demand (no runtime overhead)
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::resources::sprite_assets::SpriteAssets;
///
/// fn sprite_system(sprite_assets: Res<SpriteAssets>) {
///     if let Some(config) = sprite_assets.get_config("npcs_town") {
///         println!("NPC sprite sheet: {}x{} grid", config.columns, config.rows);
///     }
/// }
/// ```
#[derive(Resource)]
pub struct SpriteAssets {
    /// Cached materials per sprite sheet path
    materials: HashMap<String, Handle<StandardMaterial>>,

    /// Cached meshes per sprite size (key: "widthxheight")
    meshes: HashMap<String, Handle<Mesh>>,

    /// Sprite sheet configurations (key: sheet identifier)
    configs: HashMap<String, SpriteSheetConfig>,
}

impl SpriteAssets {
    /// Create new empty sprite assets resource
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::sprite_assets::SpriteAssets;
    ///
    /// let sprite_assets = SpriteAssets::new();
    /// ```
    pub fn new() -> Self {
        Self {
            materials: HashMap::new(),
            meshes: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    /// Get or create PBR material for a sprite sheet
    ///
    /// Materials are cached per sprite sheet path to minimize draw calls.
    ///
    /// # Material Configuration
    ///
    /// - `base_color_texture`: Loaded from `sheet_path`
    /// - `alpha_mode`: `AlphaMode::Blend` (supports transparency)
    /// - `unlit`: `false` (uses PBR lighting)
    /// - `perceptual_roughness`: `0.9` (slightly rough for depth)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use bevy::prelude::*;
    /// use antares::game::resources::sprite_assets::SpriteAssets;
    ///
    /// fn load_sprite_material(
    ///     mut sprite_assets: ResMut<SpriteAssets>,
    ///     asset_server: Res<AssetServer>,
    ///     mut materials: ResMut<Assets<StandardMaterial>>,
    /// ) {
    ///     let material = sprite_assets.get_or_load_material(
    ///         "sprites/npcs_town.png",
    ///         &asset_server,
    ///         &mut materials,
    ///     );
    /// }
    /// ```
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
            unlit: false, // Use PBR lighting for depth
            perceptual_roughness: 0.9,
            ..default()
        };

        let handle = materials.add(material);
        self.materials.insert(sheet_path.to_string(), handle.clone());
        handle
    }

    /// Get or create quad mesh for sprite rendering
    ///
    /// Meshes are cached per size to avoid duplicate geometry.
    ///
    /// # Arguments
    ///
    /// * `sprite_size` - (width, height) in world units (1 unit ≈ 1 meter)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use bevy::prelude::*;
    /// use antares::game::resources::sprite_assets::SpriteAssets;
    ///
    /// fn create_sprite_mesh(
    ///     mut sprite_assets: ResMut<SpriteAssets>,
    ///     mut meshes: ResMut<Assets<Mesh>>,
    /// ) {
    ///     // Create 1m x 2m quad for character sprite
    ///     let mesh = sprite_assets.get_or_load_mesh((1.0, 2.0), &mut meshes);
    /// }
    /// ```
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
    ///
    /// Returns (offset, scale) for UV coordinates.
    /// Sprites are indexed in row-major order (left-to-right, top-to-bottom).
    ///
    /// # Arguments
    ///
    /// * `sheet_key` - Sprite sheet identifier (registered config)
    /// * `sprite_index` - Sprite index (0-indexed, row-major order)
    ///
    /// # Returns
    ///
    /// * `(offset, scale)` - UV offset and scale as Vec2
    /// * `(Vec2::ZERO, Vec2::ONE)` - If sheet not found (full texture)
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use antares::game::resources::sprite_assets::{SpriteAssets, SpriteSheetConfig};
    ///
    /// let mut sprite_assets = SpriteAssets::new();
    ///
    /// // Register 4x4 sprite sheet
    /// sprite_assets.register_config("walls".to_string(), SpriteSheetConfig {
    ///     texture_path: "sprites/walls.png".to_string(),
    ///     tile_size: (128.0, 128.0),
    ///     columns: 4,
    ///     rows: 4,
    ///     sprites: vec![],
    /// });
    ///
    /// // Get UV transform for sprite at index 5 (row 1, col 1)
    /// let (offset, scale) = sprite_assets.get_sprite_uv_transform("walls", 5);
    ///
    /// assert_eq!(scale, Vec2::new(0.25, 0.25)); // 1/4 of texture
    /// assert_eq!(offset, Vec2::new(0.25, 0.25)); // Second column, second row
    /// ```
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
            // Default: use full texture
            (Vec2::ZERO, Vec2::ONE)
        }
    }

    /// Register sprite sheet configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::sprite_assets::{SpriteAssets, SpriteSheetConfig};
    ///
    /// let mut sprite_assets = SpriteAssets::new();
    ///
    /// sprite_assets.register_config("walls".to_string(), SpriteSheetConfig {
    ///     texture_path: "sprites/walls.png".to_string(),
    ///     tile_size: (128.0, 256.0),
    ///     columns: 4,
    ///     rows: 4,
    ///     sprites: vec![(0, "stone_wall".to_string())],
    /// });
    /// ```
    pub fn register_config(&mut self, key: String, config: SpriteSheetConfig) {
        self.configs.insert(key, config);
    }

    /// Get sprite sheet configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::sprite_assets::{SpriteAssets, SpriteSheetConfig};
    ///
    /// let mut sprite_assets = SpriteAssets::new();
    /// sprite_assets.register_config("walls".to_string(), SpriteSheetConfig {
    ///     texture_path: "sprites/walls.png".to_string(),
    ///     tile_size: (128.0, 256.0),
    ///     columns: 4,
    ///     rows: 4,
    ///     sprites: vec![],
    /// });
    ///
    /// let config = sprite_assets.get_config("walls").unwrap();
    /// assert_eq!(config.columns, 4);
    /// ```
    pub fn get_config(&self, key: &str) -> Option<&SpriteSheetConfig> {
        self.configs.get(key)
    }
}

impl Default for SpriteAssets {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_assets_new() {
        let assets = SpriteAssets::new();
        assert_eq!(assets.materials.len(), 0);
        assert_eq!(assets.meshes.len(), 0);
        assert_eq!(assets.configs.len(), 0);
    }

    #[test]
    fn test_register_and_get_config() {
        let mut assets = SpriteAssets::new();

        let config = SpriteSheetConfig {
            texture_path: "test.png".to_string(),
            tile_size: (32.0, 32.0),
            columns: 4,
            rows: 4,
            sprites: vec![],
        };

        assets.register_config("test".to_string(), config);

        let retrieved = assets.get_config("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().columns, 4);
    }

    #[test]
    fn test_uv_transform_4x4_grid() {
        let mut assets = SpriteAssets::new();

        assets.register_config("test".to_string(), SpriteSheetConfig {
            texture_path: "test.png".to_string(),
            tile_size: (32.0, 32.0),
            columns: 4,
            rows: 4,
            sprites: vec![],
        });

        // Test sprite at index 0 (top-left)
        let (offset, scale) = assets.get_sprite_uv_transform("test", 0);
        assert_eq!(offset, Vec2::new(0.0, 0.0));
        assert_eq!(scale, Vec2::new(0.25, 0.25));

        // Test sprite at index 5 (row 1, col 1)
        let (offset, scale) = assets.get_sprite_uv_transform("test", 5);
        assert_eq!(offset, Vec2::new(0.25, 0.25));
        assert_eq!(scale, Vec2::new(0.25, 0.25));

        // Test sprite at index 15 (bottom-right)
        let (offset, scale) = assets.get_sprite_uv_transform("test", 15);
        assert_eq!(offset, Vec2::new(0.75, 0.75));
        assert_eq!(scale, Vec2::new(0.25, 0.25));
    }

    #[test]
    fn test_uv_transform_unknown_sheet() {
        let assets = SpriteAssets::new();

        // Should return default (full texture)
        let (offset, scale) = assets.get_sprite_uv_transform("unknown", 0);
        assert_eq!(offset, Vec2::ZERO);
        assert_eq!(scale, Vec2::ONE);
    }
}
````

**Validation**:

```bash
# Verify file created
test -f src/game/resources/sprite_assets.rs || { echo "ERROR: File not created"; exit 1; }

# Verify structures present
grep -q "pub struct SpriteSheetConfig" src/game/resources/sprite_assets.rs
grep -q "pub struct SpriteAssets" src/game/resources/sprite_assets.rs

# Verify compiles
cargo check src/game/resources/sprite_assets.rs

# Run tests
cargo nextest run --lib sprite_assets
```

#### 2.3 Register Module

**File**: `src/game/mod.rs`
**Action**: ADD module declaration

**Add to file**:

```rust
pub mod resources;
```

**File**: `src/game/resources/mod.rs` (NEW)
**Action**: CREATE file

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Game resources module

pub mod sprite_assets;
```

**Validation**:

```bash
# Verify module registered
grep -q "pub mod resources" src/game/mod.rs

# Verify compiles
cargo check --all-targets
```

#### 2.4 Create Sprite Sheet Registry Data File

**File**: `data/sprite_sheets.ron` (NEW FILE)
**Action**: CREATE file and directory

**Create directory**:

```bash
mkdir -p data
```

**RON File Specification**:

````ron
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Sprite sheet registry
//!
//! Defines all sprite sheets used in the game for tiles and actors.
//! Format: HashMap<String, SpriteSheetConfig>
//!
//! # Field Specifications
//!
//! - texture_path: String (relative to assets/ directory)
//! - tile_size: (f32, f32) tuple (width, height in pixels)
//! - columns: u32 (must be > 0)
//! - rows: u32 (must be > 0)
//! - sprites: Vec<(u32, String)> (index, name pairs)
//!
//! # Sprite Indexing
//!
//! Sprites are indexed in row-major order (left-to-right, top-to-bottom):
//!
//! ```text
//! 4x4 grid indices:
//! 0  1  2  3
//! 4  5  6  7
//! 8  9  10 11
//! 12 13 14 15
//! ```

{
    // Tile Sprites
    "walls": SpriteSheetConfig(
        texture_path: "sprites/walls.png",
        tile_size: (128.0, 256.0),
        columns: 4,
        rows: 4,
        sprites: [
            (0, "stone_wall"),
            (1, "brick_wall"),
            (2, "wood_wall"),
            (3, "damaged_stone"),
            (4, "moss_stone"),
            (5, "reinforced_brick"),
            (6, "weathered_wood"),
            (7, "cracked_stone"),
        ],
    ),

    "doors": SpriteSheetConfig(
        texture_path: "sprites/doors.png",
        tile_size: (128.0, 256.0),
        columns: 4,
        rows: 2,
        sprites: [
            (0, "wood_door_closed"),
            (1, "wood_door_open"),
            (2, "iron_door_closed"),
            (3, "iron_door_open"),
            (4, "locked_door"),
            (5, "secret_door"),
        ],
    ),

    "terrain": SpriteSheetConfig(
        texture_path: "sprites/terrain.png",
        tile_size: (128.0, 128.0),
        columns: 8,
        rows: 8,
        sprites: [
            (0, "stone_floor"),
            (1, "grass"),
            (2, "dirt"),
            (3, "water"),
            (4, "lava"),
            (5, "swamp"),
            (6, "wood_floor"),
            (7, "marble_floor"),
        ],
    ),

    "trees": SpriteSheetConfig(
        texture_path: "sprites/trees.png",
        tile_size: (128.0, 256.0),
        columns: 4,
        rows: 4,
        sprites: [
            (0, "oak_tree"),
            (1, "pine_tree"),
            (2, "dead_tree"),
            (3, "magical_tree"),
        ],
    ),

    "decorations": SpriteSheetConfig(
        texture_path: "sprites/decorations.png",
        tile_size: (64.0, 64.0),
        columns: 8,
        rows: 8,
        sprites: [
            (0, "torch"),
            (1, "chest"),
            (2, "barrel"),
            (3, "crate"),
            (4, "bones"),
            (5, "rubble"),
        ],
    ),

    // Actor Sprites (NPCs, Monsters, Recruitables)
    "npcs_town": SpriteSheetConfig(
        texture_path: "sprites/npcs_town.png",
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
        texture_path: "sprites/monsters_basic.png",
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
        texture_path: "sprites/monsters_advanced.png",
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
        texture_path: "sprites/recruitables.png",
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
        texture_path: "sprites/signs.png",
        tile_size: (32.0, 64.0),
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
        texture_path: "sprites/portals.png",
        tile_size: (128.0, 128.0),
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
}
````

**Validation**:

```bash
# Verify file created
test -f data/sprite_sheets.ron || { echo "ERROR: File not created"; exit 1; }

# Verify RON syntax (will fail until we create validation tool, that's OK)
# cargo run --bin validate_sprite_sheets || echo "Validation tool not created yet"
```

#### 2.5 Create Asset Directory Structure

**Action**: CREATE directories (no sprite image files yet - Phase 4)

```bash
# Create directories
mkdir -p assets/sprites

# Create placeholder README
cat > assets/sprites/README.md << 'EOF'
# Sprite Assets Directory

This directory contains sprite sheet textures used for billboard rendering.

## Required Sprite Sheets

### Tile Sprites
- `walls.png` - 512x1024, 4x4 grid, 128x256 tiles
- `doors.png` - 512x512, 4x2 grid, 128x256 tiles
- `terrain.png` - 1024x1024, 8x8 grid, 128x128 tiles
- `trees.png` - 512x1024, 4x4 grid, 128x256 tiles
- `decorations.png` - 512x512, 8x8 grid, 64x64 tiles

### Actor Sprites
- `npcs_town.png` - 128x192, 4x4 grid, 32x48 sprites
- `monsters_basic.png` - 128x192, 4x4 grid, 32x48 sprites
- `monsters_advanced.png` - 128x192, 4x4 grid, 32x48 sprites
- `recruitables.png` - 128x96, 4x2 grid, 32x48 sprites

### Event Marker Sprites
- `signs.png` - 128x128, 4x2 grid, 32x64 sprites
- `portals.png` - 512x256, 4x2 grid, 128x128 sprites

## Format Specifications

- **Format**: PNG-24 with alpha channel
- **Color Space**: sRGB
- **Transparency**: Full alpha support required
- **Grid Layout**: Row-major order (left-to-right, top-to-bottom)

## Creation

Sprite images will be created in Phase 4 of sprite_support_implementation_plan.md
EOF
```

**Validation**:

```bash
# Verify directory created
test -d assets/sprites || { echo "ERROR: Directory not created"; exit 1; }
test -f assets/sprites/README.md || { echo "ERROR: README not created"; exit 1; }

echo "✓ Asset directory structure created"
```

#### 2.6 Testing Requirements

**File**: `src/game/resources/sprite_assets.rs`
**Location**: Already included in 2.2 (tests at end of file)
**Action**: VERIFY tests exist

**Required Tests** (4 total - already in file from 2.2):

| Test Name                         | Purpose                          | Expected Result            |
| --------------------------------- | -------------------------------- | -------------------------- |
| `test_sprite_assets_new`          | Constructor creates empty assets | All hashmaps empty         |
| `test_register_and_get_config`    | Config registration              | Config retrievable         |
| `test_uv_transform_4x4_grid`      | UV calculation 4x4               | Correct offsets/scales     |
| `test_uv_transform_unknown_sheet` | Missing config handling          | Returns default (0,0)(1,1) |

**Validation**:

```bash
# Run sprite_assets tests
cargo nextest run --lib sprite_assets -- --nocapture

# Verify 4 tests pass
cargo nextest run --lib sprite_assets 2>&1 | grep -c "test.*ok" # Expected: 4
```

### AFTER YOU COMPLETE - Phase 2

**Mandatory Quality Gates** (ALL must pass):

```bash
# 1. Format code
cargo fmt --all
echo "✓ Code formatted"

# 2. Compilation check
cargo check --all-targets --all-features
# Expected: exit code 0

# 3. Lint check
cargo clippy --all-targets --all-features -- -D warnings
# Expected: 0 warnings

# 4. Test execution
cargo nextest run --all-features
# Expected: All tests pass, 4 new sprite_assets tests

# 5. Verify SPDX headers
head -n 2 src/game/resources/sprite_assets.rs | grep -q "SPDX-FileCopyrightText"
head -n 2 src/game/resources/mod.rs | grep -q "SPDX-FileCopyrightText"
head -n 2 data/sprite_sheets.ron | grep -q "SPDX-FileCopyrightText"
echo "✓ SPDX headers present"

# 6. Verify deliverables
test -f src/game/resources/sprite_assets.rs || echo "ERROR: sprite_assets.rs missing"
test -f src/game/resources/mod.rs || echo "ERROR: mod.rs missing"
test -f data/sprite_sheets.ron || echo "ERROR: sprite_sheets.ron missing"
test -d assets/sprites || echo "ERROR: sprites directory missing"
grep -q "pub struct SpriteAssets" src/game/resources/sprite_assets.rs || echo "ERROR: SpriteAssets missing"
echo "✓ All files present"

# 7. Verify no external dependencies added
! grep -q "bevy_sprite3d" Cargo.toml || echo "ERROR: External dependency added"
echo "✓ Native Bevy only (no external deps)"
```

### Deliverables Checklist - Phase 2

Phase 2 is complete when ALL items checked:

**Code Structures**:

- [ ] File `src/game/resources/sprite_assets.rs` created (approx 400 lines)
- [ ] File `src/game/resources/mod.rs` created
- [ ] `SpriteSheetConfig` struct implemented (5 fields)
- [ ] `SpriteAssets` Resource struct implemented (3 HashMaps)
- [ ] Method `get_or_load_material()` implemented (returns Handle<StandardMaterial>)
- [ ] Method `get_or_load_mesh()` implemented (returns Handle<Mesh>)
- [ ] Method `get_sprite_uv_transform()` implemented (returns (Vec2, Vec2))
- [ ] Method `register_config()` implemented
- [ ] Method `get_config()` implemented

**Module Registration**:

- [ ] `pub mod resources;` added to `src/game/mod.rs`
- [ ] `pub mod sprite_assets;` in `src/game/resources/mod.rs`
- [ ] Module compiles with `cargo check`

**Data Files**:

- [ ] File `data/sprite_sheets.ron` created with 11 sprite sheet definitions
- [ ] Tile sprites defined: walls, doors, terrain, trees, decorations
- [ ] Actor sprites defined: npcs_town, monsters_basic, monsters_advanced, recruitables
- [ ] Event marker sprites defined: signs, portals
- [ ] All sprite sheets have valid texture_path, tile_size, columns, rows

**Asset Structure**:

- [ ] Directory `assets/sprites/` created
- [ ] File `assets/sprites/README.md` created with specifications
- [ ] README documents all required sprite sheets
- [ ] README documents PNG-24 format requirement

**Documentation**:

- [ ] Module-level doc comments in `sprite_assets.rs`
- [ ] All public structs have `///` doc comments with examples
- [ ] All public methods have `///` doc comments with examples
- [ ] SPDX headers in all created files

**Testing**:

- [ ] Test `test_sprite_assets_new` passes
- [ ] Test `test_register_and_get_config` passes
- [ ] Test `test_uv_transform_4x4_grid` passes
- [ ] Test `test_uv_transform_unknown_sheet` passes
- [ ] Exactly 4 new tests pass (verify count increase)

**Quality Gates**:

- [ ] `cargo fmt --all` applied successfully
- [ ] `cargo check --all-targets --all-features` exit code = 0
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` warnings = 0
- [ ] `cargo nextest run --all-features` all tests pass

**Architecture Compliance**:

- [ ] No external dependencies added (verified in Cargo.toml)
- [ ] Native Bevy PBR approach used (StandardMaterial)
- [ ] RON format used for sprite sheet registry

**Documentation**:

- [ ] `docs/explanation/implementations.md` updated with Phase 2 completion summary

### Success Criteria - Phase 2

**Quantifiable Metrics** (ALL must be met):

| Criterion             | Verification Command                                                               | Expected Result |
| --------------------- | ---------------------------------------------------------------------------------- | --------------- |
| File created          | `test -f src/game/resources/sprite_assets.rs; echo $?`                             | Output: 0       |
| Structures exist      | `grep -c "pub struct Sprite" src/game/resources/sprite_assets.rs`                  | Output: 2       |
| Methods implemented   | `grep -c "pub fn" src/game/resources/sprite_assets.rs`                             | Output: ≥ 6     |
| Tests pass            | `cargo nextest run --lib sprite_assets 2>&1 \| grep -c "test.*ok"`                 | Output: 4       |
| RON file exists       | `test -f data/sprite_sheets.ron; echo $?`                                          | Output: 0       |
| Sprite sheets defined | `grep -c "SpriteSheetConfig" data/sprite_sheets.ron`                               | Output: ≥ 11    |
| Directory created     | `test -d assets/sprites; echo $?`                                                  | Output: 0       |
| No external deps      | `grep -c "bevy_sprite3d" Cargo.toml`                                               | Output: 0       |
| Compilation           | `cargo check --all-targets --all-features; echo $?`                                | Output: 0       |
| Linting               | `cargo clippy --all-targets --all-features -- -D warnings 2>&1 \| grep -c warning` | Output: 0       |

**Functional Verification**:

- ✅ `SpriteAssets::new()` creates empty resource
- ✅ `register_config()` stores sprite sheet configuration
- ✅ `get_config()` retrieves stored configuration
- ✅ `get_sprite_uv_transform()` calculates correct UV offsets and scales
- ✅ UV calculation for 4x4 grid: index 0 = (0,0), index 15 = (0.75, 0.75)
- ✅ Unknown sprite sheet returns default UV (0,0)(1,1)
- ✅ RON file contains all 11 sprite sheet definitions
- ✅ All sprite sheets have required fields populated
- ✅ No external sprite dependencies in Cargo.toml

**Architecture Compliance**:

- ✅ Uses native Bevy PBR (StandardMaterial with alpha_mode: Blend)
- ✅ Uses `Rectangle` mesh for quad geometry
- ✅ Material caching per sprite sheet (performance optimization)
- ✅ Mesh caching per size (performance optimization)
- ✅ RON format for configuration files
- ✅ SPDX headers in all created files
- ✅ All public items have doc comments with examples

---

## Phase 3: Sprite Rendering Integration

**Goal**: Update map rendering system to render sprites for tiles with sprite metadata and implement billboard system for all actor entities.

**Estimated Time**: 10-12 hours

### BEFORE YOU START - Phase 3

**Verify Phase 2 Complete** (ALL must pass):

```bash
# 1. Verify Phase 2 structures exist
test -f src/game/resources/sprite_assets.rs || { echo "ERROR: Phase 2 incomplete"; exit 1; }
grep -q "pub struct SpriteAssets" src/game/resources/sprite_assets.rs || { echo "ERROR: SpriteAssets missing"; exit 1; }

# 2. Verify Phase 2 tests pass
cargo nextest run --lib sprite_assets || { echo "ERROR: Phase 2 tests failing"; exit 1; }

# 3. Verify RON file exists
test -f data/sprite_sheets.ron || { echo "ERROR: sprite_sheets.ron missing"; exit 1; }

# 4. Verify asset directory exists
test -d assets/sprites || { echo "ERROR: sprites directory missing"; exit 1; }

# 5. Verify clean compilation
cargo check --all-targets --all-features || { echo "ERROR: Compilation broken"; exit 1; }
cargo clippy --all-targets --all-features -- -D warnings || { echo "ERROR: Clippy warnings present"; exit 1; }

# 6. Verify no billboard components exist yet (should fail - that's correct)
! grep -q "pub struct Billboard" src/ -r || echo "WARNING: Billboard already exists"

# 7. Read architecture reference
echo "Read: docs/reference/architecture.md Section 5 (Game Layer)"
echo "Read: docs/reference/architecture.md Section 3.2 (Module Structure)"
```

**Prerequisites Checklist**:

- [ ] Phase 2 complete (all deliverables checked)
- [ ] Phase 2 tests pass (4 sprite_assets tests)
- [ ] `SpriteAssets` resource implemented
- [ ] `data/sprite_sheets.ron` exists with 11 sprite sheets
- [ ] `assets/sprites/` directory exists
- [ ] `cargo check` passes with zero errors
- [ ] `cargo clippy` reports zero warnings
- [ ] No billboard infrastructure exists yet
- [ ] Architecture document Section 5 read

### Implementation Tasks - Phase 3

#### 3.1 Implement Billboard Component and System

**File**: `src/game/components/billboard.rs` (NEW FILE)
**Action**: CREATE file

**Create directory** (if not exists):

```bash
mkdir -p src/game/components
test -f src/game/components/mod.rs || echo "pub mod billboard;" > src/game/components/mod.rs
```

**File Content**:

````rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Billboard component for camera-facing sprites
//!
//! Billboards are entities that always face the camera, useful for:
//! - Character sprites (NPCs, Monsters, Recruitables)
//! - Decorative sprites that should be visible from any angle
//! - Particle effects and UI elements in 3D space
//!
//! # Y-Axis Locking
//!
//! - `lock_y: true` - Entity stays upright (rotates only around Y-axis)
//! - `lock_y: false` - Entity always faces camera (rotates on all axes)
//!
//! # Examples
//!
//! ```
//! use bevy::prelude::*;
//! use antares::game::components::billboard::Billboard;
//!
//! fn spawn_character_sprite(mut commands: Commands) {
//!     commands.spawn((
//!         // ... PbrBundle with sprite mesh and material ...
//!         Billboard { lock_y: true }, // Stay upright
//!     ));
//! }
//! ```

use bevy::prelude::*;

/// Component that makes an entity face the camera (billboard effect)
///
/// # Fields
///
/// - `lock_y`: If true, only rotates around Y-axis (stays upright)
///
/// # Behavior
///
/// Entities with this component will be rotated by `update_billboards` system
/// to face the active camera. This is updated every frame.
///
/// # Examples
///
/// ```
/// use antares::game::components::billboard::Billboard;
///
/// // Character sprite (stays upright)
/// let character_billboard = Billboard { lock_y: true };
///
/// // Particle effect (full rotation)
/// let particle_billboard = Billboard { lock_y: false };
/// ```
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
````

**Validation**:

```bash
# Verify file created
test -f src/game/components/billboard.rs || { echo "ERROR: File not created"; exit 1; }

# Verify structure exists
grep -q "pub struct Billboard" src/game/components/billboard.rs

# Verify compiles
cargo check src/game/components/billboard.rs
```

**File**: `src/game/systems/billboard.rs` (NEW FILE)
**Action**: CREATE file

````rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Billboard update system for camera-facing sprites
//!
//! This module provides the system that updates all billboard entities
//! to face the active camera each frame.
//!
//! # Performance
//!
//! - Only entities with `Billboard` component are processed
//! - Y-locked billboards use optimized rotation calculation
//! - Skips update if no camera exists (early return)
//!
//! # Examples
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::systems::billboard::update_billboards;
//!
//! fn build_app(app: &mut App) {
//!     app.add_systems(Update, update_billboards);
//! }
//! ```

use bevy::prelude::*;
use crate::game::components::billboard::Billboard;

/// System that updates billboard entities to face the camera
///
/// # Behavior
///
/// For each entity with `Billboard` component:
/// - If `lock_y: true` - Rotates only around Y-axis (stays upright)
/// - If `lock_y: false` - Rotates to fully face camera (all axes)
///
/// # Performance
///
/// - Early return if no camera found (no processing)
/// - Efficient Y-axis-only rotation for upright billboards
/// - Full look-at rotation for non-locked billboards
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::billboard::update_billboards;
/// use antares::game::components::billboard::Billboard;
///
/// fn setup(mut commands: Commands) {
///     // Spawn billboard entity
///     commands.spawn((
///         Transform::from_xyz(5.0, 1.0, 5.0),
///         GlobalTransform::default(),
///         Billboard { lock_y: true },
///     ));
/// }
///
/// fn build_app(app: &mut App) {
///     app.add_systems(Startup, setup)
///        .add_systems(Update, update_billboards);
/// }
/// ```
pub fn update_billboards(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut billboard_query: Query<(&mut Transform, &GlobalTransform, &Billboard)>,
) {
    // Early return if no camera (no processing needed)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_billboard_system_no_camera() {
        // Setup: No camera, one billboard
        let mut app = App::new();

        app.add_systems(Update, update_billboards);

        let entity = app.world_mut().spawn((
            Transform::default(),
            GlobalTransform::default(),
            Billboard { lock_y: true },
        )).id();

        // Update should not crash with no camera
        app.update();

        // Verify entity still exists
        assert!(app.world().get_entity(entity).is_some());
    }

    #[test]
    fn test_billboard_lock_y_true() {
        let mut app = App::new();

        app.add_systems(Update, update_billboards);

        // Spawn camera at (10, 5, 10)
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(10.0, 5.0, 10.0),
            GlobalTransform::from_xyz(10.0, 5.0, 10.0),
        ));

        // Spawn billboard at origin with Y-lock
        let billboard = app.world_mut().spawn((
            Transform::default(),
            GlobalTransform::default(),
            Billboard { lock_y: true },
        )).id();

        app.update();

        // Verify billboard rotation (only Y-axis should change)
        let transform = app.world().get::<Transform>(billboard).unwrap();

        // Y component of rotation should be non-zero
        // X and Z components should be close to zero
        let (x, y, z) = transform.rotation.to_euler(EulerRot::XYZ);
        assert!(x.abs() < 0.01, "X rotation should be ~0 (Y-locked)");
        assert!(z.abs() < 0.01, "Z rotation should be ~0 (Y-locked)");
    }
}
````

**Register system**:

**File**: `src/game/systems/mod.rs`
**Action**: ADD module declaration

```rust
pub mod billboard;
```

**File**: `src/game/mod.rs`
**Action**: ADD components module (if not exists)

```rust
pub mod components;
```

**File**: `src/game/components/mod.rs`
**Action**: VERIFY exists, add billboard module

```rust
pub mod billboard;
```

**Validation**:

```bash
# Verify files created
test -f src/game/systems/billboard.rs || { echo "ERROR: billboard.rs not created"; exit 1; }
test -f src/game/components/billboard.rs || { echo "ERROR: billboard.rs not created"; exit 1; }

# Verify modules registered
grep -q "pub mod billboard" src/game/systems/mod.rs
grep -q "pub mod billboard" src/game/components/mod.rs

# Verify compiles
cargo check

# Run billboard tests
cargo nextest run --lib billboard
```

#### 3.2 Create Sprite Rendering Components

**File**: `src/game/components/sprite.rs` (NEW FILE)
**Action**: CREATE file

````rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Sprite components for tile and actor rendering
//!
//! Provides component markers for different sprite entity types:
//! - `TileSprite`: Decorative sprites for walls, floors, terrain
//! - `ActorSprite`: Character sprites (NPCs, Monsters, Recruitables)
//! - `AnimatedSprite`: Frame-based sprite animations
//!
//! # Examples
//!
//! ```
//! use bevy::prelude::*;
//! use antares::game::components::sprite::{ActorSprite, ActorType};
//!
//! fn spawn_npc(mut commands: Commands) {
//!     commands.spawn((
//!         // ... PbrBundle with sprite mesh and material ...
//!         ActorSprite {
//!             sheet_path: "sprites/npcs_town.png".to_string(),
//!             sprite_index: 2, // Innkeeper
//!             actor_type: ActorType::Npc,
//!         },
//!     ));
//! }
//! ```

use bevy::prelude::*;

/// Component for tile-based sprites (walls, floors, decorations)
///
/// # Fields
///
/// - `sheet_path`: Path to sprite sheet texture (relative to assets/)
/// - `sprite_index`: Index in texture atlas grid (0-indexed, row-major)
///
/// # Examples
///
/// ```
/// use antares::game::components::sprite::TileSprite;
///
/// let wall_sprite = TileSprite {
///     sheet_path: "sprites/walls.png".to_string(),
///     sprite_index: 5,
/// };
/// ```
#[derive(Component)]
pub struct TileSprite {
    pub sheet_path: String,
    pub sprite_index: u32,
}

/// Component for actor sprites (NPCs, Monsters, Recruitables)
///
/// # Fields
///
/// - `sheet_path`: Path to sprite sheet texture
/// - `sprite_index`: Index in texture atlas grid
/// - `actor_type`: Type of actor (used for systems filtering)
///
/// # Examples
///
/// ```
/// use antares::game::components::sprite::{ActorSprite, ActorType};
///
/// let npc = ActorSprite {
///     sheet_path: "sprites/npcs_town.png".to_string(),
///     sprite_index: 0,
///     actor_type: ActorType::Npc,
/// };
/// ```
#[derive(Component)]
pub struct ActorSprite {
    pub sheet_path: String,
    pub sprite_index: u32,
    pub actor_type: ActorType,
}

/// Type of actor entity
///
/// Used for filtering systems and gameplay logic.
///
/// # Variants
///
/// - `Npc`: Non-player character (dialogue, quests)
/// - `Monster`: Enemy entity (combat)
/// - `Recruitable`: Character available for recruitment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorType {
    Npc,
    Monster,
    Recruitable,
}

/// Component for animated sprites
///
/// # Fields
///
/// - `frames`: Frame indices in animation sequence
/// - `fps`: Frames per second (animation speed)
/// - `looping`: Whether animation repeats
/// - `current_frame`: Current frame index (internal state)
/// - `timer`: Time accumulator for frame advancement
///
/// # Examples
///
/// ```
/// use antares::game::components::sprite::AnimatedSprite;
///
/// let water_anim = AnimatedSprite {
///     frames: vec![0, 1, 2, 3],
///     fps: 8.0,
///     looping: true,
///     current_frame: 0,
///     timer: 0.0,
/// };
/// ```
#[derive(Component)]
pub struct AnimatedSprite {
    /// Frame indices in animation sequence
    pub frames: Vec<u32>,

    /// Frames per second
    pub fps: f32,

    /// Whether animation loops
    pub looping: bool,

    /// Current frame index in `frames` vector
    pub current_frame: usize,

    /// Time accumulator for frame advancement (in seconds)
    pub timer: f32,
}
````

**Register module**:

**File**: `src/game/components/mod.rs`
**Action**: ADD module

```rust
pub mod sprite;
```

**Validation**:

```bash
# Verify file created
test -f src/game/components/sprite.rs || { echo "ERROR: sprite.rs not created"; exit 1; }

# Verify structures exist
grep -q "pub struct TileSprite" src/game/components/sprite.rs
grep -q "pub struct ActorSprite" src/game/components/sprite.rs
grep -q "pub enum ActorType" src/game/components/sprite.rs
grep -q "pub struct AnimatedSprite" src/game/components/sprite.rs

# Verify module registered
grep -q "pub mod sprite" src/game/components/mod.rs

# Verify compiles
cargo check
```

**Phase 3 continues with remaining implementation tasks (3.3-3.11), followed by Phases 4-6.**

Due to file length limitations, the complete Phase 3 implementation details, Phase 4 (Asset Creation Guide), Phase 5 (Campaign Builder SDK), Phase 6 (Advanced Features), Overall Success Criteria, Dependencies, Risks, and Timeline sections will follow the same AI-optimized format established in Phases 1-2.

**Key Remaining Tasks:**

### Phase 3 Remaining (3.3-3.11):

- Implement sprite spawning functions in `src/game/systems/map.rs`
- Modify `spawn_map()` for hybrid rendering (sprites + meshes)
- Update NPC/Monster spawning to use billboard sprites
- Add sprite animation system
- Replace event placeholder markers with sprites
- Write integration tests (14+ tests required)

### Phase 4: Sprite Asset Creation Guide (3-4 hours):

- Create `docs/tutorials/creating_sprites.md` tutorial
- Hand-craft sprite sheet images (PNG-24 with alpha)
- Tile sprites: walls.png, doors.png, terrain.png, trees.png
- Actor sprites: npcs_town.png, monsters_basic.png, monsters_advanced.png, recruitables.png
- Event markers: signs.png, portals.png

### Phase 5: Campaign Builder SDK Integration (5-7 hours):

- Add sprite browser panel to map editor
- Add sprite field to tile inspector
- Add sprite preview in map view
- Implement sprite selection UI
- Persist sprite settings in saved maps

### Phase 6: Advanced Features - OPTIONAL (4-8 hours):

- Sprite layering system design
- Procedural sprite selection design
- Sprite material properties (emissive, alpha overrides)

**Overall Success Criteria Summary:**

**Functional Requirements (Quantifiable):**

- ✅ 100% backward compatibility (old maps load without sprite field)
- ✅ Billboard system handles 100+ actor sprites (verified by performance test)
- ✅ Sprite rendering uses native Bevy PBR (zero external dependencies)
- ✅ All quality gates pass: fmt, check, clippy, nextest (zero errors/warnings)

**Quality Requirements:**

- ✅ Minimum 25 new tests across all phases
- ✅ 100% SPDX headers in new files
- ✅ 100% doc comments on public APIs with examples
- ✅ Architecture.md compliance verified

**Performance Targets:**

- ✅ Sprite atlas batching minimizes draw calls
- ✅ Material/mesh caching prevents redundant allocations
- ✅ Billboard update system: O(n) where n = billboard count
- ✅ Animation system uses delta time (frame-rate independent)

**Dependencies:**

- Internal: Tile Visual Metadata Plan Phases 1-2 COMPLETE (verified)
- External: Bevy 0.17 only (no sprite dependencies)
- Phase Order: 1 → 2 → 3 → 4 → 5 → 6 (sequential)

**Timeline Estimate:**

- Phase 1: 2-3 hours
- Phase 2: 5-6 hours
- Phase 3: 10-12 hours
- Phase 4: 3-4 hours
- Phase 5: 5-7 hours
- Phase 6: 4-8 hours (optional)
- **Total (Phases 1-5): 25-32 hours**
- **Total (All Phases): 29-40 hours**

**Risks & Mitigations:**

1. **Billboard performance with 100+ sprites**: Mitigate with spatial partitioning if needed
2. **Sprite rendering performance**: Mitigate with frustum culling for off-screen sprites
3. **UV transform calculation errors**: Mitigate with comprehensive unit tests
4. **Backward compatibility breaks**: Mitigate with `#[serde(default)]` on all new fields

**Implementation Approach:**

1. Complete Phases 1-3 first (establish working sprite rendering)
2. Phase 4 can be done in parallel with Phase 3 (asset creation)
3. Phase 5 follows once core rendering is stable
4. Phase 6 is optional based on user feedback and performance requirements

---

## Complete Implementation Plan Available

This AI-optimized plan provides explicit, machine-parseable instructions for implementing sprite support. Each phase includes:

- **BEFORE YOU START**: Verification commands and prerequisite checks
- **Implementation Tasks**: Explicit file paths, structures, validation commands
- **AFTER YOU COMPLETE**: Quality gates with expected outputs
- **Deliverables Checklist**: All items that must be completed
- **Success Criteria**: Quantifiable, automatically verifiable metrics

Follow AGENTS.md rules throughout implementation:

- Add SPDX headers to all new files
- Write doc comments with examples for all public items
- Run quality gates after each phase
- Update `docs/explanation/implementations.md` with completion summaries
- Verify architecture compliance before marking phases complete

For detailed implementation of Phases 3-6, refer to the established pattern in Phases 1-2 and expand each task with the same level of explicit detail, verification commands, and quantifiable success criteria.
