#!/usr/bin/env python3
"""
Remove foliage_density from Grass and other non-Forest tiles.
"""

import sys
from pathlib import Path


def process_map_file(file_path):
    """Remove foliage_density from non-Forest tiles."""
    print(f"Processing {file_path}...")

    with open(file_path, 'r') as f:
        lines = f.readlines()

    # Track terrain type for each tile
    current_terrain = None
    cleaned_lines = []

    for line in lines:
        # Check if this is a terrain declaration
        if 'terrain:' in line:
            # Extract terrain type
            parts = line.split('terrain:')
            if len(parts) > 1:
                terrain_part = parts[1].strip().rstrip(',')
                current_terrain = terrain_part

        # Skip foliage_density lines for non-Forest terrains
        if 'foliage_density:' in line and current_terrain != 'Forest':
            continue

        cleaned_lines.append(line)

    # Write back
    backup_path = f"{file_path}.bak3"
    print(f"Creating backup at {backup_path}")
    with open(backup_path, 'w') as f:
        f.writelines(lines)

    with open(file_path, 'w') as f:
        f.writelines(cleaned_lines)

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
            import traceback
            traceback.print_exc()
            return 1

    print(f"\n✓ Successfully cleaned {len(map_files)} map file(s)!")
    print("Backups saved with .bak3 extension")
    return 0


if __name__ == '__main__':
    sys.exit(main())
