# How to Use Sprite Browser Functions in Campaign Builder

**Status**: Phase 5 Guide
**Target Audience**: Campaign Builder Developers, Map Editors
**Difficulty**: Intermediate

## Overview

Phase 5 provides a set of sprite registry functions that enable Campaign Builder to implement sprite selection UI. This guide explains how to use these functions to build sprite browsers, selection dialogs, and preview systems.

## Quick Start

### Loading Available Sprites

```rust
use antares::sdk::map_editor::{browse_sprite_sheets, get_sprites_for_sheet};

// Get all available sprite sheets
let sheets = browse_sprite_sheets()?;
for (key, path) in sheets {
    println!("{}: {}", key, path);
}

// Get sprites in a specific sheet
let sprites = get_sprites_for_sheet("npcs_town")?;
for (index, name) in sprites {
    println!("[{}] {}", index, name);
}
```

### Searching for Sprites

```rust
use antares::sdk::map_editor::{search_sprites, suggest_sprite_sheets};

// Find sprites by name
let results = search_sprites("guard")?;
for (sheet, index, name) in results {
    println!("{} [{}]: {}", sheet, index, name);
}

// Suggest sprite sheets
let suggestions = suggest_sprite_sheets("npc")?;
for (key, path) in suggestions {
    println!("{}: {}", key, path);
}
```

## Building a Sprite Browser Panel

### Step 1: Load Registry in UI Setup

```rust
// In Campaign Builder initialization
use antares::sdk::map_editor::browse_sprite_sheets;

pub struct SpriteBrowserState {
    sheets: Vec<(String, String)>,
    selected_sheet: Option<String>,
    selected_sprite: Option<u32>,
}

impl SpriteBrowserState {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let sheets = browse_sprite_sheets()?;
        Ok(Self {
            sheets,
            selected_sheet: None,
            selected_sprite: None,
        })
    }
}
```

### Step 2: Display Sprite Sheets List

```rust
// In egui render loop
use egui::{ComboBox, Ui};
use antares::sdk::map_editor::get_sprites_for_sheet;

pub fn show_sprite_browser(
    ui: &mut Ui,
    state: &mut SpriteBrowserState,
) -> Result<(), Box<dyn std::error::Error>> {
    // Sheet selector
    let mut selected = state.selected_sheet.clone().unwrap_or_default();
    ComboBox::from_label("Sprite Sheet")
        .selected_text(&selected)
        .show_ui(ui, |ui| {
            for (key, path) in &state.sheets {
                ui.selectable_value(&mut selected, key.clone(), 
                    format!("{}: {}", key, path));
            }
        });
    
    state.selected_sheet = Some(selected.clone());
    
    // Load sprites for selected sheet
    if let Some(sheet_key) = &state.selected_sheet {
        let sprites = get_sprites_for_sheet(sheet_key)?;
        
        ui.label(format!("Available Sprites ({}):", sprites.len()));
        
        for (index, name) in sprites {
            if ui.selectable_label(
                state.selected_sprite == Some(index),
                format!("[{}] {}", index, name)
            ).clicked() {
                state.selected_sprite = Some(index);
            }
        }
    }
    
    Ok(())
}
```

### Step 3: Create Sprite Reference from Selection

```rust
use antares::domain::world::SpriteReference;
use antares::sdk::map_editor::browse_sprite_sheets;

pub fn get_selected_sprite_reference(
    state: &SpriteBrowserState,
) -> Result<Option<SpriteReference>, Box<dyn std::error::Error>> {
    let sheets = browse_sprite_sheets()?;
    
    match (&state.selected_sheet, state.selected_sprite) {
        (Some(sheet_key), Some(sprite_index)) => {
            // Find the texture path for this sheet
            if let Some((_, texture_path)) = sheets.iter().find(|(k, _)| k == sheet_key) {
                Ok(Some(SpriteReference {
                    sheet_path: texture_path.clone(),
                    sprite_index,
                    animation: None, // Phase 6: animation editor
                }))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}
```

## Building a Tile Inspector with Sprite Selection

### Basic Tile Inspector Panel

```rust
use antares::domain::world::{Tile, TileVisualMetadata, SpriteReference};
use antares::sdk::map_editor::get_sprites_for_sheet;
use egui::Ui;

pub fn show_tile_sprite_editor(
    ui: &mut Ui,
    tile: &mut Tile,
    current_sheet: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    ui.group(|ui| {
        ui.label("Sprite Configuration");
        
        // Show current sprite
        if let Some(sprite) = &tile.visual.sprite {
            ui.label(format!("Current: {} [{}]", 
                sprite.sheet_path, 
                sprite.sprite_index));
            
            if ui.button("Remove Sprite").clicked() {
                tile.visual.sprite = None;
            }
        } else {
            ui.label("No sprite selected");
        }
        
        // Sprite grid picker
        let sprites = get_sprites_for_sheet(current_sheet)?;
        
        ui.label("Select Sprite:");
        
        // Show as grid (4 columns for compact display)
        let col_count = 4;
        for (idx, chunk) in sprites.chunks(col_count).enumerate() {
            ui.horizontal(|ui| {
                for (sprite_index, sprite_name) in chunk {
                    let is_selected = tile.visual.sprite.as_ref()
                        .map(|s| s.sprite_index == *sprite_index)
                        .unwrap_or(false);
                    
                    if ui.selectable_label(is_selected,
                        format!("[{}]", sprite_index)).clicked() {
                        tile.visual.sprite = Some(SpriteReference {
                            sheet_path: format!("sprites/{}.png", current_sheet),
                            sprite_index: *sprite_index,
                            animation: None,
                        });
                    }
                }
            });
        }
    });
    
    Ok(())
}
```

## Implementing Sprite Search

### Search Dialog

```rust
use antares::sdk::map_editor::search_sprites;
use egui::{TextEdit, Ui};

pub struct SpriteSearchState {
    search_text: String,
    search_results: Vec<(String, u32, String)>, // (sheet, index, name)
}

impl SpriteSearchState {
    pub fn new() -> Self {
        Self {
            search_text: String::new(),
            search_results: Vec::new(),
        }
    }
    
    pub fn update_search(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.search_text.is_empty() {
            self.search_results.clear();
        } else {
            self.search_results = search_sprites(&self.search_text)?;
        }
        Ok(())
    }
}

pub fn show_sprite_search(
    ui: &mut Ui,
    search_state: &mut SpriteSearchState,
) -> Result<(), Box<dyn std::error::Error>> {
    ui.horizontal(|ui| {
        ui.label("Search Sprites:");
        let response = TextEdit::singleline(&mut search_state.search_text)
            .desired_width(f32::INFINITY)
            .show(ui);
        
        if response.changed {
            search_state.update_search()?;
        }
    });
    
    if !search_state.search_results.is_empty() {
        ui.separator();
        ui.label(format!("Results ({})", search_state.search_results.len()));
        
        for (sheet, index, name) in &search_state.search_results {
            if ui.selectable_label(false, 
                format!("{} [{}]: {}", sheet, index, name)).clicked() {
                // User selected this sprite
                // Return (sheet, index) to caller
                return Ok(());
            }
        }
    }
    
    Ok(())
}
```

## Getting Grid Dimensions for Preview

### Calculating Grid Layout

```rust
use antares::sdk::map_editor::get_sprite_sheet_dimensions;

pub fn calculate_preview_grid(
    sheet_key: &str,
) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let (cols, rows) = get_sprite_sheet_dimensions(sheet_key)?;
    
    println!("Sprite sheet '{}' has {}x{} grid", sheet_key, cols, rows);
    println!("Total sprites: {}", cols * rows);
    
    Ok((cols, rows))
}
```

### Rendering Sprite Coordinates

```rust
use antares::sdk::map_editor::get_sprite_sheet_dimensions;

pub fn get_sprite_grid_position(
    sheet_key: &str,
    sprite_index: u32,
) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let (cols, _) = get_sprite_sheet_dimensions(sheet_key)?;
    
    let col = sprite_index % cols;
    let row = sprite_index / cols;
    
    Ok((col, row))
}
```

## Validating Sprite Selections

### Checking if Sheet Exists

```rust
use antares::sdk::map_editor::has_sprite_sheet;

pub fn validate_sprite_sheet(sheet_key: &str) -> Result<bool, Box<dyn std::error::Error>> {
    has_sprite_sheet(sheet_key)
}

// Usage in tile inspector
if let Some(sprite) = &tile.visual.sprite {
    if !validate_sprite_sheet(&sprite.sheet_path.replace("sprites/", "").replace(".png", ""))? {
        eprintln!("Warning: sprite sheet not found");
    }
}
```

## Complete Example: Sprite Selection Widget

```rust
use antares::domain::world::{Tile, SpriteReference};
use antares::sdk::map_editor::{
    browse_sprite_sheets, get_sprites_for_sheet, search_sprites,
};
use egui::{ComboBox, TextEdit, Ui};

pub struct SpriteSelectionWidget {
    sheets: Vec<(String, String)>,
    selected_sheet: String,
    search_text: String,
    show_search: bool,
}

impl SpriteSelectionWidget {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let sheets = browse_sprite_sheets()?;
        let selected_sheet = sheets.first().map(|(k, _)| k.clone()).unwrap_or_default();
        
        Ok(Self {
            sheets,
            selected_sheet,
            search_text: String::new(),
            show_search: false,
        })
    }
    
    pub fn show(
        &mut self,
        ui: &mut Ui,
        tile: &mut Tile,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ui.horizontal(|ui| {
            // Sheet selector
            ComboBox::from_label("Sheet")
                .selected_text(&self.selected_sheet)
                .show_ui(ui, |ui| {
                    for (key, _) in &self.sheets {
                        ui.selectable_value(&mut self.selected_sheet, key.clone(), key);
                    }
                });
            
            // Search toggle
            if ui.button("🔍").clicked() {
                self.show_search = !self.show_search;
            }
        });
        
        if self.show_search {
            ui.horizontal(|ui| {
                TextEdit::singleline(&mut self.search_text)
                    .desired_width(f32::INFINITY)
                    .show(ui);
            });
            
            if !self.search_text.is_empty() {
                let results = search_sprites(&self.search_text)?;
                for (sheet, index, name) in results.iter().take(5) {
                    if ui.selectable_label(false, 
                        format!("{} [{}]: {}", sheet, index, name)).clicked() {
                        self.selected_sheet = sheet.clone();
                        tile.visual.sprite = Some(SpriteReference {
                            sheet_path: format!("sprites/{}.png", sheet),
                            sprite_index: *index,
                            animation: None,
                        });
                    }
                }
            }
        } else {
            // Sprite grid
            let sprites = get_sprites_for_sheet(&self.selected_sheet)?;
            
            for chunk in sprites.chunks(4) {
                ui.horizontal(|ui| {
                    for (index, name) in chunk {
                        let is_selected = tile.visual.sprite.as_ref()
                            .map(|s| s.sprite_index == *index)
                            .unwrap_or(false);
                        
                        if ui.selectable_label(is_selected, 
                            format!("[{}] {}", index, name)).clicked() {
                            tile.visual.sprite = Some(SpriteReference {
                                sheet_path: format!("sprites/{}.png", self.selected_sheet),
                                sprite_index: *index,
                                animation: None,
                            });
                        }
                    }
                });
            }
        }
        
        Ok(())
    }
}
```

## Error Handling

### Common Errors and Solutions

**Error: "Sprite sheet 'xyz' not found in registry"**

```rust
// Check if sheet exists before using
use antares::sdk::map_editor::has_sprite_sheet;

match has_sprite_sheet("my_sheet") {
    Ok(true) => { /* use sheet */ },
    Ok(false) => eprintln!("Sheet not found"),
    Err(e) => eprintln!("Error: {}", e),
}
```

**Error: "Failed to read sprite_sheets.ron"**

```rust
// Handle missing file gracefully
use antares::sdk::map_editor::browse_sprite_sheets;

match browse_sprite_sheets() {
    Ok(sheets) => { /* use sheets */ },
    Err(e) => {
        eprintln!("Could not load sprites: {}", e);
        // Show fallback UI or disable sprite features
    }
}
```

**Error: "Failed to parse sprite_sheets.ron"**

```rust
// RON format error in data file - log details
match browse_sprite_sheets() {
    Ok(_) => { /* success */ },
    Err(e) => {
        eprintln!("RON parse error: {}", e);
        eprintln!("Check data/sprite_sheets.ron syntax");
    }
}
```

## Performance Considerations

### Caching Sprite Registry

```rust
use std::sync::OnceLock;
use std::collections::HashMap;
use antares::sdk::map_editor::{SpriteSheetInfo, load_sprite_registry};

static SPRITE_REGISTRY: OnceLock<HashMap<String, SpriteSheetInfo>> = OnceLock::new();

pub fn get_cached_registry() -> Result<&'static HashMap<String, SpriteSheetInfo>, 
    Box<dyn std::error::Error>> {
    SPRITE_REGISTRY.get_or_try_init(|| {
        load_sprite_registry()
    }).map_err(|e| e.into())
}
```

### Lazy-Loading Sprite Lists

```rust
// Only load sprites for selected sheet, not all sheets
pub fn load_sprites_on_demand(sheet_key: &str) 
    -> Result<Vec<(u32, String)>, Box<dyn std::error::Error>> {
    use antares::sdk::map_editor::get_sprites_for_sheet;
    get_sprites_for_sheet(sheet_key)
}
```

## Testing Sprite Selection

### Unit Test Example

```rust
#[test]
fn test_sprite_selection_creates_valid_reference() {
    use antares::sdk::map_editor::browse_sprite_sheets;
    use antares::domain::world::SpriteReference;
    
    let sheets = browse_sprite_sheets().unwrap();
    assert!(!sheets.is_empty());
    
    let (sheet_key, texture_path) = &sheets[0];
    let reference = SpriteReference {
        sheet_path: texture_path.clone(),
        sprite_index: 0,
        animation: None,
    };
    
    assert_eq!(reference.sprite_index, 0);
}
```

## Integration Checklist

- [ ] Load sprite registry in Campaign Builder initialization
- [ ] Implement sprite sheet selector in tile inspector
- [ ] Add sprite grid preview for selected sheet
- [ ] Implement sprite search functionality
- [ ] Add sprite validation on map load
- [ ] Persist sprite selections in saved maps
- [ ] Display selected sprite in map preview
- [ ] Handle sprite registry load failures gracefully
- [ ] Cache registry for performance
- [ ] Test sprite selection workflow

## See Also

- **Architecture**: `docs/reference/architecture.md` Section 4.1 (Sprites)
- **Phase 5 Guide**: `docs/explanation/phase5_campaign_builder_sdk_integration.md`
- **Phase 4 Tutorial**: `docs/tutorials/creating_sprites.md`
- **Phase 3 Rendering**: `docs/explanation/phase3_sprite_rendering_integration.md`
