# New MTL Support Plan

## Overview

This branch already includes a dedicated SDK importer tab, importer state, a
built-in plus campaign-scoped palette workflow, and creature or item RON
export. The plan is no longer to invent a new import surface. The plan is to
extend the existing importer so OBJ plus MTL input carries material assignments,
imported colors, and optional material metadata cleanly through the current
`Importer` tab.

The key change is to move MTL handling into the existing importer pipeline:

- parse `mtllib` and `usemtl` in `sdk/campaign_builder/src/mesh_obj_io.rs`
- load sidecar or manually selected `.mtl` files
- map imported materials into `MeshDefinition.color` and
  `MeshDefinition.material`
- surface imported material swatches inside the current importer UI
- preserve the existing export and palette persistence workflow

## Branch Reality Check

The branch now contains the importer infrastructure that the old plan assumed
was missing.

- `sdk/campaign_builder/src/lib.rs` already exposes an `Importer` tab.
- `sdk/campaign_builder/src/obj_importer.rs` already owns importer state,
  active mesh selection, palette persistence, and OBJ loading.
- `sdk/campaign_builder/src/obj_importer_ui.rs` already renders the standalone
  importer tab, mesh list, color editor, built-in palette, custom palette, and
  creature or item export.
- `sdk/campaign_builder/src/color_palette.rs` already persists campaign-scoped
  palette entries to `config/importer_palette.ron`.
- `sdk/campaign_builder/src/lib.rs` already synchronizes importer palette state
  and next creature ID when a campaign is loaded.
- `sdk/campaign_builder/src/mesh_obj_io.rs` still ignores `mtllib` and
  `usemtl`, so MTL support remains the missing core feature.
- `src/domain/visual/mod.rs` already has `MeshDefinition.material`,
  `MaterialDefinition`, `AlphaMode`, and `texture_path`, so the domain model is
  ready for richer import results.

## Scope Update

The new scope is narrower and more concrete than the original plan.

In scope:

- add MTL-aware parsing to the existing OBJ importer backend
- auto-detect sidecar `.mtl` files from OBJ `mtllib` directives
- allow manual `.mtl` override selection from the existing importer tab
- split imported meshes when `usemtl` changes within one object or group
- feed imported material colors into the current importer mesh list and color
  editor
- expose imported MTL swatches as a temporary import palette inside the
  existing tab
- allow users to persist useful imported swatches through the existing custom
  palette save flow
- populate `MeshDefinition.material` when the current domain model can express
  the MTL data without lossy hacks

Out of scope for the first pass:

- rebuilding the importer UI from scratch
- moving OBJ import into `creatures_editor.rs` or `item_mesh_editor.rs`
- introducing a separate palette storage format beyond
  `config/importer_palette.ron`
- full coverage of every MTL directive in the specification
- texture import workflows beyond a conservative `map_Kd` mapping if the path
  semantics are clearly correct

## Current Gaps

These are the practical problems to solve on this branch.

- `mesh_obj_io.rs` intentionally ignores `mtllib` and `usemtl` today.
- `ObjImportOptions` does not yet carry source-path or manual-MTL override
  context.
- `ImportedMesh::from_mesh_definition` currently auto-assigns palette colors by
  mesh name, which will need a priority rule so imported MTL colors can win.
- `obj_importer_ui.rs` can browse and load OBJ files, but it has no MTL file
  picker, no import-material summary, and no imported palette section.
- importer tests currently cover OBJ loading, color editing, and export, but not
  MTL parsing, material splitting, or imported palette behavior.

## Product Decisions

These decisions should drive implementation on this branch.

- Imported `Kd` colors should override mesh-name auto-color suggestions.
- Imported MTL colors should appear immediately on imported meshes.
- Imported material swatches should be visible in the importer UI for the
  current session.
- Users should be able to keep those colors by saving them through the existing
  campaign custom-palette flow.
- `usemtl` boundaries should split meshes because `MeshDefinition` does not
  support per-face materials.
- Object and group identity should be preserved when splitting on material
  boundaries.
- Missing or malformed `.mtl` files should degrade gracefully instead of failing
  the whole OBJ import when geometry is otherwise valid.

## Implementation Phases

### Phase 1: Rebaseline Around The Existing Importer

Update assumptions and target the real branch structure.

Goals:

- treat `obj_importer.rs` and `obj_importer_ui.rs` as the primary workflow
- keep the existing importer tab and export flow intact
- document the exact seam between parser changes and importer-state changes

Primary files:

- `sdk/campaign_builder/src/mesh_obj_io.rs`
- `sdk/campaign_builder/src/obj_importer.rs`
- `sdk/campaign_builder/src/obj_importer_ui.rs`
- `sdk/campaign_builder/src/color_palette.rs`
- `sdk/campaign_builder/src/lib.rs`
- `src/domain/visual/mod.rs`

Expected outcome:

The plan and implementation both target the current standalone importer tab,
not the older mesh-editor workflow.

### Phase 2: Refactor OBJ Parsing For Material-Aware Segments

Refactor `mesh_obj_io.rs` so parsing preserves enough structure for later MTL
resolution.

Goals:

- track `o`, `g`, `usemtl`, faces, normals, UVs, and source-path context
- preserve object and group names without letting material switches overwrite
  identity
- support mesh flushing on object, group, or material boundaries

Planned changes:

- introduce parsed segment structures that store active object, group, and
  material name
- separate low-level parsing from `MeshDefinition` construction
- preserve enough metadata to resolve materials after parse time

Expected outcome:

The importer can represent a multi-material OBJ deterministically before any UI
changes are made.

### Phase 3: Add MTL Parsing And Resolution

Teach the backend to find and parse `.mtl` files.

Goals:

- parse one or more `mtllib` directives from the OBJ
- resolve sidecar `.mtl` files relative to the OBJ directory
- support a manual `.mtl` override path supplied by importer state or UI
- parse a useful first-pass subset of MTL directives

First-pass directives:

- `newmtl`
- `Kd`
- `Ks`
- `Ke`
- `Ns`
- `d`
- `illum`
- optional `map_Kd`

Planned changes:

- extend `ObjImportOptions` with source-path and manual-override fields
- add MTL parsing helpers and path-resolution helpers
- support multiple material libraries instead of only the first reference
- keep non-fatal recovery for missing or partially invalid material data

Expected outcome:

Importing an OBJ file can automatically discover its materials, while the UI can
optionally point the importer at a different `.mtl` file.

### Phase 4: Map Imported Materials Into Domain Types

Populate the existing mesh and material types with imported data.

Mapping rules:

- `Kd` -> `MeshDefinition.color`
- `Kd` -> `MaterialDefinition.base_color`
- `d` -> alpha channel and `AlphaMode::Blend` when below `1.0`
- `Ke` -> `MaterialDefinition.emissive`
- `Ks` and `Ns` -> conservative heuristic for `metallic` and `roughness`, or a
  documented defer if the mapping is too lossy
- `map_Kd` -> `texture_path` only if the importer can preserve a useful relative
  path

Expected outcome:

The exported RON data preserves visible imported color and, when possible,
material intent.

### Phase 5: Integrate With Existing Importer State

Carry MTL-aware output through `ObjImporterState` without regressing current
behavior.

Goals:

- keep the current mesh list, active mesh selection, and export flow intact
- preserve imported MTL colors instead of immediately replacing them with
  name-based auto-colors
- retain mesh-name auto-assignment as the fallback when no material color exists

Planned changes:

- extend `ImportedMesh` or its construction path to remember whether color came
  from MTL or fallback heuristics
- update `load_obj_file` state setup to pass importer options needed for MTL
  resolution
- keep `auto_assign_colors()` useful as an explicit reset to built-in heuristics,
  not as an automatic override of imported colors

Expected outcome:

The importer state honors source materials while preserving the current editing
workflow.

### Phase 6: Extend The Existing Importer Tab

Add MTL controls and imported-material visibility to `obj_importer_ui.rs`.

Goals:

- show auto-detected `.mtl` status in the importer tab
- add manual `.mtl` override selection when auto-detection is absent or wrong
- show imported material swatches as a temporary session palette
- let users promote imported swatches into the existing custom campaign palette

Planned UI additions:

- an MTL source row in idle or loaded metadata
- a manual `.mtl` browse or clear action
- an imported-material palette section distinct from built-in and custom colors
- status text that explains whether colors came from MTL or fallback heuristics

SDK UI constraints from `sdk/AGENTS.md` still apply:

- wrap loops in `push_id`
- use distinct `id_salt` values for scroll areas
- use `from_id_salt` for any combo boxes that are added
- keep `TwoColumnLayout` for list or detail layouts
- call `request_repaint()` after layout-driving state changes

Expected outcome:

Users can see exactly which MTL file was used, override it if needed, apply
imported swatches, and save useful colors without leaving the importer tab.

### Phase 7: Tests

Add coverage for the new backend and UI-facing behavior.

Parser and importer tests in `sdk/campaign_builder/src/mesh_obj_io.rs`:

- relative `mtllib` resolution
- multiple `mtllib` directives
- missing `.mtl` file fallback behavior
- malformed MTL handling that still preserves OBJ geometry import
- `usemtl` boundary splitting
- object and group name preservation across material switches
- `Kd`, `d`, and `Ke` mapping into mesh and material data

State and UI tests in `sdk/campaign_builder/src/obj_importer.rs` and
`sdk/campaign_builder/src/obj_importer_ui.rs`:

- imported colors win over name-based auto-color assignment
- auto-assign remains available as an explicit user action
- manual MTL override changes imported colors as expected
- imported material swatches appear in importer session state
- custom palette persistence still writes to `config/importer_palette.ron`

Fixture policy:

- automated test fixtures should live under `data/`, not `campaigns/tutorial`
- large real-world meshes in `notes/meshes/` are fine for manual verification,
  but automated tests should use minimal stable fixtures checked into approved
  test-fixture locations

Expected outcome:

The new MTL workflow is covered at the parser, state, and importer-tab levels.

### Phase 8: Validation And Documentation

Finish with repo validation and implementation notes.

Validation order:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

Documentation updates:

- `docs/explanation/implementations.md`

Document at minimum:

- the final priority rule between MTL colors and built-in auto-colors
- the chosen `Ks` and `Ns` mapping behavior, if any
- unsupported or deferred MTL directives
- how imported swatches differ from built-in and custom palette entries

## Relevant Files

- `sdk/campaign_builder/src/mesh_obj_io.rs`
- `sdk/campaign_builder/src/obj_importer.rs`
- `sdk/campaign_builder/src/obj_importer_ui.rs`
- `sdk/campaign_builder/src/color_palette.rs`
- `sdk/campaign_builder/src/lib.rs`
- `src/domain/visual/mod.rs`
- `notes/meshes/obj_to_ron_universal.py`
- `notes/meshes/another_woman/another_woman.obj`
- `notes/meshes/another_woman/another_woman.mtl`
- `docs/explanation/implementations.md`

## Notes On The Python Reference

The Python reference still provides useful direction, but the Rust importer on
this branch should not repeat these limitations:

- only loading the first `mtllib`
- overwriting object identity when `usemtl` changes
- dropping normals, UVs, or material context in fallback paths
- silently flattening mixed-material geometry into one mesh without clear rules

## Recommended First Implementation Slice

The lowest-risk coding sequence on this branch is:

1. Refactor `mesh_obj_io.rs` to preserve material-aware parse segments.
2. Add `.mtl` parsing and sidecar resolution.
3. Map `Kd`, `d`, and `Ke` into mesh and material output.
4. Thread imported material data through `ObjImporterState`.
5. Add minimal importer-tab UI for MTL status and manual override.
6. Add parser and importer-state tests before expanding imported palette UI.

This sequence keeps the current importer tab usable while the MTL pipeline is
built underneath it.
