#!/usr/bin/env python3
"""
Clean up map RON files by removing terrain-specific fields that don't apply to each tile's terrain type.

This script fixes the issue where the map editor was adding irrelevant fields like:
- tree_type on Grass tiles (should only be on Forest)
- rock_variant on Grass tiles (should only be on Mountain)
- water_flow_direction on Grass tiles (should only be on Water/Swamp)
- foliage_density on Grass tiles (should only be on Forest)
- etc.

It preserves fields that ARE relevant to each terrain type:
- Grass: grass_density only
- Forest: tree_type, foliage_density, snow_coverage
- Mountain: rock_variant, snow_coverage
- Water/Swamp: water_flow_direction
"""

import re
import sys
from pathlib import Path


def get_relevant_fields(terrain_type):
    """Return the set of fields that are relevant for a given terrain type."""
    field_map = {
        'Grass': {'grass_density'},
        'Forest': {'tree_type', 'foliage_density', 'snow_coverage'},
        'Mountain': {'rock_variant', 'snow_coverage'},
        'Water': {'water_flow_direction'},
        'Swamp': {'water_flow_direction'},
        'Ground': set(),
        'Stone': set(),
        'Dirt': set(),
        'Lava': set(),
    }
    return field_map.get(terrain_type, set())


def clean_tile_metadata(tile_text, terrain_type):
    """Remove irrelevant terrain-specific fields from a tile's visual metadata."""
    relevant_fields = get_relevant_fields(terrain_type)

    # All terrain-specific fields that might appear
    all_terrain_fields = {
        'grass_density', 'tree_type', 'rock_variant',
        'water_flow_direction', 'foliage_density', 'snow_coverage'
    }

    # Fields to remove are those not relevant to this terrain type
    fields_to_remove = all_terrain_fields - relevant_fields

    # Remove each irrelevant field
    for field in fields_to_remove:
        # Match field with any value (handles Some(Value) format)
        # Pattern: field_name: Some(value),
        pattern = rf'^\s*{field}:.*,\s*$'
        tile_text = re.sub(pattern, '', tile_text, flags=re.MULTILINE)

    return tile_text


def process_map_file(file_path):
    """Process a map RON file to clean up terrain-specific metadata."""
    print(f"Processing {file_path}...")

    with open(file_path, 'r') as f:
        content = f.read()

    # Find all tile definitions
    # Pattern: (terrain: TerrainType, ... visual: (...), )
    tile_pattern = r'\(\s*terrain:\s*(\w+),.*?visual:\s*\((.*?)\),\s*\)'

    def clean_tile(match):
        terrain_type = match.group(1)
        full_tile = match.group(0)
        visual_section = match.group(2)

        # Clean the visual section
        cleaned_visual = clean_tile_metadata(visual_section, terrain_type)

        # Replace the visual section in the full tile
        cleaned_tile = full_tile.replace(visual_section, cleaned_visual)

        return cleaned_tile

    # Process all tiles
    cleaned_content = re.sub(tile_pattern, clean_tile, content, flags=re.DOTALL)

    # Write back
    backup_path = f"{file_path}.bak"
    print(f"Creating backup at {backup_path}")
    with open(backup_path, 'w') as f:
        f.write(content)

    with open(file_path, 'w') as f:
        f.write(cleaned_content)

    print(f"✓ Cleaned {file_path}")


def main():
    # Process map_1.ron
    maps_dir = Path(__file__).parent.parent / "campaigns" / "tutorial" / "data" / "maps"
    map_files = list(maps_dir.glob("map_*.ron"))

    if not map_files:
        print(f"No map files found in {maps_dir}")
        return 1

    for map_file in sorted(map_files):
        try:
            process_map_file(map_file)
        except Exception as e:
            print(f"Error processing {map_file}: {e}")
            return 1

    print(f"\n✓ Successfully cleaned {len(map_files)} map file(s)!")
    print("Backups saved with .bak extension")
    return 0


if __name__ == '__main__':
    sys.exit(main())
