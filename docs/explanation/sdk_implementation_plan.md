# Antares SDK Implementation Plan

## Overview

This plan outlines the development of the Antares SDK - a comprehensive toolset for creating custom RPG campaigns. The SDK builds upon the Map Content Implementation Plan as its cornerstone, extending the engine from an MM1 clone into a general-purpose RPG engine with modding support.

**Key Principles:**


## Current State Analysis

### Existing Infrastructure

**✅ SDK Backend Complete (Phases 0-3, 8-9):**

- `src/sdk/campaign_loader.rs` - Campaign loading system with metadata and validation
- `src/sdk/campaign_packager.rs` - Campaign packaging and distribution
- `src/sdk/database.rs` - Unified content database (items, spells, monsters, classes, races, quests, dialogue)
- `src/sdk/validation.rs` - Cross-reference validation and balance checking
- `src/sdk/serialization.rs` - RON format helpers
- `src/sdk/templates.rs` - Content templates for quick creation
- `src/sdk/quest_editor.rs` - Quest validation and analysis helpers
- `src/sdk/dialogue_editor.rs` - Dialogue tree validation helpers
- `src/sdk/map_editor.rs` - Map validation and content browsing
- `src/sdk/error_formatter.rs` - Enhanced error messages with suggestions
- `src/sdk/cache.rs` - Performance optimization caching
- `src/sdk/tool_config.rs` - Shared configuration for SDK tools
- `data/items.ron` - Item database with `ItemDatabase` loader
- `data/monsters.ron` - Monster definitions (planned)
- `data/spells.ron` - Spell definitions (planned)
- `data/maps/*.ron` - Map system (in progress via Map Content Plan)

**✅ Domain Layer Complete:**

- `src/domain/quest.rs` - Complete quest system with stages, objectives, conditions, rewards
- `src/domain/dialogue.rs` - Complete dialogue system with trees, nodes, choices, conditions
- `src/domain/classes.rs` - Data-driven class system
- `src/domain/character.rs` - Character system with quest flags
- `src/domain/items/` - Item system with types and effects
- `src/domain/combat/` - Combat mechanics
- `src/domain/magic/` - Spell system
- `src/domain/world/` - World and map structures

**✅ Campaign Builder GUI - Phase 2 Complete:**

- `sdk/campaign_builder/` - egui-based GUI application
  - ✅ Phase 0: Framework validation (egui confirmed, works without GPU)
  - ✅ Phase 1: Core campaign system backend
  - ✅ Phase 2: Foundation UI (metadata editor, validation, file I/O, unsaved changes tracking)
  - ❌ Phase 3: Data Editors (Items, Spells, Monsters) - **PLACEHOLDER ONLY**
  - ❌ Phase 4: Map Editor Integration - **NOT STARTED**
  - ❌ Phase 5: Quest & Dialogue Tools - **NOT STARTED**
  - ❌ Phase 6: Testing & Distribution - **NOT STARTED**

**✅ Documentation Complete (Phase 8):**

- `docs/reference/sdk_api.md` - Complete SDK API reference
- `docs/tutorials/creating_campaigns.md` - Campaign creation guide
- `docs/how-to/using_sdk_tools.md` - Tool usage guides
- `campaigns/tutorial/` - Complete example campaign
- `docs/explanation/modding_guide.md` - Comprehensive modding guide

**❌ Critical Gaps Remaining:**

- **Campaign Builder GUI Phase 3**: Data editors are placeholders (no CRUD for items/spells/monsters)
- **Campaign Builder GUI Phase 4**: Map editor not integrated into GUI
- **Campaign Builder GUI Phase 5**: Quest designer and dialogue tree editor missing
- **Campaign Builder GUI Phase 6**: Campaign packager, test play, asset manager not integrated
- **Game Engine Integration**: `GameState` lacks campaign support, game can't load/play campaigns
- **CLI Tools Status**: Class/Race/Item editors from phases 5-7 unclear if implemented

**🔧 Existing Patterns to Leverage:**

- `ItemDatabase` - proven pattern for loading/querying RON data
- RON serialization infrastructure - serde + ron crate already integrated
- Type aliases - `ItemId`, `SpellId`, `MonsterId` provide abstraction layer
- SDK helper modules - `quest_editor`, `dialogue_editor`, `map_editor` provide backend functionality
- egui immediate mode patterns - proven in Phase 2 Campaign Builder foundation

### Identified Issues

1. **GUI Incomplete**: Campaign Builder has placeholders for data editors (Phase 3), map integration (Phase 4), quest/dialogue tools (Phase 5)
2. **Game Can't Load Campaigns**: No campaign integration in `GameState` or main game CLI
3. **Disconnected Tools**: Map Builder exists as CLI but not integrated into Campaign Builder GUI
4. **Missing Distribution**: Campaign packager exists but not integrated into GUI workflow
5. **Two Planning Documents**: `sdk_implementation_plan.md` (CLI-focused, phases 0-9) vs `sdk_and_campaign_architecture.md` (GUI-focused, phases 0-6) are misaligned

## Implementation Phases

### Phase 0: Map Content Plan Completion (Prerequisite)

**Status:** ✅ COMPLETED

**Deliverables from Map Plan:**

- ✅ Phase 1: Map documentation and validation utility
- ✅ Phase 2: Map Builder tool (CLI MVP with UX improvements)
- ✅ Phase 3: Starter content (town, dungeon, forest maps)

**Timeline:** 3 weeks (completed)

**Success Criteria:**

- ✅ Map Builder tool functional with enhanced UX
- ✅ Starter maps created and tested
- ✅ Map RON format documented

**Note:** Map Builder is now the SDK's flagship tool foundation.

---

### Phase 0.5: UI Framework Validation (Prerequisite)

**Status:** ✅ COMPLETED

**Goal:** Validate UI framework choice for Campaign Builder SDK through empirical testing

**Duration:** 1 week (completed)

#### 0.5.1 Framework Prototypes Built

**egui Prototype:**

- Location: `sdk/campaign_builder/`
- Lines of code: 474
- Architecture: Immediate mode GUI
- Status: ✅ Working, kept

**iced Prototype:**

- Location: `sdk/campaign_builder_iced/` (removed)
- Lines of code: 510
- Architecture: Elm Architecture (Model-View-Update)
- Status: ❌ Failed in production, removed

#### 0.5.2 Testing Results

**Environment Testing:**

| Environment              | egui         | iced          | Winner |
| ------------------------ | ------------ | ------------- | ------ |
| Desktop with GPU         | 60 FPS ✅    | 60 FPS ✅     | Tie    |
| Software render (no GPU) | 30-60 FPS ✅ | 10-30 FPS ⚠️  | egui   |
| VM without GPU           | 35-45 FPS ✅ | **Failed** ❌ | egui   |
| Headless (Xvfb)          | Works ✅     | Difficult ❌  | egui   |

**Fatal Error (iced):**

```
error 7: failed to import supplied dmabufs:
Could not bind the given EGLImage to a CoglTexture2D
```

**Root Cause:** iced requires GPU hardware acceleration (DMA-BUF), unavailable in VM/headless environments.

#### 0.5.3 Decision Matrix

| Criterion           | Weight      | egui       | iced       | Winner   |
| ------------------- | ----------- | ---------- | ---------- | -------- |
| **No GPU Required** | 🔴 CRITICAL | 10/10 ⭐   | 3/10 ❌    | egui     |
| Code simplicity     | High        | 9/10       | 6/10       | egui     |
| Learning curve      | High        | 9/10       | 6/10       | egui     |
| Iteration speed     | High        | 9/10       | 6/10       | egui     |
| **Weighted Total**  |             | **8.4/10** | **6.5/10** | **egui** |

#### 0.5.4 Deliverables

- ✅ egui prototype: `sdk/campaign_builder/` (474 lines, fully functional)
- ✅ Comparison document: `sdk/campaign_builder/FRAMEWORK_DECISION.md`
- ✅ Documentation: Updated architecture and implementation docs
- ✅ iced prototype removed after GPU failure validation

#### 0.5.5 Success Criteria

- ✅ Both prototypes built with identical features
- ✅ GPU requirements tested empirically
- ✅ Real-world failure observed and documented (iced DMA-BUF error)
- ✅ egui validated as working in all target environments
- ✅ Framework decision documented and final

**Result:** egui definitively confirmed as the UI framework for Antares SDK. No GPU required, works everywhere from gaming PCs to headless CI/CD servers.

**References:**

- Framework decision: `sdk/campaign_builder/FRAMEWORK_DECISION.md`
- Architecture section: `docs/explanation/sdk_and_campaign_architecture.md#technology-stack`
- Implementation log: `docs/explanation/implementations.md#iced-framework-comparison-prototype`

---

### Phase 1: Data-Driven Class System

**Goal:** Migrate Character classes from hardcoded enum to data-driven RON definitions

**Duration:** 5-7 days

#### 1.1 Class Definition Data Structure

**File:** `src/domain/character.rs` (modify existing)

**Changes:**

- Reuse error patterns from `items/database.rs`
- Use existing `thiserror` for error types

#### 1.3 Create Class Data File

**File:** `data/classes.ron` (create new)

**Content:** Define all 6 MM1 classes

```ron
[
    ClassDefinition(
        id: "knight",
        name: "Knight",
        hp_die: (count: 1, sides: 10, bonus: 0),
        spell_school: None,
        is_pure_caster: false,
        spell_stat: None,
        disablement_bit: 0b00000001,
        special_abilities: ["multiple_attacks"],
    ),
    ClassDefinition(
        id: "paladin",
        name: "Paladin",
        hp_die: (count: 1, sides: 8, bonus: 0),
        spell_school: Some(Cleric),
        is_pure_caster: false,
        spell_stat: Some(Personality),
        disablement_bit: 0b00000010,
        special_abilities: [],
    ),
    // ... 4 more classes
]
```

#### 1.4 Refactor Game Systems

**Files to Modify:**

```rust
// OLD: Direct enum matching
pub fn roll_hp_gain(class: Class, rng: &mut impl Rng) -> u16 {
    let dice = match class {
        Class::Knight => DiceRoll::new(1, 10, 0),
        // ... 5 more cases
    };
    dice.roll(rng).max(1) as u16
}

// NEW: Database lookup
pub fn roll_hp_gain(
    class_id: &ClassId,
    class_db: &ClassDatabase,
    rng: &mut impl Rng,
) -> Result<u16, ProgressionError> {
    let class_def = class_db.get_class(class_id)
        .ok_or(ProgressionError::InvalidClass(class_id.clone()))?;

    let hp = class_def.hp_die.roll(rng).max(1) as u16;
    Ok(hp)
}
```

**Strategy:** Update function signatures to accept `&ClassDatabase` parameter

#### 1.5 Testing Requirements

**Test Files:**

- Load classes from RON string
- Validate all 6 MM1 classes parse correctly
- HP gain produces correct dice ranges
- Spell access logic matches old behavior
- Invalid class ID handling

**Minimum Coverage:** 80% per project standards

#### 1.6 Deliverables

- [ ] `ClassDefinition` struct implemented
- [ ] `ClassDatabase` with load/query methods
- [ ] `data/classes.ron` with 6 MM1 classes
- [ ] `progression.rs` refactored to use database
- [ ] `casting.rs` refactored to use database
- [ ] `items/types.rs` Disablement updated
- [ ] All existing tests pass with new system
- [ ] Documentation updated in `docs/explanation/implementations.md`

#### 1.7 Success Criteria

- ✅ `cargo test --all-features` passes with 0 failures
- ✅ `cargo clippy --all-targets --all-features -- -D warnings` shows 0 warnings
- ✅ Character creation works with `ClassId` strings
- ✅ Level-up HP gain matches original class dice
- ✅ Spell casting restrictions preserved
- ✅ No behavioral changes from user perspective

---

### Phase 2: Data-Driven Race System

**Goal:** Migrate Character races to data-driven definitions (similar to classes)

**Duration:** 3-4 days

#### 2.1 Race Definition Data Structure

**File:** `src/domain/character.rs` (modify existing)

```rust
pub struct RaceDefinition {
    pub id: String,                      // "human", "elf", "dwarf"
    pub name: String,                    // "Human", "Elf", "Dwarf"
    pub stat_modifiers: StatModifiers,   // +/- to base stats
    pub resistances: ResistanceModifiers,
    pub special_abilities: Vec<String>,
    pub disablement_bit: u8,             // For race-restricted items (future)
}

pub struct StatModifiers {
    pub might: i8,
    pub intellect: i8,
    pub personality: i8,
    pub endurance: i8,
    pub speed: i8,
    pub accuracy: i8,
    pub luck: i8,
}

pub type RaceId = String;
```

#### 2.2 Race Database Implementation

**File:** `src/domain/character/race_database.rs` (create new)

**Pattern:** Copy `ClassDatabase` structure, adapt for races

```rust
pub struct RaceDatabase {
    races: HashMap<RaceId, RaceDefinition>,
}

impl RaceDatabase {
    pub fn load_from_file(path: &Path) -> Result<Self, RaceDatabaseError>;
    pub fn get_race(&self, id: &RaceId) -> Option<&RaceDefinition>;
    pub fn all_races(&self) -> Vec<&RaceDefinition>;
}
```

#### 2.3 Create Race Data File

**File:** `data/races.ron` (create new)

**Content:** Define 5 MM1 races with stat modifiers

```ron
[
    RaceDefinition(
        id: "human",
        name: "Human",
        stat_modifiers: (
            might: 0, intellect: 0, personality: 0,
            endurance: 0, speed: 0, accuracy: 0, luck: 0,
        ),
        resistances: (),
        special_abilities: [],
        disablement_bit: 0b00000001,
    ),
    RaceDefinition(
        id: "elf",
        name: "Elf",
        stat_modifiers: (
            might: -2, intellect: 2, personality: 0,
            endurance: -1, speed: 1, accuracy: 2, luck: 0,
        ),
        resistances: (),
        special_abilities: ["resist_sleep"],
        disablement_bit: 0b00000010,
    ),
    // ... 3 more races
]
```

#### 2.4 Integrate with Character Creation

**File:** `src/domain/character.rs` (modify)

**Changes:**

- Load races from RON
- Stat modifiers apply correctly
- All MM1 races parse successfully
- Character creation with race IDs works

#### 2.6 Deliverables

- [ ] `RaceDefinition` and `RaceDatabase` implemented
- [ ] `data/races.ron` with 5 MM1 races
- [ ] Character creation updated to use `RaceId`
- [ ] Stat modifiers applied during character creation
- [ ] Tests updated and passing

#### 2.7 Success Criteria

- ✅ All quality checks pass (cargo test/clippy)
- ✅ Character stats reflect race modifiers
- ✅ No behavioral changes from user perspective
- ✅ Documentation updated

---

### Phase 3: SDK Foundation Module

**Goal:** Create SDK infrastructure for tool development and validation

**Duration:** 4-5 days

#### 3.1 SDK Module Structure

**File:** `src/sdk/mod.rs` (create new)

**Structure:**

/// Central database for all game content
///
/// Aggregates all content databases for unified access and validation.
pub struct ContentDatabase {
    pub classes: ClassDatabase,
    pub races: RaceDatabase,
    pub items: ItemDatabase,
    pub monsters: MonsterDatabase,  // When implemented
    pub spells: SpellDatabase,      // When implemented
    pub maps: Vec<Map>,             // Loaded maps
}

impl ContentDatabase {
    /// Load all content from a campaign directory
    ///
    /// Expected structure:
    /// ```text
    /// campaign_dir/
    /// ├── data/
    /// │   ├── classes.ron
    /// │   ├── races.ron
    /// │   ├── items.ron
    /// │   ├── monsters.ron
    /// │   └── spells.ron
    /// └── maps/
    ///     ├── town_01.ron
    ///     └── dungeon_01.ron
    /// ```
    pub fn load_campaign(path: &Path) -> Result<Self, SdkError>;

    /// Load just the core engine data (for testing)
    pub fn load_core() -> Result<Self, SdkError>;

    /// Validate entire campaign for consistency
    pub fn validate(&self) -> Vec<ValidationError>;

    /// Get statistics about loaded content
    pub fn stats(&self) -> ContentStats;
}

pub struct ContentStats {
    pub class_count: usize,
    pub race_count: usize,
    pub item_count: usize,
    pub monster_count: usize,
    pub spell_count: usize,
    pub map_count: usize,
}
````

#### 3.3 Cross-Reference Validation

**File:** `src/sdk/validation.rs` (create new)

```rust
pub enum ValidationError {
    // ID Reference Errors
    MissingClass { context: String, class_id: ClassId },
    MissingRace { context: String, race_id: RaceId },
    MissingItem { context: String, item_id: ItemId },
    MissingMonster { map: String, monster_id: MonsterId },
    MissingSpell { context: String, spell_id: SpellId },

    // Structural Errors
    DisconnectedMap { map_id: MapId },
    DuplicateId { entity_type: String, id: String },

    // Balance Warnings
    BalanceWarning {
        severity: WarningSeverity,
        message: String,
    },
}

pub struct Validator {
    db: ContentDatabase,
}

impl Validator {
    /// Validate all content and return errors
    pub fn validate_all(&self) -> Vec<ValidationError>;

    /// Check if all referenced IDs exist
    fn validate_references(&self) -> Vec<ValidationError>;

    /// Check for orphaned or unreachable content
    fn validate_connectivity(&self) -> Vec<ValidationError>;

    /// Heuristic balance checks (optional warnings)
    fn check_balance(&self) -> Vec<ValidationError>;
}
```

**Validation Rules:**

- Load campaign from directory structure
- Validation catches missing item references
- Validation catches duplicate IDs
- RON formatting produces parseable output
- Template functions create valid content

#### 3.7 Deliverables

- [ ] `src/sdk/mod.rs` with public API
- [ ] `ContentDatabase` aggregates all content
- [ ] `Validator` with cross-reference checks
- [ ] Serialization helpers for RON
- [ ] Template functions for common patterns
- [ ] SDK documented in `docs/reference/sdk_api.md`

#### 3.8 Success Criteria

- ✅ `ContentDatabase::load_campaign()` loads all data
- ✅ Validator catches reference errors in test data
- ✅ Map Builder can use SDK for validation
- ✅ All SDK functions have doc tests
- ✅ No additional dependencies beyond existing

---

### Phase 4: Enhanced Map Builder (SDK Integration)

**Goal:** Enhance Map Builder from Phase 2 with SDK validation

**Duration:** 2-3 days

**Note:** This builds on completed Map Content Plan Phase 2

#### 4.1 Add SDK Dependency to Map Builder

**File:** `tools/map-builder/main.rs` (modify existing)

**Changes:**

- Map Builder works without SDK (graceful degradation)
- Map Builder with SDK provides enhanced validation
- Invalid IDs trigger suggestions
- List commands show available content

#### 4.5 Deliverables

- [ ] Map Builder optionally loads `ContentDatabase`
- [ ] Enhanced validation uses SDK when available
- [ ] ID suggestions on validation errors
- [ ] `list` command for browsing content
- [ ] Documentation updated in map builder guide

#### 4.6 Success Criteria

- ✅ Map Builder still works standalone (no SDK required)
- ✅ With SDK, invalid references show helpful suggestions
- ✅ `list` commands display available content
- ✅ No breaking changes to existing Map Builder workflow
- ✅ Documentation includes SDK features

---

### Phase 5: Class/Race Editor Tool (CLI)

**Goal:** Create CLI editors for classes and races using SDK foundation

**Duration:** 3-4 days

**Note:** This provides CLI tooling. The egui-based Campaign Builder UI (Phase 2 of Campaign Architecture) will provide visual editors for these in a unified interface.

#### 5.1 Class Editor Implementation

**File:** `tools/class-editor/main.rs` (create new)

**Binary:** `cargo run --bin class-editor data/classes.ron`

```rust
//! Interactive class definition editor
//!
//! Features:
//! - Add/edit/remove classes
//! - Set HP dice, spell access, restrictions
//! - Preview class in action
//! - Validate against architecture constraints

use antares::sdk::{ContentDatabase, ClassDefinition};

struct ClassEditor {
    classes: Vec<ClassDefinition>,
    current_index: Option<usize>,
}

impl ClassEditor {
    fn run(&mut self) {
        loop {
            self.show_menu();
            let choice = self.get_input("Choice: ");

            match choice.as_str() {
                "1" => self.list_classes(),
                "2" => self.add_class(),
                "3" => self.edit_class(),
                "4" => self.delete_class(),
                "5" => self.preview_class(),
                "6" => self.save_and_exit(),
                "q" => break,
                _ => println!("Invalid choice"),
            }
        }
    }

    fn add_class(&mut self) {
        println!("\n=== Create New Class ===");

        let id = self.get_input("Class ID (lowercase, e.g., 'barbarian'): ");
        let name = self.get_input("Display Name: ");

        println!("\nHP Gain Die:");
        println!("1. 1d4 (Weak - Sorcerer)");
        println!("2. 1d6 (Low - Cleric, Robber)");
        println!("3. 1d8 (Medium - Paladin, Archer)");
        println!("4. 1d10 (High - Knight)");
        println!("5. 1d12 (Very High - Custom)");
        let die = self.get_hp_die();

        println!("\nSpell Access:");
        println!("1. None (Warrior classes)");
        println!("2. Cleric - Full (Cleric)");
        println!("3. Sorcerer - Full (Sorcerer)");
        println!("4. Cleric - Hybrid (Paladin)");
        println!("5. Sorcerer - Hybrid (Archer)");
        let spell_info = self.get_spell_access();

        // ... collect remaining fields ...

        let class_def = ClassDefinition {
            id, name, hp_die: die,
            spell_school: spell_info.0,
            is_pure_caster: spell_info.1,
            spell_stat: spell_info.2,
            disablement_bit: self.get_next_bit(),
            special_abilities: vec![],
        };

        self.classes.push(class_def);
        println!("✅ Class created");
    }

    fn preview_class(&self) {
        // Show sample character with this class at levels 1, 5, 10
    }

    fn save_and_exit(&self) -> Result<(), Box<dyn Error>> {
        let ron = ron::ser::to_string_pretty(&self.classes, Default::default())?;
        std::fs::write("data/classes.ron", ron)?;
        println!("✅ Saved to data/classes.ron");
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let file_path = args.get(1).map(|s| s.as_str()).unwrap_or("data/classes.ron");

    let mut editor = ClassEditor::load(file_path)?;
    editor.run();

    Ok(())
}
```

#### 5.2 Race Editor Implementation

**File:** `tools/race-editor/main.rs` (create new)

**Similar structure to Class Editor, adapted for races:**

- [ ] Create new class and save to RON
- [ ] Load existing classes and edit
- [ ] Preview shows reasonable character stats
- [ ] Saved RON file loads in game engine
- [ ] Validation catches invalid configurations

#### 5.4 Deliverables

- [ ] `tools/class-editor/` with interactive CLI
- [ ] `tools/race-editor/` with interactive CLI
- [ ] Both tools support load/edit/save workflow
- [ ] Preview functionality for design validation
- [ ] Documentation in `docs/how_to/using_class_editor.md`
- [ ] Documentation in `docs/how_to/using_race_editor.md`

#### 5.5 Success Criteria

- ✅ Non-programmers can create classes via CLI prompts
- ✅ Generated RON files are valid and load in engine
- ✅ Editors prevent common mistakes (duplicate IDs, etc.)
- ✅ Documentation clear enough for external users
- ✅ Tools installable via `cargo install --path tools/class-editor`

---

### Phase 6: Campaign Validator Tool

**Goal:** Comprehensive validation tool for complete campaigns

**Duration:** 2-3 days

#### 6.1 Validator Tool Implementation

**File:** `tools/campaign-validator/main.rs` (create new)

**Binary:** `cargo run --bin campaign-validator campaigns/my_campaign/`

```rust
//! Campaign validation tool
//!
//! Validates entire campaigns for:
//! - Reference integrity (all IDs exist)
//! - Structural issues (disconnected maps)
//! - Balance problems (level 1 area with dragons)
//! - Content completeness

use antares::sdk::{ContentDatabase, ValidationError, Validator};

fn main() -> Result<(), Box<dyn Error>> {
    let campaign_path = env::args()
        .nth(1)
        .expect("Usage: campaign-validator <campaign-directory>");

    println!("🔍 Validating campaign: {}", campaign_path);
    println!();

    // Load all content
    println!("📦 Loading content...");
    let db = ContentDatabase::load_campaign(Path::new(&campaign_path))?;

    println!("   ✅ {} classes", db.stats().class_count);
    println!("   ✅ {} races", db.stats().race_count);
    println!("   ✅ {} items", db.stats().item_count);
    println!("   ✅ {} monsters", db.stats().monster_count);
    println!("   ✅ {} spells", db.stats().spell_count);
    println!("   ✅ {} maps", db.stats().map_count);
    println!();

    // Validate
    println!("🔍 Validating references...");
    let validator = Validator::new(db);
    let errors = validator.validate_all();

    // Report results
    if errors.is_empty() {
        println!();
        println!("✅ Campaign is valid!");
        println!("   No errors found.");
        return Ok(());
    }

    // Categorize errors
    let mut critical = vec![];
    let mut warnings = vec![];

    for error in errors {
        match error {
            ValidationError::BalanceWarning { .. } => warnings.push(error),
            _ => critical.push(error),
        }
    }

    // Show critical errors
    if !critical.is_empty() {
        println!();
        println!("❌ Critical Errors ({})", critical.len());
        for error in critical {
            println!("   {}", format_error(&error));
        }
    }

    // Show warnings
    if !warnings.is_empty() {
        println!();
        println!("⚠️  Warnings ({})", warnings.len());
        for warning in warnings {
            println!("   {}", format_error(&warning));
        }
    }

    // Exit with error code if critical errors exist
    if critical.is_empty() {
        println!();
        println!("✅ No critical errors (warnings can be ignored)");
        Ok(())
    } else {
        Err("Campaign validation failed".into())
    }
}

fn format_error(error: &ValidationError) -> String {
    match error {
        ValidationError::MissingMonster { map, monster_id } => {
            format!("Map '{}' references non-existent monster ID: {}", map, monster_id)
        }
        ValidationError::MissingItem { context, item_id } => {
            format!("{} references non-existent item ID: {}", context, item_id)
        }
        ValidationError::DisconnectedMap { map_id } => {
            format!("Map '{}' is not reachable from starting location", map_id)
        }
        ValidationError::BalanceWarning { severity, message } => {
            format!("[{:?}] {}", severity, message)
        }
        _ => format!("{:?}", error),
    }
}
```

#### 6.2 Validation Rules Implementation

**Checks to Implement:**

1. **Reference Integrity:**

   - No duplicate IDs within content type
   - All maps are reachable (connected graph)
   - No circular dependencies
   - Required content exists (at least 1 town, 1 class, etc.)

3. **Balance Warnings (Heuristic):**
   - Low-level areas don't have high-level monsters
   - Starter equipment is available in town
   - Progression curve is reasonable
   - No impossible monster/party level matchups

#### 6.3 Testing Requirements

**Test Campaigns:**

#### 7.1 Item Editor Implementation

**File:** `tools/item-editor/main.rs` (create new)

**Features:**

- Create each item type (weapon, armor, consumable, etc.)
- Edit existing items
- Class restrictions work correctly
- Saved RON loads in game engine

#### 7.3 Deliverables

- [ ] `tools/item-editor/` interactive CLI
- [ ] Create/edit/delete items
- [ ] Guided prompts for all item types
- [ ] Class restriction selector
- [ ] Preview item stats
- [ ] Documentation in `docs/how_to/using_item_editor.md`

#### 7.4 Success Criteria

- ✅ Can create all item types without manual RON editing
- ✅ Class restrictions intuitive to set
- ✅ Generated items load correctly in game
- ✅ Tool prevents common mistakes
- ✅ Documentation suitable for non-programmers

---

### Phase 8: Documentation and Examples

**Goal:** Comprehensive documentation for SDK and campaign creation

**Duration:** 4-5 days

#### 8.1 SDK API Reference

**File:** `docs/reference/sdk_api.md` (create new)

**Content:**

- Quick start: Your first campaign
- Directory structure requirements
- Step-by-step workflow:
  1. Define classes and races
  2. Create items
  3. Design monsters
  4. Build maps
  5. Validate campaign
  6. Package for distribution
- Common patterns and best practices
- Troubleshooting guide

#### 8.3 Tool Usage Guides

**Files to Create:**

- 2-3 custom classes
- 2 custom races
- 10-15 custom items
- 5-10 custom monsters
- 3-4 small maps
- Quest progression example

**Purpose:** Learning reference for modders

#### 8.5 Modding Guide

**File:** `docs/tutorials/modding_guide.md` (create new)

**Content:**

- Map Builder suggests items from Item Editor database
- Class Editor validates against Race Editor data
- All tools use consistent UI/UX patterns
- Shared configuration file for tool preferences

#### 9.2 Error Message Improvements

**Review and enhance:**

- Campaign loading times
- Validation performance on large campaigns
- RON serialization/deserialization
- Map Builder responsiveness

#### 9.4 Quality Assurance

**Final checks:**

- Type aliases: `ClassId`, `RaceId`, `ItemId`, etc.
- RON format for all data files (Section 7.1-7.2)
- Module structure respects domain/application layers
- No circular dependencies introduced

**New Additions (Compatible):**

- Character struct uses `ClassId` instead of `Class` enum
- Function signatures add `&ClassDatabase` parameters

**Migration Strategy:**

- All SDK modules: >80% coverage
- Database loaders: 100% coverage
- Validation logic: >90% coverage

**Test Data:**

- Load complete MM1-style campaign
- Create campaign using SDK tools
- Validate campaign with known errors
- Modify campaign and re-validate

### Manual Testing

**User Acceptance:**

- [ ] Campaign load time <2 seconds for 100 maps
- [ ] Validation completes <5 seconds for 100 maps

### Qualitative

- [ ] External tester successfully creates campaign
- [ ] Documentation rated "clear and complete"
- [ ] Tools rated "easy to use"
- [ ] No critical bugs in SDK or tools

## Timeline Summary

| Phase                                    | Duration      | Cumulative   |
| ---------------------------------------- | ------------- | ------------ |
| Phase 0: Map Content Plan (prerequisite) | 3 weeks       | 3 weeks      |
| Phase 1: Data-Driven Classes             | 5-7 days      | ~4 weeks     |
| Phase 2: Data-Driven Races               | 3-4 days      | ~5 weeks     |
| Phase 3: SDK Foundation                  | 4-5 days      | ~6 weeks     |
| Phase 4: Enhanced Map Builder            | 2-3 days      | ~6.5 weeks   |
| Phase 5: Class/Race Editor               | 3-4 days      | ~7 weeks     |
| Phase 6: Campaign Validator              | 2-3 days      | ~7.5 weeks   |
| Phase 7: Item Editor                     | 3-4 days      | ~8 weeks     |
| Phase 8: Documentation                   | 4-5 days      | ~9 weeks     |
| Phase 9: Integration & Polish            | 3-4 days      | ~10 weeks    |
| **Total**                                | **~10 weeks** | **10 weeks** |

**Note:** Assumes part-time development (~20 hours/week). Full-time could reduce to 5-6 weeks.

## Risk Management

### Technical Risks

| Risk                                | Mitigation                                            |
| ----------------------------------- | ----------------------------------------------------- |
| Performance of data-driven lookup   | Profile early, optimize hot paths, consider caching   |
| RON parsing errors confuse users    | Enhance error messages, provide validation tools      |
| Complex refactoring introduces bugs | Comprehensive tests before/after, incremental changes |

### Project Risks

| Risk                              | Mitigation                                      |
| --------------------------------- | ----------------------------------------------- |
| Scope creep                       | Stick to phases, defer enhancements to post-SDK |
| Documentation lags implementation | Write docs as you build, not after              |
| Tools hard to use                 | Early user testing, iterate on UX               |

---

### Phase 10: Campaign Builder GUI - Data Editors (Phase 3)

**Goal:** Implement full CRUD editors for Items, Spells, and Monsters in Campaign Builder GUI

**Duration:** 2-3 weeks

**Status:** 🔲 NOT STARTED (Current: Placeholder views only)

**Dependencies:** Phase 2 Campaign Builder foundation, SDK backend (database, validation)

#### 10.1 Items Editor Implementation

**File:** `sdk/campaign_builder/src/main.rs` - Replace `show_items_editor` placeholder

**Features to Implement:**

- **List View**: Display all items from `campaign.items_file` with search/filter
- **Add Item Form**: Select item type (Weapon, Armor, Consumable, Quest), fill stats, set disablement flags
- **Edit Item Form**: Modify existing item in-place with validation
- **Delete Item**: Remove with confirmation dialog
- **Preview Panel**: Show item stats, restrictions, description as formatted display
- **Search/Filter**: By name, type, level requirement, class restrictions
- **Validation**: Real-time check against `ContentDatabase` rules
- **Integration**: Use `src/sdk/database.rs` `ItemDatabase` for loading/saving

**UI Layout:**

- Left: Searchable list with add/delete buttons
- Right: Edit form or preview panel
- Bottom: Validation errors/warnings display
- Auto-save integration with unsaved changes tracking

#### 10.2 Spells Editor Implementation

**File:** `sdk/campaign_builder/src/main.rs` - Replace `show_spells_editor` placeholder

**Features to Implement:**

- **List View**: All spells grouped by school (Cleric/Sorcerer) and level
- **Add Spell Form**: School, level, name, SP cost, target type, effect configuration
- **Edit Spell Form**: Modify spell parameters with validation
- **Delete Spell**: Remove with confirmation
- **Preview Panel**: Show spell effect description, targeting, requirements
- **Filter**: By school, level range, effect type
- **Validation**: Check SP cost formulas, level progression balance
- **Integration**: Use `src/sdk/database.rs` `SpellDatabase`

#### 10.3 Monsters Editor Implementation

**File:** `sdk/campaign_builder/src/main.rs` - Replace `show_monsters_editor` placeholder

**Features to Implement:**

- **List View**: All monsters with basic stats (HP, AC, level)
- **Add Monster Form**: Stats editor, loot table builder, special abilities checkboxes
- **Edit Monster Form**: Full monster configuration
- **Delete Monster**: Remove with dependency check (used in maps?)
- **Preview Panel**: Combat simulator preview showing expected damage/XP
- **Loot Table Editor**: Add items with drop rates
- **Special Abilities**: Regeneration, advancement, special attacks
- **Validation**: Balance checking against party power level
- **Integration**: Use `src/sdk/database.rs` `MonsterDatabase`

#### 10.4 Shared Data Editor Components

**Reusable UI Components:**

- Search bar with instant filtering
- Sortable data tables (by name, level, type)
- Form validation with inline error messages
- Confirmation dialogs for destructive actions
- Undo/redo system (basic - track last action)
- Import/export buttons (RON file operations)

#### 10.5 Integration with Validation System

**Connect to SDK Backend:**

- Use `src/sdk/validation.rs` `Validator` for cross-reference checks
- Display validation results in dedicated panel
- Mark items/spells/monsters with errors in list view
- Provide "Fix" button that opens relevant editor

#### 10.6 Testing Requirements

**Unit Tests:**

- [ ] Item CRUD operations (add, edit, delete)
- [ ] Spell CRUD operations with level restrictions
- [ ] Monster CRUD operations with loot tables
- [ ] Search and filter functionality
- [ ] Validation integration
- [ ] RON serialization round-trip

**Manual Testing:**

- [ ] Create 10+ items of each type
- [ ] Create spell progression for both schools (levels 1-7)
- [ ] Create monster set for dungeon (easy to hard)
- [ ] Test search with partial names
- [ ] Verify validation catches invalid references
- [ ] Test unsaved changes warning on tab switch

#### 10.7 Deliverables

- [ ] Full Items editor replacing placeholder in `show_items_editor`
- [ ] Full Spells editor replacing placeholder in `show_spells_editor`
- [ ] Full Monsters editor replacing placeholder in `show_monsters_editor`
- [ ] Shared UI component library for data editors
- [ ] Integration with `ContentDatabase` and `Validator`
- [ ] Updated `sdk/campaign_builder/README.md` documenting editors
- [ ] Tests achieving >80% coverage for editor logic
- [ ] `docs/explanation/implementations.md` updated with Phase 10 summary

#### 10.8 Success Criteria

- ✅ User can create/edit/delete items without touching RON files
- ✅ User can create/edit/delete spells with automatic validation
- ✅ User can create/edit/delete monsters with loot tables
- ✅ Search and filter work instantly on large datasets (100+ items)
- ✅ Validation errors shown in real-time with actionable messages
- ✅ All quality gates pass (fmt, clippy, tests)
- ✅ No crashes or data loss during editor operations

---

### Phase 11: Campaign Builder GUI - Map Editor Integration (Phase 4)

**Goal:** Integrate map editing into Campaign Builder GUI

**Duration:** 2 weeks

**Status:** 🔲 NOT STARTED

**Dependencies:** Phase 10 (data editors), existing CLI map_builder tool

**Decision:** Rewrite CLI map_builder as egui component within Campaign Builder (per user decision)

#### 11.1 Map Editor Component Architecture

**New File:** `sdk/campaign_builder/src/map_editor_component.rs`

**Component Structure:**

- Reusable egui widget for map editing
- Grid-based tile placement UI
- Event editor integrated into map view
- Uses `src/sdk/map_editor.rs` helpers for validation
- Saves to `campaign.maps_dir` in RON format

**Features to Port from CLI Map Builder:**

- Grid rendering with zoom controls
- Tile palette selection
- Event placement and configuration
- Map properties editor (size, name, terrain type)
- Connection point editor (north/south/east/west exits)
- Validation with content database (check monster IDs, item IDs)

#### 11.2 Map List View Integration

**File:** `sdk/campaign_builder/src/main.rs` - Replace `show_maps_editor` placeholder

**Features:**

- **List View**: All maps in `campaign.maps_dir` with thumbnail previews
- **Add Map**: Launch map editor component with new map
- **Edit Map**: Open existing map in editor component
- **Delete Map**: Remove with connectivity check
- **Map Properties**: Quick edit for name, size, starting position
- **Interconnection Manager**: Visual graph showing map connections
- **Validation**: Check all maps for disconnected areas

#### 11.3 Map Preview Panel

**Visual Display:**

- ASCII/tile representation of map
- Highlighted events and connections
- Minimap overview for large maps
- Click to edit in full editor

#### 11.4 Event Editor Improvements

**Enhanced Event Configuration:**

- Dropdown for event types using validated lists
- Item ID selector with search from Items editor
- Monster ID selector from Monsters editor
- NPC dialogue selector from Dialogue editor
- Quest integration (trigger quest stages)

#### 11.5 Testing Requirements

**Unit Tests:**

- [ ] Map creation and saving
- [ ] Tile placement and modification
- [ ] Event configuration
- [ ] Map interconnection validation
- [ ] RON serialization for maps

**Manual Testing:**

- [ ] Create interconnected town and dungeon maps
- [ ] Place events with item/monster references
- [ ] Verify validation catches invalid references
- [ ] Test zoom and pan controls
- [ ] Ensure large maps (100x100) perform well

#### 11.6 Deliverables

- [ ] Map editor component (`map_editor_component.rs`)
- [ ] Map list view replacing placeholder
- [ ] Event editor with content database integration
- [ ] Map interconnection visualizer
- [ ] Updated documentation
- [ ] Tests for map editing functionality

#### 11.7 Success Criteria

- ✅ User can create maps entirely in GUI
- ✅ Map editor replicates CLI map_builder functionality
- ✅ Events validate against items/monsters/quests
- ✅ Map connections visualized and validated
- ✅ No need to use CLI tools for map creation

---

### Phase 12: Campaign Builder GUI - Quest & Dialogue Tools (Phase 5)

**Goal:** Implement visual quest designer and dialogue tree editor

**Duration:** 2-3 weeks

**Status:** 🔲 NOT STARTED

**Dependencies:** Phase 10 (data editors), Phase 11 (map integration)

**Decision:** List-based editors for Phase 12, node-graph visualization deferred to Phase 15 (per user decision)

#### 12.1 Quest Designer Implementation

**File:** `sdk/campaign_builder/src/main.rs` - Replace `show_quests_editor` placeholder

**Features:**

- **Quest List View**: All quests with status (active/completed/blocked)
- **Add Quest Form**: Name, description, quest giver
- **Stage Editor**: Add/edit/delete quest stages in order
- **Objective Builder**: Per-stage objectives with types (kill, collect, visit)
- **Prerequisite Chain**: Select prerequisite quests from list
- **Reward Configuration**: Gold, XP, items, unlocked dialogue
- **Validation**: Use `src/sdk/quest_editor.rs` helpers
- **Preview**: Text description of quest flow

**Quest Stage Types:**

- Kill monsters (select from monster database)
- Collect items (select from item database)
- Visit location (select map + coordinates)
- Talk to NPC (select dialogue tree)
- Custom flag (manual flag ID)

#### 12.2 Dialogue Tree Editor Implementation

**File:** `sdk/campaign_builder/src/main.rs` - Add dialogue tree editor tab

**Features:**

- **Tree List View**: All dialogue trees with root node info
- **Node List**: Show all nodes in selected tree (list-based, not graph)
- **Add Node Form**: Speaker, text, conditions
- **Choice Editor**: Add player responses with target nodes
- **Condition Builder**: Check quest flags, inventory, stats
- **Action Configuration**: Set quest flags, give items, trigger events
- **Navigation**: Click target node to jump to it in list
- **Validation**: Use `src/sdk/dialogue_editor.rs` helpers
- **Preview**: Text walkthrough of dialogue flow

**Condition Types:**

- Quest completed/active
- Item in inventory
- Stat check (charisma, reputation)
- Flag set
- Gold amount

**Action Types:**

- Start/complete quest
- Give/take item
- Give/take gold
- Set flag
- Trigger combat

#### 12.3 Quest-Dialogue Integration

**Cross-Editor Features:**

- Quest editor can select dialogue tree for quest giver
- Dialogue editor can start/complete quests in actions
- Validation checks quest IDs in dialogue actions
- Validation checks dialogue IDs in quest configurations

#### 12.4 Testing Requirements

**Unit Tests:**

- [ ] Quest CRUD operations
- [ ] Quest stage ordering
- [ ] Prerequisite chain validation
- [ ] Dialogue tree CRUD operations
- [ ] Dialogue choice navigation
- [ ] Condition and action configuration
- [ ] Cross-reference validation (quests in dialogue, dialogue in quests)

**Manual Testing:**

- [ ] Create multi-stage quest with branching objectives
- [ ] Create dialogue tree with conditions and branching
- [ ] Link quest to dialogue tree (NPC quest giver)
- [ ] Test prerequisite chains (quest A requires quest B)
- [ ] Verify validation catches orphaned nodes
- [ ] Test condition evaluation preview

#### 12.5 Deliverables

- [ ] Quest designer replacing placeholder
- [ ] Dialogue tree editor (new tab)
- [ ] Quest-dialogue integration
- [ ] Validation integration
- [ ] Updated documentation
- [ ] Tests for quest and dialogue editing

#### 12.6 Success Criteria

- ✅ User can create complex multi-stage quests
- ✅ User can create branching dialogue trees
- ✅ Quest-dialogue integration works seamlessly
- ✅ Validation catches broken references
- ✅ List-based editors are usable for moderate complexity (10-20 nodes)

**Note:** Node-graph visualization for dialogue trees planned for Phase 15 (Polish & Advanced Features)

---

### Phase 13: Campaign Builder GUI - Distribution Tools (Phase 6)

**Goal:** Integrate campaign packaging, testing, and asset management

**Duration:** 1-2 weeks

**Status:** 🔲 NOT STARTED

**Dependencies:** Phases 10-12 (all editors complete)

#### 13.1 Campaign Packager Integration

**File:** `sdk/campaign_builder/src/main.rs` - Add export functionality

**Features:**

- **Export Wizard**: Multi-step process for campaign packaging
- **Validation Check**: Run full campaign validation before export
- **File Selection**: Choose which assets to include (data, maps, README)
- **Compression**: Use `src/sdk/campaign_packager.rs` for .zip creation
- **Metadata Generation**: Auto-generate campaign.ron from editor state
- **Version Bump**: Helper to increment version (1.0.0 → 1.0.1)
- **Distribution**: Save .zip to user-selected location

**Export Checklist:**

- All required files present (campaign.ron, data files)
- Validation passes with zero errors
- README.md exists and complete
- Assets included if present
- Version follows semantic versioning

#### 13.2 Test Play Integration

**Features:**

- **Launch Game Button**: Start Antares with current campaign
- **CLI Integration**: Execute `antares --campaign <id>` as subprocess
- **Output Capture**: Show game stdout/stderr in Campaign Builder log panel
- **Quick Test**: Save campaign, launch game, return to editor
- **Debug Mode**: Launch game with extra logging for testing

**Prerequisites:**

- Game engine must support `--campaign` flag (see Phase 14)
- Campaign must be saved before test play
- Validation should pass (or show warning)

#### 13.3 Asset Manager

**File:** `sdk/campaign_builder/src/main.rs` - Add Assets tab

**Features:**

- **Asset Browser**: Show all files in campaign directory tree
- **Upload Assets**: Drag-and-drop or file picker for images, music, sounds
- **Organize**: Move files into data/, assets/, docs/ subdirectories
- **Preview**: Display text files, show image thumbnails (if possible)
- **Remove**: Delete unused assets with confirmation
- **Validation**: Check referenced assets exist (map tilesets, dialogue portraits)

**Asset Types:**

- Tilesets (for maps)
- Music (for maps/events)
- Sound effects (for combat/events)
- Portraits (for NPCs/dialogue)
- Documentation (README, guides)

#### 13.4 Campaign Installation System

**Features:**

- **Import Campaign**: Load .zip exported from another Campaign Builder
- **Extract**: Unpack to campaigns directory
- **Validation**: Check imported campaign for errors
- **Conflict Resolution**: Handle campaigns with duplicate IDs

#### 13.5 Testing Requirements

**Unit Tests:**

- [ ] Campaign packaging (create .zip)
- [ ] File inclusion/exclusion logic
- [ ] Version validation and incrementing
- [ ] Asset upload and organization
- [ ] Campaign import and extraction

**Manual Testing:**

- [ ] Export campaign as .zip
- [ ] Import campaign .zip on different machine
- [ ] Test play launches game successfully
- [ ] Asset browser shows all files
- [ ] Version bumping works correctly

#### 13.6 Deliverables

- [ ] Export wizard integrated into File menu
- [ ] Test Play button in Tools menu
- [ ] Asset Manager tab
- [ ] Campaign import functionality
- [ ] Updated documentation
- [ ] Tests for packaging and distribution

#### 13.7 Success Criteria

- ✅ User can export campaign as distributable .zip
- ✅ Test play launches game with current campaign
- ✅ Asset manager handles images, music, sounds
- ✅ Imported campaigns work correctly
- ✅ Export includes all necessary files

---

### Phase 14: Game Engine Campaign Integration (CRITICAL)

**Goal:** Enable Antares game engine to load and play custom campaigns

**Duration:** 1-2 weeks

**Status:** 🔲 NOT STARTED (Game currently cannot load campaigns)

**Dependencies:** SDK backend complete (Phase 3), Campaign Builder Phase 2

**Rationale:** This phase is CRITICAL because without it, campaigns created in Campaign Builder cannot be played. Currently, the game has no campaign loading system.

#### 14.1 GameState Campaign Integration

**File:** `src/application/mod.rs` - Modify `GameState` struct

**Changes Needed:**

- Add `campaign: Option<Campaign>` field to `GameState`
- Modify `GameState::new()` to accept optional campaign parameter
- Update `GameState::new_game()` to use campaign config (starting gold, food, map, position)
- Ensure `World` uses campaign's map directory
- Ensure item/spell/monster databases load from campaign data paths

**Example Integration:**

```rust
pub struct GameState {
    pub campaign: Option<Campaign>,  // NEW: Campaign context
    pub world: World,
    pub roster: Roster,
    pub party: Party,
    // ... existing fields
}

impl GameState {
    pub fn new_game(campaign: Option<Campaign>) -> Result<Self, GameError> {
        let config = campaign.as_ref().map(|c| &c.config).unwrap_or_default();

        // Use campaign starting conditions
        let starting_map = config.starting_map;
        let starting_gold = config.starting_gold;
        let starting_food = config.starting_food;

        // ... initialize with campaign data
    }
}
```

#### 14.2 Main Game CLI Campaign Support

**File:** Create or modify main game binary (e.g., `src/bin/antares.rs`)

**CLI Interface:**

```bash
# Launch with specific campaign
antares --campaign my_campaign

# List available campaigns
antares --list-campaigns

# Validate campaign before playing
antares --validate-campaign my_campaign

# Continue last saved game (preserves campaign)
antares --continue

# Load specific save file
antares --load savegame.ron
```

**Implementation:**

- Add CLI argument parser (clap crate)
- Use `src/sdk/campaign_loader.rs` `CampaignLoader` to load campaign
- Pass `Campaign` to `GameState::new_game()`
- Handle errors gracefully (campaign not found, invalid campaign)

#### 14.3 Save Game Campaign Reference

**File:** Modify save game format (wherever save/load is implemented)

**Changes:**

- Add `campaign_reference: Option<CampaignReference>` to save game structure
- `CampaignReference` stores campaign ID, version, name
- On load, verify campaign is still available
- On load, verify campaign version matches (or is compatible)
- Warn user if campaign is missing/changed

**Save Game Structure:**

```rust
pub struct SaveGame {
    pub version: String,
    pub timestamp: SystemTime,
    pub campaign_reference: Option<CampaignReference>,  // NEW
    pub game_state: GameState,
}

pub struct CampaignReference {
    pub id: String,
    pub version: String,
    pub name: String,
}
```

#### 14.4 Campaign Data Loading

**Integration Points:**

- `ItemDatabase::load()` should use campaign's items file path
- `SpellDatabase::load()` should use campaign's spells file path
- `MonsterDatabase::load()` should use campaign's monsters file path
- `ClassDatabase::load()` should use campaign's classes file path
- `RaceDatabase::load()` should use campaign's races file path
- Map loading should use campaign's maps directory

**Fallback Behavior:**

- If no campaign specified, use default core content
- Core content remains at `data/items.ron`, `data/spells.ron`, etc.
- Campaigns override core content or extend it (configurable)

#### 14.5 Error Handling

**User-Friendly Messages:**

- Campaign not found: List available campaigns
- Campaign validation errors: Show specific issues
- Save game campaign mismatch: Offer to continue anyway or abort
- Missing campaign data files: Show which files are missing

#### 14.6 Testing Requirements

**Unit Tests:**

- [ ] `GameState` creation with campaign
- [ ] Campaign data loading (items, spells, monsters)
- [ ] Save game with campaign reference
- [ ] Load game with campaign reference
- [ ] Campaign version mismatch handling

**Integration Tests:**

- [ ] Launch game with `--campaign tutorial`
- [ ] Verify tutorial campaign data loaded
- [ ] Save game, verify campaign reference stored
- [ ] Load game, verify campaign restored
- [ ] Test with missing campaign (error handling)

**Manual Testing:**

- [ ] Create campaign in Campaign Builder
- [ ] Launch game with `--campaign` flag
- [ ] Verify campaign config applied (starting gold, food, map)
- [ ] Save game during play
- [ ] Load saved game
- [ ] Verify campaign still active after load

#### 14.7 Deliverables

- [ ] `GameState` with campaign support
- [ ] Main game CLI with campaign flags
- [ ] Save game format with campaign reference
- [ ] Campaign data loading integration
- [ ] Error handling and user messages
- [ ] Updated `docs/explanation/implementations.md`
- [ ] Tests for campaign loading and gameplay

#### 14.8 Success Criteria

- ✅ Game launches with `--campaign <id>` flag
- ✅ Campaign config applied (starting conditions)
- ✅ Campaign data loaded (items, spells, monsters, maps)
- ✅ Save games preserve campaign reference
- ✅ Loaded games restore campaign correctly
- ✅ Error messages guide user when campaign missing/invalid
- ✅ Core game still works without campaign (backward compatible)

**CRITICAL:** Without Phase 14, the SDK is non-functional - campaigns can be created but not played!

---

### Phase 15: Polish & Advanced Features (Phase 7)

**Goal:** User experience improvements and advanced features

**Duration:** 2-3 weeks

**Status:** 🔲 NOT STARTED

**Dependencies:** Phases 10-14 complete (all core functionality working)

#### 15.1 Undo/Redo System

**Scope:**

- Campaign metadata changes
- Data editor operations (add/edit/delete items, spells, monsters)
- Map editor tile placement
- Quest and dialogue modifications

**Implementation:**

- Command pattern for all reversible operations
- Undo/redo stack (max 50 actions)
- Keyboard shortcuts (Ctrl+Z, Ctrl+Y)
- Visual indication of undo availability

#### 15.2 Template System

**Features:**

- Pre-built item templates (common weapon/armor sets)
- Monster templates (classic RPG creatures with stats)
- Map templates (town, dungeon, wilderness layouts)
- Quest templates (fetch, kill, escort structures)
- Dialogue templates (merchant, quest giver, guard)

**Integration:**

- "New from Template" buttons in all editors
- Customizable templates (user can save their own)
- Template browser with preview

#### 15.3 Node-Graph Dialogue Visualizer

**Features:**

- Visual node graph for dialogue trees (nodes and edges)
- Drag-and-drop node positioning
- Zoom and pan controls
- Click node to edit in list editor
- Auto-layout algorithm for complex trees
- Export as image for documentation

**Library:** Consider `egui_graphs` or custom implementation

#### 15.4 Advanced Validation Features

**Enhancements:**

- Balance analyzer (party power vs monster difficulty)
- Loot economy checker (gold/item distribution)
- Quest dependency graph visualization
- Unreachable content detector (items never placed, quests never started)
- Difficulty curve analyzer (progression pacing)

#### 15.5 Collaborative Features

**Future-Focused:**

- Export campaign as git-friendly format (separate files per entity)
- Import/merge changes from other contributors
- Diff visualization for campaign changes
- Comment system for content review

#### 15.6 Accessibility Improvements

**Features:**

- Keyboard navigation for all editors
- Screen reader support (ARIA labels)
- High contrast theme option
- Font size adjustment
- Tooltips for all icons and buttons

#### 15.7 Performance Optimizations

**Targets:**

- Large campaigns (1000+ items, 100+ maps)
- Lazy loading for content lists
- Virtual scrolling for large lists
- Background validation (don't block UI)
- Incremental saving (only changed files)

#### 15.8 Testing Requirements

**Manual Testing:**

- [ ] Undo/redo in all editors
- [ ] Template system usability
- [ ] Node-graph dialogue visualization
- [ ] Balance analyzer accuracy
- [ ] Keyboard-only navigation
- [ ] Large campaign performance

#### 15.9 Deliverables

- [ ] Undo/redo system
- [ ] Template library
- [ ] Node-graph dialogue visualizer
- [ ] Advanced validation tools
- [ ] Accessibility features
- [ ] Performance optimizations
- [ ] Updated documentation

#### 15.10 Success Criteria

- ✅ Undo/redo works reliably in all editors
- ✅ Templates speed up content creation significantly
- ✅ Node-graph visualizer handles complex dialogue trees (50+ nodes)
- ✅ Large campaigns (1000+ items) load in <2 seconds
- ✅ Campaign Builder usable with keyboard only
- ✅ Balance analyzer provides actionable feedback

---

## Post-SDK Future Work

**After Phase 15 completion, consider:**

- Scripting language for custom events (Lua/Rhai integration)
- Online mod repository and browser (campaign sharing platform)
- Multiplayer campaign support (co-op gameplay)
- Advanced balance analyzer with AI suggestions
- Automated content generators (procedural dungeons, quests)
- Mobile companion app (character sheets, inventory management)
- Community features (ratings, comments, featured campaigns)

---

## Revised Timeline Summary

### Completed Phases (Weeks 1-16)

- ✅ **Phase 0**: Map Content Plan (Weeks 1-3)
- ✅ **Phase 0.5**: UI Framework Validation (Week 4)
- ✅ **Phase 1-2**: Data-Driven Class/Race System (Weeks 5-6) - _Status unclear_
- ✅ **Phase 3**: SDK Foundation Module (Week 7)
- ✅ **Phase 8**: Documentation and Examples (Weeks 8-9)
- ✅ **Phase 9**: Integration and Polish (CLI tools) (Week 10)
- ✅ **Campaign Builder Phase 0-2**: Foundation UI (Weeks 11-14)

### Remaining Phases (Estimated)

- 🔲 **Phase 10** (CB Phase 3): Data Editors - Items, Spells, Monsters (Weeks 17-19)
- 🔲 **Phase 11** (CB Phase 4): Map Editor Integration (Weeks 20-21)
- 🔲 **Phase 12** (CB Phase 5): Quest & Dialogue Tools (Weeks 22-24)
- 🔲 **Phase 13** (CB Phase 6): Distribution Tools (Weeks 25-26)
- 🔲 **Phase 14**: Game Engine Campaign Integration (Weeks 27-28) - **CRITICAL**
- 🔲 **Phase 15** (CB Phase 7): Polish & Advanced Features (Weeks 29-31)

**Total Estimated Time:** 31 weeks (~8 months)
**Time Completed:** 16 weeks (~4 months)
**Time Remaining:** 15 weeks (~4 months)

**Critical Path:**

1. Phase 10 (Data Editors) - Foundation for content creation
2. Phase 14 (Game Integration) - Makes campaigns playable
3. Phase 12 (Quest/Dialogue) - Completes content creation loop
4. Phase 13 (Distribution) - Enables sharing
5. Phase 11 (Map Integration) - Can work in parallel with Phase 12
6. Phase 15 (Polish) - Enhances UX but not blocking

---

## Alignment Notes

This updated plan reconciles two documents:

- **`sdk_implementation_plan.md`** (this file) - Originally CLI-focused with phases 0-9
- **`sdk_and_campaign_architecture.md`** - GUI-focused Campaign Builder with phases 0-6

**Key Changes:**

- Added Phases 10-15 to cover Campaign Builder GUI (previously missing)
- Added Phase 14 for Game Engine Integration (CRITICAL gap)
- Clarified Phase 9 as CLI tools infrastructure (complete)
- Confirmed egui framework decision (Phase 0.5)
- Documented Campaign Builder Phase 2 completion status
- Identified placeholder status of data editors (Phase 10 needed)

**Two-Track Development:**

1. **CLI Tools Track** (Phases 1-9): Backend SDK, CLI editors, documentation - Mostly complete
2. **GUI Tools Track** (Phases 10-15): Campaign Builder with visual editors - Partially complete

**Current Status:** CLI backend is solid, GUI frontend needs Phases 10-15 for full modding SDK vision.

---

## Conclusion

This SDK plan transforms Antares from an MM1 clone into a general-purpose RPG engine with a complete modding SDK. The Map Content Implementation Plan served as the cornerstone, with SDK features building on top of proven patterns.

**Current State:** Backend SDK infrastructure complete, Campaign Builder has foundation UI, but data editors are placeholders and game cannot load campaigns yet.

**Next Steps:**

1. **Phase 10**: Implement data editors (items, spells, monsters) in Campaign Builder GUI
2. **Phase 14**: Integrate campaign loading into game engine (CRITICAL - enables gameplay)
3. **Phase 12**: Add quest and dialogue tools to Campaign Builder
4. **Phase 11**: Integrate map editor into Campaign Builder GUI
5. **Phase 13**: Add distribution and testing tools
6. **Phase 15**: Polish and advanced features (undo/redo, templates, node-graph visualizer)

**Key Takeaway:** SDK backend is complete and solid. Focus now shifts to completing Campaign Builder GUI (Phases 10-13, 15) and integrating campaigns into game engine (Phase 14) to achieve the full modding SDK vision documented in `sdk_and_campaign_architecture.md`.
