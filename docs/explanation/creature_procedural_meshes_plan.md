# Creature Procedural Mesh Improvement Plan

## Overview
The current iteration of procedural creature meshes (e.g. `wolf.ron`, `skeleton.ron`) relies heavily on primitive boxes (`box_mesh`), resulting in a "blocky" lower-fidelity appearance. We discovered that the and the game engine currently ignores `mesh_transforms`, causing multi-part high-fidelity models like `ancientwolf.ron` to render incorrectly. 

This plan addresses both the engine bugs and the transition of every creature asset to a high-fidelity procedural standard using ellipsoids and tapered cylinders.

## User Review Required
> [!IMPORTANT]
> - **No Retirement:** Every creature currently in `campaigns/tutorial/assets/creatures` (38 total) will be upgraded, not removed.
> - **Consolidation:** Four fragmented Python scripts in `examples/` will be merged into a single unified generator.

## Implementation Phases

### Phase 1: Fix Core Rendering Bug
**Goal:** Ensure multi-part procedural meshes like `AncientWolf` and `AncientSkeleton` render correctly by applying their defined part offsets.

1. **Fix `spawn_creature` in `src/game/systems/creature_spawning.rs`:** 
   - Currently, child entities are spawned with `Transform::default()`.
   - Update to read `creature_def.mesh_transforms[mesh_index]` and apply the translation, rotation, and scale to each part.

### Phase 2: Consolidate Python Tooling
**Goal:** Create a single, maintainable generation script.

1. **Merge Scripts:** Combine `gen_monsters2.py`, `gen_dire_wolves.py`, `gen_detailed_meshes.py`, and `generate_characters.py` into a single `examples/generate_all_meshes.py`.
2. **Standardize Primitives:** Centralize advanced math for `ellipsoid`, `tapered_cylinder`, and `sphere_section`.
3. **Refactor Output:** Ensure the script can target `campaigns/tutorial/assets/creatures` directly for easy asset updates.

### Phase 3: Mass Asset Upgrade
**Goal:** Roll out high-fidelity models for all creature types.

1. **Upgrade Baseline Creatures:** Apply high-detail logic (similar to `AncientSkeleton`) to the standard `skeleton.ron`, `wolf.ron`, `direwolf.ron`, `orc.ron`, etc.
2. **Process Character Assets:** Apply anatomical upgrades to the 54 player character files (e.g., `whisper.ron`, `mira.ron`, `sirius.ron`).
3. **Template Sync:** Ensure `ancientwolf.ron` and `ancientskeleton.ron` remain the benchmark for "Ancient" variants, while bringing standard versions closer in fidelity.

### Phase 4: Verification & Polish
**Goal:** Validate rendering and performance.

1. **In-game Inspection:** Walk through the tutorial campaign to verify all 38 creatures appear anatomically correct and positioned properly.
2. **Performance Check:** Ensure triangle counts for these higher-fidelity versions remain within budget (using LODs if necessary).

## Verification Plan

### Automated Tests
- Run `cargo test` to ensure no regressions in mesh processing logic.
- Verify `MeshDefinition` validation passes for the new larger files.

### Manual Verification
- Use the map viewer to inspect creatures.
- Verify that "Ancient Wolf" parts are properly assembled, not clumped at the origin.
