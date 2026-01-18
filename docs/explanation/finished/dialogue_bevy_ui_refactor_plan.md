# Dialogue Bevy UI Refactor Implementation Plan

## Overview

This plan refactors the dialogue bubble and choice UI from the current 3D world-space mesh approach to native Bevy UI (`bevy_ui`). The current implementation uses `Mesh3d`, `StandardMaterial`, and a `Billboard` component to render dialogue in 3D space, which causes:

- Near-plane clipping artifacts (large dark boxes covering the screen)
- Alpha-blend/depth-buffer conflicts with scene geometry
- Inconsistent rendering across camera positions

The target implementation uses screen-space `Node` components (matching the existing HUD implementation in `hud.rs`), eliminating depth/transparency issues entirely.

---

## Current State Analysis

### Existing Infrastructure

| Module | Pattern | Notes |
|--------|---------|-------|
| [hud.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/hud.rs) | Native bevy_ui `Node` | Screen-space UI with flexbox layout |
| [dialogue_visuals.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_visuals.rs) | 3D world-space `Mesh3d` | Billboard rotation, StandardMaterial |
| [dialogue_choices.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_choices.rs) | 3D world-space `Transform` | Same Billboard pattern |
| [dialogue.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/components/dialogue.rs) | Components | `DialogueBubble`, `TypewriterText`, `Billboard`, constants |

**Key Pattern from HUD:**
```rust
// hud.rs uses this pattern for screen-space UI
commands.spawn((
    Node {
        position_type: PositionType::Absolute,
        bottom: Val::Px(24.0),
        left: Val::Px(0.0),
        // ... flexbox properties
    },
    BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.9)),
    MarkerComponent,
));
```

### Identified Issues

1. **3D Mesh Rendering**: `spawn_dialogue_bubble` creates `Mesh3d` + `StandardMaterial` in world space
2. **Billboard Component**: Requires per-frame rotation updates and still causes near-plane issues
3. **Alpha Blending**: `AlphaMode::Blend` material causes depth/ordering problems
4. **Stray Brace Bug**: Line 343 has a misplaced `}` causing formatting issues (minor)
5. **Parallel Implementation**: `dialogue_choices.rs` also uses 3D world-space approach

---

## Implementation Phases

### Phase 1: Core Dialogue Panel Refactor

> Replace 3D world-space dialogue bubble with screen-space bevy_ui panel.

#### 1.1 Add New UI Constants

**Location**: [dialogue.rs components](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/components/dialogue.rs)

Add new constants for screen-space layout:

- `DIALOGUE_PANEL_WIDTH: Val` — Panel width (e.g., `Val::Percent(50.0)`)
- `DIALOGUE_PANEL_BOTTOM: Val` — Distance from screen bottom (e.g., `Val::Px(120.0)`)
- `DIALOGUE_PANEL_PADDING: Val` — Internal padding
- `DIALOGUE_SPEAKER_FONT_SIZE: f32` — Speaker name text size
- `DIALOGUE_CONTENT_FONT_SIZE: f32` — Dialogue content text size

#### 1.2 Create New UI Components

**Location**: [dialogue.rs components](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/components/dialogue.rs)

Add marker components for the screen-space UI hierarchy:

- `DialoguePanelRoot` — Marks the root UI container
- `DialogueSpeakerText` — Marks the speaker name text element
- `DialogueContentText` — Marks the dialogue content text (with `TypewriterText`)
- `DialogueChoiceList` — Marks the choice button container

#### 1.3 Rewrite `spawn_dialogue_bubble`

**Location**: [dialogue_visuals.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_visuals.rs#L261)

Replace the function signature and body:

1. Remove `meshes`, `materials`, `query_camera`, `query_speaker` parameters
2. Spawn a `Node` hierarchy using the HUD pattern:
   - Root container (absolute positioned at bottom-center)
   - Speaker name text row
   - Content text with `TypewriterText` component
3. Update `ActiveDialogueUI.bubble_entity` to track the root `Node` entity
4. Remove all 3D positioning, billboard, and mesh-related code

#### 1.4 Remove Billboard System Calls

**Location**: [dialogue_visuals.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_visuals.rs)

- Remove or no-op `billboard_system` (no longer needed for dialogue)
- Remove `follow_speaker_system` (screen-space UI doesn't follow entities)
- Keep `update_typewriter_text` (works with any `Text` + `TypewriterText`)
- Keep `update_dialogue_text` (resets typewriter on node change)
- Simplify `cleanup_dialogue_bubble` to despawn `Node` entities

#### 1.5 Testing Requirements

**Existing tests** in [dialogue_visuals_test.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/tests/dialogue_visuals_test.rs) test component logic (typewriter, constants) and remain valid.

**New tests to add**:
- `test_dialogue_panel_spawns_with_correct_structure` — Verify `DialoguePanelRoot` + children exist
- `test_dialogue_panel_displays_speaker_name` — Verify speaker name text is populated
- `test_dialogue_panel_typewriter_works` — Verify typewriter animation on screen-space text

**Run existing tests**:
```bash
cargo nextest run --all-features dialogue_visuals
```

#### 1.6 Deliverables

- [ ] New UI constants added to `dialogue.rs`
- [ ] New marker components (`DialoguePanelRoot`, etc.) added
- [ ] `spawn_dialogue_bubble` rewritten to use `Node` hierarchy
- [ ] `billboard_system` and `follow_speaker_system` removed/disabled
- [ ] `cleanup_dialogue_bubble` updated for `Node` despawn
- [ ] Existing tests pass
- [ ] New tests added and pass

---

### Phase 2: Dialogue Choices Refactor

> Replace 3D choice buttons with screen-space choice list.

#### 2.1 Rewrite `spawn_choice_ui`

**Location**: [dialogue_choices.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_choices.rs#L38)

Replace 3D approach with bevy_ui:

1. Create choice container as child of `DialoguePanelRoot`
2. Use `Node` with `FlexDirection::Column` for vertical choice list
3. Each choice is a `Node` with `Text` and `DialogueChoiceButton` component
4. Remove `Transform`, `GlobalTransform`, `Billboard` from choice entities

#### 2.2 Update `update_choice_visuals`

**Location**: [dialogue_choices.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_choices.rs#L119)

- Update query to use bevy_ui components (`Node`, `Text`, `BackgroundColor`)
- Highlight selected choice with `BackgroundColor` change in addition to text color

#### 2.3 Update `cleanup_choice_ui`

**Location**: [dialogue_choices.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_choices.rs#L225)

- Despawn `Node` entities instead of 3D entities
- Query for `DialogueChoiceContainer` (which is now a `Node`)

#### 2.4 Testing Requirements

**Existing tests** in [dialogue_choice_test.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/tests/dialogue_choice_test.rs) (if exists) test choice logic.

**Run existing tests**:
```bash
cargo nextest run --all-features dialogue_choice
```

#### 2.5 Deliverables

- [ ] `spawn_choice_ui` uses screen-space `Node` layout
- [ ] `update_choice_visuals` works with bevy_ui components
- [ ] `cleanup_choice_ui` despawns `Node` entities
- [ ] Choice input system unchanged (keyboard handling is logic-only)

---

### Phase 3: Cleanup and Polish

#### 3.1 Remove Obsolete Code

**Location**: [dialogue_visuals.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_visuals.rs)

- Remove `select_worst_camera_for_bubble` function
- Remove `clamp_bubble_position_to_camera` function
- Remove `DebugDialogueDiagnostics` resource (debug 3D primitives no longer needed)
- Remove 3D-specific constants from [dialogue.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/components/dialogue.rs):
  - `DIALOGUE_BUBBLE_Y_OFFSET`
  - `DIALOGUE_MIN_CAMERA_DISTANCE`
  - `DIALOGUE_FALLBACK_ENTITY_HEIGHT`
  - `CHOICE_CONTAINER_Y_OFFSET`

#### 3.2 Remove Billboard Component Usage

**Location**: [dialogue.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/components/dialogue.rs)

- Remove `Billboard` component from dialogue (keep if used elsewhere)
- Update `DialogueBubble` struct to remove 3D entity references (`background_entity`)

#### 3.3 Update Plugin Registration

**Location**: [dialogue.rs systems](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue.rs#L88)

Update `DialoguePlugin::build` to:
- Remove `billboard_system` from `add_systems`
- Remove `follow_speaker_system` from `add_systems`
- Keep `spawn_dialogue_bubble`, `update_dialogue_text`, `update_typewriter_text`, `cleanup_dialogue_bubble`

#### 3.4 Fix Stray Brace

**Location**: [dialogue_visuals.rs line 343](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/dialogue_visuals.rs#L341-L343)

Run `cargo fmt` to fix the indentation issue, or manually remove the stray `}`.

#### 3.5 Deliverables

- [ ] Obsolete 3D helper functions removed
- [ ] 3D-specific constants removed
- [ ] Plugin system registrations updated
- [ ] `cargo fmt --all` produces no changes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` is clean

---

### Phase 4: Verification and Documentation

#### 4.1 Success Criteria

| Check | Command | Expected |
|-------|---------|----------|
| Format | `cargo fmt --all --check` | No output (exit 0) |
| Compile | `cargo check --all-targets --all-features` | No errors |
| Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | No warnings |
| Tests | `cargo nextest run --all-features` | All pass |

#### 4.2 Manual Verification

1. Start the game: `cargo run --release`
2. Load the tutorial campaign
3. Move party to tile (11,6) where Apprentice Zara is placed
4. Press E to interact
5. **Expected**: A readable dialogue panel appears at the **bottom-center of the screen**
6. **Expected**: Text animates with typewriter effect
7. **Expected**: Choices appear below dialogue content
8. **Expected**: Arrow keys navigate choices, Enter/Space confirms
9. **Expected**: NO visual artifacts, dark boxes, or screen-covering elements

#### 4.3 Update Documentation

**Location**: [dialogue_bubble_debug_summary.md](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/dialogue_bubble_debug_summary.md)

Add a final section documenting the resolution: migrated from 3D world-space to screen-space bevy_ui.

#### 4.4 Deliverables

- [ ] All automated checks pass
- [ ] Manual verification confirms readable dialogue panel
- [ ] No visual artifacts during dialogue
- [ ] Documentation updated

---

## Open Questions

1. **Speaker Portrait**: Should the dialogue panel include a speaker portrait image? HUD already has portrait loading infrastructure (`PortraitAssets`).

2. **Panel Position**: Bottom-center is the default. Should there be options for top-center or side positioning?

3. **Animation**: Should the panel have appear/disappear animation (fade in/out) or instant visibility toggle?

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing dialogue logic | Low | High | Only changing visual layer, not `DialogueState` or message handling |
| TypewriterText compatibility | Low | Medium | `TypewriterText` works with any `Text` component |
| Test failures | Medium | Low | Existing tests focus on component logic, not 3D rendering |
| Missing edge cases | Medium | Medium | Manual verification covers NPC, recruitable character, and simple dialogue flows |

---

## Rollback Procedure

If Phase 1-2 causes issues:

```bash
git checkout HEAD -- src/game/systems/dialogue_visuals.rs
git checkout HEAD -- src/game/systems/dialogue_choices.rs
git checkout HEAD -- src/game/components/dialogue.rs
cargo clean
cargo check --all-targets --all-features
```
