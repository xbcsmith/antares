# Campaign Builder - Phase 2: Foundation

A functional campaign editor for creating Antares RPG campaigns. Phase 2 delivers full metadata editing, real file I/O, validation UI, and data editor placeholders.

## Status: Phase 7.1 Complete ✅

- ✅ **Phase 0**: Framework validation (egui confirmed)
- ✅ **Phase 1**: Core campaign system (backend)
- ✅ **Phase 2**: Campaign Builder Foundation
- ✅ **Phase 3**: Data Editors (Items, Spells, Monsters)
- ✅ **Phase 4A**: Quest Editor Integration (backend)
- ✅ **Phase 6**: Map Editor Enhancement
- ✅ **Phase 7**: Quest & Dialogue Backend CRUD Operations
- ✅ **Phase 7.1**: Quest Stage & Objective Editing UI ⭐ **NEW!**
- 🔲 **Phase 7.7**: Dialogue Editor UI Integration
- 🔲 **Phase 8**: Asset System Enhancements
- 🔲 **Phase 9**: Testing & Distribution

## Features

### ✅ Implemented in Phase 2

#### Full Metadata Editor

- **Basic Info**: Campaign ID, name, version, author, description, engine version
- **Starting Conditions**: Map, position, direction, gold, food
- **Party Settings**: Max party size, max roster size
- **Difficulty**: Easy/Normal/Hard/Brutal with permadeath and multiclassing options
- **Level Range**: Starting level and max level configuration
- **Data Paths**: Configurable file paths for all game data

#### Real File I/O

- **Save/Load**: RON format serialization with pretty printing
- **File Dialogs**: Native file picker integration (Save As, Open)
- **Error Handling**: Clear error messages for I/O failures
- **Auto-format**: Clean, human-readable campaign.ron output

#### Enhanced Validation

- **Error Detection**: Required fields, format validation, range checks
- **Warning System**: Non-critical issues flagged separately
- **Color-Coded Display**: Red for errors, orange for warnings
- **Actionable Feedback**: Tells you exactly what to fix and where

#### Unsaved Changes Protection

- **Change Tracking**: Real-time detection of modifications
- **Visual Indicator**: Status bar shows saved/unsaved state
- **Warning Dialog**: Prevents accidental data loss
- **Three-Option Flow**: Save, Don't Save, or Cancel before destructive actions

#### File Structure Browser

- **Tree View**: Browse campaign directory hierarchy
- **Auto-Update**: Refreshes after save operations
- **Visual Icons**: Directories (📁) and files (📄) clearly marked
- **Manual Refresh**: Tools menu option to rescan files

#### ✅ Phase 7.1: Interactive Quest Stage & Objective Editing (NEW!)

**Quest Stages Editor**:

- **Edit Stages**: Click ✏️ button to open modal dialog with stage form
- **Delete Stages**: Click 🗑️ button for immediate removal
- **Stage Fields**: Edit stage number, name, description, and "require all objectives" flag
- **Collapsible View**: Expand/collapse stages to see objectives

**Quest Objectives Editor**:

- **Edit Objectives**: Click ✏️ button to open dynamic objective editor modal
- **Delete Objectives**: Click 🗑️ button for immediate removal
- **Type Selector**: Dropdown with 7 objective types (Kill Monsters, Collect Items, Reach Location, Talk To NPC, Deliver Item, Escort NPC, Custom Flag)
- **Dynamic Forms**: Form fields change automatically based on selected objective type
- **Add Objectives**: Click ➕ button in objectives section
- **Type Conversion**: Change objective type during editing - form updates instantly

**UI Features**:

- Modal dialogs for focused editing without navigation
- Save/Cancel buttons for explicit commit/discard
- Hover tooltips on all action buttons
- Immediate unsaved changes tracking
- Clean inline controls that don't clutter the list view

#### Data Editor Placeholders

- **Items Editor** - Ready for Phase 3 implementation
- **Spells Editor** - Ready for Phase 3 implementation
- **Monsters Editor** - Ready for Phase 3 implementation
- **Maps Editor** - Ready for Phase 4 integration
- **Dialogues Editor** - Ready for Phase 7.7 UI integration

### 📋 Coming in Phase 3

- Item database editor (weapons, armor, consumables)
- Spell database editor (cleric and sorcerer spells)
- Monster database editor (stats, loot, special abilities)
- Real-time data validation with cross-references
- Import/export data utilities

## Installation

### Prerequisites

```bash
# Rust toolchain (1.70+)
rustup --version

# Linux: OpenGL development libraries (if not already installed)
# Ubuntu/Debian:
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev

# Fedora:
sudo dnf install libxcb-devel libxkbcommon-devel
```

### Building

```bash
# From antares/ root directory
cargo build --release --package campaign_builder

# Or from sdk/campaign_builder/ directory
cargo build --release
```

### Running

```bash
# From antares/ root
cargo run --release --package campaign_builder --bin campaign-builder

# Or from sdk/campaign_builder/
cargo run --release --package campaign_builder

# Binary location after build:
# target/release/campaign-builder
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
   - Description: Brief summary of your campaign
4. **Config Tab**: Configure game rules
   - Starting map ID (e.g., `starter_town`)
   - Starting position, gold, food
   - Party/roster size limits
   - Difficulty and rules (permadeath, multiclassing)
   - Level range
   - Data file paths (use defaults or customize)
5. **Validation**: Click **Tools → Validate Campaign**
   - Fix any errors (red ❌)
   - Review warnings (orange ⚠️)
6. **Save**: **File → Save As...** and choose location
   - Creates `campaign.ron` file
   - Sets campaign directory for future saves

### Opening an Existing Campaign

1. **File → Open Campaign...** (or Ctrl+O)
2. Navigate to campaign directory
3. Select `campaign.ron` file
4. Edit metadata or configuration
5. **File → Save** (or Ctrl+S) to update

### Validation Rules

#### Errors (must fix):

- Campaign ID required and alphanumeric + underscores
- Campaign name required
- Version must follow X.Y.Z format
- Starting map required
- Max roster size must be >= max party size
- Starting level must be between 1 and max level
- All data file paths required

#### Warnings (recommended):

- Author name recommended
- Engine version should follow X.Y.Z format
- Max party size should be 1-10
- Data files should use .ron extension

### File Structure Browser

The **Files Tab** shows your campaign directory:

```
my_campaign/
├── campaign.ron         # Metadata (what you're editing)
├── README.md            # Campaign documentation
└── data/
    ├── items.ron        # Item definitions
    ├── spells.ron       # Spell definitions
    ├── monsters.ron     # Monster definitions
    ├── classes.ron      # Class definitions
    ├── races.ron        # Race definitions
    ├── quests.ron       # Quest definitions
    ├── dialogue.ron     # Dialogue trees
    └── maps/
        ├── town.ron
        └── dungeon.ron
```

Use **Tools → Refresh File Tree** to update after external changes.

## Keyboard Shortcuts

| Shortcut     | Action        |
| ------------ | ------------- |
| Ctrl+N       | New Campaign  |
| Ctrl+O       | Open Campaign |
| Ctrl+S       | Save Campaign |
| Ctrl+Shift+S | Save As...    |
| Ctrl+W       | Exit          |

## Testing Without GPU

Campaign Builder works on any hardware, even without a GPU:

```bash
# Linux: Force software rendering
LIBGL_ALWAYS_SOFTWARE=1 cargo run --release

# Test in virtual framebuffer (headless server)
xvfb-run cargo run --release

# Debug backend selection
RUST_LOG=eframe=debug cargo run --release
```

### Performance

| Hardware           | Expected FPS | Status     |
| ------------------ | ------------ | ---------- |
| Dedicated GPU      | 60+          | Excellent  |
| Integrated GPU     | 60           | Very Good  |
| Software rendering | 30-60        | Acceptable |
| VM (no GPU)        | 30-60        | Usable     |

## Campaign.ron Format

Example output from Campaign Builder:

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

- **Framework**: egui v0.29 (immediate mode GUI)
- **Backend**: eframe with glow (OpenGL)
- **Serialization**: RON (Rusty Object Notation)
- **File Dialogs**: rfd (native OS dialogs)
- **Error Handling**: thiserror

### Code Structure

```
sdk/campaign_builder/
├── Cargo.toml           # Dependencies and metadata
├── README.md            # This file
├── QUICKSTART.md        # Quick reference guide
├── FRAMEWORK_DECISION.md # egui vs iced comparison
└── src/
    └── main.rs          # Application (1717 lines)
                         # - CampaignMetadata struct
                         # - Validation system
                         # - File I/O handlers
                         # - UI implementation
                         # - 18 unit tests
```

### Key Components

```rust
// Campaign metadata (27 fields)
struct CampaignMetadata {
    id: String,
    name: String,
    version: String,
    // ... (24 more fields)
}

// Validation with severity levels
struct ValidationError {
    severity: Severity,  // Error or Warning
    message: String,
}

// File I/O error handling
enum CampaignError {
    Io(std::io::Error),
    Serialization(ron::Error),
    Deserialization(ron::error::SpannedError),
    NoPath,
}

// UI state management
struct CampaignBuilderApp {
    campaign: CampaignMetadata,
    active_tab: EditorTab,
    campaign_path: Option<PathBuf>,
    unsaved_changes: bool,
    validation_errors: Vec<ValidationError>,
    // ... (more state)
}
```

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_validation_all_pass
```

### Test Coverage

18 unit tests covering:

- ✅ Default values and initialization
- ✅ Validation rules (12 tests)
- ✅ File I/O error handling
- ✅ RON serialization/deserialization
- ✅ UI state management

```bash
Test Results: 18 passed, 0 failed (100%)
```

### Quality Gates

All quality checks passing:

```bash
✅ cargo fmt --all                                      # Code formatted
✅ cargo check --all-targets --all-features            # Compiles
✅ cargo clippy --all-targets --all-features -- -D warnings  # Zero warnings
✅ cargo test --all-features                           # 18/18 pass
✅ cargo build --release                               # Release build
```

## Roadmap

### Phase 3: Data Editors (Next)

- Item editor with add/edit/delete
- Spell editor with school filtering
- Monster editor with stats and loot
- Cross-reference validation
- Data import/export utilities

### Phase 4: Map Editor Integration

- Launch map_builder from UI
- Map preview panel
- Event editor integration
- Map interconnection manager

### Phase 5: Quest & Dialogue Tools

- Visual quest designer
- Objective chain editor
- Dialogue tree editor
- Prerequisite system

### Phase 6: Testing & Distribution

- Campaign packager (.zip export)
- Test play integration
- Template campaigns
- Asset manager
- Documentation generator

## Known Limitations

### Phase 2 Scope

- Data editors are placeholders (Phase 3)
- Map editor not integrated (Phase 4)
- Quest editor not implemented (Phase 5)
- No test play functionality yet
- No campaign export/import yet

### Technical Constraints

- Single campaign edit at a time
- No undo/redo (planned for Phase 3+)
- File tree depth limited to 2 levels
- No asset preview (images, sounds)

## Troubleshooting

### Build Errors

**Problem**: Missing OpenGL libraries

```bash
# Ubuntu/Debian
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev

# Fedora
sudo dnf install libxcb-devel
```

**Problem**: Cargo version too old

```bash
rustup update
```

### Runtime Issues

**Problem**: GUI doesn't open

- Check display environment variable: `echo $DISPLAY`
- Try software rendering: `LIBGL_ALWAYS_SOFTWARE=1 cargo run`

**Problem**: File dialog doesn't appear

- Native dialogs may not work in all environments
- This is a known rfd limitation on some Linux setups

**Problem**: Campaign won't save

- Check write permissions in target directory
- Ensure campaign path is set (use Save As first)
- Check validation errors - some block saving

## Contributing

Campaign Builder follows Antares development guidelines:

1. Read `AGENTS.md` for coding standards
2. Follow the SDK architecture in `docs/explanation/sdk_and_campaign_architecture.md`
3. Run quality gates before submitting
4. Add tests for new features
5. Update this README with changes

## Resources

- [SDK Architecture Document](../../docs/explanation/sdk_and_campaign_architecture.md)
- [Implementation Summary](../../docs/explanation/implementations.md)
- [Antares Core Architecture](../../docs/reference/architecture.md)
- [egui Documentation](https://docs.rs/egui/)
- [RON Format Specification](https://github.com/ron-rs/ron)

## License

Apache-2.0 - Same as Antares core engine.

---

**Phase 2 Complete**: Full metadata editor, validation UI, file I/O, and placeholders ready for Phase 3 data editors.
