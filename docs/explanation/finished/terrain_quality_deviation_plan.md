# Terrain Quality Deviation Correction Implementation Plan

## Overview

Correct the remaining terrain-quality deviations that still matter for runtime correctness, auditability, generated asset fidelity, Campaign Builder validation, and documentation structure while preserving the currently approved retained design choices.

This plan is the **sole canonical plan file** for terrain deviation correction:

- `docs/explanation/terrain_quality_deviation_plan.md`

This plan covers all required work across:

- Game Engine runtime material handling
- tree texture generator fidelity
- regenerated runtime asset outputs
- Campaign Builder / SDK asset validation
- implementation-summary and plan documentation updates

This plan is a planning artifact only. It does not describe completed work. Every task below is written as a direct instruction for a future implementation agent.

## Current State Analysis

### Existing Infrastructure

| Area                                      | File / Symbol                                                                                          | Verified Current State                                                                                                 |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| Terrain material cache resource           | `src/game/resources/terrain_material_cache.rs` / `TerrainMaterialCache`                                | Cache exists and stores one optional `Handle<StandardMaterial>` per terrain type                                       |
| Terrain material loading                  | `src/game/systems/terrain_materials.rs` / `load_terrain_materials_system`                              | Startup system creates textured `StandardMaterial` values with `base_color_texture: Some(_)`                           |
| Map terrain rendering                     | `src/game/systems/map.rs` / `spawn_map`                                                                | `spawn_map` accepts `terrain_cache: &TerrainMaterialCache`                                                             |
| Terrain cache wrappers                    | `src/game/systems/map.rs` / `spawn_map_system`, `handle_door_opened`, `spawn_map_markers`              | Startup wrapper uses `Res<TerrainMaterialCache>` and refresh wrappers use `Option<Res<TerrainMaterialCache>>`          |
| Tree mesh cache API                       | `src/game/systems/procedural_meshes.rs` / `ProceduralMeshCache::get_or_create_tree_mesh`               | Function accepts only `tree_type` and `meshes`; bark-material creation is handled by other cache methods               |
| Tree texture generation                   | `src/bin/generate_terrain_textures.rs` / `generate_tree_textures`, `generate_foliage_texture`          | Multiple foliage outputs are currently generated through a mostly circular silhouette helper                           |
| Terrain implementation summary            | `docs/explanation/implementations.md`                                                                  | Terrain work is currently split across three top-level terrain sections instead of one grouped section                 |
| Campaign Builder graphics UI              | `sdk/campaign_builder/src/config_editor.rs` / `show_graphics_quality_section`, `show_graphics_section` | Builder exposes graphics and grass-performance settings but no terrain-deviation-specific workflow                     |
| Campaign Builder asset validation surface | `sdk/campaign_builder/src/asset_manager.rs`                                                            | Existing asset-management surface exists and may be extended for terrain texture validation without adding a new panel |

### Identified Issues

| ID     | Issue                                                                                                         | Scope              | Required Resolution                                               |
| ------ | ------------------------------------------------------------------------------------------------------------- | ------------------ | ----------------------------------------------------------------- |
| TQD-01 | Tinted terrain material paths do not consistently preserve cached textures                                    | Game Engine        | Add tint-preserving material cloning and branch integration       |
| TQD-02 | Retained structural deviations are not recorded in one explicit canonical section                             | Documentation      | Record approved retained deviations in this file                  |
| TQD-03 | Tree foliage generator output is too generic for pine, willow, palm, and shrub silhouette requirements        | Generator + assets | Add shape-specific deterministic generation and regenerate assets |
| TQD-04 | `implementations.md` terrain structure does not match the grouped heading layout expected by the terrain plan | Documentation      | Group terrain work under one shared heading                       |
| TQD-05 | Campaign Builder / SDK impact is not yet resolved as an explicit scope decision                               | SDK                | Add required builder-side validation work                         |
| TQD-06 | Generated PNG outputs are not explicitly listed as required deliverables of generator changes                 | Generator + assets | Require regeneration and commit of exact files                    |
| TQD-07 | Current foliage-validation language is partially qualitative instead of machine-checkable                     | Tests              | Replace with explicit measurable metrics                          |

## Approved Retained Deviations

The following deviations are approved design choices. They are **not** implementation targets for change in this plan.

| Deviation                                                                                                                       | Current Code Reference                                                                        | Fixed Decision                                                                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Bark tinting remains enabled                                                                                                    | `src/game/systems/procedural_meshes.rs` / tree bark material usage inside tree spawning paths | Preserve bark tinting. Do not disable bark `color_tint` support.                                                                   |
| `spawn_map` uses `&TerrainMaterialCache` internally                                                                             | `src/game/systems/map.rs` / `spawn_map`                                                       | Preserve `spawn_map(..., terrain_cache: &TerrainMaterialCache, ...)`. Do not convert this boundary to `Res<TerrainMaterialCache>`. |
| Wrapper systems may use optional terrain-cache access for fallback-safe refresh behavior                                        | `src/game/systems/map.rs` / `handle_door_opened`, `spawn_map_markers`                         | Preserve `Option<Res<TerrainMaterialCache>>` where it supports refresh/test fallback behavior. Treat this as the approved design.  |
| `ProceduralMeshCache::get_or_create_tree_mesh` does not preload bark materials and does not accept `asset_server` / `materials` | `src/game/systems/procedural_meshes.rs` / `ProceduralMeshCache::get_or_create_tree_mesh`      | Preserve the current signature unless a concrete correctness defect is demonstrated. Do not expand the signature in this plan.     |

## Implementation Phases

### Phase 1: Correct Runtime Tinted Terrain Material Handling

#### 1.1 Foundation Work

Review every terrain-material creation path in `src/game/systems/map.rs` used by `spawn_map`.

Create a branch inventory table in implementation notes or planning notes with these fields:

| Required Field                 | Description                                 |
| ------------------------------ | ------------------------------------------- |
| `terrain_variant`              | Exact `TerrainType` branch name             |
| `branch_location`              | Exact symbol name and line range            |
| `uses_cache`                   | `true` or `false`                           |
| `supports_tint`                | `true` or `false`                           |
| `preserves_base_color_texture` | `true` or `false`                           |
| `fallback_behavior`            | Exact non-cache fallback material behavior  |
| `branch_class`                 | `floor`, `terrain-special`, or `wall/other` |

The review must cover at minimum:

- `TerrainType::Mountain` in `spawn_map`
- `TerrainType::Water` in `spawn_map`
- `TerrainType::Forest | TerrainType::Grass` in `spawn_map`
- the `_` catch-all floor branch in `spawn_map`

Do not skip any branch that constructs a `StandardMaterial` directly or indirectly.

#### 1.2 Add Foundation Functionality

Add one private helper in `src/game/systems/map.rs` with one responsibility:

- create a terrain material handle for a terrain tile while preserving cached texture data when tint is applied

Define the helper contract exactly as follows.

| Input                 | Type                            |
| --------------------- | ------------------------------- |
| `terrain`             | `TerrainType`                   |
| `tint`                | `Option<(f32, f32, f32)>`       |
| `terrain_cache`       | `&TerrainMaterialCache`         |
| `materials`           | `&Assets<StandardMaterial>`     |
| `materials_mut`       | `&mut Assets<StandardMaterial>` |
| `fallback_base_color` | `Color`                         |
| `fallback_roughness`  | `f32`                           |

| Output       | Type                       | Required Behavior                                                                                              |
| ------------ | -------------------------- | -------------------------------------------------------------------------------------------------------------- |
| return value | `Handle<StandardMaterial>` | Return cached handle unchanged when `tint == None`; return a newly added material handle when `tint.is_some()` |

The helper must obey all of these rules:

1. If a cached source material exists and `tint.is_some()`, clone the cached source material asset from `materials`.
2. Preserve `base_color_texture` from the cached source material.
3. Preserve all fields from the cached source material by default.
4. Override only:
   - `base_color`
   - `perceptual_roughness` only when the calling branch already sets it explicitly
5. If no cached source material exists, use the existing flat-color fallback behavior.
6. Never mutate the cached source material asset in place.

#### 1.3 Integrate Foundation Work

Replace all tint-applying terrain branches in `spawn_map` that currently create one-off flat-color materials.

Required integration targets:

| Target ID | Location                                                                | Required Change                                                                                                |
| --------- | ----------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| RTM-01    | `TerrainType::Mountain` branch in `src/game/systems/map.rs`             | Replace flat tinted `StandardMaterial` creation with helper usage                                              |
| RTM-02    | Any non-mountain terrain branch that applies `tile.visual.color_tint`   | Route through the helper and preserve cached textures                                                          |
| RTM-03    | Any branch that may later support tint through `tile.visual.color_tint` | Define behavior explicitly: either support tint through helper or document tint as unsupported for that branch |

Preserve all of these boundaries:

- `spawn_map` must keep `terrain_cache: &TerrainMaterialCache`
- `spawn_map_system` may continue to use `Res<TerrainMaterialCache>`
- `handle_door_opened` and `spawn_map_markers` may continue to use `Option<Res<TerrainMaterialCache>>`

#### 1.4 Testing Requirements

Add or update machine-verifiable tests.

Required runtime tests:

| Test File                               | Test Requirement                                                                                 |
| --------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `src/game/systems/terrain_materials.rs` | Verify all 9 cache-created terrain materials have `base_color_texture.is_some() == true`         |
| `src/game/systems/map.rs`               | Verify the helper returns the cached handle unchanged when `tint == None`                        |
| `src/game/systems/map.rs`               | Verify the helper returns a distinct new handle when `tint.is_some()` and a cached source exists |
| `src/game/systems/map.rs`               | Verify the cloned tinted material preserves `base_color_texture.is_some() == true`               |
| `src/game/systems/map.rs`               | Verify fallback behavior still works when `TerrainMaterialCache` is absent or empty in tests     |
| `src/game/systems/map.rs`               | Verify cached terrain materials are not mutated in place by tint application                     |

Run the required quality gates after implementation:

1. `cargo fmt --all`
2. `cargo check --all-targets --all-features`
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `cargo nextest run --all-features`

#### 1.5 Deliverables

- [ ] `src/game/systems/map.rs` contains a private helper for tint-preserving terrain material creation
- [ ] `TerrainType::Mountain` tint handling preserves `base_color_texture`
- [ ] Every terrain branch that supports tint preserves `base_color_texture` when cached material data exists
- [ ] Cached terrain materials are never mutated in place during tint application
- [ ] `spawn_map` retains the `&TerrainMaterialCache` boundary
- [ ] Wrapper systems retain existing `Res<TerrainMaterialCache>` / `Option<Res<TerrainMaterialCache>>` patterns
- [ ] Runtime tests cover cache presence, cache absence, tinted clone behavior, and handle identity rules
- [ ] All four quality gates pass with zero errors and zero warnings

#### 1.6 Success Criteria

| Criterion ID | Automatically Verifiable Condition                                                                       |
| ------------ | -------------------------------------------------------------------------------------------------------- |
| SC-RT-01     | No tint-supporting terrain branch creates a flat-color substitute when a cached textured material exists |
| SC-RT-02     | All cloned tinted materials preserve `base_color_texture.is_some() == true`                              |
| SC-RT-03     | `spawn_map` signature still includes `terrain_cache: &TerrainMaterialCache`                              |
| SC-RT-04     | Project quality gates pass with zero errors and zero warnings                                            |

---

### Phase 2: Refine Tree Texture Generator and Regenerate Runtime Assets

#### 2.1 Foundation Work

Review `src/bin/generate_terrain_textures.rs` and document the exact current generation path for each tree output file.

Create a planning table with these columns:

| Column              | Description                    |
| ------------------- | ------------------------------ |
| `filename`          | Exact output filename          |
| `current_generator` | Exact helper currently used    |
| `dimensions`        | Current width × height         |
| `required_shape`    | Target silhouette requirement  |
| `seed`              | Exact deterministic seed value |

The table must include all of the following files:

- `bark.png`
- `foliage_oak.png`
- `foliage_pine.png`
- `foliage_birch.png`
- `foliage_willow.png`
- `foliage_palm.png`
- `foliage_shrub.png`

#### 2.2 Add Generator Functionality

Refactor the tree-texture generator so each foliage type uses deterministic shape-specific alpha-mask logic instead of a shared mostly circular silhouette.

Required generator targets:

| File                 | Dimensions | Required Silhouette                                                |
| -------------------- | ---------- | ------------------------------------------------------------------ |
| `bark.png`           | `64×128`   | Fully opaque bark; no silhouette change required                   |
| `foliage_oak.png`    | `128×128`  | Wide rounded crown                                                 |
| `foliage_pine.png`   | `64×128`   | Tall narrow taper with stronger center-column occupancy than edges |
| `foliage_birch.png`  | `128×128`  | Rounded but lighter / sparser than oak                             |
| `foliage_willow.png` | `128×128`  | Downward-heavy drooping silhouette                                 |
| `foliage_palm.png`   | `128×128`  | Radial fan with multiple separated frond lobes                     |
| `foliage_shrub.png`  | `64×64`    | Compact dense low-profile bush                                     |

Implementation rules for this phase:

- Use either one dedicated helper per silhouette or one generic helper with exact per-shape parameter sets and shape-selection logic.
- Preserve output filenames exactly.
- Preserve output directories exactly.
- Preserve deterministic seeds exactly.
- Preserve bark opacity rules exactly.
- Preserve image dimensions exactly.
- Preserve the existing binary entry point exactly.

#### 2.3 Regenerate and Validate Runtime Asset Outputs

After generator changes are complete, rerun the texture generator binary and update the committed runtime asset files in:

- `assets/textures/trees/bark.png`
- `assets/textures/trees/foliage_oak.png`
- `assets/textures/trees/foliage_pine.png`
- `assets/textures/trees/foliage_birch.png`
- `assets/textures/trees/foliage_willow.png`
- `assets/textures/trees/foliage_palm.png`
- `assets/textures/trees/foliage_shrub.png`

Do not add a new output directory.
Do not rename any texture file.
Do not move tree textures outside `assets/textures/trees/`.

#### 2.4 Testing Requirements

Use measurable image metrics only. Do not use subjective visual descriptions in tests.

Required automated tests:

| Test Target         | Metric                                                                                |
| ------------------- | ------------------------------------------------------------------------------------- |
| `bark` output       | All alpha values equal `255`                                                          |
| `oak` output        | Occupied bounding-box width is greater than shrub occupied bounding-box width         |
| `pine` output       | Central vertical occupancy ratio is greater than oak central vertical occupancy ratio |
| `pine` output       | Occupied width / occupied height ratio is lower than oak width / height ratio         |
| `birch` output      | Opaque-pixel count is lower than oak opaque-pixel count for the same dimensions       |
| `willow` output     | Lower-half opaque-pixel count is greater than upper-half opaque-pixel count           |
| `palm` output       | At least 4 non-empty angular sectors exist outside the center radius                  |
| `shrub` output      | Occupied height ratio is lower than oak occupied height ratio                         |
| `shrub` output      | Lower-third density is greater than oak lower-third density                           |
| all foliage outputs | Raw pixel data is deterministic for fixed seeds                                       |
| all foliage outputs | At least one fully transparent outer-region pixel remains present                     |

Run the required quality gates after implementation:

1. `cargo fmt --all`
2. `cargo check --all-targets --all-features`
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `cargo nextest run --all-features`

#### 2.5 Deliverables

- [ ] `src/bin/generate_terrain_textures.rs` uses shape-specific foliage generation logic
- [ ] All required tree texture dimensions remain unchanged
- [ ] Deterministic seeds remain unchanged
- [ ] Tree textures are regenerated into `assets/textures/trees/`
- [ ] Automated tests verify measurable silhouette properties rather than subjective descriptions
- [ ] No runtime loader path changes are introduced
- [ ] All four quality gates pass with zero errors and zero warnings

#### 2.6 Success Criteria

| Criterion ID | Automatically Verifiable Condition                                        |
| ------------ | ------------------------------------------------------------------------- |
| SC-TX-01     | Tree texture tests assert deterministic and shape-specific output metrics |
| SC-TX-02     | Output files remain in `assets/textures/trees/` with unchanged filenames  |
| SC-TX-03     | Generated texture dimensions exactly match required values                |
| SC-TX-04     | Project quality gates pass with zero errors and zero warnings             |

---

### Phase 3: Add Campaign Builder / SDK Terrain Asset Validation

#### 3.1 Foundation Work

Audit the existing Campaign Builder / SDK asset-validation surfaces before adding code.

Review at minimum:

- `sdk/campaign_builder/src/asset_manager.rs`
- `sdk/campaign_builder/src/config_editor.rs`
- `src/sdk/validation.rs`
- any builder-visible validation or asset-listing paths that can expose terrain texture diagnostics without creating a new dedicated panel

Record the audit result in a planning table with these fields:

| Field                        | Description                     |
| ---------------------------- | ------------------------------- |
| `component`                  | file + symbol                   |
| `terrain_feature_dependency` | `none`, `indirect`, or `direct` |
| `required_change`            | exact change or `none`          |
| `reason`                     | explicit justification          |

This phase has a fixed scope decision:

- `Campaign Builder / SDK scope decision: code changes required`

#### 3.2 Add Validation Functionality

Add builder-side asset validation in existing asset/validation surfaces only. Do not create a new terrain-quality panel.

Required validation targets:

| Validation ID | Required Check                                             | Expected Failure Condition                              |
| ------------- | ---------------------------------------------------------- | ------------------------------------------------------- |
| SDK-TEX-01    | Required tree texture filename exists                      | Missing `bark.png` or any required `foliage_*.png` file |
| SDK-TEX-02    | Required tree texture filename matches exact expected name | Misnamed file present instead of required filename      |
| SDK-TEX-03    | `bark.png` dimensions equal `64×128`                       | Bark image dimensions differ from expected values       |
| SDK-TEX-04    | `foliage_oak.png` dimensions equal `128×128`               | Oak image dimensions differ from expected values        |
| SDK-TEX-05    | `foliage_pine.png` dimensions equal `64×128`               | Pine image dimensions differ from expected values       |
| SDK-TEX-06    | `foliage_birch.png` dimensions equal `128×128`             | Birch image dimensions differ from expected values      |
| SDK-TEX-07    | `foliage_willow.png` dimensions equal `128×128`            | Willow image dimensions differ from expected values     |
| SDK-TEX-08    | `foliage_palm.png` dimensions equal `128×128`              | Palm image dimensions differ from expected values       |
| SDK-TEX-09    | `foliage_shrub.png` dimensions equal `64×64`               | Shrub image dimensions differ from expected values      |

Validation rules for this phase:

1. Surface diagnostics through existing Campaign Builder asset/validation views only.
2. Do not add a dedicated terrain-quality UI panel.
3. Use explicit diagnostic messages that include:
   - expected filename
   - actual missing/mismatched asset
   - expected dimensions
   - actual dimensions when available
4. Keep validation scoped to tree texture file presence, filename exactness, and dimensions.
5. Do not add validation for subjective silhouette quality in the Campaign Builder.

#### 3.3 Integrate Validation Work

Integrate the new tree-texture validation into the existing builder/SDK validation flow so campaign authors can discover texture issues through existing asset or validation surfaces.

If any egui UI code is modified, comply with `sdk/AGENTS.md` requirements:

- wrap every loop body with `push_id`
- assign `id_salt` to every `ScrollArea`
- use `ComboBox::from_id_salt` for every combo box
- do not introduce same-frame panel registration bugs
- call `request_repaint()` for layout-driving state mutation when required

Do not create a new dedicated terrain-quality screen.

#### 3.4 Testing Requirements

Add exact SDK test coverage for filename and dimension validation.

Required SDK tests:

| Test Target                                                             | Required Assertion                                                                 |
| ----------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `sdk/campaign_builder/src/asset_manager.rs` or linked validation module | Missing required tree texture file produces a validation error                     |
| `sdk/campaign_builder/src/asset_manager.rs` or linked validation module | Misnamed tree texture file does not satisfy exact filename validation              |
| `sdk/campaign_builder/src/asset_manager.rs` or linked validation module | Bark dimension mismatch produces a validation error with expected and actual sizes |
| `sdk/campaign_builder/src/asset_manager.rs` or linked validation module | Pine dimension mismatch produces a validation error with expected and actual sizes |
| `sdk/campaign_builder/src/asset_manager.rs` or linked validation module | Fully valid required tree texture set passes validation                            |

If any egui UI code is modified, include an egui ID audit in implementation notes covering all changed loops, scroll areas, combo boxes, and panel registration behavior.

Run the required quality gates after implementation:

1. `cargo fmt --all`
2. `cargo check --all-targets --all-features`
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `cargo nextest run --all-features`

#### 3.5 Deliverables

- [ ] Builder-side tree texture validation is added to existing asset/validation surfaces
- [ ] Validation checks required filenames exactly
- [ ] Validation checks required tree texture image dimensions exactly
- [ ] Validation does not create a new dedicated terrain-quality panel
- [ ] Validation diagnostics include expected and actual values
- [ ] SDK tests cover missing files, misnamed files, dimension mismatches, and valid asset sets
- [ ] If UI code changes are made, `sdk/AGENTS.md` egui requirements are satisfied
- [ ] All four quality gates pass with zero errors and zero warnings

#### 3.6 Success Criteria

| Criterion ID | Automatically Verifiable Condition                                                                 |
| ------------ | -------------------------------------------------------------------------------------------------- |
| SC-SDK-01    | Existing builder/SDK asset-validation surfaces report missing or mismatched required tree textures |
| SC-SDK-02    | Exact filename and exact dimension validation both exist                                           |
| SC-SDK-03    | No new dedicated terrain-quality panel is added                                                    |
| SC-SDK-04    | Project quality gates pass with zero errors and zero warnings                                      |

---

### Phase 4: Normalize Terrain Documentation Structure and Cross-References

#### 4.1 Foundation Work

Review terrain documentation in all of the following files:

- `docs/explanation/implementations.md`
- `docs/explanation/finished/terrain_quality_improvement_implementation_plan.md`
- `docs/explanation/terrain_quality_deviation_plan.md`

Identify all terrain-specific implementation-summary content that must be preserved exactly:

- overviews
- deliverables
- tests
- quality-gate notes
- architecture compliance notes
- phase headings
- “What Was Built” sections

#### 4.2 Add Documentation Functionality

Restructure terrain content in `docs/explanation/implementations.md` under this exact heading hierarchy:

- `## Terrain Quality Improvement`
- `### Phase 1: Terrain Texture Foundation`
- `### Phase 2: High-Quality Grass`
- `### Phase 3: High-Quality Tree Models`

Preserve existing terrain summary content. Change heading structure only where necessary.

Add one short cross-reference subsection near the start of the grouped terrain section that points to:

- `docs/explanation/terrain_quality_deviation_plan.md`

The cross-reference must state both:

- `implementations.md` documents delivered terrain work
- `terrain_quality_deviation_plan.md` documents remaining correction work and approved retained deviations

Do not create a second terrain deviation plan file.

#### 4.3 Integrate Documentation Work

Update `docs/explanation/implementations.md` after every code-bearing phase is complete, not only at the end of all phases.

Required implementation-summary updates:

| Code-Bearing Phase | Required Summary Update                                         |
| ------------------ | --------------------------------------------------------------- |
| Phase 1            | Summarize tint-preserving terrain material changes and tests    |
| Phase 2            | Summarize tree generator changes, regenerated assets, and tests |
| Phase 3            | Summarize Campaign Builder / SDK validation work and tests      |

Apply final grouped terrain heading normalization after those summaries are present.

#### 4.4 Testing Requirements

Documentation validation requirements:

| Validation ID | Required Check                                                                     |
| ------------- | ---------------------------------------------------------------------------------- |
| DOC-01        | `implementations.md` contains one grouped `## Terrain Quality Improvement` section |
| DOC-02        | Exactly three nested terrain phase headings exist beneath that grouped section     |
| DOC-03        | The grouped terrain section references `terrain_quality_deviation_plan.md`         |
| DOC-04        | No second terrain deviation plan file is created for the same scope                |

Run the required quality gates after documentation changes:

1. `cargo fmt --all`
2. `cargo check --all-targets --all-features`
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `cargo nextest run --all-features`

#### 4.5 Deliverables

- [ ] `docs/explanation/implementations.md` contains one grouped `## Terrain Quality Improvement` section
- [ ] The grouped terrain section contains exactly three nested phase headings
- [ ] Existing terrain summary content is preserved
- [ ] The grouped terrain section cross-references `docs/explanation/terrain_quality_deviation_plan.md`
- [ ] `implementations.md` is updated after each code-bearing phase
- [ ] No second terrain deviation plan file is introduced
- [ ] All four quality gates pass with zero errors and zero warnings

#### 4.6 Success Criteria

| Criterion ID | Automatically Verifiable Condition                                                   |
| ------------ | ------------------------------------------------------------------------------------ |
| SC-DOC-01    | `implementations.md` contains one grouped terrain section                            |
| SC-DOC-02    | The grouped terrain section contains `### Phase 1`, `### Phase 2`, and `### Phase 3` |
| SC-DOC-03    | The grouped terrain section references the canonical terrain deviation plan          |
| SC-DOC-04    | Project quality gates pass with zero errors and zero warnings                        |

## Out of Scope

The following items are explicitly out of scope for this plan unless a later approved requirement changes them.

| Out-of-Scope Item                                                                        | Reason                                                          |
| ---------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Changing `spawn_map` from `&TerrainMaterialCache` to `Res<TerrainMaterialCache>`         | The current boundary is an approved retained deviation          |
| Changing `ProceduralMeshCache::get_or_create_tree_mesh` signature without a concrete bug | The current API difference is approved unless correctness fails |
| Disabling bark tinting                                                                   | Bark tinting is retained by design                              |
| Moving tree textures to a new asset directory                                            | Existing runtime loading paths must remain stable               |
| Adding a new dedicated terrain-quality Campaign Builder panel                            | Validation must live in existing asset/validation surfaces only |
| Adding campaign data schema changes                                                      | No current evidence requires new RON schema                     |
| Using `campaigns/tutorial` for tests                                                     | Project rules require stable fixtures under `data/`             |

## Quality Gates

Run the repository-required validation commands in this exact order after each code-bearing phase and after the final documentation phase:

1. `cargo fmt --all`
2. `cargo check --all-targets --all-features`
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `cargo nextest run --all-features`

Expected result for every phase:

- zero formatting issues after `cargo fmt --all`
- zero compile errors
- zero clippy warnings
- zero failing tests

## Final Plan Acceptance Criteria

The rewritten plan is acceptable only if all of the following are true.

| Acceptance ID | Requirement                                                                                                 |
| ------------- | ----------------------------------------------------------------------------------------------------------- |
| ACP-01        | One canonical plan file is used for terrain deviation correction                                            |
| ACP-02        | Game Engine runtime work is fully specified with an exact helper contract and branch targets                |
| ACP-03        | Tree texture generator work includes code changes, regenerated assets, and machine-checkable tests          |
| ACP-04        | Campaign Builder / SDK impact is explicitly resolved as required builder-side asset validation              |
| ACP-05        | Builder-side validation includes exact filename checks and exact dimension checks                           |
| ACP-06        | Builder-side validation is added only to existing asset/validation surfaces                                 |
| ACP-07        | Documentation tasks cover both `terrain_quality_deviation_plan.md` and `implementations.md`                 |
| ACP-08        | Out-of-scope boundaries are explicit                                                                        |
| ACP-09        | Deliverables are concrete and automatically verifiable                                                      |
| ACP-10        | Phase order follows implementation dependency order with code work before final documentation normalization |
