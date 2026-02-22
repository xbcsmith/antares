# Creatures Loading Pattern: Comparison

## Overview

This document compares how creatures are loaded in two parts of the Antares system:

1. **Game (Correct)**: Uses registry-based loading via `CreatureDatabase::load_from_registry()`
2. **Campaign Builder (Broken)**: Direct parsing without registry awareness

Understanding this comparison is critical for fixing the creatures editor.

---

## Pattern Comparison

### Game (Correct Pattern) ✅

**File**: `src/domain/visual/creature_database.rs`

**Function**: `pub fn load_from_registry(registry_path: &Path, campaign_root: &Path)`

**Data Flow**:

```
1. Read creatures.ron (text)
        ↓
2. Parse as Vec<CreatureReference>
        ↓
3. For each CreatureReference:
   - Get filepath from reference
   - Read filepath (e.g., "assets/creatures/goblin.ron")
   - Parse file content as CreatureDefinition
   - Validate: reference.id == creature.id
   - Add to database
        ↓
4. Return CreatureDatabase with all creatures loaded
```

**Pseudocode**:

```rust
pub fn load_from_registry(
    registry_path: &Path,
    campaign_root: &Path,
) -> Result<Self, CreatureDatabaseError> {
    // Step 1: Load registry
    let registry_contents = fs::read_to_string(registry_path)?;
    let references: Vec<CreatureReference> = ron::from_str(&registry_contents)?;

    // Step 2: Load full definitions
    let mut database = Self::new();
    for reference in references {
        // Resolve relative path
        let creature_path = campaign_root.join(&reference.filepath);

        // Read creature file
        let creature_contents = fs::read_to_string(&creature_path)?;
        let creature: CreatureDefinition = ron::from_str(&creature_contents)?;

        // Validate ID match
        if creature.id != reference.id {
            return Err(CreatureDatabaseError::ValidationError(...));
        }

        // Add to database
        database.add_creature(creature)?;
    }

    Ok(database)
}
```

**Key Characteristics**:
- ✅ Two-step process: registry → individual files
- ✅ Validates ID match between registry and file
- ✅ Loads only creatures referenced in registry
- ✅ Respects file organization (registry is source of truth)
- ✅ Individual files are self-contained definitions

---

### Campaign Builder (Broken Pattern) ❌

**File**: `sdk/campaign_builder/src/lib.rs`

**Function**: `fn load_creatures(&mut self)`

**Current Data Flow**:

```
1. Read creatures.ron (text)
        ↓
2. Parse as Vec<CreatureDefinition>  ❌ WRONG TYPE!
        ↓
3. RON parser fails because:
   - File contains CreatureReference fields
   - Struct expects CreatureDefinition fields
   - Unknown field "filepath" in CreatureDefinition
        ↓
4. Return error, creatures list remains empty
```

**Current Code**:

```rust
fn load_creatures(&mut self) {
    let creatures_file = self.campaign.creatures_file.clone();
    if let Some(ref dir) = self.campaign_dir {
        let creatures_path = dir.join(&creatures_file);
        if creatures_path.exists() {
            match fs::read_to_string(&creatures_path) {
                Ok(contents) => {
                    // ❌ Wrong type - file has CreatureReference, not CreatureDefinition
                    match ron::from_str::<Vec<antares::domain::visual::CreatureDefinition>>(
                        &contents,
                    ) {
                        Ok(creatures) => {
                            self.creatures = creatures;  // Never reached!
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to parse creatures: {}", e);
                            // creatures remains empty
                        }
                    }
                }
            }
        }
    }
}
```

**Key Problems**:
- ❌ Single-step parse (no registry awareness)
- ❌ Wrong target type (`CreatureDefinition` vs `CreatureReference`)
- ❌ No attempt to load individual files
- ❌ No ID validation
- ❌ File organization ignored

---

## Data Structure Comparison

### CreatureReference (Registry Entry)

**Size**: ~50 bytes per entry
**Total for 40 creatures**: ~2KB

```rust
pub struct CreatureReference {
    pub id: CreatureId,              // 4 bytes
    pub name: String,                // ~30 bytes (average)
    pub filepath: String,            // ~30 bytes (average)
}
```

**Example RON**:

```ron
CreatureReference(
    id: 1,
    name: "Goblin",
    filepath: "assets/creatures/goblin.ron",
),
```

**Purpose**: Lightweight index into creature files

---

### CreatureDefinition (Full Definition)

**Size**: ~5KB per creature (includes mesh data)
**Total for 40 creatures**: ~200KB

```rust
pub struct CreatureDefinition {
    pub id: CreatureId,
    pub name: String,
    pub meshes: Vec<MeshDefinition>,      // Heavy!
    pub mesh_transforms: Vec<MeshTransform>,  // Heavy!
    pub scale: f32,
    pub color_tint: Option<[f32; 4]>,
}
```

**Example RON**:

```ron
CreatureDefinition(
    id: 1,
    name: "Goblin",
    meshes: [
        MeshDefinition( ... mesh data ... ),
        MeshDefinition( ... mesh data ... ),
    ],
    mesh_transforms: [
        MeshTransform( ... transform data ... ),
    ],
    scale: 1.0,
    color_tint: None,
)
```

**Purpose**: Complete creature definition with all visual data

---

## File Organization Comparison

### Current (Correct) File Structure

```
campaigns/tutorial/
├── data/
│   ├── creatures.ron                          ← Registry (2KB)
│   ├── items.ron
│   ├── spells.ron
│   └── monsters.ron
├── assets/
│   ├── creatures/                            ← Individual definitions
│   │   ├── goblin.ron                        (5KB)
│   │   ├── dragon.ron                        (8KB)
│   │   ├── skeleton.ron                      (4KB)
│   │   ├── wolf.ron                          (3KB)
│   │   ├── orc.ron                           (5KB)
│   │   └── ... (35 more)
│   ├── portraits/
│   ├── tiles/
│   └── music/
└── maps/
    └── ...
```

**Size Breakdown**:
- Registry file: ~2KB
- Individual creature files: ~200KB total
- **Total**: ~202KB (reasonable, modular)

**Why This Is Better**:
- ✅ Registry is lightweight (fast to scan)
- ✅ Individual files are independent
- ✅ Easy to add/remove creatures
- ✅ Game loads only creatures in registry
- ✅ Asset organization matches code structure

---

## Step-by-Step Comparison: Loading Goblin

### Game Loading Process

```
Step 1: Read creatures.ron
└─> Contains: Vec<CreatureReference>

Step 2: Find Goblin reference
└─> CreatureReference {
      id: 1,
      name: "Goblin",
      filepath: "assets/creatures/goblin.ron"
    }

Step 3: Read assets/creatures/goblin.ron
└─> CreatureDefinition {
      id: 1,
      name: "Goblin",
      meshes: [...],
      mesh_transforms: [...],
      scale: 1.0,
      color_tint: None
    }

Step 4: Validate
└─> reference.id (1) == creature.id (1) ✓

Step 5: Add to database
└─> CreatureDatabase now contains goblin
```

**Result**: Goblin successfully loaded

---

### Campaign Builder (Current Broken Process)

```
Step 1: Read creatures.ron
└─> Contains: Vec<CreatureReference>

Step 2: Try to parse as Vec<CreatureDefinition>
└─> ERROR: Unknown field "filepath" in CreatureDefinition

Step 3: Parse fails, error message shown
└─> "Failed to parse creatures: unknown field `filepath`"

Step 4: Return error
└─> creatures list remains empty

Step 5: Creatures editor shows no creatures
└─> User sees empty list
```

**Result**: Goblin NOT loaded, editor is broken

---

## The Fix: Adapt Campaign Builder to Use Game Pattern

### Before (Broken)

```rust
// Single-step parse, wrong type
match ron::from_str::<Vec<CreatureDefinition>>(&contents) {
    Ok(creatures) => {
        self.creatures = creatures;  // Never happens!
    }
    Err(e) => {
        // This is what happens
        self.status_message = format!("Failed to parse creatures: {}", e);
    }
}
```

### After (Fixed)

```rust
// Step 1: Parse registry
match ron::from_str::<Vec<CreatureReference>>(&contents) {
    Ok(references) => {
        let mut creatures = Vec::new();

        // Step 2: Load each creature file
        for reference in references {
            let creature_path = dir.join(&reference.filepath);
            match fs::read_to_string(&creature_path) {
                Ok(creature_contents) => {
                    match ron::from_str::<CreatureDefinition>(&creature_contents) {
                        Ok(creature) => {
                            // Validate ID match
                            if creature.id == reference.id {
                                creatures.push(creature);  // Success!
                            } else {
                                eprintln!("ID mismatch in {}", reference.filepath);
                            }
                        }
                        Err(e) => eprintln!("Failed to parse creature: {}", e),
                    }
                }
                Err(e) => eprintln!("Failed to read creature file: {}", e),
            }
        }

        self.creatures = creatures;
    }
    Err(e) => {
        self.status_message = format!("Failed to parse creatures registry: {}", e);
    }
}
```

---

## Why This Pattern Exists

### Problem It Solves

**Before**: All creatures in one file
- Single goblin.ron containing 40 creatures
- File size: ~200KB
- Hard to manage individual creatures
- Any change requires editing entire file

**After**: Registry + individual files
- creatures.ron: lightweight index (~2KB)
- 40 individual files in assets/creatures/
- Easy to add/remove/edit individual creatures
- Registry decouples structure from content
- Scales to hundreds of creatures easily

### Why Game Uses This Pattern

1. **Performance**: Load only creatures needed
2. **Modularity**: Each creature is independent
3. **Asset Management**: Organize by type/region
4. **Parallel Loading**: Load multiple files concurrently
5. **Streaming**: Load creatures as areas become accessible
6. **Version Control**: Commit individual changes easily

---

## Validation Requirements

### Game (Strict)

When loading from registry:

```rust
if creature.id != reference.id {
    return Err(CreatureDatabaseError::ValidationError(
        reference.id,
        format!("ID mismatch: registry={}, file={}",
            reference.id, creature.id)
    ));
}
```

- ✅ Must match exactly
- ✅ Fails the entire load if mismatch
- ✅ Prevents out-of-sync data

### Campaign Builder (Should Match)

After fix:

```rust
if creature.id != reference.id {
    load_errors.push(format!(
        "ID mismatch for {}: registry={}, file={}",
        reference.filepath, reference.id, creature.id
    ));
} else {
    creatures.push(creature);
}
```

- ✅ Validates the same way
- ✅ Prevents invalid saves
- ✅ Provides clear error messages

---

## Summary Table

| Aspect | Game | Campaign Builder (Current) | Campaign Builder (Fixed) |
|--------|------|---------------------------|-------------------------|
| **Registry Parse** | `Vec<CreatureReference>` | N/A | `Vec<CreatureReference>` |
| **Individual Files** | Loads each | Ignored | Loads each |
| **ID Validation** | ✅ Strict | ❌ None | ✅ Strict |
| **File Organization** | Respected | Not supported | Respected |
| **Error Handling** | Detailed | Generic | Detailed |
| **Creatures Loaded** | All from registry | None | All from registry |

---

## Key Takeaway

The creatures editor fails because the campaign builder doesn't speak the same "language" as the game:

- **Game says**: "Here's a registry, load each creature from its file"
- **Campaign Builder says**: "Here's all creatures in one blob"

**The fix**: Make the campaign builder use the same two-step registry pattern as the game.

Once implemented, both will be in alignment, and the creatures editor will work correctly.
