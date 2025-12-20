# How to Use the Name Generator

This guide shows you how to use the Antares name generator for creating character and NPC names in your campaigns.

## Overview

The name generator provides thematic names inspired by celestial bodies, particularly:

- **Antares**: The red supergiant "rival to Mars", heart of Scorpius
- **Arcturus**: The "Guardian of the Bear", bright northern star
- **Star**: General celestial-themed names
- **Fantasy**: Traditional fantasy RPG names

## Using the Command-Line Tool

### Basic Usage

Generate 5 fantasy names (default):

```bash
cargo run --bin antares-name-gen
```

### Generate Star-Themed Names

```bash
cargo run --bin antares-name-gen --theme star -n 10
```

Output example:
```
=== STAR-THEMED CHARACTER NAMES ===
Theme: Celestial Bodies & Constellations
Provider: Random Generation

1. Antarion
2. Vegaar
3. Rigelix
4. Siriuseth
5. Altairian
6. Betelus
7. Denebos
8. Polluxis
9. Spicael
10. Algolix
```

### Generate Names with Lore

Add backstory descriptions to each name:

```bash
cargo run --bin antares-name-gen --theme antares --lore -n 3
```

Output example:
```
=== ANTARES CHARACTER NAMES ===
Theme: Red Supergiant | Rival to Mars | Heart of Scorpius
Provider: Random Generation

1. Crimsonus the Wanderer
   The blood-red light of Antares illuminated Crimsonus the Wanderer's birth, granting them the strength of the cosmos.

2. Scorpiusar
   Born under the scorpion's heart, Scorpiusar carries the fierce legacy of Antares within.

3. Marsheart
   Marked by the red star Antares, Marsheart walks the path between light and shadow.
```

### Generate Arcturus-Themed Names

Perfect for guardian or protector characters:

```bash
cargo run --bin antares-name-gen --theme arcturus --lore -n 5
```

### Quiet Mode (For Scripting)

Generate names without headers for use in scripts:

```bash
cargo run --bin antares-name-gen --theme fantasy -n 100 --quiet > npc_names.txt
```

This creates a simple list:
```
Thalion
Kormendor
Malric
Velwen
Drathor
...
```

## Using the Rust API

### In Your Campaign Builder Code

```rust
use antares::sdk::name_generator::{NameGenerator, NameTheme};

fn main() {
    let generator = NameGenerator::new();
    
    // Generate a single name
    let npc_name = generator.generate(NameTheme::Fantasy);
    println!("NPC: {}", npc_name);
}
```

### Generate Names with Titles

```rust
use antares::sdk::name_generator::{NameGenerator, NameTheme};

let generator = NameGenerator::new();

// 40% chance of including a title like "the Brave" or "Starborn"
let hero_name = generator.generate_with_title(NameTheme::Star);
println!("Hero: {}", hero_name);
// Output might be: "Antarion the Wise" or "Vegaar Starborn"
```

### Generate Names with Lore

```rust
use antares::sdk::name_generator::{NameGenerator, NameTheme};

let generator = NameGenerator::new();

let (name, lore) = generator.generate_with_lore(NameTheme::Antares);
println!("{}", name);
println!("Background: {}", lore);
```

### Batch Generation

Generate multiple names at once:

```rust
use antares::sdk::name_generator::{NameGenerator, NameTheme};

let generator = NameGenerator::new();

// Generate 20 townsfolk names
let townsfolk = generator.generate_multiple(20, NameTheme::Fantasy);
for name in townsfolk {
    println!("- {}", name);
}
```

### Generate Complete NPC List

Generate names with backstories for a whole town:

```rust
use antares::sdk::name_generator::{NameGenerator, NameTheme};

let generator = NameGenerator::new();

let npcs = generator.generate_multiple_with_lore(10, NameTheme::Arcturus);

for (i, (name, backstory)) in npcs.iter().enumerate() {
    println!("{}. {}", i + 1, name);
    println!("   {}\n", backstory);
}
```

## Theme Selection Guide

### Fantasy Theme

Use for: Generic NPCs, townspeople, generic adventurers

Characteristics:
- Traditional fantasy naming conventions
- Easy to pronounce
- Combinations like: Thalion, Kormendor, Malric, Velwen

```bash
cargo run --bin antares-name-gen --theme fantasy -n 10
```

### Star Theme

Use for: Celestial-themed characters, astronomers, mystics, stargazers

Characteristics:
- Based on real star names (Sirius, Vega, Rigel, etc.)
- Exotic but recognizable
- Combinations like: Antarion, Vegaar, Rigelix, Siriuseth

```bash
cargo run --bin antares-name-gen --theme star -n 10
```

### Antares Theme

Use for: Warriors, fierce characters, scorpion cult members, red-themed characters

Characteristics:
- Aggressive, powerful-sounding names
- Red/crimson/Mars associations
- Scorpion constellation theme
- Combinations like: Crimsonus, Scorpiusar, Marsheart, RedEclipse

```bash
cargo run --bin antares-name-gen --theme antares -n 10
```

### Arcturus Theme

Use for: Guardians, protectors, wise elders, northern tribes, bear-themed characters

Characteristics:
- Guardian and protector themes
- Bear constellation associations
- Northern star symbolism
- Combinations like: Guardianar, Bearwatcher, Sentinelix, Arcturon

```bash
cargo run --bin antares-name-gen --theme arcturus -n 10
```

## Common Use Cases

### 1. Populating a Town

Generate varied NPC names for a town or city:

```bash
# Generate 30 generic names
cargo run --bin antares-name-gen --theme fantasy -n 30 --quiet > town_npcs.txt

# Generate 10 guard names (guardian theme)
cargo run --bin antares-name-gen --theme arcturus -n 10 --quiet > town_guards.txt

# Generate 5 wizard names (star theme)
cargo run --bin antares-name-gen --theme star -n 5 --lore > town_wizards.txt
```

### 2. Creating Pre-Generated Characters

Generate characters with backstories for players to choose from:

```bash
cargo run --bin antares-name-gen --theme star -n 6 --lore > pregen_characters.txt
```

### 3. Enemy NPC Names

Generate antagonist names:

```bash
# Aggressive warrior enemies
cargo run --bin antares-name-gen --theme antares -n 10 --lore

# Boss-level enemies with titles
cargo run --bin antares-name-gen --theme antares -n 5 --lore
```

### 4. Integration with Character Editor

In your character editor code:

```rust
use antares::sdk::name_generator::{NameGenerator, NameTheme};

fn on_generate_name_button_clicked(&mut self, race_id: &str) {
    let generator = NameGenerator::new();
    
    // Choose theme based on race or class
    let theme = match race_id {
        "elf" => NameTheme::Star,      // Elves get celestial names
        "dwarf" => NameTheme::Fantasy,  // Dwarves get traditional names
        "human" => NameTheme::Fantasy,  // Humans get standard names
        _ => NameTheme::Fantasy,
    };
    
    self.character_name = generator.generate_with_title(theme);
}
```

## CLI Reference

### Command-Line Options

```
antares-name-gen [OPTIONS]

Options:
  -n, --number <NUMBER>    Number of names to generate [default: 5]
  -t, --theme <THEME>      Theme for name generation [default: fantasy]
                           Possible values: fantasy, star, antares, arcturus
  -l, --lore               Include lore descriptions with each name
  -q, --quiet              Suppress header output (names only)
  -h, --help               Print help
  -V, --version            Print version
```

### Examples

```bash
# Default: 5 fantasy names
antares-name-gen

# 10 star-themed names
antares-name-gen -n 10 -t star

# 3 Antares names with lore
antares-name-gen -n 3 -t antares -l

# 100 names for scripting
antares-name-gen -n 100 -q > names.txt

# Mix and match
antares-name-gen --number 15 --theme arcturus --lore --quiet
```

## Tips and Best Practices

1. **Variety**: Mix themes within a campaign for diversity
2. **Consistency**: Use the same theme for related NPCs (e.g., all town guards use Arcturus theme)
3. **Scripting**: Use `--quiet` mode for generating large batches to import into databases
4. **Lore Integration**: Use `--lore` flag to generate quick backstories during session prep
5. **Reusability**: Generate more names than you need and keep a list for future sessions

## Troubleshooting

### Names Feel Too Similar

Generate more names or mix themes:

```bash
# Generate 50 names for better variety
cargo run --bin antares-name-gen -n 50 --theme star
```

### Want More Control

The generator uses randomization. Run it multiple times until you find names you like:

```bash
# Keep running until satisfied
cargo run --bin antares-name-gen -n 10 --theme antares
```

### Need Different Naming Style

Currently, the generator provides four themes. For completely custom names:
- Use the fantasy theme as a base
- Manually edit generated names
- Consider contributing custom word lists to the project

## Related Documentation

- `docs/reference/architecture.md` - Overall system architecture
- `docs/explanation/implementations.md` - Implementation details
- `src/sdk/name_generator.rs` - API documentation
