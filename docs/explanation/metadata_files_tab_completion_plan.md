# Metadata Files Tab Completion Plan

## Overview

The Metadata Editor's Files tab is missing the NPCs file path field. The Conditions file field already exists and works correctly. This plan adds the NPCs file path field following the same pattern.

## Current State Analysis

### Existing Infrastructure

- **Files Tab UI**: [`campaign_editor.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/campaign_editor.rs) lines 648-876
- **Edit Buffer**: `CampaignMetadataEditBuffer` has `conditions_file` (line 112) but no `npcs_file`
- **Domain Metadata**: `CampaignMetadata` in app state has both `npcs_file` and `conditions_file` fields
- **Existing Pattern**: All file fields follow the same pattern (text edit + browse button)

### Identified Issues

1. **Missing NPCs File Field in Edit Buffer**
   - `CampaignMetadataEditBuffer` struct (line 79-113) has `conditions_file` but no `npcs_file`
   - Need to add `pub npcs_file: String` field

2. **Missing NPCs File in Buffer Methods**
   - `from_metadata()` method (lines 117-149) doesn't copy `npcs_file`
   - `apply_to()` method (lines 152-182) doesn't apply `npcs_file`

3. **Missing NPCs File in Files Tab UI**
   - Files grid (lines 650-875) has Conditions File but no NPCs File
   - Need to add NPCs File row following the same pattern

## Implementation Steps

### Step 1: Add npcs_file to CampaignMetadataEditBuffer

**File**: `sdk/campaign_builder/src/campaign_editor.rs`

**Location**: Line 112 (after `dialogue_file`, before `conditions_file`)

**Add**:
```rust
pub npcs_file: String,
```

### Step 2: Update from_metadata() method

**File**: `sdk/campaign_builder/src/campaign_editor.rs`

**Location**: Line 146 (after `dialogue_file`, before `conditions_file`)

**Add**:
```rust
npcs_file: m.npcs_file.clone(),
```

### Step 3: Update apply_to() method

**File**: `sdk/campaign_builder/src/campaign_editor.rs`

**Location**: Line 180 (after `dialogue_file`, before `conditions_file`)

**Add**:
```rust
dest.npcs_file = self.npcs_file.clone();
```

### Step 4: Add NPCs File UI field to Files tab

**File**: `sdk/campaign_builder/src/campaign_editor.rs`

**Location**: Lines 851-852 (insert before "Conditions File:" label)

**Add**:
```rust
ui.label("NPCs File:");
ui.horizontal(|ui| {
    if ui
        .text_edit_singleline(&mut self.buffer.npcs_file)
        .changed()
    {
        self.has_unsaved_changes = true;
        *unsaved_changes = true;
    }
    if ui.button("📁").on_hover_text("Browse").clicked() {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("RON", &["ron"])
            .pick_file()
        {
            self.buffer.npcs_file = p.display().to_string();
            self.has_unsaved_changes = true;
            *unsaved_changes = true;
        }
    }
});
ui.end_row();

```

### Step 5: Update comment

**File**: `sdk/campaign_builder/src/campaign_editor.rs`

**Location**: Line 649

**Change from**:
```rust
// Files grid: items, spells, monsters, classes, races, characters, maps_dir, quests, dialogue, conditions
```

**Change to**:
```rust
// Files grid: items, spells, monsters, classes, races, characters, maps_dir, quests, dialogue, npcs, conditions
```

## Verification

### Build Verification

```bash
cd /Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder
cargo check
cargo clippy -- -D warnings
```

### Manual Testing

1. Run Campaign Builder: `cargo run --bin campaign_builder`
2. Navigate to Metadata tab
3. Click "Files" section
4. Verify "NPCs File:" field appears before "Conditions File:"
5. Edit the NPCs file path
6. Click browse button and select a file
7. Save metadata
8. Reload and verify NPCs file path persists

## Notes

- The Conditions file field already exists and works correctly
- This change only adds the missing NPCs file field
- No changes needed to domain `CampaignMetadata` (already has `npcs_file`)
- Follows exact same pattern as all other file fields
- Very low risk - simple addition following established pattern
