# Using the `antares-sdk` CLI

**Target Audience**: Campaign creators, content designers, tool developers
**Difficulty**: Beginner to Intermediate

This guide covers every subcommand in the unified `antares-sdk` binary introduced
in the SDK CLI consolidation. All developer and content-creation tools are now
accessed through a single entry point.

---

## Table of Contents

1. [Overview](#overview)
2. [Building and Installing](#building-and-installing)
3. [Global Flags](#global-flags)
4. [Name Generator — `antares-sdk names`](#name-generator--antares-sdk-names)
5. [Campaign Validator — `antares-sdk campaign validate`](#campaign-validator--antares-sdk-campaign-validate)
6. [Class Editor — `antares-sdk class`](#class-editor--antares-sdk-class)
7. [Race Editor — `antares-sdk race`](#race-editor--antares-sdk-race)
8. [Item Editor — `antares-sdk item`](#item-editor--antares-sdk-item)
9. [Map Tools — `antares-sdk map`](#map-tools--antares-sdk-map)
10. [Texture Generator — `antares-sdk textures generate`](#texture-generator--antares-sdk-textures-generate)
11. [Migration from Old Binaries](#migration-from-old-binaries)
12. [Common Workflows](#common-workflows)
13. [Troubleshooting](#troubleshooting)

---

## Overview

All Antares SDK tools are now unified under a single binary:

```bash
antares-sdk <SUBCOMMAND> [OPTIONS]
```

| Subcommand                 | Purpose                                           |
| -------------------------- | ------------------------------------------------- |
| `names`                    | Generate fantasy character names for NPCs         |
| `campaign validate`        | Validate a campaign directory for correctness     |
| `class`                    | Interactive class definition editor (REPL)        |
| `race`                     | Interactive race definition editor (REPL)         |
| `item`                     | Interactive item definition editor (REPL)         |
| `map validate`             | Validate one or more map RON files                |
| `map build`                | Interactive map builder REPL                      |
| `textures generate`        | Generate placeholder terrain and tree textures    |

Run `antares-sdk --help` to see this list at any time.
Run `antares-sdk <SUBCOMMAND> --help` for subcommand-specific help.

---

## Building and Installing

### Build the unified binary

```bash
cargo build --release --bin antares-sdk
```

The binary is placed at `target/release/antares-sdk`.

### Add to PATH (optional)

```bash
# macOS / Linux
export PATH="$PATH:$(pwd)/target/release"

# Or copy to a system location
sudo cp target/release/antares-sdk /usr/local/bin/
```

### Run without installing

All examples in this guide use `cargo run` so you do not need to install
the binary:

```bash
cargo run --bin antares-sdk -- <SUBCOMMAND> [OPTIONS]
```

---

## Global Flags

The following flags may be placed **before** the subcommand name and apply to
all subcommands:

| Flag        | Effect                                                   |
| ----------- | -------------------------------------------------------- |
| `--verbose` | Enable debug-level logging (tracing at `DEBUG`)          |
| `--quiet`   | Suppress informational output (tracing at `ERROR`)       |
| `--help`    | Print top-level help and exit                            |
| `--version` | Print the binary version and exit                        |

### Examples

```bash
# Enable debug logging for a campaign validation run
antares-sdk --verbose campaign validate campaigns/tutorial

# Suppress all non-error output while generating names
antares-sdk --quiet names --theme fantasy --number 100
```

> **Note**: Each subcommand may also have its own `--quiet` flag with
> subcommand-specific semantics (e.g. `names --quiet` suppresses the header
> banner). These are independent of the top-level `--quiet`.

---

## Name Generator — `antares-sdk names`

Generates fantasy character names for NPCs, towns, and worlds.

### Synopsis

```
antares-sdk names [OPTIONS]
```

### Options

| Flag                    | Description                                      | Default    |
| ----------------------- | ------------------------------------------------ | ---------- |
| `-n, --number <N>`      | Number of names to generate                      | `5`        |
| `-t, --theme <THEME>`   | Name theme (see themes table below)              | `fantasy`  |
| `-l, --lore`            | Include a short lore / backstory with each name  | `false`    |
| `-q, --quiet`           | Print names only, no decorative header           | `false`    |

### Themes

| Theme      | Best For                                       | Example Names           |
| ---------- | ---------------------------------------------- | ----------------------- |
| `fantasy`  | Generic NPCs, townspeople, merchants           | Thalion, Kormendor      |
| `star`     | Astronomers, mystics, celestial beings         | Antarion, Vegaar        |
| `antares`  | Warriors, aggressive characters, antagonists   | Crimsonus, Scorpiusar   |
| `arcturus` | Guardians, protectors, wise elders             | Guardianar, Sentinelix  |

### Examples

```bash
# Generate 5 fantasy names (default)
cargo run --bin antares-sdk -- names

# Generate 10 star-themed names
cargo run --bin antares-sdk -- names --number 10 --theme star

# Generate names with backstories
cargo run --bin antares-sdk -- names --number 5 --theme antares --lore

# Bulk-generate 50 names for a town, saving to a file
cargo run --bin antares-sdk -- names --number 50 --theme fantasy --quiet > npcs.txt

# Generate 6 hero names with lore for session prep
cargo run --bin antares-sdk -- names --number 6 --theme star --lore > heroes.txt
```

### Example Output

```
=== ANTARES CHARACTER NAMES ===
Theme: Fantasy | Provider: Random Generation

1. Thalion the Swift
2. Kormendor
3. Velwen of the North
4. Aldris the Wise
5. Eryndal
```

---

## Campaign Validator — `antares-sdk campaign validate`

Validates the content of an Antares campaign directory, checking for
broken references, duplicate IDs, disconnected maps, and data integrity
errors.

### Synopsis

```
antares-sdk campaign validate [OPTIONS] [CAMPAIGN]
```

### Arguments

| Argument     | Description                                                  |
| ------------ | ------------------------------------------------------------ |
| `[CAMPAIGN]` | Path to the campaign directory to validate (e.g. `campaigns/tutorial`) |

### Options

| Flag                   | Description                                                  | Default  |
| ---------------------- | ------------------------------------------------------------ | -------- |
| `-a, --all`            | Validate all campaigns in the search directory               | `false`  |
| `-d, --dir <DIR>`      | Directory to search when `--all` is used                     | `campaigns/` |
| `-v, --verbose`        | Print detailed validation results including all warnings     | `false`  |
| `-j, --json`           | Output results as JSON (useful for CI pipelines)             | `false`  |
| `-e, --errors-only`    | Suppress warnings; show errors only                          | `false`  |

### Examples

```bash
# Validate a single campaign
cargo run --bin antares-sdk -- campaign validate campaigns/tutorial

# Validate all campaigns under the default campaigns/ directory
cargo run --bin antares-sdk -- campaign validate --all

# Validate all campaigns in a custom directory
cargo run --bin antares-sdk -- campaign validate --all --dir my_campaigns/

# Verbose validation with all warnings shown
cargo run --bin antares-sdk -- --verbose campaign validate campaigns/tutorial

# Output JSON for CI integration
cargo run --bin antares-sdk -- campaign validate campaigns/tutorial --json

# Show errors only (suppress warnings)
cargo run --bin antares-sdk -- campaign validate campaigns/tutorial --errors-only
```

### Example Output

```
Loading campaign from: campaigns/tutorial
✓ Loaded campaign: Tutorial Campaign v1.0.0

Content Summary:
  Classes:    5
  Races:      4
  Items:     120
  Monsters:   45
  Spells:     30
  Maps:       15

Running validation...
✓ No errors found.

Campaign is valid!
```

---

## Class Editor — `antares-sdk class`

Interactive menu-driven REPL for creating and editing character class
definitions stored in RON format.

### Synopsis

```
antares-sdk class [OPTIONS] [FILE]
```

### Arguments

| Argument | Description                                      | Default              |
| -------- | ------------------------------------------------ | -------------------- |
| `[FILE]` | Path to the classes RON file                     | `data/classes.ron`   |

### Options

| Flag                     | Description                                                          |
| ------------------------ | -------------------------------------------------------------------- |
| `--campaign <DIR>`       | Campaign directory; opens `<DIR>/data/classes.ron` instead of FILE   |

### Examples

```bash
# Edit the default classes file
cargo run --bin antares-sdk -- class

# Edit a specific file
cargo run --bin antares-sdk -- class campaigns/tutorial/data/classes.ron

# Use --campaign shorthand (equivalent to the line above)
cargo run --bin antares-sdk -- class --campaign campaigns/tutorial

# Edit a custom path
cargo run --bin antares-sdk -- class /path/to/my/classes.ron
```

### Interface

```
╔════════════════════════════════════════╗
║    ANTARES CLASS EDITOR v0.1.0         ║
╚════════════════════════════════════════╝
File: data/classes.ron
Classes: 5

  [1] List Classes
  [2] Add Class
  [3] Edit Class
  [4] Delete Class
  [5] Preview Class
  [6] Save & Exit
  [Q] Quit (discard changes)
```

---

## Race Editor — `antares-sdk race`

Interactive menu-driven REPL for creating and editing character race
definitions stored in RON format.

### Synopsis

```
antares-sdk race [OPTIONS] [FILE]
```

### Arguments

| Argument | Description                                  | Default            |
| -------- | -------------------------------------------- | ------------------ |
| `[FILE]` | Path to the races RON file                   | `data/races.ron`   |

### Options

| Flag                 | Description                                                        |
| -------------------- | ------------------------------------------------------------------ |
| `--campaign <DIR>`   | Campaign directory; opens `<DIR>/data/races.ron` instead of FILE   |

### Examples

```bash
# Edit the default races file
cargo run --bin antares-sdk -- race

# Edit a campaign-specific file
cargo run --bin antares-sdk -- race --campaign campaigns/tutorial

# Edit a specific file path
cargo run --bin antares-sdk -- race campaigns/my_campaign/data/races.ron
```

### Interface

```
========================================
    ANTARES RACE EDITOR v0.2.0
========================================
File: data/races.ron
Races: 4

  [1] List Races
  [2] Add Race
  [3] Edit Race
  [4] Delete Race
  [5] Preview Race
  [6] Save & Exit
  [Q] Quit (discard changes)
```

---

## Item Editor — `antares-sdk item`

Interactive menu-driven editor for creating and editing item definitions.
Supports weapons, armor, accessories, consumables, ammunition, and quest items.

### Synopsis

```
antares-sdk item [OPTIONS] [FILE]
```

### Arguments

| Argument | Description                                  | Default          |
| -------- | -------------------------------------------- | ---------------- |
| `[FILE]` | Path to the items RON file                   | `data/items.ron` |

### Options

| Flag                 | Description                                                       |
| -------------------- | ----------------------------------------------------------------- |
| `--campaign <DIR>`   | Campaign directory; opens `<DIR>/data/items.ron` instead of FILE  |

### Examples

```bash
# Edit the default items file
cargo run --bin antares-sdk -- item

# Edit a campaign-specific file
cargo run --bin antares-sdk -- item --campaign campaigns/tutorial

# Edit a specific file path
cargo run --bin antares-sdk -- item campaigns/my_campaign/data/items.ron
```

### Supported Item Types

| Type         | Description                                          |
| ------------ | ---------------------------------------------------- |
| `Weapon`     | Melee and ranged weapons with damage dice            |
| `Armor`      | Light, medium, and heavy armor with AC bonuses       |
| `Accessory`  | Rings, amulets, belts, cloaks with stat bonuses      |
| `Consumable` | Potions, food, and single-use effects                |
| `Ammo`       | Arrows, bolts, and stones                            |
| `Quest`      | Quest-specific key items                             |

---

## Map Tools — `antares-sdk map`

The `map` subcommand groups two tools: the interactive map builder REPL and
the map file validator.

### `antares-sdk map build` — Interactive Map Builder

Interactive REPL for creating and editing game maps stored in RON format.

```
antares-sdk map build
```

The builder launches an interactive prompt where you can:
- Create a new map with custom dimensions
- Set individual tiles (floor, wall, door, water, etc.)
- Bulk-fill regions with a terrain type
- Add events (combat, treasure, text, teleport, etc.)
- Show a text-art preview of the map
- Save the map to a RON file

#### Quick Reference

```
> help         — List all commands
> create       — Create a new map
> load <file>  — Load an existing map
> set x y <T>  — Set tile at (x, y) to terrain T
> fill ...     — Bulk fill a region
> event ...    — Add a map event
> show         — Display map preview
> save <file>  — Save map to file
> quit         — Exit the builder
```

#### Example Session

```bash
cargo run --bin antares-sdk -- map build
> create 20 20 dungeon
Map created: 20×20 (dungeon)
> set 0 0 floor
> fill 0 0 20 20 floor
> save data/maps/dungeon_1.ron
Saved to data/maps/dungeon_1.ron
> quit
```

---

### `antares-sdk map validate` — Map Validator

Validates one or more map RON files for structural correctness, checking tile
consistency, event references (monster IDs, item IDs), and exit connectivity.

#### Synopsis

```
antares-sdk map validate [OPTIONS] <FILE>...
```

#### Arguments

| Argument    | Description                            |
| ----------- | -------------------------------------- |
| `<FILE>...` | One or more map RON files to validate  |

#### Options

| Flag                     | Description                                                    |
| ------------------------ | -------------------------------------------------------------- |
| `--campaign-dir <DIR>`   | Campaign directory used to resolve monster and item IDs         |

#### Examples

```bash
# Validate a single map
cargo run --bin antares-sdk -- map validate data/maps/dungeon_1.ron

# Validate multiple maps at once
cargo run --bin antares-sdk -- map validate data/maps/town.ron data/maps/dungeon_1.ron

# Validate with campaign context for ID resolution
cargo run --bin antares-sdk -- map validate \
  --campaign-dir campaigns/tutorial \
  campaigns/tutorial/data/maps/map_1.ron

# Validate all maps in a directory (using shell glob)
cargo run --bin antares-sdk -- map validate data/maps/*.ron
```

---

## Texture Generator — `antares-sdk textures generate`

Generates placeholder PNG textures for terrain tiles and tree sprites. Useful
for prototyping new campaigns before custom artwork is ready.

### Synopsis

```
antares-sdk textures generate [OPTIONS]
```

### Options

| Flag                    | Description                                      | Default             |
| ----------------------- | ------------------------------------------------ | ------------------- |
| `--output-dir <DIR>`    | Directory to write the generated PNG files into  | `assets/textures`   |

### Examples

```bash
# Generate textures into the default assets/textures directory
cargo run --bin antares-sdk -- textures generate

# Generate into a custom output directory
cargo run --bin antares-sdk -- textures generate --output-dir /tmp/prototype_textures

# Generate into a campaign-specific asset directory
cargo run --bin antares-sdk -- textures generate \
  --output-dir campaigns/my_campaign/assets/textures
```

### Generated Files

The generator produces PNG files for:
- Ground terrain types (grass, dirt, stone, sand, snow, etc.)
- Wall tiles (stone, brick, wood)
- Water and lava tiles
- Forest and tree sprites
- Special tiles (door, chest, stairs)

---

## Migration from Old Binaries

If you were using the individual tool binaries from earlier SDK versions,
here is the mapping to the new unified `antares-sdk` commands:

| Old Command                                                       | New Command                                                          |
| ----------------------------------------------------------------- | -------------------------------------------------------------------- |
| `cargo run --bin name_gen -- -n 5 -t fantasy`                     | `cargo run --bin antares-sdk -- names -n 5 --theme fantasy`          |
| `cargo run --bin class_editor`                                    | `cargo run --bin antares-sdk -- class`                               |
| `cargo run --bin class_editor -- path/to/classes.ron`             | `cargo run --bin antares-sdk -- class path/to/classes.ron`           |
| `cargo run --bin race_editor`                                     | `cargo run --bin antares-sdk -- race`                                |
| `cargo run --bin item_editor`                                     | `cargo run --bin antares-sdk -- item`                                |
| `cargo run --bin map_builder`                                     | `cargo run --bin antares-sdk -- map build`                           |
| `cargo run --bin validate_map -- map.ron`                         | `cargo run --bin antares-sdk -- map validate map.ron`                |
| `cargo run --bin campaign_validator -- campaigns/tutorial`        | `cargo run --bin antares-sdk -- campaign validate campaigns/tutorial`|
| `cargo run --bin generate_terrain_textures`                       | `cargo run --bin antares-sdk -- textures generate`                   |
| `cargo run --bin update_tutorial_maps`                            | *(deleted — one-time migration, no longer needed)*                   |

### New `--campaign` shorthand

The class, race, and item editors accept a `--campaign <DIR>` flag that
resolves the correct data file automatically:

```bash
# Old way
cargo run --bin class_editor -- campaigns/tutorial/data/classes.ron

# New shorthand
cargo run --bin antares-sdk -- class --campaign campaigns/tutorial
```

---

## Common Workflows

### Workflow 1: Create a New Campaign from Scratch

```bash
# 1. Set up the directory structure
mkdir -p campaigns/new_campaign/data/maps

# 2. Create class definitions
cargo run --bin antares-sdk -- class --campaign campaigns/new_campaign

# 3. Create race definitions
cargo run --bin antares-sdk -- race --campaign campaigns/new_campaign

# 4. Create item definitions
cargo run --bin antares-sdk -- item --campaign campaigns/new_campaign

# 5. Build maps
cargo run --bin antares-sdk -- map build
#    > create 20 20 outdoor
#    > ... (design your map) ...
#    > save campaigns/new_campaign/data/maps/world_1.ron

# 6. Validate the campaign
cargo run --bin antares-sdk -- campaign validate campaigns/new_campaign
```

### Workflow 2: Generate NPC Names for a Town

```bash
# Generate 20 townspeople names, save to file
cargo run --bin antares-sdk -- names --number 20 --theme fantasy --quiet > npcs.txt

# Generate 6 hero names with backstories
cargo run --bin antares-sdk -- names --number 6 --theme star --lore > heroes.txt

# Generate 10 guard names (Arcturus = guardian theme)
cargo run --bin antares-sdk -- names --number 10 --theme arcturus
```

### Workflow 3: Validate Before Committing

```bash
# Validate all campaigns before a git commit
cargo run --bin antares-sdk -- campaign validate --all

# Or validate a specific campaign verbosely
cargo run --bin antares-sdk -- --verbose campaign validate campaigns/tutorial
```

### Workflow 4: Prototype Textures Then Validate Maps

```bash
# 1. Generate placeholder textures
cargo run --bin antares-sdk -- textures generate

# 2. Build a map
cargo run --bin antares-sdk -- map build

# 3. Validate the map
cargo run --bin antares-sdk -- map validate data/maps/new_map.ron \
  --campaign-dir campaigns/my_campaign

# 4. Validate the full campaign
cargo run --bin antares-sdk -- campaign validate campaigns/my_campaign
```

---

## Troubleshooting

### `antares-sdk: command not found`

The binary has not been built yet, or it is not on your PATH.

```bash
cargo build --bin antares-sdk
./target/debug/antares-sdk --help
```

### `error: unrecognized subcommand`

Check the spelling of the subcommand. Run `antares-sdk --help` to see the
full list.

### `ParseError` when loading a RON file

A RON syntax error exists in the data file. Common causes:

- Missing commas between struct fields
- Unclosed parentheses or braces
- Incorrect enum variant name capitalisation

Use the campaign validator to identify the problem:

```bash
cargo run --bin antares-sdk -- campaign validate campaigns/my_campaign --verbose
```

### Map validation reports unknown monster or item IDs

Provide the `--campaign-dir` flag so the validator can load the campaign's
`monsters.ron` and `items.ron` for ID resolution:

```bash
cargo run --bin antares-sdk -- map validate map.ron \
  --campaign-dir campaigns/my_campaign
```

### Editor changes not persisting

Make sure to choose **Save & Exit** (option `6`) in the interactive REPL
before quitting. Pressing `Q` discards all unsaved changes.

---

## See Also

- **Name Generator Tutorial**: `docs/tutorials/name_generator_quickstart.md`
- **Map Builder Guide**: `docs/how-to/using_map_builder.md`
- **Campaign Authoring Guide**: `docs/how-to/creating_and_validating_campaigns.md`
- **Architecture Reference**: `docs/reference/architecture.md`
- **Implementation Notes**: `docs/explanation/implementations.md`

---

**Last Updated**: 2026
**SDK Version**: 0.1.0
