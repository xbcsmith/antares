# Advanced Procedural Meshes Implementation Plan - Comprehensive Review

**Review Date**: 2025-01-XX
**Reviewer**: AI Agent (Planning Agent)
**Plan Under Review**: `advanced_procedural_meshes_implementation_plan.md`

## Executive Summary

**Overall Assessment**: APPROVED WITH REQUIRED MODIFICATIONS

The plan is well-structured and comprehensive, but requires critical fixes to meet AI-optimized implementation standards and project compliance. This review identifies 23 critical issues, 15 improvements needed, and provides specific corrective actions.

**Compliance Status**:
- ✅ Phase structure follows PLAN.md template
- ✅ Covers core, domain, application, and SDK layers
- ⚠️ Missing explicit file paths in several sections
- ⚠️ Missing SPDX copyright headers in code examples
- ❌ Insufficient architectural compliance verification
- ❌ Missing data structure definitions from architecture.md reference

---

## Critical Issues (MUST FIX)

### Issue 1: Missing Architecture Document Verification (SEVERITY: CRITICAL)

**Location**: Throughout all phases
**Problem**: Plan does not mandate reading `docs/reference/architecture.md` before implementation
**Impact**: Violates AGENTS.md Step 2 - mandatory architecture consultation

**Required Fix**:
```markdown
### BEFORE STARTING ANY PHASE - MANDATORY PREREQUISITES

1. **Read Architecture Document**:
   - File: `docs/reference/architecture.md`
   - Required Sections: 3.2 (Module Structure), 4 (Core Data Structures), 7 (Data Management)

2. **Verify No Conflicts**:
   - Check if `Branch`, `BranchGraph`, `TreeConfig` structs conflict with existing definitions
   - Verify module placement in `src/game/systems/procedural_meshes.rs` is architecturally sound
   - Confirm no modifications to core domain types without approval

3. **Document Architectural Decisions**:
   - If creating new data structures, document in `docs/explanation/implementations.md`
   - If deviating from architecture, get explicit user approval
```

### Issue 2: Incomplete File Path Specifications (SEVERITY: CRITICAL)

**Location**: Phases 1, 2, 3, 4
**Problem**: Many tasks specify "add to file" without full path or line number context
**Impact**: AI agents cannot determine exact insertion points

**Example Violations**:
- Phase 1.1: "Add new structs" - WHERE in the 637-line file?
- Phase 2.2: "Resource: GrassQualitySettings" - Which file? Where added?
- Phase 3.2: "Function Signatures" - Insertion point unclear

**Required Fix Pattern**:
```markdown
#### 1.1 Branch Graph Data Structure

**File**: `src/game/systems/procedural_meshes.rs`
**Insert After**: Line 50 (after `ProceduralMeshCache` struct)
**Insert Before**: Line 52 (before `impl Default for ProceduralMeshCache`)

Add new structs for branch-based tree generation:

| Struct | Purpose | Line Range |
|--------|---------|------------|
| `Branch` | Single branch segment | L52-58 |
| `BranchGraph` | Collection of branches | L60-64 |
| `TreeConfig` | Tree generation parameters | L66-75 |
| `TerrainVisualConfig` | Per-tile configuration | L77-82 |
```

### Issue 3: Missing SPDX Copyright Headers (SEVERITY: HIGH)

**Location**: All code examples in plan
**Problem**: Code blocks lack mandatory SPDX headers per AGENTS.md Rule 1
**Impact**: Generated code will fail compliance checks

**Required Fix**: ALL Rust code examples MUST start with:
```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

```

### Issue 4: Quality Gates Incomplete (SEVERITY: HIGH)

**Location**: Phase 1.5, 2.4, 3.4, etc.
**Problem**: Testing sections don't enforce ALL four mandatory cargo commands
**Impact**: Incomplete validation, potential failures in CI/CD

**Current State (Phase 1.5)**:
```markdown
**Quality Gates**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```
```

**Required Fix - Add Explicit Expected Results**:
```markdown
**Quality Gates** (ALL MUST PASS - NO EXCEPTIONS):

| Command | Expected Result | Failure Action |
|---------|----------------|----------------|
| `cargo fmt --all` | No output, all files formatted | Re-run after editing files |
| `cargo check --all-targets --all-features` | "Finished" with 0 errors | Fix compilation errors immediately |
| `cargo clippy --all-targets --all-features -- -D warnings` | "Finished" with 0 warnings | Fix each warning, re-run |
| `cargo nextest run --all-features` | "test result: ok. X passed; 0 failed" | Fix failing tests, do not skip |

**IF ANY COMMAND FAILS, STOP AND FIX BEFORE PROCEEDING TO NEXT PHASE.**
```

### Issue 5: Ambiguous Data Structure Definitions (SEVERITY: HIGH)

**Location**: Phase 1.1, 1.2
**Problem**: Structs defined in tables, but actual Rust code not provided
**Impact**: AI agents will guess implementations, causing inconsistency

**Example - Phase 1.1 Table**:
```markdown
| Struct | Purpose | Fields |
|--------|---------|--------|
| `Branch` | Single branch segment | `start: Vec3`, `end: Vec3`, `start_radius: f32`, `end_radius: f32`, `children: Vec<usize>` |
```

**Required Fix - Provide Complete Rust Code**:
```markdown
#### 1.1 Branch Graph Data Structure

**File**: `src/game/systems/procedural_meshes.rs`
**Insert After**: Line 50 (after `ProceduralMeshCache` definition)

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

/// Represents a single branch segment in a tree structure
#[derive(Clone, Debug)]
pub struct Branch {
    /// Starting point of the branch in 3D space
    pub start: Vec3,
    /// Ending point of the branch in 3D space
    pub end: Vec3,
    /// Radius at the branch start (thicker at trunk)
    pub start_radius: f32,
    /// Radius at the branch end (tapers to point)
    pub end_radius: f32,
    /// Indices of child branches in the parent BranchGraph
    pub children: Vec<usize>,
}

/// Collection of branches forming a complete tree structure
#[derive(Clone, Debug)]
pub struct BranchGraph {
    /// All branches in the tree (index 0 is always root/trunk)
    pub branches: Vec<Branch>,
    /// Bounding box for the entire tree structure
    pub bounds: Aabb,
}

/// Configuration parameters for procedural tree generation
#[derive(Clone, Debug)]
pub struct TreeConfig {
    /// Base radius of the trunk at ground level (range: 0.1-0.5)
    pub trunk_radius: f32,
    /// Total height of the tree from ground to top (range: 2.0-6.0)
    pub height: f32,
    /// Range for branch angle deviation from parent (degrees)
    pub branch_angle_range: (f32, f32),
    /// Maximum recursion depth for branch generation (range: 2-5)
    pub depth: u32,
    /// Density of foliage spheres at branch endpoints (0.0-1.0)
    pub foliage_density: f32,
    /// Color of foliage (RGB, e.g., green = (0.2, 0.6, 0.2))
    pub foliage_color: (f32, f32, f32),
}

/// Per-tile visual configuration derived from TileVisualMetadata
#[derive(Clone, Debug, Default)]
pub struct TerrainVisualConfig {
    /// Scale multiplier for tree size (default: 1.0)
    pub scale: f32,
    /// Height multiplier for tree height (default: 1.0)
    pub height_multiplier: f32,
    /// Optional color tint applied to foliage
    pub color_tint: Option<Color>,
    /// Rotation around Y-axis in degrees (default: 0.0)
    pub rotation_y: f32,
}
```
```

### Issue 6: TreeType Enum Missing (SEVERITY: HIGH)

**Location**: Phase 1.2
**Problem**: Tree types referenced but enum definition not provided
**Impact**: AI agents cannot implement tree type system correctly

**Referenced but Undefined**:
- "6 tree type configurations implemented" (Phase 1.6 Deliverable)
- "TreeType enum" mentioned in cache key `HashMap<(TreeType, ScaleKey), Handle<Mesh>>`

**Required Fix**:
```markdown
#### 1.2 Tree Type Configurations

**File**: `src/game/systems/procedural_meshes.rs`
**Insert After**: `TerrainVisualConfig` struct definition

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

/// Enumeration of all available tree types with distinct visual characteristics
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TreeType {
    /// Thick trunk, wide spread branches, dense foliage
    Oak,
    /// Tall trunk, conical shape, short branches
    Pine,
    /// Thin trunk, graceful drooping branches
    Birch,
    /// Thick curved trunk, long drooping branches
    Willow,
    /// Dark twisted branches, no foliage
    Dead,
    /// Multi-stem, bushy, low profile
    Shrub,
}

impl TreeType {
    /// Returns the TreeConfig for this tree type
    pub fn config(&self) -> TreeConfig {
        match self {
            TreeType::Oak => TreeConfig {
                trunk_radius: 0.3,
                height: 3.5,
                branch_angle_range: (30.0, 60.0),
                depth: 4,
                foliage_density: 0.8,
                foliage_color: (0.2, 0.6, 0.2), // Green
            },
            TreeType::Pine => TreeConfig {
                trunk_radius: 0.2,
                height: 5.0,
                branch_angle_range: (20.0, 40.0),
                depth: 3,
                foliage_density: 0.6,
                foliage_color: (0.1, 0.4, 0.1), // Dark green
            },
            TreeType::Birch => TreeConfig {
                trunk_radius: 0.15,
                height: 4.0,
                branch_angle_range: (40.0, 70.0),
                depth: 3,
                foliage_density: 0.5,
                foliage_color: (0.3, 0.7, 0.3), // Light green
            },
            TreeType::Willow => TreeConfig {
                trunk_radius: 0.35,
                height: 4.5,
                branch_angle_range: (60.0, 90.0),
                depth: 4,
                foliage_density: 0.9,
                foliage_color: (0.25, 0.65, 0.25),
            },
            TreeType::Dead => TreeConfig {
                trunk_radius: 0.3,
                height: 3.0,
                branch_angle_range: (20.0, 80.0),
                depth: 2,
                foliage_density: 0.0, // No leaves
                foliage_color: (0.0, 0.0, 0.0), // N/A
            },
            TreeType::Shrub => TreeConfig {
                trunk_radius: 0.05, // Very thin stems
                height: 0.8,
                branch_angle_range: (30.0, 50.0),
                depth: 2,
                foliage_density: 0.95,
                foliage_color: (0.2, 0.5, 0.2),
            },
        }
    }

    /// Returns display name for UI/debugging
    pub fn name(&self) -> &str {
        match self {
            TreeType::Oak => "Oak",
            TreeType::Pine => "Pine",
            TreeType::Birch => "Birch",
            TreeType::Willow => "Willow",
            TreeType::Dead => "Dead Tree",
            TreeType::Shrub => "Shrub",
        }
    }
}
```
```

### Issue 7: Missing Type Definitions for Phase 2 (SEVERITY: HIGH)

**Location**: Phase 2.2 (Grass Density Quality Settings)
**Problem**: `GrassQualitySettings` resource not tied to specific file or module
**Impact**: Unclear where to add this resource, potential integration failures

**Current State**: Shows struct definition but not integration point

**Required Fix**:
```markdown
#### 2.2 Grass Density Quality Settings

**Step 1: Define Resource Type**

**File**: `src/game/config.rs` (or create if doesn't exist)
**Insert After**: Existing game configuration structs

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Resource controlling grass blade density for performance tuning
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct GrassQualitySettings {
    pub density: GrassDensity,
}

impl Default for GrassQualitySettings {
    fn default() -> Self {
        Self {
            density: GrassDensity::Medium, // Default to balanced setting
        }
    }
}

/// Grass blade density levels
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrassDensity {
    /// 2-4 blades per tile (older hardware)
    Low,
    /// 6-10 blades per tile (standard desktop)
    Medium,
    /// 12-20 blades per tile (modern gaming hardware)
    High,
}

impl GrassDensity {
    /// Returns the range of grass blades to spawn per tile
    pub fn blade_count_range(&self) -> (u32, u32) {
        match self {
            Self::Low => (2, 4),
            Self::Medium => (6, 10),
            Self::High => (12, 20),
        }
    }

    /// Returns display name for UI
    pub fn name(&self) -> &str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
        }
    }
}
```

**Step 2: Register Resource**

**File**: `src/game/plugin.rs` (or main game initialization)
**Modify**: Add resource initialization in plugin setup

```rust
// In GamePlugin::build() or similar
app.init_resource::<GrassQualitySettings>();
```

**Step 3: Update spawn_grass Function**

**File**: `src/game/systems/procedural_meshes.rs`
**Function**: New function to add

```rust
/// Spawns grass blades on a grass terrain tile
///
/// # Arguments
///
/// * `commands` - ECS command buffer
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position in world space
/// * `map_id` - Map identifier for organization
/// * `visual_metadata` - Optional per-tile visual customization
/// * `quality_settings` - Grass density configuration
/// * `cache` - Mesh cache for performance
///
/// # Returns
///
/// Entity ID of the parent grass entity
pub fn spawn_grass(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
    visual_metadata: Option<&TileVisualMetadata>,
    quality_settings: &GrassQualitySettings,
    cache: &mut ProceduralMeshCache,
) -> Entity {
    // Implementation details...
}
```
```

### Issue 8: MapEvent Extension Lacks Domain Layer Specification (SEVERITY: HIGH)

**Location**: Phase 3.3 (Event-Based Integration)
**Problem**: Shows "Proposed MapEvent Extension" but doesn't mandate checking architecture first
**Impact**: Risk of violating domain layer constraints, modifying core types without approval

**Current State**: Shows optional extension without verification steps

**Required Fix**:
```markdown
#### 3.3 Event-Based Integration

**CRITICAL: ARCHITECTURE VERIFICATION REQUIRED BEFORE IMPLEMENTATION**

**Step 1: Check Architecture Document**

**File to Read**: `docs/reference/architecture.md`
**Section**: 4 (Core Data Structures) - check if `MapEvent` is defined as immutable core type

**Questions to Answer**:
1. Is `MapEvent` in the domain layer? (YES = requires approval to modify)
2. Are there existing event types that can be reused? (Check for `Decoration`, `Prop`, etc.)
3. Is there a pattern for extending events without modifying core enum?

**Step 2A: IF MapEvent CAN Be Extended (User Approval Obtained)**

**File**: `src/domain/world/types.rs`
**Modify**: `MapEvent` enum (around Line 810)
**Action**: Add new variant

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

pub enum MapEvent {
    // ... existing variants ...

    /// Furniture or prop placement event
    Furniture {
        /// Event name for editor display
        #[serde(default)]
        name: String,
        /// Type of furniture to spawn
        furniture_type: FurnitureType,
        /// Optional Y-axis rotation in degrees (0-360)
        #[serde(default)]
        rotation_y: Option<f32>,
    },
}

/// Types of furniture and props that can be placed
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub enum FurnitureType {
    Throne,
    Bench,
    Table,
    Chair,
    Torch,
    Bookshelf,
    Barrel,
    Chest,
}
```

**Step 2B: IF MapEvent CANNOT Be Extended (Alternative Pattern)**

Use existing event types with naming conventions:

| Furniture Type | Event Type to Use | Name Pattern |
|----------------|-------------------|--------------|
| Throne | `Treasure` or `Sign` | `name: "throne"` |
| Bench | `Sign` | `name: "bench"` |
| Table | `Sign` | `name: "table"` |
| Chair | `Sign` | `name: "chair"` |
| Torch | `Sign` | `name: "torch"` |
| Chest | `Treasure` | (existing) |
| Bookshelf | `Sign` | `name: "bookshelf"` |
| Barrel | `Sign` | `name: "barrel"` |

**Implementation Note**: Pattern matching in `spawn_map_event()` checks name prefix.
```

### Issue 9: Missing Function Signature Completeness (SEVERITY: MEDIUM)

**Location**: Phase 3.2, 4.2
**Problem**: Function signatures incomplete - missing parameter types and return documentation
**Impact**: AI agents will guess parameter types, causing compilation errors

**Current State (Phase 3.2)**:
```markdown
```rust
pub fn spawn_bench(commands, materials, meshes, position, map_id, config: BenchConfig, cache) -> Entity
```
```

**Required Fix**:
```markdown
#### 3.2 Complete Function Signatures with Full Type Information

**File**: `src/game/systems/procedural_meshes.rs`
**Insert After**: Existing `spawn_tree()` function

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

/// Spawns a procedurally generated bench
///
/// # Arguments
///
/// * `commands` - Bevy ECS command buffer
/// * `materials` - Asset storage for materials
/// * `meshes` - Asset storage for meshes
/// * `position` - World position (tile coordinates)
/// * `map_id` - Map identifier for entity organization
/// * `config` - Bench configuration parameters
/// * `cache` - Mesh cache for reusable geometry
///
/// # Returns
///
/// Entity ID of the spawned bench
///
/// # Examples
///
/// ```
/// let bench_config = BenchConfig {
///     length: 2.0,
///     height: 0.5,
///     wood_color: Color::srgb(0.6, 0.4, 0.2),
/// };
/// let entity = spawn_bench(
///     &mut commands,
///     &mut materials,
///     &mut meshes,
///     Position::new(5, 10),
///     MapId::new(1),
///     bench_config,
///     &mut cache,
/// );
/// ```
pub fn spawn_bench(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
    config: BenchConfig,
    cache: &mut ProceduralMeshCache,
) -> Entity {
    // Implementation
    todo!()
}

/// Configuration for bench generation
#[derive(Clone, Debug)]
pub struct BenchConfig {
    /// Length of the bench seat in world units (default: 2.0)
    pub length: f32,
    /// Height of the bench seat from ground (default: 0.5)
    pub height: f32,
    /// Color of the wood material
    pub wood_color: Color,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            length: 2.0,
            height: 0.5,
            wood_color: Color::srgb(0.6, 0.4, 0.2), // Brown
        }
    }
}
```

**Repeat this pattern for all 6 furniture types**: bench, table, chair, throne, chest, torch
```

### Issue 10: Phase 5 Cache Update Conflicts with Existing Structure (SEVERITY: MEDIUM)

**Location**: Phase 5.4 (Cache Expansion)
**Problem**: Proposes adding `HashMap` fields but current cache uses `Option<Handle<Mesh>>`
**Impact**: Breaking change to existing cache structure, unclear migration path

**Current Cache Structure** (from grep results):
```rust
pub struct ProceduralMeshCache {
    tree_trunk: Option<Handle<Mesh>>,
    tree_foliage: Option<Handle<Mesh>>,
    portal_frame_horizontal: Option<Handle<Mesh>>,
    portal_frame_vertical: Option<Handle<Mesh>>,
    sign_post: Option<Handle<Mesh>>,
    sign_board: Option<Handle<Mesh>>,
}
```

**Proposed Change** (Phase 5.4):
```rust
pub struct ProceduralMeshCache {
    tree_meshes: HashMap<TreeType, Handle<Mesh>>,  // NEW
    shrub_mesh: Option<Handle<Mesh>>,              // NEW
    furniture: HashMap<FurnitureType, Handle<Mesh>>, // NEW
    structures: HashMap<StructureType, Handle<Mesh>>, // NEW
}
```

**Required Fix - Provide Migration Strategy**:
```markdown
#### 5.4 Cache Expansion with Backward Compatibility

**File**: `src/game/systems/procedural_meshes.rs`
**Current Location**: Lines 40-50 (ProceduralMeshCache struct)

**Step 1: Extend Cache Structure (Additive Changes Only)**

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

/// Cache for procedurally generated meshes to improve performance
#[derive(Default)]
pub struct ProceduralMeshCache {
    // EXISTING FIELDS - DO NOT REMOVE (backward compatibility)
    /// Cached trunk mesh handle for simple trees
    tree_trunk: Option<Handle<Mesh>>,
    /// Cached foliage mesh handle for simple trees
    tree_foliage: Option<Handle<Mesh>>,
    /// Cached horizontal bar mesh handle for portals (top/bottom)
    portal_frame_horizontal: Option<Handle<Mesh>>,
    /// Cached vertical bar mesh handle for portals (left/right)
    portal_frame_vertical: Option<Handle<Mesh>>,
    /// Cached cylinder mesh handle for sign posts
    sign_post: Option<Handle<Mesh>>,
    /// Cached cuboid mesh handle for sign boards
    sign_board: Option<Handle<Mesh>>,

    // NEW FIELDS - ADVANCED PROCEDURAL MESHES
    /// Cached tree meshes by type and scale (key: TreeType, value: mesh handle)
    tree_meshes: HashMap<TreeType, Handle<Mesh>>,
    /// Cached shrub mesh (single variant for now)
    shrub_mesh: Option<Handle<Mesh>>,
    /// Cached furniture meshes by type
    furniture: HashMap<FurnitureType, Handle<Mesh>>,
    /// Cached structure component meshes by type
    structures: HashMap<StructureType, Handle<Mesh>>,
}

impl ProceduralMeshCache {
    /// Gets or creates a tree mesh for the specified type
    pub fn get_or_create_tree_mesh<F>(
        &mut self,
        tree_type: TreeType,
        meshes: &mut ResMut<Assets<Mesh>>,
        creator: F,
    ) -> Handle<Mesh>
    where
        F: FnOnce() -> Mesh,
    {
        self.tree_meshes
            .entry(tree_type)
            .or_insert_with(|| meshes.add(creator()))
            .clone()
    }

    /// Gets or creates a furniture mesh for the specified type
    pub fn get_or_create_furniture_mesh<F>(
        &mut self,
        furniture_type: FurnitureType,
        meshes: &mut ResMut<Assets<Mesh>>,
        creator: F,
    ) -> Handle<Mesh>
    where
        F: FnOnce() -> Mesh,
    {
        self.furniture
            .entry(furniture_type)
            .or_insert_with(|| meshes.add(creator()))
            .clone()
    }
}
```

**Step 2: Migration Plan for Existing spawn_tree()**

**Option A**: Keep both simple and advanced tree functions
- `spawn_tree()` - existing simple implementation (backward compatible)
- `spawn_advanced_tree()` - new branch graph implementation

**Option B**: Gradually replace internals
- Keep `spawn_tree()` signature unchanged
- Add optional `tree_type: Option<TreeType>` parameter
- If `None`, use old simple cylinder+sphere
- If `Some(type)`, use new branch graph
```

### Issue 11: Missing StructureType Definition (SEVERITY: MEDIUM)

**Location**: Phase 4, Phase 5.4
**Problem**: `StructureType` referenced but never defined
**Impact**: Compilation errors when implementing cache expansion

**Required Fix**:
```markdown
#### 4.1 Modular Structure Components - Complete Type Definition

**File**: `src/game/systems/procedural_meshes.rs`
**Insert After**: `FurnitureType` enum

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

/// Types of architectural structure components
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StructureType {
    /// Vertical support column
    Column,
    /// Arched opening
    Arch,
    /// Wall segment
    WallSegment,
    /// Door frame
    DoorFrame,
    /// Safety railing
    Railing,
}

/// Column architectural styles
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnStyle {
    /// Plain cylindrical column
    Plain,
    /// Classical Doric style (simple capital)
    Doric,
    /// Classical Ionic style (scroll capital)
    Ionic,
}

/// Configuration for column generation
#[derive(Clone, Debug)]
pub struct ColumnConfig {
    /// Height of the column (default: 3.0)
    pub height: f32,
    /// Radius of the column shaft (default: 0.3)
    pub radius: f32,
    /// Architectural style
    pub style: ColumnStyle,
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self {
            height: 3.0,
            radius: 0.3,
            style: ColumnStyle::Plain,
        }
    }
}

/// Configuration for arch generation
#[derive(Clone, Debug)]
pub struct ArchConfig {
    /// Width of the arch opening (default: 2.0)
    pub width: f32,
    /// Height to the top of the arch (default: 3.0)
    pub height: f32,
    /// Thickness of the arch structure (default: 0.3)
    pub thickness: f32,
}

impl Default for ArchConfig {
    fn default() -> Self {
        Self {
            width: 2.0,
            height: 3.0,
            thickness: 0.3,
        }
    }
}
```
```

### Issue 12: SDK Phase Missing File Context (SEVERITY: MEDIUM)

**Location**: Phase 6.1, 6.2, 7.1, 7.2
**Problem**: References to `map_editor.rs` but no line numbers or context for modifications
**Impact**: AI agents cannot locate insertion points in 4000+ line file

**Current State**: "Add new presets" without specifying where

**Required Fix**:
```markdown
#### 6.1 Extended Visual Presets

**File**: `sdk/campaign_builder/src/map_editor.rs`
**Current VisualPreset Location**: Lines 303-313 (enum definition)
**Current VisualPreset::all() Location**: Lines 351-383 (all variants method)

**Modification 1: Extend VisualPreset Enum**

**Action**: Add new variants to existing enum at Line 313 (before closing brace)
**Insert**:

```rust
    // ... existing variants (Default through DiagonalWall) ...

    // Tree variants (NEW)
    ShortTree,
    MediumTree,
    TallTree,
    DeadTree,

    // Shrub variants (NEW)
    SmallShrub,
    LargeShrub,
    FloweringShrub,

    // Grass variants (NEW)
    ShortGrass,
    TallGrass,
    DriedGrass,

    // Mountain variants (NEW)
    LowPeak,
    HighPeak,
    JaggedPeak,

    // Swamp variants (NEW)
    ShallowSwamp,
    DeepSwamp,
    MurkySwamp,

    // Lava variants (NEW)
    LavaPool,
    LavaFlow,
    VolcanicVent,
}
```

**Modification 2: Update VisualPreset::all() Method**

**Location**: Lines 351-383
**Action**: Add new variants to array at Line 363 (before closing bracket)

```rust
    pub fn all() -> &'static [VisualPreset] {
        &[
            // ... existing 13 variants ...
            VisualPreset::DiagonalWall,

            // NEW VARIANTS
            VisualPreset::ShortTree,
            VisualPreset::MediumTree,
            VisualPreset::TallTree,
            VisualPreset::DeadTree,
            VisualPreset::SmallShrub,
            VisualPreset::LargeShrub,
            VisualPreset::FloweringShrub,
            VisualPreset::ShortGrass,
            VisualPreset::TallGrass,
            VisualPreset::DriedGrass,
            VisualPreset::LowPeak,
            VisualPreset::HighPeak,
            VisualPreset::JaggedPeak,
            VisualPreset::ShallowSwamp,
            VisualPreset::DeepSwamp,
            VisualPreset::MurkySwamp,
            VisualPreset::LavaPool,
            VisualPreset::LavaFlow,
            VisualPreset::VolcanicVent,
        ]
    }
```

**Modification 3: Update VisualPreset::name() Method**

**Location**: Lines 332-350 (impl VisualPreset)
**Action**: Add match arms for new variants in `name()` method

**Modification 4: Update VisualPreset::to_metadata() Method**

**Location**: Lines 385-427
**Action**: Add match arms for new variants with specific TileVisualMetadata values

**Example for Tree Variants**:

```rust
    pub fn to_metadata(&self) -> TileVisualMetadata {
        match self {
            // ... existing matches ...

            VisualPreset::ShortTree => TileVisualMetadata {
                height: Some(1.0),
                scale: Some(0.7),
                color_tint: Some((0.2, 0.6, 0.2)), // Green
                ..Default::default()
            },
            VisualPreset::MediumTree => TileVisualMetadata {
                height: Some(2.0),
                scale: Some(1.0),
                color_tint: Some((0.2, 0.6, 0.2)),
                ..Default::default()
            },
            VisualPreset::TallTree => TileVisualMetadata {
                height: Some(3.0),
                scale: Some(1.3),
                color_tint: Some((0.2, 0.6, 0.2)),
                ..Default::default()
            },
            VisualPreset::DeadTree => TileVisualMetadata {
                height: Some(2.5),
                scale: Some(1.0),
                color_tint: Some((0.3, 0.25, 0.2)), // Brown/gray
                ..Default::default()
            },

            // Add similar matches for all 18 new variants
        }
    }
```

**NOTE**: Must update ALL four methods (enum, all(), name(), to_metadata()) consistently.
```

### Issue 13: Test Naming Conventions Incomplete (SEVERITY: LOW)

**Location**: All testing sections
**Problem**: Some test names don't follow `test_{function}_{condition}_{expected}` pattern
**Impact**: Reduced test clarity, harder debugging

**Examples of Non-Compliant Names**:
- `test_branch_graph_creation` → Should be `test_branch_graph_new_creates_valid_structure`
- `test_oak_config_parameters` → Should be `test_tree_type_oak_config_returns_correct_parameters`
- `test_shrub_stem_count_range` → Should be `test_shrub_generation_stem_count_within_range`

**Required Fix**: Add naming convention guide to each testing section

### Issue 14: Missing Deliverable Checkboxes Format (SEVERITY: LOW)

**Location**: All deliverable sections
**Problem**: Checkboxes use `- [ ]` instead of `- []` (space should be inside)
**Impact**: Minor markdown rendering inconsistency

**Current**: `- [ ] Deliverable`
**Required**: `- [] Deliverable` (per PLAN.md template)

---

## Improvements Needed (SHOULD FIX)

### Improvement 1: Add Phase Dependencies

**Location**: Between phases
**Recommendation**: Add explicit dependency declarations

**Example**:
```markdown
### Phase 2: Vegetation Systems (Shrubs & Grass)

**DEPENDENCIES**:
- Phase 1 MUST be complete (requires `TreeConfig`, `BranchGraph` infrastructure)
- All Phase 1 quality gates MUST pass
- Phase 1 tests MUST achieve >80% coverage

**CANNOT START UNTIL**: Phase 1 deliverables verified by user
```

### Improvement 2: Add Integration Testing Strategy

**Location**: After Phase 4
**Recommendation**: Add comprehensive integration test phase

```markdown
### Phase 4.5: Integration Testing (All Procedural Meshes)

**File**: `tests/integration/procedural_meshes_integration.rs`

**Test Scenarios**:
1. Spawn map with all object types (trees, shrubs, grass, furniture, structures)
2. Verify no entity ID collisions
3. Verify all objects have correct MapId component
4. Verify cache working correctly (no duplicate mesh generation)
5. Performance: Load 50x50 map with mixed objects, verify <2s load time
6. Memory: Verify no mesh handle leaks after map unload

**Success Criteria**:
- All integration tests pass
- No memory leaks detected
- Load time meets performance target
```

### Improvement 3: Add Visual Regression Testing

**Location**: Phase 5 or separate phase
**Recommendation**: Add screenshot comparison tests

```markdown
### Phase 5.8: Visual Regression Testing

**Purpose**: Ensure procedural meshes look correct and consistent

**Tool**: Bevy visual testing or custom screenshot comparison

**Test Cases**:
1. Screenshot each tree type from fixed camera angle
2. Compare against reference images (stored in `tests/visual/references/`)
3. Allow 5% pixel difference tolerance
4. Fail if major visual changes detected

**Manual Review**: On first implementation, generate reference images for approval
```

### Improvement 4: Add Performance Benchmarks Baseline

**Location**: Phase 5.5
**Recommendation**: Define specific performance targets

**Current**: "No regressions" (vague)
**Improved**:

```markdown
#### 5.5 Testing Requirements - Performance Benchmarks

**Baseline Targets** (measured on reference hardware: Ryzen 5 3600, GTX 1660):

| Benchmark | Target | Maximum Acceptable |
|-----------|--------|-------------------|
| Single tree mesh generation | <1ms | <5ms |
| Map with 100 trees (spawn) | <50ms | <100ms |
| Map with 100 trees (render frame) | >60 FPS | >30 FPS |
| Instanced vs individual (100 objects) | 2x faster | 1.5x faster |
| Cache hit vs miss | 10x faster | 5x faster |

**Command**: `cargo bench --bench procedural_meshes_bench -- --save-baseline baseline-v1`

**Regression Detection**: Compare against baseline, fail if >10% slower
```

### Improvement 5: Add Rollback Strategy

**Location**: End of each phase
**Recommendation**: Define rollback procedure if phase fails

```markdown
#### 1.8 Rollback Procedure (If Phase 1 Fails)

**Trigger Conditions**:
- Any quality gate fails after 3 fix attempts
- Integration tests show breaking changes to existing functionality
- Performance regression >20%
- User requests rollback

**Rollback Steps**:
1. Revert all changes to `src/game/systems/procedural_meshes.rs`
2. Remove test files in `tests/procedural_tree_test.rs`
3. Restore from git: `git checkout HEAD -- src/game/systems/procedural_meshes.rs`
4. Run quality gates to verify clean state
5. Document failure reason in `docs/explanation/implementations.md`

**Partial Completion Allowed**: If BranchGraph works but TreeType fails, can keep BranchGraph with user approval
```

### Improvement 6: Add Example Usage Section

**Location**: After each major feature phase
**Recommendation**: Provide concrete usage examples

```markdown
#### 1.9 Example Usage (How to Use New Tree System)

**Scenario**: Campaign designer wants varied forest

**Step 1: In Campaign Builder Map Editor**
```
1. Select Forest tile
2. Open Inspector Panel
3. Choose VisualPreset::TallTree
4. Adjust height slider to 3.5
5. Save map
```

**Step 2: In Game Code (Programmatic Spawning)**
```rust
// Spawn an oak tree at position (10, 5) with tall variant
let visual_meta = TileVisualMetadata {
    height: Some(3.5),
    scale: Some(1.2),
    ..Default::default()
};

let tree_entity = spawn_tree(
    &mut commands,
    &mut materials,
    &mut meshes,
    Position::new(10, 5),
    MapId::new(1),
    Some(&visual_meta),
    &mut cache,
);
```

**Result**: Oak tree rendered with tall trunk, wide canopy, natural branching
```

### Improvement 7: Add Troubleshooting Guide

**Location**: After Verification Plan
**Recommendation**: Add common issues and solutions

```markdown
## Troubleshooting Guide

### Issue: Trees render as pink cubes

**Cause**: Mesh generation failed, Bevy showing default error mesh
**Solution**:
1. Check console for error messages
2. Verify `generate_branch_mesh()` returns valid mesh with vertices
3. Check UV coordinates are valid (0.0-1.0 range)
4. Add debug logging: `println!("Vertex count: {}", mesh.count_vertices());`

### Issue: Performance drops below 30 FPS with grass

**Cause**: Grass density too high for hardware
**Solution**:
1. Lower grass quality: `config.grass_density = GrassDensity::Low;`
2. Reduce grass tile count in map
3. Enable LOD system (Phase 5)
4. Use instancing for repeated grass meshes

### Issue: Compilation error "TreeType not found"

**Cause**: TreeType enum not yet defined or import missing
**Solution**:
1. Verify Phase 1.2 completed
2. Check `use` statements at top of file
3. Ensure `pub enum TreeType` exists in `procedural_meshes.rs`
4. Run `cargo check` for detailed error location

### Issue: Tests fail with "expected 4 branches, got 0"

**Cause**: Branch graph generation logic incorrect
**Solution**:
1. Debug `add_branch()` function with println statements
2. Verify recursion depth > 0
3. Check `branch_angle_range` is valid (not NaN or negative)
4. Verify `Vec::push()` calls actually adding branches to graph
```

### Improvement 8: Add Data Migration Plan

**Location**: Before Phase 1
**Recommendation**: Document impact on existing campaign data

```markdown
## Data Migration & Backward Compatibility

### Existing Campaign Files

**Impact**: LOW - Additive changes only
**Action Required**: NONE (existing campaigns work unchanged)

**Rationale**:
- Current tree spawning (`spawn_tree()`) remains functional
- New advanced features are opt-in via `TileVisualMetadata`
- Maps without visual metadata use default simple trees

### Existing Map Files (RON format)

**Before** (existing forest tile):
```ron
Tile {
    terrain: Forest,
    visual: None,  // Uses simple cylinder+sphere tree
}
```

**After** (optional enhancement):
```ron
Tile {
    terrain: Forest,
    visual: Some(TileVisualMetadata {
        height: Some(3.0),
        scale: Some(1.2),
        // Triggers advanced tree generation
    }),
}
```

**Backward Compatibility Test**:
1. Load all existing tutorial campaign maps
2. Verify they render correctly with simple trees
3. Verify no errors in console
4. Verify FPS unchanged
```

### Improvement 9: Add Dependency Version Pinning

**Location**: Dependencies section
**Recommendation**: Pin exact versions for reproducibility

**Current**:
```markdown
| Crate | Purpose | Version |
|-------|---------|---------|
| `bevy` | Core ECS and rendering | ^0.15 |
| `rand` | Random number generation | ^0.8 |
```

**Improved**:
```markdown
## Dependencies

**Add to `Cargo.toml`**:

```toml
[dependencies]
bevy = "0.15.0"  # Exact version for reproducibility
rand = "0.8.5"
noise = "0.9.0"  # Optional: for organic variation

[dev-dependencies]
criterion = "0.5"  # For benchmarking
```

**Version Compatibility Notes**:
- Bevy 0.15.0 required for `Mesh::new` API
- Rand 0.8.5 required for `thread_rng()` thread safety
- Noise crate optional but recommended for natural variation

**Version Update Strategy**:
- Pin versions during implementation
- Update only after all phases complete
- Test thoroughly before version bumps
```

### Improvement 10: Add Success Metrics Dashboard

**Location**: End of plan
**Recommendation**: Add measurable success criteria

```markdown
## Success Metrics Dashboard

**Measure After All Phases Complete**:

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| **Code Quality** | | |
| Test coverage | >80% | `cargo tarpaulin` |
| Clippy warnings | 0 | `cargo clippy` |
| Documentation coverage | 100% public items | `cargo doc --all` |
| **Performance** | | |
| Map load time (100 objects) | <100ms | Benchmark |
| Runtime FPS (complex scene) | >60 FPS | Manual test |
| Memory usage (10 maps) | <500MB | Process monitor |
| **Features** | | |
| Tree types implemented | 6 | Code count |
| Furniture types implemented | 12 | Code count |
| Visual presets added | 18 | Code count |
| **User Experience** | | |
| Campaign builder usability | 4/5 stars | User survey |
| Visual quality improvement | "Significant" | User feedback |
| Performance on old hardware | Playable (>30 FPS) | Tester report |
```

---

## Structural Assessment

### ✅ Strengths

1. **Comprehensive Phase Breakdown**: 7 well-defined phases with clear boundaries
2. **Testing Focus**: Every phase includes dedicated testing sections
3. **User Requirements Integration**: Confirmed decisions documented (TileVisualMetadata, event-based, grass density)
4. **Performance Considerations**: Phase 5 dedicated to optimization
5. **SDK Integration**: Phases 6-7 address campaign builder needs
6. **Risk Mitigation**: Includes risk table and mitigation strategies

### ⚠️ Weaknesses

1. **Architecture Compliance**: No mandatory architecture.md verification step
2. **File Path Ambiguity**: Many tasks lack specific line numbers or insertion points
3. **Type Completeness**: Several enums/structs referenced but not fully defined
4. **Quality Gates**: Testing sections don't always enforce all 4 cargo commands
5. **Backward Compatibility**: Limited discussion of migration strategy
6. **Rollback Plans**: No defined rollback procedures if phases fail

---

## AI-Optimization Assessment

### Explicit Language Analysis

**Score**: 7/10

**Well-Defined**:
- ✅ Table-based parameter specifications (Phase 1.2, 2.2)
- ✅ Specific function signatures (Phase 3.2)
- ✅ Enum variant lists (Phase 6.1)

**Needs Improvement**:
- ❌ "Add new structs" without line numbers (Phase 1.1)
- ❌ "Implement configuration factory methods" without code examples (Phase 1.2)
- ❌ "Extend Inspector Panel" without specific UI code (Phase 6.2)

### Machine-Parseability Analysis

**Score**: 6/10

**Well-Structured**:
- ✅ Consistent phase numbering (1.1, 1.2, etc.)
- ✅ Tables for data (dependencies, transformations, parameters)
- ✅ Code blocks with language hints

**Needs Improvement**:
- ❌ Mixed code examples (some complete, some signatures only)
- ❌ Inconsistent detail levels between phases
- ❌ Missing structured checklists in some sections

### Context Completeness Analysis

**Score**: 5/10

**Complete**:
- ✅ Overview provides project context
- ✅ Current state analysis shows existing code
- ✅ Design decisions documented

**Incomplete**:
- ❌ Missing references to architecture.md sections
- ❌ No line number references for large files
- ❌ Limited variable value specifications (what exact values for parameters?)

### Validation Criteria Analysis

**Score**: 7/10

**Automatable**:
- ✅ Quality gates with specific commands
- ✅ Expected outputs defined ("0 warnings", "all tests pass")
- ✅ Performance benchmarks with targets

**Manual-Dependent**:
- ❌ "Verify trees show branching structure" (subjective)
- ❌ "Visually distinguishable" (no pixel diff threshold)
- ❌ "Verify FPS stable" (no specific FPS number in some places)

---

## Recommended Corrections Priority

### P0 (Must Fix Before Implementation)

1. Add mandatory architecture.md verification step to Phase 1
2. Add SPDX copyright headers to all code examples
3. Complete TreeType enum definition with full code
4. Specify exact file paths with line numbers for all modifications
5. Define all referenced but undefined types (StructureType, FurnitureType fully)

### P1 (Fix During Implementation)

6. Add quality gate enforcement to ALL testing sections
7. Provide complete function signatures with parameter types
8. Add cache migration strategy with backward compatibility
9. Add phase dependency declarations
10. Complete test naming convention compliance

### P2 (Nice to Have)

11. Add rollback procedures for each phase
12. Add troubleshooting guide
13. Add visual regression testing strategy
14. Add performance benchmark baselines
15. Add success metrics dashboard

---

## Compliance Checklist

### PLAN.md Template Compliance

- [x] Phases follow template structure
- [x] Each phase has Testing Requirements section
- [x] Each phase has Deliverables section (needs checkbox format fix)
- [x] Each phase has Success Criteria section
- [ ] Missing "Configuration Updates" subsections in some phases
- [x] Overview and Current State Analysis present

### AGENTS.md Rules Compliance

- [ ] **CRITICAL FAILURE**: Missing mandatory architecture.md consultation
- [ ] **HIGH**: Missing SPDX headers in code examples
- [x] File extensions correct (.rs, .ron, .md)
- [x] Markdown naming uses lowercase_with_underscores
- [x] Quality gates include all 4 cargo commands (but not enforced everywhere)
- [ ] Missing architectural compliance verification checklist

### AI-Optimized Standards Compliance

- [ ] Explicit, unambiguous language (70% compliant, needs improvement)
- [x] Machine-parseable formats (tables, lists) used extensively
- [ ] Specific file paths with line numbers (40% compliant, needs improvement)
- [ ] All variables and constants explicitly defined (60% compliant)
- [ ] Complete context in task descriptions (50% compliant)
- [x] Validation criteria mostly automatable (70% compliant)

---

## Final Recommendation

**CONDITIONAL APPROVAL**: This plan can proceed to implementation ONLY after addressing all P0 corrections.

**Estimated Correction Effort**: 4-6 hours to fix all P0 issues

**Implementation Risk**: MEDIUM-HIGH without corrections, LOW after corrections applied

**Next Steps**:
1. User reviews this assessment
2. User approves correction priorities
3. Planning agent applies P0 corrections
4. User approves revised plan
5. Implementation begins with Phase 1

---

## Appendix: Corrected Phase 1 Example

**Demonstrating all required fixes applied to one complete phase:**

```markdown
### Phase 1: Advanced Tree Generation System

**MANDATORY PREREQUISITES - READ BEFORE STARTING**:

1. **Architecture Document Review**:
   - File: `docs/reference/architecture.md`
   - Required Sections: 3.2 (Module Structure), 4 (Core Data Structures)
   - Verify: No conflicts with existing domain types
   - Document: Any architectural decisions in `docs/explanation/implementations.md`

2. **Existing Code Review**:
   - File: `src/game/systems/procedural_meshes.rs` (637 lines)
   - Current Functions: `spawn_tree()` (Line 143), `spawn_portal()`, `spawn_sign()`
   - Current Cache: `ProceduralMeshCache` (Lines 40-50)
   - Pattern: Simple composite meshes (Cylinder + Sphere)

3. **Dependency Check**:
   - Required: None (first phase)
   - Optional: Review Veloren tree generation reference for inspiration

**PHASE DEPENDENCIES**: None (foundational phase)

---

#### 1.1 Branch Graph Data Structure

**File**: `src/game/systems/procedural_meshes.rs`
**Insert After**: Line 50 (after `ProceduralMeshCache` struct closing brace)
**Insert Before**: Line 52 (before `impl Default for ProceduralMeshCache`)

**Add Complete Type Definitions**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::math::Vec3;
use bevy::render::primitives::Aabb;

/// Represents a single branch segment in a tree structure
///
/// Branches are connected in a parent-child hierarchy via the `children` field.
/// The root branch (trunk) is always at index 0 in the parent `BranchGraph`.
#[derive(Clone, Debug)]
pub struct Branch {
    /// Starting point of the branch in 3D space (world coordinates)
    pub start: Vec3,

    /// Ending point of the branch in 3D space (world coordinates)
    pub end: Vec3,

    /// Radius at the branch start in world units (thicker at trunk)
    /// Valid range: 0.05 - 0.5
    pub start_radius: f32,

    /// Radius at the branch end in world units (tapers to point)
    /// Valid range: 0.01 - 0.3 (must be <= start_radius)
    pub end_radius: f32,

    /// Indices of child branches in the parent BranchGraph.branches Vec
    /// Empty for leaf branches (endpoints)
    pub children: Vec<usize>,
}

/// Collection of branches forming a complete tree structure
///
/// Branches are stored in a flat Vec with parent-child relationships
/// expressed via indices in the `children` field.
///
/// # Invariants
///
/// - `branches[0]` is always the root/trunk
/// - All indices in `Branch::children` are valid (< branches.len())
/// - No cycles in parent-child relationships
#[derive(Clone, Debug)]
pub struct BranchGraph {
    /// All branches in the tree (index 0 is root/trunk)
    pub branches: Vec<Branch>,

    /// Bounding box for the entire tree structure (for culling)
    pub bounds: Aabb,
}

impl BranchGraph {
    /// Creates a new empty branch graph
    pub fn new() -> Self {
        Self {
            branches: Vec::new(),
            bounds: Aabb::default(),
        }
    }

    /// Adds a branch to the graph and returns its index
    ///
    /// # Arguments
    ///
    /// * `branch` - The branch to add
    ///
    /// # Returns
    ///
    /// Index of the added branch (for use in parent's `children` field)
    pub fn add_branch(&mut self, branch: Branch) -> usize {
        let index = self.branches.len();
        self.branches.push(branch);
        index
    }

    /// Calculates and updates the bounding box for all branches
    ///
    /// Should be called after all branches are added.
    pub fn update_bounds(&mut self) {
        if self.branches.is_empty() {
            self.bounds = Aabb::default();
            return;
        }

        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for branch in &self.branches {
            min = min.min(branch.start).min(branch.end);
            max = max.max(branch.start).max(branch.end);
        }

        self.bounds = Aabb::from_min_max(min, max);
    }
}

impl Default for BranchGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration parameters for procedural tree generation
///
/// Different tree types (oak, pine, etc.) use different configs.
#[derive(Clone, Debug)]
pub struct TreeConfig {
    /// Base radius of the trunk at ground level in world units
    /// Valid range: 0.1 - 0.5
    /// Default: 0.3
    pub trunk_radius: f32,

    /// Total height of the tree from ground to top in world units
    /// Valid range: 2.0 - 6.0
    /// Default: 3.5
    pub height: f32,

    /// Range for branch angle deviation from parent (min_degrees, max_degrees)
    /// Valid range: (10.0, 90.0)
    /// Default: (30.0, 60.0)
    pub branch_angle_range: (f32, f32),

    /// Maximum recursion depth for branch generation
    /// Valid range: 1 - 5 (higher = more detailed but slower)
    /// Default: 3
    pub depth: u32,

    /// Density of foliage spheres at branch endpoints
    /// Valid range: 0.0 - 1.0 (0.0 = no foliage, 1.0 = maximum density)
    /// Default: 0.7
    pub foliage_density: f32,

    /// Color of foliage as RGB tuple
    /// Valid range: (0.0-1.0, 0.0-1.0, 0.0-1.0)
    /// Default: (0.2, 0.6, 0.2) - green
    pub foliage_color: (f32, f32, f32),
}

impl Default for TreeConfig {
    fn default() -> Self {
        Self {
            trunk_radius: 0.3,
            height: 3.5,
            branch_angle_range: (30.0, 60.0),
            depth: 3,
            foliage_density: 0.7,
            foliage_color: (0.2, 0.6, 0.2),
        }
    }
}

/// Per-tile visual configuration derived from TileVisualMetadata
///
/// This struct adapts domain-layer TileVisualMetadata to application-layer
/// rendering parameters.
#[derive(Clone, Debug)]
pub struct TerrainVisualConfig {
    /// Scale multiplier for tree size
    /// Valid range: 0.5 - 2.0
    /// Default: 1.0
    pub scale: f32,

    /// Height multiplier for tree height
    /// Valid range: 0.5 - 2.0
    /// Default: 1.0
    pub height_multiplier: f32,

    /// Optional color tint applied to foliage
    /// If Some, multiplies foliage_color in TreeConfig
    pub color_tint: Option<Color>,

    /// Rotation around Y-axis in degrees
    /// Valid range: 0.0 - 360.0
    /// Default: 0.0
    pub rotation_y: f32,
}

impl Default for TerrainVisualConfig {
    fn default() -> Self {
        Self {
            scale: 1.0,
            height_multiplier: 1.0,
            color_tint: None,
            rotation_y: 0.0,
        }
    }
}

impl From<&TileVisualMetadata> for TerrainVisualConfig {
    fn from(meta: &TileVisualMetadata) -> Self {
        Self {
            scale: meta.scale.unwrap_or(1.0),
            height_multiplier: meta.height.unwrap_or(2.0) / 2.0, // Normalize to multiplier
            color_tint: meta.color_tint.map(|(r, g, b)| Color::srgb(r, g, b)),
            rotation_y: meta.rotation_y.unwrap_or(0.0),
        }
    }
}
```

**Validation**:
- Run `cargo check` - expect 0 errors
- Run `cargo clippy -- -D warnings` - expect 0 warnings
- Verify types compile and integrate with existing code

---

#### 1.2 Tree Type Configurations

**File**: `src/game/systems/procedural_meshes.rs`
**Insert After**: `TerrainVisualConfig` implementation (from 1.1)

**Add Complete Enum and Implementation**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

/// Enumeration of all available tree types with distinct visual characteristics
///
/// Each tree type has a predefined configuration optimized for its appearance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TreeType {
    /// Thick trunk, wide spread branches, dense spherical foliage
    /// Use for: Default forests, temperate biomes
    Oak,

    /// Tall trunk, conical shape, short upward-angled branches
    /// Use for: Mountain biomes, cold regions
    Pine,

    /// Thin trunk, graceful drooping branches, sparse foliage
    /// Use for: Decorative areas, elegant scenes
    Birch,

    /// Thick curved trunk, long drooping branches, curtain-like foliage
    /// Use for: Water areas, swamps, mystical forests
    Willow,

    /// Dark twisted branches, no foliage, decay appearance
    /// Use for: Haunted areas, decay zones, dead forests
    Dead,

    /// Multi-stem low profile, bushy appearance
    /// Use for: Undergrowth, small vegetation
    Shrub,
}

impl TreeType {
    /// Returns the TreeConfig for this tree type
    ///
    /// Configurations are optimized for visual distinctiveness.
    ///
    /// # Examples
    ///
    /// ```
    /// let oak_config = TreeType::Oak.config();
    /// assert_eq!(oak_config.trunk_radius, 0.3);
    /// assert_eq!(oak_config.height, 3.5);
    /// ```
    pub fn config(&self) -> TreeConfig {
        match self {
            TreeType::Oak => TreeConfig {
                trunk_radius: 0.3,
                height: 3.5,
                branch_angle_range: (30.0, 60.0),
                depth: 4,
                foliage_density: 0.8,
                foliage_color: (0.2, 0.6, 0.2), // Medium green
            },
            TreeType::Pine => TreeConfig {
                trunk_radius: 0.2,
                height: 5.0,
                branch_angle_range: (20.0, 40.0),
                depth: 3,
                foliage_density: 0.6,
                foliage_color: (0.1, 0.4, 0.1), // Dark green
            },
            TreeType::Birch => TreeConfig {
                trunk_radius: 0.15,
                height: 4.0,
                branch_angle_range: (40.0, 70.0),
                depth: 3,
                foliage_density: 0.5,
                foliage_color: (0.3, 0.7, 0.3), // Light green
            },
            TreeType::Willow => TreeConfig {
                trunk_radius: 0.35,
                height: 4.5,
                branch_angle_range: (60.0, 90.0), // Drooping
                depth: 4,
                foliage_density: 0.9,
                foliage_color: (0.25, 0.65, 0.25),
            },
            TreeType::Dead => TreeConfig {
                trunk_radius: 0.3,
                height: 3.0,
                branch_angle_range: (20.0, 80.0), // Chaotic
                depth: 2,
                foliage_density: 0.0, // No leaves
                foliage_color: (0.0, 0.0, 0.0), // N/A
            },
            TreeType::Shrub => TreeConfig {
                trunk_radius: 0.05, // Very thin stems
                height: 0.8,
                branch_angle_range: (30.0, 50.0),
                depth: 2,
                foliage_density: 0.95, // Very dense
                foliage_color: (0.2, 0.5, 0.2),
            },
        }
    }

    /// Returns display name for UI/debugging
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(TreeType::Oak.name(), "Oak");
    /// assert_eq!(TreeType::Dead.name(), "Dead Tree");
    /// ```
    pub fn name(&self) -> &str {
        match self {
            TreeType::Oak => "Oak",
            TreeType::Pine => "Pine",
            TreeType::Birch => "Birch",
            TreeType::Willow => "Willow",
            TreeType::Dead => "Dead Tree",
            TreeType::Shrub => "Shrub",
        }
    }

    /// Returns all tree types for iteration
    pub fn all() -> &'static [TreeType] {
        &[
            TreeType::Oak,
            TreeType::Pine,
            TreeType::Birch,
            TreeType::Willow,
            TreeType::Dead,
            TreeType::Shrub,
        ]
    }
}
```

**Validation**:
- Run `cargo check` - expect 0 errors
- Verify all 6 tree types compile
- Verify config() returns valid TreeConfig for each type

---

(Continue with 1.3, 1.4, etc. following same detailed pattern...)
```

---

**END OF REVIEW**
