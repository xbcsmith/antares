# Antares SDK API Reference

**Version**: 0.1.0
**Target Audience**: Campaign creators, modders, tool developers

This document provides a complete technical reference for the Antares SDK modules, types, and functions.

---

## Table of Contents

1. [Overview](#overview)
2. [Module Structure](#module-structure)
3. [Core Modules](#core-modules)
   - [database](#database-module)
   - [validation](#validation-module)
   - [serialization](#serialization-module)
   - [templates](#templates-module)
   - [campaign_loader](#campaign_loader-module)
   - [campaign_packager](#campaign_packager-module)
4. [Editor Modules](#editor-modules)
   - [map_editor](#map_editor-module)
   - [quest_editor](#quest_editor-module)
   - [dialogue_editor](#dialogue_editor-module)
5. [Type Reference](#type-reference)
6. [Error Types](#error-types)
7. [Usage Patterns](#usage-patterns)

---

## Overview

The Antares SDK provides a unified content management system for creating, validating, and packaging game campaigns. It consists of:

- **Content Database**: Centralized access to all game content (classes, races, items, monsters, spells, maps)
- **Validation System**: Cross-reference checking and balance validation
- **Serialization Utilities**: RON format helpers and data merging
- **Content Templates**: Pre-configured templates for quick content creation
- **Campaign Management**: Loading, validation, and packaging tools
- **Editor Utilities**: Helper functions for building content editing tools

---

## Module Structure

```
antares::sdk
├── database          # Unified content database
├── validation        # Cross-reference and balance validation
├── serialization     # RON format utilities
├── templates         # Content templates
├── campaign_loader   # Campaign loading and validation
├── campaign_packager # Campaign packaging and distribution
├── map_editor        # Map editing helpers
├── quest_editor      # Quest validation helpers
└── dialogue_editor   # Dialogue validation helpers
```

---

## Core Modules

### database Module

**Purpose**: Provides unified access to all game content types.

#### `ContentDatabase`

Central database for all campaign content.

**Fields**:
- `classes: HashMap<ClassId, ClassDefinition>` - All character classes
- `races: HashMap<RaceId, RaceDefinition>` - All playable races
- `items: HashMap<ItemId, Item>` - All items
- `monsters: HashMap<MonsterId, Monster>` - All monsters
- `spells: HashMap<SpellId, Spell>` - All spells
- `maps: HashMap<MapId, Map>` - All maps

**Methods**:

```rust
pub fn new() -> Self
```
Creates an empty content database.

```rust
pub fn load_campaign(path: &str) -> Result<Self, DatabaseError>
```
Loads all content from a campaign directory structure:
- `path/data/classes.ron`
- `path/data/races.ron`
- `path/data/items.ron`
- `path/data/monsters.ron`
- `path/data/spells.ron`
- `path/data/maps/*.ron`

**Returns**: `Ok(ContentDatabase)` if all files load successfully.
**Errors**: `DatabaseError` if files are missing, invalid, or have parse errors.

```rust
pub fn load_core() -> Result<Self, DatabaseError>
```
Loads core game content from `data/` directory.

```rust
pub fn validate(&self) -> Result<(), Vec<ValidationError>>
```
Performs basic structural validation (duplicate IDs, required fields).

```rust
pub fn stats(&self) -> ContentStats
```
Returns summary statistics about loaded content.

**Example**:

```rust
use antares::sdk::database::ContentDatabase;

let db = ContentDatabase::load_campaign("campaigns/my_campaign")?;
let stats = db.stats();

println!("Loaded {} classes, {} items, {} maps",
    stats.class_count, stats.item_count, stats.map_count);
```

#### `ContentStats`

Summary statistics for content database.

**Fields**:
- `class_count: usize`
- `race_count: usize`
- `item_count: usize`
- `monster_count: usize`
- `spell_count: usize`
- `map_count: usize`

---

### validation Module

**Purpose**: Cross-reference validation and balance checking.

#### `Validator`

Validates content references and game balance.

**Constructor**:

```rust
pub fn new(db: &ContentDatabase) -> Self
```
Creates a validator for the given content database.

**Methods**:

```rust
pub fn validate_all(&self) -> Result<Vec<ValidationError>, String>
```
Performs comprehensive validation:
- Cross-reference checking (all IDs exist)
- Map connectivity verification
- Duplicate ID detection
- Balance warnings (optional)

**Returns**: Vector of validation errors (empty if valid).

```rust
fn validate_references(&self) -> Vec<ValidationError>
```
Internal: Validates all ID references exist.

```rust
fn validate_connectivity(&self) -> Vec<ValidationError>
```
Internal: Ensures maps are reachable from starting map.

```rust
fn check_balance(&self) -> Vec<ValidationError>
```
Internal: Checks for balance issues (optional warnings).

**Example**:

```rust
use antares::sdk::{ContentDatabase, Validator};

let db = ContentDatabase::load_campaign("campaigns/my_campaign")?;
let validator = Validator::new(&db);
let errors = validator.validate_all()?;

if errors.is_empty() {
    println!("Campaign is valid!");
} else {
    for error in &errors {
        eprintln!("Error: {}", error);
    }
}
```

#### `ValidationError`

Enumeration of all validation error types.

**Variants**:

```rust
MissingClass { context: String, class_id: ClassId }
```
Referenced class ID does not exist.

```rust
MissingRace { context: String, race_id: RaceId }
```
Referenced race ID does not exist.

```rust
MissingItem { context: String, item_id: ItemId }
```
Referenced item ID does not exist.

```rust
MissingMonster { map: String, monster_id: MonsterId }
```
Monster referenced in map does not exist.

```rust
MissingSpell { context: String, spell_id: SpellId }
```
Spell ID referenced does not exist.

```rust
DisconnectedMap { map_id: MapId }
```
Map is not reachable from starting map.

```rust
DuplicateId { entity_type: String, id: u32 }
```
Multiple entities share the same ID.

```rust
BalanceWarning { severity: Severity, message: String }
```
Optional balance warning (informational).

#### `Severity`

Warning severity level.

**Variants**:
- `Info` - Informational message
- `Warning` - Potential issue
- `Error` - Critical problem

---

### serialization Module

**Purpose**: RON format utilities and data manipulation.

#### Functions

```rust
pub fn format_ron<T: Serialize>(value: &T) -> Result<String, SerializationError>
```
Formats a value as pretty-printed RON with proper indentation.

**Parameters**:
- `value` - Any serializable type

**Returns**: Formatted RON string.

**Example**:

```rust
use antares::sdk::serialization::format_ron;
use antares::domain::items::Item;

let item = Item::new_weapon("Longsword", /* ... */);
let ron_string = format_ron(&item)?;
println!("{}", ron_string);
```

```rust
pub fn validate_ron_syntax(ron_string: &str) -> Result<(), SerializationError>
```
Checks if a RON string is syntactically valid.

**Parameters**:
- `ron_string` - RON text to validate

**Returns**: `Ok(())` if valid, `Err` with parse error details.

```rust
pub fn merge_ron_data<T>(base: &T, overlay: &T) -> Result<T, SerializationError>
where T: Serialize + DeserializeOwned
```
Merges two RON-serializable structures (overlay takes precedence).

**Use Case**: Campaign mods that override core content.

---

### templates Module

**Purpose**: Pre-configured content templates for quick creation.

#### Functions

```rust
pub fn basic_weapon(name: &str, damage: (u8, u8)) -> Item
```
Creates a basic weapon template.

**Parameters**:
- `name` - Weapon name
- `damage` - Damage dice (count, sides), e.g., `(1, 8)` = 1d8

**Returns**: `Item` with weapon type and basic properties.

**Example**:

```rust
use antares::sdk::templates::basic_weapon;

let dagger = basic_weapon("Dagger", (1, 4));
let longsword = basic_weapon("Longsword", (1, 8));
```

```rust
pub fn basic_armor(name: &str, ac_bonus: i8) -> Item
```
Creates a basic armor template.

**Parameters**:
- `name` - Armor name
- `ac_bonus` - Armor class bonus

**Returns**: `Item` with armor type.

```rust
pub fn town_map(name: &str, width: u16, height: u16) -> Map
```
Creates a town map template with standard features:
- Outdoor environment
- Floor tiles by default
- Daylight lighting

**Parameters**:
- `name` - Map name
- `width`, `height` - Map dimensions

**Returns**: `Map` with town defaults.

```rust
pub fn dungeon_map(name: &str, width: u16, height: u16) -> Map
```
Creates a dungeon map template with standard features:
- Indoor environment
- Wall tiles by default
- Dark (requires light)

**Parameters**:
- `name` - Map name
- `width`, `height` - Map dimensions

**Returns**: `Map` with dungeon defaults.

---

### campaign_loader Module

**Purpose**: Campaign loading and validation reporting.

#### `CampaignLoader`

Loads and validates complete campaigns.

**Methods**:

```rust
pub fn new() -> Self
```
Creates a new campaign loader.

```rust
pub fn load(&self, path: &str) -> Result<Campaign, CampaignError>
```
Loads a campaign from a directory.

**Parameters**:
- `path` - Campaign root directory

**Expected Structure**:
```
campaigns/my_campaign/
├── campaign.ron         # Campaign metadata
├── data/
│   ├── classes.ron
│   ├── races.ron
│   ├── items.ron
│   ├── monsters.ron
│   ├── spells.ron
│   └── maps/
│       ├── town.ron
│       └── dungeon.ron
└── README.md (optional)
```

**Returns**: `Ok(Campaign)` with all content loaded.
**Errors**: `CampaignError` with details of missing/invalid files.

```rust
pub fn validate(&self, campaign: &Campaign) -> ValidationReport
```
Validates a loaded campaign and generates a report.

**Returns**: `ValidationReport` with errors, warnings, and statistics.

#### `Campaign`

Represents a complete campaign with all content.

**Fields**:
- `info: CampaignInfo` - Campaign metadata
- `content: ContentDatabase` - All campaign content

#### `CampaignInfo`

Campaign metadata from `campaign.ron`.

**Fields**:
- `id: String` - Unique campaign identifier
- `name: String` - Display name
- `version: String` - Campaign version (semver)
- `author: String` - Creator name
- `description: String` - Campaign description
- `starting_map: MapId` - Initial map
- `min_engine_version: String` - Minimum Antares version required

#### `ValidationReport`

Campaign validation report.

**Fields**:
- `errors: Vec<ValidationError>` - Critical errors
- `warnings: Vec<ValidationError>` - Non-critical warnings
- `stats: ContentStats` - Content statistics
- `is_valid: bool` - True if no critical errors

**Methods**:

```rust
pub fn print_summary(&self)
```
Prints a formatted summary to stdout.

---

### campaign_packager Module

**Purpose**: Campaign packaging and distribution.

#### `CampaignPackager`

Packages campaigns for distribution.

**Methods**:

```rust
pub fn new() -> Self
```
Creates a new campaign packager.

```rust
pub fn package(&self, campaign_path: &str, output_path: &str)
    -> Result<PackageManifest, PackageError>
```
Packages a campaign into a distributable archive.

**Parameters**:
- `campaign_path` - Source campaign directory
- `output_path` - Output `.tar.gz` or `.zip` file

**Returns**: `PackageManifest` with package metadata.

**Process**:
1. Validates campaign
2. Creates manifest
3. Archives all content files
4. Generates checksums

```rust
pub fn unpack(&self, package_path: &str, destination: &str)
    -> Result<(), PackageError>
```
Unpacks a campaign package.

**Parameters**:
- `package_path` - Source `.tar.gz` or `.zip` file
- `destination` - Destination directory

#### `PackageManifest`

Package metadata.

**Fields**:
- `campaign_info: CampaignInfo` - Campaign details
- `packaged_at: String` - ISO 8601 timestamp
- `files: Vec<String>` - List of included files
- `checksums: HashMap<String, String>` - SHA256 checksums

---

## Editor Modules

### map_editor Module

**Purpose**: Helper functions for map editing tools.

#### Functions

```rust
pub fn validate_map(map: &Map, db: &ContentDatabase) -> Vec<ValidationError>
```
Validates a map against the content database.

**Checks**:
- Tile positions within bounds
- Monster IDs exist
- Item IDs exist
- Spell IDs exist
- Exit targets exist

```rust
pub fn suggest_item_ids(db: &ContentDatabase, query: &str) -> Vec<(ItemId, String)>
```
Returns item suggestions matching the query string.

**Parameters**:
- `db` - Content database
- `query` - Search string (case-insensitive substring match)

**Returns**: Vector of `(ItemId, name)` tuples.

```rust
pub fn suggest_monster_ids(db: &ContentDatabase, query: &str) -> Vec<(MonsterId, String)>
```
Returns monster suggestions matching the query string.

```rust
pub fn suggest_spell_ids(db: &ContentDatabase, query: &str) -> Vec<(SpellId, String)>
```
Returns spell suggestions matching the query string.

```rust
pub fn suggest_map_ids(db: &ContentDatabase, query: &str) -> Vec<(MapId, String)>
```
Returns map suggestions matching the query string.

```rust
pub fn browse_items(db: &ContentDatabase) -> Vec<(ItemId, String, String)>
```
Returns all items with ID, name, and type.

```rust
pub fn browse_monsters(db: &ContentDatabase) -> Vec<(MonsterId, String, u8)>
```
Returns all monsters with ID, name, and level.

```rust
pub fn browse_spells(db: &ContentDatabase) -> Vec<(SpellId, String, u8)>
```
Returns all spells with ID, name, and level.

```rust
pub fn browse_maps(db: &ContentDatabase) -> Vec<(MapId, String, String)>
```
Returns all maps with ID, name, and environment type.

```rust
pub fn is_valid_item_id(db: &ContentDatabase, id: ItemId) -> bool
```
Checks if an item ID exists.

```rust
pub fn is_valid_monster_id(db: &ContentDatabase, id: MonsterId) -> bool
```
Checks if a monster ID exists.

```rust
pub fn is_valid_spell_id(db: &ContentDatabase, id: SpellId) -> bool
```
Checks if a spell ID exists.

```rust
pub fn is_valid_map_id(db: &ContentDatabase, id: MapId) -> bool
```
Checks if a map ID exists.

---

### quest_editor Module

**Purpose**: Quest validation helpers.

#### Functions

```rust
pub fn validate_quest(quest: &Quest, db: &ContentDatabase)
    -> Result<(), Vec<QuestValidationError>>
```
Validates a quest definition.

**Checks**:
- Objective item/monster IDs exist
- Reward item/spell IDs exist
- Completion event IDs exist

```rust
pub fn get_quest_dependencies(quest: &Quest) -> QuestDependencies
```
Extracts all content IDs referenced by a quest.

```rust
pub fn generate_quest_summary(quest: &Quest) -> String
```
Generates a human-readable quest summary.

---

### dialogue_editor Module

**Purpose**: Dialogue validation helpers.

#### Functions

```rust
pub fn validate_dialogue(dialogue: &Dialogue, db: &ContentDatabase)
    -> Result<(), Vec<DialogueValidationError>>
```
Validates a dialogue tree.

**Checks**:
- Response branch IDs exist
- Item/quest references exist
- No circular loops (infinite dialogue)

```rust
pub fn analyze_dialogue(dialogue: &Dialogue) -> DialogueStats
```
Analyzes dialogue structure.

**Returns**: `DialogueStats` with node count, branch count, depth.

```rust
pub fn generate_dialogue_summary(dialogue: &Dialogue) -> String
```
Generates a text summary of dialogue flow.

---

## Type Reference

### Domain Type Aliases

All SDK functions use these type aliases from `antares::domain`:

```rust
pub type ClassId = u32;
pub type RaceId = u32;
pub type ItemId = u32;
pub type MonsterId = u32;
pub type SpellId = u32;
pub type MapId = u32;
pub type EventId = u32;
pub type CharacterId = u32;
pub type TownId = u32;
```

**Important**: Always use these aliases instead of raw `u32`.

### Core Domain Types

The SDK works with these domain types from `antares::domain`:

- **Character Types**: `Character`, `Class`, `Race`, `Attributes`
- **Item Types**: `Item`, `ItemType`, `WeaponData`, `ArmorData`, `Disablement`, `Bonus`
- **Monster Types**: `Monster`, `MonsterDefinition`
- **Spell Types**: `Spell`, `SpellEffect`, `SpellTarget`
- **Map Types**: `Map`, `MapMetadata`, `Tile`, `TileType`, `Event`, `Exit`, `NPC`

See `docs/reference/architecture.md` Section 4 for complete type definitions.

---

## Error Types

### `DatabaseError`

Errors from content database operations.

**Variants**:
- `FileNotFound(String)` - Required file missing
- `ParseError(String)` - RON parse error
- `IoError(std::io::Error)` - File I/O error

### `SerializationError`

Errors from RON serialization operations.

**Variants**:
- `ParseError(String)` - Invalid RON syntax
- `SerializationError(String)` - Serialization failed
- `MergeError(String)` - Data merge failed

### `CampaignError`

Errors from campaign loading.

**Variants**:
- `InvalidStructure(String)` - Missing required directories/files
- `ConfigError(String)` - Invalid `campaign.ron`
- `ContentError(DatabaseError)` - Content loading failed
- `ValidationError(Vec<ValidationError>)` - Validation failed

### `PackageError`

Errors from campaign packaging.

**Variants**:
- `ValidationFailed(Vec<ValidationError>)` - Campaign invalid
- `IoError(std::io::Error)` - File I/O error
- `CompressionError(String)` - Archive creation failed

---

## Usage Patterns

### Pattern 1: Validate a Campaign

```rust
use antares::sdk::{CampaignLoader, Validator};

fn validate_campaign(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Load campaign
    let loader = CampaignLoader::new();
    let campaign = loader.load(path)?;

    // Validate
    let validator = Validator::new(&campaign.content);
    let errors = validator.validate_all()?;

    // Report results
    if errors.is_empty() {
        println!("✓ Campaign is valid!");
    } else {
        eprintln!("✗ Found {} errors:", errors.len());
        for error in &errors {
            eprintln!("  - {}", error);
        }
    }

    Ok(())
}
```

### Pattern 2: Create Content with Templates

```rust
use antares::sdk::templates::{basic_weapon, basic_armor};
use antares::sdk::serialization::format_ron;

fn create_starter_gear() -> Result<(), Box<dyn std::error::Error>> {
    let mut items = vec![
        basic_weapon("Rusty Dagger", (1, 4)),
        basic_armor("Leather Armor", 2),
    ];

    // Customize
    items[0].value = 5;
    items[1].value = 15;

    // Save to RON
    let ron = format_ron(&items)?;
    std::fs::write("data/starter_items.ron", ron)?;

    Ok(())
}
```

### Pattern 3: Package a Campaign

```rust
use antares::sdk::CampaignPackager;

fn package_campaign(campaign_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let packager = CampaignPackager::new();

    // Package campaign
    let manifest = packager.package(
        campaign_path,
        "releases/my_campaign_v1.0.tar.gz"
    )?;

    println!("Packaged: {}", manifest.campaign_info.name);
    println!("Version: {}", manifest.campaign_info.version);
    println!("Files: {}", manifest.files.len());

    Ok(())
}
```

### Pattern 4: ID Suggestion for Editor UI

```rust
use antares::sdk::{ContentDatabase, map_editor};

fn suggest_items_for_loot(query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let db = ContentDatabase::load_core()?;

    // Get suggestions
    let suggestions = map_editor::suggest_item_ids(&db, query);

    // Display to user
    println!("Items matching '{}':", query);
    for (id, name) in suggestions {
        println!("  [{}] {}", id, name);
    }

    Ok(())
}
```

### Pattern 5: Cross-Reference Validation

```rust
use antares::sdk::{ContentDatabase, Validator, ValidationError};

fn check_missing_items(campaign_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let db = ContentDatabase::load_campaign(campaign_path)?;
    let validator = Validator::new(&db);
    let errors = validator.validate_all()?;

    // Filter for missing item errors
    let missing_items: Vec<_> = errors.iter()
        .filter(|e| matches!(e, ValidationError::MissingItem { .. }))
        .collect();

    if !missing_items.is_empty() {
        eprintln!("Missing items referenced:");
        for error in missing_items {
            eprintln!("  {}", error);
        }
    }

    Ok(())
}
```

---

## Best Practices

### 1. Always Validate Before Packaging

```rust
// ✓ GOOD
let loader = CampaignLoader::new();
let campaign = loader.load("campaigns/my_campaign")?;
let report = loader.validate(&campaign);

if report.is_valid {
    let packager = CampaignPackager::new();
    packager.package("campaigns/my_campaign", "output.tar.gz")?;
}

// ✗ BAD - Skip validation
let packager = CampaignPackager::new();
packager.package("campaigns/my_campaign", "output.tar.gz")?; // May fail later
```

### 2. Use Type Aliases

```rust
// ✓ GOOD
fn get_item(db: &ContentDatabase, id: ItemId) -> Option<&Item> {
    db.items.get(&id)
}

// ✗ BAD
fn get_item(db: &ContentDatabase, id: u32) -> Option<&Item> {
    db.items.get(&id)
}
```

### 3. Handle Errors Gracefully

```rust
// ✓ GOOD
match ContentDatabase::load_campaign(path) {
    Ok(db) => println!("Loaded {} items", db.items.len()),
    Err(DatabaseError::FileNotFound(file)) => {
        eprintln!("Missing required file: {}", file);
    },
    Err(e) => eprintln!("Load error: {}", e),
}

// ✗ BAD
let db = ContentDatabase::load_campaign(path).unwrap();
```

### 4. Use Validator for Cross-References

```rust
// ✓ GOOD - Validates item exists
let validator = Validator::new(&db);
validator.validate_all()?;

// ✗ BAD - Direct access without validation
let item = db.items.get(&item_id).unwrap(); // May panic
```

---

## See Also

- **Architecture**: `docs/reference/architecture.md` - Core data structures
- **Tutorial**: `docs/tutorials/creating_campaigns.md` - Step-by-step campaign creation
- **How-To**: `docs/how-to/creating_and_validating_campaigns.md` - Practical task guides
- **Modding**: `docs/explanation/modding_guide.md` - Conceptual modding guide

---

**Last Updated**: 2024
**SDK Version**: 0.1.0
