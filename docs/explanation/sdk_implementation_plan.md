# Antares SDK Implementation Plan

## Overview

This plan outlines the development of the Antares SDK - a comprehensive toolset for creating custom RPG campaigns. The SDK builds upon the Map Content Implementation Plan as its cornerstone, extending the engine from an MM1 clone into a general-purpose RPG engine with modding support.

**Key Principles:**
- Map Content Plan (Phases 1-3) executes first, unchanged
- SDK adds capability without refactoring existing code
- Data-driven design enables modding without recompilation
- All content uses RON format per architecture.md Section 7

## Current State Analysis

### Existing Infrastructure

**✅ Already Data-Driven:**
- `data/items.ron` - Item database with `ItemDatabase` loader
- `data/monsters.ron` - Monster definitions (planned)
- `data/spells.ron` - Spell definitions (planned)
- `data/maps/*.ron` - Map system (in progress via Map Content Plan)

**❌ Code-Driven (Needs Migration):**
- Character classes - hardcoded `enum Class` in `src/domain/character.rs`
- Character races - hardcoded `enum Race` in `src/domain/character.rs`
- Class-specific game logic - HP dice, spell access, item restrictions

**🔧 Existing Patterns to Leverage:**
- `ItemDatabase` - proven pattern for loading/querying RON data
- RON serialization infrastructure - serde + ron crate already integrated
- Type aliases - `ItemId`, `SpellId`, `MonsterId` provide abstraction layer

### Identified Issues

1. **Tight Coupling**: Class enum embedded in multiple systems (progression, magic, items)
2. **Hardcoded Logic**: HP gain dice, spell school access uses pattern matching
3. **Bitflag Restrictions**: `Disablement` system uses fixed class bits
4. **No Validation**: Tools lack cross-reference validation (e.g., map references non-existent monster)
5. **No SDK API**: Engine internals not exposed for tool development

## Implementation Phases

### Phase 0: Map Content Plan Completion (Prerequisite)

**Status:** Execute separately per `map_content_implementation_plan_v2.md`

**Deliverables from Map Plan:**
- Phase 1: Map documentation and validation utility
- Phase 2: Map Builder tool (CLI MVP)
- Phase 3: Starter content (town, dungeon, forest maps)

**Timeline:** 3 weeks (see map plan)

**Success Criteria:**
- Map Builder tool functional
- Starter maps created and tested
- Map RON format documented

**Note:** This phase must complete before SDK work begins. Map Builder becomes the SDK's flagship tool.

---

### Phase 1: Data-Driven Class System

**Goal:** Migrate Character classes from hardcoded enum to data-driven RON definitions

**Duration:** 5-7 days

#### 1.1 Class Definition Data Structure

**File:** `src/domain/character.rs` (modify existing)

**Changes:**
```rust
// NEW: Data structure for class definitions
pub struct ClassDefinition {
    pub id: String,                          // "knight", "sorcerer"
    pub name: String,                        // "Knight", "Sorcerer"
    pub hp_die: DiceRoll,                   // Hit dice (1d10, 1d4, etc.)
    pub spell_school: Option<SpellSchool>,  // Cleric, Sorcerer, or None
    pub is_pure_caster: bool,               // Full vs hybrid caster
    pub spell_stat: Option<SpellStat>,      // INT or PER for spell points
    pub disablement_bit: u8,                // Bitflag for item restrictions
    pub special_abilities: Vec<String>,     // "multiple_attacks", etc.
}

pub enum SpellStat {
    Intellect,
    Personality,
}

// CHANGE: Replace enum with ID-based reference
pub type ClassId = String;

// MODIFY: Character struct
pub struct Character {
    pub name: String,
    pub race: Race,
    pub class_id: ClassId,  // Changed from: pub class: Class,
    // ... rest unchanged
}
```

**Rationale:** Maintains existing `Character` struct layout, minimizes refactoring

#### 1.2 Class Database Implementation

**File:** `src/domain/character/class_database.rs` (create new)

**Pattern:** Mirror existing `ItemDatabase` implementation

```rust
pub struct ClassDatabase {
    classes: HashMap<ClassId, ClassDefinition>,
}

impl ClassDatabase {
    pub fn new() -> Self;
    pub fn load_from_file(path: &Path) -> Result<Self, ClassDatabaseError>;
    pub fn load_from_string(ron_data: &str) -> Result<Self, ClassDatabaseError>;
    pub fn get_class(&self, id: &ClassId) -> Option<&ClassDefinition>;
    pub fn all_classes(&self) -> Vec<&ClassDefinition>;
    pub fn validate(&self) -> Vec<ValidationError>;
}
```

**Dependencies:**
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
- `src/domain/progression.rs` - HP gain function
- `src/domain/magic/casting.rs` - Spell access checks
- `src/domain/items/types.rs` - Disablement validation

**Pattern for Refactoring:**
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
- `src/domain/character/class_database.rs` - Unit tests for database
- `src/domain/progression.rs` - Update existing tests to use ClassDatabase
- `src/domain/magic/casting.rs` - Update spell access tests

**Test Coverage:**
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
- Update `Character::new()` to accept `RaceId`
- Apply stat modifiers from `RaceDefinition` during creation
- Update tests to use race IDs instead of enum

#### 2.5 Testing Requirements

**Coverage:**
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
```rust
//! Antares SDK - Content creation and validation tools
//!
//! This module provides programmatic access to game systems for
//! content creation tools and campaign validation.

pub mod database;      // Unified content database
pub mod validation;    // Cross-reference validation
pub mod serialization; // RON helpers
pub mod templates;     // Common content patterns

pub use database::ContentDatabase;
pub use validation::{ValidationError, Validator};
```

#### 3.2 Unified Content Database

**File:** `src/sdk/database.rs` (create new)

```rust
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
```

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
- Every `ItemId` in maps exists in `items.ron`
- Every `MonsterId` in maps exists in `monsters.ron`
- Every `ClassId` referenced exists in `classes.ron`
- All map transitions point to existing maps
- No duplicate IDs within a content type

#### 3.4 RON Serialization Helpers

**File:** `src/sdk/serialization.rs` (create new)

```rust
/// Pretty-print RON with consistent formatting
pub fn format_ron<T: Serialize>(data: &T) -> Result<String, SerializationError>;

/// Validate RON syntax without full deserialization
pub fn validate_ron_syntax(ron_str: &str) -> Result<(), SerializationError>;

/// Load and merge multiple RON files (for mod composition)
pub fn merge_ron_data<T>(files: &[PathBuf]) -> Result<Vec<T>, SerializationError>
where
    T: DeserializeOwned;
```

#### 3.5 Content Templates

**File:** `src/sdk/templates.rs` (create new)

```rust
/// Common patterns for content creation tools
pub mod templates {
    /// Create a basic melee weapon
    pub fn basic_weapon(
        id: ItemId,
        name: &str,
        damage_dice: DiceRoll,
    ) -> Item;

    /// Create a simple armor piece
    pub fn basic_armor(
        id: ItemId,
        name: &str,
        ac_bonus: u8,
    ) -> Item;

    /// Create a town map template
    pub fn town_map(
        id: MapId,
        name: &str,
        width: u32,
        height: u32,
    ) -> Map;

    /// Create a dungeon map template
    pub fn dungeon_map(
        id: MapId,
        name: &str,
        width: u32,
        height: u32,
    ) -> Map;
}
```

#### 3.6 Testing Requirements

**Tests:**
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
```rust
// Add to MapBuilder struct
struct MapBuilder {
    map: Map,
    content_db: Option<ContentDatabase>,  // NEW: Optional SDK integration
}

impl MapBuilder {
    fn new() -> Self {
        // Try to load content database for validation
        let content_db = ContentDatabase::load_core().ok();
        Self { map: Map::default(), content_db }
    }

    // NEW: Enhanced validation command
    fn validate_advanced(&self) {
        if let Some(db) = &self.content_db {
            let errors = db.validate();
            if errors.is_empty() {
                println!("✅ Map validated successfully");
            } else {
                println!("❌ Validation errors:");
                for error in errors {
                    println!("  - {}", error);
                }
            }
        } else {
            println!("⚠️  SDK not available, using basic validation");
            self.validate_basic();
        }
    }

    // Existing basic validation still works
    fn validate_basic(&self) {
        // Original VALID_MONSTER_IDS check
    }
}
```

#### 4.2 Add Smart ID Suggestions

**Enhancement:** When user enters invalid ID, suggest valid alternatives

```rust
fn add_event(&mut self, x: u32, y: u32, event_type: &str) {
    // ... existing code ...

    // NEW: Suggest valid IDs if SDK available
    if let Some(db) = &self.content_db {
        if event_type == "monster" {
            if db.monsters.get_monster(monster_id).is_none() {
                println!("❌ Monster ID {} not found", monster_id);
                println!("💡 Available monsters:");
                for monster in db.monsters.all_monsters().iter().take(10) {
                    println!("   {} - {}", monster.id, monster.name);
                }
            }
        }
    }
}
```

#### 4.3 Add Interactive Content Browser

**New Command:** `list <content_type>`

```rust
fn process_command(&mut self, line: &str) {
    match parts[0] {
        // ... existing commands ...

        "list" => {
            if let Some(db) = &self.content_db {
                match parts.get(1) {
                    Some(&"monsters") => {
                        println!("Available Monsters:");
                        for m in db.monsters.all_monsters() {
                            println!("  {} - {} (HP: {})", m.id, m.name, m.hp);
                        }
                    }
                    Some(&"items") => {
                        println!("Available Items:");
                        for i in db.items.all_items() {
                            println!("  {} - {} ({}g)", i.id, i.name, i.base_cost);
                        }
                    }
                    _ => println!("Usage: list <monsters|items|classes|races>"),
                }
            } else {
                println!("SDK not loaded");
            }
        }

        _ => println!("Unknown command"),
    }
}
```

#### 4.4 Testing Requirements

**Tests:**
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

### Phase 5: Class/Race Editor Tool

**Goal:** Create interactive editor for classes and races

**Duration:** 3-4 days

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
- Set stat modifiers (+/- to attributes)
- Set resistance modifiers
- Configure special abilities
- Preview stat distribution at character creation

#### 5.3 Testing Requirements

**Manual Testing Checklist:**
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
   - All `ItemId` references exist in `items.ron`
   - All `MonsterId` references exist in `monsters.ron`
   - All `SpellId` references exist in `spells.ron`
   - All class restrictions reference valid classes

2. **Structural Validation:**
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
- Valid campaign (should pass)
- Campaign with missing monster ID (should fail)
- Campaign with disconnected map (should warn)
- Campaign with balance issues (should warn)

#### 6.4 Deliverables

- [ ] `tools/campaign-validator/` CLI tool
- [ ] Validates all reference integrity
- [ ] Checks structural issues
- [ ] Optional balance warnings
- [ ] Clear, actionable error messages
- [ ] Exit codes for CI/CD integration
- [ ] Documentation in `docs/how_to/validating_campaigns.md`

#### 6.5 Success Criteria

- ✅ Catches all reference errors in test campaigns
- ✅ Provides helpful suggestions for fixes
- ✅ Can be used in automated workflows
- ✅ Performance acceptable for large campaigns (100+ maps)
- ✅ Documentation includes examples of common errors

---

### Phase 7: Item Editor Tool

**Goal:** Visual/interactive editor for item database

**Duration:** 3-4 days

#### 7.1 Item Editor Implementation

**File:** `tools/item-editor/main.rs` (create new)

**Features:**
- Browse existing items by category
- Create new items with guided prompts
- Edit item properties
- Set class/race restrictions via checkboxes
- Preview item stats and effects
- Validate item definitions

```rust
struct ItemEditor {
    items: ItemDatabase,
    classes: ClassDatabase,  // For validation
    current_item: Option<ItemId>,
}

impl ItemEditor {
    fn create_weapon(&mut self) {
        println!("\n=== Weapon Creator ===");

        let id = self.get_next_id();
        let name = self.get_input("Name: ");

        println!("Damage (examples: 1d6, 2d4, 1d8+2): ");
        let damage = self.parse_dice();

        println!("To-Hit/Damage Bonus (0 for none): ");
        let bonus = self.get_i8();

        println!("Hands Required (1 or 2): ");
        let hands = self.get_u8();

        println!("Base Cost (gold): ");
        let cost = self.get_u32();

        println!("\nClass Restrictions:");
        let disablement = self.select_classes();

        // ... create Item struct ...

        self.items.add_item(item)?;
        println!("✅ Weapon created: {}", name);
    }

    fn select_classes(&self) -> Disablement {
        println!("Select classes that CAN use this item:");
        let mut bits = 0u8;

        for class in self.classes.all_classes() {
            let answer = self.get_input(&format!("  {} [y/N]: ", class.name));
            if answer.to_lowercase() == "y" {
                bits |= class.disablement_bit;
            }
        }

        Disablement(bits)
    }
}
```

#### 7.2 Testing Requirements

**Manual Testing:**
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
- Overview of SDK architecture
- `ContentDatabase` API reference
- `Validator` API reference
- Serialization helpers
- Template functions
- Code examples for each module

#### 8.2 Campaign Creation Guide

**File:** `docs/tutorials/campaign_creation_guide.md` (create new)

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
- `docs/how_to/using_map_builder.md` (update existing)
- `docs/how_to/using_class_editor.md`
- `docs/how_to/using_race_editor.md`
- `docs/how_to/using_item_editor.md`
- `docs/how_to/validating_campaigns.md`

**Format:** Follow Diataxis framework (task-oriented guides)

#### 8.4 Example Campaign

**Directory:** `campaigns/example_tutorial/` (create new)

**Content:** Minimal but complete campaign demonstrating all features
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
- Introduction to Antares modding
- SDK overview
- Campaign structure
- Creating custom content
- Testing your mod
- Sharing with community
- Best practices

#### 8.6 Deliverables

- [ ] `docs/reference/sdk_api.md` - Complete API docs
- [ ] `docs/tutorials/campaign_creation_guide.md` - Step-by-step guide
- [ ] `docs/tutorials/modding_guide.md` - Modder introduction
- [ ] Individual tool guides in `docs/how_to/`
- [ ] `campaigns/example_tutorial/` - Working example
- [ ] README.md updated with SDK information
- [ ] Architecture.md updated with SDK additions

#### 8.7 Success Criteria

- ✅ External user can create campaign following documentation
- ✅ All SDK functions documented with examples
- ✅ Example campaign loads and plays correctly
- ✅ Documentation passes markdownlint
- ✅ No broken links in documentation

---

### Phase 9: Integration and Polish

**Goal:** Final integration, bug fixes, and user experience improvements

**Duration:** 3-4 days

#### 9.1 Cross-Tool Integration

**Enhancements:**
- Map Builder suggests items from Item Editor database
- Class Editor validates against Race Editor data
- All tools use consistent UI/UX patterns
- Shared configuration file for tool preferences

#### 9.2 Error Message Improvements

**Review and enhance:**
- All error messages provide actionable guidance
- Validation errors suggest fixes
- Tool help text is clear and complete
- Examples included in error messages where helpful

#### 9.3 Performance Optimization

**Profile and optimize:**
- Campaign loading times
- Validation performance on large campaigns
- RON serialization/deserialization
- Map Builder responsiveness

#### 9.4 Quality Assurance

**Final checks:**
- All cargo commands pass (test, clippy, fmt)
- Documentation complete and accurate
- Example campaign fully functional
- Tools work on all supported platforms
- No breaking changes to core engine

#### 9.5 Deliverables

- [ ] All tools integrated and cross-functional
- [ ] Error messages reviewed and improved
- [ ] Performance acceptable for large campaigns
- [ ] All quality checks pass
- [ ] Release notes prepared
- [ ] Migration guide from pre-SDK version

#### 9.6 Success Criteria

- ✅ Complete campaign creation workflow functional
- ✅ External tester can create campaign using only docs
- ✅ All tools pass quality gates
- ✅ Performance metrics acceptable
- ✅ No known critical bugs

---

## Architecture Compliance

### Data Structure Integrity

**Preserved from architecture.md:**
- Type aliases: `ClassId`, `RaceId`, `ItemId`, etc.
- RON format for all data files (Section 7.1-7.2)
- Module structure respects domain/application layers
- No circular dependencies introduced

**New Additions (Compatible):**
- `src/sdk/` module for tool support (application layer)
- `data/classes.ron` and `data/races.ron` (data layer)
- `tools/` directory for SDK binaries (external to engine)

### Backward Compatibility

**Breaking Changes:**
- Character struct uses `ClassId` instead of `Class` enum
- Function signatures add `&ClassDatabase` parameters

**Migration Strategy:**
- Provide conversion functions: `Class::from_str()` → `ClassId`
- Document migration in `docs/explanation/sdk_migration.md`
- Support both old and new APIs for 1-2 versions (if needed)

## Testing Strategy

### Unit Tests

**Coverage Requirements:**
- All SDK modules: >80% coverage
- Database loaders: 100% coverage
- Validation logic: >90% coverage

**Test Data:**
- Minimal valid campaign (pass validation)
- Invalid campaigns with each error type
- Edge cases (empty campaign, huge campaign)

### Integration Tests

**Test Scenarios:**
- Load complete MM1-style campaign
- Create campaign using SDK tools
- Validate campaign with known errors
- Modify campaign and re-validate

### Manual Testing

**User Acceptance:**
- Non-programmer creates simple campaign
- Experienced modder creates complex campaign
- All tools usable without source code access

## Success Metrics

### Quantitative

- [ ] All quality checks pass (cargo test/clippy/fmt)
- [ ] >80% code coverage for SDK modules
- [ ] Campaign load time <2 seconds for 100 maps
- [ ] Validation completes <5 seconds for 100 maps

### Qualitative

- [ ] External tester successfully creates campaign
- [ ] Documentation rated "clear and complete"
- [ ] Tools rated "easy to use"
- [ ] No critical bugs in SDK or tools

## Timeline Summary

| Phase | Duration | Cumulative |
|-------|----------|------------|
| Phase 0: Map Content Plan (prerequisite) | 3 weeks | 3 weeks |
| Phase 1: Data-Driven Classes | 5-7 days | ~4 weeks |
| Phase 2: Data-Driven Races | 3-4 days | ~5 weeks |
| Phase 3: SDK Foundation | 4-5 days | ~6 weeks |
| Phase 4: Enhanced Map Builder | 2-3 days | ~6.5 weeks |
| Phase 5: Class/Race Editor | 3-4 days | ~7 weeks |
| Phase 6: Campaign Validator | 2-3 days | ~7.5 weeks |
| Phase 7: Item Editor | 3-4 days | ~8 weeks |
| Phase 8: Documentation | 4-5 days | ~9 weeks |
| Phase 9: Integration & Polish | 3-4 days | ~10 weeks |
| **Total** | **~10 weeks** | **10 weeks** |

**Note:** Assumes part-time development (~20 hours/week). Full-time could reduce to 5-6 weeks.

## Risk Management

### Technical Risks

| Risk | Mitigation |
|------|------------|
| Performance of data-driven lookup | Profile early, optimize hot paths, consider caching |
| RON parsing errors confuse users | Enhance error messages, provide validation tools |
| Complex refactoring introduces bugs | Comprehensive tests before/after, incremental changes |

### Project Risks

| Risk | Mitigation |
|------|------------|
| Scope creep | Stick to phases, defer enhancements to post-SDK |
| Documentation lags implementation | Write docs as you build, not after |
| Tools hard to use | Early user testing, iterate on UX |

## Post-SDK Future Work

**After SDK completion, consider:**
- GUI tools (Tauri or egui-based editors)
- Scripting language for custom events (Lua/Rhai)
- Online mod repository and browser
- Visual map editor (grid-based UI)
- Advanced balance analyzer tools
- Automated content generators

## Conclusion

This SDK plan transforms Antares from an MM1 clone into a general-purpose RPG engine. The Map Content Implementation Plan serves as the cornerstone, with SDK features building on top of proven patterns. By maintaining backward compatibility and following the existing architecture, this plan minimizes refactoring while maximizing extensibility.

**Key Takeaway:** Execute Map Content Plan first, then add SDK incrementally. This validates the architecture with real content before investing in tooling.
