# Name Generator Quick Start

Get started with the Antares name generator in 5 minutes.

## What Is It?

A fantasy character name generator for creating NPCs and characters in your Antares campaigns. Features four thematic styles:

- **Fantasy**: Traditional RPG names (Thalion, Kormendor, Velwen)
- **Star**: Celestial-themed names (Antarion, Vegaar, Rigelix)
- **Antares**: Warrior/Mars-themed (Crimsonus, Scorpiusar, Marsheart)
- **Arcturus**: Guardian/Bear-themed (Guardianar, Sentinelix, Arcturon)

## Quick Start: CLI

### Install

The tool is included with Antares. Build it once:

```bash
cargo build --bin antares-name-gen
```

### Generate Your First Names

```bash
# Generate 5 names (default)
cargo run --bin antares-name-gen

# Generate 10 star-themed names
cargo run --bin antares-name-gen -n 10 --theme star

# Generate names with backstories
cargo run --bin antares-name-gen -n 5 --theme antares --lore
```

### Example Output

```
=== ANTARES CHARACTER NAMES ===
Theme: Red Supergiant | Rival to Mars | Heart of Scorpius
Provider: Random Generation

1. Crimsonus the Wanderer
   The blood-red light of Antares illuminated Crimsonus the Wanderer's birth, 
   granting them the strength of the cosmos.

2. Scorpiusar
   Born under the scorpion's heart, Scorpiusar carries the fierce legacy of 
   Antares within.

3. Marsheart the Eternal
   Marked by the red star Antares, Marsheart the Eternal walks the path 
   between light and shadow.
```

## Quick Start: Rust API

### Add to Your Code

```rust
use antares::sdk::name_generator::{NameGenerator, NameTheme};

fn main() {
    let generator = NameGenerator::new();
    
    // Generate a name
    let npc_name = generator.generate(NameTheme::Fantasy);
    println!("NPC: {}", npc_name);
    
    // Generate with a title (40% chance)
    let hero = generator.generate_with_title(NameTheme::Star);
    println!("Hero: {}", hero);
    
    // Generate with backstory
    let (name, lore) = generator.generate_with_lore(NameTheme::Antares);
    println!("{}: {}", name, lore);
    
    // Generate multiple names
    let npcs = generator.generate_multiple(10, NameTheme::Fantasy);
    for name in npcs {
        println!("- {}", name);
    }
}
```

### Run the Example

```bash
cargo run --example name_generator_example
```

## Common Tasks

### Populate a Town

Generate 20 generic NPC names:

```bash
cargo run --bin antares-name-gen -n 20 --theme fantasy --quiet > npcs.txt
```

### Create Pre-Generated Characters

Generate characters with backstories:

```bash
cargo run --bin antares-name-gen -n 6 --theme star --lore > characters.txt
```

### Generate Guard Names

Use the Arcturus (guardian) theme:

```bash
cargo run --bin antares-name-gen -n 10 --theme arcturus
```

### Generate Enemy Names

Use the Antares (warrior) theme:

```bash
cargo run --bin antares-name-gen -n 10 --theme antares --lore
```

## Theme Selection Guide

| Theme     | Best For                          | Example Names           |
|-----------|-----------------------------------|-------------------------|
| Fantasy   | Generic NPCs, townspeople         | Thalion, Kormendor      |
| Star      | Astronomers, mystics, celestial   | Antarion, Vegaar        |
| Antares   | Warriors, aggressive characters   | Crimsonus, Scorpiusar   |
| Arcturus  | Guardians, protectors, wise elders| Guardianar, Sentinelix  |

## CLI Options

```
antares-name-gen [OPTIONS]

Options:
  -n, --number <N>    Number of names (default: 5)
  -t, --theme <THEME> Theme: fantasy, star, antares, arcturus (default: fantasy)
  -l, --lore          Include lore descriptions
  -q, --quiet         Names only (no header)
  -h, --help          Show help
  -V, --version       Show version
```

## Next Steps

- **Full Guide**: See `docs/how-to/use_name_generator.md` for detailed usage
- **API Docs**: Run `cargo doc --open` and search for `name_generator`
- **Examples**: Check `examples/name_generator_example.rs` for code samples

## Troubleshooting

**Problem**: Names feel too similar

**Solution**: Generate more names for better variety:
```bash
cargo run --bin antares-name-gen -n 50 --theme star
```

**Problem**: Want a specific style not covered by themes

**Solution**: Use Fantasy theme as a base and manually edit the output, or run multiple times until you find names you like.

## Tips

1. **Mix themes** for diverse NPCs in a single campaign
2. **Use quiet mode** (`-q`) for scripting and bulk generation
3. **Save outputs** to text files for future reference
4. **Generate extras** - make more names than you need
5. **Combine with lore** for quick NPC backstories during session prep

## Help

For more information:
- CLI help: `cargo run --bin antares-name-gen -- --help`
- API docs: `cargo doc --open`
- Implementation details: `docs/explanation/implementations.md`
