# Campaign Data Structures

This document provides a comprehensive reference for the data structures used in Antares campaigns. It is intended to serve as a specification for creating external tools (e.g., Python scripts) to manipulate campaign data, as well as a reference for content creators.

## 1. Data Types & Serialization

Antares uses [RON (Rusty Object Notation)](https://github.com/ron-rs/ron) for data serialization. RON is similar to JSON but supports Rust specifics like structs, enums, and tuples directly.

### Primitives

#### `AttributePair`
Used for primary statistics (Might, Intellect, etc.). Stores a base value (permanent) and a current value (temporary, for buffs/debuffs).

*   **Rust Definition**: `src/domain/character.rs`
*   **Structure**:
    ```rust
    struct AttributePair {
        base: u8,
        current: u8,
    }
    ```
*   **Serialization**:
    *   **Simple (Recommended)**: Just the number. Expands to `base` = `current`.
        ```ron
        might: 15
        ```
    *   **Full**: Explicit base and current.
        ```ron
        might: (base: 15, current: 18) // Started with 15, buffed to 18
        ```

#### `AttributePair16`
Same as `AttributePair` but uses `u16`. Primarily used for HP and SP.

*   **Rust Definition**: `src/domain/character.rs`
*   **Serialization**:
    *   **Simple**: `50`
    *   **Full**: `(base: 50, current: 40)`

#### `Option<T>`
*   **None**: `None`
*   **Some**: `Some(value)`

## 2. Core Structures

### CharacterDefinition
Defines a character template (pre-made character or NPC).

*   **Source**: `src/domain/character_definition.rs`
*   **File Location**: `campaigns/<campaign>/data/characters.ron` (typically)

#### Schema
```ron
(
    id: String,                 // Unique ID (e.g. "pregen_knight")
    name: String,               // Display name
    race_id: String,            // Reference to races.ron
    class_id: String,           // Reference to classes.ron
    sex: Sex,                   // Male, Female, Other
    alignment: Alignment,       // Good, Neutral, Evil
    
    // Stats
    base_stats: (
        might: AttributePair,
        intellect: AttributePair,
        personality: AttributePair,
        endurance: AttributePair,
        speed: AttributePair,
        accuracy: AttributePair,
        luck: AttributePair,
    ),

    // Optional overrides
    hp_override: Option<AttributePair16>, // Overrides calculated HP
    
    // Meta
    portrait_id: String,        // Filename stem (e.g. "knight_01")
    description: String,
    
    // Starting State
    starting_gold: u32,
    starting_gems: u32,
    starting_food: u8,          // Default: 10
    
    starting_items: [           // List of Item IDs (u8)
        1, 5, 10
    ],
    
    starting_equipment: (       // Equipped Item IDs (u8)
        weapon: Some(1),
        armor: Some(10),
        shield: None,
        helmet: None,
        boots: None,
        accessory1: None,
        accessory2: None,
    ),
    
    // Flags
    is_premade: bool,           // If true, appears in "New Game" selection
    starts_in_party: bool,      // If true, auto-added to party
)
```

#### Notable Fields
-   `base_stats`: Can use simple integer values for readability.
-   `hp_override`: Replaces the legacy `hp_base` and `hp_current` fields.
-   `portrait_id`: Should be the filename without extension (e.g., `assets/portraits/knight_01.png` -> `"knight_01"`).

---

### NpcDefinition
Defines reusable NPC data (identity, dialogue, quests).

*   **Source**: `src/domain/world/npc.rs`
*   **File Location**: `campaigns/<campaign>/data/npcs.ron`

#### Schema
```ron
(
    id: String,                 // Logical ID (e.g. "village_elder")
    name: String,               // Display Name
    description: String,
    portrait_id: String,        // Filename stem
    
    dialogue_id: Option<u16>,   // ID of default DialogueTree
    quest_ids: [u16],           // list of Quest IDs involved with
    
    faction: Option<String>,
    is_merchant: bool,
    is_innkeeper: bool,
)
```

---

### MapBlueprint
Defines the layout and content of a map level.

*   **Source**: `src/domain/world/blueprint.rs`
*   **File Location**: `campaigns/<campaign>/maps/*.ron`

#### Schema
```ron
(
    id: u16,
    name: String,
    description: String,
    width: u32,
    height: u32,
    environment: EnvironmentType, // Outdoor, Indoor, Dungeon, Cave
    
    // Terrain
    tiles: [
        (
            x: i32,
            y: i32,
            code: TileCode, // Floor, Wall, Door, Grass, etc.
        ),
        // ... more tiles
    ],
    
    // Triggered Events (Traps, Teleports, Scripts)
    events: [
        (
            position: (x: 5, y: 10),
            event_type: Teleport(
                map_id: 2,
                x: 1,
                y: 1
            ),
            // OR
            event_type: NpcDialogue("dialogue_id"),
        )
    ],
    
    // NPC Instances
    npc_placements: [
        (
            npc_id: String,     // Reference to NpcDefinition.id ("village_elder")
            position: (x: 10, y: 15),
            facing: Some(South), // North, South, East, West
            dialogue_override: None, // Optional DialogueID override
        )
    ],
    
    exits: [],
    starting_position: (x: 0, y: 0),
)
```

---

### DialogueTree & Quest
Referenced by numeric ID (u16).

*   **DialogueTree**: `src/domain/dialogue.rs`
*   **Quest**: `src/domain/quest.rs`

These are typically stored in `dialogues.ron` and `quests.ron`. Currently they use numeric IDs (`u16`) which must be unique within their respective files.

## 3. Validation Rules

1.  **Strict IDs**:
    *   `CharacterDefinition.id` must be unique across all loaded definitions.
    *   `NpcDefinition.id` must be unique.
    *   `MapBlueprint.npc_placements` must reference existing `NpcDefinition.id`s.

2.  **Stat Integrity**:
    *   `AttributePair.current` should conceptually not exceed `base` *on creation*, though the engine supports it (buffs).
    *   `hp_override.current` MUST NOT exceed `hp_override.base`. If it does, it will be clamped at runtime, but tools should warn.

3.  **Filenames**:
    *   `portrait_id` should typically be lowercase alphanumeric with underscores.

## 4. Verification (Python Example)

The following Python script demonstrates how one might parse and validate these files using `ronre` (or a similar regex-based approach, as true RON parsers for Python are rare/complex).

> **Note**: For robust production tools, consider binding to the Rust `ron` crate or using a strict parser. This example uses simple pattern matching for demonstration.

```python
import re

# Mock content of a characters.ron file
ron_content = """
[
    (
        id: "pregen_knight",
        name: "Sir Galahad",
        race_id: "human",
        class_id: "knight",
        sex: Male,
        alignment: Good,
        base_stats: (
            might: 16,
            intellect: 8,
            personality: 12,
            endurance: 14,
            speed: 10,
            accuracy: 14,
            luck: 10,
        ),
        hp_override: Some((base: 25, current: 25)),
        portrait_id: "knight_01",
        starting_gold: 150,
    )
]
"""

def validate_character_stats(ron_text):
    print("Validating Character Definitions...")
    
    # Regex to find hp_override patterns
    # Matches: hp_override: Some((base: 50, current: 60))
    hp_pattern = re.compile(r'hp_override:\s*Some\s*\(\s*\(\s*base:\s*(\d+),\s*current:\s*(\d+)\s*\)\s*\)')
    
    issues = []
    
    for match in hp_pattern.finditer(ron_text):
        base = int(match.group(1))
        current = int(match.group(2))
        
        if current > base:
            issues.append(f"HP Integrity Check Failed: current ({current}) > base ({base})")
        else:
            print(f"HP Check Passed: {current}/{base}")

    if not issues:
        print("All HP validations passed.")
    else:
        for issue in issues:
            print(f"ERROR: {issue}")

if __name__ == "__main__":
    validate_character_stats(ron_content)
```
