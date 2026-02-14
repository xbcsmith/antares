# How to Use Terrain-Specific Controls in Map Editor

This guide explains how to use the terrain-specific visual controls in the Campaign Builder's map editor to customize grass density, tree types, rock variants, water flow, and more.

## Overview

The map editor provides context-sensitive controls based on the terrain type of the selected tile. This allows you to fine-tune the visual appearance of different terrain types to create rich, varied landscapes.

## Prerequisites

- Campaign Builder installed and running
- A campaign loaded in the Campaign Builder
- A map open in the map editor
- Basic familiarity with tile selection and the inspector panel

## Accessing Terrain Controls

1. **Select a tile**: Click on any tile in the map grid
2. **Open inspector panel**: The right sidebar displays tile properties
3. **Locate terrain controls**: Scroll to the "Terrain-Specific Settings" section

The specific controls shown depend on the tile's terrain type.

## Terrain-Specific Controls by Type

### Grassland and Plains Tiles

**Available Controls**:

- **Grass Density**: Dropdown selector (None, Low, Medium, High, VeryHigh)
- **Foliage Density**: Slider from 0.0 to 2.0 (controls visual density multiplier)

**Use Cases**:

- Create natural transitions from sparse grass to dense meadows
- Build varied grassland regions with different visual intensity
- Design grazing areas with controlled vegetation density

**Example Workflow**:

1. Select a grassland tile in your map
2. Choose "Grass Density" → "VeryHigh" from the dropdown
3. Adjust "Foliage Density" slider to 1.5 for a lush appearance
4. Observe the tile update with denser grass rendering in the viewport
5. Apply the same settings to adjacent tiles for consistency

### Forest Tiles

**Available Controls**:

- **Tree Type**: Dropdown selector (Oak, Pine, Dead, Palm, Willow)
- **Foliage Density**: Slider from 0.0 to 2.0
- **Snow Coverage**: Slider from 0.0 to 1.0 (for snowy forests)

**Use Cases**:

- Create biome-specific forests (temperate, boreal, tropical)
- Add seasonal variations (snowy pine forests, dead trees)
- Design diverse tree coverage with multiple species

**Example Workflow - Snowy Pine Forest**:

1. Select a forest tile
2. Choose "Tree Type" → "Pine" from the dropdown
3. Adjust "Snow Coverage" slider to 0.8 (heavy snow coverage)
4. Set "Foliage Density" to 1.2 for visibility through snow
5. Result: A winter forest with pine trees laden with snow

**Example Workflow - Dead Forest**:

1. Select multiple forest tiles
2. Set "Tree Type" → "Dead" for all tiles
3. Reduce "Foliage Density" to 0.5 for sparse, gnarled appearance
4. Apply a dark tint color via the color tinting control
5. Result: An ominous, dead forest region

### Mountain and Hill Tiles

**Available Controls**:

- **Rock Variant**: Dropdown selector (Smooth, Jagged, Layered, Crystal)
- **Snow Coverage**: Slider from 0.0 to 1.0

**Use Cases**:

- Differentiate rock formations visually (sharp vs. weathered)
- Create crystal cave formations or geode areas
- Apply realistic snow coverage to high-altitude peaks

**Example Workflow - Crystal Caverns**:

1. Select a mountain tile
2. Choose "Rock Variant" → "Crystal"
3. Apply a blue or cyan tint color using the color control
4. Set "Snow Coverage" to 0.0 (deep cave, no surface snow)
5. Result: Crystalline formations with magical blue highlights

**Example Workflow - Snow-Capped Peak**:

1. Select a mountain tile representing the peak
2. Choose "Rock Variant" → "Smooth" or "Layered"
3. Set "Snow Coverage" to 0.9 (heavy snow coverage)
4. Use the height override to make the peak stand out
5. Result: A snow-covered mountain peak

### Water and Swamp Tiles

**Available Controls**:

- **Water Flow Direction**: Dropdown selector (Still, North, South, East, West)

**Use Cases**:

- Create flowing rivers and streams with directional flow
- Design waterfalls and cascading water features
- Construct swamp regions with still water

**Example Workflow - Flowing River**:

1. Select the first water tile in your river path
2. Set "Water Flow Direction" → "East"
3. Select the next tile to the east
4. Set "Water Flow Direction" → "East" (continue flow)
5. When river bends south, select the next tile
6. Set "Water Flow Direction" → "South"
7. Result: An animated river flowing east then south

**Example Workflow - Still Swamp**:

1. Select a swamp/water tile
2. Set "Water Flow Direction" → "Still"
3. Apply a green or murky tint via the color control
4. Increase foliage density for vegetation
5. Result: A stagnant swamp area

### Desert and Snow Tiles

**Available Controls**:

- **Snow Coverage**: Slider from 0.0 to 1.0 (partial or full coverage)

**Use Cases**:

- Add realistic snow dusting to desert peaks
- Create blizzard conditions with high coverage
- Design transition zones between biomes

**Example Workflow - Desert Peak with Snow**:

1. Select a desert terrain tile
2. Adjust "Snow Coverage" to 0.3 for light dusting
3. Apply a light brown/tan tint for desert sand
4. Result: Desert terrain with snowy highlights

## Using Visual Presets

Visual presets provide quick, pre-configured settings for common scenarios.

**How to Use Presets**:

1. Select a tile in the map
2. Locate the "Visual Presets" section in the inspector panel
3. Click category tabs to filter presets:
   - **All**: All available presets
   - **Walls**: Wall-specific configurations
   - **Nature**: Trees, grass, natural features
   - **Water**: Water and liquid features
   - **Structures**: Buildings and architectural elements
4. Click a preset button to apply it to the selected tile

**Example Presets**:

- **SmallTree**: Height=2.0, Scale=0.5, Green tint
- **TallWall**: Height=3.5, appropriate depth
- **ShallowWater**: Reduced height for shallow water appearance
- **HighMountain**: Height=5.0, gray tint, rough terrain

**Tips for Preset Usage**:

- Presets apply visual settings regardless of terrain type
- Combine presets with terrain-specific controls for best results
- Use presets as starting points, then fine-tune with sliders

## Clearing Terrain Properties

To reset terrain-specific settings to their default values:

1. Select the tile you want to reset
2. Scroll to the "Terrain-Specific Settings" section
3. Click the **"Clear Terrain Properties"** button
4. All terrain-specific fields reset to `None` (defaults apply)

**When to Clear Properties**:

- After experimenting with settings
- To restore a tile to its base terrain appearance
- When you want to start fresh with a tile

## Best Practices

### Natural Transitions

- **Use gradients**: Vary grass density across adjacent tiles for smooth transitions
- **Mix vegetation**: Combine different tree types within forest regions
- **Blend terrain**: Use intermediate rock variants for variety

### Biome Consistency

- **Match terrain context**: Use Palm trees only in desert/tropical maps
- **Respect altitude**: Higher mountains should have higher snow coverage
- **Consider climate**: Snowy regions should use appropriate tree types (Pine, Dead)

### Water Design

- **Flow consistency**: Ensure water flow directions form logical paths
- **Cascade direction**: Use South flow for waterfalls
- **Matching edges**: Adjacent water tiles should have compatible flow directions

### Performance

- **Avoid extreme density**: Very high foliage density may impact performance
- **Use selectively**: Apply snow coverage only where needed
- **Test rendering**: Check viewport performance with complex terrain setups

## Troubleshooting

### Q: The terrain controls don't appear for my selected tile

**A**:

- Verify a tile is actually selected (should be highlighted in the grid)
- Check that the terrain type matches the expected controls
- Example: Forest tiles show tree_type controls, but grassland tiles don't

### Q: My changes don't apply to the tile

**A**:

- Ensure you've selected the correct tile (check coordinates in inspector)
- Verify the map is not read-only
- Check that you clicked "Apply" or saved if required

### Q: A preset doesn't look right for my terrain type

**A**:

- Presets apply their configured values to any terrain type
- This may not be ideal for all terrain combinations
- Manually adjust terrain-specific controls after applying a preset
- Consider using presets as starting points only

### Q: How do I find the "Terrain-Specific Settings" section?

**A**:

- In the inspector panel on the right, scroll down past basic tile properties
- Look for a collapsible section labeled "Terrain-Specific Settings"
- If not visible, ensure a tile is selected in the map

### Q: Can I apply settings to multiple tiles at once?

**A**:

- **Yes!** You can select multiple tiles using **Shift+Click** or the **Select Tool**
- The inspector panel will show "N tiles selected"
- Changes made to controls will queue up for all selected tiles
- Click **"Apply"** to update all selected tiles at once

### Q: Why do the controls reset after I click Apply?

**A**:

- This is intentional behavior to prevent accidental edits
- After successfully applying changes to the map, the editor controls reset to defaults
- Your selected tiles remain selected so you can verify the changes
- To make further edits, simply adjust the controls again and click Apply

## See Also

- [Map Editor Reference](../reference/map_editor_reference.md)
- [TileVisualMetadata Specification](../reference/tile_visual_metadata_specification.md)
- Tutorial map examples: `campaigns/tutorial/data/maps/`
