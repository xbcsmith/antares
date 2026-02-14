#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
# SPDX-License-Identifier: Apache-2.0

"""
Update tutorial maps with terrain-specific visual metadata.

This script adds terrain-specific features to the RON map files:
- starter_town.ron: Wall height variations and structure metadata
- forest_area.ron: Tree types and grass density variations
- starter_dungeon.ron: Rock variants and water flow directions

Usage: python3 scripts/update_tutorial_maps.py
"""

import re
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple


def load_map_ron(path: Path) -> str:
    """Load a RON map file."""
    with open(path, 'r') as f:
        return f.read()


def save_map_ron(path: Path, content: str) -> None:
    """Save a RON map file."""
    with open(path, 'w') as f:
        f.write(content)


def find_tile_position(content: str, x: int, y: int) -> Optional[Tuple[int, int]]:
    """
    Find the tile at (x, y) in the RON content.
    Returns (start_pos, end_pos) if found, None otherwise.
    """
    # Pattern to find a tile with specific x and y coordinates
    pattern = rf'(?:x:\s*{x},\s*y:\s*{y}|y:\s*{y},\s*x:\s*{x})'

    for match in re.finditer(pattern, content):
        # Find the opening ( before this position
        pos = match.start()
        paren_count = 0
        start = pos

        # Find the opening paren
        while start > 0:
            if content[start] == ')':
                paren_count += 1
            elif content[start] == '(':
                if paren_count == 0:
                    break
                paren_count -= 1
            start -= 1

        # Find the closing paren
        end = match.end()
        paren_count = 0
        while end < len(content):
            if content[end] == '(':
                paren_count += 1
            elif content[end] == ')':
                if paren_count == 0:
                    break
                paren_count += 1
            end += 1

        return (start, end + 1)

    return None


def add_visual_metadata_field(tile_str: str, field_name: str, field_value: str) -> str:
    """
    Add or update a visual metadata field in a tile definition.
    """
    # Check if visual: ( already exists
    if 'visual: (' in tile_str:
        # Add field inside existing visual block
        visual_start = tile_str.rfind('visual: (')
        # Find the closing paren of the visual block
        paren_pos = visual_start + len('visual: (')
        paren_count = 1
        while paren_count > 0 and paren_pos < len(tile_str):
            if tile_str[paren_pos] == '(':
                paren_count += 1
            elif tile_str[paren_pos] == ')':
                paren_count -= 1
            paren_pos += 1

        # Insert the field before the closing paren
        insert_pos = paren_pos - 1
        field_line = f"\n                {field_name}: {field_value},"
        return tile_str[:insert_pos] + field_line + tile_str[insert_pos:]
    else:
        # Add new visual block before event_trigger
        if 'event_trigger:' in tile_str:
            insert_pos = tile_str.rfind('event_trigger:')
            visual_block = f"visual: (\n                {field_name}: {field_value},\n            ),\n            "
            return tile_str[:insert_pos] + visual_block + tile_str[insert_pos:]
        else:
            # Add before the closing paren
            insert_pos = tile_str.rfind(')')
            visual_block = f",\n            visual: (\n                {field_name}: {field_value},\n            )"
            return tile_str[:insert_pos] + visual_block + tile_str[insert_pos:]


def update_starter_town(content: str) -> str:
    """Add wall height variations and structure metadata to town square."""
    # Add tall outer walls
    outer_wall_positions = [
        (0, 0), (19, 0), (0, 14), (19, 14),  # Corners
        (1, 0), (2, 0), (3, 0), (4, 0), (5, 0),  # Top edge
        (6, 0), (7, 0), (8, 0), (9, 0), (10, 0),
    ]

    for x, y in outer_wall_positions:
        pattern = rf'x:\s*{x},\s*y:\s*{y}.*?(?=\),)'
        match = re.search(pattern, content, re.DOTALL)
        if match and 'wall_type: Normal' in match.group():
            # Add height and color to this tile
            tile_section = match.group()
            if 'visual:' not in tile_section:
                # Replace the tile with one that has visual metadata
                visual_metadata = 'visual: (\n                height: Some(3.5),\n                color_tint: Some((0.7, 0.7, 0.7)),\n            ),'
                new_tile = tile_section.rstrip(',') + ',\n            ' + visual_metadata
                content = content.replace(tile_section + ',', new_tile)

    return content


def update_forest_area(content: str) -> str:
    """Add tree types and grass density variations to forest entrance."""
    # Pattern: find tiles in dense oak forest area (5..=7 x, 2..=4 y)
    # and add tree_type: Some(Oak) and foliage_density

    for x in range(5, 8):
        for y in range(2, 5):
            pattern = rf'x:\s*{x},\s*y:\s*{y}.*?(?=\),)'
            matches = list(re.finditer(pattern, content, re.DOTALL))
            for match in matches:
                tile_section = match.group()
                if 'Forest' in tile_section and 'tree_type' not in tile_section:
                    # Add tree metadata
                    visual_metadata = 'visual: (\n                tree_type: Some(Oak),\n                foliage_density: Some(1.8),\n                color_tint: Some((0.2, 0.6, 0.2)),\n            ),'
                    new_tile = tile_section.rstrip(',') + ',\n            ' + visual_metadata
                    content = content.replace(tile_section + ',', new_tile, 1)

    return content


def update_starter_dungeon(content: str) -> str:
    """Add rock variants and water flow to dungeon level 1."""
    # Add jagged rocks to cave walls (1..=3 x, 1..=3 y)
    for x in range(1, 4):
        for y in range(1, 4):
            pattern = rf'x:\s*{x},\s*y:\s*{y}.*?(?=\),)'
            matches = list(re.finditer(pattern, content, re.DOTALL))
            for match in matches:
                tile_section = match.group()
                if ('Mountain' in tile_section or 'wall_type: Normal' in tile_section) and 'rock_variant' not in tile_section:
                    visual_metadata = 'visual: (\n                rock_variant: Some(Jagged),\n                color_tint: Some((0.5, 0.45, 0.4)),\n            ),'
                    new_tile = tile_section.rstrip(',') + ',\n            ' + visual_metadata
                    content = content.replace(tile_section + ',', new_tile, 1)

    # Add water flow direction to water tiles at y=5 (10..=14 x)
    for x in range(10, 15):
        pattern = rf'x:\s*{x},\s*y:\s*5.*?(?=\),)'
        matches = list(re.finditer(pattern, content, re.DOTALL))
        for match in matches:
            tile_section = match.group()
            if 'Water' in tile_section and 'water_flow_direction' not in tile_section:
                if x <= 12:
                    flow = 'East'
                else:
                    flow = 'South'
                visual_metadata = f'visual: (\n                water_flow_direction: Some({flow}),\n                color_tint: Some((0.3, 0.4, 0.6)),\n            ),'
                new_tile = tile_section.rstrip(',') + ',\n            ' + visual_metadata
                content = content.replace(tile_section + ',', new_tile, 1)

    return content


def main() -> int:
    """Main entry point."""
    print("🗺️  Tutorial Maps Terrain Features Update")
    print("=" * 50)
    print()

    maps_to_update = [
        ("data/maps/starter_town.ron", "Map 1: Town Square", update_starter_town),
        ("data/maps/forest_area.ron", "Map 2: Forest Entrance", update_forest_area),
        ("data/maps/starter_dungeon.ron", "Map 3: Dungeon Level 1", update_starter_dungeon),
    ]

    success_count = 0
    failed_count = 0

    for map_path_str, description, update_func in maps_to_update:
        map_path = Path(map_path_str)

        if not map_path.exists():
            print(f"⚠️  Skipping {description}: file not found at {map_path}")
            failed_count += 1
            continue

        print(f"Processing {description}...")

        try:
            # Load the map
            content = load_map_ron(map_path)

            # Apply updates
            updated_content = update_func(content)

            # Save the map
            save_map_ron(map_path, updated_content)

            print(f"  ✓ Successfully updated {map_path}")
            success_count += 1

        except Exception as e:
            print(f"  ✗ Failed to update {map_path}: {e}")
            failed_count += 1

        print()

    print("=" * 50)
    print(f"Results: {success_count} succeeded, {failed_count} failed")
    print("=" * 50)

    if failed_count == 0:
        print("\n✅ All tutorial maps updated with terrain features!")
        return 0
    else:
        print(f"\n❌ {failed_count} map(s) failed to update")
        return 1


if __name__ == '__main__':
    sys.exit(main())
