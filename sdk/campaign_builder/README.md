# Antares Campaign Builder

A full-featured campaign editor for creating Antares RPG campaigns. Provides a
complete graphical interface for authoring campaign metadata, game data, maps,
quests, dialogues, and creature assets — all serialised to RON format.

## Features

### Campaign Metadata Editor

- **Basic Info**: Campaign ID, name, version, author, description, engine version
- **Starting Conditions**: Map, position, direction, gold, food
- **Party Settings**: Max party size, max roster size
- **Difficulty**: Easy/Normal/Hard/Brutal with permadeath and multiclassing options
- **Level Range**: Starting level and max level configuration
- **Data Paths**: Configurable file paths for all game data

### File I/O

- **Save/Load**: RON format serialisation with pretty printing
- **File Dialogs**: Native file picker integration (Save As, Open)
- **Error Handling**: Clear error messages for I/O failures
- **Auto-Save**: Configurable periodic auto-save with crash recovery

### Validation

- **Error Detection**: Required fields, format validation, range checks
- **Warning System**: Non-critical issues flagged separately
- **Color-Coded Display**: Red for errors, orange for warnings
- **Advanced Validation**: Balance analysis, loot economy checks, unreachable
  content detection, quest dependency graph

### Unsaved Changes Protection

- **Change Tracking**: Real-time detection of modifications
- **Visual Indicator**: Status bar shows saved/unsaved state
- **Warning Dialog**: Prevents accidental data loss on destructive actions

### Data Editors

- **Items Editor** — weapons, armour, consumables with full stat editing
- **Spells Editor** — cleric and sorcerer spells with school and effect editing
- **Monsters Editor** — stats, loot tables, resistances, special attacks
- **Classes Editor** — class definitions with autocomplete for proficiencies
- **Races Editor** — race definitions and stat modifiers
- **Conditions Editor** — status condition definitions
- **Proficiencies Editor** — proficiency definitions with validation

### Map Editor

- **Tile Placement**: Terrain type selection and multi-tile editing
- **Advanced Terrain Variants**: Trees, shrubs, grass, mountains, swamp, lava
- **Event Editor**: Place and edit map events with full type coverage
  (Sign, NPC Dialogue, Encounter, Container, Teleport, Combat, etc.)
- **Facing/Behaviour Fields**: Direction and AI behaviour per event
- **Visual Feedback**: Hover highlight, selection state, editing indicators

### Quest Editor

- **Quest List**: Add, edit, delete quests with inline list view
- **Stage Editor**: Modal dialog with stage number, name, description,
  and "require all objectives" flag
- **Objective Editor**: Dynamic forms that adapt to objective type
  (Kill Monsters, Collect Items, Reach Location, Talk To NPC,
  Deliver Item, Escort NPC, Custom Flag)
- **Collapsible View**: Expand/collapse stages to see objectives

### Dialogue Editor

- **Node Tree Editor**: Visual hierarchy of dialogue nodes
- **Reachability Stats**: Detect and highlight unreachable nodes
- **Navigation Controls**: Jump to node, breadcrumb navigation
- **Choice Validation**: Validate jump targets and show errors inline

### NPC Editor

- **NPC List**: Add, edit, delete NPC definitions
- **Stock Templates**: Assign stock item templates to merchants

### Creature Asset Editor

- **Registry Mode**: Browse, search, filter, and sort all registered creature
  mesh assets; open one for editing or register a new asset from disk
- **Three-Panel Edit Mode**: Mesh list (left) | 3D preview (center) |
  mesh properties (right)
- **Undo/Redo**: Full command history for all mesh editing operations
- **Workflow Integration**: Keyboard shortcuts, context menus, auto-save,
  enhanced preview options (grid, axes, bounding box, wireframe, lighting)

### Item Mesh Editor

- **Registry/Edit Modes**: Same two-mode navigation as the Creature Asset Editor
- **Visual Properties**: Colors, scale, emissive settings with live 3D preview
- **Undo/Redo**: Full editing history

### Template Browser

- **Pre-built Templates**: Ready-made items, monsters, quests, dialogues, and maps
- **One-Click Apply**: Insert a template directly into the active data list

### Campaign Packager

- **Distribution Tools**: Export a complete campaign as a self-contained archive
- **Asset Bundling**: Collects all referenced data and map files

### Developer Tools

- **Debug Panel**: Internal state inspector for troubleshooting
- **Logging**: Configurable log level for editor operations
- **File Structure Browser**: Tree view of the campaign directory with
  auto-refresh after save operations

## Installation

### Prerequisites

```bash
# Rust toolchain (1.70+)
rustup --version
```

#### Linux — OpenGL development libraries

```bash
# Ubuntu/Debian
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev

# Fedora
sudo dnf install libxcb-devel libxkbcommon-devel
```

### Building

```bash
# From antares/ root directory
cargo build --release --package campaign_builder

# Or from sdk/campaign_builder/
cargo build --release
```

### Running

```bash
# From antares/ root (Cargo)
cargo run --release --package campaign_builder --bin campaign-builder

# Auto-load the tutorial campaign
cargo run --release --package campaign_builder --bin campaign-builder \
  -- --campaign campaigns/tutorial

# Using the top-level Makefile helper (recommended)
make sdk
```

## Usage Guide

### Creating a New Campaign

1. **Launch** the Campaign Builder
2. **File → New Campaign** (or Ctrl+N)
3. **Metadata Tab**: Fill in basic information
   - Campaign ID: `my_campaign` (alphanumeric + underscores only)
   - Name: `My First Campaign` (display name)
   - Version: `1.0.0` (semantic versioning)
   - Author: Your name
   - Description: Brief summary
4. **Config Tab**: Configure game rules
   - Starting map ID, position, gold, food
   - Party/roster size limits
   - Difficulty and rules (permadeath, multiclassing)
   - Level range and data file paths
5. **Validation**: Click **Tools → Validate Campaign**
   - Fix any errors (red ❌)
   - Review warnings (orange ⚠️)
6. **Save**: **File → Save As...** to choose location

### Opening an Existing Campaign

1. **File → Open Campaign...** (or Ctrl+O)
2. Navigate to the campaign directory and select `campaign.ron`
3. Edit as needed
4. **File → Save** (or Ctrl+S) to update

### Validation Rules

#### Errors (must fix)

- Campaign ID required and alphanumeric + underscores
- Campaign name required
- Version must follow X.Y.Z format
- Starting map required
- Max roster size must be ≥ max party size
- Starting level must be between 1 and max level
- All data file paths required

#### Warnings (recommended)

- Author name recommended
- Engine version should follow X.Y.Z format
- Max party size should be 1–10
- Data files should use `.ron` extension

### Campaign File Structure

```
my_campaign/
├── campaign.ron         # Metadata (edited directly in the Campaign Builder)
└── data/
    ├── items.ron
    ├── spells.ron
    ├── monsters.ron
    ├── classes.ron
    ├── races.ron
    ├── quests.ron
    ├── dialogue.ron
    └── maps/
        ├── town.ron
        └── dungeon.ron
```

Use **Tools → Refresh File Tree** to update the file browser after external changes.

## Keyboard Shortcuts

| Shortcut     | Action        |
| ------------ | ------------- |
| Ctrl+N       | New Campaign  |
| Ctrl+O       | Open Campaign |
| Ctrl+S       | Save Campaign |
| Ctrl+Shift+S | Save As...    |
| Ctrl+Z       | Undo          |
| Ctrl+Y       | Redo          |
| Ctrl+W       | Exit          |

## Testing Without a GPU

The Campaign Builder runs on any hardware, including software rendering:

```bash
# Linux: Force software rendering
LIBGL_ALWAYS_SOFTWARE=1 cargo run --release --package campaign_builder

# Headless server (virtual framebuffer)
xvfb-run cargo run --release --package campaign_builder

# Debug backend selection
RUST_LOG=eframe=debug cargo run --release --package campaign_builder
```

| Hardware           | Expected FPS | Status     |
| ------------------ | ------------ | ---------- |
| Dedicated GPU      | 60+          | Excellent  |
| Integrated GPU     | 60           | Very Good  |
| Software rendering | 30–60        | Acceptable |
| VM (no GPU)        | 30–60        | Usable     |

## Campaign.ron Format

```ron
CampaignMetadata(
    id: "my_first_campaign",
    name: "My First Campaign",
    version: "1.0.0",
    author: "Campaign Creator",
    description: "A classic dungeon crawl adventure",
    engine_version: "0.1.0",
    starting_map: "starter_town",
    starting_position: (10, 10),
    starting_direction: "North",
    starting_gold: 100,
    starting_food: 10,
    max_party_size: 6,
    max_roster_size: 20,
    difficulty: Normal,
    permadeath: false,
    allow_multiclassing: false,
    starting_level: 1,
    max_level: 20,
    items_file: "data/items.ron",
    spells_file: "data/spells.ron",
    monsters_file: "data/monsters.ron",
    classes_file: "data/classes.ron",
    races_file: "data/races.ron",
    maps_dir: "data/maps/",
    quests_file: "data/quests.ron",
    dialogue_file: "data/dialogue.ron",
)
```

## Architecture

### Technology Stack

| Component      | Library                     |
| -------------- | --------------------------- |
| GUI framework  | egui v0.29 (immediate mode) |
| Backend        | eframe with glow (OpenGL)   |
| Serialisation  | RON (Rusty Object Notation) |
| File dialogs   | rfd (native OS dialogs)     |
| Error handling | thiserror                   |

### Source Layout

```
sdk/campaign_builder/
├── Cargo.toml
├── README.md                   # This file
├── QUICKSTART.md               # Quick-start guide
├── src/
│   ├── lib.rs                  # CampaignBuilderApp, top-level UI, RON I/O
│   ├── advanced_validation.rs  # Balance, loot, reachability checks
│   ├── asset_manager.rs        # Asset file management
│   ├── auto_save.rs            # Periodic auto-save and crash recovery
│   ├── campaign_editor.rs      # Campaign metadata editor panel
│   ├── characters_editor.rs    # Character data editor
│   ├── classes_editor.rs       # Class definitions editor
│   ├── conditions_editor.rs    # Status condition editor
│   ├── config_editor.rs        # Game config editor
│   ├── context_menu.rs         # Right-click context menu system
│   ├── creature_undo_redo.rs   # Undo/redo commands for creature editing
│   ├── creatures_editor.rs     # Creature asset editor (registry + edit modes)
│   ├── creatures_manager.rs    # Creature registry file I/O and validation
│   ├── creatures_workflow.rs   # Unified workflow state (undo, shortcuts, menus)
│   ├── dialogue_editor.rs      # Dialogue tree editor
│   ├── furniture_editor.rs     # Furniture/prop definitions editor
│   ├── item_mesh_editor.rs     # Item mesh asset editor
│   ├── items_editor.rs         # Item database editor
│   ├── keyboard_shortcuts.rs   # Keyboard shortcut manager
│   ├── map_editor.rs           # Map tile and event editor
│   ├── mesh_obj_io.rs          # OBJ mesh import/export
│   ├── monsters_editor.rs      # Monster database editor
│   ├── npc_editor.rs           # NPC definitions editor
│   ├── packager.rs             # Campaign packager / distribution tools
│   ├── preview_features.rs     # Enhanced 3D preview options
│   ├── preview_renderer.rs     # Simplified mesh preview renderer
│   ├── proficiencies_editor.rs # Proficiency definitions editor
│   ├── quests_editor.rs        # Quest, stage, and objective editor
│   ├── races_editor.rs         # Race definitions editor
│   ├── spells_editor.rs        # Spell database editor
│   ├── template_browser.rs     # Template browser dialog
│   ├── templates.rs            # Pre-built content templates
│   ├── tray.rs                 # System tray integration
│   ├── ui_helpers.rs           # Shared UI helpers (autocomplete, validation, lists)
│   └── undo_redo.rs            # General undo/redo command stack
└── tests/
    ├── bug_verification.rs
    ├── creature_asset_editor_tests.rs
    ├── furniture_customization_tests.rs
    ├── furniture_editor_tests.rs
    ├── furniture_properties_tests.rs
    ├── gui_integration_test.rs
    ├── map_data_validation.rs
    ├── mesh_editing_tests.rs
    ├── template_system_integration_tests.rs
    └── ui_improvements_test.rs
```

## Testing

```bash
# Run all tests
cargo nextest run --all-features

# Run with output
cargo nextest run --all-features -- --nocapture

# Run a specific test
cargo nextest run --all-features test_validation_all_pass
```

### Quality Gates

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

## Troubleshooting

### Build Errors

**Missing OpenGL libraries (Linux)**

```bash
# Ubuntu/Debian
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev

# Fedora
sudo dnf install libxcb-devel
```

**Cargo version too old**

```bash
rustup update
```

### Runtime Issues

**GUI doesn't open**

```bash
echo $DISPLAY
LIBGL_ALWAYS_SOFTWARE=1 cargo run --package campaign_builder
```

**File dialog doesn't appear**

Native dialogs may not work in all Linux environments — this is a known `rfd`
limitation on some setups.

**Campaign won't save**

- Check write permissions in the target directory
- Ensure the campaign path is set (use File → Save As first)
- Check validation errors — some block saving

## Contributing

Campaign Builder follows Antares development guidelines:

1. Read `AGENTS.md` and `sdk/AGENTS.md` for coding standards
2. Follow the SDK architecture in `docs/explanation/sdk_and_campaign_architecture.md`
3. Run all quality gates before submitting
4. Add tests for new features
5. Update this README with any user-visible changes

## Resources

- [SDK Architecture](../../docs/explanation/sdk_and_campaign_architecture.md)
- [Implementation Summary](../../docs/explanation/implementations.md)
- [Antares Core Architecture](../../docs/reference/architecture.md)
- [egui Documentation](https://docs.rs/egui/)
- [RON Format Specification](https://github.com/ron-rs/ron)

## License

Apache-2.0 — same as the Antares core engine.
