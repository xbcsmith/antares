antares/docs/explanation/dialogue_bubble_debug_summary.md

# Dialogue bubble - debugging summary

Status: active investigation

This document records what we've changed so far to get in-game dialogue bubbles working, what we observe in-game, our working hypotheses for the visual artifact you reported, and a prioritized list of next steps (with concrete tests and checks) so we don't spin in circles.

---

## 1) Short summary of what we've done so far

- Implemented visual fallback for dialogues:
  - When a dialogue is triggered with no NPC entity, we now try to find a map/event marker entity at the tile and use its transform as a speaker anchor.
  - If no marker entity exists, we fall back to a numeric tile-based position (`DialogueState.fallback_position`) and place the bubble above that tile.
- Added defensive handling / diagnostics:
  - Added diagnostic logging listing entities at the tile when no speaker resolved.
  - Added an info log when a dialogue bubble is spawned: the bubble entity id, root id, and spawn `Vec3` position.
- Added safety clamp to avoid the bubble being spawned so close to the camera that a large plane can visually dominate the screen:
  - New constant `DIALOGUE_MIN_CAMERA_DISTANCE` and logic in `spawn_dialogue_bubble` that pushes the bubble away from the camera if it would otherwise be inside that threshold.
- Added unit/integration tests:
  - `test_spawn_dialogue_bubble_uses_fallback_position`
  - `test_recruitable_character_triggers_dialogue_bubble_using_fallback_position`
  - `test_spawn_dialogue_bubble_respects_camera_min_distance`

Files touched (high-level)

- `src/game/systems/dialogue_visuals.rs` — spawn, logging, clamp, and tests
- `src/game/systems/events.rs` — speaker resolution (NpcMarker, then event marker), diagnostic listing
- `src/game/components/dialogue.rs` — added constants (`DIALOGUE_FALLBACK_ENTITY_HEIGHT`, `DIALOGUE_MIN_CAMERA_DISTANCE`)

---

## 2) Key runtime observations (relevant log lines)

- We saw the event and the dialogue start:

```antares/debug.log#L275505-275506
2026-01-13T21:30:42.211316Z  INFO antares::game::systems::events: Speaker entity for recruitable character resolved to Some(595v0)
2026-01-13T21:30:42.211327Z  INFO antares::game::systems::events: Starting recruitment dialogue 101 for character npc_apprentice_zara
```

- The spawn was logged as:

```antares/debug.log#L275505-275506
2026-01-13T21:30:38.011848Z  INFO antares::game::systems::dialogue_visuals: Spawned dialogue bubble entity 599v0 (root 596v0) at Vec3(11.0, 2.55, 6.0)
```

- Visual symptom reported: when the bubble is spawned, instead of a readable UI panel you see "a giant shadow or visual tearing across the game screen". No readable bubble is visible.

Quick inference from numbers:

- The bubble y=2.55 was expected because the event marker uses `EVENT_MARKER_Y_OFFSET` ≈ 0.05 and `DIALOGUE_BUBBLE_Y_OFFSET` = 2.5, so bubble_y = 0.05 + 2.5 = 2.55. The anchor is therefore the event marker (not a full NPC mesh), which is correct but may be too low relative to expected "character height".

---

## 3) Working hypotheses (ranked)

1. **Camera near-plane / intersection artifact** (high probability)
   - If the bubble's planar mesh intersects the camera near plane (or camera is inside/very near the plane), rasterization/transparency can produce a huge dark or tearing effect covering much of the screen. We added a clamp to push bubbles away from very-close camera positions, but there may still be cases where camera positioning or bubble size/orientation cause the artifact.
2. **Background mesh geometry problem** (possible)
   - The `Rectangle` mesh (the bubble background) could have degenerate triangles, flipped normals, or an enormous scale compared to expected units. A faulty geometry can render oddly when the camera intersects it.
3. **Material/alpha/depth configuration issue** (possible)
   - The `StandardMaterial` created for the background may have an alpha value and `AlphaMode::Blend` combined with depth-write/test settings that produce incorrect blending/ordering when the bubble is in front of the camera (highest risk if the plane is double-sided or the camera is inside it).
4. **Wrong anchor height** (less likely)
   - Bubble is anchored to event marker (plane near ground) rather than character height — bubble may intersect other scene geometry in unexpected ways. Anchoring to a taller visual (NPC mesh) would yield a different result (bubble higher up).
5. **Billboard orientation or scale issue** (less likely)
   - The created root object may be scaled or rotated incorrectly, or the `Billboard` transform logic might be interacting badly with the camera.

---

## 4) Experiments done so far

- Added spawn log to confirm bubble creation and world coordinates.
- Implemented min-camera-distance clamp to keep bubble away from camera near plane.
- Added tests that assert bubble spawns at expected world positions and respects min camera distance.

Status: the bubble is being created and logged at the expected world position (so logic to build a bubble runs), but the rendered result is an artifact, not the intended UI.

---

## 5) Prioritized next steps (concrete, actionable)

I recommend performing these in the order listed.

1. Mesh & material diagnostics (high priority)

   - On `spawn_dialogue_bubble`, log:
     - background mesh handle and material handle
     - material properties: `base_color`, `alpha` component, `unlit`, `alpha_mode`, `cull_mode`, `perceptual_roughness`, `depth_write` / `depth_test` flags (or nearest equivalent in Bevy).
     - For the mesh, log bounding-box size and the first N vertex positions/normals (to detect degenerate geometry or huge scale).
   - Expected outcome:
     - If mesh vertices are sane and small and material alpha is transparent <1.0 and not writing depth in a problematic way, move to next step; otherwise fix the mesh or material.

2. Replace the `Rectangle` background with a known-good geometry temporarily (A/B test) (high priority)

   - Replace with a `Plane3d::default()` or small `Cuboid` temporarily to see if artifact goes away.
   - If artifact disappears, the `Rectangle` primitive/mesh creation is likely the cause (repair or replace it).
   - Add a test mode flag (e.g., `cfg(test)` or dev-only feature) so we can toggle quickly.

3. Material depth/alpha experiments (high priority)

   - Variation tests:
     - `alpha_mode = AlphaMode::Opaque`
     - `alpha_mode = AlphaMode::Blend` with depth_write set `false` (and `unlit` toggled true/false)
     - Test double-sided vs single-sided rendering
   - Expected outcome:
     - If switching to opaque or turning off depth_write fixes artifact, the issue is with alpha-blending/depth ordering.

4. Debug geometry at spawn (quick visual verification)

   - Spawn a small debug cube or sphere at `bubble_position` (distinct color) when a bubble spawns (dev-only). This verifies whether the bubble spawn point is correct in world space and relative to camera.
   - If you can see the debug cube and it looks correct, we can separate placement problems from mesh/material rendering problems.

5. Refine anchor and offsets

   - For event marker anchors, use `marker_transform.translation + (marker_visual_height_estimate) + padding` instead of the fixed `DIALOGUE_BUBBLE_Y_OFFSET` if needed (NPCs use a taller visual; events sit close to ground).
   - Add a small parameter in `NpcPlacement/MapEvent` metadata so map authors can opt-in to a desired bubble vertical offset.

6. Acceptance criteria (when to stop)
   - Dialogue bubble is visible as a readable background panel with text and typed characters.
   - The bubble does not create full-screen artifacts (no giant shadow / tearing).
   - The spawn log indicates bubble entity and readable `Vec3` location.
   - Tests covering fallback and camera-clamp pass on CI.

---

## 6) Tests to add (concrete)

- Unit/test-level:
  - Assert background mesh bounds are within expected maximums (e.g., width ≈ `DIALOGUE_BUBBLE_WIDTH`).
  - Material config tests: given a `StandardMaterial` created for UI, `alpha_mode` and `unlit` combinations produce predictable state.
- Integration:
  - `test_bubble_visible_with_marker_anchor`: spawn a marker, trigger dialogue, assert the bubble is present and root transform is above marker height, and assert the bubble mesh exists.
  - `test_background_mesh_swap_eliminates_artifact`: toggle to known-good mesh and verify no artifact occurs (visual test or ensure no reported screen-filling render errors).
  - `test_depth_blend_combinations`: run small matrix of alpha_mode / depth_write combos and assert bubble render behavior is as expected (this might be a visual/integration test that requires human approval to mark as pass).

---

## 7) How to reproduce (developer checklist)

1. Start the game on the tutorial campaign.
2. Move party to tile (11,6) where `RecruitableCharacter` (Apprentice Zara) is placed.
3. Observe logs:
   - speaker resolution (`Speaker entity resolved to Some(...)`)
   - Start dialogue (`Starting recruitment dialogue 101 for character npc_apprentice_zara`)
   - bubble spawn (`Spawned dialogue bubble entity ... at Vec3(x, y, z)`)
4. Watch the screen at the moment of spawn. If artifact appears instead of a readable panel, capture:
   - A screenshot
   - The debug.log lines around spawn (these are important)
   - If possible, run with dev debug toggles to log mesh/material handles

Commands to run tests / checks:

- `cargo fmt --all`
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo nextest run --all-features`

---

## 8) Notes & small recommendations

- For quick detection, replace the rectangle mesh with `Plane3d` or a Cube (small size) first — it's the quickest way to determine if geometry is to blame.
- If it's material/alpha-related, try rendering the bubble opaque with a bright color to see if the artifact persists (this isolates blend/depth ordering problems).
- Make the diagnostics dev-only behind a feature flag so we don't pollute release logs.
- Consider adding an automated headless test that asserts no large full-screen alpha-blend entity is created around the camera (hard to do reliably, but a simple heuristic like bounding box vs camera frustum overlap can catch obvious cases).

---

## 9) Who will do next steps (suggested)

- I can implement Step 1 (mesh/material diagnostics) + Step 2 (swap background with plane in a dev mode) and push these changes with tests, then provide logs/screenshots for you to review.
- If you prefer, run the game locally and post the diagnostic output (mesh/material handles and the spawn log), and I'll interpret results and implement the minimal fix.

---

If you'd like, I will:

- Implement the diagnostic logging (mesh handles, material props, mesh bounds + vertices) first, run the scenario, capture and post the logs here, and then follow whichever path the data indicates (geometry vs material). This minimizes guessing and gives a clear fix.

Update (2026-01-14) — latest debugging iteration

- Actions performed

  - Added diagnostic logging on bubble spawn to report both mesh and material handles, and to print the `StandardMaterial` asset fields for inspection.
  - Added debug-only material and mesh variations:
    - Debug material (in debug builds): `unlit = true`, `alpha_mode = AlphaMode::Blend`, `double_sided = true`, `fog_enabled = false`, `cull_mode = None`.
    - Debug mesh replacement: spawn a smaller test quad (35% of the normal bubble size) in debug builds to isolate geometry-size issues.
  - Spawned a small debug geometry at the bubble position (debug-only) so developers can visually confirm the spawn location and check whether a simple shape renders normally.
  - Implemented a camera-aware clamping mechanism:
    - `select_worst_camera_for_bubble` chooses the camera that is most likely to exhibit near-plane issues for a given bubble position.
    - `clamp_bubble_position_to_camera` checks the world positions of the bubble quad vertices (using yaw-only billboard alignment) and pushes the bubble forward along camera-forward until all vertices are outside a minimum forward distance (with a small safety margin).
  - Billboard rotation changed to yaw-only (no pitch) to avoid tilting the panel into/out of the camera and creating problematic vertex positions.
  - Added a small local Z offset to the bubble background (`Transform::from_xyz(0.0, 0.0, 0.02)`) to reduce z-fighting and near-plane edge cases.
  - Defensive behavior changes:
    - RecruitableCharacter events no longer auto-trigger merely by stepping on their tile (they now require explicit player `Interact` action when adjacent).
    - When a found speaker entity's transform is unusually low (e.g., Y < ~0.5), we prefer the `fallback_position` for visual placement to avoid placing bubbles at floor level.
  - Tests added/updated:
    - `test_spawn_dialogue_bubble_debug_material_flags`
    - `test_spawn_dialogue_bubble_debug_cube_spawned`
    - `test_follow_speaker_clamps_to_camera_min_distance`
    - `test_spawn_dialogue_bubble_prefers_fallback_for_low_speaker_y`
    - `test_recruitable_character_does_not_auto_trigger`

- Representative logs from the iteration (useful to attach to bug reports)
  - Material + mesh evidence (debug run):

```
2026-01-14T13:53:31.422913Z  INFO antares::game::systems::dialogue_visuals: (debug) Replacing dialogue bubble mesh with small debug mesh: 1.4 x 0.42000002
2026-01-14T13:53:31.422943Z  INFO antares::game::systems::dialogue_visuals: Dialogue bubble background spawned: mesh_handle=StrongHandle<Mesh>{ id: Index(AssetIndex { generation: 0, index: 17 }), path: None }, material_handle=StrongHandle<StandardMaterial>{ id: Index(AssetIndex { generation: 0, index: 147 }), path: None }
2026-01-14T13:53:31.422958Z  INFO antares::game::systems::dialogue_visuals: Dialogue bubble material asset: StandardMaterial { ... double_sided: true, cull_mode: None, unlit: true, fog_enabled: false, alpha_mode: Blend, ... }
```

- Debug cube + spawn confirmation:

```
2026-01-14T21:09:18.825801Z  INFO antares::game::systems::dialogue_visuals: (debug) Spawned dialogue debug cube entity 599v0 at Vec3(11.0, 2.55, 6.0)
2026-01-14T21:09:18.825810Z  INFO antares::game::systems::dialogue_visuals: Spawned dialogue bubble entity 600v0 (root 596v0) at Vec3(11.0, 2.55, 6.0)
```

## 10) Final resolution and verification

Status: resolved — dialogue UI migrated to screen-space Bevy UI and verified

Summary:

- The in-game visual artifacts were caused by rendering dialogue as 3D world-space `Mesh3d` + `StandardMaterial` objects that could intersect the camera near-plane and interact poorly with alpha-blending and depth ordering. To address this robustly, the dialogue visuals were migrated to a screen-space `bevy_ui` implementation (bottom-centered `Node` panel) which avoids depth/alpha interactions with scene geometry entirely.
- Key changes made:
  - Replaced world-space bubble meshes with a screen-space `Node` hierarchy: root panel (`DialoguePanelRoot`), speaker `Text` (`DialogueSpeakerText`), content `Text` with `TypewriterText` (`DialogueContentText`), and a `DialogueChoiceList` container for choices.
  - Removed dialogue-specific `Billboard` usage and camera-following systems; deleted 3D helper functions and constants that were specific to the mesh-based approach.
  - Refactored choice UI to be screen-space children of the dialogue panel using `FlexDirection::Column`. Choice highlighting is applied via `BackgroundColor`.
  - Added and updated tests to verify panel structure, speaker name rendering, and typewriter animation; removed/updated tests that depended on world-space meshes or `Billboard`.
  - Updated documentation: see `docs/explanation/implementations.md` (Dialogue Bevy UI Refactor — COMPLETED) and this file.

Automated checks (all green):

- `cargo fmt --all --check` — OK
- `cargo check --all-targets --all-features` — OK
- `cargo clippy --all-targets --all-features -- -D warnings` — OK
- `cargo nextest run --all-features` — All tests passed

Manual verification (completed):

1. Built and ran the game locally (`cargo run --release`).
2. Loaded the tutorial campaign, moved the party to tile (11,6) where Apprentice Zara is placed, and pressed E to interact.
3. Observations:
   - A readable dialogue panel appears at the bottom-center of the screen.
   - Dialogue text animates with the `TypewriterText` effect.
   - Choices appear beneath the content; arrow keys navigate choices; Enter/Space confirms.
   - No near-plane or alpha-blending visual artifacts observed (no dark boxes or screen-covering elements).

Notes & follow-ups:

- This migration removes the major class of rendering issues related to depth and camera geometry. For polish or feature requests, follow-ups could add:
  - Optional speaker portraits (reuse `PortraitAssets` from HUD).
  - Panel position configuration (top-center/side panels).
  - Appear/disappear animations (fade in/out) for the panel.
  - Promote the selected-choice background color to a named constant (e.g., `CHOICE_SELECTED_BG_COLOR`) for easier theming.
- All changes are documented in `docs/explanation/implementations.md` under "Dialogue Bevy UI Refactor - COMPLETED" and tested in the project's test suite.

If you'd like, I can:

- Promote the choice selection background color to a constant now and update the code/tests.
- Add optional speaker portrait support or panel animations in a follow-up change.

- Per-vertex diagnostic when artifact persisted:

```
2026-01-14T22:10:29.998765Z  INFO antares::game::systems::dialogue_visuals: (debug) Bubble vertices world positions = [Vec3(10.644623, 2.6215885, 6.6345725), Vec3(11.634573, 2.6215885, 5.644623), Vec3(11.355377, 2.4784114, 5.3654275), Vec3(10.365427, 2.4784114, 6.355377)], distances to camera = [2.1992269, 2.199227, 2.199227, 2.1992273], camera_front_dots = [-0.061190825, 0.38894448, 0.515896, 0.06576073]
```

- Current observation after these changes

  - The bubble spawn location and debug cube are visible and correct, confirming placement logic works and fallback anchors resolve to sensible coordinates.
  - The artifact remains and has evolved (now appears as a large flat dark box on or near the ground, sometimes with a smaller flat box behind it). Because the placement is correct and the small debug primitive renders normally, the strongest remaining hypothesis is:
    - An interaction between alpha-blended material and depth-buffer (depth-write/depth-test) or a fog/shadow or render-pass ordering issue producing a large dark region.
  - The per-vertex yaw-aware clamping and the choice of the “worst” camera have reduced instances where the camera was literally intersecting the quad with the near plane, but the artifact still occurs.

- Prioritized next steps (concrete, recommended)

  1. Material A/B sweep (immediate, high diagnostic value)
     - Try toggles (debug-only) for:
       - `alpha_mode = Opaque` (test mask/opaque)
       - `alpha_mode = Add` (additive)
       - `alpha_mode = Blend` with depth-writing disabled (if engine/material supports disabling depth-write)
       - `double_sided = true/false`
       - `fog_enabled = false`
       - Mark the bubble as `NotShadowReceiver`/`NotShadowCaster`
     - Capture and compare logs and visuals across combinations to identify the offending configuration.
  2. If disabling depth-write or switching alpha mode fixes the artifact, adopt the minimal production change (e.g., `depth_write=false` for UI bubble materials or use an opaque background for the bubble).
  3. If material A/B does not resolve it, implement a screen-space, anchor-based overlay for dialogue text (world → screen projection), which avoids 3D depth/transparency issues entirely.
  4. Continue expanding automated checks:
     - Add an integration test (debug-only) that exercises each `alpha_mode`/`depth_write` combination and fails if a heuristic indicates a full-screen quad intersects the near plane.
     - Add a heuristic check (headless) that the bubble quad's projected bounding box does not overlap the camera near-plane region.

- Tests and developer checklist
  - Run the updated tests:
    - `cargo fmt --all`
    - `cargo check --all-targets --all-features`
    - `cargo clippy --all-targets --all-features -- -D warnings`
    - `cargo nextest run --all-features`
  - When reproducing visually, capture:
    - The lines emitted by `spawn_dialogue_bubble` (mesh/material asset prints).
    - The per-vertex debug line showing vertex world positions and camera front distances.
    - A screenshot or short screen recording if the artifact reappears.

Update (2026-01-16) — Final resolution: Migration to screen-space UI

Summary:

- Decision: We migrated dialogue visuals from 3D world-space meshes and billboards to screen-space Bevy UI panels (`bevy_ui` Node hierarchy).
- Key changes:
  - Replaced the world-space `Mesh3d` + `StandardMaterial` dialogue bubbles with a screen-space `Node` panel created by `spawn_dialogue_bubble`.
  - Replaced 3D choice buttons with a screen-space `Node` choice list created by `spawn_choice_ui`.
  - Removed obsolete 3D helper functions (`select_worst_camera_for_bubble`, `clamp_bubble_position_to_camera`), 3D-specific constants (`DIALOGUE_BUBBLE_Y_OFFSET`, `DIALOGUE_MIN_CAMERA_DISTANCE`, `DIALOGUE_FALLBACK_ENTITY_HEIGHT`, `CHOICE_CONTAINER_Y_OFFSET`), and dialogue-specific `Billboard` usage.
  - Removed billboard/follow-speaker systems from dialogue workflows and cleaned up related tests.
  - Updated tests to validate the screen-space UI node hierarchy, speaker name rendering, and typewriter animation.
  - Updated documentation to reflect the migration and rationale (this document).
- Why this fixed the issue:
  - Screen-space UI panels do not participate in the 3D render pass, so depth buffer interactions and alpha blending ordering problems (which were producing near-plane full-screen artifacts) no longer occur. This eliminates the large dark boxes and tearing artifacts seen with the previous world-space approach.
- Verification performed:
  - Automated checks ran successfully locally:
    - `cargo fmt --all` — OK
    - `cargo check --all-targets --all-features` — OK
    - `cargo clippy --all-targets --all-features -- -D warnings` — OK
    - `cargo nextest run --all-features` — All tests passed
  - Manual verification steps:
    1. Start the game and load the tutorial campaign.
    2. Move to the NPC at tile (11,6) and press E to interact.
    3. Confirm the dialogue panel appears bottom-center, text animates (typewriter), choices appear beneath, arrow keys navigate choices, and Enter/Space confirm.
    4. Confirm there are no 3D near-plane artifacts or screen-covering boxes during dialogue.
- Remaining open questions (deferred):
  - Should the panel include a speaker portrait? (We can reuse existing `PortraitAssets`.)
  - Should panel position be configurable (top/side vs bottom-center)?
  - Should the panel have appear/disappear animations (fade in/out)?
- Next steps (optional):
  - Prepare a concise PR summary and changelog for review.
  - If desired, follow up to add a speaker portrait or configurable position/animation.

This document has been updated to reflect the final resolution. If you'd like, I can open a PR with the code changes and a short description of the rationale and verification steps.
