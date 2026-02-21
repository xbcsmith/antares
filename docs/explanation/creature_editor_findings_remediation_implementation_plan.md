# Creature Editor Findings Remediation Implementation Plan

## Overview

This plan addresses the reviewed creature-editor findings with a phased, risk-first approach. The sequence starts with correctness and data-integrity fixes, then restores missing core editor behavior, then resolves architecture mismatches and documentation drift. The goal is to make creature creation and editing reliable before expanding UX depth.

## Current State Analysis

### Existing Infrastructure

- Creature editor entry points and mode switching exist in `sdk/campaign_builder/src/creatures_editor.rs`.
- Template browser integration exists in `sdk/campaign_builder/src/lib.rs` and `sdk/campaign_builder/src/template_browser.rs`.
- Registry/file loading uses `CreatureReference` plus per-file assets in `sdk/campaign_builder/src/lib.rs`.
- Documentation and implementation-status tracking exist in `docs/how-to/create_creatures.md` and `docs/explanation/implementations.md`.

### Identified Issues

- Critical ID allocation risk in template-created creatures due to potentially stale `CreatureIdManager` state.
- Asset editor contains multiple user-visible no-op actions.
- 3D preview integration in creature editor remains placeholder-only.
- `creature_assets.rs` currently assumes an incompatible persistence model.
- User docs and implementation status claims diverge from current feature reality.
- Legacy dead-path UI function remains and increases regression risk.

## Implementation Phases

### Phase 1: Core Implementation

#### 1.1 Foundation Work

- Normalize ID allocation for all template entry paths in `sdk/campaign_builder/src/lib.rs`.
- Refresh `creatures_editor_state.id_manager` from authoritative `self.creatures` immediately before `suggest_next_id(...)` in `show_creature_template_browser_dialog`.
- Reuse existing `CreatureReference` derivation pattern from `CreaturesEditorState::show_registry_mode` to avoid divergent logic.

#### 1.2 Add Foundation Functionality

- Add explicit duplicate-ID guard before appending generated template creature in `show_creature_template_browser_dialog`.
- Return user-facing status message on conflict with current owner creature name and ID.
- Ensure fallback ID retry path is deterministic and bounded.

#### 1.3 Integrate Foundation Work

- Consolidate ID refresh and validation in a shared helper used by both:
- Tools menu flow (`show_creature_template_browser_dialog`).
- Sentinel-triggered flow from `creatures_editor::OPEN_CREATURE_TEMPLATES_SENTINEL`.
- Ensure no behavior depends on prior visit to Creatures tab.

#### 1.4 Testing Requirements

- Add unit tests in `sdk/campaign_builder/src/lib.rs` test module:
- Template creation from Tools path with pre-populated IDs.
- Duplicate-ID detection path with actionable status message.
- Sentinel-driven open and create path uses refreshed manager.
- Add integration test under `sdk/campaign_builder/tests/` for end-to-end template creation without opening Creatures tab first.

#### 1.5 Deliverables

- [] ID manager refresh before template ID suggestion in `sdk/campaign_builder/src/lib.rs`.
- [] Duplicate-ID conflict guard and status messaging.
- [] Shared helper for ID synchronization used across template entry paths.
- [] Regression tests for stale-state and duplicate-ID scenarios.

#### 1.6 Success Criteria

- Creating creature from templates never produces duplicate IDs from stale state.
- Both template entry paths behave identically and deterministically.
- All new ID-path tests pass under `cargo nextest run --all-features`.

### Phase 2: Feature Implementation

#### 2.1 Feature Work

- Implement currently stubbed actions in `sdk/campaign_builder/src/creatures_editor.rs`:
- `Validate Mesh`.
- `Show Issues`.
- `Save As...`.
- `Export RON`.
- `Revert Changes`.
- Define expected behavior and state transitions for each action in edit mode.

#### 2.2 Integrate Feature

- Wire mesh validation to existing domain validation utilities where available.
- Add in-editor issue collection and display state for errors and warnings.
- Implement `Save As...` path generation and write flow aligned with registry plus asset-file model.
- Implement `Export RON` as clipboard and or file export action with deterministic formatting.
- Implement `Revert Changes` reload from source asset path while preserving editor mode consistency.

#### 2.3 Configuration Updates

- Ensure Save and Revert actions keep `unsaved_changes`, `preview_dirty`, `selected_creature`, and selection buffers coherent.
- Ensure save-as updates references consistently and does not orphan files.
- Route action outcomes through unified status-message handling.

#### 2.4 Testing requirements

- Add tests in `sdk/campaign_builder/src/creatures_editor.rs` test module for each action path.
- Add integration tests for Save-As plus registry consistency and Revert behavior.
- Add negative tests for validation failure and write and read I/O errors.

#### 2.5 Deliverables

- [] All stubbed action handlers in `creatures_editor.rs` implemented.
- [] Validation issue panel wired and user-visible.
- [] Save-As, Revert, and Export workflows functional.
- [] Regression coverage for edit action state transitions.

#### 2.6 Success Criteria

- Every visible action button in edit mode performs its documented function.
- Invalid meshes produce actionable issues before save.
- Revert restores file-backed state without index and selection corruption.

### Phase 3: Core Implementation

#### 3.1 Foundation Work

- Reconcile or remove incompatible assumptions in `sdk/campaign_builder/src/creature_assets.rs`.
- Align all creature persistence helpers to active `CreatureReference` plus `assets/creatures/*.ron` architecture.

#### 3.2 Add Foundation Functionality

- Update asset manager APIs to operate on reference-backed asset files.
- Add migration and compatibility guard behavior if legacy `Vec<CreatureDefinition>` files are encountered.

#### 3.3 Integrate Foundation Work

- Remove or deprecate dead and legacy editor path `show_list_mode` in `sdk/campaign_builder/src/creatures_editor.rs`.
- Ensure active dispatch only uses `show_registry_mode` for list behavior.

#### 3.4 Testing Requirements

- Add coverage ensuring persistence helpers round-trip with current registry format.
- Add test proving no runtime call path reaches deprecated list-mode function.

#### 3.5 Deliverables

- [] `creature_assets.rs` aligned to active persistence architecture.
- [] Dead-path list mode removed or deprecated with tests.
- [] Compatibility behavior documented for legacy data shape.

#### 3.6 Success Criteria

- No module assumes `data/creatures.ron` is `Vec<CreatureDefinition>` in active flow.
- Editor list mode has a single authoritative implementation path.

### Phase 4: Feature Implementation

#### 4.1 Feature Work

- Replace preview placeholder in `show_preview_panel` with integrated renderer path.
- Wire preview updates to mesh, transform, and color edits and selection changes.
- Ensure selected-mesh highlight and refresh semantics are deterministic.

#### 4.2 Integrate Feature

- Connect preview controls and state synchronization between `creatures_editor.rs` and preview subsystem.
- Add lightweight fallback UI for renderer unavailable and error conditions.

#### 4.3 Configuration Updates

- Align preview configuration defaults with current editor state initialization.
- Ensure preview toggles are persisted in editor state and reset appropriately in mode transitions.

#### 4.4 Testing requirements

- Add tests for preview-dirty lifecycle and state synchronization triggers.
- Add UI integration tests verifying preview updates after transform and color changes.

#### 4.5 Deliverables

- [] Placeholder preview replaced with functional renderer integration.
- [] Selected mesh and edit changes reflected in preview path.
- [] Fallback error state for preview subsystem.

#### 4.6 Success Criteria

- Preview reflects edits without requiring mode switch or reopen.
- Preview subsystem failure does not break edit workflow.

### Phase 5: Feature Implementation

#### 5.1 Feature Work

- Update `docs/how-to/create_creatures.md` to match currently shipped UI only.
- Remove or clearly mark non-implemented workflows (Variations, LOD, Animation, Materials) as future and planned.

#### 5.2 Integrate Feature

- Reconcile completion claims in `docs/explanation/implementations.md` with actual delivered functionality.
- Add explicit note for partially implemented phases and open TODO items.

#### 5.3 Configuration Updates

- Standardize creature-editor terminology for modes and entry points across docs.
- Ensure all doc navigation paths correspond to actual menus and tabs.

#### 5.4 Testing requirements

- Add doc parity checklist test script or lint-style assertions for critical navigation strings.
- Verify all referenced menu actions exist in `sdk/campaign_builder/src/lib.rs` and `sdk/campaign_builder/src/creatures_editor.rs`.

#### 5.5 Deliverables

- [] `docs/how-to/create_creatures.md` corrected for current behavior.
- [] `docs/explanation/implementations.md` phase status corrected to reality.
- [] Doc parity checklist for creature-editor entry points and actions.

#### 5.6 Success Criteria

- User-facing instructions no longer reference missing UI paths.
- Implementation-status documentation no longer overstates completion.

## Recommended Implementation Order

1. Phase 1: Correctness and data integrity.
2. Phase 2: Core editor behavior.
3. Phase 3: Architecture consistency and cleanup.
4. Phase 4: Preview integration.
5. Phase 5: Documentation and status reconciliation.

This order prevents data corruption first, restores expected edit operations second, removes conflicting data assumptions third, then completes preview depth and documentation accuracy.
