# Creature Editor Workflows

This guide covers the end-to-end workflows for creating and editing creatures
using the Antares Campaign Builder creature editor.

## Overview

The creature editor operates in two modes:

- **Registry Mode** - Browse, search, and manage all registered creatures.
- **Asset Editor Mode** - Edit a single creature's meshes, transforms, colors,
  and properties with real-time 3D preview.

The top bar always shows the current mode indicator
(e.g. `Registry Mode` or `Asset Editor: goblin.ron`) and a breadcrumb trail
(e.g. `Creatures > Goblin > left_leg`).

---

## Creating a New Creature

### From Scratch

1. Open the **Creatures** tab in the Campaign Builder.
2. The editor starts in **Registry Mode**.
3. Click **New** in the toolbar.
4. A dialog appears: **Create From Template or Scratch?**
   - Choose **From Scratch** to start with an empty creature.
5. The editor switches to **Asset Editor Mode**.
   - The top bar shows: `Asset Editor: new_creature.ron`
   - Breadcrumb: `Creatures > New Creature`
6. In the **Mesh List** panel (left), click **Add Mesh** to add your first mesh.
7. Select a primitive type (Cube, Sphere, Cylinder, Pyramid, or Cone) and
   configure its properties.
8. The **3D Preview** panel (center) updates in real time.
9. Use the **Mesh Properties** panel (right) to adjust the mesh transform,
   color, and geometry.
10. When satisfied, click **Save** or press `Ctrl+S`.
    - The asset file is written to disk.
    - The registry entry is added automatically.
11. Click **Back to Registry** or press `Escape` to return to the list view.

### From a Template

1. Open the **Creatures** tab.
2. Click **New** in the toolbar.
3. In the dialog choose **From Template**.
4. The **Template Browser** opens. Browse by category
   (Humanoid, Creature, Undead, Robot, Primitive) or search by name/tag.
5. Select a template and click **Apply**.
6. The editor opens in **Asset Editor Mode** with the template meshes
   pre-populated.
7. Customize meshes, transforms, and colors as needed.
8. Click **Save** (`Ctrl+S`) and return to the registry.

---

## Editing an Existing Creature

1. In **Registry Mode**, locate the creature in the list.
   - Use the **Search** box to filter by name or ID.
   - Use the **Category** dropdown to filter by type.
2. Select the creature row and click **Edit Asset**, or double-click the row.
3. The editor switches to **Asset Editor Mode**.
   - The top bar shows: `Asset Editor: <filename>.ron`
   - Breadcrumb: `Creatures > <Name>`
4. The three-panel layout is shown:
   - **Left** – Mesh List
   - **Center** – 3D Preview
   - **Right** – Mesh Properties / Creature-Level Properties
5. Make changes (add/remove/reorder meshes, edit transforms, change colors).
6. Save with `Ctrl+S` or the **Save** button.
7. Return to the registry with **Back to Registry** or `Escape`.

---

## Switching Between Registry and Asset Editor

The top bar always shows a **Back to Registry** button, visible in both modes.
Click it at any time to return to the registry list.

If there are unsaved changes, the editor marks the session dirty
(`has_unsaved_changes = true`). A future prompt will warn you before discarding
unsaved work.

---

## Keyboard Shortcuts

All shortcuts are registered in the `ShortcutManager` and can be customised.
Defaults are:

### Navigation

| Action | Shortcut |
|--------|----------|
| Save current asset | `Ctrl+S` |
| New creature | `Ctrl+N` |
| Open template browser | `Ctrl+O` |
| Return to registry | `Escape` |
| Cycle panels | `Tab` |

### Editing

| Action | Shortcut |
|--------|----------|
| Undo | `Ctrl+Z` |
| Redo | `Ctrl+Y` |
| Delete selected mesh | `Del` |
| Duplicate selected mesh | `Ctrl+D` |
| Select all | `Ctrl+A` |

### Preview

| Action | Shortcut |
|--------|----------|
| Reset camera | `Space` |
| Pan camera | `W` / `A` / `S` / `D` |
| Rotate camera | `Q` / `E` |
| Toggle wireframe | `R` |
| Toggle grid | `G` |

---

## Context Menus

### Mesh List (right-click on a mesh entry)

- **Edit Properties** – Opens the Mesh Properties panel for this mesh.
- **Duplicate** – Creates a copy of the mesh and appends it to the list.
- **Delete** – Removes the mesh (undoable via `Ctrl+Z`).
- **Move Up / Move Down** – Reorders the mesh in the list.
- **Export to OBJ** – Exports the mesh geometry to an OBJ file.
- **Copy as RON** – Copies the mesh definition to the clipboard as RON text.
- **Hide / Show in Preview** – Toggles mesh visibility in the 3D preview.

### 3D Preview (right-click in the preview area)

- **Reset Camera** – Returns the camera to its default isometric position.
- **Focus on Selected** – Frames the camera around the selected mesh.
- **Wireframe On/Off** – Toggles wireframe overlay.
- **Grid On/Off** – Toggles the ground-plane grid.
- **Normals On/Off** – Displays vertex normal vectors.
- **Snapshot** – Saves a PNG screenshot of the current preview.
- **Copy Camera Settings** – Copies the current camera config to the clipboard.

---

## Undo and Redo

Every destructive edit is recorded in the `CreatureUndoRedoManager` and can be
undone with `Ctrl+Z` / redone with `Ctrl+Y`.

The following operations are tracked:

| Operation | Description |
|-----------|-------------|
| **Add Mesh** | Adds a mesh and its transform to the creature. |
| **Remove Mesh** | Removes a mesh and stores it for redo/undo. |
| **Modify Transform** | Records old and new transform for a mesh. |
| **Modify Mesh** | Records old and new mesh geometry. |
| **Modify Creature Props** | Records old and new name, scale, and tint. |

The undo stack is cleared whenever you open a different creature for editing.

The status bar shows the description of the next undo/redo action:

```text
Undo: Add mesh 'head'
Redo: Remove mesh 'tail'
```

---

## Auto-Save and Crash Recovery

The editor automatically saves a backup of the creature being edited.

### Configuration

- Interval: every **60 seconds** of dirty (unsaved) state.
- Max backups: **5** per creature.
- Location: `.campaign_builder/autosave/creature_<name>_<timestamp>.ron`

### Recovery

On startup, the editor scans for auto-save files. If any are found, a prompt
appears:

```text
Recover unsaved changes to "Goblin"?  [Yes]  [No]
```

- **Yes** – Loads the auto-save file into the editor.
- **No** – Discards the auto-save and continues normally.

Auto-save is also triggered before every risky operation (primitive replacement,
mass-delete, etc.) so that a crash mid-operation can be recovered.

---

## Enhanced Preview Features

### Preview Toolbar

The preview panel toolbar provides:

- **Snapshot** – Save the current viewport as a PNG image.
- **Turntable** – Auto-rotate the creature for a 360-degree inspection.
- **Lighting** dropdown – Choose from `Day`, `Night`, `Dungeon`, or `Studio`
  lighting presets.
- **Animation Speed** slider – Adjust playback speed when animations are loaded.

### Preview Overlays

Enable overlays in the preview options (gear icon in the panel header):

| Overlay | Description |
|---------|-------------|
| Mesh Names | Shows each mesh name on hover. |
| Transform Axes | Displays X/Y/Z axes for the selected mesh. |
| Bounding Boxes | Shows per-mesh axis-aligned bounding boxes. |
| Center of Mass | Marks the aggregate center of all meshes. |
| Normals | Visualises vertex normals as short line segments. |
| Statistics | Shows mesh count, vertex count, triangle count, and FPS. |

---

## Typical Workflows

### Workflow A: Create Goblin from Template

```text
1. Creatures tab → New → From Template
2. Template Browser → Category: Humanoid → Select "Basic Humanoid"
3. Asset Editor opens with 6 meshes pre-built
4. Rename creature: "Goblin"
5. Adjust scale: 0.7 (shorter than human)
6. Change color tint: [0.3, 0.6, 0.2, 1.0] (green)
7. Ctrl+S → Saved to goblin.ron
8. Escape → Registry Mode
```

### Workflow B: Edit Dragon, Undo Mistake

```text
1. Registry → Search "Dragon" → Edit Asset
2. Asset Editor: dragon.ron
3. Select "wings" mesh → Delete (Del key)
   - Undo stack: [Delete wings]
4. Realise mistake → Ctrl+Z
   - Wings restored
5. Modify wing transform instead
6. Ctrl+S → Saved
7. Escape → Registry
```

### Workflow C: Recover After Crash

```text
1. Launch Campaign Builder
2. Prompt: "Recover unsaved changes to Lich? [Yes] [No]"
3. Click Yes
4. Asset Editor opens with pre-crash Lich state
5. Review changes, then Ctrl+S to confirm save
```

---

## Validation

Before saving, the editor validates the creature asset:

- Every mesh must have at least one vertex.
- Index arrays must be multiples of 3 (triangle lists).
- No index may exceed the vertex count.
- Normals count, when present, must equal vertex count.

Validation errors appear in the **Validation Panel** (toggle with the
shield icon). Warnings do not block saving; errors do.

---

## File Format

Creature assets are stored in RON format:

```ron
(
    id: 42,
    name: "Goblin",
    meshes: [
        (
            name: Some("body"),
            vertices: [
                (0.0, 0.0, 0.0),
                (1.0, 0.0, 0.0),
                (0.5, 1.0, 0.0),
            ],
            indices: [0, 1, 2],
            normals: None,
            uvs: None,
            color: (0.3, 0.6, 0.2, 1.0),
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        ),
    ],
    mesh_transforms: [
        (
            translation: (0.0, 0.0, 0.0),
            rotation: (0.0, 0.0, 0.0),
            scale: (1.0, 1.0, 1.0),
        ),
    ],
    scale: 0.7,
    color_tint: Some((0.3, 0.6, 0.2, 1.0)),
)
```

The registry file (`creatures.ron`) stores lightweight references:

```ron
[
    (id: 42, name: "Goblin",   filepath: "assets/creatures/goblin.ron"),
    (id: 43, name: "Troll",    filepath: "assets/creatures/troll.ron"),
    (id: 44, name: "Dragon",   filepath: "assets/creatures/dragon.ron"),
]
```

---

## Related How-To Guides

- `manage_creature_registry.md` – Registry Management UI (Phase 1)
- `edit_creature_assets.md` – Asset Editor UI and mesh editing (Phases 2-4)
- `create_creatures.md` – General creature creation overview
- `using_creatures_editor.md` – Complete creatures editor reference
