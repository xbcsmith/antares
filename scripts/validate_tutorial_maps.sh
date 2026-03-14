#!/bin/bash
# SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
# SPDX-License-Identifier: Apache-2.0

# Validation script for tutorial maps with terrain features
# This script verifies that all tutorial maps have valid RON syntax,
# load correctly, and contain the expected terrain-specific features.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
MAPS_DIR="$PROJECT_ROOT/data/maps"

echo "🗺️  Tutorial Maps Validation"
echo "=============================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

validate_count=0
success_count=0
failed_count=0

# Function to validate a single map
validate_map() {
    local map_file=$1
    local map_name=$2
    local expected_id=$3

    ((validate_count++))

    if [ ! -f "$map_file" ]; then
        echo -e "${RED}✗${NC} $map_name: File not found at $map_file"
        ((failed_count++))
        return 1
    fi

    echo "Validating $map_name..."

    # Check for RON syntax errors by attempting to parse with cargo
    if cargo run --bin validate_map -- "$map_file" > /dev/null 2>&1; then
        echo -e "  ${GREEN}✓${NC} RON syntax is valid"
    else
        echo -e "  ${RED}✗${NC} RON syntax error or validation failed"
        ((failed_count++))
        return 1
    fi

    # Check for required terrain-specific fields
    case $expected_id in
        1)
            # Town Square: should have height fields for walls
            if grep -q "height: Some" "$map_file"; then
                echo -e "  ${GREEN}✓${NC} Wall height variations found"
            else
                echo -e "  ${YELLOW}⚠${NC}  No wall height variations found"
            fi
            ;;
        2)
            # Forest Entrance: should have tree_type and grass_density
            if grep -q "tree_type: Some" "$map_file"; then
                echo -e "  ${GREEN}✓${NC} Tree types found"
            else
                echo -e "  ${YELLOW}⚠${NC}  No tree types found"
            fi
            if grep -q "grass_density: Some" "$map_file"; then
                echo -e "  ${GREEN}✓${NC} Grass density variations found"
            else
                echo -e "  ${YELLOW}⚠${NC}  No grass density variations found"
            fi
            ;;
        3)
            # Dungeon Level 1: should have rock_variant and water_flow_direction
            if grep -q "rock_variant: Some" "$map_file"; then
                echo -e "  ${GREEN}✓${NC} Rock variants found"
            else
                echo -e "  ${YELLOW}⚠${NC}  No rock variants found"
            fi
            if grep -q "water_flow_direction: Some" "$map_file"; then
                echo -e "  ${GREEN}✓${NC} Water flow directions found"
            else
                echo -e "  ${YELLOW}⚠${NC}  No water flow directions found"
            fi
            ;;
    esac

    # Check for duplicate visual fields (which would be invalid RON)
    local visual_count=$(grep -c "visual:" "$map_file" || true)
    local tile_count=$(grep -c "x: [0-9]" "$map_file" || true)

    # A rough check: visual count should be less than or equal to tile count
    if [ "$visual_count" -le "$((tile_count + 10))" ]; then
        echo -e "  ${GREEN}✓${NC} No obvious duplicate visual fields"
    else
        echo -e "  ${YELLOW}⚠${NC}  Potential duplicate visual fields detected"
    fi

    echo -e "  ${GREEN}✓${NC} $map_name validation passed"
    ((success_count++))
    echo ""
}

# Validate all three tutorial maps
validate_map "$MAPS_DIR/starter_town.ron" "Map 1: Town Square" 1
validate_map "$MAPS_DIR/forest_area.ron" "Map 2: Forest Entrance" 2
validate_map "$MAPS_DIR/starter_dungeon.ron" "Map 3: Dungeon Level 1" 3

# Summary
echo "=============================="
echo "Validation Summary"
echo "=============================="
echo "Total maps checked: $validate_count"
echo -e "Passed: ${GREEN}$success_count${NC}"
echo -e "Failed: ${RED}$failed_count${NC}"
echo ""

if [ $failed_count -eq 0 ]; then
    echo -e "${GREEN}✅ All tutorial maps validated successfully!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some validation checks failed${NC}"
    exit 1
fi
