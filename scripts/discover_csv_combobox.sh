#!/bin/bash
# SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
# SPDX-License-Identifier: Apache-2.0

# discover_csv_combobox.sh
#
# Purpose:
#   Discover CSV-style parsing/formatting patterns and ComboBox usage across
#   the SDK Campaign Builder source and top-level bin editors.
#
#   This script generates raw result files under:
#     docs/explanation/csv_migration_raw_results.md
#     docs/explanation/combobox_raw_results.md
#
#   These files are intended as a discovery aide for the CSV-to-Vec migration
#   implementation plan. They are raw outputs and will require manual curation
#   (moving entries into the canonical inventory and checklist documents).
#
# Usage:
#   ./scripts/discover_csv_combobox.sh [--output-dir <dir>]
#
# Notes:
# - The script is intentionally read-only and will not modify code files.
# - It focuses on `sdk/campaign_builder/src` and `src` roots by default.
# - It is safe to add or re-run; results are overwritten.
#
# Output:
#   * docs/explanation/csv_migration_raw_results.md
#   * docs/explanation/combobox_raw_results.md
#
# Example:
#   cd /home/bsmith/go/src/github.com/xbcsmith/antares
#   ./scripts/discover_csv_combobox.sh
#
set -euo pipefail

# Default output directory for raw results
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUTPUT_DIR_DEFAULT="$PROJECT_ROOT/docs/explanation"

# Parse args (optional)
OUTPUT_DIR="$OUTPUT_DIR_DEFAULT"
while [ $# -gt 0 ]; do
    case "$1" in
        --output-dir)
            if [ -z "${2-}" ]; then
                echo "ERROR: --output-dir requires an argument" >&2
                exit 2
            fi
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -h|--help)
            cat <<EOF
Usage: $(basename "$0") [--output-dir <dir>]

Discover CSV (.split/.join) and ComboBox occurrences in:
  - sdk/campaign_builder/src
  - src (top-level CLI bin editors)

Generates:
  - <output-dir>/csv_migration_raw_results.md
  - <output-dir>/combobox_raw_results.md
EOF
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            exit 1
            ;;
    esac
done

# Paths to search (primary)
SEARCH_DIRS=(
    "$PROJECT_ROOT/sdk/campaign_builder/src"
    "$PROJECT_ROOT/src"
)

# Result files
CSV_RESULTS_FILE="$OUTPUT_DIR/csv_migration_raw_results.md"
COMBOBOX_RESULTS_FILE="$OUTPUT_DIR/combobox_raw_results.md"

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

# Helper: safely run grep and append results
append_header() {
    local file="$1"
    local header="$2"
    echo "" >> "$file"
    echo "## $header" >> "$file"
    echo "" >> "$file"
}

append_command_output() {
    local file="$1"
    shift
    local cmd_output
    # We use subshell to capture stderr as well (silence no-match errors)
    cmd_output="$( { "$@" 2>/dev/null || true; } | sed 's/^/* /' )"
    if [ -n "$cmd_output" ]; then
        printf "%s\n" "$cmd_output" >> "$file"
    else
        printf "  * (no results)\n\n" >> "$file"
    fi
}

timestamp() {
    date -u +"%Y-%m-%d %H:%M:%SZ"
}

# Write CSV Results header
{
    echo "# CSV Migration Raw Results"
    echo
    echo "Generated: $(timestamp)"
    echo
    echo "This file contains raw search results for CSV-style parsing and formatting in the SDK and top-level editors."
    echo "It includes examples of '.split(' with comma separators, `.join(` with comma joining, and any comments referring to 'comma-separated'."
    echo
    echo "Paths searched:"
    for p in "${SEARCH_DIRS[@]}"; do
        if [ -d "$p" ]; then
            echo "  - $p"
        fi
    done
    echo
} > "$CSV_RESULTS_FILE"

# Write ComboBox Results header
{
    echo "# ComboBox Raw Results"
    echo
    echo "Generated: $(timestamp)"
    echo
    echo "This file contains raw search results for `egui::ComboBox` and related ComboBox usage inside the SDK and top-level editors."
    echo
    echo "Paths searched:"
    for p in "${SEARCH_DIRS[@]}"; do
        if [ -d "$p" ]; then
            echo "  - $p"
        fi
    done
    echo
} > "$COMBOBOX_RESULTS_FILE"

# Discover CSV split occurrences (.split( with comma inside)
append_header "$CSV_RESULTS_FILE" "`.split(` occurrences containing a comma (CSV usage)"
{
    for dir in "${SEARCH_DIRS[@]}"; do
        if [ -d "$dir" ]; then
            # Find occurrences of the literal ".split(" and filter to lines containing a comma,
            # ignore lines like split('.') since those won't have commas.
            grep -rn --line-number --color=never --include="*.rs" -F ".split(" "$dir" 2>/dev/null || true
        fi
    done
} | grep -E "," || true | sed 's/^/* /' >> "$CSV_RESULTS_FILE" || true

# Discover .join(...) occurrences containing a comma (formatting to CSV)
append_header "$CSV_RESULTS_FILE" "`.join(` occurrences with a comma (CSV formatting output)"
{
    for dir in "${SEARCH_DIRS[@]}"; do
        if [ -d "$dir" ]; then
            grep -rn --line-number --color=never --include="*.rs" -F ".join(" "$dir" 2>/dev/null || true
        fi
    done
} | grep -E "," || true | sed 's/^/* /' >> "$CSV_RESULTS_FILE" || true

# Discover textual hints: 'comma separated' comments
append_header "$CSV_RESULTS_FILE" "'comma separated' / 'comma-separated' textual hints"
{
    for dir in "${SEARCH_DIRS[@]}"; do
        if [ -d "$dir" ]; then
            grep -rn --line-number --color=never --include="*.rs" -E "comma[- ]?separated" "$dir" 2>/dev/null || true
        fi
    done
} | sed 's/^/* /' >> "$CSV_RESULTS_FILE" || true

# Discover ComboBox occurrences
append_header "$COMBOBOX_RESULTS_FILE" "egui::ComboBox usages (explicit)"
{
    for dir in "${SEARCH_DIRS[@]}"; do
        if [ -d "$dir" ]; then
            grep -rn --line-number --color=never --include="*.rs" "egui::ComboBox" "$dir" 2>/dev/null || true
        fi
    done
} | sed 's/^/* /' >> "$COMBOBOX_RESULTS_FILE" || true

append_header "$COMBOBOX_RESULTS_FILE" "ComboBox::from* (alternative usage patterns)"
{
    for dir in "${SEARCH_DIRS[@]}"; do
        if [ -d "$dir" ]; then
            grep -rn --line-number --color=never --include="*.rs" -E "ComboBox::from|ComboBox::from_id|ComboBox::from_id_salt|ComboBox::from_id_source|ComboBox::from_label" "$dir" 2>/dev/null || true
        fi
    done
} | sed 's/^/* /' >> "$COMBOBOX_RESULTS_FILE" || true

append_header "$COMBOBOX_RESULTS_FILE" "Generic ComboBox references (non-namespaced; include other potential usages)"
{
    for dir in "${SEARCH_DIRS[@]}"; do
        if [ -d "$dir" ]; then
            grep -rn --line-number --color=never --include="*.rs" "ComboBox" "$dir" 2>/dev/null || true
        fi
    done
} | sed 's/^/* /' >> "$COMBOBOX_RESULTS_FILE" || true

# Produce counts & a small summary for convenience (append to both files)
{
    echo ""
    echo "----"
    echo "### Summary of counts"
} >> "$CSV_RESULTS_FILE"

{
    csv_split_count=0
    csv_join_count=0
    csv_comma_hint_count=0
    for dir in "${SEARCH_DIRS[@]}"; do
        if [ -d "$dir" ]; then
            # Count split occurrences with comma
            n=$(grep -rn --color=never --include="*.rs" -F ".split(" "$dir" 2>/dev/null || true | grep -E "," | wc -l || true)
            csv_split_count=$((csv_split_count + n))
            # Count join occurrences with comma
            j=$(grep -rn --color=never --include="*.rs" -F ".join(" "$dir" 2>/dev/null || true | grep -E "," | wc -l || true)
            csv_join_count=$((csv_join_count + j))
            # Count comma-separated textual hints
            t=$(grep -rn --color=never --include="*.rs" -E "comma[- ]?separated" "$dir" 2>/dev/null || true | wc -l || true)
            csv_comma_hint_count=$((csv_comma_hint_count + t))
        fi
    done
    echo ""
    echo "Results:"
    echo "- CSV .split occurrences (with comma): $csv_split_count"
    echo "- CSV .join occurrences (with comma): $csv_join_count"
    echo "- 'comma separated' textual hints: $csv_comma_hint_count"
} >> "$CSV_RESULTS_FILE"

{
    echo ""
    echo "----"
    echo "### Summary of counts"
} >> "$COMBOBOX_RESULTS_FILE"

{
    combobox_total=0
    combobox_egui=0
    combobox_from=0
    for dir in "${SEARCH_DIRS[@]}"; do
        if [ -d "$dir" ]; then
            e=$(grep -rn --color=never --include="*.rs" "egui::ComboBox" "$dir" 2>/dev/null || true | wc -l || true)
            f=$(grep -rn --color=never --include="*.rs" -E "ComboBox::from|ComboBox::from_id|ComboBox::from_id_salt|ComboBox::from_id_source" "$dir" 2>/dev/null || true | wc -l || true)
            t=$(grep -rn --color=never --include="*.rs" "ComboBox" "$dir" 2>/dev/null || true | wc -l || true)
            combobox_egui=$((combobox_egui + e))
            combobox_from=$((combobox_from + f))
            combobox_total=$((combobox_total + t))
        fi
    done
    echo ""
    echo "Results:"
    echo "- Total ComboBox references: $combobox_total"
    echo "- `egui::ComboBox` occurrences: $combobox_egui"
    echo "- `ComboBox::from*` occurrences: $combobox_from"
} >> "$COMBOBOX_RESULTS_FILE"

# Print final status & summary for the user
echo "Discovery complete."
echo "  CSV raw results file:  $CSV_RESULTS_FILE"
echo "  ComboBox raw results file: $COMBOBOX_RESULTS_FILE"
echo ""
echo "Quick summary (stdout):"
echo "------------------------"
echo "  CSV split (with comma), CSV join (with comma), 'comma-separated' hints in code documented in: $CSV_RESULTS_FILE"
echo "  ComboBox usage (egui::ComboBox/ComboBox::from/etc) documented in: $COMBOBOX_RESULTS_FILE"
echo ""
echo "Next steps:"
echo "  - Manually review the raw results and migrate relevant findings into the canonical inventory and checklist documents:"
echo "      - docs/explanation/csv_migration_inventory.md"
echo "      - docs/explanation/combobox_inventory.md"
echo ""
echo "  - Use the `--output-dir` option to place output in a different directory if needed."
echo ""
exit 0
