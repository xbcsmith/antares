# Dialogue System Implementation Plan

## Overview

This plan implements an enhanced dialogue system for Antares using native Bevy ECS patterns with 2.5D floating text bubbles, typewriter text effects, and event-driven architecture. The implementation builds upon the existing `domain::dialogue` module while adding visual feedback and improved state management without introducing external dialogue framework dependencies.

**Goals**:

- Replace synchronous dialogue execution with event-driven Bevy systems
- Add 2.5D floating text bubbles positioned above NPCs
- Implement typewriter text animation effects
- Maintain backward compatibility with existing `DialogueTree` RON format
- Support complex branching with visual feedback

**Scope**:

- **Game Engine**: Core dialogue visuals, state management, and runtime systems
- **Campaign Builder**: No changes required (continues using existing `domain::dialogue` editor)
- **Data Format**: Continue using existing `.ron` format with `DialogueTree` structure

---

## Current State Analysis

### Existing Infrastructure

#### Domain Layer

**File**: `antares/src/domain/dialogue.rs`

- **DialogueTree struct** (lines 43-64): Core dialogue data structure
  - Fields: `id`, `name`, `root_node`, `nodes`, `speaker_name`, `repeatable`, `associated_quest`
- **DialogueNode struct** (lines 164-185): Individual dialogue nodes
  - Fields: `id`, `text`, `speaker_override`, `choices`, `conditions`, `actions`, `is_terminal`
- **DialogueChoice struct** (lines 255-270): Player response options
  - Fields: `text`, `target_node`, `conditions`, `actions`, `ends_dialogue`
- **DialogueCondition enum** (lines 311-344): Conditional branching logic
- **DialogueAction enum** (lines 387-426): Game state modifications

#### Game Layer

**File**: `antares/src/game/systems/dialogue.rs`

- **DialoguePlugin** (line 48): Registers dialogue systems
- **StartDialogue event** (lines 35-38): Triggers dialogue start
- **SelectDialogueChoice event** (lines 42-45): Player choice selection
- **handle_start_dialogue system** (lines 70-107): Starts dialogue, sets `GameMode::Dialogue`
- **handle_select_choice system** (lines 113-225): Processes choices, validates conditions
- **evaluate_conditions function** (lines 235-353): Evaluates branching logic
- **execute_action function** (lines 365-464): Executes dialogue actions

#### Application Layer

**File**: `antares/src/application/dialogue.rs` (referenced in systems)

- **DialogueState**: Tracks active dialogue tree and current node

#### Data Files

**File**: `antares/campaigns/tutorial/data/dialogues.ron`

- Format: RON serialization of `Vec<DialogueTree>`
- Example dialogue: "Arcturus Story" (id: 1, root_node: 1)

#### Campaign Builder

**File**: `antares/sdk/campaign_builder/src/dialogue_editor.rs`

- Visual editor for `DialogueTree` structures
- Outputs RON format compatible with domain layer

### Identified Issues

1. **No Visual Feedback**: Current system only updates `GameMode` and logs text to `GameLog`

   - No on-screen dialogue bubbles
   - No indication of who is speaking
   - No visual choice presentation

2. **Synchronous Execution**: Actions execute immediately in update loop

   - No animation timing
   - Text appears instantly (no typewriter effect)
   - Poor user experience

3. **No NPC Integration**: Dialogue system doesn't position visuals relative to NPCs

   - No spatial relationship between speaker and text
   - Breaks immersion in 2.5D environment

4. **Limited State Visibility**: Dialogue state hidden in `GameMode` enum

   - Difficult to query from other systems
   - No component-based approach for ECS integration

5. **Missing Visual Components**: No Bevy components for dialogue UI
   - Text rendering happens in separate UI layer
   - Cannot use Bevy's transform/hierarchy system

---

## Architecture Compliance

### Phase Alignment

Per `docs/reference/architecture.md` Section 8:

- **Phase 3: World (Weeks 6-8)** includes "NPCs and dialogue"
- This implementation completes Phase 3 dialogue requirements
- Adds visual polish typically in Phase 6 (typewriter effects, floating bubbles)

### Data Structure Compliance

- **MAINTAINS** existing `DialogueTree`, `DialogueNode`, `DialogueChoice` structures (Section 4)
- **USES** `GameMode::Dialogue` as defined in architecture (lines 133-138)
- **FOLLOWS** RON data format specification (Section 7.1)

### Module Structure Compliance

Per Section 3.2:

- Domain layer: No changes (data structures remain)
- Game layer: Add new systems and components
- Application layer: Minimal changes to `DialogueState`

---

## Implementation Phases

### Phase 1: Component and Resource Foundation

#### 1.1 Create Dialogue Components Module

**Files to Create**:

- [ ] `antares/src/game/components.rs` (new module declaration)
- [ ] `antares/src/game/components/mod.rs`
- [ ] `antares/src/game/components/dialogue.rs`

**File**: `antares/src/game/components/dialogue.rs`

**Type Aliases to Define** (lines 8-12):

```rust
pub type DialogueBubbleEntity = Entity;
pub type DialogueBackgroundEntity = Entity;
pub type DialogueTextEntity = Entity;
```

**Constants to Define** (lines 14-23):

```rust
pub const DIALOGUE_BUBBLE_Y_OFFSET: f32 = 2.5;
pub const DIALOGUE_BUBBLE_WIDTH: f32 = 4.0;
pub const DIALOGUE_BUBBLE_HEIGHT: f32 = 1.2;
pub const DIALOGUE_BUBBLE_PADDING: f32 = 0.2;
pub const DIALOGUE_TEXT_SIZE: f32 = 24.0;
pub const DIALOGUE_TYPEWRITER_SPEED: f32 = 0.05; // seconds per character
pub const DIALOGUE_BACKGROUND_COLOR: Color = Color::srgba(0.1, 0.1, 0.15, 0.9);
pub const DIALOGUE_TEXT_COLOR: Color = Color::srgb(0.95, 0.95, 0.95);
pub const DIALOGUE_CHOICE_COLOR: Color = Color::srgb(0.8, 0.8, 0.3);
```

**Component Structures to Define** (lines 25-90):

```rust
/// Marks an entity as a dialogue bubble UI element
#[derive(Component, Debug)]
pub struct DialogueBubble {
    /// Entity that spawned this dialogue (typically NPC)
    pub speaker_entity: Entity,
    /// Root entity of the bubble hierarchy
    pub root_entity: Entity,
    /// Background mesh entity
    pub background_entity: Entity,
    /// Text entity
    pub text_entity: Entity,
    /// Vertical offset from speaker position
    pub y_offset: f32,
}

/// Billboard component - makes entity always face the camera
#[derive(Component, Debug)]
pub struct Billboard;

/// Typewriter text animation state
#[derive(Component, Debug)]
pub struct TypewriterText {
    /// Full text to display
    pub full_text: String,
    /// Currently visible character count
    pub visible_chars: usize,
    /// Time since last character reveal
    pub timer: f32,
    /// Seconds per character
    pub speed: f32,
    /// Whether animation is complete
    pub finished: bool,
}

/// Resource tracking active dialogue UI entity
#[derive(Resource, Debug, Default)]
pub struct ActiveDialogueUI {
    pub bubble_entity: Option<Entity>,
}
```

**Deliverables**:

- [ ] `antares/src/game/components.rs` created with `pub mod dialogue;`
- [ ] `antares/src/game/components/mod.rs` created with `pub mod dialogue;`
- [ ] `antares/src/game/components/dialogue.rs` created with all components above
- [ ] `antares/src/game/mod.rs` updated (line 7): Add `pub mod components;`
- [ ] All constants defined with exact values specified above
- [ ] All type aliases defined
- [ ] `cargo check --all-targets --all-features` passes with 0 errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes with 0 warnings

#### 1.2 Update Application Layer DialogueState

**Files to Modify**:

- [ ] `antares/src/application/dialogue.rs`

**Changes Required**:

**Add field** to `DialogueState` struct (insert at appropriate line):

```rust
/// Current node's full text (for visual systems)
pub current_text: String,
/// Current node's speaker name
pub current_speaker: String,
/// Current node's available choices
pub current_choices: Vec<String>,
```

**Add method** to `DialogueState`:

```rust
/// Updates dialogue state with new node information
pub fn update_node(&mut self, text: String, speaker: String, choices: Vec<String>) {
    self.current_text = text;
    self.current_speaker = speaker;
    self.current_choices = choices;
}
```

**Deliverables**:

- [ ] `DialogueState` extended with visual state fields
- [ ] `update_node` method implemented
- [ ] Existing `start` method updated to initialize new fields
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes

#### 1.3 Testing Requirements

**Files to Create**:

- [ ] `antares/src/game/components/dialogue.rs` - test module at end of file (line 100+)

**Tests to Implement**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typewriter_text_initialization() {
        let typewriter = TypewriterText {
            full_text: "Hello, world!".to_string(),
            visible_chars: 0,
            timer: 0.0,
            speed: DIALOGUE_TYPEWRITER_SPEED,
            finished: false,
        };
        assert_eq!(typewriter.visible_chars, 0);
        assert!(!typewriter.finished);
    }

    #[test]
    fn test_dialogue_bubble_constants() {
        assert!(DIALOGUE_BUBBLE_Y_OFFSET > 0.0);
        assert!(DIALOGUE_TYPEWRITER_SPEED > 0.0);
        assert!(DIALOGUE_BUBBLE_WIDTH > 0.0);
    }
}
```

**Deliverables**:

- [ ] Tests implemented in `dialogue.rs` test module
- [ ] `cargo nextest run --all-features` passes with 100% pass rate
- [ ] Code coverage >80% for new components module

#### 1.4 Success Criteria

**Automated Checks**:

- [ ] `cargo fmt --all` produces zero changes
- [ ] `cargo check --all-targets --all-features` exits with code 0
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` reports 0 warnings
- [ ] `cargo nextest run --all-features` shows all tests passing
- [ ] No compilation errors or warnings

**Code Review Checklist**:

- [ ] All constants use `pub const` with explicit types
- [ ] All type aliases follow naming convention (EntitySuffix)
- [ ] Components derive `Component, Debug`
- [ ] Resource derives `Resource, Debug, Default`
- [ ] SPDX copyright header present in all new files

---

### Phase 2: Visual System Implementation

#### 2.1 Create Dialogue Visuals System

**Files to Create**:

- [ ] `antares/src/game/systems/dialogue_visuals.rs`

**File**: `antares/src/game/systems/dialogue_visuals.rs`

**SPDX Header** (lines 1-2):

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0
```

**Module Documentation** (lines 4-15):

```rust
//! Dialogue visual rendering systems
//!
//! This module implements 2.5D floating dialogue bubbles that appear above NPCs during conversations.
//! Features:
//! - Billboard text (always faces camera)
//! - Typewriter text animation
//! - Semi-transparent background panel
//! - Dynamic positioning above speaker entity
//!
//! # Architecture Reference
//! See `docs/reference/architecture.md` Section 3.2 for module structure.
//! See `docs/explanation/dialogue_system_implementation_plan.md` for implementation details.
```

**Imports** (lines 17-30):

```rust
use bevy::prelude::*;
use crate::game::components::dialogue::*;
use crate::application::dialogue::DialogueState;
use crate::application::GameMode;
use crate::game::resources::GlobalState;
```

**System Functions** (lines 32-200):

**Function**: `spawn_dialogue_bubble` (lines 32-120)

```rust
/// Spawns dialogue bubble UI when dialogue starts
///
/// Listens for GameMode::Dialogue state changes and creates a 3D floating bubble
/// positioned above the speaking NPC.
///
/// # Entity Hierarchy
/// - Root (SpatialBundle) - positioned at NPC + Y offset
///   - Background (Mesh3d, Billboard) - semi-transparent panel
///   - Text (Text2d, Billboard, TypewriterText) - animated text
pub fn spawn_dialogue_bubble(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    active_ui: Res<ActiveDialogueUI>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query_npc: Query<&Transform, With</* NPC component marker */>>, // TODO: Define NPC component
) {
    // Check if dialogue just started and no bubble exists
    if let GameMode::Dialogue(ref dialogue_state) = global_state.0.mode {
        if active_ui.bubble_entity.is_some() {
            return; // Bubble already exists
        }

        // TODO: Get NPC entity from dialogue context
        // For now, spawn at world origin + offset
        let speaker_position = Vec3::new(0.0, DIALOGUE_BUBBLE_Y_OFFSET, 0.0);

        // Spawn root entity
        let root_entity = commands.spawn(SpatialBundle {
            transform: Transform::from_translation(speaker_position),
            ..default()
        }).id();

        // Spawn background panel
        let background_entity = commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(DIALOGUE_BUBBLE_WIDTH, DIALOGUE_BUBBLE_HEIGHT))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: DIALOGUE_BACKGROUND_COLOR,
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Billboard,
        )).set_parent(root_entity).id();

        // Spawn text entity
        let text_entity = commands.spawn((
            Text2d::new(&dialogue_state.current_text),
            TextFont {
                font_size: DIALOGUE_TEXT_SIZE,
                ..default()
            },
            TextColor(DIALOGUE_TEXT_COLOR),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)), // Slightly in front
            TypewriterText {
                full_text: dialogue_state.current_text.clone(),
                visible_chars: 0,
                timer: 0.0,
                speed: DIALOGUE_TYPEWRITER_SPEED,
                finished: false,
            },
            Billboard,
        )).set_parent(root_entity).id();

        // Create DialogueBubble component
        commands.entity(root_entity).insert(DialogueBubble {
            speaker_entity: Entity::PLACEHOLDER, // TODO: Track actual NPC entity
            root_entity,
            background_entity,
            text_entity,
            y_offset: DIALOGUE_BUBBLE_Y_OFFSET,
        });

        // Update active UI resource
        commands.insert_resource(ActiveDialogueUI {
            bubble_entity: Some(root_entity),
        });
    }
}
```

**Function**: `update_typewriter_text` (lines 122-160)

```rust
/// Animates typewriter text effect
///
/// Gradually reveals characters over time based on TypewriterText.speed.
pub fn update_typewriter_text(
    time: Res<Time>,
    mut query: Query<(&mut Text2d, &mut TypewriterText)>,
) {
    for (mut text, mut typewriter) in query.iter_mut() {
        if typewriter.finished {
            continue;
        }

        typewriter.timer += time.delta_secs();

        // Reveal next character when timer exceeds speed threshold
        while typewriter.timer >= typewriter.speed {
            typewriter.timer -= typewriter.speed;
            typewriter.visible_chars += 1;

            if typewriter.visible_chars >= typewriter.full_text.len() {
                typewriter.visible_chars = typewriter.full_text.len();
                typewriter.finished = true;
                break;
            }
        }

        // Update visible text
        let visible_text: String = typewriter.full_text
            .chars()
            .take(typewriter.visible_chars)
            .collect();
        **text = visible_text;
    }
}
```

**Function**: `billboard_system` (lines 162-180)

```rust
/// Makes billboard entities always face the camera
pub fn billboard_system(
    camera_query: Query<&Transform, With<Camera3d>>,
    mut billboard_query: Query<&mut Transform, (With<Billboard>, Without<Camera3d>)>,
) {
    if let Ok(camera_transform) = camera_query.get_single() {
        for mut transform in billboard_query.iter_mut() {
            let direction = camera_transform.translation - transform.translation;
            if direction.length_squared() > 0.0 {
                transform.look_at(camera_transform.translation, Vec3::Y);
            }
        }
    }
}
```

**Function**: `cleanup_dialogue_bubble` (lines 182-200)

```rust
/// Despawns dialogue bubble when dialogue ends
pub fn cleanup_dialogue_bubble(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    mut active_ui: ResMut<ActiveDialogueUI>,
    bubble_query: Query<Entity, With<DialogueBubble>>,
) {
    // Check if dialogue ended
    if !matches!(global_state.0.mode, GameMode::Dialogue(_)) {
        if let Some(bubble_entity) = active_ui.bubble_entity.take() {
            if let Ok(entity) = bubble_query.get(bubble_entity) {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}
```

**Deliverables**:

- [ ] `antares/src/game/systems/dialogue_visuals.rs` created with all functions above
- [ ] SPDX header present
- [ ] All functions have `///` doc comments with examples
- [ ] All constants referenced via `crate::game::components::dialogue::*`
- [ ] `cargo check --all-targets --all-features` passes

#### 2.2 Integrate Visual Systems into Plugin

**Files to Modify**:

- [ ] `antares/src/game/systems/mod.rs`
- [ ] `antares/src/game/systems/dialogue.rs`

**File**: `antares/src/game/systems/mod.rs`

**Add** (at appropriate line):

```rust
pub mod dialogue_visuals;
```

**File**: `antares/src/game/systems/dialogue.rs`

**Update** `DialoguePlugin::build` method:

```rust
impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<StartDialogue>()
            .add_message::<SelectDialogueChoice>()
            .init_resource::<crate::game::components::dialogue::ActiveDialogueUI>()
            .add_systems(
                Update,
                (
                    handle_start_dialogue,
                    handle_select_choice,
                    handle_recruitment_actions,
                    crate::game::systems::dialogue_visuals::spawn_dialogue_bubble,
                    crate::game::systems::dialogue_visuals::update_typewriter_text,
                    crate::game::systems::dialogue_visuals::billboard_system,
                    crate::game::systems::dialogue_visuals::cleanup_dialogue_bubble,
                ),
            );
    }
}
```

**Deliverables**:

- [ ] `dialogue_visuals` module declared in `mod.rs`
- [ ] `DialoguePlugin` updated with visual systems
- [ ] `ActiveDialogueUI` resource registered
- [ ] Systems run in `Update` schedule
- [ ] `cargo check --all-targets --all-features` passes

#### 2.3 Testing Requirements

**Files to Create**:

- [ ] `antares/src/game/systems/dialogue_visuals.rs` - test module (line 202+)

**Tests to Implement**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typewriter_reveals_characters_over_time() {
        let mut typewriter = TypewriterText {
            full_text: "Hello".to_string(),
            visible_chars: 0,
            timer: 0.0,
            speed: 0.1,
            finished: false,
        };

        // Simulate time passing
        typewriter.timer += 0.1;
        assert_eq!(typewriter.visible_chars, 0); // Not yet updated

        // Manual update logic (system would do this)
        while typewriter.timer >= typewriter.speed {
            typewriter.timer -= typewriter.speed;
            typewriter.visible_chars += 1;
        }
        assert_eq!(typewriter.visible_chars, 1);
    }

    #[test]
    fn test_typewriter_finishes_when_complete() {
        let mut typewriter = TypewriterText {
            full_text: "Hi".to_string(),
            visible_chars: 2,
            timer: 0.0,
            speed: 0.1,
            finished: false,
        };

        assert_eq!(typewriter.visible_chars, typewriter.full_text.len());
        // Should mark as finished when visible_chars >= full_text.len()
    }
}
```

**Deliverables**:

- [ ] Test module created in `dialogue_visuals.rs`
- [ ] Tests cover typewriter animation logic
- [ ] Tests cover billboard rotation logic (if testable without full Bevy app)
- [ ] `cargo nextest run --all-features` passes
- [ ] Code coverage >80% for `dialogue_visuals.rs`

#### 2.4 Success Criteria

**Automated Checks**:

- [ ] `cargo fmt --all` produces zero changes
- [ ] `cargo check --all-targets --all-features` exits with code 0
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` reports 0 warnings
- [ ] `cargo nextest run --all-features` shows all tests passing

**Manual Verification** (requires running game):

- [ ] Run: `cargo run --bin antares`
- [ ] Load campaign: "tutorial"
- [ ] Trigger dialogue (approach NPC and press interaction key)
- [ ] **VERIFY**: Floating bubble spawns above NPC position
- [ ] **VERIFY**: Text animates character-by-character (visual typewriter effect)
- [ ] **VERIFY**: Background panel is semi-transparent dark color
- [ ] **VERIFY**: Text is white/light colored and readable
- [ ] **VERIFY**: Bubble rotates to face camera when player moves

---

### Phase 3: Event-Driven Logic Integration

#### 3.1 Update Dialogue State on Node Changes

**Files to Modify**:

- [ ] `antares/src/game/systems/dialogue.rs`

**Changes Required**:

**Update** `handle_start_dialogue` function (around line 70-107):

**Add** after setting `GameMode::Dialogue(...)`:

```rust
// Update DialogueState with current node text and choices
if let Some(node) = tree.get_node(root) {
    let speaker = tree.speaker_name.as_deref().unwrap_or("NPC").to_string();
    let choices: Vec<String> = node.choices.iter().map(|c| c.text.clone()).collect();

    if let GameMode::Dialogue(ref mut state) = global_state.0.mode {
        state.update_node(node.text.clone(), speaker, choices);
    }
}
```

**Update** `handle_select_choice` function (around line 113-225):

**Add** after advancing to next node:

```rust
// Update DialogueState with new node information
if let Some(next_node) = tree.get_node(target_node_id) {
    let speaker = tree.speaker_name.as_deref().unwrap_or("NPC").to_string();
    let choices: Vec<String> = next_node.choices.iter().map(|c| c.text.clone()).collect();

    if let GameMode::Dialogue(ref mut state) = global_state.0.mode {
        state.update_node(next_node.text.clone(), speaker, choices);
    }
}
```

**Deliverables**:

- [ ] `handle_start_dialogue` updates `DialogueState` with root node info
- [ ] `handle_select_choice` updates `DialogueState` with next node info
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes

#### 3.2 Add Dialogue Text Update System

**Files to Modify**:

- [ ] `antares/src/game/systems/dialogue_visuals.rs`

**Add System** (line 201+):

```rust
/// Updates dialogue bubble text when node changes
pub fn update_dialogue_text(
    global_state: Res<GlobalState>,
    active_ui: Res<ActiveDialogueUI>,
    mut query_bubble: Query<&DialogueBubble>,
    mut query_text: Query<(&mut Text2d, &mut TypewriterText)>,
) {
    if let GameMode::Dialogue(ref dialogue_state) = global_state.0.mode {
        if let Some(bubble_entity) = active_ui.bubble_entity {
            if let Ok(bubble) = query_bubble.get_mut(bubble_entity) {
                if let Ok((mut text, mut typewriter)) = query_text.get_mut(bubble.text_entity) {
                    // Check if text changed
                    if typewriter.full_text != dialogue_state.current_text {
                        // Reset typewriter animation for new text
                        typewriter.full_text = dialogue_state.current_text.clone();
                        typewriter.visible_chars = 0;
                        typewriter.timer = 0.0;
                        typewriter.finished = false;
                        **text = String::new(); // Clear visible text
                    }
                }
            }
        }
    }
}
```

**Update** `DialoguePlugin` in `antares/src/game/systems/dialogue.rs`:

```rust
.add_systems(
    Update,
    (
        handle_start_dialogue,
        handle_select_choice,
        handle_recruitment_actions,
        crate::game::systems::dialogue_visuals::spawn_dialogue_bubble,
        crate::game::systems::dialogue_visuals::update_dialogue_text, // NEW
        crate::game::systems::dialogue_visuals::update_typewriter_text,
        crate::game::systems::dialogue_visuals::billboard_system,
        crate::game::systems::dialogue_visuals::cleanup_dialogue_bubble,
    ),
);
```

**Deliverables**:

- [ ] `update_dialogue_text` system implemented
- [ ] System registered in `DialoguePlugin`
- [ ] System detects text changes via `Changed<DialogueState>` or direct comparison
- [ ] Typewriter animation resets when text changes
- [ ] `cargo check --all-targets --all-features` passes

#### 3.3 Add Input Handling for Dialogue Advancement

**Files to Modify**:

- [ ] `antares/src/game/systems/input.rs` (or create if doesn't exist)
- [ ] `antares/src/game/systems/dialogue.rs`

**Add Event** to `dialogue.rs`:

```rust
/// Event to advance dialogue (show next text chunk or choice)
#[derive(Event, Debug)]
pub struct AdvanceDialogue;
```

**Register Event** in `DialoguePlugin::build`:

```rust
app.add_event::<AdvanceDialogue>()
```

**Add System** in appropriate input handling location:

```rust
/// System to send AdvanceDialogue event when player presses action key during dialogue
pub fn dialogue_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    global_state: Res<GlobalState>,
    mut ev_advance: EventWriter<AdvanceDialogue>,
) {
    if matches!(global_state.0.mode, GameMode::Dialogue(_)) {
        if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::KeyE) {
            ev_advance.send(AdvanceDialogue);
        }
    }
}
```

**Deliverables**:

- [ ] `AdvanceDialogue` event defined
- [ ] Input system sends event on Space/E key press during dialogue
- [ ] Event registered in plugin
- [ ] `cargo check --all-targets --all-features` passes

#### 3.4 Testing Requirements

**Files to Create**:

- [ ] `antares/tests/dialogue_integration_test.rs`

**Integration Test Structure**:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;
use antares::game::systems::dialogue::*;
use antares::game::components::dialogue::*;
use antares::application::dialogue::DialogueState;
use antares::application::GameMode;

#[test]
fn test_dialogue_state_updates_on_start() {
    // Setup minimal Bevy app
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Add necessary resources and plugins

    // Trigger StartDialogue event
    // Verify DialogueState.current_text is populated
    // Verify DialogueState.current_speaker is populated
}

#[test]
fn test_dialogue_bubble_spawns_when_dialogue_starts() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Setup dialogue systems

    // Start dialogue
    // Run update cycles
    // Query for DialogueBubble component
    // Assert entity exists
}

#[test]
fn test_typewriter_text_animates_over_frames() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    // Spawn entity with TypewriterText
    // Advance time
    // Run update
    // Check visible_chars increased
}
```

**Deliverables**:

- [ ] Integration test file created
- [ ] Tests verify dialogue state updates
- [ ] Tests verify bubble spawning
- [ ] Tests verify typewriter animation
- [ ] `cargo nextest run --all-features` passes all integration tests

#### 3.5 Success Criteria

**Automated Checks**:

- [ ] `cargo fmt --all` produces zero changes
- [ ] `cargo check --all-targets --all-features` exits with code 0
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` reports 0 warnings
- [ ] `cargo nextest run --all-features` passes 100% of tests
- [ ] Integration tests pass

**Functional Tests** (manual verification):

- [ ] Run: `cargo run --bin antares`
- [ ] Load campaign: "tutorial"
- [ ] Approach NPC "Arcturus" at position specified in map data
- [ ] Press 'E' to interact
- [ ] **VERIFY**: Dialogue bubble spawns with text "I am the Great Arcturus!"
- [ ] **VERIFY**: Text animates character-by-character
- [ ] **VERIFY**: Press Space or E to advance
- [ ] **VERIFY**: Text updates to next node: "Hello There! Come and sit and talk a while."
- [ ] **VERIFY**: Typewriter animation resets and plays for new text
- [ ] **VERIFY**: When dialogue ends, bubble despawns cleanly

---

### Phase 4: Choice Display and Selection

#### 4.1 Add Choice Display Components

**Files to Modify**:

- [ ] `antares/src/game/components/dialogue.rs`

**Add Component** (line 95+):

```rust
/// Marks a choice button entity
#[derive(Component, Debug)]
pub struct DialogueChoiceButton {
    /// Index in DialogueState.current_choices
    pub choice_index: usize,
}

/// Resource tracking currently selected choice
#[derive(Resource, Debug, Default)]
pub struct SelectedChoice {
    pub index: usize,
}
```

**Add Constants** (after existing constants):

```rust
pub const CHOICE_BUTTON_WIDTH: f32 = 3.5;
pub const CHOICE_BUTTON_HEIGHT: f32 = 0.4;
pub const CHOICE_BUTTON_SPACING: f32 = 0.5;
pub const CHOICE_Y_OFFSET: f32 = -1.0; // Below dialogue text
```

**Deliverables**:

- [ ] `DialogueChoiceButton` component defined
- [ ] `SelectedChoice` resource defined
- [ ] Choice display constants defined
- [ ] `cargo check --all-targets --all-features` passes

#### 4.2 Create Choice Spawning System

**Files to Modify**:

- [ ] `antares/src/game/systems/dialogue_visuals.rs`

**Add System** (line 250+):

```rust
/// Spawns choice buttons below dialogue text
pub fn spawn_dialogue_choices(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    active_ui: Res<ActiveDialogueUI>,
    query_bubble: Query<&DialogueBubble>,
    query_existing: Query<Entity, With<DialogueChoiceButton>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Despawn old choices
    for entity in query_existing.iter() {
        commands.entity(entity).despawn_recursive();
    }

    if let GameMode::Dialogue(ref dialogue_state) = global_state.0.mode {
        if dialogue_state.current_choices.is_empty() {
            return; // No choices to display
        }

        if let Some(bubble_entity) = active_ui.bubble_entity {
            if let Ok(bubble) = query_bubble.get(bubble_entity) {
                let num_choices = dialogue_state.current_choices.len();

                for (index, choice_text) in dialogue_state.current_choices.iter().enumerate() {
                    let y_position = CHOICE_Y_OFFSET - (index as f32 * CHOICE_BUTTON_SPACING);

                    // Spawn choice button background
                    let button_entity = commands.spawn((
                        Mesh3d(meshes.add(Plane3d::default().mesh().size(CHOICE_BUTTON_WIDTH, CHOICE_BUTTON_HEIGHT))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: DIALOGUE_CHOICE_COLOR,
                            unlit: true,
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        })),
                        Transform::from_translation(Vec3::new(0.0, y_position, 0.0)),
                        Billboard,
                        DialogueChoiceButton { choice_index: index },
                    )).set_parent(bubble.root_entity).id();

                    // Spawn choice text
                    commands.spawn((
                        Text2d::new(choice_text),
                        TextFont {
                            font_size: DIALOGUE_TEXT_SIZE * 0.8,
                            ..default()
                        },
                        TextColor(Color::srgb(0.1, 0.1, 0.1)), // Dark text on light button
                        Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                        Billboard,
                    )).set_parent(button_entity);
                }
            }
        }
    }
}
```

**Deliverables**:

- [ ] `spawn_dialogue_choices` system implemented
- [ ] System despawns old choices before spawning new ones
- [ ] Choices positioned below dialogue text with proper spacing
- [ ] Choice text rendered on buttons
- [ ] `cargo check --all-targets --all-features` passes

#### 4.3 Add Choice Selection Input

**Files to Modify**:

- [ ] `antares/src/game/systems/dialogue.rs` (or input system location)

**Add System**:

```rust
/// Handles numeric key input to select dialogue choices
pub fn dialogue_choice_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    global_state: Res<GlobalState>,
    mut ev_select: EventWriter<SelectDialogueChoice>,
    mut selected_choice: ResMut<SelectedChoice>,
) {
    if let GameMode::Dialogue(ref dialogue_state) = global_state.0.mode {
        let num_choices = dialogue_state.current_choices.len();

        if num_choices == 0 {
            return;
        }

        // Number keys 1-9 for choice selection
        for key_num in 1..=9 {
            let key_code = match key_num {
                1 => KeyCode::Digit1,
                2 => KeyCode::Digit2,
                3 => KeyCode::Digit3,
                4 => KeyCode::Digit4,
                5 => KeyCode::Digit5,
                6 => KeyCode::Digit6,
                7 => KeyCode::Digit7,
                8 => KeyCode::Digit8,
                9 => KeyCode::Digit9,
                _ => continue,
            };

            if keyboard.just_pressed(key_code) {
                let choice_index = key_num - 1;
                if choice_index < num_choices {
                    ev_select.send(SelectDialogueChoice { choice_index });
                    selected_choice.index = choice_index;
                }
            }
        }
    }
}
```

**Register System** in `DialoguePlugin`:

```rust
.add_systems(
    Update,
    (
        // ... existing systems ...
        dialogue_choice_input_system,
    ),
);
```

**Deliverables**:

- [ ] Choice input system implemented
- [ ] Number keys 1-9 map to choice indices 0-8
- [ ] `SelectDialogueChoice` event sent on valid key press
- [ ] System registered in plugin
- [ ] `cargo check --all-targets --all-features` passes

#### 4.4 Testing Requirements

**Files to Modify**:

- [ ] `antares/tests/dialogue_integration_test.rs`

**Add Tests**:

```rust
#[test]
fn test_dialogue_choices_spawn_correctly() {
    let mut app = App::new();
    // Setup app with dialogue systems

    // Create DialogueState with choices
    // Run update
    // Query for DialogueChoiceButton entities
    // Assert correct number spawned
}

#[test]
fn test_choice_selection_advances_dialogue() {
    let mut app = App::new();
    // Setup app

    // Start dialogue with choices
    // Send SelectDialogueChoice event with index 0
    // Run update
    // Verify dialogue advanced to target node
}
```

**Deliverables**:

- [ ] Tests verify choice spawning
- [ ] Tests verify choice selection
- [ ] `cargo nextest run --all-features` passes

#### 4.5 Success Criteria

**Automated Checks**:

- [ ] `cargo fmt --all` produces zero changes
- [ ] `cargo check --all-targets --all-features` exits with code 0
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` reports 0 warnings
- [ ] `cargo nextest run --all-features` passes 100%

**Manual Verification**:

- [ ] Run: `cargo run --bin antares`
- [ ] Start dialogue with "Arcturus" (dialogue_id: 1)
- [ ] After root node displays, verify choices appear:
  - [ ] Choice 1: "Tell me more."
  - [ ] Choice 2: "Farewell."
- [ ] Press '1' key
- [ ] **VERIFY**: Dialogue advances to node 2
- [ ] **VERIFY**: Text updates to "Hello There! Come and sit and talk a while."
- [ ] **VERIFY**: Choices despawn (terminal node)
- [ ] Restart dialogue, press '2' key at first choice
- [ ] **VERIFY**: Dialogue ends, bubble despawns

---

### Phase 5: Documentation and Polish

#### 5.1 Update Implementation Documentation

**Files to Modify**:

- [ ] `antares/docs/explanation/implementations.md`

**Add Section**:

```markdown
## Dialogue System with 2.5D Visuals

**Implementation Date**: [YYYY-MM-DD]
**Phase**: Phase 3 - World (NPCs and Dialogue)

### Overview

Implemented enhanced dialogue system with 2.5D floating text bubbles, typewriter animation effects, and event-driven state management using native Bevy ECS patterns.

### Components Implemented

- **DialogueBubble**: Main component tracking bubble entity hierarchy
- **TypewriterText**: Animates text character-by-character reveal
- **Billboard**: Makes text/UI elements face camera in 2.5D space
- **DialogueChoiceButton**: Marks choice selection buttons
- **ActiveDialogueUI**: Resource tracking active dialogue bubble entity

### Systems Implemented

- **spawn_dialogue_bubble**: Creates 3D floating UI above speaker
- **update_dialogue_text**: Updates text when node changes
- **update_typewriter_text**: Animates text reveal
- **billboard_system**: Rotates entities to face camera
- **cleanup_dialogue_bubble**: Despawns UI when dialogue ends
- **spawn_dialogue_choices**: Creates choice buttons below dialogue
- **dialogue_choice_input_system**: Handles number key input for choices

### Constants Defined

Located in `antares/src/game/components/dialogue.rs`:

- DIALOGUE_BUBBLE_Y_OFFSET: 2.5
- DIALOGUE_BUBBLE_WIDTH: 4.0
- DIALOGUE_BUBBLE_HEIGHT: 1.2
- DIALOGUE_TEXT_SIZE: 24.0
- DIALOGUE_TYPEWRITER_SPEED: 0.05

### Data Format

Continues using existing `DialogueTree` RON format. No migration required.

### Testing Coverage

- Unit tests: 15 tests covering components and animation logic
- Integration tests: 5 tests covering full dialogue flow
- Coverage: >85% for dialogue_visuals.rs

### Files Modified

- Created: src/game/components/dialogue.rs
- Created: src/game/systems/dialogue_visuals.rs
- Modified: src/game/systems/dialogue.rs
- Modified: src/application/dialogue.rs
- Created: tests/dialogue_integration_test.rs
```

**Deliverables**:

- [ ] `implementations.md` updated with dialogue system section
- [ ] All implementation details documented
- [ ] File paths and line numbers included
- [ ] Test coverage statistics included

#### 5.2 Add Example Dialogue Data

**Files to Create**:

- [ ] `antares/campaigns/tutorial/data/example_complex_dialogue.ron`

**Example Dialogue** (demonstrating all features):

```ron
[
    (
        id: 999,
        name: "Complex Dialogue Example",
        root_node: 1,
        nodes: {
            1: (
                id: 1,
                text: "Greetings, adventurer! I am the keeper of ancient knowledge.",
                speaker_override: Some("Ancient Keeper"),
                choices: [
                    (
                        text: "What knowledge do you possess?",
                        target_node: Some(2),
                        conditions: [],
                        actions: [],
                        ends_dialogue: false,
                    ),
                    (
                        text: "I seek a quest.",
                        target_node: Some(3),
                        conditions: [],
                        actions: [],
                        ends_dialogue: false,
                    ),
                    (
                        text: "Farewell.",
                        target_node: None,
                        conditions: [],
                        actions: [],
                        ends_dialogue: true,
                    ),
                ],
                conditions: [],
                actions: [],
                is_terminal: false,
            ),
            2: (
                id: 2,
                text: "I know of the ancient ruins to the north. Many treasures lie within, but also great danger.",
                speaker_override: None,
                choices: [
                    (
                        text: "Tell me about the treasures.",
                        target_node: Some(4),
                        conditions: [],
                        actions: [],
                        ends_dialogue: false,
                    ),
                    (
                        text: "What dangers lurk there?",
                        target_node: Some(5),
                        conditions: [],
                        actions: [],
                        ends_dialogue: false,
                    ),
                ],
                conditions: [],
                actions: [],
                is_terminal: false,
            ),
            3: (
                id: 3,
                text: "Very well! I have a quest for you. Retrieve the Crystal of Light from the ruins.",
                speaker_override: None,
                choices: [
                    (
                        text: "I accept this quest!",
                        target_node: None,
                        conditions: [],
                        actions: [
                            StartQuest(quest_id: 10),
                            GiveGold(amount: 100),
                        ],
                        ends_dialogue: true,
                    ),
                    (
                        text: "I need more information first.",
                        target_node: Some(2),
                        conditions: [],
                        actions: [],
                        ends_dialogue: false,
                    ),
                ],
                conditions: [],
                actions: [],
                is_terminal: false,
            ),
            4: (
                id: 4,
                text: "Legends speak of powerful magical artifacts and mountains of gold.",
                speaker_override: None,
                choices: [
                    (
                        text: "Thank you for the information.",
                        target_node: None,
                        conditions: [],
                        actions: [],
                        ends_dialogue: true,
                    ),
                ],
                conditions: [],
                actions: [],
                is_terminal: false,
            ),
            5: (
                id: 5,
                text: "Undead creatures, traps, and dark magic protect the ruins. Only the brave should venture there.",
                speaker_override: None,
                choices: [
                    (
                        text: "I am brave. I will go.",
                        target_node: Some(3),
                        conditions: [],
                        actions: [],
                        ends_dialogue: false,
                    ),
                    (
                        text: "Perhaps I should prepare more.",
                        target_node: None,
                        conditions: [],
                        actions: [],
                        ends_dialogue: true,
                    ),
                ],
                conditions: [],
                actions: [],
                is_terminal: false,
            ),
        },
        speaker_name: Some("Ancient Keeper"),
        repeatable: true,
        associated_quest: Some(10),
    ),
]
```

**Deliverables**:

- [ ] Example dialogue file created demonstrating:
  - [ ] Multiple branching paths
  - [ ] Conditional choices
  - [ ] Dialogue actions (quest start, gold reward)
  - [ ] Speaker name override
  - [ ] Terminal and non-terminal nodes
- [ ] File saved in RON format
- [ ] File validates with existing `DialogueTree` deserializer

#### 5.3 Create User Documentation

**Files to Create**:

- [ ] `antares/docs/how-to/create_dialogues.md`

**Content**:

````markdown
# How to Create Dialogues

This guide explains how to create dialogue trees for NPCs in Antares.

## File Location

Dialogues are stored in RON format: `campaigns/{campaign_name}/data/dialogues.ron`

## Basic Structure

```ron
[
    (
        id: 1,                    // Unique dialogue ID (u16)
        name: "NPC Greeting",     // Editor reference name
        root_node: 1,             // Starting node ID
        nodes: {                  // Map of node_id -> DialogueNode
            1: (
                id: 1,
                text: "Hello, traveler!",
                speaker_override: None,
                choices: [
                    (
                        text: "Hello!",
                        target_node: Some(2),
                        conditions: [],
                        actions: [],
                        ends_dialogue: false,
                    ),
                ],
                conditions: [],
                actions: [],
                is_terminal: false,
            ),
        },
        speaker_name: Some("Village Elder"),
        repeatable: true,
        associated_quest: None,
    ),
]
```
````

## Node Types

- **Branch Node**: Has multiple choices, not terminal
- **Terminal Node**: `is_terminal: true`, ends dialogue
- **Action Node**: Executes actions (give items, start quests)

## Dialogue Actions

Available actions in choice or node:

- `StartQuest(quest_id: 10)`: Starts a quest
- `GiveItems(items: [(item_id: 5, quantity: 1)])`: Gives items to party
- `GiveGold(amount: 100)`: Awards gold
- `TakeGold(amount: 50)`: Removes gold
- `GrantExperience(amount: 500)`: Awards XP

## Dialogue Conditions

Control choice availability:

- `HasQuest(quest_id: 10)`: Player has quest active
- `CompletedQuest(quest_id: 10)`: Quest is complete
- `HasItem(item_id: 5, quantity: 1)`: Player has item
- `HasGold(amount: 100)`: Player has minimum gold
- `MinLevel(level: 5)`: Character level requirement

## Visual Presentation

- Dialogues appear as floating bubbles above NPCs
- Text animates with typewriter effect (0.05s per character)
- Choices numbered 1-9, press number key to select
- Press Space or E to advance dialogue

## Testing Your Dialogue

1. Add dialogue to `dialogues.ron`
2. Assign to NPC in map data: `dialogue_override: Some(your_dialogue_id)`
3. Run game: `cargo run --bin antares`
4. Interact with NPC (press 'E')

## Tips

- Keep dialogue text under 100 characters for readability
- Use speaker_override for multi-character conversations
- Test all branching paths
- Validate RON syntax with `campaign_validator`

```

**Deliverables**:
- [ ] User guide created with examples
- [ ] All dialogue features documented
- [ ] RON syntax examples included
- [ ] Testing instructions provided

#### 5.4 Success Criteria

**Documentation Checks**:
- [ ] `implementations.md` updated with complete implementation summary
- [ ] Example dialogue file validates successfully
- [ ] User guide created in `docs/how-to/`
- [ ] All new files have SPDX headers
- [ ] Markdown files use lowercase_with_underscores.md naming

**Quality Checks**:
- [ ] All documentation reviewed for clarity
- [ ] Code examples in docs are syntactically correct
- [ ] Links to files use correct paths
- [ ] No broken references

---

## Final Validation Checklist

### Code Quality
- [ ] `cargo fmt --all` produces zero changes
- [ ] `cargo check --all-targets --all-features` exits with code 0
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` reports 0 warnings
- [ ] `cargo nextest run --all-features` passes 100% of tests
- [ ] No `unwrap()` or `expect()` without justification
- [ ] All public items have `///` doc comments with examples
- [ ] Code coverage >80% for new modules

### Testing
- [ ] Unit tests for all components (dialogue.rs test module)
- [ ] Unit tests for visual systems (dialogue_visuals.rs test module)
- [ ] Integration tests (tests/dialogue_integration_test.rs)
- [ ] Manual testing completed with all scenarios passing
- [ ] Example dialogue tested in-game

### Documentation
- [ ] `docs/explanation/implementations.md` updated
- [ ] `docs/how-to/create_dialogues.md` created
- [ ] Example dialogue file created
- [ ] All filenames use `lowercase_with_underscores.md`
- [ ] No emojis in documentation
- [ ] All code blocks specify language or file path

### Files and Structure
- [ ] All new `.rs` files have SPDX headers (lines 1-2)
- [ ] All files in correct module structure per architecture.md
- [ ] No circular dependencies introduced
- [ ] Module declarations added to parent `mod.rs` files
- [ ] Components in `src/game/components/dialogue.rs`
- [ ] Systems in `src/game/systems/dialogue_visuals.rs`

### Architecture Compliance
- [ ] No modifications to `domain::dialogue` data structures
- [ ] Continues using existing RON dialogue format
- [ ] No new external dependencies added (bevy only)
- [ ] Proper separation of concerns (domain/application/game layers)
- [ ] Constants defined, not hardcoded
- [ ] Type aliases used consistently
- [ ] No changes to Campaign Builder required

### Backward Compatibility
- [ ] Existing dialogue files load without modification
- [ ] Old dialogue system still functions (coexists)
- [ ] Campaign Builder continues working unchanged
- [ ] No breaking changes to `DialogueTree` struct

---

## Dependency Analysis

### No New External Dependencies Required

**Decision**: Implement dialogue visuals using only existing Bevy dependencies.

**Rationale**:
- **bevy_talks**: Not needed - existing `domain::dialogue` provides sufficient graph structure
- **bevy_animated_text**: Not needed - typewriter effect is ~30 lines of code
- **bevy_mod_billboard**: Not needed - billboard rotation is ~15 lines of code

**Benefits**:
- No dependency management overhead
- Smaller binary size
- Full control over implementation
- Easier to customize for game-specific needs

**Existing Dependencies Used**:
- `bevy` = "0.17" (already in Cargo.toml line 28)
  - ECS framework
  - Transform/hierarchy system
  - Mesh/material rendering
  - Input handling

---

## Migration Strategy

### No Migration Required

**Current System**: Uses `domain::dialogue::DialogueTree` with RON format
**New System**: Extends current system with visual layer

**Approach**:
1. **Domain layer unchanged**: `DialogueTree` remains source of truth
2. **Application layer extended**: `DialogueState` gains visual state fields
3. **Game layer enhanced**: New visual systems read from existing state
4. **Data format unchanged**: Continue using `.ron` files with `DialogueTree` format

**Backward Compatibility**:
- All existing dialogue files work without modification
- Campaign Builder continues outputting same format
- Old synchronous system can remain for headless testing
- New visual systems are additive, not replacements

**Testing Strategy**:
- Verify existing dialogue files load correctly
- Test that old `handle_start_dialogue` still works
- Ensure visual systems are optional (can be disabled)

---

## Risk Assessment

### Low Risk
- **No external dependencies**: Reduces supply chain vulnerabilities
- **No data migration**: Existing content works unchanged
- **Additive changes**: Old systems still functional
- **Isolated implementation**: Changes contained to game layer

### Medium Risk
- **Performance**: Billboard system runs every frame on all dialogue entities
  - **Mitigation**: Only runs when `GameMode::Dialogue` active
  - **Mitigation**: Typically 1-3 entities max (dialogue bubble + choices)
- **Visual glitches**: Text might flicker during animation
  - **Mitigation**: Test thoroughly with various text lengths
  - **Mitigation**: Tune `DIALOGUE_TYPEWRITER_SPEED` constant

### Monitored Areas
- Bevy version compatibility (currently 0.17)
- Text rendering performance with long dialogue strings
- Memory usage with deeply nested dialogue trees

---

## Open Questions

### Resolved
1. **Use external dialogue framework?** → NO, extend existing domain::dialogue
2. **Data format migration?** → NO, keep existing RON format
3. **Billboard dependency?** → NO, implement custom

### For User Decision
1. **NPC Entity Tracking**: How to link DialogueBubble to NPC entity?
   - **Option A**: Add `speaker_entity` field to `StartDialogue` event
   - **Option B**: Query for NPC component at dialogue position
   - **Option C**: Store in `DialogueState` when interaction starts
   - **Recommended**: Option A (most explicit)

2. **Choice Highlighting**: Should selected choice be visually highlighted?
   - **Option A**: Change color when selected
   - **Option B**: Add selection indicator (arrow/bracket)
   - **Option C**: No highlighting, just number labels
   - **Recommended**: Option B (clearest feedback)

3. **Text Speed Configuration**: Should typewriter speed be configurable?
   - **Option A**: Global constant only
   - **Option B**: Per-dialogue override in RON data
   - **Option C**: User preference setting
   - **Recommended**: Option A for initial implementation, Option C for future

---

## Success Metrics

### Quantitative
- [ ] Zero compilation errors
- [ ] Zero clippy warnings
- [ ] 100% test pass rate
- [ ] >80% code coverage for new modules
- [ ] <10ms per-frame overhead for dialogue systems
- [ ] Zero memory leaks (bubble cleanup verified)

### Qualitative
- [ ] Dialogue bubbles are visually clear and readable
- [ ] Typewriter effect feels smooth and natural
- [ ] Choice selection is intuitive
- [ ] System is easy to extend with new features
- [ ] Code is well-documented and maintainable

### User Acceptance
- [ ] Can create new dialogues using existing editor
- [ ] Dialogues display correctly in-game
- [ ] Text animation feels polished
- [ ] Choice selection is responsive
- [ ] No regressions in existing functionality

---

## Timeline Estimate

### Phase 1: Foundation (2-3 hours)
- Component definitions
- Constant setup
- Resource creation
- Basic tests

### Phase 2: Visuals (4-5 hours)
- Spawn/cleanup systems
- Typewriter animation
- Billboard rotation
- Visual system tests

### Phase 3: Integration (3-4 hours)
- State update logic
- Text change detection
- Input handling
- Integration tests

### Phase 4: Choices (3-4 hours)
- Choice display
- Input handling
- Selection logic
- Choice tests

### Phase 5: Documentation (2-3 hours)
- Implementation docs
- User guide
- Example dialogue

**Total Estimated Time**: 14-19 hours for complete implementation

---

## Appendix: File Structure

### New Files Created
```

antares/
├── src/
│ ├── game/
│ │ ├── components/
│ │ │ ├── mod.rs (new)
│ │ │ └── dialogue.rs (new)
│ │ └── systems/
│ │ └── dialogue_visuals.rs (new)
│ └── ...
├── tests/
│ └── dialogue_integration_test.rs (new)
├── campaigns/
│ └── tutorial/
│ └── data/
│ └── example_complex_dialogue.ron (new)
└── docs/
├── explanation/
│ └── implementations.md (updated)
└── how-to/
└── create_dialogues.md (new)

```

### Modified Files
```

antares/
├── src/
│ ├── game/
│ │ ├── mod.rs (add components module)
│ │ └── systems/
│ │ ├── mod.rs (add dialogue_visuals module)
│ │ └── dialogue.rs (update plugin, add state updates)
│ └── application/
│ └── dialogue.rs (extend DialogueState)
└── Cargo.toml (no changes - uses existing bevy dependency)

```

---

**End of Implementation Plan**

This plan provides explicit, AI-agent-executable instructions for implementing an enhanced dialogue system while maintaining full backward compatibility with existing content and tools.
```
