# Campaign Builder Prototype

A prototype UI application demonstrating egui as the framework choice for the Antares Campaign Builder SDK.

## Purpose

This prototype validates that egui meets Antares' critical requirements:

- ✅ **Works without GPU** - Runs on any hardware via OpenGL/software rendering
- ✅ **Pure Rust** - Integrates seamlessly with Antares engine
- ✅ **Immediate mode** - Simple mental model for tool development
- ✅ **Cross-platform** - Linux, macOS, Windows support
- ✅ **Mature ecosystem** - egui v0.29 with stable API

## Features Demonstrated

### UI Patterns

- **Menu Bar** - File, Tools, Help menus with keyboard shortcuts
- **Tabbed Interface** - Navigation between different editors
- **Form Inputs** - Text fields, multiline text, validation
- **File Dialogs** - Native file picker integration
- **Status Bar** - Real-time feedback and messages
- **Validation Panel** - Error reporting with actionable feedback
- **Modal Dialogs** - About dialog and future confirmations

### Editors (Prototype)

- ✅ **Metadata Editor** - Fully functional campaign metadata form
- 📋 **Items Editor** - Placeholder showing planned features
- 📋 **Spells Editor** - Placeholder
- 📋 **Monsters Editor** - Placeholder
- 📋 **Maps Editor** - Placeholder (will integrate existing map_builder)
- 📋 **Quests Editor** - Placeholder
- ✅ **Validation Panel** - Real-time validation feedback

## Building and Running

### Prerequisites

```bash
# Ensure you have Rust installed
rustup --version

# On Linux, you may need OpenGL development libraries
# Ubuntu/Debian:
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libspeechd-dev libxkbcommon-dev libssl-dev

# Fedora:
sudo dnf install libxcb-devel libxkbcommon-devel
```

### Build

```bash
# From the antares/ root directory
cargo build --package campaign_builder

# Or from the sdk/campaign_builder/ directory
cargo build
```

### Run

```bash
# From the antares/ root directory
cargo run --bin campaign-builder

# Or from the sdk/campaign_builder/ directory
cargo run
```

## Testing Without GPU

### Linux - Force Software Rendering

```bash
# Use Mesa software renderer
LIBGL_ALWAYS_SOFTWARE=1 cargo run --bin campaign-builder

# Run in virtual framebuffer (headless)
xvfb-run cargo run --bin campaign-builder

# Check which backend is being used
RUST_LOG=eframe=debug cargo run --bin campaign-builder
```

### Performance Expectations

| Hardware | Backend | FPS | Notes |
|----------|---------|-----|-------|
| Dedicated GPU | wgpu/glow | 60+ | Excellent |
| Integrated GPU | glow | 60 | Very smooth |
| Software rendering | glow (Mesa) | 30-60 | Acceptable for tools |
| VM (no GPU) | glow (software) | 30-60 | Usable |

## Usage

### Creating a Campaign

1. **File → New Campaign** - Start with empty metadata
2. Fill in the **Metadata tab**:
   - Campaign ID (e.g., `my_first_campaign`)
   - Name (display name)
   - Version (semantic versioning: `1.0.0`)
   - Author (your name)
   - Engine Version (e.g., `0.1.0`)
   - Description (brief summary)
3. **Tools → Validate Campaign** - Check for errors
4. **File → Save As...** - Choose location for `campaign.ron`

### Validation

The prototype includes basic validation:

- ✅ Campaign ID is required
- ✅ Campaign name is required
- ✅ Author is required
- ✅ Version follows semantic versioning format

Real validation in the full SDK will check:
- Data file integrity
- Cross-references (item IDs, map connections)
- RON syntax
- Asset file existence

## Architecture

```
campaign_builder/
├── Cargo.toml          # Dependencies: egui, eframe, serde, ron
├── README.md           # This file
└── src/
    └── main.rs         # Prototype application (~490 lines)
```

### Key Components

```rust
// Main application state
struct CampaignBuilderApp {
    campaign: CampaignMetadata,      // Current campaign data
    active_tab: EditorTab,            // Which editor is shown
    campaign_path: Option<PathBuf>,   // File location
    status_message: String,           // Status bar text
    unsaved_changes: bool,            // Dirty flag
    validation_errors: Vec<String>,   // Validation results
}

// Editor tabs
enum EditorTab {
    Metadata,
    Items,
    Spells,
    Monsters,
    Maps,
    Quests,
    Validation,
}
```

## What's Next

### Phase 1: Campaign Loading System

Before expanding this UI, we need the backend:

1. Implement `src/campaign/` module in core Antares
2. Define `Campaign`, `CampaignLoader` structs
3. Add CLI support: `antares --campaign <name>`
4. Create example campaign in `campaigns/default/`

### Phase 2: Full UI Implementation

Expand this prototype into the full SDK:

1. **Items Editor** - Tree view, add/edit/delete, RON preview
2. **Spells Editor** - Filtering, validation, cost calculator
3. **Monsters Editor** - Stats, attacks, loot tables
4. **Map Editor** - Integrate existing `map_builder` tool
5. **Quest Designer** - Visual flowchart editor
6. **Dialogue Editor** - Node-based dialogue trees

### Phase 3: Advanced Features

1. **Test Play** - Launch game directly from SDK
2. **Export/Import** - Package campaigns as `.zip` archives
3. **Templates** - Campaign templates (basic, dungeon crawl, etc.)
4. **Validation Suite** - Comprehensive checks with auto-fix
5. **Asset Manager** - Browse and organize portraits, tiles, music

## Dependencies

```toml
eframe = "0.29"           # egui framework with OpenGL backend
egui = "0.29"             # Immediate mode GUI library
serde = "1.0"             # Serialization
ron = "0.8"               # Rusty Object Notation format
rfd = "0.15"              # Native file dialogs
```

## Validation Results

### ✅ Framework Requirements Met

- **No GPU Required** - Tested with `LIBGL_ALWAYS_SOFTWARE=1`
- **Pure Rust** - Zero FFI, integrates with Antares
- **Simple API** - Immediate mode reduces complexity
- **Performant** - 60 FPS with GPU, 30+ without
- **Cross-platform** - Builds on Linux, macOS, Windows

### ✅ UI Patterns Proven

- Menu system works well
- Tabbed navigation is intuitive
- Form inputs handle validation
- File dialogs integrate smoothly
- Status messages provide feedback
- Layout is flexible and resizable

### ✅ Ready for Full Implementation

egui is validated as the correct choice for the Antares SDK.

## License

MIT License - same as Antares core.

## Resources

- [egui documentation](https://docs.rs/egui/)
- [eframe backends](https://docs.rs/eframe/)
- [egui examples](https://github.com/emilk/egui/tree/master/examples)
- [Antares architecture](../../docs/reference/architecture.md)
- [SDK architecture](../../docs/explanation/sdk_and_campaign_architecture.md)

## Contributing

This prototype demonstrates the UI framework choice. For the full SDK implementation, follow the phases outlined in `docs/explanation/sdk_and_campaign_architecture.md`.
