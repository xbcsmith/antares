# How to Create Creatures

A comprehensive guide to creating custom creatures using the procedural mesh system.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Basic Customization](#basic-customization)
3. [Creating Variations](#creating-variations)
4. [Working with Meshes](#working-with-meshes)
5. [Advanced Features](#advanced-features)
6. [Best Practices](#best-practices)
7. [Troubleshooting](#troubleshooting)

---

## Getting Started

### Opening the Creature Editor

The Creature Editor is part of the Campaign Builder application. There are two
ways to navigate to it inside an open campaign:

- **Path A (via Tools menu):** Click **Tools** → **Creature Editor**. This
  switches the active panel to the Creatures editor without leaving the current
  campaign session.
- **Path B (direct tab):** Click the **Creatures** tab in the left sidebar.

Both paths open the same editor. You must have a campaign open before either
path is available.

### Creature Editor Layout

When you first enter the Creatures editor it shows the **Registry List** mode:
a flat list of all registered creature assets in the campaign, with a toolbar
at the top (New, Edit, Delete, Duplicate). Single-clicking a creature in the
list selects it.

The three-panel layout (Mesh List / 3D Preview placeholder / Mesh Properties)
only appears after you open an individual creature for editing, either by
clicking **New** to create one from scratch or by selecting a creature and
clicking **Edit** (or double-clicking the row).

### Understanding Templates

Templates are pre-built creature structures that serve as starting points:

- **Humanoid** (Beginner): Two-legged creatures with torso, head, and arms
- **Quadruped** (Intermediate): Four-legged animals with tail
- **Dragon** (Advanced): Winged creatures with complex structure
- **Robot** (Intermediate): Mechanical creatures with modular parts
- **Undead** (Intermediate): Skeletal creatures with bone structure
- **Beast** (Advanced): Muscular predators with claws and horns

### Loading Your First Template

1. In the Template Browser, click **Browse Templates**
2. Select a category (e.g., "Humanoid")
3. Click on a template to see its preview
4. Click **Load Template** to start customizing

---

## Basic Customization

### Changing Colors

Each creature has a base color that can be modified:

1. Select your creature in the preview pane
2. In the Properties Panel, find **Color Tint**
3. Click the color picker
4. Adjust RGB values (range: 0.0 to 1.0)
5. Click **Apply**

**Example color values:**

- Red: `[1.0, 0.0, 0.0, 1.0]`
- Green: `[0.0, 1.0, 0.0, 1.0]`
- Blue: `[0.0, 0.0, 1.0, 1.0]`
- Gold: `[1.0, 0.84, 0.0, 1.0]`

### Adjusting Scale

Change the overall size of your creature:

1. In the Properties Panel, find **Scale**
2. Enter a scale value:
   - `0.5` = Half size
   - `1.0` = Normal size (default)
   - `2.0` = Double size
   - `5.0` = Five times larger
3. Press **Enter** or click **Apply**

### Modifying Transforms

Each mesh part has its own transform (position, rotation, scale):

1. In the Mesh List, select a specific mesh (e.g., "Head")
2. Expand the **Transform** section
3. Modify:
   - **Translation**: Move the mesh (X, Y, Z coordinates)
   - **Rotation**: Rotate the mesh (in radians)
   - **Scale**: Resize the mesh independently

**Example**: To move the head forward:

- Set Translation Z to `0.5`

---

## Creating Variations

Variations allow you to create multiple versions of a creature without duplicating the entire definition.

### Using the Variation Editor

1. With your creature loaded, click **Edit** → **Variations**
2. Click **Add Variation**
3. Enter a variation name (e.g., "Red Dragon")
4. Modify the variation properties:
   - **Color Tint**: Change the color
   - **Scale**: Adjust size
   - **Mesh Visibility**: Show/hide specific meshes

### Example: Creating Color Variants

To create red and blue dragon variants:

1. Load the dragon template
2. Create variation "Red Dragon":
   - Color Tint: `[1.0, 0.2, 0.2, 1.0]`
3. Create variation "Blue Dragon":
   - Color Tint: `[0.2, 0.4, 1.0, 1.0]`
4. Save each variation

### Example: Creating Size Variants

To create young and ancient dragon variants:

1. Create variation "Young Dragon":
   - Scale: `0.5`
2. Create variation "Ancient Dragon":
   - Scale: `3.0`

---

## Working with Meshes

### Understanding Mesh Structure

Each creature is made of multiple meshes (body parts). A humanoid template has:

- Torso
- Head
- Left arm
- Right arm
- Left leg (optional)
- Right leg (optional)

### Adding Meshes

1. Click **Mesh** → **Add Mesh**
2. Choose a primitive generator:
   - **Cube**: Box-shaped mesh
   - **Sphere**: Rounded mesh (approximated with vertices)
   - **Cylinder**: Tube-shaped mesh
   - **Pyramid**: Pointed mesh
3. Set the mesh properties (size, color, position)
4. Click **Create**

### Removing Meshes

1. Select a mesh in the Mesh List
2. Click **Mesh** → **Remove Mesh**
3. Confirm the deletion

### Editing Mesh Properties

For each mesh, you can edit:

1. **Vertices**: The 3D points that define the mesh shape
2. **Indices**: The triangles connecting vertices
3. **Normals**: Surface direction (for lighting)
4. **UVs**: Texture coordinates
5. **Color**: Base color of the mesh

### Primitive Generators

Use primitive generators to quickly create common shapes:

**Cube Generator:**

```
Width: 1.0
Height: 1.0
Depth: 1.0
Center: [0.0, 0.0, 0.0]
```

**Sphere Generator:**

```
Radius: 0.5
Subdivisions: 16
Center: [0.0, 0.0, 0.0]
```

**Cylinder Generator:**

```
Radius: 0.3
Height: 1.0
Segments: 12
```

### Mesh Validation

Before saving, validate your meshes:

1. Click **Tools** → **Validate Mesh**
2. Check for errors:
   - Degenerate triangles
   - Inverted normals
   - Disconnected vertices
3. Click **Auto-Fix** to resolve common issues

---

## Advanced Features

### Generating LOD Levels

LOD (Level of Detail) improves performance by using simpler meshes at a distance:

1. Select your creature
2. Click **Tools** → **Generate LOD**
3. Configure LOD settings:
   - **Number of Levels**: 3 (recommended)
   - **Reduction Factor**: 0.5 (each level has half the triangles)
4. Click **Generate**

The system will create:

- LOD 0: Full detail (close range)
- LOD 1: Medium detail (medium range)
- LOD 2: Low detail (far range)

### Applying Materials and Textures

Materials define how light interacts with your creature:

1. Select a mesh
2. Click **Material** → **Edit Material**
3. Set material properties:
   - **Base Color**: Primary color
   - **Metallic**: 0.0 (non-metallic) to 1.0 (metallic)
   - **Roughness**: 0.0 (shiny) to 1.0 (rough)
   - **Emissive**: Self-illumination color
   - **Alpha Mode**: Opaque, Blend, or Mask

**Example material (Shiny Metal):**

```
Base Color: [0.7, 0.7, 0.8, 1.0]
Metallic: 1.0
Roughness: 0.2
Emissive: [0.0, 0.0, 0.0, 1.0]
Alpha Mode: Opaque
```

### Adding Textures

1. In the Material Editor, click **Add Texture**
2. Browse to your texture file (PNG, JPEG)
3. Select the texture path
4. The texture will be applied to the mesh UVs

### Creating Simple Animations

Create keyframe animations for basic movement:

1. Click **Animation** → **New Animation**
2. Name your animation (e.g., "Idle")
3. Set duration (e.g., `2.0` seconds)
4. Add keyframes:
   - **Keyframe 0.0s**: Starting pose
   - **Keyframe 1.0s**: Middle pose (e.g., head rotated)
   - **Keyframe 2.0s**: Return to starting pose
5. Click **Preview Animation** to test

**Example: Bobbing head animation**

```
Keyframe 0.0s: Head Y = 1.5
Keyframe 1.0s: Head Y = 1.6
Keyframe 2.0s: Head Y = 1.5
```

---

## Best Practices

### Avoiding Degenerate Triangles

Degenerate triangles (zero area) cause rendering issues:

**Bad:**

```
Triangle with vertices at same position:
[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]
```

**Good:**

```
Triangle with distinct vertices:
[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]
```

### Proper Normal Orientation

Normals should point outward from the surface:

1. In the Mesh Editor, enable **Show Normals**
2. Check that arrows point away from the mesh
3. If normals are inverted, click **Flip Normals**

### UV Mapping Guidelines

For proper texture mapping:

1. UVs should be in the range `0.0` to `1.0`
2. Avoid overlapping UVs (unless intentional)
3. Minimize stretching and distortion

### Performance Considerations

Keep your creatures performant:

- **Vertex Count**: Aim for < 1000 vertices per creature
- **Triangle Count**: Keep under 2000 triangles for common creatures
- **Mesh Count**: Fewer meshes = better performance (combine when possible)
- **LOD Usage**: Always generate LOD levels for creatures used in combat
- **Texture Size**: Use power-of-two textures (256x256, 512x512, 1024x1024)

### Naming Conventions

Use clear, descriptive names:

**Good:**

- `red_dragon_adult.ron`
- `skeleton_warrior.ron`
- `goblin_shaman.ron`

**Bad:**

- `creature1.ron`
- `test.ron`
- `untitled.ron`

---

## Troubleshooting

### Preview is Black or Invisible

**Possible causes:**

- Color values outside 0.0-1.0 range
- All meshes hidden
- Camera positioned incorrectly

**Solutions:**

1. Reset color tint to `[1.0, 1.0, 1.0, 1.0]`
2. Check mesh visibility in Mesh List
3. Click **View** → **Reset Camera**

### Mesh Appears Inside-Out

**Cause:** Inverted normals or incorrect winding order

**Solution:**

1. Select the mesh
2. Click **Mesh** → **Flip Normals**
3. Or click **Mesh** → **Recalculate Normals**

### Creature Has Holes or Gaps

**Cause:** Missing triangles or incorrect indices

**Solution:**

1. Click **Tools** → **Validate Mesh**
2. Review the error report
3. Manually fix missing indices or use **Auto-Repair**

### Changes Don't Save

**Cause:** File permissions or invalid data

**Solution:**

1. Check that you have write permissions to the data directory
2. Validate your creature before saving
3. Ensure creature ID is unique

### Performance is Slow

**Cause:** Too many vertices or complex meshes

**Solution:**

1. Generate LOD levels
2. Reduce mesh complexity
3. Combine multiple small meshes into one

### Texture Doesn't Appear

**Possible causes:**

- Texture path is incorrect
- UVs are not defined
- Texture file format not supported

**Solutions:**

1. Verify texture path is relative to project root
2. Generate UVs if missing: **Mesh** → **Generate UVs**
3. Use PNG or JPEG format

---

## Examples

### Example 1: Creating a Fire Demon

Starting from the humanoid template:

1. Load `humanoid.ron`
2. Set color tint to `[1.0, 0.3, 0.0, 1.0]` (orange-red)
3. Scale to `1.5` (larger than normal)
4. Add emissive material:
   - Emissive: `[0.8, 0.2, 0.0, 1.0]`
5. Add spike meshes to head (using Pyramid primitive)
6. Save as `fire_demon.ron`

### Example 2: Creating a Giant Spider

Starting from the quadruped template:

1. Load `quadruped.ron`
2. Remove tail mesh
3. Add 4 additional leg meshes (for 8 total legs)
4. Set color to dark gray `[0.2, 0.2, 0.2, 1.0]`
5. Scale body to `0.8` (make body smaller)
6. Scale legs to `1.2` (make legs longer)
7. Save as `giant_spider.ron`

### Example 3: Creating an Animated Golem

Starting from the robot template:

1. Load `robot.ron`
2. Change color to stone gray `[0.5, 0.5, 0.5, 1.0]`
3. Set material:
   - Roughness: `0.8` (rough stone)
   - Metallic: `0.0` (non-metallic)
4. Create "Walk" animation:
   - Keyframe 0.0s: Left leg forward
   - Keyframe 0.5s: Right leg forward
   - Keyframe 1.0s: Left leg forward (loop)
5. Generate LOD levels (3 levels)
6. Save as `stone_golem.ron`

---

## Next Steps

Congratulations! You now know how to create custom creatures. Continue learning:

- **Template Reference**: See [Creature Templates](../reference/creature_templates.md)
- **Advanced Animation**: See Phase 10 documentation (skeletal animation)
- **Material System**: See Phase 5 documentation (PBR materials)
- **Performance Optimization**: See Phase 9 documentation (LOD, instancing)

---

## See Also

- [Creature Creation Quickstart](../tutorials/creature_creation_quickstart.md)
- [Template Gallery Reference](../reference/creature_templates.md)
- [Procedural Mesh Implementation Plan](../explanation/procedural_mesh_implementation_plan.md)
