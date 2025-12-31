# Tile Visual Metadata Guide

## Overview

The Tile Visual Metadata system allows map authors to customize the visual appearance of individual tiles beyond their terrain and wall types. This enables rich, varied environments with architectural detail, terrain variety, and visual storytelling without requiring separate asset files.

## Purpose and Use Cases

### Architectural Variety
- Create castle walls (height=3.0) vs garden walls (height=1.0)
- Vary wall thickness for defensive structures vs decorative borders
- Build multi-level dungeons with raised platforms and sunken pits

### Terrain Diversity
- Differentiate small hills (height=2.0) from towering peaks (height=5.0)
- Create forests with saplings (scale=0.5) and ancient trees (scale=2.0)
- Add visual interest to otherwise uniform terrain types

### Material Representation
- Color-tint walls to represent different materials (sandstone, granite, marble)
- Distinguish water depth with color variations
- Create lava flows with bright orange/red tints

### Environmental Storytelling
- Use sunken terrain (y_offset=-0.5) for craters or excavation sites
- Raised platforms (y_offset=+0.5) for altars or monuments
- Scaled objects for emphasis or perspective

## Visual Metadata Fields

All fields are optional (`Option<T>`) and use `None` to indicate default behavior based on terrain/wall type.

### `height: Option<f32>`

**Description:** Vertical dimension (Y-axis) of the tile's visual representation in world units (1 unit ≈ 10 feet).

**Default Values (when `None`):**
- Normal walls: 2.5 units (25 feet)
- Doors: 2.5 units
- Mountains: 3.0 units (30 feet)
- Trees (Forest): 2.2 units (22 feet)
- Torches: 2.5 units
- Other terrain: 0.1 units (ground level)

**Typical Range:** 0.1 to 10.0 units

**Examples:**
```ron
height: Some(1.0),  // Short garden wall (10 feet)
height: Some(3.5),  // Tall castle fortification (35 feet)
height: Some(5.0),  // Towering mountain peak (50 feet)
height: None,       // Use default for terrain/wall type
```

**Use Cases:**
- Varying wall heights for architectural interest
- Creating hills vs mountains
- Emphasizing important structures

### `width_x: Option<f32>`

**Description:** Horizontal dimension along the X-axis in world units.

**Default Value (when `None`):** 1.0 (full tile width)

**Typical Range:** 0.1 to 1.0 units

**Examples:**
```ron
width_x: Some(0.5),  // Half-width pillar
width_x: Some(0.2),  // Thin barrier
width_x: None,       // Full tile width (default)
```

**Use Cases:**
- Creating pillars or columns
- Narrow barriers or railings
- Non-blocking visual elements

### `width_z: Option<f32>`

**Description:** Horizontal dimension along the Z-axis (depth) in world units.

**Default Value (when `None`):** 1.0 (full tile depth)

**Typical Range:** 0.1 to 1.0 units

**Examples:**
```ron
width_z: Some(0.3),  // Thin decorative border
width_z: Some(0.8),  // Most of tile depth
width_z: None,       // Full tile depth (default)
```

**Use Cases:**
- Thin walls or fences
- Partial obstructions
- Decorative borders

### `color_tint: Option<(f32, f32, f32)>`

**Description:** RGB color multiplier applied to the base material color. Each component ranges from 0.0 (black) to 1.0 (full color).

**Default Value (when `None`):** No tinting (use base material color)

**Application:** Multiplicative - each color component is multiplied by the tint.
- `base_color.r * tint.0`
- `base_color.g * tint.1`
- `base_color.b * tint.2`

**Examples:**
```ron
color_tint: Some((0.9, 0.7, 0.4)),    // Warm sandstone
color_tint: Some((0.3, 0.3, 0.35)),   // Dark granite
color_tint: Some((0.95, 0.95, 0.98)), // White marble
color_tint: Some((0.8, 0.5, 0.2)),    // Copper/bronze
color_tint: Some((0.2, 0.8, 0.3)),    // Vibrant green foliage
color_tint: None,                     // Use base material color
```

**Common Tint Recipes:**

| Material      | RGB Tint           | Description                    |
|---------------|--------------------|--------------------------------|
| Sandstone     | (0.9, 0.7, 0.4)    | Warm desert stone              |
| Granite       | (0.3, 0.3, 0.35)   | Dark igneous rock              |
| Marble        | (0.95, 0.95, 0.98) | White polished stone           |
| Obsidian      | (0.1, 0.1, 0.15)   | Black volcanic glass           |
| Copper        | (0.8, 0.5, 0.2)    | Oxidized metal                 |
| Wood (oak)    | (0.6, 0.4, 0.2)    | Brown hardwood                 |
| Wood (pine)   | (0.7, 0.6, 0.4)    | Light softwood                 |
| Grass (lush)  | (0.3, 0.7, 0.2)    | Vibrant green vegetation       |
| Grass (dry)   | (0.6, 0.6, 0.3)    | Yellowed grass                 |
| Ice           | (0.8, 0.9, 1.0)    | Frozen water with blue tint    |
| Lava (dim)    | (0.8, 0.3, 0.1)    | Cooled magma                   |
| Lava (bright) | (1.0, 0.5, 0.1)    | Molten rock                    |

**Use Cases:**
- Representing different building materials
- Creating visual variety in forests (light/dark foliage)
- Distinguishing water depth or lava temperature
- Thematic area differentiation (fire dungeon, ice cave)

### `scale: Option<f32>`

**Description:** Uniform scale multiplier applied to all dimensions (width_x, height, width_z).

**Default Value (when `None`):** 1.0 (no scaling)

**Typical Range:** 0.1 to 3.0

**Application:** Applied **after** dimension calculations:
- `final_width_x = (width_x ?? 1.0) * scale`
- `final_height = effective_height * scale`
- `final_width_z = (width_z ?? 1.0) * scale`

**Examples:**
```ron
scale: Some(0.5),  // Half-size sapling or small boulder
scale: Some(1.5),  // Slightly larger feature
scale: Some(2.0),  // Double-size ancient tree or monument
scale: None,       // Normal size (default)
```

**Use Cases:**
- Creating forests with size variety (saplings to ancient trees)
- Emphasizing important objects (large altar, towering statue)
- Reducing scale for rubble or debris
- Progressive terrain features (small to large mountains)

### `y_offset: Option<f32>`

**Description:** Vertical offset from ground level in world units. Positive values raise the feature, negative values sink it.

**Default Value (when `None`):** 0.0 (ground level)

**Typical Range:** -2.0 to 2.0 units

**Application:** Offsets the center point of the mesh along the Y-axis.

**Examples:**
```ron
y_offset: Some(-0.5),  // Sunken pit or crater
y_offset: Some(0.0),   // Ground level (explicit default)
y_offset: Some(0.5),   // Raised platform or pedestal
y_offset: Some(1.0),   // Elevated structure
y_offset: None,        // Ground level (default)
```

**Use Cases:**
- Creating multi-level dungeons (raised walkways, sunken chambers)
- Environmental storytelling (craters, excavations, altars)
- Vertical terrain variation without changing tile height
- Floating platforms or suspended objects

## RON Syntax Examples

### Minimal Example (Use Defaults)
```ron
(
    terrain: Ground,
    wall_type: Normal,
    blocked: true,
    is_special: false,
    is_dark: false,
    visited: false,
    x: 5,
    y: 10,
    event_trigger: None,
    // visual field omitted - all defaults apply
)
```

### Partial Customization
```ron
(
    terrain: Ground,
    wall_type: Normal,
    blocked: false,
    is_special: false,
    is_dark: false,
    visited: false,
    x: 5,
    y: 10,
    event_trigger: None,
    visual: (
        height: Some(1.5),          // Custom height
        width_x: None,              // Default width
        width_z: Some(0.2),         // Thin wall
        color_tint: None,           // Default color
        scale: None,                // Default scale
        y_offset: None,             // Ground level
    ),
)
```

### Full Customization
```ron
(
    terrain: Mountain,
    wall_type: None,
    blocked: true,
    is_special: true,
    is_dark: false,
    visited: false,
    x: 12,
    y: 8,
    event_trigger: None,
    visual: (
        height: Some(6.0),                   // Towering peak
        width_x: Some(1.0),                  // Full width
        width_z: Some(1.0),                  // Full depth
        color_tint: Some((0.3, 0.25, 0.3)),  // Purple-grey stone
        scale: Some(1.2),                    // Slightly larger
        y_offset: Some(0.0),                 // Ground level
    ),
)
```

## Common Scenarios

### Scenario 1: Castle Fortifications

**Goal:** Create imposing castle walls that are taller and thicker than normal walls.

```ron
// Outer wall - tall and grey
(
    terrain: Ground,
    wall_type: Normal,
    blocked: true,
    is_special: false,
    is_dark: false,
    visited: false,
    x: 10,
    y: 5,
    event_trigger: None,
    visual: (
        height: Some(3.5),
        width_x: Some(1.0),
        width_z: Some(1.0),
        color_tint: Some((0.5, 0.5, 0.6)),
        scale: None,
        y_offset: None,
    ),
)
```

### Scenario 2: Forest Variety

**Goal:** Create a forest with small bushes, normal trees, and ancient giants.

```ron
// Small bush
(
    terrain: Forest,
    wall_type: None,
    blocked: false,
    is_special: false,
    is_dark: false,
    visited: false,
    x: 3,
    y: 7,
    event_trigger: None,
    visual: (
        height: None,
        width_x: None,
        width_z: None,
        color_tint: Some((0.4, 0.8, 0.3)),
        scale: Some(0.5),
        y_offset: None,
    ),
)

// Ancient tree
(
    terrain: Forest,
    wall_type: None,
    blocked: false,
    is_special: false,
    is_dark: false,
    visited: false,
    x: 5,
    y: 7,
    event_trigger: None,
    visual: (
        height: None,
        width_x: None,
        width_z: None,
        color_tint: Some((0.2, 0.5, 0.15)),
        scale: Some(2.0),
        y_offset: None,
    ),
)
```

### Scenario 3: Multi-Level Dungeon

**Goal:** Create a chamber with a sunken pit and a raised altar.

```ron
// Sunken pit
(
    terrain: Ground,
    wall_type: None,
    blocked: false,
    is_special: false,
    is_dark: true,
    visited: false,
    x: 8,
    y: 8,
    event_trigger: None,
    visual: (
        height: Some(0.5),
        width_x: None,
        width_z: None,
        color_tint: Some((0.3, 0.25, 0.2)),
        scale: None,
        y_offset: Some(-1.0),
    ),
)

// Raised altar
(
    terrain: Ground,
    wall_type: None,
    blocked: false,
    is_special: true,
    is_dark: false,
    visited: false,
    x: 12,
    y: 8,
    event_trigger: None,
    visual: (
        height: Some(1.0),
        width_x: Some(0.8),
        width_z: Some(0.8),
        color_tint: Some((0.9, 0.9, 0.95)),
        scale: None,
        y_offset: Some(0.5),
    ),
)
```

### Scenario 4: Material-Themed Dungeon

**Goal:** Create a copper mine with oxidized metal walls.

```ron
// Copper ore wall
(
    terrain: Ground,
    wall_type: Normal,
    blocked: true,
    is_special: false,
    is_dark: false,
    visited: false,
    x: 15,
    y: 10,
    event_trigger: None,
    visual: (
        height: Some(2.5),
        width_x: None,
        width_z: None,
        color_tint: Some((0.8, 0.5, 0.2)),
        scale: None,
        y_offset: None,
    ),
)
```

### Scenario 5: Mountain Range with Progressive Heights

**Goal:** Create a realistic mountain range with increasing elevation.

```ron
// Foothills
(
    terrain: Mountain,
    wall_type: None,
    blocked: true,
    is_special: false,
    is_dark: false,
    visited: false,
    x: 20,
    y: 5,
    event_trigger: None,
    visual: (
        height: Some(2.0),
        width_x: None,
        width_z: None,
        color_tint: Some((0.5, 0.45, 0.4)),
        scale: None,
        y_offset: None,
    ),
)

// Mid-range peak
(
    terrain: Mountain,
    wall_type: None,
    blocked: true,
    is_special: false,
    is_dark: false,
    visited: false,
    x: 21,
    y: 5,
    event_trigger: None,
    visual: (
        height: Some(4.0),
        width_x: None,
        width_z: None,
        color_tint: Some((0.4, 0.35, 0.3)),
        scale: None,
        y_offset: None,
    ),
)

// Highest summit
(
    terrain: Mountain,
    wall_type: None,
    blocked: true,
    is_special: true,
    is_dark: false,
    visited: false,
    x: 22,
    y: 5,
    event_trigger: None,
    visual: (
        height: Some(6.0),
        width_x: None,
        width_z: None,
        color_tint: Some((0.9, 0.9, 0.95)),  // Snow-capped
        scale: None,
        y_offset: None,
    ),
)
```

## Performance Considerations

### Mesh Caching

The rendering system implements mesh caching to minimize GPU overhead:

- Meshes are cached by their dimensions `(width_x, height, width_z)`
- Tiles with identical dimensions share the same mesh
- Cache is local to each map spawn (cleared when map unloads)

**Best Practice:** Reuse dimension combinations when possible to maximize cache hits.

**Example:**
```ron
// These two tiles share the same mesh (good)
visual: (height: Some(2.5), width_x: None, width_z: None, ...)
visual: (height: Some(2.5), width_x: None, width_z: None, ...)

// These create separate meshes (acceptable but less efficient)
visual: (height: Some(2.5), width_x: None, width_z: None, ...)
visual: (height: Some(2.51), width_x: None, width_z: None, ...)
```

### Memory Usage

Each unique mesh dimension set consumes:
- Vertex buffer memory (~few KB per mesh)
- Index buffer memory (~few KB per mesh)
- GPU texture cache (shared across all instances)

**Guideline:** Maps with 100-200 unique dimension combinations are well within reasonable limits.

### Rendering Cost

Color tinting and transforms are applied per-entity and have negligible cost. The primary cost is unique mesh count.

## Map Editing Workflow

### Using Campaign Builder GUI

1. Open the map editor
2. Select a tile by clicking on the map grid
3. In the Inspector panel, expand "Visual Properties"
4. Adjust sliders for height, width, scale, offset
5. Use the color picker for tinting
6. Click "Apply" to update the tile
7. Changes are visible immediately in the map view

### Editing RON Files Directly

1. Open the map `.ron` file in a text editor
2. Locate the tile by its `x` and `y` coordinates
3. Add or modify the `visual:` field
4. Save the file
5. Reload the map in-game or in the editor to see changes

### Validation

The system validates:
- Coordinate bounds (tiles within map dimensions)
- Value ranges (though no hard limits enforced)
- RON syntax correctness

Invalid visual metadata is logged as a warning and ignored (falls back to defaults).

## Backward Compatibility

Maps created before the visual metadata system remain fully compatible:

- Tiles without a `visual` field use hardcoded defaults
- Existing maps require no modification
- Visual metadata can be added incrementally
- `#[serde(default)]` ensures deserialization succeeds with missing fields

**Migration Strategy:** Gradually add visual customization to existing maps during content updates.

## Reference Example Map

See `data/maps/visual_metadata_examples.ron` for a complete demonstration map featuring:

- **Section 1 (x: 0-4):** Castle walls vs garden walls (height variations)
- **Section 2 (x: 5-9):** Mountain range with progressive heights (2.0 to 5.0 units)
- **Section 3 (x: 10-14):** Color-tinted walls (sandstone, granite, marble, copper)
- **Section 4 (x: 15-19):** Scaled trees (small, normal, large)
- **Section 5 (x: 20-24):** Vertical offset variations (sunken, ground, raised)

Load this map to see all features in action.

## Best Practices

1. **Start with Defaults:** Only customize when needed for visual storytelling
2. **Use Color Tints Sparingly:** Subtle tints (0.7-1.0 range) often look better than extreme values
3. **Combine Fields:** Use `height` + `color_tint` + `scale` together for maximum variety
4. **Think in World Units:** 1 unit ≈ 10 feet; keep proportions realistic
5. **Test Visually:** Always preview changes in the editor or game
6. **Document Custom Tiles:** Add comments in RON files explaining unique visual choices
7. **Reuse Dimensions:** Maximize mesh cache efficiency by reusing dimension sets

## Troubleshooting

### Tile Looks Wrong

- **Check default behavior:** Ensure `None` values are intentional
- **Verify ranges:** RGB tints should be 0.0-1.0
- **Test in isolation:** Try the tile in an empty map section

### Performance Issues

- **Count unique meshes:** Use similar dimensions to reduce mesh cache size
- **Reduce visual complexity:** Not every tile needs customization

### RON Parse Errors

- **Validate syntax:** Use a RON linter or the campaign builder validator
- **Check field names:** `color_tint` not `color`, `y_offset` not `offset`
- **Verify tuple format:** `Some((0.5, 0.5, 0.5))` not `Some(0.5, 0.5, 0.5)`

## Future Enhancements

Potential future additions to the visual metadata system:

- **Rotation:** `rotation_y` for oriented features
- **Custom Meshes:** `mesh_id` to reference external mesh assets
- **Material Overrides:** `material_id` for advanced shaders
- **Animation:** `animation_id` for animated tiles
- **Emissive Lighting:** `emissive_color` and `emissive_strength`
- **Transparency:** `alpha` for semi-transparent features

These are not currently implemented but the architecture supports extensibility.

## Summary

The Tile Visual Metadata system provides powerful per-tile customization while maintaining:

- **Backward compatibility** (all fields optional)
- **Performance** (mesh caching strategy)
- **Simplicity** (RON format integration)
- **Flexibility** (combine fields for rich variety)

Use it to create visually compelling, detailed environments that enhance player immersion and storytelling.

For implementation details, see `docs/explanation/implementations.md` Phase 2.
