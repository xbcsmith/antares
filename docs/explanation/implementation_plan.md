# Game Engine Implementation Plan

## Overview
This plan outlines the integration of the Bevy game engine into the Antares project. The goal is to replace the current CLI/TUI loop with a graphical, first-person, pseudo-3D interface while preserving the existing domain logic and SDK workflow.

## Current State Analysis

### Existing Infrastructure
-   **Domain Logic**: Robust `antares::domain` crate with RPG systems (Items, Spells, Monsters, Maps).
-   **SDK**: `campaign_builder` tool for creating game content (RON files).
-   **Application**: `antares` binary currently runs a CLI loop.
-   **Data**: `data/` directory containing core game assets.

### Identified Issues
-   **No Graphics**: Current game is text-only.
-   **No Engine**: Missing windowing, input handling, and rendering systems.
-   **CLI Limitation**: Cannot support "progressively more complicated games" as requested.

## Implementation Phases

## Implementation Phases

### Phase 1: Bevy Integration & Core Loop
**Goal**: Replace the CLI loop with a Bevy App window that runs the game state.

#### 1.1 Add Dependencies
-   Add `bevy` (default features) and `bevy_egui` to `Cargo.toml`.
-   Ensure compatibility with existing `serde`, `ron` dependencies.

#### 1.2 Refactor Entry Point
-   Modify `src/bin/antares.rs` to initialize `bevy::App`.
-   Create `AntaresPlugin` to organize game systems.
-   Wrap `antares::application::GameState` in a Bevy `Resource`.

#### 1.3 Implement Game State Resource
-   Create `struct GlobalState(pub GameState);` wrapper.
-   Initialize `GlobalState` using existing `CampaignLoader` to read `campaign.ron`, `items.ron`, etc.

#### 1.4 Testing Requirements
-   `cargo test` must pass.
-   Manual: Run `cargo run --bin antares` -> Should open a window (black screen) instead of CLI.

#### 1.5 Deliverables
-   Compilable `antares` binary with Bevy.
-   Window opens and closes correctly.

#### 1.6 Success Criteria
-   Application launches a window.
-   `GameState` is accessible within Bevy systems.

### Phase 2: Map Rendering (Pseudo-3D)
**Goal**: Visualize the maze-like world using Bevy's 3D capabilities.

#### 2.1 Map Resource & Loading
-   System to read `MapId` from `GlobalState` and load the corresponding `Map` data (from `maps/*.ron`).
-   Spawn 3D primitives (cubes/planes) for walls based on the grid.

#### 2.2 Basic Camera
-   Spawn a `Camera3dBundle`.
-   Position camera based on `GlobalState.party.location`.

#### 2.3 Texture Loading
-   Load basic textures for walls/floor from `assets/`.
-   Apply materials to meshes.

#### 2.4 Testing Requirements
-   Manual: Load a campaign, verify walls appear in correct grid positions.

#### 2.5 Deliverables
-   `MapRenderingPlugin`.
-   Visual representation of the current map.

#### 2.6 Success Criteria
-   Can see walls and floor in 3D.
-   Map matches the data defined in SDK.

### Phase 3: Player Movement & Interaction
**Goal**: Allow the player to navigate the world.

#### 3.1 Input Handling
-   Map Keyboard (WASD/Arrows) to `MoveAction` events.
-   Handle discrete grid-based movement (Forward, Backward, Turn Left, Turn Right).

#### 3.2 State Update System
-   System to process `MoveAction`.
-   Call `antares::domain` logic to validate move (check walls, doors).
-   Update `GlobalState.party.location`.

#### 3.3 Camera Sync
-   System to interpolate Camera transform to new `party.location`.
-   Add smooth transition animations (optional but recommended).

#### 3.4 Testing Requirements
-   Manual: Walk around the map. Collision with walls should stop movement.

#### 3.5 Deliverables
-   `InputPlugin`.
-   `MovementSystem`.

#### 3.6 Success Criteria
-   Player can move through the maze.
-   Walls block movement.
-   Turning updates view 90 degrees.

### Phase 4: Event System Implementation
**Goal**: Implement logic to handle map events (Teleport, Signs, Traps, etc.).

#### 4.1 Event Trigger System
-   System to check `GlobalState.map.events` at `party.location` after movement.
-   Emit `MapEventTriggered` events.

#### 4.2 Event Handlers
-   **Teleport**: System to handle map transitions (load new map, update position).
-   **Sign/Text**: Display text in the game log or a modal window.
-   **Trap**: Apply damage to party members and flash screen.
-   **Treasure**: Add items to party inventory (basic implementation).
-   **Encounter**: Placeholder for combat start (e.g., "A group of monsters attacks!").

#### 4.3 Testing Requirements
-   Manual: Step on a teleport tile -> load new map.
-   Manual: Step on a sign -> see text.

#### 4.4 Deliverables
-   `EventPlugin`.
-   Functional Teleporters and Signs.

#### 4.5 Success Criteria
-   Can travel between maps.
-   Can interact with world objects.

### Phase 5: UI & HUD Integration
**Goal**: Display game information using `bevy_egui`.

#### 5.1 HUD Layout
-   Implement `bevy_egui` system.
-   Draw "Viewports": 3D View (Top Left), Party List (Bottom), Text Log (Right).

#### 5.2 Party Status
-   Render party member names, HP, SP, Status in the Party List panel.

#### 5.3 Game Log
-   Redirect `println!` style output to an on-screen text log buffer.

#### 5.4 Testing Requirements
-   Manual: Verify HUD updates when party takes damage or moves.

#### 5.5 Deliverables
-   `UiPlugin`.
-   Functional HUD.

#### 5.6 Success Criteria
-   Can see party stats.
-   Can read game messages.

### Phase 6: Campaign Integration
**Goal**: Ensure full compatibility with SDK-generated campaigns.

#### 6.1 Campaign Loader UI
-   Add "Main Menu" state in Bevy.
-   List available campaigns (scanned from `campaigns/`).
-   Button to "Load Campaign" -> transitions to Gameplay state.

#### 6.2 Save/Load UI
-   In-game menu to trigger `SaveGameManager`.

#### 6.3 Testing Requirements
-   Manual: Create campaign in SDK, launch Game, select Campaign, Play.

#### 6.4 Deliverables
-   Main Menu scene.
-   Campaign selection flow.

#### 6.5 Success Criteria
-   Seamless flow from SDK creation to Game execution.

## Copyright
We will follow the [SPDX Spec](https://spdx.github.io/spdx-spec/) for copyright and licensing information.
