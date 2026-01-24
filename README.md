# Antares

A classic turn-based RPG built in Rust, inspired by Might and Magic 1.

## Overview

Antares is a modern implementation of classic grid-based dungeon crawling RPGs, featuring:

- **Turn-based tactical combat** with party-based gameplay
- **First-person exploration** through grid-based dungeons
- **Character customization** with multiple races and classes
- **Campaign system** supporting modular, data-driven content
- **Per-campaign configuration** for graphics, audio, controls, and camera settings

## Features

### Core Gameplay

- **Party Management**: Create and manage adventuring parties (up to 6 active members)
- **Character Classes**: Knight, Paladin, Archer, Cleric, Sorcerer, Robber
- **Character Races**: Human, Elf, Dwarf, Gnome, Half-Orc, Halfling
- **Combat System**: Turn-based tactical combat with monsters and spells
- **Magic System**: Cleric and Sorcerer spell casting with spell points
- **Equipment System**: Weapons, armor, and items with class restrictions
- **Quest System**: Track objectives, progress, and rewards
- **Dialogue System**: Interactive NPC conversations with visual bubbles and choices
- **In-game Menu System**: Save/load games, configure settings, pause and resume gameplay
- **Save/Load**: Campaign progress persistence

### Dialogue System

- **2.5D Visual Dialogues**: Floating text bubbles above NPCs with billboard effect
- **Typewriter Animation**: Character-by-character text reveal for immersive reading
- **Interactive Choices**: Arrow key/number navigation with visual feedback on selected option
- **Branching Conversations**: Complex dialogue trees with conditions and actions
- **Quest Integration**: Start quests, modify game state through dialogue selections
- **RON Data Format**: Easy-to-edit dialogue content in Rusty Object Notation

See `docs/tutorials/dialogue_system_usage.md` for complete usage guide and examples.

### Game Menu System

- **In-Game Menu**: Press ESC to pause and access menu (works during exploration, combat, and dialogue)
- **Save/Load**: Create timestamped save files and restore previous game state
- **Settings Configuration**: Adjust audio volumes (Master, Music, SFX, Ambient)
- **Resume Gameplay**: Return to exact point you left off, preserving all game state
- **Quick Access**: Main menu with Resume, Save, Load, Settings, and Quit options

See `docs/how-to/using_game_menu.md` for complete usage guide.

### Campaign System

- **Modular Campaigns**: Self-contained campaign directories with metadata
- **Data-Driven Content**: Items, spells, monsters, maps defined in RON files
- **Custom Configuration**: Per-campaign graphics, audio, controls, and camera settings
- **Campaign Templates**: Easy-to-copy templates for creating new campaigns

### Game Configuration (Phase 6 - Latest)

Each campaign can customize game settings via `config.ron`:

- **Graphics**: Resolution, fullscreen, vsync, MSAA, shadow quality
- **Audio**: Master/music/SFX/ambient volume controls
- **Controls**: Customizable key bindings for movement, interaction, menus
- **Camera**: Mode (FirstPerson/Tactical/Isometric), FOV, lighting, shadows

See `campaigns/config.template.ron` for a comprehensive template with examples.

## Keyboard Controls

### Gameplay

| Key | Action          |
| --- | --------------- |
| W   | Move forward    |
| A   | Turn left       |
| D   | Turn right      |
| E   | Interact/Use    |
| ESC | Open/Close menu |

### Menu Navigation

| Key        | Action                   |
| ---------- | ------------------------ |
| Up Arrow   | Select previous option   |
| Down Arrow | Select next option       |
| Enter      | Confirm selection        |
| Space      | Confirm selection        |
| Backspace  | Go back to previous menu |
| ESC        | Resume gameplay (Resume) |

## Getting Started

### Prerequisites

- Rust 1.83+ (latest stable)
- Cargo (comes with Rust)

### Building

```bash
# Clone the repository
git clone https://github.com/xbcsmith/antares.git
cd antares

# Build the project
cargo build --release

# Run the tutorial campaign
cargo run --release --bin antares -- campaigns/tutorial
```

### Running Tests

```bash
# Run all tests
cargo nextest run --all-features

# Run specific test suite
cargo nextest run --test game_config_integration

# Run with formatting and linting
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Project Structure

```
antares/
├── campaigns/              # Campaign content
│   ├── tutorial/          # Tutorial campaign
│   │   ├── campaign.ron   # Campaign metadata
│   │   ├── config.ron     # Game configuration
│   │   └── data/          # Game data (items, spells, maps, etc.)
│   └── config.template.ron # Configuration template
├── data/                   # Core game data
├── docs/                   # Documentation
│   ├── explanation/       # Architecture and design docs
│   ├── how-to/            # Task-oriented guides
│   ├── reference/         # Technical specifications
│   └── tutorials/         # Learning-oriented guides
├── src/                    # Core game engine
│   ├── application/       # Application layer (game state, saves, quests)
│   ├── domain/            # Domain layer (characters, combat, items, spells)
│   ├── game/              # Game systems (rendering, input, audio, camera)
│   ├── sdk/               # Campaign SDK (loading, validation, builders)
│   └── bin/               # Executable binaries
├── sdk/                    # Campaign builder SDK
│   └── campaign_builder/  # Visual campaign editor
└── tests/                  # Integration tests
```

## Campaign Development

### Creating a Campaign

1. **Copy the template**:

   ```bash
   cp -r campaigns/tutorial campaigns/my_campaign
   ```

2. **Edit campaign metadata** (`campaign.ron`):

   - Set campaign name, author, description
   - Configure starting conditions (map, position, gold, etc.)
   - Set difficulty and game rules

3. **Customize game configuration** (`config.ron`):

   - Copy from `campaigns/config.template.ron`
   - Adjust graphics settings for your target audience
   - Customize controls and camera mode
   - Set audio levels

4. **Create content**:

   - Define items in `data/items.ron`
   - Create spells in `data/spells.ron`
   - Design monsters in `data/monsters.ron`
   - Build maps in `data/maps/`

5. **Test your campaign**:
   ```bash
   cargo run --bin antares -- campaigns/my_campaign
   ```

### Configuration Examples

See `docs/explanation/game_config_schema.md` for complete documentation and examples:

- **Tactical RPG**: Top-down camera, battlefield overview
- **Action RPG**: First-person, fast rotation, fluid movement
- **Exploration RPG**: Isometric view, atmospheric audio
- **Horror RPG**: Low lighting, high ambient sound

## Tools

### Campaign Builder

Visual editor for creating and editing campaigns (work in progress):

```bash
cargo run --bin campaign-builder
```

### Data Editors

Command-line editors for game data:

```bash
# Item editor
cargo run --bin item_editor

# Spell editor
cargo run --bin spell_editor

# Class editor
cargo run --bin class_editor

# Race editor
cargo run --bin race_editor

# Map builder
cargo run --bin map_builder
```

## Documentation

- **[Architecture](docs/reference/architecture.md)**: System design and structure
- **[Game Config Schema](docs/explanation/game_config_schema.md)**: Configuration reference
- **[Implementation Plan](docs/explanation/game_config_implementation_plan.md)**: Phase-by-phase development plan
- **[Implementations](docs/explanation/implementations.md)**: Completed phase summaries

## Development

### Quality Gates

All code must pass these checks before merging:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

### Contributing

1. Read `AGENTS.md` for AI agent development guidelines
2. Follow the architecture defined in `docs/reference/architecture.md`
3. Use RON format for game data files
4. Add tests for all new functionality
5. Update documentation in `docs/explanation/implementations.md`

## Technology Stack

- **Language**: Rust 2021 Edition
- **Game Engine**: Bevy 0.17
- **UI**: bevy_egui
- **Serialization**: RON, Serde
- **Testing**: Nextest

## Roadmap

### Completed Phases

- ✅ Phase 1: Core Configuration Infrastructure
- ✅ Phase 2: Camera System Integration
- ✅ Phase 3: Input System Integration
- ✅ Phase 4: Graphics Configuration
- ✅ Phase 5: Audio System Foundation
- ✅ Phase 6: Tutorial Campaign Configuration
- ✅ Phase 7: Game Menu System (Save/Load/Settings/Resume)

### Upcoming

- Audio playback integration (music, SFX, ambient)
- Runtime configuration changes (settings menu)
- Hardware detection and quality presets
- Per-map configuration overrides
- Accessibility options (colorblind modes, UI scaling)

## License

This project is licensed under the Apache 2.0 License - see the LICENSE file for details.

## Authors

- Brett Smith (@xbcsmith)
- Antares Contributors

## Acknowledgments

- Inspired by classic RPGs like Might and Magic 1
- Built with Bevy game engine
- Community feedback and contributions
