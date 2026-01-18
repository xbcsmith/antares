# Game Engine Options Analysis

## Context
The current `antares` codebase consists of:
1.  **Core Domain Logic**: Extensive implementation of RPG systems (Stats, Items, Spells, Combat, etc.) in `src/domain`.
2.  **CLI/TUI Application**: A text-based game loop in `src/bin/antares.rs` using `rustyline`.
3.  **SDK**: A campaign builder GUI (using `egui` presumably, based on the plan, though I didn't check the SDK deps, likely `eframe`).

The goal is to build a **Might and Magic 1 style engine** (First-person, turn-based, pseudo-3D, tile-based) that can support "progressively more complicated games".

## Option 1: Custom Rust Engine (From Scratch)

### Approach
-   Build a custom rendering loop using low-level libraries.
-   **Windowing**: `winit`
-   **Rendering**: `wgpu` (modern, hard) or `pixels`/`softbuffer` (retro, easier) or `minifb`.
-   **Audio**: `rodio` or `kira`.
-   **Input**: `winit` events or `gilrs` for gamepads.

### Pros
-   **Total Control**: The engine is tailored exactly to the game's needs (e.g., specific raycasting for pseudo-3D).
-   **No Bloat**: Only includes exactly what is needed.
-   **Architecture Fit**: Can directly use the existing Object-Oriented/Functional `domain` structs without adapting to an Entity-Component-System (ECS).

### Cons
-   **High Effort**: Requires implementing basic systems (windowing, loop, time, asset loading, text rendering) before gameplay code.
-   **Maintenance**: You own every bug in the engine.
-   **Scalability**: "Progressively more complicated games" means you will be constantly adding features to your engine (physics, particles, advanced UI) that general-purpose engines already have.

## Option 2: Leverage Bevy (Recommended)

### Approach
-   Use Bevy as the framework for Windowing, Input, Audio, and Rendering.
-   **Integration**:
    -   **Model**: Keep existing `antares::domain` logic as the "Source of Truth".
    -   **View/Controller**: Use Bevy Systems to read Input, update the `domain` state, and Render the scene.
    -   Store the `GameState` as a Bevy `Resource`.

### Pros
-   **Feature Rich**: Out-of-the-box support for 2D/3D rendering, UI, Audio, Input, Asset Management.
-   **Scalability**: Excellent for "progressively more complicated games". Bevy is capable of modern 3D graphics, complex shaders, etc.
-   **Productivity**: Focus on *gameplay* (drawing the maze, handling combat) rather than *plumbing* (opening a window, loading a font).
-   **Cross-Platform**: Easier deployment to Web (WASM), Windows, Mac, Linux.

### Cons
-   **Learning Curve**: Bevy's ECS paradigm is different.
-   **Binary Size**: Larger than a minimal custom engine.
-   **Architecture Friction**: Need to bridge the gap between the existing "Rust Structs" domain and Bevy's ECS. (Mitigated by using the "Resource" pattern initially).

## Recommendation

**Leverage Bevy.**

Rationale:
1.  **Future Proofing**: The requirement to support "more complicated games" strongly favors a general-purpose engine.
2.  **Velocity**: We can get a MM1-style renderer up and running much faster with Bevy's sprite/mesh tools than writing a raycaster or rasterizer from scratch.
3.  **SDK Synergy**: The SDK likely uses `egui`. Bevy has excellent `bevy_egui` integration, allowing us to potentially merge tools or share UI logic.

### Proposed Next Steps (if Bevy is chosen)
1.  Add `bevy` and `bevy_egui` dependencies.
2.  Refactor `src/bin/antares.rs` to initialize a Bevy App instead of a `loop`.
3.  Wrap `antares::application::GameState` in a Bevy `Resource`.
4.  Implement a `render_maze` system using Bevy's 3D or 2D features to visualize the grid.
