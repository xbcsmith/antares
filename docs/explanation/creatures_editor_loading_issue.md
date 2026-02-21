# Creatures Editor Loading Issue - Analysis and Solution

## Executive Summary

The Creatures Editor is unable to load creatures from the tutorial campaign's `creatures.ron` file. The root cause is a **type mismatch**: the campaign builder attempts to parse `creatures.ron` as `Vec<CreatureDefinition>`, but the file actually contains `Vec<CreatureReference>` entries.

The system uses **registry-based loading** (game architecture pattern), where:
1. A lightweight registry file (`creatures.ron`) contains only references (ID, name, filepath)
2. Individual creature definition files (`assets/creatures/*.ron`) contain full definitions
3. The loader reads the registry, then loads each individual file

The campaign builder's `load_creatures()` function does not implement this two-step process.

---

## Current Architecture

### Data Structures

**CreatureReference** (in `src/domain/visual/mod.rs`):
```rust
pub struct CreatureReference {
    pub id: CreatureId,
    pub name: String,
    pub filepath: String,  // e.g., "assets/creatures/goblin.ron"
}
```

**CreatureDefinition** (in `src/domain/visual/mod.rs`):
```rust
pub struct CreatureDefinition {
    pub id: CreatureId,
    pub name: String,
    pub meshes: Vec<MeshDefinition>,
    pub mesh_transforms: Vec<MeshTransform>,
    pub scale: f32,
    pub color_tint: Option<[f32; 4]>,
}
```

### Files in Tutorial Campaign

**Registry File** (`campaigns/tutorial/data/creatures.ron`):
- Contains ~40 `CreatureReference` entries
- Lightweight (<5KB)
- Maps creature IDs to file paths
- Organized by category (Monsters, NPCs, Templates, Variants)

**Individual Creature Files** (`campaigns/tutorial/assets/creatures/*.ron`):
- 40+ individual files (goblin.ron, dragon.ron, etc.)
- Each contains a complete `CreatureDefinition`
- Contains all mesh data and transforms

### How the Game Loads Creatures

The game uses `CreatureDatabase::load_from_registry()` in `src/domain/visual/creature_database.rs`:

1. **Step 1**: Parse `creatures.ron` as `Vec<CreatureReference>`
2. **Step 2**: For each reference, load the file at `campaign_root + reference.filepath`
3. **Step 3**: Parse each file as `CreatureDefinition`
4. **Step 4**: Add to database with validation

This is eager loading - all creatures are loaded at campaign startup.

---

## The Problem

### Current Campaign Builder Implementation

In `sdk/campaign_builder/src/lib.rs`, the `load_creatures()` function:

```rust
fn load_creatures(&mut self) {
    let creatures_file = self.campaign.creatures_file.clone();
    if let Some(ref dir) = self.campaign_dir {
        let creatures_path = dir.join(&creatures_file);
        if creatures_path.exists() {
            match fs::read_to_string(&creatures_path) {
                Ok(contents) => {
                    match ron::from_str::<Vec<antares::domain::visual::CreatureDefinition>>(
                        &contents,  // ❌ WRONG TYPE!
                    ) {
                        Ok(creatures) => {
                            self.creatures = creatures;
                            // ...
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to parse creatures: {}", e);
                        }
                    }
                }
            }
        }
    }
}
```

**The Issue**: Line with `ron::from_str::<Vec<CreatureDefinition>>` fails because:
- The file contains `CreatureReference` entries
- Each entry points to a separate file with the actual definition
- No attempt is made to load the individual files

### Error Flow

1. User opens campaign with creatures.ron registry file
2. `load_creatures()` tries to parse as `CreatureDefinition`
3. RON parser fails on unknown `filepath` field (not part of `CreatureDefinition`)
4. Error message: "Failed to parse creatures: ..."
5. Creatures list remains empty
6. Creatures Editor shows empty list

---

## Solution Architecture

### Phase 1: Registry-Based Loading

Implement two-step loading mirroring `CreatureDatabase::load_from_registry()`:

```
campaigns/tutorial/
├── data/
│   └── creatures.ron          ← Registry file (Vec<CreatureReference>)
└── assets/creatures/
    ├── goblin.ron             ← Individual definitions
    ├── dragon.ron
    ├── skeleton.ron
    └── ... (40+ more)
```

### Phase 2: Campaign Builder Changes Required

**File**: `sdk/campaign_builder/src/lib.rs`

**Function**: `load_creatures()`

Changes:
1. Parse `creatures.ron` as `Vec<CreatureReference>` instead of `Vec<CreatureDefinition>`
2. For each reference, load the individual creature file
3. Combine all creature definitions into the editor's list
4. Store mapping of creature ID → filepath for later use

**Function**: `save_creatures()`

Changes:
1. When saving, write **both**:
   - Registry file (creatures.ron) with all CreatureReference entries
   - Individual creature files in assets/creatures/
2. Maintain directory structure (create assets/creatures/ if needed)
3. Use creature ID as filename or preserve existing names

### Phase 3: Creatures Editor Enhancement

**File**: `sdk/campaign_builder/src/creatures_editor.rs`

The editor currently works with `Vec<CreatureDefinition>` in memory, which is correct. Changes needed:

1. **Add metadata**: Track original filename for each creature
   - When saving, use original name (e.g., goblin.ron) not ID-based
   - Allows user to rename creatures without breaking references

2. **Display filepath**: Show where each creature is defined
   - UI feedback: "Edit creature → Save to assets/creatures/goblin.ron"

3. **Validate references**: Ensure creature ID matches registry entry
   - Prevent ID mismatches like the game does

---

## Implementation Plan

### Step 1: Add Creatures Manager Integration

The campaign builder has a `CreaturesManager` (in `sdk/campaign_builder/src/creatures_manager.rs`) that handles:
- Loading registry file as `Vec<CreatureReference>`
- Validation (ID ranges, duplicates, categories)
- Dirty flag tracking

**Current limitation**: `CreaturesManager` works with `CreatureReference`, not full definitions.

**Solution**: Add helper to load full definitions:

```rust
// In creatures_manager.rs or creatures_editor.rs
pub fn load_full_definitions(
    references: &[CreatureReference],
    campaign_root: &Path,
) -> Result<Vec<CreatureDefinition>, CreatureLoadError> {
    let mut creatures = Vec::new();
    for reference in references {
        let filepath = campaign_root.join(&reference.filepath);
        let content = fs::read_to_string(&filepath)?;
        let creature = ron::from_str::<CreatureDefinition>(&content)?;

        // Validate ID match
        if creature.id != reference.id {
            return Err(CreatureLoadError::IdMismatch { ... });
        }

        creatures.push(creature);
    }
    Ok(creatures)
}
```

### Step 2: Update load_creatures()

```rust
fn load_creatures(&mut self) {
    let creatures_file = self.campaign.creatures_file.clone();
    if let Some(ref dir) = self.campaign_dir {
        let creatures_path = dir.join(&creatures_file);
        if creatures_path.exists() {
            // Step 1: Load registry as Vec<CreatureReference>
            match fs::read_to_string(&creatures_path) {
                Ok(contents) => {
                    match ron::from_str::<Vec<CreatureReference>>(&contents) {
                        Ok(references) => {
                            // Step 2: Load full definitions for each reference
                            match load_full_definitions(&references, dir) {
                                Ok(creatures) => {
                                    let count = creatures.len();
                                    self.creatures = creatures;
                                    self.status_message =
                                        format!("Loaded {} creatures", count);
                                }
                                Err(e) => {
                                    self.status_message =
                                        format!("Failed to load creature files: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            self.status_message =
                                format!("Failed to parse creatures registry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    self.status_message =
                        format!("Failed to read creatures file: {}", e);
                }
            }
        }
    }
}
```

### Step 3: Update save_creatures()

```rust
fn save_creatures(&mut self) -> Result<(), String> {
    if let Some(ref dir) = self.campaign_dir {
        // Create registry entries from loaded creatures
        let references: Vec<CreatureReference> = self.creatures
            .iter()
            .map(|creature| CreatureReference {
                id: creature.id,
                name: creature.name.clone(),
                filepath: format!("assets/creatures/{}.ron",
                    creature.name.to_lowercase().replace(" ", "_")),
            })
            .collect();

        // Save registry file
        let registry_path = dir.join(&self.campaign.creatures_file);
        if let Some(parent) = registry_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let registry_ron = ron::ser::to_string_pretty(&references,
            ron::ser::PrettyConfig::new().struct_names(false))?;
        fs::write(&registry_path, registry_ron)?;

        // Save individual creature files
        let creatures_dir = dir.join("assets/creatures");
        fs::create_dir_all(&creatures_dir)?;

        let creature_ron_config = ron::ser::PrettyConfig::new()
            .struct_names(false)
            .enumerate_arrays(false);

        for (reference, creature) in references.iter().zip(self.creatures.iter()) {
            let creature_path = dir.join(&reference.filepath);
            let creature_content = ron::ser::to_string_pretty(creature,
                creature_ron_config.clone())?;
            fs::write(&creature_path, creature_content)?;
        }

        self.unsaved_changes = true;
        Ok(())
    } else {
        Err("No campaign directory set".to_string())
    }
}
```

---

## Testing Strategy

### Unit Tests

1. **Registry Loading**
   - Parse valid registry file → correct count of references
   - Parse registry with duplicate IDs → error
   - Load full definitions → all creatures present

2. **File Path Resolution**
   - Relative paths resolved correctly from campaign root
   - Missing creature files → error with filename
   - ID mismatch detection → validation error

3. **Save/Load Round-Trip**
   - Load creatures → save → load again → identical data
   - Registry and individual files both present after save
   - Old data correctly replaced (no orphaned files)

### Integration Tests

1. **Tutorial Campaign**
   - Load tutorial campaign → 40 creatures loaded
   - All creatures match those in tutorial/assets/creatures/
   - Save campaign → registry and files preserved

2. **New Campaign**
   - Create new campaign → create creatures → save
   - Registry file created with correct structure
   - Individual files created with correct content

---

## Migration Path

### For Existing Campaigns

Campaigns using the old "all creatures in one file" format must be migrated:

1. **Before**: `creatures.ron` contains `Vec<CreatureDefinition>` (embedded data)
2. **After**: `creatures.ron` contains `Vec<CreatureReference>` (registry), data split into individual files

**Migration steps** (automated):
1. Read old creatures.ron as `Vec<CreatureDefinition>`
2. Create registry: for each creature, generate filename and create reference
3. Create assets/creatures/ directory
4. Write individual files (one per creature)
5. Write new registry file

---

## Summary of Changes

| File | Change | Reason |
|------|--------|--------|
| `sdk/campaign_builder/src/lib.rs` | `load_creatures()` | Implement registry-based loading |
| `sdk/campaign_builder/src/lib.rs` | `save_creatures()` | Save both registry and individual files |
| `sdk/campaign_builder/src/creatures_editor.rs` | Minor (optional) | Add filepath tracking for UX |
| `sdk/campaign_builder/src/creatures_manager.rs` | Add helper | Load full definitions from registry |

**No changes needed**:
- `src/domain/visual/mod.rs` - Already has correct structures
- `src/domain/visual/creature_database.rs` - Already implements pattern correctly
- Game loading code - Already working correctly

---

## Conclusion

The root cause is clear: the campaign builder doesn't implement the registry-based loading pattern that the game uses. The solution is straightforward: adapt the campaign builder's creature loading to match how the game's `CreatureDatabase` loads creatures.

This brings the campaign builder into alignment with the game's architecture and allows the creatures editor to successfully load and manipulate creatures from the tutorial campaign.
