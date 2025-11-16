# Using SDK Map Editor Integration

This guide shows how to use the SDK map editor integration functions to enhance map editing tools with smart suggestions, content browsing, and validation.

## Quick Start

### Prerequisites

```rust
use antares::sdk::database::ContentDatabase;
use antares::sdk::map_editor::*;
use antares::domain::world::Map;
```

### Load Content Database

```rust
// Load campaign content
let db = ContentDatabase::load_campaign("campaigns/my_campaign")?;

// Or load core game content
let db = ContentDatabase::load_core("data")?;
```

---

## Content Browsing

### List All Available Content

```rust
// Browse all monsters
let monsters = browse_monsters(&db);
for (id, name) in monsters {
    println!("Monster [{}]: {}", id, name);
}

// Browse all items
let items = browse_items(&db);
for (id, name) in items {
    println!("Item [{}]: {}", id, name);
}

// Browse all spells
let spells = browse_spells(&db);
for (id, name) in spells {
    println!("Spell [{:#06x}]: {}", id, name);
}

// Browse all maps
let maps = browse_maps(&db);
for (id, width, height) in maps {
    println!("Map [{}]: {}x{}", id, width, height);
}
```

### Use Case: Populating a Content List

```rust
fn show_available_monsters(db: &ContentDatabase) {
    println!("╔═══ Available Monsters ═══╗");
    let monsters = browse_monsters(db);

    if monsters.is_empty() {
        println!("No monsters found in database");
        return;
    }

    for (id, name) in monsters.iter().take(20) {
        println!("  [{}] {}", id, name);
    }

    if monsters.len() > 20 {
        println!("  ... and {} more", monsters.len() - 20);
    }
}
```

---

## Smart ID Suggestions

### Autocomplete with Fuzzy Search

```rust
// Suggest monsters matching "gob"
let suggestions = suggest_monster_ids(&db, "gob");
// Returns: [(1, "Goblin"), (2, "Goblin Warrior"), ...]

// Suggest items matching "sword"
let suggestions = suggest_item_ids(&db, "sword");
// Returns: [(10, "Short Sword"), (11, "Long Sword"), ...]

// Suggest spells matching "fire"
let suggestions = suggest_spell_ids(&db, "fire");
// Returns: [(0x1000, "Fireball"), (0x1001, "Fire Shield"), ...]

// Suggest maps matching "dungeon"
let map_ids = suggest_map_ids(&db, "dungeon");
// Returns: [5, 6, 7]
```

### Use Case: Interactive Autocomplete

```rust
fn handle_monster_input(db: &ContentDatabase, user_input: &str) {
    let suggestions = suggest_monster_ids(db, user_input);

    if suggestions.is_empty() {
        println!("💡 No monsters found matching '{}'", user_input);
        println!("Type 'browse monsters' to see all available monsters");
        return;
    }

    println!("💡 Suggestions for '{}':", user_input);
    for (id, name) in suggestions {
        println!("  [{}] {}", id, name);
    }
}
```

### Use Case: Command-Line Parser

```rust
fn parse_encounter_command(db: &ContentDatabase, args: &[&str]) {
    // User typed: "add encounter gob"
    if args.len() < 2 {
        println!("Usage: add encounter <monster_name_or_id>");
        return;
    }

    let input = args[1];

    // Try to parse as ID first
    if let Ok(monster_id) = input.parse::<u32>() {
        if is_valid_monster_id(db, monster_id) {
            println!("Adding encounter with monster ID {}", monster_id);
            // Add encounter...
            return;
        }
    }

    // Otherwise, suggest matches
    let suggestions = suggest_monster_ids(db, input);

    if suggestions.len() == 1 {
        let (id, name) = &suggestions[0];
        println!("Adding encounter with {} [{}]", name, id);
        // Add encounter...
    } else if suggestions.is_empty() {
        println!("❌ No monsters found matching '{}'", input);
    } else {
        println!("Multiple matches found. Please be more specific:");
        for (id, name) in suggestions {
            println!("  [{}] {}", id, name);
        }
    }
}
```

---

## Validation

### Validate a Map

```rust
use antares::sdk::validation::Severity;

fn validate_and_report(db: &ContentDatabase, map: &Map) -> Result<bool, Box<dyn std::error::Error>> {
    let errors = validate_map(db, map)?;

    if errors.is_empty() {
        println!("✅ Map {} is valid!", map.id);
        return Ok(true);
    }

    // Count by severity
    let error_count = errors.iter().filter(|e| e.severity() == Severity::Error).count();
    let warning_count = errors.iter().filter(|e| e.severity() == Severity::Warning).count();
    let info_count = errors.iter().filter(|e| e.severity() == Severity::Info).count();

    println!("\n╔═══ Validation Results ═══╗");
    println!("Errors:   {}", error_count);
    println!("Warnings: {}", warning_count);
    println!("Info:     {}", info_count);
    println!();

    // Display all issues
    for error in &errors {
        let icon = match error.severity() {
            Severity::Error => "❌",
            Severity::Warning => "⚠️",
            Severity::Info => "ℹ️",
        };
        println!("{} {}", icon, error);
    }

    Ok(error_count == 0)
}
```

### Quick ID Validation

```rust
// Check if IDs exist before using them
fn add_encounter_safe(db: &ContentDatabase, map: &mut Map, monster_id: u32, x: i32, y: i32) {
    // Validate monster ID exists
    if !is_valid_monster_id(db, monster_id) {
        eprintln!("❌ Error: Monster ID {} not found in database", monster_id);
        eprintln!("   Use 'browse monsters' or 'suggest monster <name>' to find valid IDs");
        return;
    }

    // Add the encounter
    use antares::domain::world::MapEvent;
    use antares::domain::types::Position;

    let event = MapEvent::Encounter {
        monster_group: vec![monster_id as u8],
    };

    map.add_event(Position::new(x, y), event);
    println!("✅ Added encounter at ({}, {})", x, y);
}
```

### Real-Time Validation in Editor

```rust
fn on_map_modified(db: &ContentDatabase, map: &Map) {
    // Quick validation on each edit
    match validate_map(db, map) {
        Ok(errors) => {
            let error_count = errors.iter().filter(|e| e.severity() == Severity::Error).count();

            if error_count > 0 {
                println!("⚠️  Map has {} validation errors", error_count);
                // Show first error as hint
                if let Some(first_error) = errors.first() {
                    println!("   {}", first_error);
                }
            } else {
                println!("✅ Map is valid");
            }
        }
        Err(e) => {
            eprintln!("Validation error: {}", e);
        }
    }
}
```

---

## Integration Patterns

### Pattern 1: CLI Tool with Commands

```rust
fn process_command(db: &ContentDatabase, map: &mut Map, command: &str) {
    let parts: Vec<&str> = command.split_whitespace().collect();

    match parts[0] {
        "browse" => {
            if parts.len() < 2 {
                println!("Usage: browse <monsters|items|spells|maps>");
                return;
            }
            match parts[1] {
                "monsters" => {
                    for (id, name) in browse_monsters(db) {
                        println!("  [{}] {}", id, name);
                    }
                }
                "items" => {
                    for (id, name) in browse_items(db) {
                        println!("  [{}] {}", id, name);
                    }
                }
                _ => println!("Unknown category: {}", parts[1]),
            }
        }
        "suggest" => {
            if parts.len() < 3 {
                println!("Usage: suggest <monster|item|spell> <partial>");
                return;
            }
            let category = parts[1];
            let partial = parts[2];

            match category {
                "monster" => {
                    for (id, name) in suggest_monster_ids(db, partial) {
                        println!("  [{}] {}", id, name);
                    }
                }
                "item" => {
                    for (id, name) in suggest_item_ids(db, partial) {
                        println!("  [{}] {}", id, name);
                    }
                }
                _ => println!("Unknown category: {}", category),
            }
        }
        "validate" => {
            let _ = validate_and_report(db, map);
        }
        _ => println!("Unknown command: {}", parts[0]),
    }
}
```

### Pattern 2: GUI Autocomplete Widget

```rust
struct AutocompleteWidget {
    db: ContentDatabase,
    input_text: String,
    suggestions: Vec<(u32, String)>,
}

impl AutocompleteWidget {
    fn on_text_changed(&mut self, new_text: &str, category: &str) {
        self.input_text = new_text.to_string();

        // Update suggestions based on category
        self.suggestions = match category {
            "monster" => suggest_monster_ids(&self.db, new_text),
            "item" => suggest_item_ids(&self.db, new_text),
            "spell" => suggest_spell_ids(&self.db, new_text),
            _ => Vec::new(),
        };
    }

    fn render(&self) {
        // Render input box
        println!("Input: {}", self.input_text);

        // Render suggestions dropdown
        if !self.suggestions.is_empty() {
            println!("Suggestions:");
            for (id, name) in &self.suggestions {
                println!("  [{}] {}", id, name);
            }
        }
    }
}
```

### Pattern 3: Validation on Save

```rust
fn save_map_with_validation(db: &ContentDatabase, map: &Map, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Validate before saving
    let errors = validate_map(db, map)?;

    let has_errors = errors.iter().any(|e| e.severity() == Severity::Error);

    if has_errors {
        println!("❌ Cannot save map with validation errors:");
        for error in errors.iter().filter(|e| e.severity() == Severity::Error) {
            println!("   {}", error);
        }
        return Err("Validation failed".into());
    }

    // Show warnings but allow save
    let warnings: Vec<_> = errors.iter().filter(|e| e.severity() == Severity::Warning).collect();
    if !warnings.is_empty() {
        println!("⚠️  Saving map with {} warnings:", warnings.len());
        for warning in warnings {
            println!("   {}", warning);
        }
    }

    // Serialize and save
    let ron_string = ron::ser::to_string_pretty(map, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, ron_string)?;

    println!("✅ Saved map to {}", path);
    Ok(())
}
```

---

## Common Pitfalls

### Pitfall 1: Not Loading Database

**Wrong:**
```rust
let db = ContentDatabase::new(); // Empty database!
let monsters = browse_monsters(&db); // Returns empty vector
```

**Right:**
```rust
let db = ContentDatabase::load_campaign("campaigns/my_campaign")?;
let monsters = browse_monsters(&db); // Returns actual monsters
```

### Pitfall 2: Ignoring Validation Errors

**Wrong:**
```rust
let errors = validate_map(&db, &map)?;
// Assume map is valid, save anyway
```

**Right:**
```rust
let errors = validate_map(&db, &map)?;
let has_errors = errors.iter().any(|e| e.severity() == Severity::Error);
if has_errors {
    return Err("Cannot save invalid map".into());
}
```

### Pitfall 3: Not Handling Empty Suggestions

**Wrong:**
```rust
let suggestions = suggest_monster_ids(&db, "xyz");
let (id, _) = suggestions[0]; // Panics if empty!
```

**Right:**
```rust
let suggestions = suggest_monster_ids(&db, "xyz");
if suggestions.is_empty() {
    println!("No matches found");
    return;
}
let (id, _) = &suggestions[0];
```

---

## Performance Tips

1. **Cache Database**: Load once, reuse throughout editor session
2. **Limit Browsing**: Use `.take(N)` to limit displayed results
3. **Validate on Save**: Don't validate on every keystroke
4. **Use Quick Checks**: `is_valid_*()` is faster than full validation

---

## See Also

- [SDK Implementation Plan](../explanation/sdk_and_campaign_architecture.md)
- [Phase 4 Implementation Summary](../explanation/phase4_map_editor_integration.md)
- [Map Format Documentation](../reference/map_ron_format.md)
- [Creating Maps Guide](creating_maps.md)
