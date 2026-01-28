#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
# SPDX-License-Identifier: Apache-2.0

"""
Generate placeholder sprite PNG files for testing and development.

This script creates simple colored grid-based sprite sheets that match
the specifications in data/sprite_sheets.ron. Useful for testing sprite
rendering before final artwork is created.

Usage:
    python scripts/generate_placeholder_sprites.py
    python scripts/generate_placeholder_sprites.py --output-dir custom/path
    python scripts/generate_placeholder_sprites.py --sheets walls terrain

    # Single-sheet mode:
    # - Use `--type <name>` to produce a single placeholder sheet (e.g. npc)
    # - Optionally provide `--size WIDTHxHEIGHT` (e.g. 32x48)
    # - Use `--output` to write a single FILE path (overrides --output-dir for this run)
    python scripts/generate_placeholder_sprites.py --output campaigns/tutorial/assets/sprites/placeholders/npc_placeholder.png --size 32x48 --type npc
"""

import argparse
import os
import sys
from pathlib import Path
from typing import Dict, Tuple

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:
    print("ERROR: PIL/Pillow not found. Install with:")
    print("  pip install Pillow")
    sys.exit(1)


# Sprite sheet specifications matching data/sprite_sheets.ron
SPRITE_SHEETS: Dict[str, Dict] = {
    "walls": {
        "tile_size": (128, 256),
        "columns": 4,
        "rows": 4,
        "colors": [
            (100, 100, 100),    # stone_wall
            (150, 100, 50),     # brick_wall
            (139, 90, 43),      # wood_wall
            (80, 80, 80),       # damaged_stone
            (100, 120, 80),     # moss_stone
            (180, 120, 80),     # reinforced_brick
            (120, 80, 50),      # weathered_wood
            (70, 70, 70),       # cracked_stone
            (110, 110, 110),    # placeholder 8
            (120, 120, 120),    # placeholder 9
            (130, 130, 130),    # placeholder 10
            (140, 140, 140),    # placeholder 11
            (150, 150, 150),    # placeholder 12
            (160, 160, 160),    # placeholder 13
            (170, 170, 170),    # placeholder 14
            (180, 180, 180),    # placeholder 15
        ],
        "names": [
            "stone_wall", "brick_wall", "wood_wall", "damaged_stone",
            "moss_stone", "reinforced_brick", "weathered_wood", "cracked_stone",
            "p8", "p9", "p10", "p11", "p12", "p13", "p14", "p15",
        ],
    },
    "doors": {
        "tile_size": (128, 256),
        "columns": 4,
        "rows": 2,
        "colors": [
            (139, 69, 19),      # wood_door_closed
            (184, 134, 11),     # wood_door_open
            (128, 128, 128),    # iron_door_closed
            (169, 169, 169),    # iron_door_open
            (80, 20, 20),       # locked_door
            (60, 40, 60),       # secret_door
            (100, 100, 100),    # placeholder 6
            (110, 110, 110),    # placeholder 7
        ],
        "names": [
            "wood_door_closed", "wood_door_open", "iron_door_closed", "iron_door_open",
            "locked_door", "secret_door", "p6", "p7",
        ],
    },
    "terrain": {
        "tile_size": (128, 128),
        "columns": 8,
        "rows": 8,
        "colors": [
            (180, 180, 180),    # stone_floor
            (34, 139, 34),      # grass
            (139, 90, 43),      # dirt
            (64, 164, 223),     # water
            (255, 69, 0),       # lava
            (107, 142, 35),     # swamp
            (210, 180, 140),    # wood_floor
            (240, 248, 255),    # marble_floor
            (200, 200, 200),    # p8
            (210, 210, 210),    # p9
            (220, 220, 220),    # p10
            (230, 230, 230),    # p11
            (240, 240, 240),    # p12
            (100, 100, 100),    # p13
            (110, 110, 110),    # p14
            (120, 120, 120),    # p15
            (130, 130, 130),    # p16
            (140, 140, 140),    # p17
            (150, 150, 150),    # p18
            (160, 160, 160),    # p19
            (170, 170, 170),    # p20
            (180, 180, 180),    # p21
            (190, 190, 190),    # p22
            (200, 200, 200),    # p23
            (210, 210, 210),    # p24
            (220, 220, 220),    # p25
            (230, 230, 230),    # p26
            (240, 240, 240),    # p27
            (95, 95, 95),       # p28
            (105, 105, 105),    # p29
            (115, 115, 115),    # p30
            (125, 125, 125),    # p31
            (135, 135, 135),    # p32
            (145, 145, 145),    # p33
            (155, 155, 155),    # p34
            (165, 165, 165),    # p35
            (175, 175, 175),    # p36
            (185, 185, 185),    # p37
            (195, 195, 195),    # p38
            (205, 205, 205),    # p39
            (215, 215, 215),    # p40
            (225, 225, 225),    # p41
            (235, 235, 235),    # p42
            (245, 245, 245),    # p43
            (90, 90, 90),       # p44
            (100, 100, 100),    # p45
            (110, 110, 110),    # p46
            (120, 120, 120),    # p47
            (130, 130, 130),    # p48
            (140, 140, 140),    # p49
            (150, 150, 150),    # p50
            (160, 160, 160),    # p51
            (170, 170, 170),    # p52
            (180, 180, 180),    # p53
            (190, 190, 190),    # p54
            (200, 200, 200),    # p55
            (210, 210, 210),    # p56
            (220, 220, 220),    # p57
            (230, 230, 230),    # p58
            (240, 240, 240),    # p59
            (250, 250, 250),    # p60
            (85, 85, 85),       # p61
            (125, 125, 125),    # p62
            (165, 165, 165),    # p63
        ],
        "names": [
            "stone_floor", "grass", "dirt", "water", "lava", "swamp", "wood_floor", "marble_floor",
        ] + [f"p{i}" for i in range(8, 64)],
    },
    "trees": {
        "tile_size": (128, 256),
        "columns": 4,
        "rows": 4,
        "colors": [
            (34, 139, 34),      # oak_tree
            (0, 100, 0),        # pine_tree
            (101, 67, 33),      # dead_tree
            (75, 0, 130),       # magical_tree
            (50, 50, 50),       # p4
            (60, 60, 60),       # p5
            (70, 70, 70),       # p6
            (80, 80, 80),       # p7
            (90, 90, 90),       # p8
            (100, 100, 100),    # p9
            (110, 110, 110),    # p10
            (120, 120, 120),    # p11
            (130, 130, 130),    # p12
            (140, 140, 140),    # p13
            (150, 150, 150),    # p14
            (160, 160, 160),    # p15
        ],
        "names": [
            "oak_tree", "pine_tree", "dead_tree", "magical_tree",
        ] + [f"p{i}" for i in range(4, 16)],
    },
    "decorations": {
        "tile_size": (64, 64),
        "columns": 8,
        "rows": 8,
        "colors": [
            (255, 200, 0),      # torch
            (139, 90, 43),      # chest
            (160, 82, 45),      # barrel
            (188, 143, 143),    # crate
            (139, 69, 19),      # bones
            (169, 169, 169),    # rubble
        ] + [(100 + i*5, 100 + i*5, 100 + i*5) for i in range(58)],
        "names": [
            "torch", "chest", "barrel", "crate", "bones", "rubble",
        ] + [f"p{i}" for i in range(6, 64)],
    },
    "npcs_town": {
        "tile_size": (32, 48),
        "columns": 4,
        "rows": 4,
        "colors": [
            (200, 100, 50),     # guard
            (100, 100, 200),    # merchant
            (150, 100, 50),     # innkeeper
            (180, 140, 80),     # blacksmith
            (200, 200, 100),    # priest
            (150, 150, 150),    # noble
            (100, 150, 100),    # peasant
            (200, 150, 100),    # child
            (120, 100, 100),    # elder
            (100, 100, 150),    # mage_npc
            (150, 100, 100),    # warrior_npc
            (100, 150, 100),    # rogue_npc
            (200, 100, 100),    # captain
            (150, 150, 100),    # mayor
            (180, 180, 180),    # servant
            (100, 100, 100),    # beggar
        ],
        "names": [
            "guard", "merchant", "innkeeper", "blacksmith",
            "priest", "noble", "peasant", "child",
            "elder", "mage_npc", "warrior_npc", "rogue_npc",
            "captain", "mayor", "servant", "beggar",
        ],
    },
    "monsters_basic": {
        "tile_size": (32, 48),
        "columns": 4,
        "rows": 4,
        "colors": [
            (100, 150, 50),     # goblin
            (150, 100, 50),     # orc
            (200, 200, 200),    # skeleton
            (100, 150, 100),    # zombie
            (150, 100, 150),    # wolf
            (180, 100, 50),     # bear
            (100, 100, 100),    # spider
            (200, 100, 200),    # bat
            (150, 150, 100),    # rat
            (100, 150, 100),    # snake
            (150, 200, 100),    # slime
            (200, 100, 100),    # imp
            (180, 100, 80),     # bandit
            (140, 100, 100),    # thug
            (100, 100, 200),    # cultist
            (150, 100, 150),    # ghoul
        ],
        "names": [
            "goblin", "orc", "skeleton", "zombie",
            "wolf", "bear", "spider", "bat",
            "rat", "snake", "slime", "imp",
            "bandit", "thug", "cultist", "ghoul",
        ],
    },
    "monsters_advanced": {
        "tile_size": (32, 48),
        "columns": 4,
        "rows": 4,
        "colors": [
            (200, 100, 0),      # dragon
            (100, 50, 150),     # lich
            (200, 0, 0),        # demon
            (200, 100, 150),    # vampire
            (150, 100, 200),    # beholder
            (180, 100, 50),     # minotaur
            (100, 100, 150),    # troll
            (150, 100, 50),     # ogre
            (200, 200, 150),    # wraith
            (255, 200, 0),      # elemental
            (180, 180, 180),    # golem
            (100, 150, 100),    # hydra
            (200, 100, 100),    # wyvern
            (150, 100, 200),    # chimera
            (100, 100, 100),    # basilisk
            (150, 150, 100),    # manticore
        ],
        "names": [
            "dragon", "lich", "demon", "vampire",
            "beholder", "minotaur", "troll", "ogre",
            "wraith", "elemental", "golem", "hydra",
            "wyvern", "chimera", "basilisk", "manticore",
        ],
    },
    "recruitables": {
        "tile_size": (32, 48),
        "columns": 4,
        "rows": 2,
        "colors": [
            (200, 100, 100),    # warrior_recruit
            (100, 100, 200),    # mage_recruit
            (100, 200, 100),    # rogue_recruit
            (200, 200, 100),    # cleric_recruit
            (100, 200, 200),    # ranger_recruit
            (200, 100, 200),    # paladin_recruit
            (200, 150, 100),    # bard_recruit
            (150, 150, 150),    # monk_recruit
        ],
        "names": [
            "warrior_recruit", "mage_recruit", "rogue_recruit", "cleric_recruit",
            "ranger_recruit", "paladin_recruit", "bard_recruit", "monk_recruit",
        ],
    },
    "signs": {
        "tile_size": (32, 64),
        "columns": 4,
        "rows": 2,
        "colors": [
            (139, 90, 43),      # wooden_sign
            (128, 128, 128),    # stone_marker
            (200, 50, 50),      # warning_sign
            (50, 150, 200),     # info_sign
            (200, 200, 50),     # quest_marker
            (150, 100, 50),     # shop_sign
            (255, 0, 0),        # danger_sign
            (50, 200, 50),      # direction_sign
        ],
        "names": [
            "wooden_sign", "stone_marker", "warning_sign", "info_sign",
            "quest_marker", "shop_sign", "danger_sign", "direction_sign",
        ],
    },
    "portals": {
        "tile_size": (128, 128),
        "columns": 4,
        "rows": 2,
        "colors": [
            (100, 100, 255),    # teleport_pad
            (200, 100, 200),    # dimensional_gate
            (200, 150, 100),    # stairs_up
            (100, 100, 100),    # stairs_down
            (0, 0, 255),        # portal_blue
            (255, 0, 0),        # portal_red
            (100, 50, 50),      # trap_door
            (50, 200, 100),     # exit_portal
        ],
        "names": [
            "teleport_pad", "dimensional_gate", "stairs_up", "stairs_down",
            "portal_blue", "portal_red", "trap_door", "exit_portal",
        ],
    },
}


def create_placeholder_sprite_sheet(
    name: str,
    spec: Dict,
    output_dir: Path,
) -> bool:
    """
    Create a single placeholder sprite sheet PNG.

    Args:
        name: Sprite sheet name (e.g., 'walls', 'npcs_town')
        spec: Sheet specification with tile_size, columns, rows, colors
        output_dir: Directory to write PNG file

    Returns:
        True if successful, False otherwise
    """
    tile_width, tile_height = spec["tile_size"]
    columns = spec["columns"]
    rows = spec["rows"]
    colors = spec["colors"]

    # Calculate image dimensions
    img_width = tile_width * columns
    img_height = tile_height * rows

    # Create RGBA image with transparent background
    image = Image.new("RGBA", (img_width, img_height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)

    # Fill each tile with a solid color
    for row in range(rows):
        for col in range(columns):
            sprite_index = row * columns + col

            # Get color for this sprite (cycle if we run out)
            color_idx = sprite_index % len(colors)
            base_color = colors[color_idx]

            # Calculate tile position
            x0 = col * tile_width
            y0 = row * tile_height
            x1 = x0 + tile_width
            y1 = y0 + tile_height

            # Add alpha channel to color
            color_with_alpha = (*base_color, 255)

            # Draw filled rectangle
            draw.rectangle([x0, y0, x1, y1], fill=color_with_alpha)

            # Draw border for visibility
            border_color = (0, 0, 0, 200)
            draw.rectangle([x0, y0, x1 - 1, y1 - 1], outline=border_color, width=1)

    # Save PNG file
    output_file = output_dir / f"{name}.png"
    image.save(output_file, "PNG")

    # Verify file was created
    if output_file.exists():
        file_size = output_file.stat().st_size
        print(f"✓ {name:20} → {output_file.name:30} ({file_size:,} bytes)")
        return True
    else:
        print(f"✗ {name:20} → FAILED to create")
        return False


def main():
    """Generate all placeholder sprite sheets."""
    parser = argparse.ArgumentParser(
        description="Generate placeholder sprite PNG files for testing",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Output file path for a single placeholder (overrides --output-dir when --type is used)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("assets/sprites"),
        help="Output directory for PNG files (default: assets/sprites)",
    )
    parser.add_argument(
        "--sheets",
        nargs="+",
        help="Specific sheets to generate (default: all)",
    )
    parser.add_argument(
        "--size",
        type=str,
        help="Single placeholder size as WIDTHxHEIGHT (e.g. 32x48); used with --type",
    )
    parser.add_argument(
        "--type",
        type=str,
        help="Generate a single placeholder of a given type (e.g. 'npc'); when set, the script generates only one sheet and exits",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="Overwrite existing files",
    )

    args = parser.parse_args()

    # If user requested a single placeholder (via --type), handle it and exit early
    if args.type:
        # Parse custom size if provided
        if args.size:
            try:
                w, h = map(int, args.size.lower().split("x"))
            except Exception:
                parser.error("--size must be WIDTHxHEIGHT, e.g. 32x48")
        else:
            # Defaults per recognized types (extendable)
            defaults = {"npc": (32, 48)}
            w, h = defaults.get(args.type, (32, 32))

        spec = {
            "tile_size": (w, h),
            "columns": 1,
            "rows": 1,
            "colors": [(200, 200, 200)],
            "names": [f"{args.type}_placeholder"],
        }

        # If an output file path provided, write there directly
        if args.output:
            output_path = Path(args.output)
            output_path.parent.mkdir(parents=True, exist_ok=True)
            sheet_name = output_path.stem
            out_dir = output_path.parent

            if output_path.exists() and not args.force:
                print(f"⊘ {sheet_name:20} → {output_path.name:30} (exists, use --force to overwrite)")
            else:
                create_placeholder_sprite_sheet(sheet_name, spec, out_dir)
        else:
            # Use --output-dir if no single-file output provided
            args.output_dir.mkdir(parents=True, exist_ok=True)
            sheet_name = f"{args.type}_placeholder"
            out_dir = args.output_dir
            if (out_dir / f"{sheet_name}.png").exists() and not args.force:
                print(f"⊘ {sheet_name:20} → {sheet_name}.png                 (exists, use --force to overwrite)")
            else:
                create_placeholder_sprite_sheet(sheet_name, spec, out_dir)

        # Done: generated single placeholder, exit early
        return

    # Ensure output directory exists for the normal multi-sheet generation flow
    args.output_dir.mkdir(parents=True, exist_ok=True)

    print(f"\n📦 Generating placeholder sprite sheets...")
    print(f"   Output directory: {args.output_dir.absolute()}\n")

    # Determine which sheets to generate
    if args.sheets:
        sheets_to_gen = {k: v for k, v in SPRITE_SHEETS.items() if k in args.sheets}
        invalid = set(args.sheets) - set(SPRITE_SHEETS.keys())
        if invalid:
            print(f"⚠️  Unknown sheets: {', '.join(invalid)}")
    else:
        sheets_to_gen = SPRITE_SHEETS

    # Generate each sprite sheet
    success_count = 0
    for sheet_name, sheet_spec in sheets_to_gen.items():
        output_file = args.output_dir / f"{sheet_name}.png"

        # Check if file exists
        if output_file.exists() and not args.force:
            print(f"⊘ {sheet_name:20} → {output_file.name:30} (exists, use --force to overwrite)")
            continue

        # Generate the sprite sheet
        if create_placeholder_sprite_sheet(sheet_name, sheet_spec, args.output_dir):
            success_count += 1

    # Summary
    print(f"\n✅ Generated {success_count} sprite sheet(s)")
    print(f"   Location: {args.output_dir.absolute()}\n")
    print("Next steps:")
    print("  1. Verify sprites loaded: cargo check")
    print("  2. Run tests: cargo nextest run")
    print("  3. Create a test map using these sprites")
    print()


if __name__ == "__main__":
    main()
