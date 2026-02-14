# TileVisualMetadata Specification

Technical reference for the `TileVisualMetadata` struct and terrain-specific fields in Antares.

## Overview

`TileVisualMetadata` defines the complete visual representation of a tile in the game world, including geometry, materials, sprites, and terrain-specific characteristics. This struct is used by both the rendering system and the Campaign Builder SDK.

**Location**: `src/domain/world/types.rs`

**Serialization Format**: RON (Rusty Object Notation)

**Usage Context**:
- Stored in `Tile.visual` field for each map tile
- Loaded from map data files (`.ron` format)
- Edited via Campaign Builder GUI
- Applied by rendering system during mesh generation

## Struct Definition

```rust
pub struct TileVisualMetadata {
    // Geometric properties
    pub height: Option<f32>,
    pub width_x: Option<f32>,
    pub width_z: Option<f32>,
    pub color_tint: Option<(u8, u8, u8)>,
    pub scale: Option<f32>,
    pub y_offset: Option<f32>,
    pub rotation_y: Option<f32>,

    // Sprite properties
    pub sprite: Option<SpriteReference>,
    pub sprite_layers: Option<Vec<LayeredSprite>>,
    pub sprite_rule: Option<SpriteSelectionRule>,

    // Terrain-specific properties
    pub grass_density: Option<GrassDensity>,
    pub tree_type: Option<TreeType>,
    pub rock_variant: Option<RockVariant>,
    pub water_flow_direction: Option<WaterFlowDirection>,
    pub foliage_density: Option<f32>,
    pub snow_coverage: Option<f32>,
}
```

## Core Geometric Properties

### height: Option<f32>

**Purpose**: Vertical displacement of the mesh above the base tile level

**Range**: No enforced limits; typical range 0.0-5.0

**Default Behavior**: If `None`, uses terrain-based defaults:
- Ground, Grass, Water, Dirt, Forest: 1.0
- Stone: 1.2
- Lava, Swamp: 0.8
- Mountain: 2.0

**Serialization**: Omitted from output if `None`

**Example**:
```ron
height: Some(2.5)
```

### width_x: Option<f32>

**Purpose**: Horizontal extent along X-axis

**Range**: No enforced limits; typical range 0.5-2.0

**Default Behavior**: If `None`, defaults to 1.0

**Serialization**: Omitted from output if `None`

**Use Case**: Making tiles wider or narrower than standard

### width_z: Option<f32>

**Purpose**: Horizontal extent along Z-axis (depth)

**Range**: No enforced limits; typical range 0.5-2.0

**Default Behavior**: If `None`, defaults to 1.0

**Serialization**: Omitted from output if `None`

**Use Case**: Creating elongated or compressed tiles

### color_tint: Option<(u8, u8, u8)>

**Purpose**: RGB color overlay applied to the tile mesh

**Format**: Tuple of three u8 values (Red, Green, Blue)

**Range**: 0-255 for each channel

**Default Behavior**: If `None`, no tinting applied (white/neutral)

**Serialization**: Omitted from output if `None`

**Example**:
```ron
color_tint: Some((200, 150, 100))  // Brownish tint
```

**Common Values**:
- `(255, 255, 255)` - White (no tinting effect)
- `(128, 128, 128)` - Gray (darkens tile)
- `(100, 150, 255)` - Blue (water-like)
- `(139, 69, 19)` - Brown (stone/earth)

### scale: Option<f32>

**Purpose**: Uniform scaling multiplier for the entire mesh

**Range**: Typical range 0.1-3.0 (negative values may invert)

**Default Behavior**: If `None`, defaults to 1.0

**Serialization**: Omitted from output if `None`

**Effect**: Multiplied with `height`, `width_x`, `width_z` in render calculations

**Example**:
```ron
scale: Some(1.5)  // 50% larger
```

### y_offset: Option<f32>

**Purpose**: Vertical offset added to Y position after height calculation

**Range**: No enforced limits; typical range -1.0 to 1.0

**Default Behavior**: If `None`, no offset applied

**Serialization**: Omitted from output if `None`

**Effect**: Fine-tuning for alignment without affecting height

### rotation_y: Option<f32>

**Purpose**: Rotation around the Y-axis (vertical)

**Range**: 0.0-360.0 degrees (or use radians via `rotation_y_radians()`)

**Default Behavior**: If `None`, defaults to 0.0 (no rotation)

**Serialization**: Omitted from output if `None`

**Example**:
```ron
rotation_y: Some(45.0)  // 45 degree rotation
```

## Sprite Properties

### sprite: Option<SpriteReference>

**Purpose**: Attach a single sprite to the tile

**Type**: `SpriteReference` struct

**Structure**:
```rust
pub struct SpriteReference {
    pub sheet_path: String,      // Path to sprite sheet (e.g., "sprites/trees.png")
    pub sprite_index: u32,       // Index in the sprite sheet
    pub animation: Option<SpriteAnimation>,
    pub material_properties: Option<SpriteMaterialProperties>,
}
```

**Serialization**: Omitted if `None`

### sprite_layers: Option<Vec<LayeredSprite>>

**Purpose**: Multiple layered sprites for complex visuals

**Type**: Vector of `LayeredSprite` structs

**Structure**:
```rust
pub struct LayeredSprite {
    pub sprite: SpriteReference,
    pub layer: SpriteLayer,      // Background, Midground, Foreground
    pub offset_y: Option<f32>,   // Y-offset for this layer
}
```

**Use Cases**:
- Layered trees (trunk + foliage)
- Composite structures (walls + decorations)
- Depth-ordered visuals

### sprite_rule: Option<SpriteSelectionRule>

**Purpose**: Dynamic sprite selection based on conditions

**Type**: `SpriteSelectionRule` enum

**Variants**:
```rust
pub enum SpriteSelectionRule {
    Fixed {
        sheet_path: String,
        sprite_index: u32,
    },
    Random {
        sheet_path: String,
        sprite_indices: Vec<u32>,
        seed: u64,
    },
    Autotile {
        sheet_path: String,
        rules: Vec<AutotileRule>,
    },
}
```

**Use Cases**:
- Random grass variants for natural look
- Autotiling for seamless transitions
- Procedural terrain generation

## Terrain-Specific Fields

These fields customize the appearance of terrain types without changing the base terrain type.

### grass_density: Option<GrassDensity>

**Purpose**: Control grass rendering density on grassland tiles

**Type**: `GrassDensity` enum

**Variants**:
```rust
pub enum GrassDensity {
    None,      // No grass
    Low,       // Sparse grass
    Medium,    // Standard grass (default)
    High,      // Dense grass
    VeryHigh,  // Very dense grass
}
```

**Default**: `Medium` (when accessor is called on `None`)

**Serialization**: Omitted if `None`

**Example**:
```ron
grass_density: Some(High)
```

**Rendering Impact**:
- Affects grass mesh density in shader
- Higher values increase polygon count
- Visual quality vs. performance tradeoff

### tree_type: Option<TreeType>

**Purpose**: Specify tree species for forest tiles

**Type**: `TreeType` enum

**Variants**:
```rust
pub enum TreeType {
    Oak,     // Broadleaf, deciduous
    Pine,    // Conifer, evergreen
    Dead,    // Bare, leafless
    Palm,    // Tropical, fronds
    Willow,  // Drooping, near-water
}
```

**Default**: `Oak` (when accessor is called on `None`)

**Serialization**: Omitted if `None`

**Example**:
```ron
tree_type: Some(Pine)
```

**Visual Characteristics**:
- **Oak**: Wide canopy, sturdy trunk
- **Pine**: Tall, conical shape
- **Dead**: Skeletal, branch-like
- **Palm**: Tropical fronds, thin trunk
- **Willow**: Drooping branches, graceful

### rock_variant: Option<RockVariant>

**Purpose**: Differentiate rock formations on mountain/stone tiles

**Type**: `RockVariant` enum

**Variants**:
```rust
pub enum RockVariant {
    Smooth,   // Weathered, rounded edges
    Jagged,   // Sharp, broken formations
    Layered,  // Stratified, geological
    Crystal,  // Geometric, magical
}
```

**Default**: `Smooth` (when accessor is called on `None`)

**Serialization**: Omitted if `None`

**Example**:
```ron
rock_variant: Some(Crystal)
```

**Use Cases**:
- **Smooth**: Natural, eroded terrain
- **Jagged**: Combat arenas, hazardous areas
- **Layered**: Mines, underground regions
- **Crystal**: Magical locations, caves

### water_flow_direction: Option<WaterFlowDirection>

**Purpose**: Animate water with directional flow

**Type**: `WaterFlowDirection` enum

**Variants**:
```rust
pub enum WaterFlowDirection {
    Still,   // Stationary water
    North,   // Flow toward negative Z
    South,   // Flow toward positive Z
    East,    // Flow toward positive X
    West,    // Flow toward negative X
}
```

**Default**: `Still` (when accessor is called on `None`)

**Serialization**: Omitted if `None`

**Example**:
```ron
water_flow_direction: Some(East)
```

**Rendering Impact**:
- Affects UV animation in water shader
- Directions follow standard compass conventions
- Multiple tiles can form rivers/currents

### foliage_density: Option<f32>

**Purpose**: Scalar multiplier for vegetation density

**Type**: `f32`

**Range**: 0.0 (none) to 2.0+ (very dense)

**Default**: `1.0` (when accessor is called on `None`)

**Serialization**: Omitted if `None`

**Example**:
```ron
foliage_density: Some(1.5)
```

**Interaction**:
- Multiplies base foliage amount
- Works with grass_density and tree_type
- Affects polygon/draw call count

### snow_coverage: Option<f32>

**Purpose**: Partial or complete snow coverage on tiles

**Type**: `f32`

**Range**: 0.0 (no snow) to 1.0 (fully covered)

**Default**: `0.0` (when accessor is called on `None`)

**Serialization**: Omitted if `None`

**Example**:
```ron
snow_coverage: Some(0.8)
```

**Rendering**:
- Interpolates between base texture and snow texture
- Value of 0.5 = 50% snow blend
- High values may reduce performance

## Accessor Methods

These methods provide typed access with defaults:

```rust
impl TileVisualMetadata {
    pub fn grass_density(&self) -> GrassDensity {
        self.grass_density.unwrap_or_default()
    }

    pub fn tree_type(&self) -> TreeType {
        self.tree_type.unwrap_or_default()
    }

    pub fn rock_variant(&self) -> RockVariant {
        self.rock_variant.unwrap_or_default()
    }

    pub fn water_flow_direction(&self) -> WaterFlowDirection {
        self.water_flow_direction.unwrap_or_default()
    }

    pub fn foliage_density(&self) -> f32 {
        self.foliage_density.unwrap_or(1.0)
    }

    pub fn snow_coverage(&self) -> f32 {
        self.snow_coverage.unwrap_or(0.0)
    }

    pub fn has_terrain_overrides(&self) -> bool {
        self.grass_density.is_some()
            || self.tree_type.is_some()
            || self.rock_variant.is_some()
            || self.water_flow_direction.is_some()
            || self.foliage_density.is_some()
            || self.snow_coverage.is_some()
    }
}
```

### has_terrain_overrides() -> bool

**Purpose**: Check if any terrain fields are set

**Returns**: `true` if any terrain-specific field contains `Some(_)`

**Use Case**: Determining whether to use special rendering paths

## Serialization Behavior

### skip_serializing_if Attributes

All optional fields use:
```rust
#[serde(skip_serializing_if = "Option::is_none")]
```

**Effect**:
- `None` values are omitted from serialized output
- Produces minimal RON with only set values
- Reduces file size and improves readability

**Example - Minimal Metadata**:
```ron
visual: (
    height: Some(2.0),
    grass_density: Some(High),
)
```

**Example - Maximal Metadata**:
```ron
visual: (
    height: Some(3.5),
    width_x: Some(1.2),
    width_z: Some(1.0),
    color_tint: Some((180, 140, 80)),
    scale: Some(1.5),
    y_offset: Some(0.2),
    rotation_y: Some(45.0),
    sprite: Some((
        sheet_path: "sprites/trees.png",
        sprite_index: 5,
    )),
    grass_density: Some(High),
    tree_type: Some(Pine),
    rock_variant: Some(Jagged),
    water_flow_direction: Some(East),
    foliage_density: Some(1.8),
    snow_coverage: Some(0.6),
)
```

## Usage in Map Files

Complete tile definition in a map RON file:

```ron
tiles: [
    // Grassland with high density
    (
        x: 0,
        y: 0,
        terrain: Grass,
        wall_type: None,
        blocked: false,
        visual: (
            grass_density: Some(VeryHigh),
            foliage_density: Some(1.5),
            color_tint: Some((200, 220, 150)),
        ),
    ),
    // Forest with pine trees and snow
    (
        x: 1,
        y: 0,
        terrain: Forest,
        wall_type: None,
        blocked: false,
        visual: (
            tree_type: Some(Pine),
            snow_coverage: Some(0.6),
            foliage_density: Some(1.2),
        ),
    ),
    // Mountain with crystal rocks
    (
        x: 2,
        y: 0,
        terrain: Mountain,
        wall_type: None,
        blocked: false,
        visual: (
            height: Some(4.0),
            rock_variant: Some(Crystal),
            color_tint: Some((100, 150, 200)),
            snow_coverage: Some(0.8),
        ),
    ),
    // River flowing east
    (
        x: 3,
        y: 0,
        terrain: Water,
        wall_type: None,
        blocked: false,
        visual: (
            water_flow_direction: Some(East),
            height: Some(0.5),
        ),
    ),
]
```

## Validation Rules

The rendering system enforces these constraints:

### Height Validation
- Negative heights are clamped to 0.0
- Very large heights (>100.0) may cause rendering artifacts

### Width Validation
- Negative widths are treated as 1.0
- Zero widths use default (1.0)

### Color Validation
- RGB values are u8 (0-255 enforced at type level)
- All combinations are valid

### Scale Validation
- Scale of 0.0 collapses geometry (not recommended)
- Negative scales may invert normals

### Density Validation
- Foliage density is not clamped (values >2.0 allowed but may impact perf)
- Snow coverage should be 0.0-1.0 (higher values blend fully)

### Rotation Validation
- No validation; any f32 value accepted
- Rendered modulo 360.0 for visual interpretation

## Implementation Notes

### Backward Compatibility

The struct uses `#[serde(default)]` on the impl, allowing:
- Old maps without terrain fields to load correctly
- Gradual migration of existing maps
- Adding new fields without breaking old data

### Memory Layout

Total struct size: ~200 bytes (varies by platform)
- Option<f32>: 8 bytes each
- Option<(u8, u8, u8)>: 4 bytes
- Option enums: 8-16 bytes depending on variant

### Performance Considerations

- Terrain fields have minimal CPU cost (checked once at load)
- GPU cost depends on rendering complexity (shader complexity)
- Snow coverage and foliage density are most expensive visually

## Enum Default Implementations

```rust
impl Default for GrassDensity {
    fn default() -> Self {
        GrassDensity::Medium
    }
}

impl Default for TreeType {
    fn default() -> Self {
        TreeType::Oak
    }
}

impl Default for RockVariant {
    fn default() -> Self {
        RockVariant::Smooth
    }
}

impl Default for WaterFlowDirection {
    fn default() -> Self {
        WaterFlowDirection::Still
    }
}
```

## Design Rationale

### Why Optional Fields?

Making terrain fields optional allows:
- Minimal serialized output (only customized values stored)
- Backward compatibility with older map formats
- Clear distinction between "not set" and "set to default"
- Efficient storage for large maps

### Why Separate Accessors?

The accessor methods (e.g., `grass_density()`) provide:
- Guaranteed non-null returns for game logic
- Consistent defaults across the codebase
- Type-safe defaults (no magic strings)

### Why Enums for Terrain?

Using enums instead of strings provides:
- Compile-time checking of valid values
- Efficient serialization (variants, not strings)
- IDE autocomplete and refactoring support
- Prevention of typos in map files

## Related Structures

### SpriteReference

Attached to `sprite` field; defines single sprite attachment.

### SpriteAnimation

Attached to sprites; defines frame-based animation.

### SpriteMaterialProperties

Attached to sprites; defines shader parameters (emissive, alpha, metallic, roughness).

### LayeredSprite

Combines SpriteReference with layer ordering.

## See Also

- [Tile Structure Reference](./tile_structure_reference.md)
- [Map Format Specification](./map_format_specification.md)
- [Terrain Types Reference](./terrain_types_reference.md)
- [Campaign Builder Guide](../how-to/use_terrain_specific_controls.md)
