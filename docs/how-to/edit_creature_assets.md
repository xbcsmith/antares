# How to Edit Creature Assets

This guide explains how to use the Creature Asset Editor UI to create and modify creature visual definitions in the Antares campaign builder.

## Overview

The Creature Asset Editor provides a comprehensive interface for:

- Managing creature meshes (add, remove, duplicate, reorder)
- Editing mesh transforms (translation, rotation, scale)
- Modifying mesh colors and properties
- Replacing meshes with primitive shapes
- Previewing creatures in 3D (preview integration)
- Setting creature-level properties (scale, color tint)
- Saving and loading creature assets

## Opening the Asset Editor

1. Launch the campaign builder application
2. Navigate to the **Creatures** section
3. Select a creature from the registry list, or click **New Creature**
4. The editor will switch to **Edit Mode** with the three-panel layout

## Editor Layout

The asset editor uses a three-panel layout:

```
+------------------+------------------------+--------------------+
| Mesh List        | 3D Preview             | Mesh Properties    |
| (250px)          | (flex)                 | (350px)            |
|                  |                        |                    |
| ☑ head           | [Interactive Preview]  | Transform          |
| ☑ torso          |                        | Translation X: 0.0 |
| ☐ left_arm       | Grid | Wireframe       | Translation Y: 0.0 |
| ☐ right_arm      | Camera Distance: 5.0   | Translation Z: 0.0 |
| ☑ left_leg       |                        |                    |
| ☑ right_leg      |                        | Rotation (degrees) |
|                  |                        | Pitch: 0.0         |
| [Add] [Dupe] [X] |                        | Yaw: 0.0           |
+------------------+------------------------+--------------------+
| Creature Properties: ID [1] Name [Goblin] Scale [1.0] Tint [▣]|
+----------------------------------------------------------------+
```

### Left Panel: Mesh List

- **Visibility Checkboxes**: Toggle mesh visibility in preview
- **Color Indicator**: Colored dot showing mesh color
- **Mesh Name**: Display name or `unnamed_mesh_N` if not set
- **Vertex Count**: Badge showing number of vertices `(234 verts)`
- **Selection**: Click to select and edit mesh properties

**Toolbar Buttons:**

- **➕ Add Primitive**: Opens primitive generator dialog
- **📋 Duplicate**: Clones selected mesh and transform
- **🗑 Delete**: Removes selected mesh (with confirmation)

### Center Panel: 3D Preview

The preview displays your creature in real-time with camera controls:

**Controls:**

- **Grid**: Toggle ground grid helper
- **Wireframe**: Show wireframe overlay
- **Normals**: Display normal vectors (debug)
- **Axes**: Show coordinate axes (X=red, Y=green, Z=blue)
- **🔄 Reset Camera**: Return to default view
- **Camera Distance**: Slider to zoom (1.0 - 10.0)
- **Background**: Color picker for background

**Camera Interaction:**

- **Left-drag**: Rotate camera around creature
- **Right-drag**: Pan camera (move target)
- **Scroll wheel**: Zoom in/out
- **Double-click**: Focus on selected mesh

> **Note**: Full 3D preview integration with Bevy is pending. Current version shows a placeholder.

### Right Panel: Mesh Properties

When a mesh is selected, this panel shows detailed properties:

**Mesh Info:**

- **Name**: Text field for mesh name
- **Color**: RGBA color picker
- **Vertices**: Count (read-only)
- **Triangles**: Count (read-only)

**Transform:**

- **Translation**: X, Y, Z sliders (-5.0 to 5.0)
- **Rotation**: Pitch, Yaw, Roll in degrees (0-360)
- **Scale**: X, Y, Z with optional uniform scaling checkbox

**Geometry:**

- Vertex/triangle counts
- Normals presence indicator
- UVs presence indicator

**Action Buttons:**

- **🔄 Replace with Primitive**: Generate new geometry
- **🔍 Validate Mesh**: Check for geometry issues
- **↺ Reset Transform**: Return to identity transform

### Bottom Panel: Creature Properties

Global creature settings:

- **ID**: Creature ID with category badge (read-only in edit mode)
- **Name**: Creature display name
- **Scale**: Global scale multiplier (0.1 - 5.0)
- **Color Tint**: Optional RGBA tint applied to all meshes

**File Operations:**

- **💾 Save Asset**: Write changes to file
- **💾 Save As...**: Create new creature file
- **📋 Export RON**: Copy as RON text
- **↺ Revert Changes**: Reload from file

## Common Tasks

### Adding a New Mesh

1. Click **➕ Add Primitive** in the mesh list toolbar
2. Select primitive type: Cube | Sphere | Cylinder | Pyramid | Cone
3. Configure settings (size, segments, etc.)
4. Choose color:
   - **Use current mesh color**: Inherit from selected mesh
   - **Custom**: Pick a custom RGBA color
5. Click **✓ Generate**

The new mesh appears in the mesh list with an identity transform.

### Editing Mesh Transforms

1. **Select a mesh** from the mesh list (left panel)
2. Expand **Transform** section in properties panel (right)
3. Adjust values:
   - **Translation**: Drag sliders or type values
   - **Rotation**: Enter degrees (0-360)
   - **Scale**: Enable/disable uniform scaling checkbox
4. Preview updates in real-time
5. Click **💾 Save Asset** when done

**Tips:**

- Enable **Uniform Scaling** to maintain proportions
- Use **↺ Reset Transform** to return to identity
- Translation is in world units relative to creature origin

### Changing Mesh Colors

1. Select a mesh from the mesh list
2. In the properties panel, click the **Color** button
3. Adjust RGBA sliders:
   - **R, G, B**: Color channels (0.0 - 1.0)
   - **A**: Alpha/transparency (0.0 = transparent, 1.0 = opaque)
4. Preview updates immediately
5. Click **💾 Save Asset** to commit

### Replacing a Mesh with a Primitive

To regenerate mesh geometry while preserving transforms:

1. Select the mesh to replace
2. Click **🔄 Replace with Primitive** in properties panel
3. **Primitive Generator Dialog** opens:
   - Select primitive type
   - Configure size and subdivision settings
   - Choose color options
   - Check **Preserve transform** to keep position/rotation/scale
   - Check **Keep mesh name** to retain the name
4. Click **✓ Generate**

The mesh geometry is replaced in-place. This is useful for:

- Converting between primitive types
- Adjusting subdivision levels (e.g., low-poly to high-poly sphere)
- Regenerating geometry after parameter changes

### Duplicating Meshes

1. Select a mesh from the mesh list
2. Click **📋 Duplicate** in the toolbar
3. A copy is created with the same transform
4. Modify the duplicate's transform to position it differently

**Use Cases:**

- Creating symmetrical body parts (left/right arms)
- Repeating elements (armor plates, scales)
- Creating variations with slight transform differences

### Removing Meshes

1. Select the mesh to remove
2. Click **🗑 Delete** in the toolbar
3. Mesh and its transform are removed immediately

**Warning**: This action is immediate. Use **↺ Revert Changes** to undo if needed.

### Reordering Meshes

Mesh order affects rendering (later meshes draw over earlier ones).

To reorder meshes:

1. Use drag-and-drop in the mesh list (if implemented)
2. Or manually swap meshes in the underlying data structure

> **Note**: Drag-to-reorder support is in the implementation plan but may not be active yet.

### Setting Creature Scale

The global scale multiplier affects all meshes:

1. Locate **Scale** slider in the bottom creature properties panel
2. Adjust from 0.1 (tiny) to 5.0 (huge)
3. Preview shows scaled creature
4. Scale of 1.0 = original size

**Example Scales:**

- **0.5**: Small creatures (rats, cats)
- **1.0**: Human-sized
- **1.5**: Large humanoids (ogres)
- **2.0+**: Giants, dragons

### Applying Color Tint

To tint all meshes with a color overlay:

1. In creature properties panel (bottom), check the **Color Tint** checkbox
2. Click the color picker button
3. Adjust RGBA values
4. Tint is multiplied with each mesh's color

**Use Cases:**

- Status effects (poisoned = green tint, frozen = blue tint)
- Team colors in multiplayer
- Day/night lighting adjustments

To remove tint, uncheck the **Color Tint** checkbox.

### Saving Your Work

**Save to Current File:**

1. Click **💾 Save Asset** in creature properties panel
2. Changes are written to the creature's `.ron` file
3. Unsaved changes indicator clears

**Save As New Creature:**

1. Click **💾 Save As...**
2. Enter new creature ID and name
3. New file is created, registry updated

**Export RON Text:**

1. Click **📋 Export RON**
2. RON representation is copied to clipboard
3. Paste into text editor or documentation

**Revert Changes:**

1. Click **↺ Revert Changes**
2. Creature reloads from file, discarding edits
3. Use if you want to abandon unsaved changes

## Primitive Types Reference

### Cube

- **Parameters**: Size (edge length)
- **Vertices**: 24 (6 faces × 4 vertices)
- **Triangles**: 12 (6 faces × 2 triangles)
- **Use Cases**: Buildings, blocks, robot parts, furniture

### Sphere

- **Parameters**: Radius, Segments (longitude), Rings (latitude)
- **Vertices**: (Rings + 1) × (Segments + 1)
- **Triangles**: Variable based on subdivisions
- **Use Cases**: Heads, balls, planets, round objects
- **Tips**: Higher segments/rings = smoother sphere, more polygons

### Cylinder

- **Parameters**: Radius, Height, Segments
- **Vertices**: Variable (includes caps)
- **Triangles**: Variable
- **Use Cases**: Limbs, columns, trees, barrels

### Pyramid

- **Parameters**: Base Size (square base)
- **Vertices**: 5 (4 base corners + 1 apex)
- **Triangles**: 6 (2 for base + 4 sides)
- **Use Cases**: Egyptian pyramids, roofs, spikes

### Cone

- **Parameters**: Base Radius, Height, Segments
- **Vertices**: Variable (circular base + apex)
- **Triangles**: Variable
- **Use Cases**: Wizard hats, horns, cones, pointed objects

## Tips and Best Practices

### Performance Optimization

- **Keep vertex counts reasonable**: 100-500 vertices per mesh for most creatures
- **Use LOD (Level of Detail)**: Simpler meshes for distant creatures (future feature)
- **Avoid excessive subdivision**: 8-16 segments usually sufficient for spheres/cylinders

### Modeling Workflow

1. **Start with basic primitives**: Block out the creature shape
2. **Position major body parts**: Head, torso, limbs
3. **Add details**: Hands, feet, facial features
4. **Adjust transforms**: Fine-tune positions and scales
5. **Set colors**: Differentiate body parts
6. **Test at various scales**: Ensure creature looks good at gameplay scales

### Naming Conventions

- Use descriptive names: `head`, `left_arm`, `torso`, not `mesh_1`
- Include side for symmetric parts: `left_arm`, `right_arm`
- Use consistent casing: `lower_case_with_underscores`
- Examples:
  - `head`
  - `torso_upper`
  - `left_hand`
  - `right_leg_lower`
  - `tail_segment_1`

### Color Guidelines

- **Keep meshes distinct**: Different colors for different body parts
- **Consider lighting**: Colors appear darker in-game with lighting
- **Use alpha carefully**: Transparency can cause rendering issues
- **Test with color tint**: Ensure meshes still visible when tinted

### Transform Guidelines

- **Origin at ground level**: Place creature so Y=0 is the ground
- **Face forward**: Creature should face +Z direction by default
- **Center horizontally**: X=0, Z=0 should be creature center
- **Reasonable scales**: Mesh scale 1.0 ± 50% is typical

## Troubleshooting

### Mesh Not Visible in Preview

- Check visibility checkbox in mesh list
- Verify mesh has vertices and indices
- Ensure camera distance allows viewing (try Reset Camera)
- Check if mesh color alpha is > 0.0

### Transform Not Working

- Verify mesh is selected
- Check that values are within slider ranges
- Try resetting transform and re-applying
- Ensure preview dirty flag is set (triggers update)

### Colors Look Wrong

- Check mesh color alpha channel (should be 1.0 for opaque)
- Verify creature color tint isn't overriding
- Preview lighting may affect appearance
- Export and test in actual game renderer

### Save Fails

- Check campaign directory exists and is writable
- Verify creature ID is valid (1-255)
- Ensure name is not empty
- Check for validation errors (ID conflicts, etc.)

### Validation Errors

- **No Meshes**: Add at least one mesh
- **Mismatched Transform Count**: Ensure meshes and transforms arrays are same length
- **Invalid Mesh Data**: Regenerate mesh with primitive generator
- **Duplicate ID**: Change creature ID to avoid conflicts

## Keyboard Shortcuts (Future Feature)

Planned shortcuts for Phase 5:

- **Ctrl+S**: Save asset
- **Ctrl+Z**: Undo
- **Ctrl+Y**: Redo
- **Delete**: Remove selected mesh
- **Ctrl+D**: Duplicate selected mesh
- **R**: Reset camera
- **G**: Toggle grid
- **W**: Toggle wireframe

## Next Steps

After editing creature assets:

1. **Validate Creature**: Ensure no errors or warnings
2. **Test in Preview**: Verify appearance from multiple angles
3. **Save Asset**: Write changes to file
4. **Update Registry**: Ensure creature is registered
5. **Test in Game**: Load campaign and verify creature appears correctly

## Related Documentation

- [Manage Creature Registry](manage_creature_registry.md) - Phase 1 registry management
- [Architecture Reference](../reference/architecture.md) - System design
- [Primitive Generators API](../reference/primitive_generators.md) - Mesh generation functions

## Conclusion

The Creature Asset Editor provides a comprehensive visual workflow for creating and modifying creature meshes. By combining primitive generation, transform editing, and real-time preview, you can rapidly iterate on creature designs without external 3D modeling tools.

For complex creatures requiring custom meshes, future phases will add OBJ import and advanced mesh editing tools.
