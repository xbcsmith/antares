# Tutorial Campaign Procedural Mesh Integration Plan

## Overview

This plan outlines the integration of procedural mesh-based creature visuals into the tutorial campaign, replacing sprite placeholders with 3D procedural meshes. The implementation creates a lightweight creature registry that references individual mesh files, maps monsters and NPCs to creature visuals, and ensures proper loading and rendering within the campaign.

**Scope**: Update tutorial campaign (`campaigns/tutorial/`) to use procedural mesh creatures from `assets/creatures/` directory.

**Key Changes**:

- Create lightweight creature registry (`data/creatures.ron`) with file references instead of embedded mesh data
- Add `CreatureReference` struct to domain layer for registry entries
- Implement eager loading of creature meshes at campaign startup
- Map 11 monster types to 32 available creature meshes
- Update NPCs to use procedural meshes instead of sprites
- Maintain backward compatibility with sprite fallback system

**Architecture Decision**: Registry-Based Creature Loading

```
┌─────────────────────────────────────────────────────────────┐
│ PREVIOUS APPROACH (Rejected)                                │
│ - Single creatures.ron file with embedded MeshDefinition    │
│ - File size: >1MB with all mesh data inline                 │
│ - Hard to edit individual creatures                         │
│ - Single point of failure for all creatures                 │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ NEW APPROACH (Implemented)                                   │
│ - Lightweight creatures.ron registry with file references   │
│ - File size: <5KB with only ID + name + filepath            │
│ - Individual .ron files remain single source of truth       │
│ - Easy to edit individual creatures independently           │
│ - Eager loading at campaign startup (all files loaded once) │
│ - Relative paths from campaign root for portability         │
└─────────────────────────────────────────────────────────────┘

Example Registry Entry:
    CreatureReference(
        id: 1,
        name: "Goblin",
        filepath: "assets/creatures/goblin.ron",
    )
```

## Current State Analysis

### Existing Infrastructure

**Creature Assets (32 files in `campaigns/tutorial/assets/creatures/`)**:

- Monster creatures: goblin, dragon, skeleton, skeleton_warrior, zombie, kobold, orc, ogre, giant_rat, wolf, lich, evil_lich, fire_elemental, red_dragon, pyramid_dragon, dying_goblin
- NPC creatures: innkeeper, merchant, village_elder, wizard_arcturus, high_priest, high_priestess, ranger, apprentice_zara, kira, mira, sirius, whisper, old_gareth
- Template creatures: template_dwarf_cleric, template_elf_mage, template_human_fighter

**Monster Definitions (`campaigns/tutorial/data/monsters.ron`)**:

- 11 monster types defined: Goblin (id:1), Kobold (id:2), Giant Rat (id:3), Orc (id:10), Skeleton (id:11), Wolf (id:12), Ogre (id:20), Zombie (id:21), Fire Elemental (id:22), Dragon (id:30), Lich (id:31)
- All have `visual_id: Option<CreatureId>` field currently unpopulated
- `MonsterDefinition` struct in `src/domain/combat/database.rs` supports visual linking

**NPC Definitions (`campaigns/tutorial/data/npcs.ron`)**:

- 12 NPCs defined with `sprite: Option<SpriteReference>` field (all `None`)
- `NpcDefinition` struct in `src/domain/world/npc.rs` has sprite field
- NPCs include: tutorial_elder_village, tutorial_innkeeper_town, tutorial_merchant_town, tutorial_priestess_town, tutorial_wizard_arcturus, tutorial_wizard_arcturus_brother, tutorial_ranger_lost, tutorial_elder_village2, tutorial_innkeeper_town2, tutorial_merchant_town2, tutorial_priest_town2, tutorial_goblin_dying

**Campaign Configuration (`campaigns/tutorial/campaign.ron`)**:

- References data files: items, spells, monsters, classes, races, characters, maps, quests, dialogues, conditions, npcs, proficiencies
- No creatures_file reference currently present

**Domain Layer Support**:

- `CreatureDatabase` exists in `src/domain/visual/creature_database.rs`
- `CreatureDefinition` type in `src/domain/visual/mod.rs`
- `CreatureId` type alias defined in `src/domain/types.rs`
- Loading from RON files supported via `load_from_file` and `load_from_string`

### Identified Issues

1. **No Creature Registry**: Individual creature RON files exist but no centralized registry file maps IDs to creature file paths
2. **Bloated Embedded Data**: Previous approach embedded full MeshDefinition data in single file (>1MB), needs lightweight reference system
3. **Unpopulated Visual IDs**: Monster definitions lack `visual_id` values linking to creatures
4. **NPC Visual Mechanism**: NPCs use sprite system but need procedural mesh support
5. **Campaign Loading Gap**: Campaign doesn't load creature registry during initialization
6. **Missing Domain Type**: No `CreatureReference` struct to represent lightweight registry entries
7. **Missing Visual Mappings**: No documentation of which monsters/NPCs map to which creature files
8. **Incomplete Creature Coverage**: Some variations (skeleton_warrior, evil_lich, dying_goblin) need mapping strategy

## Implementation Phases

### Phase 1: Creature Registry System Implementation

**Objective**: Create lightweight creature registry system with file references, add `CreatureReference` struct to domain layer, and establish eager loading pattern at campaign startup.

#### 1.1 Add `CreatureReference` Struct to Domain Layer

**File**: `src/domain/visual/mod.rs`

**Current Issue**: No lightweight struct to represent creature registry entries that reference external mesh files.

**Required Change**:

Add `CreatureReference` struct after `CreatureDefinition`:

````rust
/// Lightweight creature registry entry
///
/// Used in campaign creature registries to reference external creature mesh files
/// instead of embedding full MeshDefinition data inline.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::CreatureReference;
///
/// let reference = CreatureReference {
///     id: 1,
///     name: "Goblin".to_string(),
///     filepath: "assets/creatures/goblin.ron".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatureReference {
    /// Unique creature identifier matching the referenced creature file
    pub id: CreatureId,

    /// Display name for editor/debugging
    pub name: String,

    /// Relative path to creature definition file from campaign root
    ///
    /// Example: "assets/creatures/goblin.ron"
    pub filepath: String,
}
````

**Rationale**:

- Separates registry (lightweight references) from data (full mesh definitions)
- Reduces `creatures.ron` file size from >1MB to a few KB
- Enables individual creature file editing without touching massive database
- Maintains single source of truth (individual `.ron` files)
- Supports eager loading at campaign startup with file resolution

**Testing**:

- Verify `CreatureReference` compiles and implements required traits
- Test RON serialization/deserialization of `Vec<CreatureReference>`
- Ensure `id` field properly uses `CreatureId` type alias

#### 1.2 Verify Individual Creature Files

**Files**: All 32 files in `campaigns/tutorial/assets/creatures/*.ron`

**Current Status**: Individual creature files already exist with proper `CreatureDefinition` format including:

- SPDX headers
- `id` field
- `name` field
- `meshes` array with `MeshDefinition` entries
- `mesh_transforms` array
- `scale` field
- `color_tint` field

**Verification Tasks**:

1. **Validate file integrity**: Ensure each file parses as valid `CreatureDefinition`
2. **Verify ID uniqueness**: Confirm no duplicate IDs across all 32 files
3. **Check required fields**: All files have `id`, `name`, `meshes`, `mesh_transforms`
4. **Validate mesh counts**: Each file has matching `meshes.len() == mesh_transforms.len()`
5. **Confirm SPDX headers**: All files include copyright and license information

**ID Assignment Strategy**:

- Monster base creatures: 1-50
- NPC creatures: 51-100
- Template creatures: 101-150
- Variant creatures: 151-200

**Specific ID Assignments**:

| Creature File              | Assigned ID | Category | Notes                  |
| -------------------------- | ----------- | -------- | ---------------------- |
| goblin.ron                 | 1           | Monster  | Base goblin            |
| kobold.ron                 | 2           | Monster  | Base kobold            |
| giant_rat.ron              | 3           | Monster  | Base rat               |
| orc.ron                    | 10          | Monster  | Base orc               |
| skeleton.ron               | 11          | Monster  | Base skeleton          |
| wolf.ron                   | 12          | Monster  | Base wolf              |
| ogre.ron                   | 20          | Monster  | Base ogre              |
| zombie.ron                 | 21          | Monster  | Base zombie            |
| fire_elemental.ron         | 22          | Monster  | Base elemental         |
| dragon.ron                 | 30          | Monster  | Generic dragon         |
| lich.ron                   | 31          | Monster  | Base lich              |
| red_dragon.ron             | 32          | Monster  | Red dragon variant     |
| pyramid_dragon.ron         | 33          | Monster  | Pyramid dragon variant |
| dying_goblin.ron           | 151         | Variant  | Wounded goblin NPC     |
| skeleton_warrior.ron       | 152         | Variant  | Elite skeleton         |
| evil_lich.ron              | 153         | Variant  | Powerful lich          |
| village_elder.ron          | 51          | NPC      | Elder character        |
| innkeeper.ron              | 52          | NPC      | Innkeeper character    |
| merchant.ron               | 53          | NPC      | Merchant character     |
| high_priest.ron            | 54          | NPC      | Male priest            |
| high_priestess.ron         | 55          | NPC      | Female priest          |
| wizard_arcturus.ron        | 56          | NPC      | Wizard character       |
| ranger.ron                 | 57          | NPC      | Ranger character       |
| old_gareth.ron             | 58          | NPC      | Old character variant  |
| apprentice_zara.ron        | 59          | NPC      | Apprentice character   |
| kira.ron                   | 60          | NPC      | Kira character         |
| mira.ron                   | 61          | NPC      | Mira character         |
| sirius.ron                 | 62          | NPC      | Sirius character       |
| whisper.ron                | 63          | NPC      | Whisper character      |
| template_human_fighter.ron | 101         | Template | Human fighter base     |
| template_elf_mage.ron      | 102         | Template | Elf mage base          |
| template_dwarf_cleric.ron  | 103         | Template | Dwarf cleric base      |

#### 1.3 Create Creature Registry File

**File**: `campaigns/tutorial/data/creatures.ron`

**Structure**: Array of `CreatureReference` entries pointing to individual files

```ron
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// Tutorial Campaign Creature Registry
// Lightweight registry mapping creature IDs to external mesh definition files
// Individual creature files located in assets/creatures/ directory
// Loaded eagerly at campaign startup for performance

[
    // Monster Creatures (IDs 1-50)
    CreatureReference(
        id: 1,
        name: "Goblin",
        filepath: "assets/creatures/goblin.ron",
    ),
    CreatureReference(
        id: 2,
        name: "Kobold",
        filepath: "assets/creatures/kobold.ron",
    ),
    CreatureReference(
        id: 3,
        name: "GiantRat",
        filepath: "assets/creatures/giant_rat.ron",
    ),
    // ... all 32 creatures
]
```

**Key Design Decisions**:

- **Registry-based**: References to files, not embedded data (reduces size from >1MB to ~2KB)
- **Relative paths**: Paths relative to campaign root for portability
- **Eager loading**: All creatures loaded at campaign startup (simpler than lazy loading)
- **Single source of truth**: Individual `.ron` files remain authoritative

#### 1.4 Implement Creature Registry Loading

**Files**:

- `src/domain/visual/creature_database.rs`
- `src/sdk/campaign_loader.rs`

**Required Changes**:

1. **Add registry loading method** to `CreatureDatabase`:

```rust
/// Loads creature registry and resolves all file references eagerly
///
/// # Arguments
///
/// * `registry_path` - Path to creatures.ron registry file
/// * `campaign_root` - Campaign root directory for resolving relative paths
///
/// # Returns
///
/// Returns `CreatureDatabase` with all creatures loaded from individual files
///
/// # Errors
///
/// Returns error if registry file invalid or any referenced creature file fails to load
pub fn load_from_registry(
    registry_path: &Path,
    campaign_root: &Path,
) -> Result<Self, Box<dyn std::error::Error>> {
    // 1. Load registry file as Vec<CreatureReference>
    // 2. For each reference, resolve filepath relative to campaign_root
    // 3. Load full CreatureDefinition from resolved path
    // 4. Insert into database with id as key
    // 5. Return populated database
}
```

2. **Update campaign loader** to use registry loading:

```rust
// In load_campaign() function
let creature_db = if let Some(creatures_file) = &metadata.creatures_file {
    let registry_path = campaign_path.join(creatures_file);
    CreatureDatabase::load_from_registry(&registry_path, campaign_path)?
} else {
    CreatureDatabase::new()
};
```

**Rationale**:

- Eager loading at startup ensures all creatures available immediately
- Centralized error handling during load phase (fail-fast)
- No runtime file I/O during gameplay
- Simpler than lazy loading with on-demand file resolution

#### 1.5 Update Campaign Metadata

**File**: `campaigns/tutorial/campaign.ron`

**Changes**:

- Add field: `creatures_file: "data/creatures.ron"`
- Position after `monsters_file` line for logical grouping

**Validation**:

- Verify `CampaignMetadata` struct in `src/sdk/campaign_loader.rs` already has `creatures_file` field (it does, with default)
- Confirm field is properly deserialized

#### 1.6 Testing Requirements

**After 1.1 (CreatureReference Struct)**:

- Verify `CreatureReference` compiles with all required traits
- Test RON serialization/deserialization
- Confirm `id` field properly uses `CreatureId` type alias

**After 1.2 (Individual File Verification)**:

- Each of 32 files in `assets/creatures/` parses as valid `CreatureDefinition`
- No duplicate IDs across all creatures
- Each creature has matching `meshes.len() == mesh_transforms.len()`
- All files include SPDX headers

**After 1.3 (Registry File Creation)**:

- `data/creatures.ron` parses as `Vec<CreatureReference>`
- All 32 creatures referenced with unique IDs
- All filepaths resolve correctly from campaign root
- Registry file size < 5KB (compared to >1MB for embedded approach)

**After 1.4 (Registry Loading)**:

- `CreatureDatabase::load_from_registry()` successfully loads all 32 creatures
- Each referenced file resolves and parses correctly
- Loaded database contains all 32 creatures with correct IDs
- Loading time acceptable (<1 second for all creatures)

**After 1.5 (Campaign Integration)**:

- Campaign loads successfully with `creatures_file: "data/creatures.ron"`
- Creature database available in loaded campaign state
- No errors during campaign startup

**Commands**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

#### 1.7 Deliverables

- [ ] `src/domain/visual/mod.rs`: `CreatureReference` struct added with proper documentation
- [ ] `src/domain/visual/creature_database.rs`: `load_from_registry()` method implemented
- [ ] `src/sdk/campaign_loader.rs`: Updated to use registry loading
- [ ] All 32 files in `campaigns/tutorial/assets/creatures/` verified:
  - [x] Already have SPDX headers
  - [x] Already have `id` field
  - [x] Already have `mesh_transforms` arrays
  - [x] Already have proper `CreatureDefinition` format
- [ ] `campaigns/tutorial/data/creatures.ron` created as lightweight registry:
  - [ ] Contains 32 `CreatureReference` entries
  - [ ] Uses relative paths from campaign root
  - [ ] File size < 5KB (vs >1MB for embedded approach)
- [ ] `campaigns/tutorial/campaign.ron` updated with `creatures_file: "data/creatures.ron"`
- [ ] All files validate with `cargo check`
- [ ] Registry loading tested with all 32 creatures
- [ ] Documentation updated to reflect registry-based architecture

#### 1.8 Success Criteria

- `CreatureReference` struct exists in domain layer with proper documentation
- `CreatureDatabase::load_from_registry()` method implemented with eager loading
- All 32 individual creature files validate as proper `CreatureDefinition` format
- Lightweight registry file (`creatures.ron`) contains all 32 references with relative paths
- Registry file size dramatically reduced (< 5KB vs >1MB for embedded approach)
- Campaign loader successfully loads creature database from registry
- All 32 creatures accessible by ID after campaign load
- No compilation errors or warnings
- Individual creature files remain single source of truth
- Easy to edit individual creatures without touching registry file
- Loading time acceptable (< 1 second for all creatures at startup)

### Phase 2: Monster Visual Mapping

**Objective**: Link monster definitions to creature visuals via `visual_id` field.

#### 2.1 Monster-to-Creature Mapping

**File**: `campaigns/tutorial/data/monsters.ron`

**Monster Mapping Table**:

| Monster ID | Monster Name   | Creature ID | Creature File      | Notes                 |
| ---------- | -------------- | ----------- | ------------------ | --------------------- |
| 1          | Goblin         | 1           | goblin.ron         | Exact match           |
| 2          | Kobold         | 2           | kobold.ron         | Exact match           |
| 3          | Giant Rat      | 3           | giant_rat.ron      | Exact match           |
| 10         | Orc            | 10          | orc.ron            | Exact match           |
| 11         | Skeleton       | 11          | skeleton.ron       | Exact match           |
| 12         | Wolf           | 12          | wolf.ron           | Exact match           |
| 20         | Ogre           | 20          | ogre.ron           | Exact match           |
| 21         | Zombie         | 21          | zombie.ron         | Exact match           |
| 22         | Fire Elemental | 22          | fire_elemental.ron | Exact match           |
| 30         | Dragon         | 30          | dragon.ron         | Generic dragon visual |
| 31         | Lich           | 31          | lich.ron           | Base lich visual      |

**Changes Required**:

- Update each monster entry to include `visual_id: Some({creature_id})`
- Example: Goblin (id:1) gets `visual_id: Some(1)`

**Before**:

```ron
(
    id: 1,
    name: "Goblin",
    stats: ( ... ),
    // ... other fields
    visual_id: None,
)
```

**After**:

```ron
(
    id: 1,
    name: "Goblin",
    stats: ( ... ),
    // ... other fields
    visual_id: Some(1),
)
```

#### 2.2 Variant Creature Strategy

**Elite/Variant Monsters** (future expansion):

- Skeleton Warrior visual (id:152) - use for elite skeleton encounters
- Evil Lich visual (id:153) - use for lich boss encounters
- Red Dragon visual (id:32) - use for fire dragon encounters
- Pyramid Dragon visual (id:33) - use for ancient dragon encounters

**Recommendation**: Document these mappings but don't implement until corresponding monster definitions are created.

#### 2.3 Missing Creature Visuals

**Monsters Needing New Creatures** (none currently):

- All 11 tutorial monsters have matching creature visuals
- Future monster additions will require new creature mesh files

#### 2.4 Testing Requirements

**Validation**:

- All `visual_id` values reference existing creature IDs from Phase 1
- No broken references (visual_id pointing to non-existent creature)
- Monster loading succeeds with visual_id populated

**Test Approach**:

- Load monsters.ron and creatures.ron
- For each monster with visual_id, verify creature exists in database
- Log warnings for any missing creature references

#### 2.5 Deliverables

- [ ] All 11 monsters in `monsters.ron` have `visual_id` field populated
- [ ] Monster-to-creature mapping table documented
- [ ] Variant creature strategy documented for future use
- [ ] No broken creature references

#### 2.6 Success Criteria

- Every monster definition has valid `visual_id` value
- All referenced creature IDs exist in creature database
- Monster loading completes without errors
- Visual mappings are documented and verifiable

### Phase 3: NPC Procedural Mesh Integration

**Objective**: Update NPC system to use procedural meshes instead of sprites.

#### 3.1 NPC Visual Architecture Decision

**Current State**: `NpcDefinition` has `sprite: Option<SpriteReference>` field

**Options**:

**Option A**: Add `creature_id: Option<CreatureId>` field alongside sprite

- Pros: Backward compatible, supports both systems
- Cons: Two visual systems in parallel, more complex

**Option B**: Repurpose sprite field to support creature references

- Pros: Minimal schema change, single visual field
- Cons: Less explicit, requires enum or variant handling

**Option C**: Replace sprite with `visual: NpcVisual` enum

- Pros: Explicit, type-safe, future-proof
- Cons: Breaking change, requires migration

**Recommendation**: Option A for tutorial campaign (maintain compatibility)

#### 3.2 Update NPC Definitions

**File**: `campaigns/tutorial/data/npcs.ron`

**Approach**: Add `creature_id` field to each NPC (requires domain struct update first)

**NPC-to-Creature Mapping**:

| NPC ID                           | NPC Name                    | Creature ID | Creature File       | Rationale                          |
| -------------------------------- | --------------------------- | ----------- | ------------------- | ---------------------------------- |
| tutorial_elder_village           | Village Elder Town Square   | 51          | village_elder.ron   | Exact match                        |
| tutorial_elder_village2          | Village Elder Mountain Pass | 51          | village_elder.ron   | Same visual, different location    |
| tutorial_innkeeper_town          | InnKeeper Town Square       | 52          | innkeeper.ron       | Exact match                        |
| tutorial_innkeeper_town2         | Innkeeper Mountain Pass     | 52          | innkeeper.ron       | Same visual, different location    |
| tutorial_merchant_town           | Merchant Town Square        | 53          | merchant.ron        | Exact match                        |
| tutorial_merchant_town2          | Merchant Mountain Pass      | 53          | merchant.ron        | Same visual, different location    |
| tutorial_priestess_town          | High Priestess Town Square  | 55          | high_priestess.ron  | Exact match                        |
| tutorial_priest_town2            | High Priest Mountain Pass   | 54          | high_priest.ron     | Gender swap                        |
| tutorial_wizard_arcturus         | Arcturus                    | 56          | wizard_arcturus.ron | Exact match                        |
| tutorial_wizard_arcturus_brother | Arcturus Brother            | 58          | old_gareth.ron      | Different character, similar style |
| tutorial_ranger_lost             | Lost Ranger                 | 57          | ranger.ron          | Exact match                        |
| tutorial_goblin_dying            | Dying Goblin                | 151         | dying_goblin.ron    | Special variant                    |

**Implementation Note**: If `NpcDefinition` doesn't support `creature_id` field, add to domain struct first before updating data file.

#### 3.3 Domain Layer Updates (if needed)

**File**: `src/domain/world/npc.rs`

**Potential Change** (verify if needed):

```rust
pub struct NpcDefinition {
    // ... existing fields
    pub sprite: Option<SpriteReference>,

    // Add new field
    #[serde(default)]
    pub creature_id: Option<CreatureId>,
}
```

**Backward Compatibility**: Use `#[serde(default)]` so existing RON files without `creature_id` continue working.

#### 3.4 Testing Requirements

**Validation**:

- NPCs load successfully with creature_id field
- All creature_id values reference valid creatures from database
- Sprite fallback still works when creature_id is None
- No rendering errors when NPCs spawn

**Test Cases**:

- Load npcs.ron with new creature_id fields
- Verify each NPC creature reference is valid
- Test NPC spawning in game (visual verification)
- Confirm placeholder sprites still work as fallback

#### 3.5 Deliverables

- [ ] `NpcDefinition` struct updated with `creature_id` field (if needed)
- [ ] All 12 NPCs in `npcs.ron` have `creature_id` populated
- [ ] NPC-to-creature mapping table documented
- [ ] Sprite fallback mechanism verified working

#### 3.6 Success Criteria

- All NPCs have valid creature_id references
- No broken creature references
- NPCs render with procedural meshes in-game
- Fallback to placeholder sprites works when creature missing
- Backward compatibility maintained for old NPC definitions

### Phase 4: Campaign Loading Integration

**Objective**: Ensure campaign properly loads and uses creature database.

#### 4.1 Campaign Loading Verification

**Files to Review**:

- `src/domain/campaign/loader.rs` - campaign loading logic
- `src/domain/campaign/metadata.rs` - CampaignMetadata struct
- `src/infrastructure/campaign/` - infrastructure campaign loading

**Required Changes**:

1. Add `creatures_file` field to `CampaignMetadata` struct if missing
2. Load creature database during campaign initialization
3. Make `CreatureDatabase` accessible to rendering systems
4. Validate creature references during campaign loading

**Validation Points**:

- Campaign loads `data/creatures.ron` successfully
- Creature database is accessible to monster spawning
- Creature database is accessible to NPC spawning
- Missing creature files produce clear error messages

#### 4.2 Monster Spawning Integration

**System**: Combat encounter spawning

**Verification**:

- Monsters spawn with correct creature visual based on `visual_id`
- Missing `visual_id` falls back to default/placeholder
- Creature meshes render correctly in combat mode
- Creature scale/health/speed properties are respected

**Files to Check**:

- Monster spawning systems in game engine layer
- Combat encounter initialization
- Creature mesh generation from `CreatureDefinition`

#### 4.3 NPC Spawning Integration

**System**: World NPC placement

**Verification**:

- NPCs spawn with correct creature visual based on `creature_id`
- NPCs without `creature_id` fall back to sprite system
- Creature meshes render correctly in exploration mode
- NPC positioning and facing work with procedural meshes

**Files to Check**:

- NPC spawning systems in game engine layer
- Map loading and NPC placement
- Creature mesh generation for NPCs

#### 4.4 Testing Requirements

**Integration Tests**:

- Load tutorial campaign with new creature database
- Spawn test monster encounter with creature visuals
- Place test NPC with creature visual
- Verify rendering in first-person view
- Test both Exploration and Combat game modes

**Performance Tests**:

- Measure creature loading time
- Verify mesh generation caching works
- Check memory usage with all creatures loaded
- Profile rendering performance with multiple creatures

**Commands**:

```bash
cargo nextest run --all-features
cargo run --release --bin antares -- --campaign tutorial
```

#### 4.5 Deliverables

- [ ] Campaign loads creature database on initialization
- [ ] Monsters spawn with procedural mesh visuals
- [ ] NPCs spawn with procedural mesh visuals
- [ ] Fallback mechanisms work correctly
- [ ] Integration tests pass
- [ ] No performance regressions

#### 4.6 Success Criteria

- Tutorial campaign launches without errors
- All creatures load from database successfully
- Monsters visible in combat with correct meshes
- NPCs visible in exploration with correct meshes
- Sprite placeholders work when creature missing
- Campaign runs at acceptable frame rate

### Phase 5: Documentation and Content Audit

**Objective**: Document implementation and identify missing content.

#### 5.1 Create Integration Documentation

**File**: `campaigns/tutorial/README.md`

**Sections to Add/Update**:

1. **Visual Assets**: Overview of procedural mesh system
2. **Creature Database**: How to add new creatures
3. **Monster Visuals**: Mapping table documentation
4. **NPC Visuals**: Mapping table documentation
5. **Troubleshooting**: Common issues with creature loading

**Content**:

- Creature ID assignment strategy
- How to create new creature mesh files
- How to link monsters/NPCs to creatures
- Fallback sprite system explanation

#### 5.2 Missing Content Inventory

**Current Status**: All 11 tutorial monsters have creature visuals

**Unused Creatures** (available but not referenced):

- skeleton_warrior.ron (id:152) - Elite skeleton variant
- evil_lich.ron (id:153) - Boss lich variant
- red_dragon.ron (id:32) - Fire dragon variant
- pyramid_dragon.ron (id:33) - Ancient dragon variant
- apprentice_zara.ron (id:59) - Apprentice NPC
- kira.ron (id:60) - Kira NPC
- mira.ron (id:61) - Mira NPC
- sirius.ron (id:62) - Sirius NPC
- whisper.ron (id:63) - Whisper NPC

**Recommendations**:

- Document unused creatures for future quest expansion
- Consider adding elite encounters using variant creatures
- Use character creatures for future NPC additions

#### 5.3 Create Mapping Reference File

**File**: `campaigns/tutorial/creature_mappings.md`

**Content**:

- Complete monster-to-creature mapping table
- Complete NPC-to-creature mapping table
- Creature ID assignment ranges
- Available unused creatures
- Guidelines for adding new creatures

**Format**: Markdown tables for easy reference

#### 5.4 Update Implementation Status

**File**: `docs/explanation/implementations.md`

**Entry to Add**:

```markdown
## Tutorial Campaign Procedural Mesh Integration

**Date**: {completion_date}
**Phase**: Content Integration
**Files Modified**:

- campaigns/tutorial/data/creatures.ron (created)
- campaigns/tutorial/data/monsters.ron (visual_id added)
- campaigns/tutorial/data/npcs.ron (creature_id added)
- campaigns/tutorial/campaign.ron (creatures_file added)
- src/domain/world/npc.rs (creature_id field added, if needed)

**Summary**: Integrated 32 procedural mesh creatures into tutorial campaign,
replacing sprite placeholders. All 11 monsters and 12 NPCs now use 3D procedural
meshes for visual representation.

**Testing**: Integration tests pass, campaign loads successfully, visual rendering
verified in-game.
```

#### 5.5 Deliverables

- [ ] `campaigns/tutorial/README.md` updated with creature documentation
- [ ] `campaigns/tutorial/creature_mappings.md` created
- [ ] Unused creatures documented for future use
- [ ] `docs/explanation/implementations.md` updated

#### 5.6 Success Criteria

- Complete documentation of creature system
- All mappings clearly documented
- Future content creators have clear guidelines
- Implementation properly recorded in project documentation

### Phase 6: Campaign Builder Creatures Editor Integration

**Objective**: Enable visual editing of creature mappings in `creatures.ron` from the Campaign Builder UI's Creatures Editor tab.

#### 6.1 UI Components for Creatures Editor

**Location**: Campaign Builder → Creatures Tab

**Required UI Elements**:

1. **Creature List Panel** (Left Side)

   - Scrollable list of all creature references
   - Group by category: Monsters (1-50), NPCs (51-100), Templates (101-150), Variants (151-200)
   - Display: ID, Name, Status (✓ File Exists / ⚠ Missing)
   - Search/filter capability
   - Sort options (by ID, by Name, by Category)

2. **Creature Details Editor** (Right Side)

   - Creature ID field (numeric input, validated)
   - Creature Name field (text input)
   - Filepath field (text input with file browser button)
   - "Browse" button to select .ron file from assets/creatures/
   - Preview of selected creature mesh (if renderer available)
   - Validation status indicators

3. **Action Buttons**
   - "Add New Creature" button
   - "Delete Selected Creature" button
   - "Save Changes" button (writes to creatures.ron)
   - "Reload from File" button (discards unsaved changes)
   - "Validate All" button (checks all file references)

#### 6.2 Backend Implementation

**File**: `src/ui/campaign_builder/creatures_editor.rs` (new)

**Core Functionality**:

```antares/docs/explanation/tutorial_procedural_mesh_integration_plan.md#L1-50
/// Manages the creatures editor tab in the campaign builder
pub struct CreaturesEditor {
    /// Path to the campaign's creatures.ron file
    creatures_file_path: PathBuf,

    /// In-memory representation of creature registry
    creatures: Vec<CreatureReference>,

    /// Currently selected creature index
    selected_index: Option<usize>,

    /// Unsaved changes flag
    is_dirty: bool,

    /// Validation results cache
    validation_results: HashMap<CreatureId, ValidationResult>,
}

impl CreaturesEditor {
    /// Load creatures.ron into editor
    pub fn load_from_file(path: PathBuf) -> Result<Self, EditorError>;

    /// Save current state back to creatures.ron
    pub fn save_to_file(&self) -> Result<(), EditorError>;

    /// Add a new creature reference
    pub fn add_creature(&mut self, creature: CreatureReference) -> Result<(), EditorError>;

    /// Update existing creature reference
    pub fn update_creature(&mut self, index: usize, creature: CreatureReference) -> Result<(), EditorError>;

    /// Delete creature reference
    pub fn delete_creature(&mut self, index: usize) -> Result<(), EditorError>;

    /// Validate all creature file references exist
    pub fn validate_all(&mut self) -> ValidationReport;

    /// Check for duplicate IDs
    pub fn check_duplicate_ids(&self) -> Vec<CreatureId>;

    /// Suggest next available ID in range
    pub fn suggest_next_id(&self, category: CreatureCategory) -> CreatureId;
}
```

#### 6.3 Validation Logic

**ID Range Validation**:

- Monsters: 1-50
- NPCs: 51-100
- Templates: 101-150
- Variants: 151-200
- Custom: 201+ (campaign-specific)

**File Reference Validation**:

```antares/docs/explanation/tutorial_procedural_mesh_integration_plan.md#L51-70
pub enum ValidationResult {
    Valid,
    FileNotFound(PathBuf),
    InvalidPath(String),
    DuplicateId(CreatureId),
    IdOutOfRange { id: CreatureId, expected_range: std::ops::Range<u32> },
    InvalidRonSyntax(String),
}

pub struct ValidationReport {
    pub total_creatures: usize,
    pub valid_count: usize,
    pub warnings: Vec<(CreatureId, String)>,
    pub errors: Vec<(CreatureId, ValidationResult)>,
}
```

**Real-Time Validation**:

- Check file exists on Browse button click
- Validate ID uniqueness on change
- Validate ID range based on creature category
- Parse .ron file to verify syntax (optional, can be expensive)
- Mark dirty flag on any edit

#### 6.4 RON File Operations

**Reading creatures.ron**:

```antares/docs/explanation/tutorial_procedural_mesh_integration_plan.md#L71-85
use serde::Deserialize;
use std::fs;

pub fn load_creatures_registry(path: &Path) -> Result<Vec<CreatureReference>, EditorError> {
    let content = fs::read_to_string(path)
        .map_err(|e| EditorError::FileReadError(e.to_string()))?;

    let creatures: Vec<CreatureReference> = ron::from_str(&content)
        .map_err(|e| EditorError::RonParseError(e.to_string()))?;

    Ok(creatures)
}
```

**Writing creatures.ron**:

```antares/docs/explanation/tutorial_procedural_mesh_integration_plan.md#L86-110
use ron::ser::{PrettyConfig, to_string_pretty};

pub fn save_creatures_registry(
    path: &Path,
    creatures: &[CreatureReference],
    preserve_header: bool,
) -> Result<(), EditorError> {
    // Preserve header comments if requested
    let header = if preserve_header {
        CREATURES_RON_HEADER
    } else {
        ""
    };

    // Configure pretty printing
    let pretty = PrettyConfig::new()
        .depth_limit(2)
        .separate_tuple_members(true)
        .enumerate_arrays(true);

    let ron_content = to_string_pretty(creatures, pretty)
        .map_err(|e| EditorError::RonSerializeError(e.to_string()))?;

    let final_content = format!("{}\n{}", header, ron_content);

    fs::write(path, final_content)
        .map_err(|e| EditorError::FileWriteError(e.to_string()))?;

    Ok(())
}
```

#### 6.5 User Workflow

**Adding a New Creature**:

1. Click "Add New Creature" button
2. System suggests next available ID based on selected category
3. User enters creature name
4. User clicks "Browse" to select creature .ron file from assets/creatures/
5. System validates file exists and is valid RON
6. User clicks "Save Changes" to write to creatures.ron
7. System revalidates entire registry

**Editing Existing Creature**:

1. Select creature from list
2. Modify ID, name, or filepath in details panel
3. System shows validation status in real-time
4. User clicks "Save Changes" to persist
5. System marks unsaved changes until save

**Deleting a Creature**:

1. Select creature from list
2. Click "Delete Selected Creature"
3. System prompts for confirmation
4. System checks if creature is referenced by monsters/NPCs (optional warning)
5. Remove from list on confirmation
6. User clicks "Save Changes" to persist

**Batch Validation**:

1. Click "Validate All" button
2. System checks all file references exist
3. System checks for duplicate IDs
4. System displays validation report with errors/warnings
5. User can click on errors to navigate to problematic entries

#### 6.6 Integration Points

**Campaign Builder Main Window**:

- Add "Creatures" tab to campaign builder tabs
- Load creatures.ron when campaign is loaded
- Show unsaved changes indicator in tab title
- Prompt to save on tab close if dirty

**Cross-References**:

- Show usage count (how many monsters/NPCs reference each creature)
- Optional: Click to view which monsters/NPCs use this creature
- Warning when deleting a creature that's in use
- Auto-suggest creatures when editing monsters/NPCs

**File Watching** (Optional):

- Detect external changes to creatures.ron
- Prompt user to reload if file modified outside editor
- Prevent data loss from concurrent edits

#### 6.7 Testing Requirements

**Unit Tests**:

- Test loading valid creatures.ron
- Test loading malformed RON (expect errors)
- Test saving with header preservation
- Test duplicate ID detection
- Test ID range validation
- Test file reference validation

**Integration Tests**:

- Load tutorial campaign creatures.ron
- Add new creature and save
- Delete creature and save
- Modify creature and save
- Validate round-trip (load → save → load) preserves data

**UI Tests** (if applicable):

- Test add creature workflow
- Test edit creature workflow
- Test delete creature workflow
- Test validation display
- Test unsaved changes warning

**Example Test**:

```antares/docs/explanation/tutorial_procedural_mesh_integration_plan.md#L111-135
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_tutorial_creatures_registry() {
        let path = PathBuf::from("campaigns/tutorial/data/creatures.ron");
        let result = load_creatures_registry(&path);

        assert!(result.is_ok());
        let creatures = result.unwrap();
        assert!(creatures.len() > 0);

        // Verify first creature
        let goblin = &creatures[0];
        assert_eq!(goblin.id, 1);
        assert_eq!(goblin.name, "Goblin");
        assert_eq!(goblin.filepath, "assets/creatures/goblin.ron");
    }

    #[test]
    fn test_detect_duplicate_ids() {
        let mut editor = CreaturesEditor::new();
        editor.add_creature(CreatureReference { id: 1, name: "A".into(), filepath: "a.ron".into() }).unwrap();
        editor.add_creature(CreatureReference { id: 1, name: "B".into(), filepath: "b.ron".into() }).unwrap();

        let duplicates = editor.check_duplicate_ids();
        assert_eq!(duplicates, vec![1]);
    }
}
```

#### 6.8 Error Handling

**User-Facing Errors**:

- Clear error messages in UI (not debug dumps)
- Specific guidance on how to fix issues
- Highlight problematic fields in red
- Show tooltip with error details

**Error Categories**:

```antares/docs/explanation/tutorial_procedural_mesh_integration_plan.md#L136-150
#[derive(Error, Debug)]
pub enum EditorError {
    #[error("Failed to read creatures file: {0}")]
    FileReadError(String),

    #[error("Failed to write creatures file: {0}")]
    FileWriteError(String),

    #[error("Invalid RON syntax: {0}")]
    RonParseError(String),

    #[error("Duplicate creature ID: {0}")]
    DuplicateId(CreatureId),

    #[error("Creature ID {0} out of valid range for category")]
    IdOutOfRange(CreatureId),

    #[error("Creature file not found: {0}")]
    CreatureFileNotFound(PathBuf),
}
```

#### 6.9 UI/UX Considerations

**Visual Feedback**:

- Green checkmark for valid creatures
- Yellow warning for missing files
- Red error for validation failures
- Loading spinner during validation
- Progress bar for batch operations

**Keyboard Shortcuts**:

- Ctrl+S: Save changes
- Ctrl+N: Add new creature
- Delete: Delete selected creature
- Ctrl+F: Focus search box
- Ctrl+R: Reload from file

**Accessibility**:

- Tab navigation support
- Screen reader labels
- High contrast mode support
- Keyboard-only operation

#### 6.10 Deliverables

- [ ] `src/ui/campaign_builder/creatures_editor.rs` implemented
- [ ] UI components for creature list and detail editing
- [ ] RON loading/saving with header preservation
- [ ] Validation logic for IDs and file references
- [ ] Integration with main campaign builder window
- [ ] Unit tests for editor logic
- [ ] Integration tests with tutorial creatures.ron
- [ ] User documentation for creatures editor

#### 6.11 Success Criteria

- User can load creatures.ron from any campaign
- User can add/edit/delete creature references via UI
- Changes persist correctly to creatures.ron file
- All validation catches common errors before save
- UI prevents data loss (unsaved changes warning)
- Round-trip load/save preserves all data and formatting
- Editor integrates seamlessly with campaign builder
- Documentation explains workflow clearly

#### 6.12 Future Enhancements

**Possible Extensions** (not required for initial implementation):

- Creature mesh preview in editor
- Drag-and-drop creature file selection
- Bulk import from directory
- Export creature subset to new campaign
- Creature duplication feature
- Undo/redo support
- Template creature generation wizard
- Usage analytics (which creatures are most used)
- Integration with monster/NPC editors (click to edit creature)

## Cross-Cutting Concerns

### Backward Compatibility

**Sprite Fallback System**:

- Keep `assets/sprites/placeholders/` directory
- Maintain sprite loading code for NPCs
- Gracefully handle missing creature_id fields
- Use placeholder sprite when creature not found

**RON File Compatibility**:

- Use `#[serde(default)]` for new optional fields
- Support old campaign files without creatures_file
- Maintain existing monster definitions without visual_id

### Error Handling

**Creature Loading Errors**:

- Clear error messages for missing creature files
- Validation of creature ID references during loading
- Graceful degradation to placeholders on error
- Logging of creature loading issues

**Runtime Errors**:

- Handle missing creature visuals without crashing
- Warn when visual_id references non-existent creature
- Fall back to default mesh/sprite on render failure

### Performance Considerations

**Creature Loading**:

- Load creature database once during campaign init
- Cache parsed CreatureDefinition objects
- Lazy-load individual creature meshes on demand

**Mesh Generation**:

- Use existing mesh caching system from Phase 2 implementation
- Generate meshes once, reuse for multiple instances
- Consider instancing for repeated creatures

### Validation Strategy

**Pre-Runtime Validation**:

- RON syntax validation via `cargo check`
- Creature ID reference validation during loading
- Schema validation for all data files

**Runtime Validation**:

- Verify creature exists before spawning
- Validate mesh data integrity
- Check for circular references

## Migration Path

### Step-by-Step Migration

1. **Phase 1 First** - Create foundation (creature database)
2. **Phase 2 Second** - Update monster data (low risk)
3. **Phase 3 Third** - Update NPC data (requires domain changes)
4. **Phase 4 Fourth** - Integration testing (validate everything works)
5. **Phase 5 Last** - Documentation (record decisions)

### Rollback Strategy

**If Issues Arise**:

- Creature database optional - campaign works without it
- Monsters work without visual_id (field is Option)
- NPCs fall back to sprites when creature_id missing
- Can disable creature loading via campaign config

### Testing Between Phases

**After Phase 1**:

```bash
cargo check
ron::from_str validation on creatures.ron
```

**After Phase 2**:

```bash
cargo nextest run
Validate monster loading with visual_id
```

**After Phase 3**:

```bash
cargo nextest run
Validate NPC loading with creature_id
```

**After Phase 4**:

```bash
cargo run --release -- --campaign tutorial
Visual verification in-game
```

## Risk Mitigation

### High Risk Items

1. **Domain Struct Changes**: Adding `creature_id` to `NpcDefinition` requires code changes

   - Mitigation: Use `#[serde(default)]` for backward compatibility
   - Mitigation: Comprehensive unit tests for NPC loading

2. **Campaign Loading Integration**: Creature database must load correctly

   - Mitigation: Add error handling and fallback
   - Mitigation: Test with and without creatures_file

3. **Visual Rendering**: Creatures must render correctly in-game
   - Mitigation: Manual testing in both game modes
   - Mitigation: Fall back to placeholders on failure

### Medium Risk Items

1. **RON File Syntax**: Large creatures.ron file prone to syntax errors

   - Mitigation: Use automated RON validation
   - Mitigation: Build incrementally, test often

2. **ID Collisions**: Risk of duplicate creature IDs

   - Mitigation: Follow strict ID assignment table
   - Mitigation: Validation script to check for duplicates

3. **Missing References**: visual_id or creature_id pointing to non-existent creatures
   - Mitigation: Validation during campaign loading
   - Mitigation: Clear error messages with ID information

### Low Risk Items

1. **Performance Impact**: Creature loading adds initialization time

   - Mitigation: Already tested in procedural mesh implementation
   - Mitigation: Caching system in place

2. **Documentation Drift**: Mappings become outdated
   - Mitigation: Generate mapping tables from data files
   - Mitigation: Include in code review checklist

## Implementation Order Summary

1. **Phase 1** (Foundation): Create creature database and update campaign metadata
2. **Phase 2** (Monsters): Map monsters to creature visuals
3. **Phase 3** (NPCs): Update NPCs to use creature meshes
4. **Phase 4** (Integration): Verify campaign loading and rendering
5. **Phase 5** (Documentation): Document implementation and inventory

**Estimated Effort**:

- Phase 1: 2-3 hours (file consolidation and ID assignment)
- Phase 2: 1 hour (update visual_id fields)
- Phase 3: 2-4 hours (domain changes + data updates)
- Phase 4: 3-4 hours (integration testing and debugging)
- Phase 5: 1-2 hours (documentation)

**Total**: 9-14 hours

## Success Metrics

### Functional Metrics

- [ ] All 32 creatures load without errors
- [ ] All 11 monsters have valid creature visuals
- [ ] All 12 NPCs have valid creature visuals
- [ ] Campaign launches successfully
- [ ] Creatures render correctly in both game modes

### Quality Metrics

- [ ] Zero RON parsing errors
- [ ] Zero broken creature references
- [ ] All integration tests pass
- [ ] No performance regression (< 5% frame time increase)
- [ ] All documentation complete and accurate

### Coverage Metrics

- [ ] 100% of tutorial monsters mapped to creatures (11/11)
- [ ] 100% of tutorial NPCs mapped to creatures (12/12)
- [ ] 100% of creature files included in database (32/32)
- [ ] 0 placeholder sprites used in normal operation

## Appendix: File Structure

### New Files Created

```
campaigns/tutorial/data/creatures.ron          # Creature database (Phase 1)
campaigns/tutorial/creature_mappings.md        # Mapping reference (Phase 5)
docs/explanation/tutorial_procedural_mesh_integration_plan.md  # This plan
```

### Modified Files

```
campaigns/tutorial/campaign.ron                # Add creatures_file reference
campaigns/tutorial/data/monsters.ron           # Add visual_id to all monsters
campaigns/tutorial/data/npcs.ron              # Add creature_id to all NPCs
campaigns/tutorial/README.md                   # Document creature system
src/domain/world/npc.rs                       # Add creature_id field (if needed)
docs/explanation/implementations.md            # Record implementation
```

### Preserved Files

```
campaigns/tutorial/assets/sprites/placeholders/  # Keep for fallback
campaigns/tutorial/assets/creatures/*.ron         # Source files (referenced by database)
```

## Conclusion

This implementation plan provides a phased approach to integrating procedural mesh creatures into the tutorial campaign. The strategy prioritizes backward compatibility, maintains fallback mechanisms, and ensures comprehensive testing at each phase. By following this plan, the tutorial campaign will transition from placeholder sprites to rich 3D procedural meshes while maintaining stability and extensibility for future content additions.
