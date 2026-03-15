# Furniture as RON Implementation Plan

## Overview

Furniture is currently hardcoded: `FurnitureType` is a closed Rust enum with 8 variants, appearance presets are `&'static str` constants in code, and procedural meshes are generated entirely from compiled functions. Campaign authors cannot define new furniture types, customize existing ones, or import custom 3D models for furniture.

This plan introduces a data-driven furniture system mirroring the existing item pipeline: a `furniture.ron` catalog file per campaign, a `FurnitureDefinition` domain struct, a `FurnitureDatabase` loader, a `furniture_mesh_registry.ron` for OBJ-imported custom meshes, and a dedicated `FurnitureEditor` tab in the Campaign Builder SDK with full CRUD, import/export, OBJ import, and preset management.

## Current State Analysis

### Existing Infrastructure

- **`FurnitureType` enum** — [types.rs:L1235-L1253](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/world/types.rs#L1235-L1253): 8 hardcoded variants (Throne, Bench, Table, Chair, Torch, Bookshelf, Barrel, Chest)
- **`FurnitureAppearancePreset`** — [types.rs:L1224-L1233](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/world/types.rs#L1224-L1233): code-only struct with `&'static str name`, used by `FurnitureType::default_presets()` (hardcoded Vec)
- **`FurnitureFlags`** — [types.rs:L1185-L1218](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/world/types.rs#L1185-L1218): `lit`, `locked`, `blocking` booleans
- **`FurnitureMaterial`** — [types.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/world/types.rs): Wood, Stone, Metal, Gold with PBR properties
- **`FurnitureCategory`** — Seating, Storage, Decoration, Lighting, Utility (hardcoded, no data file)
- **`MapEvent::Furniture`** — [types.rs:L2109-L2131](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/world/types.rs#L2109-L2131): inline in map RON files, references `FurnitureType` enum variant directly
- **Procedural mesh functions** — `spawn_throne()`, `spawn_bench()`, etc. in [procedural_meshes.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/procedural_meshes.rs)
- **Map editor furniture panel** — [map_editor.rs:L5130-L5334](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/map_editor.rs#L5130-L5334): iterates `FurnitureType::all()`, shows combo boxes for type/material/rotation/scale/color/presets
- **Items pipeline (reference)**: `items.ron` → `Item` struct → `ItemDatabase` → `items_editor.rs` (List/Add/Edit modes, search, filters, import/export/duplicate, RON serialization) → `item_mesh_registry.ron` + `item_mesh_editor.rs` for OBJ-imported meshes
- **OBJ importer** — [obj_importer.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/obj_importer.rs): `ExportType` enum has `Creature` and `Item`; loads OBJ files, assigns colors, exports to RON mesh definitions

### Identified Issues

1. **Closed enum = no extensibility** — campaign authors cannot add new furniture types without recompiling the game engine
2. **No reusable templates** — every `MapEvent::Furniture` specifies material/scale/color/flags from scratch; no shared definitions
3. **No custom mesh import** — items have OBJ→RON mesh import but furniture does not
4. **Hardcoded presets** — `default_presets()` returns `Vec<FurnitureAppearancePreset>` with `&'static str` names; cannot be authored from data
5. **No dedicated editor** — furniture editing is a sub-panel of the map event editor; no list/CRUD/search like items have
6. **No furniture database** — `ContentDatabase` loads items, spells, monsters, NPCs, etc. but has no `FurnitureDatabase`

## Implementation Phases

### Phase 1: Domain Types and `furniture.ron` Data File

Define the `FurnitureDefinition` struct and `FurnitureDatabase`, create the RON data file format, and add loading into `ContentDatabase`.

#### 1.1 Foundation Work

Create a `FurnitureDefinition` struct in a new module `src/domain/world/furniture.rs`:

- `id: FurnitureId` (new type alias `pub type FurnitureId = u32;` in `src/domain/types.rs`)
- `name: String` — display name (e.g. "Iron-Bound Dungeon Door", "Royal Throne")
- `category: FurnitureCategory` — Seating, Storage, Decoration, Lighting, Utility, Passage
- `base_type: FurnitureType` — the procedural mesh template to use (Throne, Bench, etc.)
- `material: FurnitureMaterial` — default material
- `scale: f32` — default scale
- `color_tint: Option<[f32; 3]>` — optional default color
- `flags: FurnitureFlags` — default flags (lit, locked, blocking)
- `icon: Option<String>` — override icon emoji
- `tags: Vec<String>` — freeform tags for filtering
- `mesh_id: Option<FurnitureMeshId>` — optional custom mesh from `furniture_mesh_registry.ron` (overrides procedural mesh)
- `description: Option<String>` — flavor text

All fields derive `Serialize, Deserialize` for RON support.

#### 1.2 `FurnitureDatabase`

Create `FurnitureDatabase` in `src/domain/world/furniture.rs` (or `src/domain/items/database.rs` alongside `ItemDatabase`):

- `items: Vec<FurnitureDefinition>` — loaded from `furniture.ron`
- `get_by_id(id: FurnitureId) -> Option<&FurnitureDefinition>`
- `get_by_name(name: &str) -> Option<&FurnitureDefinition>`
- `get_by_category(cat: FurnitureCategory) -> Vec<&FurnitureDefinition>`
- `get_by_base_type(t: FurnitureType) -> Vec<&FurnitureDefinition>`
- `add(def: FurnitureDefinition) -> Result<(), FurnitureDatabaseError>`

#### 1.3 RON Data File

Create `campaigns/tutorial/data/furniture.ron` and `data/test_campaign/data/furniture.ron` with initial definitions seeded from current `FurnitureType::default_presets()`:

```ron
[
    (
        id: 1,
        name: "Simple Throne",
        category: Seating,
        base_type: Throne,
        material: Wood,
        scale: 1.0,
        color_tint: None,
        flags: (lit: false, locked: false, blocking: true),
        icon: None,
        tags: [],
        mesh_id: None,
        description: Some("A simple wooden throne."),
    ),
    // ... one entry per existing preset
]
```

#### 1.4 Integrate into `ContentDatabase` and `CampaignLoader`

- Add `furniture: FurnitureDatabase` field to `ContentDatabase`
- Add `furniture.ron` to the campaign loading sequence in `CampaignLoader`
- Update `campaign.ron` config to list `furniture.ron` as a data file

#### 1.5 Testing Requirements

- Unit test: `FurnitureDefinition` round-trips through RON serialization
- Unit test: `FurnitureDatabase` CRUD operations
- Unit test: `CampaignLoader` loads `furniture.ron` from `data/test_campaign`
- Unit test: `FurnitureId` type alias works correctly

#### 1.6 Deliverables

- [ ] `FurnitureDefinition` struct with derive Serialize/Deserialize
- [ ] `FurnitureId` type alias
- [ ] `FurnitureDatabase` with lookup methods
- [ ] `furniture.ron` in `campaigns/tutorial/data/` and `data/test_campaign/data/`
- [ ] `ContentDatabase` and `CampaignLoader` integration
- [ ] Tests passing

#### 1.7 Success Criteria

- `cargo nextest run` passes with furniture data loading from test campaign
- `FurnitureDefinition` instances can be serialized to/from RON format
- Existing procedural furniture rendering still works (backward compatible, `FurnitureType` enum is unchanged)

---

### Phase 2: `MapEvent::Furniture` References `FurnitureId`

Update the map event system so furniture placements reference definitions by ID rather than inlining all properties.

#### 2.1 Feature Work

- Add `furniture_id: Option<FurnitureId>` field to `MapEvent::Furniture` — when present, it overrides `furniture_type`/`material`/`scale`/`color_tint`/`flags` with values from the `FurnitureDatabase` definition
- Existing inline fields remain as overrides: if both `furniture_id` and `material` are specified, the inline `material` wins (same pattern as `mesh_descriptor_override` on items)
- Update `furniture_rendering.rs` to resolve `furniture_id` from `ContentDatabase` when spawning

#### 2.2 Backward Compatibility

- All existing `MapEvent::Furniture` entries (inline `furniture_type` + properties) continue to work unchanged
- `furniture_id` defaults to `None` via `#[serde(default)]` so no RON migration is needed
- When `furniture_id` is `Some`, lookup from `FurnitureDatabase`; if ID not found, fall back to inline fields and log a warning

#### 2.3 Map Editor Integration

- In the map editor `EventType::Furniture` panel, add a "Template:" dropdown that lists all `FurnitureDefinition` entries from the loaded campaign
- Selecting a template populates the inline fields (material, scale, color, flags) as starting values and sets `furniture_id`
- User can still override any individual field after selecting a template

#### 2.4 Testing Requirements

- Unit test: `MapEvent::Furniture` with `furniture_id: Some(1)` resolves fields from database
- Unit test: inline overrides take precedence over definition defaults
- Unit test: missing `furniture_id` falls back gracefully
- Verify existing map loading tests pass unchanged

#### 2.5 Deliverables

- [ ] `furniture_id` field on `MapEvent::Furniture`
- [ ] Resolution logic in `furniture_rendering.rs`
- [ ] Map editor template dropdown
- [ ] Tests passing

#### 2.6 Success Criteria

- A `MapEvent::Furniture` with only `furniture_id: Some(1)` renders the correct furniture using all defaults from the definition
- Existing map files with inline-only furniture continue to render identically

---

### Phase 3: Furniture Editor in Campaign Builder SDK

Build a dedicated `FurnitureEditor` tab mirroring the `ItemsEditor` pattern.

#### 3.1 Feature Work: `furniture_editor.rs`

Create `sdk/campaign_builder/src/furniture_editor.rs` with:

- **`FurnitureEditorState`** — `mode: FurnitureEditorMode` (List/Add/Edit), `search_query`, `selected_furniture`, `edit_buffer: FurnitureDefinition`, `filter_category`, `filter_base_type`, `show_import_dialog`, `import_export_buffer`
- **List view** — two-column layout (list + detail preview), `TwoColumnLayout`, `EditorToolbar` with New/Save/Load/Import/Export/Reload, `StandardListItemConfig` with category badges and type icons
- **Edit form** — fields for all `FurnitureDefinition` properties: name, category combo, base type combo, material combo, scale slider, color tint RGB, flags checkboxes, tags list, description text area
- **Import/Export** — RON paste/copy dialog identical to items
- **Toolbar actions** — New (auto-increment ID), Save (write to `furniture.ron`), Load (file picker), Import/Export (RON text), Reload, Duplicate, Delete

#### 3.2 Register Editor Tab

- Add `FurnitureEditor` tab to `CampaignBuilderApp` in [main.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/main.rs) or [campaign_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/campaign_editor.rs)
- Add `furniture: Vec<FurnitureDefinition>` to the campaign builder state
- Wire save/load to `furniture.ron` in the campaign data directory

#### 3.3 Configuration Updates

- Add `"furniture"` key to `ControlsConfig` / campaign config if referenced
- Ensure the tab appears with a 🪑 icon in the editor sidebar

#### 3.4 Testing Requirements

- Unit test: `FurnitureEditorState::default_furniture()` creates valid `FurnitureDefinition`
- Unit test: Add/Edit/Delete operations on furniture list
- Unit test: Save produces valid RON that round-trips
- Follow `sdk/AGENTS.md` egui ID audit rules (push_id in loops, id_salt on ScrollArea, etc.)

#### 3.5 Deliverables

- [ ] `furniture_editor.rs` with List/Add/Edit modes
- [ ] Two-column layout with search, filters, badges
- [ ] Toolbar: New, Save, Load, Import, Export, Reload, Duplicate, Delete
- [ ] Campaign builder tab registration
- [ ] Tests passing

#### 3.6 Success Criteria

- Campaign authors can create, edit, duplicate, delete, import, and export furniture definitions entirely from the SDK UI
- Saving writes to `furniture.ron` in RON format
- Loading an existing campaign populates the furniture list

---

### Phase 4: Furniture Mesh Registry and OBJ Import

Add custom mesh support for furniture, mirroring the item mesh pipeline.

#### 4.1 Feature Work: `furniture_mesh_registry.ron`

Create a mesh registry file at `campaigns/tutorial/data/furniture_mesh_registry.ron`:

```ron
[
    (
        id: 10001,
        name: "CustomOakTable",
        filepath: "assets/furniture/oak_table.ron",
    ),
]
```

- `FurnitureMeshId` type alias (`pub type FurnitureMeshId = u32;`) in `src/domain/types.rs`
- `FurnitureMeshDatabase` struct similar to `ItemMeshDatabase` — loads registry, resolves mesh definition files
- When a `FurnitureDefinition` has `mesh_id: Some(10001)`, the rendering system loads the custom RON mesh instead of calling the procedural `spawn_*` function

#### 4.2 OBJ Import for Furniture and Category Support

Extend the OBJ importer to support Furniture and better directory organization:

- Add `ExportType::Furniture` variant to [obj_importer.rs:L49-L56](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/obj_importer.rs#L49-L56)
- Add a "Category" text field (or combo box) to the OBJ importer UI and `ObjImporterState`.
- Update `preview_export_relative_path` in `obj_importer_ui.rs` to use the category:
  - If category is "Weapons", export path becomes `assets/items/weapons/{name}.ron`.
  - If category is "Tables", export path becomes `assets/furniture/tables/{name}.ron`.
  - Fallback to root `assets/items/{name}.ron` or `assets/furniture/{name}.ron` if category is empty.
- Update `item_mesh_editor.rs` to also support category subfolders when saving new meshes (the existing `assets/items/**/*.ron` glob already reads them, but `default_save_as_path` hardcodes the root directory).
- When exporting as Furniture: write RON mesh definition to the respective path and create a `furniture_mesh_registry.ron` entry.
- Add `furniture_id` field to `ObjImporterState` for suggested next ID (same pattern as `creature_id`).

#### 4.3 Rendering Integration

- In `spawn_furniture()` dispatch, add a check: if the furniture definition has a `mesh_id`, load the custom mesh from the registry and spawn as a `Mesh3d` entity (same approach as item mesh rendering) instead of calling the procedural spawn function
- Apply `FurnitureMaterial` PBR properties to the loaded mesh
- This is the bridge where OBJ-imported 3D models become furniture in the game

#### 4.4 Furniture Editor → OBJ Importer Link

- Add "Open in OBJ Importer" button on the furniture edit form (when `mesh_id` is set), mirroring `requested_open_item_mesh` pattern from `items_editor.rs`
- Add mesh ID selector combo box in the furniture edit form listing all registered furniture mesh entries

#### 4.5 Testing Requirements

- Unit test: `FurnitureMeshDatabase` loads registry from `data/test_campaign`
- Unit test: `ExportType::Furniture` variant exists and default values are correct
- Unit test: OBJ export path generator uses category subfolders (e.g. `assets/items/weapons/sword.ron`)
- Unit test: furniture with `mesh_id` loads custom mesh definition instead of procedural mesh
- Round-trip test: OBJ → export as furniture RON mesh in category folder → load in `FurnitureMeshDatabase` → render

#### 4.6 Deliverables

- [ ] `FurnitureMeshId` type alias
- [ ] `FurnitureMeshDatabase` struct and loader
- [ ] `furniture_mesh_registry.ron` file format and campaign data
- [ ] `ExportType::Furniture` in OBJ importer
- [ ] Category subfolder support in OBJ importer and Item Mesh Editor
- [ ] Custom mesh rendering path in `spawn_furniture()`
- [ ] Furniture editor ↔ OBJ importer cross-tab navigation
- [ ] Tests passing
- [ ] `docs/explanation/implementations.md` updated

#### 4.7 Success Criteria

- A campaign author can: import an OBJ file → export as Furniture mesh → assign the mesh ID to a furniture definition → place the furniture on a map tile → see their custom 3D model rendered in-game
- All quality gates pass: `cargo fmt`, `cargo check`, `cargo clippy`, `cargo nextest run`
