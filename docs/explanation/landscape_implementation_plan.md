# Landscape Implementation Plan

## Goal

Add a data-driven `landscape` category to the importer and SDK so campaign authors can import and place outdoor decoration meshes such as trees, shrubs, brush, rocks, and similar environmental props. The feature should also replace or repair the current default procedural tree/brush visuals by using the imported tree RON assets in `assets/meshes/` and the tree texture set in `campaigns/tutorial/assets/textures/trees/`.

## Current State

- `Map` data currently stores terrain tiles, map events, NPC placements, dropped items, locks, outdoor flags, and sky configuration. It does not have a first-class collection for static landscape placements.
- `TileVisualMetadata` already supports terrain-tied procedural variation through fields such as `tree_type`, `grass_density`, `rock_variant`, `foliage_density`, color tint, scale, height, offset, and rotation.
- The Campaign Builder importer supports `Creature`, `Item`, and `Furniture` export types. It does not expose a landscape target.
- Furniture already provides a useful pattern: a campaign RON definition database, a mesh registry, optional custom mesh IDs, map-editor palette placement, and runtime rendering through shared mesh infrastructure.
- New landscape mesh source files exist in `assets/meshes/`: `oak_tree_short.ron`, `pine_tree.ron`, `dead_tree.ron`, `palm_tree.ron`, and `brush.ron`.
- Tree texture files exist in `campaigns/tutorial/assets/textures/trees/`: `bark.png`, `foliage_oak.png`, `foliage_pine.png`, `foliage_birch.png`, `foliage_willow.png`, `foliage_palm.png`, and `foliage_shrub.png`.

## Design Principles

- Keep gameplay terrain data separate from placed visual decoration. Terrain remains `Tile` and `TileVisualMetadata`; authored landscape instances live in a dedicated placement list.
- Use RON for all game data. No JSON or YAML game content.
- Add explicit type aliases for new identifiers, such as `LandscapeId` and `LandscapeMeshId`, instead of raw integer types.
- Reuse the existing `MeshDefinition` and texture-path rules where possible. Mesh texture paths must resolve from the active campaign asset root and start with `assets/`.
- Keep imported meshes optional. Procedural trees, grass, and rock visuals remain fallback paths when no custom landscape definition is placed.
- Treat this as an architecture change. The first implementation phase should update `docs/reference/architecture.md` before changing `Map` and domain data structures.
- All tests and fixtures must use `data/test_campaign`, never `campaigns/tutorial`.

## Proposed Data Model

Introduce a new domain module for landscape definitions and placement, likely `src/domain/world/landscape.rs`, with re-exports from `src/domain/world/mod.rs`.

Suggested core types:

- `LandscapeId`: identifier for reusable landscape definitions.
- `LandscapeMeshId`: identifier for imported landscape mesh assets.
- `LandscapeCategory`: broad palette grouping, initially `Tree`, `Shrub`, `Brush`, `Rock`, `Grass`, `GroundCover`, `Ruin`, and `Custom`.
- `LandscapeDefinition`: reusable campaign definition containing ID, name, category, icon, tags, optional `mesh_id`, fallback procedural kind, default scale, default color tint, optional blocking/collision behavior, and description.
- `LandscapePlacement`: map instance containing `landscape_id`, `position: Position`, optional sub-tile offset, rotation, scale override, tint override, and optional placement flags.
- `LandscapeDatabase`: loads `data/landscape.ron`.
- `LandscapeMeshDatabase`: loads `data/landscape_mesh_registry.ron` and references mesh RON files under `assets/meshes/landscape/` or an equivalent campaign-relative path.

Add `landscape_placements: Vec<LandscapePlacement>` to `Map` after architecture approval. This keeps multiple decorations per tile possible without overloading `MapEvent`, `Tile`, or `TileVisualMetadata`.

## Asset and Data File Layout

Recommended campaign layout:

| Purpose | Path |
| --- | --- |
| Reusable landscape definitions | `data/landscape.ron` |
| Landscape mesh registry | `data/landscape_mesh_registry.ron` |
| Imported landscape mesh RON files | `assets/meshes/landscape/*.ron` |
| Tree and foliage textures | `assets/textures/trees/*.png` |
| Other landscape textures | `assets/textures/landscape/<asset_slug>/*` |

The existing `assets/meshes/*.ron` files can be used as the seed assets, but the final campaign-facing registry should point at campaign-relative asset paths so runtime loading and packaging stay consistent.

## Phase 0: Architecture and Scope Alignment

### Deliverables

- Update `docs/reference/architecture.md` Section 4.2 with the new landscape identifiers, definition types, placement type, databases, and `Map.landscape_placements` field.
- Decide exact identifier ranges and aliases in `src/domain/types.rs`.
- Decide whether landscape meshes reuse `CreatureDefinition`/`MeshDefinition` directly or use a small `LandscapeMeshDefinition` wrapper around `MeshDefinition` for clearer validation and future editor metadata.
- Document how landscape differs from furniture:
  - Furniture is interactable indoor/outdoor structure or object content.
  - Landscape is static environmental decoration, usually outdoor, usually non-interactable, and often placed many times.

### Acceptance Criteria

- Architecture names, field names, and module placement are approved before implementation.
- No code changes to core data structures happen before this phase is complete.

## Phase 1: Fix Default Trees and Brush Asset Baseline

### Deliverables

- Audit the five existing mesh RON files in `assets/meshes/` and confirm their vertex data, UVs, material data, and `texture_path` values.
- Fix stale texture paths in the tree and brush RON files. The current imported paths should be replaced or backed by real campaign assets so every referenced texture exists.
- Create initial tutorial landscape definitions and mesh registry entries for:
  - Oak tree: `oak_tree_short.ron`
  - Pine tree: `pine_tree.ron`
  - Dead tree: `dead_tree.ron`
  - Palm tree: `palm_tree.ron`
  - Brush/shrub: `brush.ron`
- Add matching self-contained test fixture entries under `data/test_campaign` for loader and round-trip tests.
- Repair the procedural fallback tree materials so default tree rendering is acceptable even when no landscape mesh is registered. Specifically, verify `bark.png` and the `foliage_*.png` assets are used or intentionally replaced by mesh geometry/material colors.
- Map existing procedural variants to imported defaults where practical:
  - `TreeType::Oak` → oak landscape definition
  - `TreeType::Pine` → pine landscape definition
  - `TreeType::Dead` → dead tree landscape definition
  - `TreeType::Palm` → palm landscape definition
  - `TreeType::Shrub` → brush/shrub landscape definition

### Notes

The current procedural code has tree texture constants, but foliage material creation intentionally avoids loading the foliage texture. This phase should make an explicit decision: either restore texture-backed foliage with proper alpha handling, or make imported RON landscape meshes the preferred tree path and keep procedural trees as a simpler fallback.

### Acceptance Criteria

- Default tutorial trees and brush render with valid textures or intentionally texture-free species materials.
- Missing texture paths produce useful diagnostics rather than silent invisible/white assets.
- Test fixtures do not reference `campaigns/tutorial`.

## Phase 2: Domain Loading, Validation, and Serialization

### Deliverables

- Add landscape definition and mesh database types with `///` documentation and tests.
- Extend campaign/game data loading to load `data/landscape.ron` and `data/landscape_mesh_registry.ron` as optional campaign data.
- Extend map RON serialization and deserialization with `landscape_placements`, defaulting to an empty vector and skipping serialization when empty.
- Add validation:
  - every placement references an existing `LandscapeDefinition`,
  - every definition `mesh_id` references an existing landscape mesh registry entry,
  - every mesh texture path starts with `assets/`,
  - every referenced texture exists when validating a campaign directory,
  - placement positions are inside map bounds,
  - blocking landscape definitions do not conflict with essential map movement rules.
- Add migration-safe defaults for maps that do not yet contain landscape placements.

### Acceptance Criteria

- Existing maps load unchanged.
- New maps can round-trip landscape placements through RON.
- Loader tests use `CampaignLoader::new("data")` with `test_campaign` or `data/test_campaign` path-based fixtures.

## Phase 3: Runtime Rendering and Map Spawn Integration

### Deliverables

- Add a runtime landscape spawning system that runs during map spawn after terrain and before NPCs/items, or in a clearly documented order that avoids z-fighting.
- Reuse the existing mesh conversion pipeline for imported `MeshDefinition` assets where possible.
- Add a `LandscapeEntity` component with map ID, landscape ID, and placement metadata for cleanup and debugging.
- Apply placement transform data: tile center, sub-tile offset, y-offset, rotation, scale, and tint.
- Support optional blocking/collision metadata without making every landscape object block movement by default.
- Preserve existing terrain-tied procedural visuals as fallback behavior for forest, grass, and mountain tiles.
- Add culling/LOD follow-up hooks for dense placement scenarios.

### Acceptance Criteria

- A map can render multiple placed landscape meshes on the same tile.
- Imported trees render from registry definitions when placed in the map editor.
- Procedural tree/grass/rock rendering still works for terrain-only maps.
- Unloading/reloading a map despawns old landscape entities cleanly.

## Phase 4: Importer Landscape Export Support

### Deliverables

- Add `ExportType::Landscape` to `sdk/campaign_builder/src/obj_importer.rs`.
- Add an importer `Landscape` radio option and category dropdown using `ComboBox::from_id_salt`.
- Add landscape category helpers and tests similar to the item and furniture category helpers.
- Export selected OBJ/GLB meshes to the landscape mesh destination.
- Copy OBJ/MTL or GLB textures to a campaign-relative landscape texture directory.
- Upsert `data/landscape_mesh_registry.ron`.
- Upsert `data/landscape.ron` with a reusable `LandscapeDefinition` referencing the new `mesh_id`.
- Emit an importer UI signal, such as `ObjImporterUiSignal::Landscape`, so the Campaign Builder reloads landscape definitions after export.
- Add landscape ID suggestion/allocation that uses `LandscapeId` and `LandscapeMeshId` aliases.

### egui Requirements

- Every loop body added to importer UI must use `ui.push_id` with a stable ID.
- Every new `ScrollArea` must use a distinct `id_salt`.
- Every new `ComboBox` must use `from_id_salt`.
- Any layout-driving state change must call `request_repaint()`.

### Acceptance Criteria

- Importing a model as Landscape creates both registry and definition data.
- The new landscape appears in the SDK without restarting the Campaign Builder.
- Texture paths are portable and campaign-relative.

## Phase 5: SDK Landscape Editor and Map Placement

### Deliverables

- Add a Landscape editor/palette or extend an existing asset editor with a dedicated Landscape tab.
- Show landscape definitions grouped by `LandscapeCategory`.
- Provide preview metadata: icon, name, category, scale, tags, mesh availability, and texture validation status.
- Add a `PlaceLandscape` map editor tool.
- Add placement inspector controls for selected landscape instances: definition, position, offset, rotation, scale, tint, and blocking flag override if supported.
- Extend undo/redo with landscape placement add/remove/replace actions.
- Add map grid overlay markers for landscape placements without obscuring terrain, events, NPCs, and dropped items.
- Add duplicate, delete, rotate, and random-variation conveniences for rapid forest/brush placement.
- Ensure map save applies current metadata and landscape placements before writing RON.

### egui Requirements

If the Landscape editor or placement panel uses multi-column layout, it must follow the project layout rule: compute `available_size()` before `ui.horizontal`, allocate every column with `ui.allocate_ui(egui::vec2(width, col_h), ...)`, use `.auto_shrink([true, false])` on column scroll areas, and put navigation hints in the title row rather than a separate bottom bar.

### Acceptance Criteria

- A user can select a landscape asset and place it on a map tile.
- A user can edit, move, rotate, scale, duplicate, and delete landscape placements.
- Map RON contains stable `landscape_placements` data after save.
- Reopening the campaign shows the same placements.

## Phase 6: Testing, Fixtures, and Quality Gates

### Required Tests

- Domain tests for `LandscapeDefinition`, `LandscapePlacement`, category serialization, defaults, and validation errors.
- Mesh registry tests for valid/invalid texture paths.
- Campaign loader tests using `data/test_campaign`.
- Map RON round-trip tests proving `landscape_placements` survives serialization.
- Runtime spawning tests for transform application and cleanup behavior where feasible.
- Importer export tests for registry upsert, definition upsert, existing-definition update, category mapping, and texture copy handling.
- SDK map editor state tests for add/remove/replace placement actions and undo/redo.

### Fixture Requirements

- Add all new fixture files to `data/test_campaign`.
- Do not reference `campaigns/tutorial` from tests.
- Keep tutorial campaign data as live content only; mirror minimal landscape assets into `data/test_campaign` when tests need them.

### Quality Gates

Run the project quality gates after implementation work:

- `cargo fmt --all`
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo nextest run --all-features`

## Phase 7: Documentation and Cleanup

### Deliverables

- Update `docs/explanation/implementations.md` for each implementation slice.
- Update modding or SDK documentation only if the feature is complete enough for users.
- Add comments and doc examples to every public type and function introduced for landscape support.
- Remove or clearly deprecate any old tree/brush hardcoded paths that are superseded by landscape definitions.
- Ensure tutorial campaign data uses the same validated format as the SDK exporter.

## Suggested Implementation Order

1. Architecture update and data model approval.
2. Tree/brush asset audit and texture-path repair.
3. Domain landscape definitions, databases, and map placement serialization.
4. Loader and validation support.
5. Runtime rendering of placed landscape meshes.
6. Importer `Landscape` export path.
7. SDK Landscape editor and Map editor placement tool.
8. Fixture expansion, integration tests, and quality gates.
9. Final documentation and cleanup.

## Final Acceptance Criteria

- The importer exposes `Landscape` as a first-class export target.
- Imported trees, shrubs, brush, and rocks become reusable landscape definitions.
- The Map editor can place, edit, save, reload, and delete landscape placements.
- Runtime maps render placed landscape meshes with correct textures.
- Default tree and brush visuals are fixed, with imported RON meshes preferred where configured and procedural generation retained as fallback.
- All new data uses RON.
- All test data lives under `data/test_campaign`.
- The four Rust quality gates pass with zero warnings and zero errors.
