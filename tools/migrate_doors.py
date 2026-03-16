#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
# SPDX-License-Identifier: Apache-2.0
"""
Migration script: converts wall_type: Door tiles to MapEvent::Furniture events.

For each tile with `wall_type: Door`:
  1. Extracts the tile's (x, y) coordinates.
  2. Replaces `wall_type: Door,` with `wall_type: r#None,` in the tile block.
  3. Inserts a Furniture event at (x, y) in the map's events section.

This migration is idempotent — running it twice on the same file is safe.

Usage:
    python3 tools/migrate_doors.py [--dry-run] [paths...]

    --dry-run   Show what would change without writing any files.
    paths       One or more .ron files or directories containing .ron files.
                Defaults to both campaign map directories when omitted.

Examples:
    # Migrate everything (both test_campaign and tutorial)
    python3 tools/migrate_doors.py

    # Preview changes without writing
    python3 tools/migrate_doors.py --dry-run

    # Migrate a specific file
    python3 tools/migrate_doors.py data/test_campaign/data/maps/map_1.ron
"""

import re
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# RON template for the new Furniture event entry
# ---------------------------------------------------------------------------
# Indented to match the 8-space indent used by all existing event entries.
FURNITURE_EVENT_TEMPLATE = """\
        (
            x: {x},
            y: {y},
        ): Furniture(
            name: "Door",
            furniture_id: None,
            furniture_type: Door,
            rotation_y: None,
            scale: 1.0,
            material: Wood,
            flags: (
                lit: false,
                locked: false,
                blocking: true,
            ),
            color_tint: None,
            key_item_id: None,
        ),"""

# ---------------------------------------------------------------------------
# Regexes
# ---------------------------------------------------------------------------

# Matches a Door tile's fields after wall_type up through x/y.
# Field order in a tile block is always:
#   terrain, wall_type, blocked, is_special, is_dark, visited, x, y, visual
_DOOR_TILE_RE = re.compile(
    r"wall_type:\s*Door,"
    r"\s*blocked:[^,]+,"
    r"\s*is_special:[^,]+,"
    r"\s*is_dark:[^,]+,"
    r"\s*visited:[^,]+,"
    r"\s*x:\s*(\d+),"
    r"\s*y:\s*(\d+),",
    re.DOTALL,
)

# Matches the entire events block: `    events: {` … `\n    },`
# Non-greedy so it stops at the *first* `\n    },` — which is always the
# end of the events map (confirmed: no other `^    },` lines exist in these
# files).
_EVENTS_BLOCK_RE = re.compile(
    r"(    events: \{)(.*?)(\n    \},)",
    re.DOTALL,
)


# ---------------------------------------------------------------------------
# Core transform functions
# ---------------------------------------------------------------------------


def find_door_positions(content: str) -> list[tuple[int, int]]:
    """Return a list of (x, y) for every tile whose wall_type is Door."""
    return [
        (int(m.group(1)), int(m.group(2)))
        for m in _DOOR_TILE_RE.finditer(content)
    ]


def already_migrated(content: str, x: int, y: int) -> bool:
    """Return True if a Furniture event for (x, y) already exists in content."""
    # Match the exact key format written by this script.
    needle = f"x: {x},\n            y: {y},\n        ): Furniture("
    return needle in content


def replace_door_wall_types(content: str) -> str:
    """Replace every `wall_type: Door,` with `wall_type: r#None,`."""
    return content.replace("wall_type: Door,", "wall_type: r#None,")


def add_furniture_events(content: str, positions: list[tuple[int, int]]) -> str:
    """Insert one Furniture event entry per position into the events section.

    New entries are appended just before the closing `},` of the events map so
    they appear at the end of the existing event list.

    Args:
        content:   Full text of the RON map file.
        positions: List of (x, y) tuples for which to add events.

    Returns:
        Modified file content, or the original content unchanged if the events
        section could not be located (a warning is printed in that case).
    """
    if not positions:
        return content

    new_entries = "\n".join(
        FURNITURE_EVENT_TEMPLATE.format(x=x, y=y) for x, y in positions
    )

    def _insert(m: re.Match) -> str:  # type: ignore[type-arg]
        return m.group(1) + m.group(2) + "\n" + new_entries + m.group(3)

    result, n_subs = _EVENTS_BLOCK_RE.subn(_insert, content, count=1)
    if n_subs == 0:
        print("  WARNING: events section not found — no events inserted!", file=sys.stderr)
        return content
    return result


# ---------------------------------------------------------------------------
# Per-file migration
# ---------------------------------------------------------------------------


def migrate_file(path: Path, dry_run: bool = False) -> None:
    """Migrate a single RON map file in-place (unless dry_run is True)."""
    content = path.read_text(encoding="utf-8")

    doors = find_door_positions(content)
    if not doors:
        print(f"  {path}: no Door tiles found — skipping")
        return

    # Split doors into already-migrated vs. still pending
    pending: list[tuple[int, int]] = []
    skipped: list[tuple[int, int]] = []
    for x, y in doors:
        if already_migrated(content, x, y):
            skipped.append((x, y))
        else:
            pending.append((x, y))

    if skipped:
        print(f"  {path}: {len(skipped)} door(s) already migrated: {skipped}")
    if not pending:
        print(f"  {path}: nothing to do")
        return

    print(f"  {path}: migrating {len(pending)} door(s) at {pending}")

    # Apply transformations
    new_content = replace_door_wall_types(content)
    new_content = add_furniture_events(new_content, pending)

    if new_content == content:
        print(f"  {path}: WARNING — content unchanged after transforms", file=sys.stderr)
        return

    if dry_run:
        print(f"  {path}: [DRY RUN] would overwrite ({len(pending)} change(s))")
    else:
        path.write_text(new_content, encoding="utf-8")
        print(f"  {path}: written ({len(pending)} change(s))")


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    dry_run = "--dry-run" in sys.argv
    raw_paths = [a for a in sys.argv[1:] if not a.startswith("--")]

    if raw_paths:
        targets: list[Path] = [Path(p) for p in raw_paths]
    else:
        # Default: both map directories relative to the project root.
        script_dir = Path(__file__).resolve().parent
        project_root = script_dir.parent
        targets = [
            project_root / "data" / "test_campaign" / "data" / "maps",
            project_root / "campaigns" / "tutorial" / "data" / "maps",
        ]

    if dry_run:
        print("DRY RUN — no files will be modified\n")

    for target in targets:
        if target.is_dir():
            ron_files = sorted(target.glob("*.ron"))
            if not ron_files:
                print(f"  {target}: no .ron files found")
                continue
            for ron_file in ron_files:
                migrate_file(ron_file, dry_run)
        elif target.is_file():
            migrate_file(target, dry_run)
        else:
            print(f"ERROR: path does not exist: {target}", file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    main()
