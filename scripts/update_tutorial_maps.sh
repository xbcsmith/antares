#!/bin/bash
# SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
# SPDX-License-Identifier: Apache-2.0

# Script to update tutorial maps with terrain-specific visual metadata
# This script adds terrain features to the tutorial maps:
# - starter_town.ron: Wall height variations and structure metadata
# - forest_area.ron: Tree types and grass density variations
# - starter_dungeon.ron: Rock variants and water flow directions

set -e

MAPS_DIR="data/maps"
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo "🗺️  Tutorial Maps Terrain Features Update"
echo "=========================================="
echo ""

# Function to add visual metadata to town square walls
update_town_square() {
    local input_file=$1
    local output_file=$2

    echo "Processing Map 1: Town Square..."

    # Create a temporary file with added visual metadata
    cp "$input_file" "$output_file"

    # Add height and color_tint to outer wall tiles
    # These patterns match wall tiles on the perimeter

    # For tiles at x=0 or x=19 (left/right walls) with wall_type: Normal
    sed -i.bak 's/\(terrain: Ground,\s*wall_type: Normal,\s*blocked: false,\s*is_special: false,\s*is_dark: false,\s*visited: false,\s*x: \(0\|19\),\)/\1/g' "$output_file"

    # Simpler approach: Add visual metadata before event_trigger: None on outer walls
    # This is pattern-based and works for the existing map structure

    perl -i -0777 -pe '
        # For outer perimeter walls (x=0, x=19, y=0, y=14) with wall_type: Normal
        # Add visual metadata with height and color

        # Top wall row (y=0)
        s/(x: (\d+),\s*y: 0,\s*event_trigger: None,\s*\),\s*\()(?=terrain: Ground,\s*wall_type: Normal)/$1visual: (height: Some(3.5), color_tint: Some((0.7, 0.7, 0.7))), /g if /wall_type: Normal/;
    ' "$output_file"

    echo "  ✓ Town Square updated"
}

# Function to add visual metadata to forest entrance
update_forest_entrance() {
    local input_file=$1
    local output_file=$2

    echo "Processing Map 2: Forest Entrance..."

    cp "$input_file" "$output_file"

    # Add tree types and grass density for forest tiles
    # Pattern: Forest terrain tiles in specific areas get tree_type and foliage_density

    perl -i -0777 -pe '
        # Find Forest tiles and add tree metadata before event_trigger
        s/(terrain: Forest,.*?)(event_trigger: None,)/$1visual: (tree_type: Some(Oak), foliage_density: Some(1.8), color_tint: Some((0.2, 0.6, 0.2))), $2/sg if /(x: [5-7],|x: [5-7],\s*y: [2-4],)/;
    ' "$output_file"

    echo "  ✓ Forest Entrance updated"
}

# Function to add visual metadata to dungeon
update_dungeon() {
    local input_file=$1
    local output_file=$2

    echo "Processing Map 3: Dungeon Level 1..."

    cp "$input_file" "$output_file"

    # Add rock variants and water flow for dungeon
    # Pattern: Stone/Mountain terrain and Water tiles get appropriate metadata

    perl -i -0777 -pe '
        # Add rock variants to Stone tiles
        s/(terrain: Stone,.*?)(event_trigger: None,)/$1visual: (rock_variant: Some(Jagged), color_tint: Some((0.5, 0.45, 0.4))), $2/sg if /(x: [1-3],.*y: [1-3],|y: [1-3],.*x: [1-3],)/;

        # Add water flow to Water tiles
        s/(terrain: Water,.*?x: (1[0-4]),.*?)(event_trigger: None,)/$1visual: (water_flow_direction: Some(East), color_tint: Some((0.3, 0.4, 0.6))), $3/sg;
    ' "$output_file"

    echo "  ✓ Dungeon Level 1 updated"
}

# Main update logic
if [ ! -d "$MAPS_DIR" ]; then
    echo "❌ Error: $MAPS_DIR directory not found"
    exit 1
fi

success_count=0
failed_count=0

# Update starter_town.ron
if [ -f "$MAPS_DIR/starter_town.ron" ]; then
    update_town_square "$MAPS_DIR/starter_town.ron" "$TEMP_DIR/starter_town.ron"
    if [ $? -eq 0 ]; then
        cp "$TEMP_DIR/starter_town.ron" "$MAPS_DIR/starter_town.ron"
        ((success_count++))
    else
        echo "  ✗ Town Square failed"
        ((failed_count++))
    fi
else
    echo "⚠️  starter_town.ron not found"
    ((failed_count++))
fi

# Update forest_area.ron
if [ -f "$MAPS_DIR/forest_area.ron" ]; then
    update_forest_entrance "$MAPS_DIR/forest_area.ron" "$TEMP_DIR/forest_area.ron"
    if [ $? -eq 0 ]; then
        cp "$TEMP_DIR/forest_area.ron" "$MAPS_DIR/forest_area.ron"
        ((success_count++))
    else
        echo "  ✗ Forest Entrance failed"
        ((failed_count++))
    fi
else
    echo "⚠️  forest_area.ron not found"
    ((failed_count++))
fi

# Update starter_dungeon.ron
if [ -f "$MAPS_DIR/starter_dungeon.ron" ]; then
    update_dungeon "$MAPS_DIR/starter_dungeon.ron" "$TEMP_DIR/starter_dungeon.ron"
    if [ $? -eq 0 ]; then
        cp "$TEMP_DIR/starter_dungeon.ron" "$MAPS_DIR/starter_dungeon.ron"
        ((success_count++))
    else
        echo "  ✗ Dungeon Level 1 failed"
        ((failed_count++))
    fi
else
    echo "⚠️  starter_dungeon.ron not found"
    ((failed_count++))
fi

echo ""
echo "=========================================="
echo "Results: $success_count succeeded, $failed_count failed"
echo "=========================================="

if [ $failed_count -eq 0 ]; then
    echo ""
    echo "✅ All tutorial maps updated with terrain features!"
    exit 0
else
    echo ""
    echo "❌ Some maps failed to update"
    exit 1
fi
