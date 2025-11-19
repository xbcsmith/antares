# Phase 13: Campaign Builder GUI - Distribution Tools Implementation

**Implementation Date:** 2025-01-24
**Phase:** 13 (Distribution Tools)
**Status:** ✅ COMPLETE

---

## Overview

Phase 13 implements distribution tools for the Antares Campaign Builder, enabling creators to package, test, and share their campaigns. This phase adds campaign packaging, test play integration, asset management, and campaign installation functionality.

---

## Deliverables

### 13.1 Campaign Packager Integration ✅

**File:** `sdk/campaign_builder/src/packager.rs`

**Features Implemented:**

- **Export Wizard State Machine**: Multi-step packaging process with validation, file selection, metadata confirmation, and export settings
- **Version Management**: Semantic versioning parser and increment utilities (major, minor, patch)
- **Package File Selection**: Automatic detection of campaign files to include (data, maps, RON files)
- **Integration with SDK**: Uses `antares::sdk::campaign_packager` for ZIP creation
- **Export Methods**: `export_campaign()` and `import_campaign()` on `CampaignBuilderApp`

**Key Types:**

```rust
pub struct ExportWizard {
    pub current_step: ExportWizardStep,
    pub validation_passed: bool,
    pub selected_files: Vec<PathBuf>,
    pub output_path: Option<PathBuf>,
    pub compression_level: u8,
    pub include_readme: bool,
    pub include_all_maps: bool,
    pub progress_message: String,
    pub export_complete: bool,
    pub export_error: Option<String>,
}

pub enum ExportWizardStep {
    Validation,
    FileSelection,
    Metadata,
    Settings,
    Exporting,
    Complete,
}

pub enum VersionIncrement {
    Major,  // 1.0.0 -> 2.0.0
    Minor,  // 1.0.0 -> 1.1.0
    Patch,  // 1.0.0 -> 1.0.1
}
```

**Version Utilities:**

```rust
pub fn parse_version(version: &str) -> Option<(u32, u32, u32)>;
pub fn increment_version(version: &str, increment: VersionIncrement) -> String;
```

**Export Process:**

1. Validate campaign (check for errors)
2. Save campaign to disk
3. Create packager with compression level
4. Package campaign directory into .zip
5. Update status message

**Import Process:**

1. Select .zip package file
2. Choose installation directory
3. Extract and validate campaign
4. Update status message

### 13.2 Test Play Integration ✅

**File:** `sdk/campaign_builder/src/test_play.rs`

**Features Implemented:**

- **Test Play Session Management**: Launch, monitor, and terminate game process
- **Output Capture**: Capture stdout/stderr from game for debugging
- **Debug Mode Support**: Launch game with `--debug` flag
- **Process Lifecycle**: Automatic cleanup on session drop
- **Configuration**: Flexible test play configuration with validation

**Key Types:**

```rust
pub struct TestPlaySession {
    process: Option<Child>,
    campaign_id: String,
    output_log: Vec<String>,
    error_log: Vec<String>,
    start_time: std::time::Instant,
    is_active: bool,
}

pub struct TestPlayConfig {
    pub game_executable: PathBuf,
    pub debug_mode: bool,
    pub auto_save: bool,
    pub validate_first: bool,
    pub max_log_lines: usize,
}
```

**Methods:**

```rust
impl TestPlaySession {
    pub fn new(campaign_id: String) -> Self;
    pub fn launch(&mut self, game_executable: &PathBuf, debug_mode: bool) -> Result<(), std::io::Error>;
    pub fn is_running(&mut self) -> bool;
    pub fn terminate(&mut self) -> Result<(), std::io::Error>;
    pub fn output_log(&self) -> &[String];
    pub fn error_log(&self) -> &[String];
    pub fn elapsed(&self) -> std::time::Duration;
}

impl CampaignBuilderApp {
    pub fn launch_test_play(&mut self, config: &TestPlayConfig) -> Result<TestPlaySession, CampaignError>;
    pub fn can_launch_test_play(&self, config: &TestPlayConfig) -> bool;
}
```

**Launch Process:**

1. Auto-save campaign if enabled
2. Run validation if enabled (block if errors found)
3. Check game executable exists
4. Create test play session
5. Spawn game process with `--campaign <id>` flag
6. Capture output and return session handle

### 13.3 Asset Manager ✅

**File:** `sdk/campaign_builder/src/asset_manager.rs`

**Features Implemented:**

- **Asset Type Classification**: Automatic detection of asset types from file extensions and paths
- **Directory Scanning**: Recursive scan of campaign directory for assets
- **Asset Organization**: Add, remove, and move assets between subdirectories
- **Reference Tracking**: Mark assets as referenced/unreferenced by campaign data
- **Size Tracking**: Track individual and total asset sizes with human-readable formatting
- **Type Filtering**: Query assets by type (tileset, portrait, music, sound, docs, data)

**Asset Types:**

```rust
pub enum AssetType {
    Tileset,       // .png, .jpg in tilesets/
    Portrait,      // .png, .jpg in portraits/
    Music,         // .mp3, .ogg, .flac, .midi
    Sound,         // .wav, .ogg in sounds/
    Documentation, // .md, .txt, .pdf
    Data,          // .ron, .yaml, .json, .toml
    Other,         // Unknown types
}
```

**Key Types:**

```rust
pub struct Asset {
    pub path: PathBuf,
    pub asset_type: AssetType,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
    pub is_referenced: bool,
}

pub struct AssetManager {
    campaign_dir: PathBuf,
    assets: HashMap<PathBuf, Asset>,
    total_size: u64,
}
```

**Methods:**

```rust
impl AssetManager {
    pub fn new(campaign_dir: PathBuf) -> Self;
    pub fn scan_directory(&mut self) -> Result<(), std::io::Error>;
    pub fn add_asset(&mut self, source_path: &Path, dest_subdir: &str) -> Result<PathBuf, std::io::Error>;
    pub fn remove_asset(&mut self, asset_path: &Path) -> Result<(), std::io::Error>;
    pub fn move_asset(&mut self, asset_path: &Path, dest_subdir: &str) -> Result<PathBuf, std::io::Error>;
    pub fn assets(&self) -> &HashMap<PathBuf, Asset>;
    pub fn assets_by_type(&self, asset_type: AssetType) -> Vec<&Asset>;
    pub fn unreferenced_assets(&self) -> Vec<&Asset>;
    pub fn mark_referenced(&mut self, asset_path: &Path, referenced: bool);
    pub fn total_size(&self) -> u64;
    pub fn total_size_string(&self) -> String;
    pub fn asset_count(&self) -> usize;
    pub fn asset_count_by_type(&self, asset_type: AssetType) -> usize;
}
```

**Asset Type Detection:**

- **Images** (.png, .jpg, .bmp, .gif): Classified as Tileset or Portrait based on parent directory
- **Audio** (.mp3, .ogg, .wav, .flac, .midi): Classified as Music or Sound based on parent directory
- **Documentation** (.md, .txt, .pdf): Classified as Documentation
- **Data** (.ron, .yaml, .json, .toml): Classified as Data
- **Other**: Unknown file types

**Suggested Directory Structure:**

```
campaign_dir/
├── assets/
│   ├── tilesets/
│   ├── portraits/
│   ├── music/
│   └── sounds/
├── data/
├── docs/
└── maps/
```

### 13.4 Campaign Installation System ✅

**Features:**

- **Import Campaign**: Uses `CampaignPackager::install_package()` to extract .zip archives
- **Conflict Detection**: Checks for existing campaigns with same ID (handled by file system)
- **Validation**: Campaigns are validated during import process
- **Error Handling**: Comprehensive error messages for import failures

**Integration:**

```rust
impl CampaignBuilderApp {
    pub fn import_campaign(
        &mut self,
        package_path: PathBuf,
        install_dir: PathBuf,
    ) -> Result<(), CampaignError>;
}
```

---

## Main Application Integration

### State Management

Added to `CampaignBuilderApp`:

```rust
// Phase 13: Distribution tools state
export_wizard: Option<packager::ExportWizard>,
test_play_session: Option<test_play::TestPlaySession>,
test_play_config: test_play::TestPlayConfig,
asset_manager: Option<asset_manager::AssetManager>,
show_export_dialog: bool,
show_test_play_panel: bool,
show_asset_manager: bool,
```

### UI Integration

**New Tab:** `EditorTab::Assets`

**Methods Added:**

```rust
impl CampaignBuilderApp {
    fn show_assets_editor(&mut self, ui: &mut egui::Ui);
    fn show_dialogues_editor(&mut self, ui: &mut egui::Ui);
    fn save_dialogues_to_file(&self, path: &std::path::Path) -> Result<(), CampaignError>;
}
```

**Asset Manager UI:**

- Asset statistics (total count, total size)
- Refresh button to rescan directory
- Type filters (Tileset, Portrait, Music, Sound, Documentation, Data)
- Asset list with path, type, size, and reference status
- Unreferenced assets warning

**Dialogue Editor UI:**

- Integrated `DialogueEditorWidget` from Phase 12
- Auto-save on changes
- Save to `campaign.dialogue_file`

---

## Testing

### Unit Tests Added

**Packager Module (packager.rs):** 18 tests

- Export wizard creation and state machine
- Step progression and validation
- Can proceed logic
- Version parsing (valid and invalid)
- Version incrementing (major, minor, patch)
- Export wizard flags and configuration

**Test Play Module (test_play.rs):** 14 tests

- Session creation and lifecycle
- Logging (output and error logs)
- Clear logs functionality
- Elapsed time tracking
- Configuration defaults and customization
- Process management without actual process

**Asset Manager Module (asset_manager.rs):** 21 tests

- Asset type display names and subdirectories
- Asset type detection from paths (images, audio, data, docs)
- Asset creation and size formatting (B, KB, MB, GB)
- Asset manager creation and state
- Reference tracking (mark referenced/unreferenced)
- Unreferenced asset queries
- Asset filtering by type
- Asset count by type
- Type equality

**Total Tests:** 53 new tests (all passing)

### Test Coverage

- Export wizard state machine: ✅ Complete
- Version parsing and incrementing: ✅ Complete
- Test play session lifecycle: ✅ Complete
- Asset type detection: ✅ Complete
- Asset size formatting: ✅ Complete
- Asset reference tracking: ✅ Complete
- Asset filtering and querying: ✅ Complete

---

## Architecture Compliance

### Data Structure Integrity

- **No modifications to core domain types**: All new types are SDK/editor-specific
- **PathBuf usage**: Consistent use for file paths
- **Error handling**: Proper `Result<T, E>` returns with descriptive errors
- **Type safety**: Strong typing for asset types, version components, wizard steps

### Module Organization

```
sdk/campaign_builder/src/
├── asset_manager.rs   (Phase 13) ✅
├── dialogue_editor.rs (Phase 12)
├── map_editor.rs      (Phase 11)
├── packager.rs        (Phase 13) ✅
├── quest_editor.rs    (Phase 12)
├── test_play.rs       (Phase 13) ✅
└── main.rs            (updated for Phase 13) ✅
```

### Separation of Concerns

- **Packager**: Export/import logic, version management
- **Test Play**: Game process lifecycle management
- **Asset Manager**: File scanning, organization, reference tracking
- **Main App**: UI rendering, state coordination

---

## Quality Gates

All quality checks passed:

```bash
✅ cargo fmt --all
✅ cargo check --all-targets --all-features
✅ cargo clippy --all-targets --all-features -- -D warnings
✅ cargo test --all-features (212 passed)
```

---

## Usage Examples

### Exporting a Campaign

```rust
use antares::sdk::campaign_builder::packager::{ExportWizard, VersionIncrement, increment_version};
use std::path::PathBuf;

// Create export wizard
let mut wizard = ExportWizard::new();

// Run validation
app.validate_campaign();
wizard.validation_passed = !app.validation_errors.iter().any(|e| e.severity == Severity::Error);

// Select files
wizard.selected_files = app.get_package_files();

// Set output path
wizard.output_path = Some(PathBuf::from("/path/to/my_campaign_v1.0.0.zip"));

// Increment version
let new_version = increment_version(&app.campaign.version, VersionIncrement::Patch);
app.campaign.version = new_version;

// Export
app.export_campaign(wizard.output_path.unwrap(), wizard.compression_level)?;
```

### Launching Test Play

```rust
use antares::sdk::campaign_builder::test_play::{TestPlayConfig, TestPlaySession};
use std::path::PathBuf;

// Configure test play
let config = TestPlayConfig {
    game_executable: PathBuf::from("/usr/local/bin/antares"),
    debug_mode: true,
    auto_save: true,
    validate_first: true,
    max_log_lines: 1000,
};

// Launch game
let session = app.launch_test_play(&config)?;

// Monitor session
while session.is_running() {
    // Check logs
    for line in session.output_log() {
        println!("{}", line);
    }
    std::thread::sleep(std::time::Duration::from_millis(100));
}
```

### Managing Assets

```rust
use antares::sdk::campaign_builder::asset_manager::{AssetManager, AssetType};
use std::path::PathBuf;

// Create asset manager
let mut manager = AssetManager::new(PathBuf::from("/path/to/campaign"));
manager.scan_directory()?;

// Add a tileset
let tileset_path = PathBuf::from("/home/user/dungeon.png");
let added_path = manager.add_asset(&tileset_path, "assets/tilesets")?;

// Mark as referenced
manager.mark_referenced(&added_path, true);

// Get statistics
println!("Total assets: {}", manager.asset_count());
println!("Total size: {}", manager.total_size_string());

// Find unreferenced assets
let unreferenced = manager.unreferenced_assets();
for asset in unreferenced {
    println!("Unused: {}", asset.path.display());
}

// Filter by type
let tilesets = manager.assets_by_type(AssetType::Tileset);
println!("Tilesets: {}", tilesets.len());
```

---

## Known Limitations

### Current Phase Limitations

1. **No UI for Export Wizard**: Export wizard state exists but UI rendering not implemented (requires egui integration)
2. **No Test Play Output Display**: Test play session logs captured but not displayed in UI
3. **No Asset Upload UI**: Asset manager has add/remove/move logic but no file picker integration
4. **No Process Monitoring**: Test play session checks if running but doesn't poll logs in real-time
5. **No Dependency Resolution**: Campaign installation doesn't check for missing dependencies

### Deferred to Future Phases

1. **Export Wizard Multi-Step UI**: Will be added when egui dialog system is implemented
2. **Test Play Log Viewer**: Real-time log display panel (requires background polling)
3. **Asset Preview**: Image thumbnails, audio playback (requires image/audio libraries)
4. **Drag-and-Drop Upload**: File picker integration for asset upload
5. **Campaign Marketplace**: Online campaign sharing and discovery

---

## Future Enhancements (Phase 15)

Phase 15 will add:

- **Advanced Export Options**: Selective file inclusion, custom compression
- **Test Play Hot-Reload**: Auto-reload campaign on save during test play
- **Asset Dependencies**: Track which maps/events use which assets
- **Automated Testing**: Run automated test suite on campaign before export
- **Campaign Templates**: Pre-configured campaign packages for quick start
- **Collaborative Features**: Multi-user editing, version control integration

---

## Dependencies

### SDK Dependencies

- `antares::sdk::campaign_packager`: ZIP packaging and installation
- `antares::sdk::validation`: Campaign validation before export
- `antares::domain::dialogue::DialogueTree`: Dialogue data structures
- `antares::domain::quest::Quest`: Quest data structures

### External Crates

- `std::process`: Process spawning for test play
- `std::fs`: File system operations for assets
- `std::collections::HashMap`: Asset storage
- `serde`: Serialization for campaign data

---

## Conclusion

Phase 13 successfully implements the distribution tools foundation for the Antares Campaign Builder. Campaign creators can now:

- ✅ Package campaigns as distributable .zip archives
- ✅ Manage semantic versioning with increment utilities
- ✅ Launch the game for test play sessions
- ✅ Manage campaign assets (images, sounds, music, tilesets)
- ✅ Track asset usage and find unreferenced files
- ✅ Import campaigns from .zip packages

The implementation provides robust state management and utilities, with UI integration points ready for Phase 15 polish and advanced features.

**Next Steps:**

1. Implement export wizard UI with egui dialogs
2. Add test play log viewer panel
3. Implement asset upload with file picker
4. Add asset preview capabilities
5. Begin Phase 14: Game Engine Campaign Integration

**All architecture compliance rules followed. All quality gates passed. Phase 13 complete.**
