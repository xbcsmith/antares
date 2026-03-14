#!/bin/bash
# SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
# SPDX-License-Identifier: Apache-2.0

# Creature editor documentation parity checks.
#
# Verifies that key navigation and action labels documented in
# docs/how-to/create_creatures.md are present in the active UI code paths.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DOC_FILE="$PROJECT_ROOT/docs/how-to/create_creatures.md"
APP_FILE="$PROJECT_ROOT/sdk/campaign_builder/src/lib.rs"
EDITOR_FILE="$PROJECT_ROOT/sdk/campaign_builder/src/creatures_editor.rs"

check_contains() {
    local file="$1"
    local needle="$2"
    local label="$3"

    if rg --fixed-strings --quiet -- "$needle" "$file"; then
        echo "PASS: $label"
    else
        echo "FAIL: $label"
        echo "  Missing text: $needle"
        echo "  File: $file"
        exit 1
    fi
}

echo "Creature editor doc parity validation"

echo "Checking documented navigation paths"
check_contains "$DOC_FILE" 'Tools -> Creature Editor' 'Doc mentions Tools creature editor path'
check_contains "$DOC_FILE" 'Tools -> Creature Templates...' 'Doc mentions Tools creature templates path'
check_contains "$APP_FILE" 'ui.button("🦎 Creature Editor")' 'Tools menu has Creature Editor button'
check_contains "$APP_FILE" 'ui.button("🐉 Creature Templates...")' 'Tools menu has Creature Templates button'
check_contains "$APP_FILE" 'EditorTab::Creatures' 'Creatures tab dispatch exists'

echo "Checking documented registry actions"
check_contains "$DOC_FILE" '`Register Asset`' 'Doc mentions Register Asset action'
check_contains "$DOC_FILE" '`Browse Templates`' 'Doc mentions Browse Templates action'
check_contains "$EDITOR_FILE" 'ui.button("📥 Register Asset")' 'Registry has Register Asset button'
check_contains "$EDITOR_FILE" 'ui.button("📋 Browse Templates")' 'Registry/edit has Browse Templates button'
check_contains "$EDITOR_FILE" 'OPEN_CREATURE_TEMPLATES_SENTINEL' 'Template sentinel integration exists'

echo "Checking documented edit-mode actions"
check_contains "$DOC_FILE" '`Validate Mesh`' 'Doc mentions Validate Mesh action'
check_contains "$DOC_FILE" '`Show Issues`' 'Doc mentions Show Issues action'
check_contains "$DOC_FILE" '`Save As...`' 'Doc mentions Save As action'
check_contains "$DOC_FILE" '`Export RON`' 'Doc mentions Export RON action'
check_contains "$DOC_FILE" '`Revert Changes`' 'Doc mentions Revert Changes action'
check_contains "$EDITOR_FILE" 'ui.button("🔍 Validate Mesh")' 'Mesh panel has Validate Mesh button'
check_contains "$EDITOR_FILE" 'ui.button("Show Issues")' 'Properties panel has Show Issues button'
check_contains "$EDITOR_FILE" 'ui.button("💾 Save As...")' 'Properties panel has Save As button'
check_contains "$EDITOR_FILE" 'ui.button("📋 Export RON")' 'Properties panel has Export RON button'
check_contains "$EDITOR_FILE" 'ui.button("↺ Revert Changes")' 'Properties panel has Revert Changes button'

echo "All creature editor doc parity checks passed."
