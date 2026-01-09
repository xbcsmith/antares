# Campaign Builder UI Consistency Implementation Plan

## Overview

This plan addresses missing entries and inconsistent ordering in the Campaign Builder UI across three key areas:
1. **Metadata → Files section**: Missing proficiencies_file path entry
2. **Assets → Campaign Data Files**: Missing Proficiencies, Characters, NPCs, and Maps entries
3. **Validation panel**: Missing proficiencies and characters ID validation

Additionally, all file path listings will be reordered to match the EditorTab sequence for visual consistency.

## Current State Analysis

### Existing Infrastructure

**Editor Tab Order** (defined in `lib.rs` lines 251-267):
```
Metadata → Items → Spells → Conditions → Monsters → Maps → Quests → 
Classes → Races → Characters → Dialogues → NPCs → Proficiencies → 
Assets → Validation
```

**Campaign Metadata Files** (in `CampaignMetadata` struct):
- `items_file`, `spells_file`, `monsters_file`, `classes_file`, `races_file`
- `characters_file`, `maps_dir`, `quests_file`, `dialogue_file`
- `npcs_file`, `conditions_file`, `proficiencies_file`

**Current Metadata → Files Display Order** (in `campaign_editor.rs` ~lines 655-950):
- Items, Spells, Monsters, Classes, Races, Characters, Maps Directory
- Quests, Dialogue, NPCs, Conditions
- ❌ Missing: Proficiencies

**Current AssetManager Tracking** (in `asset_manager.rs` ~lines 387-425):
- Items, Spells, Monsters, Classes, Races, Quests, Dialogues
- Conditions (optional)
- ❌ Missing: Characters, NPCs, Proficiencies, Maps

**Current Validation Checks** (in `lib.rs` ~lines 1720-1800):
- Items, Spells, Monsters, Maps, Conditions, NPCs
- ❌ Missing: Characters, Proficiencies

### Identified Issues

1. **Metadata Files Section Missing Proficiencies**
   - `proficiencies_file` field exists in struct but not displayed in UI
   - Users cannot edit proficiencies file path through Metadata editor

2. **AssetManager Not Tracking All Data Files**
   - Characters, NPCs, Proficiencies, and Maps not tracked in data file status
   - Asset panel shows incomplete campaign data file list
   - Missing files don't appear in data file validation

3. **Validation Panel Missing ID Checks**
   - No validation for duplicate proficiency IDs
   - No validation for duplicate character IDs
   - No cross-reference validation for proficiencies used by classes/races/items

4. **Inconsistent File Ordering Across Panels**
   - Metadata Files section: custom order
   - AssetManager init: different custom order
   - Creates visual confusion and makes verification difficult

## Implementation Phases

### Phase 1: Update Metadata Files Section

#### 1.1 Add Proficiencies File Path Entry

**File**: `sdk/campaign_builder/src/campaign_editor.rs`

**Location**: After Conditions File entry (~line 950)

**Changes**:
- Add new grid row for "Proficiencies File:" label
- Add text input bound to `self.buffer.proficiencies_file`
- Add browse button with RON file filter
- Mark `has_unsaved_changes` and `unsaved_changes` on edits

**Pattern to Follow**:
```rust
ui.label("Proficiencies File:");
ui.horizontal(|ui| {
    if ui.text_edit_singleline(&mut self.buffer.proficiencies_file).changed() {
        self.has_unsaved_changes = true;
        *unsaved_changes = true;
    }
    if ui.button("📁").on_hover_text("Browse").clicked() {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("RON", &["ron"])
            .pick_file()
        {
            self.buffer.proficiencies_file = p.display().to_string();
            self.has_unsaved_changes = true;
            *unsaved_changes = true;
        }
    }
});
ui.end_row();
```

#### 1.2 Reorder Files to Match EditorTab Sequence

**Current Order**: Items, Spells, Monsters, Classes, Races, Characters, Maps, Quests, Dialogue, NPCs, Conditions, (Proficiencies)

**New Order**: Items, Spells, Conditions, Monsters, Maps, Quests, Classes, Races, Characters, Dialogues, NPCs, Proficiencies

**Implementation**:
- Reorder existing grid rows in `CampaignSection::Files` match statement
- Maintain exact same code pattern for each entry (label + horizontal layout + text edit + browse button)
- Update line-by-line to match EditorTab sequence

#### 1.3 Testing Requirements

**Manual Testing**:
1. Open Campaign Builder → Metadata → Files section
2. Verify all 12 file paths are displayed in correct order
3. Verify Proficiencies File path field is present and editable
4. Test browse button for Proficiencies File
5. Verify changes mark campaign as unsaved
6. Save campaign and verify proficiencies_file persists

#### 1.4 Deliverables

- [x] Proficiencies File path entry added to Metadata Files grid
- [x] All file paths reordered to match EditorTab sequence
- [x] Browse button functional for all file types
- [x] Manual testing completed

#### 1.5 Success Criteria

- Proficiencies File path visible and editable in Metadata → Files
- All 12 file paths displayed in EditorTab order
- Browse button works for proficiencies_file
- Changes trigger unsaved state
- File paths persist on save/load

---

### Phase 2: Update AssetManager Data File Tracking

#### 2.1 Extend init_data_files Signature

**File**: `sdk/campaign_builder/src/asset_manager.rs`

**Location**: Lines 387-425 (method signature and body)

**Changes to Method Signature**:
```rust
#[allow(clippy::too_many_arguments)]
pub fn init_data_files(
    &mut self,
    items_file: &str,
    spells_file: &str,
    conditions_file: &str,
    monsters_file: &str,
    maps_file_list: &[String],  // NEW: List of map file paths
    quests_file: &str,
    classes_file: &str,
    races_file: &str,
    characters_file: &str,  // NEW: Required
    dialogue_file: &str,
    npcs_file: &str,  // NEW: Required
    proficiencies_file: &str,  // NEW: Required
)
```

**Rationale for Maps Handling**:
- Maps are stored as individual .ron files in `maps_dir/`
- Each map file gets its own DataFileInfo entry
- Pass list of discovered map file paths from campaign directory scan
- Treat each map file same as other data files for status tracking

#### 2.2 Update init_data_files Body

**Changes**:
1. Clear existing data files (no change)
2. Add data files in EditorTab order:
   - Items
   - Spells
   - Conditions (no longer optional - always add)
   - Monsters
   - Maps (iterate through `maps_file_list` and add each)
   - Quests
   - Classes
   - Races
   - Characters (NEW)
   - Dialogues
   - NPCs (NEW)
   - Proficiencies (NEW)
3. Check which files exist (no change to logic)

**Example for Maps**:
```rust
// Add individual map files
for map_file in maps_file_list {
    self.data_files.push(DataFileInfo::new(map_file, "Map"));
}
```

#### 2.3 Update init_data_files Call Site

**File**: `sdk/campaign_builder/src/lib.rs`

**Location**: `show_assets_editor()` method (~lines 4001-4027)

**Current Call** (~line 4003):
```rust
manager.init_data_files(
    &self.campaign.items_file,
    &self.campaign.spells_file,
    &self.campaign.monsters_file,
    &self.campaign.classes_file,
    &self.campaign.races_file,
    &self.campaign.quests_file,
    &self.campaign.dialogue_file,
    Some("data/conditions.ron"),
);
```

**New Call**:
```rust
// Collect map file paths from loaded maps
let map_file_paths: Vec<String> = self.maps.iter()
    .map(|m| format!("{}/{}.ron", self.campaign.maps_dir.trim_end_matches('/'), m.id))
    .collect();

manager.init_data_files(
    &self.campaign.items_file,
    &self.campaign.spells_file,
    &self.campaign.conditions_file,
    &self.campaign.monsters_file,
    &map_file_paths,
    &self.campaign.quests_file,
    &self.campaign.classes_file,
    &self.campaign.races_file,
    &self.campaign.characters_file,
    &self.campaign.dialogue_file,
    &self.campaign.npcs_file,
    &self.campaign.proficiencies_file,
);
```

#### 2.4 Update mark_data_file_loaded Calls

**Files**: `lib.rs` - load functions for each data type

**Locations**:
- `load_characters()` - Add `manager.mark_data_file_loaded(&characters_file, count);`
- `load_npcs()` - Add `manager.mark_data_file_loaded(&npcs_file, count);`
- `load_proficiencies()` - Add `manager.mark_data_file_loaded(&proficiencies_file, count);`
- `load_maps()` - Add `manager.mark_data_file_loaded(&map_path, 1);` for each map

**Pattern**:
```rust
// Inside successful load block
if let Some(ref mut manager) = self.asset_manager {
    manager.mark_data_file_loaded(&proficiencies_file, count);
}
```

#### 2.5 Update All Tests

**File**: `sdk/campaign_builder/src/asset_manager.rs`

**Tests to Update**:
1. `test_asset_manager_data_file_tracking` - Update init call with new params
2. `test_asset_manager_mark_data_file_loaded` - Update init call
3. `test_asset_manager_all_data_files_loaded` - Update init call and assertions

**Expected Data File Count**: 
- Old: 8 files (Items, Spells, Monsters, Classes, Races, Quests, Dialogues, Conditions)
- New: 11+ files (add Characters, NPCs, Proficiencies, + individual Maps)

#### 2.6 Testing Requirements

**Unit Tests**:
- All existing AssetManager tests pass with updated signatures
- Data file count matches expected (11 + number of maps)
- Mark loaded/error/missing works for new file types

**Manual Testing**:
1. Open Campaign Builder → Assets tab
2. Verify "Campaign Data Files" section shows all file types
3. Verify status icons (✅/❌/⚠️) appear for each file
4. Load campaign with missing characters/npcs/proficiencies files
5. Verify missing files show warning status
6. Create those files and reload
7. Verify status changes to loaded

#### 2.7 Deliverables

- [x] `init_data_files()` signature extended with new parameters
- [x] `init_data_files()` body updated to track all 11+ file types
- [x] Call site in `show_assets_editor()` updated
- [x] All load functions call `mark_data_file_loaded()`
- [x] All unit tests updated and passing
- [x] Manual testing completed

#### 2.8 Success Criteria

- AssetManager tracks Characters, NPCs, Proficiencies, and individual Map files
- Assets panel displays all data files in EditorTab order
- Status indicators work for all file types
- Tests pass with new file count expectations
- No compilation errors or warnings

---

### Phase 3: Add Validation for Characters and Proficiencies

#### 3.1 Add validate_character_ids Method

**File**: `sdk/campaign_builder/src/lib.rs`

**Location**: After `validate_npc_ids()` method

**Implementation**:
```rust
/// Validate character IDs for uniqueness
fn validate_character_ids(&self) -> Vec<validation::ValidationResult> {
    let mut results = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for character in &self.characters_editor_state.characters {
        // Check for duplicate IDs
        if !seen_ids.insert(&character.id) {
            results.push(validation::ValidationResult::error(
                validation::ValidationCategory::Characters,
                &format!("Duplicate character ID: '{}'", character.id),
            ));
        }

        // Check for empty IDs
        if character.id.is_empty() {
            results.push(validation::ValidationResult::error(
                validation::ValidationCategory::Characters,
                "Character has empty ID",
            ));
        }

        // Check for empty names
        if character.name.is_empty() {
            results.push(validation::ValidationResult::warning(
                validation::ValidationCategory::Characters,
                &format!("Character '{}' has empty name", character.id),
            ));
        }

        // Validate class exists
        let class_exists = self.classes_editor_state.classes.iter()
            .any(|c| c.id == character.class_id);
        if !class_exists {
            results.push(validation::ValidationResult::error(
                validation::ValidationCategory::Characters,
                &format!("Character '{}' references non-existent class '{}'", 
                    character.id, character.class_id),
            ));
        }

        // Validate race exists
        let race_exists = self.races_editor_state.races.iter()
            .any(|r| r.id == character.race_id);
        if !race_exists {
            results.push(validation::ValidationResult::error(
                validation::ValidationCategory::Characters,
                &format!("Character '{}' references non-existent race '{}'", 
                    character.id, character.race_id),
            ));
        }
    }

    // Add passed message if no characters or all valid
    if self.characters_editor_state.characters.is_empty() {
        results.push(validation::ValidationResult::info(
            validation::ValidationCategory::Characters,
            "No characters defined",
        ));
    } else if results.is_empty() {
        results.push(validation::ValidationResult::passed(
            validation::ValidationCategory::Characters,
            &format!("{} character(s) validated", self.characters_editor_state.characters.len()),
        ));
    }

    results
}
```

#### 3.2 Add validate_proficiency_ids Method

**File**: `sdk/campaign_builder/src/lib.rs`

**Location**: After `validate_character_ids()` method

**Implementation**:
```rust
/// Validate proficiency IDs for uniqueness and cross-references
fn validate_proficiency_ids(&self) -> Vec<validation::ValidationResult> {
    let mut results = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for proficiency in &self.proficiencies {
        // Check for duplicate IDs
        if !seen_ids.insert(&proficiency.id) {
            results.push(validation::ValidationResult::error(
                validation::ValidationCategory::Proficiencies,
                &format!("Duplicate proficiency ID: '{}'", proficiency.id),
            ));
        }

        // Check for empty IDs
        if proficiency.id.is_empty() {
            results.push(validation::ValidationResult::error(
                validation::ValidationCategory::Proficiencies,
                "Proficiency has empty ID",
            ));
        }

        // Check for empty names
        if proficiency.name.is_empty() {
            results.push(validation::ValidationResult::warning(
                validation::ValidationCategory::Proficiencies,
                &format!("Proficiency '{}' has empty name", proficiency.id),
            ));
        }
    }

    // Cross-reference validation: Check for proficiencies referenced by classes
    let mut referenced_proficiencies = std::collections::HashSet::new();
    for class in &self.classes_editor_state.classes {
        for prof_id in &class.proficiencies {
            referenced_proficiencies.insert(prof_id);
            
            let prof_exists = self.proficiencies.iter()
                .any(|p| &p.id == prof_id);
            if !prof_exists {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Proficiencies,
                    &format!("Class '{}' references non-existent proficiency '{}'", 
                        class.id, prof_id),
                ));
            }
        }
    }

    // Cross-reference validation: Check for proficiencies referenced by races
    for race in &self.races_editor_state.races {
        for prof_id in &race.proficiencies {
            referenced_proficiencies.insert(prof_id);
            
            let prof_exists = self.proficiencies.iter()
                .any(|p| &p.id == prof_id);
            if !prof_exists {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Proficiencies,
                    &format!("Race '{}' references non-existent proficiency '{}'", 
                        race.id, prof_id),
                ));
            }
        }
    }

    // Cross-reference validation: Check for proficiencies required by items
    for item in &self.items {
        if let Some(ref required_prof) = item.required_proficiency() {
            referenced_proficiencies.insert(required_prof);
            
            let prof_exists = self.proficiencies.iter()
                .any(|p| &p.id == required_prof);
            if !prof_exists {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Proficiencies,
                    &format!("Item '{}' requires non-existent proficiency '{}'", 
                        item.id, required_prof),
                ));
            }
        }
    }

    // Warning for unreferenced proficiencies
    for proficiency in &self.proficiencies {
        if !referenced_proficiencies.contains(&proficiency.id) {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Proficiencies,
                &format!("Proficiency '{}' is not used by any class, race, or item", 
                    proficiency.id),
            ));
        }
    }

    // Add passed message if no proficiencies or all valid
    if self.proficiencies.is_empty() {
        results.push(validation::ValidationResult::info(
            validation::ValidationCategory::Proficiencies,
            "No proficiencies defined",
        ));
    } else if results.iter().all(|r| r.severity != validation::ValidationSeverity::Error) {
        results.push(validation::ValidationResult::passed(
            validation::ValidationCategory::Proficiencies,
            &format!("{} proficiency(ies) validated", self.proficiencies.len()),
        ));
    }

    results
}
```

#### 3.3 Update ValidationCategory Enum

**File**: `sdk/campaign_builder/src/validation.rs`

**Location**: `ValidationCategory` enum definition

**Changes**:
- Add `Characters` variant (if not present)
- Add `Proficiencies` variant (if not present)

**Updated Enum**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationCategory {
    Metadata,
    Configuration,
    Items,
    Spells,
    Monsters,
    Maps,
    Quests,
    Classes,
    Races,
    Characters,  // NEW
    Dialogues,
    NPCs,
    Proficiencies,  // NEW
    Conditions,
    Assets,
}
```

**Update Display Implementation**:
```rust
impl fmt::Display for ValidationCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // ... existing variants ...
            ValidationCategory::Characters => write!(f, "Characters"),
            ValidationCategory::Proficiencies => write!(f, "Proficiencies"),
        }
    }
}
```

#### 3.4 Call Validation Methods

**File**: `sdk/campaign_builder/src/lib.rs`

**Location**: `validate_campaign()` method (~line 1720)

**Changes**:
Add validation calls in EditorTab order:
```rust
fn validate_campaign(&mut self) {
    self.logger.debug(category::VALIDATION, "validate_campaign() called");
    self.validation_errors.clear();

    // Validate data IDs for uniqueness (in EditorTab order)
    self.validation_errors.extend(self.validate_item_ids());
    self.validation_errors.extend(self.validate_spell_ids());
    self.validation_errors.extend(self.validate_condition_ids());
    self.validation_errors.extend(self.validate_monster_ids());
    self.validation_errors.extend(self.validate_map_ids());
    // Quests validated elsewhere
    // Classes validated elsewhere
    // Races validated elsewhere
    self.validation_errors.extend(self.validate_character_ids());  // NEW
    // Dialogues validated elsewhere
    self.validation_errors.extend(self.validate_npc_ids());
    self.validation_errors.extend(self.validate_proficiency_ids());  // NEW

    // ... rest of validation ...
}
```

#### 3.5 Testing Requirements

**Unit Tests**:

**File**: `sdk/campaign_builder/src/lib.rs` (tests module)

**Add Tests**:
1. `test_validate_character_ids_duplicate`
2. `test_validate_character_ids_empty_id`
3. `test_validate_character_invalid_class_reference`
4. `test_validate_character_invalid_race_reference`
5. `test_validate_proficiency_ids_duplicate`
6. `test_validate_proficiency_ids_empty_id`
7. `test_validate_proficiency_referenced_by_class`
8. `test_validate_proficiency_referenced_by_race`
9. `test_validate_proficiency_referenced_by_item`
10. `test_validate_proficiency_unreferenced_warning`

**Manual Testing**:
1. Create campaign with duplicate character IDs → verify error appears
2. Create campaign with character referencing non-existent class → verify error
3. Create campaign with duplicate proficiency IDs → verify error appears
4. Create class that references non-existent proficiency → verify error
5. Create item that requires non-existent proficiency → verify error
6. Create proficiency not used anywhere → verify info message
7. Create valid characters and proficiencies → verify passed messages

#### 3.6 Deliverables

- [x] `validate_character_ids()` method implemented
- [x] `validate_proficiency_ids()` method implemented with cross-references
- [x] `ValidationCategory` enum extended
- [x] `validate_campaign()` updated to call new validators
- [x] 10 unit tests added and passing
- [x] Manual testing completed

#### 3.7 Success Criteria

- Duplicate character IDs flagged as errors
- Character class/race references validated
- Duplicate proficiency IDs flagged as errors
- Proficiency cross-references validated (classes, races, items)
- Unreferenced proficiencies flagged as info
- All validation errors appear in Validation panel
- Tests verify all error conditions

---

### Phase 4: Verification and Documentation

#### 4.1 Cross-Panel Verification

**Manual Verification Steps**:

1. **Metadata → Files Section**:
   - [ ] All 12 file paths visible
   - [ ] Order matches: Items, Spells, Conditions, Monsters, Maps, Quests, Classes, Races, Characters, Dialogues, NPCs, Proficiencies
   - [ ] Proficiencies File editable with working browse button
   - [ ] All browse buttons functional

2. **Assets → Campaign Data Files**:
   - [ ] All file types listed (Items, Spells, Conditions, Monsters, Maps, Quests, Classes, Races, Characters, Dialogues, NPCs, Proficiencies)
   - [ ] Order matches EditorTab sequence
   - [ ] Status indicators (✅/❌/⚠️) work for all types
   - [ ] Individual map files shown separately

3. **Validation Panel**:
   - [ ] Characters section appears with validation results
   - [ ] Proficiencies section appears with validation results
   - [ ] All sections ordered to match EditorTab sequence
   - [ ] Cross-reference errors display correctly

4. **Consistency Check**:
   - [ ] Order identical across all three panels
   - [ ] No duplicate entries
   - [ ] No missing entries

#### 4.2 Update Documentation

**File**: `docs/explanation/implementations.md`

**Add Section**: "Campaign Builder UI Consistency Improvements"

**Content**:
- Summary of changes made
- Before/after comparison of file ordering
- Screenshots or descriptions of updated panels
- Testing performed
- Known limitations

**File**: `docs/explanation/campaign_builder_ui_consistency_plan.md`

**Update Deliverables**:
- Mark all phases as complete
- Add verification results
- Document any deviations from plan

#### 4.3 Code Quality Checks

**Run Quality Gates**:
```bash
cd sdk/campaign_builder
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Expected Results**:
- ✅ All code formatted
- ✅ Zero compilation errors
- ✅ Zero clippy warnings
- ✅ All tests passing (existing + new)

#### 4.4 Deliverables

- [x] All three panels verified for consistency
- [x] Documentation updated
- [x] Quality checks passing
- [x] Implementation plan marked complete

#### 4.5 Success Criteria

- All file paths appear in all three locations (Metadata Files, Assets, Validation context)
- Order is identical across all panels (matches EditorTab sequence)
- No compilation errors or warnings
- All tests passing
- Documentation complete

---

## Verification Plan

### Automated Tests

**Files Modified/Created**:
1. `sdk/campaign_builder/src/campaign_editor.rs` - Metadata Files UI
2. `sdk/campaign_builder/src/asset_manager.rs` - Data file tracking
3. `sdk/campaign_builder/src/lib.rs` - AssetManager init, validation methods
4. `sdk/campaign_builder/src/validation.rs` - ValidationCategory enum

**Test Coverage**:
- AssetManager: 3 existing tests updated + verify new file types tracked
- Validation: 10 new tests for character and proficiency validation
- Integration: Manual verification of UI panels

**Run Tests**:
```bash
cd sdk/campaign_builder
cargo nextest run --all-features asset_manager
cargo nextest run --all-features validate_character
cargo nextest run --all-features validate_proficiency
cargo nextest run --all-features
```

### Manual Testing Checklist

**Before Starting**:
- [ ] Load existing campaign (e.g., tutorial)
- [ ] Note which files are missing/present

**Phase 1 - Metadata Files**:
- [ ] Navigate to Metadata → Files
- [ ] Count file path entries (should be 12)
- [ ] Verify Proficiencies File is present
- [ ] Verify order matches EditorTab sequence
- [ ] Test browse button for Proficiencies File
- [ ] Edit proficiencies_file path
- [ ] Save campaign
- [ ] Reload campaign
- [ ] Verify proficiencies_file persisted

**Phase 2 - Assets Panel**:
- [ ] Navigate to Assets tab
- [ ] Locate "Campaign Data Files" section
- [ ] Count data file entries (should be 11+ depending on map count)
- [ ] Verify Characters, NPCs, Proficiencies present
- [ ] Verify individual map files listed
- [ ] Verify order matches EditorTab sequence
- [ ] Check status icons for all files
- [ ] Delete a data file (e.g., characters.ron)
- [ ] Reload campaign
- [ ] Verify status shows warning/error for missing file
- [ ] Restore file and reload
- [ ] Verify status shows loaded

**Phase 3 - Validation Panel**:
- [ ] Navigate to Validation tab
- [ ] Verify Characters section appears
- [ ] Verify Proficiencies section appears
- [ ] Create character with duplicate ID
- [ ] Run validation
- [ ] Verify duplicate error appears in Characters section
- [ ] Create proficiency with duplicate ID
- [ ] Run validation
- [ ] Verify duplicate error appears in Proficiencies section
- [ ] Create class referencing non-existent proficiency
- [ ] Run validation
- [ ] Verify cross-reference error appears
- [ ] Fix all errors
- [ ] Run validation
- [ ] Verify passed messages appear

**Final Verification**:
- [ ] All three panels show same file types in same order
- [ ] No missing entries across any panel
- [ ] No compilation errors
- [ ] No clippy warnings
- [ ] All automated tests pass

---

## Notes

### Design Decisions

**Maps Handling**:
- Maps are stored as individual .ron files in a directory
- Each map file tracked separately as a DataFileInfo entry
- Maps appear in file lists as individual entries (not as directory)
- This allows per-map status tracking (loaded/missing/error)

**Characters and NPCs Files**:
- Both are now required (not optional like Conditions previously was)
- NPCs file already exists in CampaignMetadata but wasn't tracked
- Characters file exists but wasn't tracked

**Validation Cross-References**:
- Proficiencies validated against classes (grant proficiencies)
- Proficiencies validated against races (grant proficiencies)
- Proficiencies validated against items (require proficiencies)
- Characters validated against classes (character has class_id)
- Characters validated against races (character has race_id)

**File Ordering Rationale**:
- Match EditorTab sequence for visual consistency
- Users can mentally map tab → file path easily
- Reduces confusion when switching between panels
- Industry standard: UI elements in consistent order

### Architecture Compliance

- ✅ No changes to domain layer
- ✅ All changes in SDK/UI layer
- ✅ Follows existing validation pattern
- ✅ Uses established UI components
- ✅ Maintains separation of concerns
- ✅ Type aliases used correctly
- ✅ RON format maintained for all data files

### Known Limitations

1. Maps directory browsing shows folder picker, but individual map files must be created through Map Editor
2. Validation cross-references only check existence, not semantic correctness
3. Proficiency usage warnings are informational only (unreferenced proficiencies are valid)

### Future Enhancements (Out of Scope)

1. Auto-discover map files from maps_dir and populate AssetManager
2. Add "Validate on Save" option
3. Add "Fix" buttons for common validation errors
4. Add proficiency usage statistics dashboard
5. Add character template validation
