#!/usr/bin/env python3
"""
Simple script to remove foliage_density from non-Forest tiles.
"""

import re
import sys
from pathlib import Path


def process_map_file(file_path):
    """Remove foliage_density from non-Forest tiles."""
    print(f"Processing {file_path}...")
    
    with open(file_path, 'r') as f:
        content = f.read()
    
    # Strategy: Find all tiles, check terrain type, remove foliage_density if not Forest
    # Pattern to match a complete tile definition
    tile_pattern = r'\(\s*terrain:\s*(\w+),.*?\),\s*\)(?=\s*(?:\(|events:|]\s*,))'
    
    def clean_tile(match):
        terrain_type = match.group(1)
        full_tile = match.group(0)
        
        # Only remove foliage_density from non-Forest tiles
        if terrain_type != 'Forest':
            # Remove the foliage_density line
            cleaned = re.sub(r'^\s*foliage_density:.*,\s*$', '', full_tile, flags=re.MULTILINE)
            return cleaned
        else:
            return full_tile
    
    # Process all tiles
    cleaned_content = re.sub(tile_pattern, clean_tile, content, flags=re.DOTALL)
    
    # Write back
    backup_path = f"{file_path}.bak2"
    print(f"Creating backup at {backup_path}")
    with open(backup_path, 'w') as f:
        f.write(content)
    
    with open(file_path, 'w') as f:
        f.write(cleaned_content)
    
    print(f"✓ Cleaned {file_path}")


def main():
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
    print("Backups saved with .bak2 extension")
    return 0


if __name__ == '__main__':
    sys.exit(main())
