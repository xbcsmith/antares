# Fix: Grass Tiles Rendering Unwanted Vegetation

## Problem

When applying "Visual Properties" in the Map Editor to Grass tiles, the system was adding irrelevant terrain-specific metadata fields:
- `tree_type` (only relevant for Forest)
- `rock_variant` (only relevant for Mountain)
- `water_flow_direction` (only relevant for Water/Swamp)
- Inappropriate `foliage_density` and `snow_coverage` values

This caused the game to render trees, rocks, or other unwanted objects on what should be simple grass tiles.

## Root Cause

In `sdk/campaign_builder/src/map_editor.rs`, the "Apply" button unconditionally called `apply_terrain_state_to_selection()`, which set **all** terrain-specific fields to default values regardless of terrain type:

```rust
pub fn apply_to_metadata(&self, metadata: &mut TileVisualMetadata) {
    metadata.grass_density = Some(self.grass_density);      // ✓ Relevant for Grass
    metadata.tree_type = Some(self.tree_type);              // ✗ NOT for Grass
    metadata.rock_variant = Some(self.rock_variant);        // ✗ NOT for Grass
    metadata.water_flow_direction = Some(self.water_flow_direction); // ✗ NOT for Grass
    metadata.foliage_density = Some(self.foliage_density);
    metadata.snow_coverage = Some(self.snow_coverage);
}
```

## Solution

### 1. Created Terrain-Aware Metadata Application (Code Fix)

Added `apply_to_metadata_for_terrain()` method that only sets fields relevant to each terrain type:

```rust
pub fn apply_to_metadata_for_terrain(&self, metadata: &mut TileVisualMetadata, terrain_type: TerrainType) {
    match terrain_type {
        TerrainType::Grass => {
            metadata.grass_density = Some(self.grass_density);
            metadata.foliage_density = Some(self.foliage_density);
            // Don't set tree_type, rock_variant, water_flow_direction
        }
        TerrainType::Forest => {
            metadata.tree_type = Some(self.tree_type);
            metadata.foliage_density = Some(self.foliage_density);
            metadata.snow_coverage = Some(self.snow_coverage);
        }
        TerrainType::Mountain => {
            metadata.rock_variant = Some(self.rock_variant);
            metadata.snow_coverage = Some(self.snow_coverage);
        }
        TerrainType::Water | TerrainType::Swamp => {
            metadata.water_flow_direction = Some(self.water_flow_direction);
        }
        _ => {
            // Ground, Stone, Dirt, Lava: no terrain-specific fields
        }
    }
}
```

### 2. Updated Method Calls

Modified `apply_terrain_state_to_selection()` to:
1. Retrieve each tile's terrain type
2. Call `apply_to_metadata_for_terrain()` with the correct terrain type
3. Only set relevant fields for each tile

### 3. Cleaned Existing Map Files (Data Fix)

Created and ran `scripts/clean_map_metadata.py` to remove irrelevant fields from all existing map RON files:
- Processed 6 map files (map_1.ron through map_6.ron)
- Created backups with `.bak` extension
- Removed fields that don't match each tile's terrain type

## Field Relevance Matrix

| Terrain Type | Relevant Fields |
|--------------|----------------|
| **Grass** | `grass_density`, `foliage_density` |
| **Forest** | `tree_type`, `foliage_density`, `snow_coverage` |
| **Mountain** | `rock_variant`, `snow_coverage` |
| **Water** | `water_flow_direction` |
| **Swamp** | `water_flow_direction` |
| **Ground/Stone/Dirt/Lava** | *(none)* |

## Testing

1. ✅ Code compiles successfully (`cargo check --package campaign_builder`)
2. ✅ Cleaned map files have reduced metadata (verified with grep)
3. ✅ Game builds successfully (`cargo build --bin antares`)

## Next Steps for Users

1. **New edits**: The Map Editor will now only set relevant fields when you click "Apply"
2. **Existing maps**: Already cleaned by the script (backups available as `*.ron.bak`)
3. **Verification**: Load a map in-game and confirm grass tiles show only grass blades, no trees/rocks
4. **Re-apply if needed**: If you want to adjust grass density or height, you can now safely re-apply visual properties without adding unwanted vegetation

## Files Changed

- `sdk/campaign_builder/src/map_editor.rs`: Added terrain-aware metadata application
- `campaigns/tutorial/data/maps/map_*.ron`: Cleaned irrelevant fields
- `scripts/clean_map_metadata.py`: Utility script for future cleanup needs
