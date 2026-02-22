# Terrain Quality Improvement Implementation Plan

## Overview

Improve terrain, grass, and tree procedural meshes to use high-quality PBR textures and advanced geometries. The improvements must retain the current TileMap ecosystem, where `TileVisualMetadata` configures specific variants and features (e.g., grass density, tree types) dynamically for the player.

## Current State Analysis

### Existing Infrastructure

- Procedural modeling currently uses basic Bevy primitives (Spheres, Cylinders) in `src/game/systems/advanced_trees.rs`.
- Grass is rendered as procedurally deformed flat colored triangles in `src/game/systems/advanced_grass.rs`.
- `TileVisualMetadata` configures `height`, `scale`, `color_tint`, `grass_density`, `TreeType`, and `grass_blade_config`.
- Mesh caching exists in `src/game/systems/procedural_meshes.rs` to avoid redundant GPU allocations.

### Identified Issues

- Procedural grass and trees look cartoonish, lacking textures, detailed geometry, and PBR materials.
- Base level ground, stone, and mountain tiles rely on simple flat-color materials dynamically generated in `src/game/systems/map.rs`.

## Implementation Phases

### Phase 1: Material and Texture Foundation

#### 1.1 Foundation Work
Expand `domain::world::TerrainType` material handling in `src/game/systems/map.rs` to support loading base color, normal, and roughness textures via Bevy's `AssetServer`.

#### 1.2 Add Foundation Functionality
Create a `TerrainMaterialCache` resource to load and store standard terrain textures (e.g. `grass`, `stone`, `mountain`, `water`) at application startup, reducing dynamic load times.

#### 1.3 Integrate Foundation Work
Update `src/game/systems/map.rs` to apply the newly cached textured materials to floor tiles instead of flat colors, while retaining the ability to apply `color_tint` from `TileVisualMetadata`.

#### 1.4 Testing Requirements
Run `cargo test` to ensure map data structures remain valid. Launch the client map viewer (`cargo run --bin map_builder`) to visually verify textures on ground tiles instead of flat green/gray colors.

#### 1.5 Deliverables
- [ ] Implement `TerrainMaterialCache` resource and loading logic.
- [ ] Update `src/game/systems/map.rs` to apply textured `StandardMaterial` to ground tiles.

#### 1.6 Success Criteria
Mountains, stone, and grass ground tiles display detailed textures instead of flat minimal shading.

### Phase 2: High-Quality Grass Generation

#### 2.1 Feature Work
Update `create_grass_blade_mesh` in `src/game/systems/advanced_grass.rs` to properly map UV coordinates to a realistic grass blade texture atlas.

#### 2.2 Integrate Feature
Modify `spawn_grass_cluster` to use an alpha-masked `StandardMaterial` loaded from a high-quality grass texture rather than simple colored opaque triangles.

#### 2.3 Configuration Updates
Ensure `world::GrassBladeConfig` properties (like `color_variation`) continue to tint the textured blades via vertex colors or material base color parameters, maintaining TileMap tweaking.

#### 2.4 Testing Requirements
Spawn into a map abundant with grass tiles and observe the new textured grass blades. Modify `TileVisualMetadata` in a map `.ron` file to ensure grass density scaling and tint adjustments continue to function.

#### 2.5 Deliverables
- [ ] Update `src/game/systems/advanced_grass.rs` with textured, alpha-masked leaf mesh generation.

#### 2.6 Success Criteria
Procedural grass fields appear detailed and realistic, matching the `ideal_grass.png` reference target, whilst maintaining good frame rates.

### Phase 3: High-Quality Tree Models

#### 3.1 Feature Work
Deprecate the procedural spheres and basic cylinders in `src/game/systems/advanced_trees.rs`. Introduce GLTF model loading for `TreeType` variants (Oak, Pine, Birch, Willow, Dead, Shrub), or rewrite the procedural generator to use plane-based alpha-cutout foliage and detailed textured bark.

#### 3.2 Integrate Feature
Update `ProceduralMeshCache` inside `src/game/systems/procedural_meshes.rs` to retrieve and serve the new GLTF models or refined procedural meshes for each `TreeType`.

#### 3.3 Configuration Updates
Retain integration with `TerrainVisualConfig` so it can continue to apply scale and Y-axis rotation dynamically to the populated tree entities from the map.

#### 3.4 Testing Requirements
Load a heavily forested map. Verify trees render with detailed leaves and proper bark textures, and confirm that their size scales match what is configured via the TileMap metadata. Check distance culling and standard performance markers.

#### 3.5 Deliverables
- [ ] Replace procedural tree primitives with textured geometries or GLTF implementations for all `TreeType` variants.
- [ ] Integrate new models into `ProceduralMeshCache`.

#### 3.6 Success Criteria
In-game trees match the realism level of the `ideal_tree.jpg` reference and no longer resemble simple sphere colliders.
