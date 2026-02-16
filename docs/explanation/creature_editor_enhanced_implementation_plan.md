# Creature Editor Enhanced Implementation Plan

## Overview

This plan addresses the architectural shift in creature data representation from embedded `MeshDefinition` structures in `creatures.ron` to file references pointing to individual creature asset files in `campaigns/tutorial/assets/creatures/`. The Creature Editor UI must now provide comprehensive editing capabilities for:

1. **Individual Creature Asset Files** - Full mesh editing capabilities for `.ron` files containing `CreatureDefinition` with embedded meshes
2. **Creature Register Management** - Managing the `creatures.ron` registry file that maps `CreatureId` to file paths
   **Mesh-Level Editing** - Adding, removing, modifying individual meshes within creatures (with both modal dialog and inline table editing)
3. **Visual Preview** - Real-time 3D preview of creature meshes
4. **Template System** - Pre-built creature templates with live previews and primitive generators

## Current State Analysis

### Existing Infrastructure

**Domain Types** (`src/domain/visual/mod.rs`):

- `CreatureDefinition` - Contains `id`, `name`, `meshes: Vec<MeshDefinition>`, `mesh_transforms`, `scale`, `color_tint`
- `MeshDefinition` - Contains `vertices`, `indices`, `normals`, `uvs`, `color`, plus advanced fields (`lod_levels`, `material`, `texture_path`)
- `MeshTransform` - Translation, rotation, scale per mesh
- `CreatureReference` - Used in `creatures.ron` registry: `id`, `name`, `filepath`

**Existing SDK Components**:

- `creatures_editor.rs` - Basic UI scaffold with list/add/edit modes
- `creatures_manager.rs` - File operations, validation, ID management with category-based ID ranges
- `creature_templates.rs` - Template generation functions
- `primitive_generators.rs` - Cube, sphere, cylinder, pyramid generators
- `preview_renderer.rs` - Embedded Bevy app for 3D preview
- `creature_assets.rs` - Asset loading utilities

**Campaign Structure**:

- `campaigns/tutorial/data/creatures.ron` - Registry file containing `Vec<CreatureReference>`
- `campaigns/tutorial/assets/creatures/*.ron` - Individual creature files with full `CreatureDefinition`
- 48 existing creature asset files (monsters, NPCs, templates, variants)

**ID Categories** (from `creatures_manager.rs`):

- Monsters: 1-49
- NPCs: 50-99
- Templates: 100-149
- Variants: 150-199
- Custom: 200+

### Identified Issues

1. **Dual-Level Editing Required** - Editor must handle both registry-level operations (managing references) and asset-level operations (editing mesh data)
2. **File Synchronization** - Changes to creature assets must keep registry in sync
3. **Missing Mesh Editor UI** - No detailed mesh property editor exists
4. **Limited Template Integration** - Templates exist but aren't integrated into UI workflow
5. **No Mesh Manipulation Tools** - Cannot add/remove/duplicate/reorder meshes
6. **Preview Not Wired** - 3D preview renderer exists but isn't connected to editor
7. **Validation Gaps** - Asset file validation separate from registry validation
8. **Category Management** - ID category system exists but not exposed in UI

## Implementation Phases

### Phase 1: Creature Registry Management UI

#### 1.1 Registry Editor Panel

Enhance `creatures_editor.rs` List Mode:

**Registry Overview Section** (top panel):

- Display: "32 creatures registered (15 Monsters, 8 NPCs, 6 Templates, 3 Variants)"
- Category filter dropdown: All | Monsters | NPCs | Templates | Variants | Custom
- Search bar: Filter by name or ID
- Sort options: By ID | By Name | By Category
- Status indicators: Valid (green) | Missing File (red) | Invalid Reference (yellow)

**Registry List View** (left panel, 250px):

```
[ID] Name                 Status  Category
[01] Goblin               ✓      Monster
[02] Kobold               ✓      Monster
[51] VillageElder         ✓      NPC
[52] Innkeeper            ⚠      NPC (file modified)
[99] OldTemplate          ✗      NPC (missing file)
```

**Registry Actions** (toolbar):

- Button: "Add New Creature" - Creates new asset file + adds registry entry
- Button: "Add Existing File" - Imports existing creature file into registry
- Button: "Remove Selected" - Removes from registry (optional: delete file)
- Button: "Revalidate All" - Re-checks all file references
- Button: "Auto-Fix IDs" - Reassigns IDs to match category ranges

#### 1.2 Registry Entry Editor

When registry entry selected (no asset editing):

**Reference Details Panel** (right panel):

- Text Input: `id` (with category validation)
- Text Input: `name` (display name in registry)
- Text Input: `filepath` (relative to campaign root)
- Button: "Browse" - File picker for selecting creature asset
- Category Display: Auto-detected from ID (read-only colored badge)
- Status Display: File exists, parseable, matches ID

**Quick Actions**:

- Button: "Open in Asset Editor" - Switch to asset editing mode
- Button: "Validate Reference" - Check file exists and matches
- Button: "Duplicate Entry" - Copy reference, prompt for new ID
- Button: "Export Reference" - Copy registry entry as RON

#### 1.3 ID Management Tools

Create `sdk/campaign_builder/src/creature_id_manager.rs`:

```rust
pub struct CreatureIdManager {
    registry: Vec<CreatureReference>,
    used_ids: HashSet<CreatureId>,
}

impl CreatureIdManager {
    pub fn suggest_next_id(&self, category: CreatureCategory) -> CreatureId;
    pub fn validate_id(&self, id: CreatureId, category: CreatureCategory) -> Result<(), IdError>;
    pub fn check_conflicts(&self) -> Vec<IdConflict>;
    pub fn auto_reassign_ids(&mut self, category: Option<CreatureCategory>) -> Vec<IdChange>;
    pub fn find_gaps(&self, category: CreatureCategory) -> Vec<CreatureId>;
}
```

UI Integration:

- Badge showing category color-coded by ID range
- Warning icon if ID outside category range
- Auto-suggest next available ID when creating new creature
- Validation error if ID conflicts or out of range

#### 1.4 Testing Requirements

**Unit Tests** (`sdk/campaign_builder/tests/creature_registry_tests.rs`):

- `test_registry_load_all_references` - Load registry file successfully
- `test_add_creature_reference` - Add new reference to registry
- `test_remove_creature_reference` - Remove reference from registry
- `test_duplicate_id_detection` - Detect duplicate IDs across registry
- `test_category_validation` - ID matches category range
- `test_auto_suggest_next_id` - Suggests correct next ID per category
- `test_validate_file_references` - All filepaths exist and parse
- `test_registry_save_roundtrip` - Save and reload preserves data

**Integration Tests**:

- `test_registry_editor_add_remove` - Add/remove entries via UI
- `test_registry_category_filter` - Filter by category works
- `test_registry_search` - Search by name/ID works
- `test_validate_all_button` - Validation reports all issues

#### 1.5 Deliverables

- [ ] Enhanced `creatures_editor.rs` with registry management UI
- [ ] `creature_id_manager.rs` with ID management logic
- [ ] Category badge UI component
- [ ] Validation status indicators in list view
- [ ] Add/remove registry entry functionality
- [ ] ID conflict detection and resolution tools
- [ ] Unit tests with >80% coverage
- [ ] Documentation in `docs/how-to/manage_creature_registry.md`

#### 1.6 Success Criteria

- Can view all registered creatures with status indicators
- Can filter by category and search by name/ID
- Can add/remove registry entries without editing assets
- ID conflicts and category mismatches clearly displayed
- Validation shows which files are missing or invalid
- Auto-suggest provides correct next ID per category

---

### Phase 2: Creature Asset Editor UI

#### 2.1 Asset Editor Mode

Enhance `creatures_editor.rs` Edit Mode for full asset editing:

**Editor Layout** (when creature asset selected):

```
+------------------+------------------------+--------------------+
| Mesh List        | 3D Preview             | Mesh Properties    |
| (250px)          | (flex)                 | (350px)            |
|                  |                        |                    |
| ☑ head           | [Interactive Bevy      | [Selected Mesh     |
| ☑ torso          |  Preview Window]       |  Property Editor]  |
| ☐ left_arm       |                        |                    |
| ☐ right_arm      | Controls:              |                    |
| ☑ left_leg       | [ Grid ] [ Wireframe ] |                    |
| ☑ right_leg      | [ Normals ] [ Reset ]  |                    |
|                  |                        |                    |
| [Add] [Dupe] [X] |                        |                    |
+------------------+------------------------+--------------------+
| Creature Properties (bottom panel)                             |
| ID: [  1  ] Name: [ Goblin        ] Scale: [ 0.72 ] Tint: [▣]|
+----------------------------------------------------------------+
```

**Mesh List Panel** (left, 250px):

- Checkboxes to show/hide meshes in preview
- Selectable list (click to edit properties)
- Display mesh name or "unnamed_mesh_3" if no name
- Vertex count badge: "(234 verts)"
- Color indicator dot matching mesh color
- Drag-to-reorder support (updates `meshes` and `mesh_transforms` order)

**Mesh List Toolbar**:

- Button: "Add Primitive" - Dropdown: Cube | Sphere | Cylinder | Pyramid | Cone
- Button: "Duplicate" - Clone selected mesh and transform
- Button: "Delete" - Remove selected mesh (with confirmation)
- Button: "Import OBJ" - Load mesh from external OBJ file (future)

#### 2.2 3D Preview Integration

Wire up `preview_renderer.rs` to creature editor:

**Preview Controls** (overlay on preview):

- Toggle: Grid floor (on/off)
- Toggle: Wireframe overlay (on/off)
- Toggle: Normal visualization (debug arrows)
- Button: Reset Camera (return to default view)
- Slider: Camera Distance (1.0 - 10.0)
- Color Picker: Background color

**Camera Controls**:

- Left-drag: Rotate camera around creature
- Right-drag: Pan camera
- Scroll: Zoom in/out
- Double-click: Focus on selected mesh

**Preview Updates**:

- Automatically refresh when mesh data changes
- Highlight selected mesh in preview (brighter or outlined)
- Show mesh coordinate axes when mesh selected
- Display bounding box around creature

#### 2.3 Mesh Property Editor Panel

When mesh selected, show detailed properties (right panel, 350px):

**Mesh Info Section**:

```
Name: [left_leg                    ]
Color: [▣] (0.36, 0.50, 0.22, 1.0)

Vertices: 8      Triangles: 12
```

**Transform Section**:

```
Translation
  X: [-0.15] slider (-5.0 to 5.0)
  Y: [ 0.00] slider (-5.0 to 5.0)
  Z: [ 0.00] slider (-5.0 to 5.0)

Rotation (degrees)
  Pitch: [  0.0] slider (0-360)
  Yaw:   [  0.0] slider (0-360)
  Roll:  [  0.0] slider (0-360)

Scale
  X: [1.0] Y: [1.0] Z: [1.0]
  [☑] Uniform scaling
```

**Geometry Section**:

```
☐ Auto-calculate normals
☐ Generate UVs (planar projection)

Vertices: [View/Edit Table]
Indices:  [View/Edit Table]
Normals:  [View/Edit Table] (if present)
```

**Advanced Section** (collapsible):

```
Material: [None ▼]
Texture:  [None] [Browse...]

LOD Levels: [Not configured] [Configure...]
```

**Action Buttons**:

- Button: "Replace with Primitive" - Regenerate geometry
- Button: "Export to OBJ" - Export this mesh only
- Button: "Validate Mesh" - Check for issues
- Button: "Reset Transform" - Identity transform

#### 2.4 Primitive Replacement Flow

When "Replace with Primitive" clicked:

**Primitive Generator Dialog**:

```
Select Primitive Type:
  (•) Cube     ( ) Sphere    ( ) Cylinder
  ( ) Pyramid  ( ) Cone      ( ) Custom

Cube Settings:
  Size: [1.0]

Color:
  [▣] Use current mesh color
  [▣] Custom: (0.5, 0.5, 0.5, 1.0)

Options:
  [☑] Preserve transform
  [☑] Keep mesh name

[Generate] [Cancel]
```

Update mesh in-place with new geometry.

#### 2.5 Creature-Level Properties

Bottom panel for creature-wide settings:

**Identity Section**:

- `id`: Text input with category badge
- `name`: Text input (creature display name)

**Visual Settings**:

- `scale`: Slider (0.1 - 5.0) - Global scale multiplier
- `color_tint`: Color picker with "None" checkbox

**Validation Display**:

- Error count: "3 errors, 1 warning"
- Button: "Show Issues" - Expand validation panel

**File Operations**:

- Button: "Save Asset" - Write to creature file
- Button: "Save As" - Write to new file, update registry
- Button: "Export RON" - Copy asset as RON text
- Button: "Revert Changes" - Reload from file

#### 2.6 Testing Requirements

**Unit Tests** (`sdk/campaign_builder/tests/creature_asset_editor_tests.rs`):

- `test_load_creature_asset` - Load creature from file
- `test_add_mesh_to_creature` - Add new mesh
- `test_remove_mesh_from_creature` - Remove mesh (updates transforms)
- `test_duplicate_mesh` - Clone mesh and transform
- `test_reorder_meshes` - Drag-drop reorder
- `test_update_mesh_transform` - Modify transform values
- `test_update_mesh_color` - Modify mesh color
- `test_replace_mesh_with_primitive` - Primitive generation
- `test_creature_scale_multiplier` - Global scale application
- `test_save_asset_to_file` - Write modified asset

**Integration Tests** (`sdk/campaign_builder/tests/creature_editor_integration.rs`):

- `test_full_creature_creation_workflow` - Create from scratch to save
- `test_edit_existing_creature_workflow` - Load, edit, save
- `test_mesh_manipulation_workflow` - Add, edit, remove, save
- `test_preview_updates_on_change` - Preview reflects edits
- `test_validation_before_save` - Invalid creatures can't save

**Manual UI Tests**:

- Load creature asset - All meshes appear in list and preview
- Select mesh - Properties populate correctly
- Change transform - Preview updates in real-time
- Change color - Preview shows new color
- Add cube primitive - New mesh appears
- Delete mesh - Mesh removed from list and preview
- Save asset - File written correctly
- Reload asset - All changes persisted

#### 2.7 Deliverables

- [ ] Enhanced `creatures_editor.rs` with full asset editing UI
- [ ] Three-panel layout (mesh list, preview, properties)
- [ ] Integrated 3D preview with camera controls
- [ ] Mesh property editor with transform/color/geometry controls
- [ ] Primitive replacement dialog
- [ ] Mesh add/duplicate/delete operations
- [ ] Creature-level property editor
- [ ] Save/load asset file operations
- [ ] Unit tests with >80% coverage
- [ ] Documentation in `docs/how-to/edit_creature_assets.md`

#### 2.8 Success Criteria

- Can load any existing creature asset file
- Can add/remove/duplicate meshes
- Can edit mesh transforms with sliders
- Can change mesh colors with picker
- Can replace mesh with primitive
- Preview updates reflect all changes immediately
- Can save modified creature to file
- Validation prevents saving invalid creatures
- All 48 existing creatures load without errors

---

### Phase 3: Template System Integration

#### 3.1 Template Browser UI

Create `sdk/campaign_builder/src/creature_template_browser.rs`:

**Template Browser Dialog** (modal, 800x600):

```
+----------------------------------+---------------------------------+
| Category Filter                  | Template Details                |
|                                  |                                 |
| [All        ▼] Search: [____]   | Humanoid Fighter                |
|                                  |                                 |
| Humanoids                        | Category: Humanoid              |
|   ☐ Fighter                      | Complexity: ★★☆☆☆              |
|   ☐ Mage                         | Mesh Count: 12                  |
|   ☐ Cleric                       |                                 |
|                                  | A basic humanoid fighter with   |
| Creatures                        | head, torso, arms, legs, and    |
|   ☐ Quadruped                    | sword. Good starting point.     |
|   ☐ Dragon                       |                                 |
|   ☐ Spider                       | [Live 3D Preview]               |
|                                  | [Rotating Bevy render]          |
| Primitives                       | [Turntable: ON]                 |
|   ☐ Cube                         |                                 |
|   ☐ Sphere                       |                                 |
|   ☐ Cylinder                     |                                 |
|                                  |                                 |
+----------------------------------+---------------------------------+
| [Use Template] [Close]                                            |
+-------------------------------------------------------------------+
```

**Live Preview Implementation**:

- Embed Bevy renderer in template browser (reuse `preview_renderer.rs`)
- Auto-generate creature on template selection
- Turntable rotation by default (360° in 5 seconds)
- User can pause rotation and interact with camera
- Preview updates when switching between templates

**Template Categories**:

- Humanoids: Fighter, Mage, Cleric, Rogue, Archer
- Creatures: Quadruped, Dragon, Spider, Snake, Bird
- Undead: Skeleton, Zombie, Ghost, Lich
- Robots: Basic, Advanced, Flying
- Primitives: Cube, Sphere, Cylinder, Pyramid, Cone

#### 3.2 Template Metadata System

Enhance `creature_templates.rs`:

```rust
pub struct TemplateMetadata {
    pub id: String,
    pub name: String,
    pub category: TemplateCategory,
    pub complexity: Complexity,
    pub mesh_count: usize,
    pub description: String,
    pub tags: Vec<String>,
}

pub enum TemplateCategory {
    Humanoid,
    Creature,
    Undead,
    Robot,
    Primitive,
}

pub enum Complexity {
    Beginner,    // 1-5 meshes
    Intermediate, // 6-15 meshes
    Advanced,     // 16-30 meshes
    Expert,       // 31+ meshes
}

pub struct TemplateRegistry {
    templates: Vec<(TemplateMetadata, fn() -> CreatureDefinition)>,
}

impl TemplateRegistry {
    pub fn all_templates() -> &'static [TemplateMetadata];
    pub fn by_category(cat: TemplateCategory) -> Vec<&'static TemplateMetadata>;
    pub fn search(query: &str) -> Vec<&'static TemplateMetadata>;
    pub fn generate(template_id: &str) -> Result<CreatureDefinition, TemplateError>;
}
```

#### 3.3 Enhanced Template Generators

Expand `creature_templates.rs` with production-ready templates:

**Humanoid Templates**:

- `template_humanoid_fighter()` - Sword and shield, armor
- `template_humanoid_mage()` - Robes, staff, pointed hat
- `template_humanoid_cleric()` - Holy symbol, mace, robes
- `template_humanoid_rogue()` - Daggers, hood, light armor
- `template_humanoid_archer()` - Bow, quiver, light armor

**Creature Templates**:

- `template_quadruped_basic()` - Four legs, body, head, tail
- `template_quadruped_wolf()` - Wolf-like proportions
- `template_dragon_basic()` - Body, wings, tail, head, legs
- `template_spider_basic()` - Eight legs, body segments
- `template_snake_basic()` - Segmented body, no legs

**Undead Templates**:

- `template_skeleton_basic()` - Bones, simplified geometry
- `template_zombie_basic()` - Humanoid, decomposed appearance
- `template_ghost_basic()` - Translucent, flowing form

**Robot Templates**:

- `template_robot_basic()` - Cubic, mechanical
- `template_robot_advanced()` - More complex joints
- `template_robot_flying()` - With propulsion elements

#### 3.4 Template Application Workflow

**From Creature Editor**:

Button: "New from Template" (in creature list mode)
→ Opens Template Browser
→ User selects template
→ Dialog: "Create New Creature"

- Name: [ ]
- Category: [Monster ▼] (auto-suggests ID)
- Suggested ID: [5] (next available in category)
  → Creates new asset file at `assets/creatures/{name}.ron`
  → Adds registry entry to `creatures.ron`
  → Opens in Asset Editor mode

**From Asset Editor**:

Button: "Load Template" (in mesh list toolbar)
→ Opens Template Browser
→ User selects template
→ Warning: "Replace current meshes? [Yes] [Merge] [Cancel]"
→ If Yes: Clear all meshes, load template
→ If Merge: Append template meshes to current
→ Preview updates

#### 3.5 Testing Requirements

**Unit Tests** (`sdk/campaign_builder/tests/template_system_tests.rs`):

- `test_all_templates_generate` - All templates create valid creatures
- `test_template_metadata_complete` - All templates have metadata
- `test_template_search` - Search finds correct templates
- `test_template_category_filter` - Filter by category works
- `test_template_complexity_correct` - Mesh counts match complexity
- `test_generate_all_humanoids` - All humanoid templates valid
- `test_generate_all_creatures` - All creature templates valid
- `test_template_preview_renders` - Live previews render without errors

**Integration Tests**:

- `test_template_browser_ui` - Browser renders without errors
- `test_template_browser_preview` - Live preview updates on selection
- `test_create_from_template_workflow` - Full creation flow works
- `test_merge_template_workflow` - Merge templates into existing
- `test_template_turntable_rotation` - Auto-rotation works correctly

#### 3.6 Deliverables

- [ ] `creature_template_browser.rs` with browser UI and embedded Bevy renderer
- [ ] Enhanced `creature_templates.rs` with metadata system
- [ ] 15+ production-ready templates
- [ ] Live preview with turntable rotation in template browser
- [ ] Integration with creature editor "New from Template" flow
- [ ] Template merge functionality
- [ ] Unit tests with >80% coverage
- [ ] Documentation in `docs/how-to/use_creature_templates.md`

#### 3.7 Success Criteria

- Template browser displays all available templates
- Can filter by category and search by name
- Live preview renders selected template with turntable rotation
- Can pause rotation and interact with preview camera
- Can create new creature from template
- Can merge template into existing creature
- All templates generate valid creatures
- Templates have correct metadata and complexity ratings

---

### Phase 4: Advanced Mesh Editing Tools

#### 4.1 Mesh Vertex Editor

Create `sdk/campaign_builder/src/mesh_vertex_editor.rs`:

**Vertex Table View** (in mesh property panel):

```
Vertices (8)                              [Add Vertex] [Delete] [Export]

  #  | X       | Y       | Z       | [Actions]
 ----+---------+---------+---------+-----------
  0  | -0.150  | -0.600  | -0.100  | [Edit] [X]
  1  | -0.050  | -0.600  | -0.100  | [Edit] [X]
  2  | -0.050  |  0.000  | -0.100  | [Edit] [X]
  3  | -0.150  |  0.000  | -0.100  | [Edit] [X]
  4  | -0.150  | -0.600  |  0.050  | [Edit] [X]
  5  | -0.050  | -0.600  |  0.050  | [Edit] [X]
  6  | -0.050  |  0.000  |  0.050  | [Edit] [X]
  7  | -0.150  |  0.000  |  0.050  | [Edit] [X]

[☑] Show in preview  [☐] Snap to grid (0.05)
```

**Vertex Edit Dialog** (for precision edits):

```
Edit Vertex #2

X: [-0.050] Y: [ 0.000] Z: [-0.100]

[☑] Snap to grid: [0.05 ▼]

[Apply] [Cancel]
```

**Inline Editing** (for quick edits):

- Double-click cell to edit value directly in table
- Tab/Enter to move to next cell
- Escape to cancel edit

**Bulk Operations**:

- Select multiple vertices (checkbox in table)
- Button: "Translate Selected" - Move all by offset
- Button: "Scale Selected" - Scale from centroid
- Button: "Mirror Selected" - Mirror across axis

#### 4.2 Mesh Index Editor

**Index Table View**:

```
Triangles (12)                            [Add Triangle] [Delete] [Auto]

  #  | V1 | V2 | V3 | [Actions]
 ----+----+----+----+-----------
  0  |  0 |  1 |  2 | [Edit] [X]
  1  |  2 |  3 |  0 | [Edit] [X]
  2  |  1 |  5 |  6 | [Edit] [X]
  3  |  6 |  2 |  1 | [Edit] [X]
  ...

[Auto-Calculate from Geometry] (uses marching cubes or convex hull)
```

**Validation**:

- Highlight invalid indices (out of bounds)
- Show warnings for degenerate triangles (duplicate vertices)
- Show warnings for non-manifold edges

#### 4.3 Mesh Normal Editor

**Normal Table View** (if normals present):

```
Normals (8)                               [Calculate] [Clear] [Export]

  #  | X       | Y       | Z       | Length | [Actions]
 ----+---------+---------+---------+--------+-----------
  0  |  0.000  |  0.000  |  1.000  | 1.000  | [Edit] [X]
  1  |  0.000  |  0.000  |  1.000  | 1.000  | [Edit] [X]
  ...

[☑] Auto-normalize (keep unit length)
```

**Normal Operations**:

- Button: "Calculate from Geometry" - Auto-generate smooth normals
- Button: "Calculate Flat" - One normal per triangle
- Button: "Flip All" - Reverse all normals
- Button: "Clear" - Remove normals (will be auto-generated)

#### 4.4 Mesh Validation Tools

Create `sdk/campaign_builder/src/mesh_validator.rs`:

```rust
pub struct MeshValidationReport {
    pub errors: Vec<MeshError>,
    pub warnings: Vec<MeshWarning>,
    pub info: Vec<MeshInfo>,
}

pub enum MeshError {
    NoVertices,
    NoIndices,
    InvalidIndex { index: u32, max: usize },
    DegenerateTriangle { triangle_idx: usize },
    NonManifoldEdge { vertex_a: usize, vertex_b: usize },
    MismatchedNormalCount { vertices: usize, normals: usize },
}

pub enum MeshWarning {
    UnnormalizedNormal { index: usize, length: f32 },
    DuplicateVertex { index_a: usize, index_b: usize },
    UnusedVertex { index: usize },
    LargeTriangle { triangle_idx: usize, area: f32 },
}

pub enum MeshInfo {
    VertexCount(usize),
    TriangleCount(usize),
    BoundingBox([f32; 3], [f32; 3]),
    SurfaceArea(f32),
}

pub fn validate_mesh(mesh: &MeshDefinition) -> MeshValidationReport;
```

**Validation Panel** (in mesh property editor):

```
Mesh Validation                           [Validate]

Status: ⚠ 1 warning

Warnings:
  • Vertex #5 is unused by triangles
  • Triangle #8 has very small area (0.0001)

Info:
  • 8 vertices, 12 triangles
  • Bounding box: (-0.15, -0.6, -0.1) to (-0.05, 0.0, 0.05)
  • Surface area: 0.234 square units

[Auto-Fix Issues]
```

#### 4.5 Mesh Import/Export

**RON Export**:

- Button: "Copy Mesh as RON" - Copy to clipboard
- Formatted RON representation of `MeshDefinition`
- Useful for sharing or manual editing

**RON Creature Export**:

- Button: "Export Creature as RON" - Save entire `CreatureDefinition`
- Option: Pretty-print for readability
- Option: Compact format for production

**Note**: OBJ and other mesh formats deferred to future phase. RON-only for initial implementation maintains consistency with campaign data format.

#### 4.6 Testing Requirements

**Unit Tests** (`sdk/campaign_builder/tests/mesh_editing_tools_tests.rs`):

- `test_vertex_add_remove` - Add/remove vertices
- `test_vertex_edit_coordinates` - Modify vertex positions
- `test_index_validation` - Detect invalid indices
- `test_normal_calculation` - Auto-calculate normals
- `test_mesh_validation_errors` - Detect all error types
- `test_mesh_validation_warnings` - Detect all warning types
- `test_auto_fix_issues` - Auto-fix common problems
- `test_ron_export_import_roundtrip` - Export and re-import preserves data

#### 4.7 Deliverables

- [ ] `mesh_vertex_editor.rs` with vertex table and editing
- [ ] `mesh_validator.rs` with comprehensive validation
- [ ] Index editor UI
- [ ] Normal editor UI
- [ ] Validation panel with auto-fix
- [ ] RON import/export utilities (OBJ deferred)
- [ ] Unit tests with >80% coverage
- [ ] Documentation in `docs/how-to/advanced_mesh_editing.md`

#### 4.8 Success Criteria

- Can view and edit vertex positions both inline and via dialog
- Can add/remove vertices and update indices
- Can auto-calculate normals from geometry
- Validation detects all errors and warnings
- Auto-fix resolves common issues
- Can export meshes and creatures to RON format
- All validation rules documented

---

### Phase 5: Workflow Integration & Polish

#### 5.1 Unified Workflow

**Creature Creation Workflow**:

1. User clicks "New Creature" in registry list
2. Dialog: "Create From Template or Scratch?"
   - Option: "From Template" → Opens template browser
   - Option: "From Scratch" → Empty creature
3. Creature Editor opens in asset edit mode
4. User adds/edits meshes
5. Preview updates in real-time
6. User clicks "Save" → Writes asset file + adds registry entry
7. Returns to registry list view

**Creature Editing Workflow**:

1. User selects creature in registry list
2. Clicks "Edit Asset" → Opens asset editor
3. Mesh list, preview, properties panels visible
4. User modifies meshes, transforms, colors
5. Preview updates continuously
6. User clicks "Save" → Writes asset file
7. Returns to registry list view

**Dual-Mode Indicator**:

- Top bar shows current mode: "Registry Mode" or "Asset Editor: goblin.ron"
- Breadcrumb: Creatures > Monsters > Goblin > left_leg
- Button: "Back to Registry" (always visible)

#### 5.2 Enhanced Preview Features

**Preview Toolbar Enhancements**:

- Button: "Snapshot" - Save preview image to file
- Button: "Turntable" - Auto-rotate creature for inspection
- Dropdown: "Lighting" - Day | Night | Dungeon | Studio
- Slider: "Animation Speed" (if animations present)

**Preview Overlays**:

- Show mesh names on hover
- Show transform axes for selected mesh
- Show bounding boxes per mesh
- Show center of mass indicator

#### 5.3 Keyboard Shortcuts

Define shortcuts for common operations:

**Navigation**:

- `Ctrl+S` - Save current asset
- `Ctrl+N` - New creature
- `Ctrl+O` - Open template browser
- `Escape` - Return to registry mode
- `Tab` - Cycle through panels

**Editing**:

- `Del` - Delete selected mesh
- `Ctrl+D` - Duplicate selected mesh
- `Ctrl+Z` - Undo (if undo system integrated)
- `Ctrl+Y` - Redo (if undo system integrated)

**Preview**:

- `Space` - Reset camera
- `W/A/S/D` - Pan camera
- `Q/E` - Rotate camera
- `R` - Toggle wireframe
- `G` - Toggle grid

#### 5.4 Context Menus

**Mesh List Context Menu** (right-click on mesh):

- Edit Properties
- Duplicate
- Delete

---

- Move Up
- Move Down

---

- Export to OBJ
- Copy as RON

---

- Hide/Show in Preview

**Preview Context Menu** (right-click on preview):

- Reset Camera
- Focus on Selected

---

- Wireframe On/Off
- Grid On/Off
- Normals On/Off

---

- Snapshot
- Copy Camera Settings

#### 5.5 Undo/Redo Integration

Integrate with existing `undo_redo.rs` system:

**Undoable Operations**:

- Add mesh
- Remove mesh
- Modify mesh transform
- Modify mesh color
- Modify mesh geometry (vertices/indices)
- Modify creature properties (name, scale, tint)
- Replace mesh with primitive
- Reorder meshes

**Implementation**:

```rust
pub enum CreatureEditAction {
    AddMesh { mesh: MeshDefinition, transform: MeshTransform },
    RemoveMesh { index: usize, mesh: MeshDefinition, transform: MeshTransform },
    ModifyTransform { index: usize, old: MeshTransform, new: MeshTransform },
    ModifyMesh { index: usize, old: MeshDefinition, new: MeshDefinition },
    ModifyCreatureProps { old: CreatureProperties, new: CreatureProperties },
}

impl UndoableAction for CreatureEditAction {
    fn undo(&self, state: &mut CreaturesEditorState);
    fn redo(&self, state: &mut CreaturesEditorState);
    fn description(&self) -> String;
}
```

#### 5.6 Auto-Save & Recovery

**Auto-Save Feature**:

- Every 60 seconds, save editor state to temp file
- Format: `.campaign_builder/autosave/creature_{id}_{timestamp}.ron`
- On startup, check for autosave files
- Prompt: "Recover unsaved changes to Goblin? [Yes] [No] [Compare]"

**Crash Recovery**:

- Store editor state in autosave before every risky operation
- On crash and restart, detect autosave files
- Offer recovery with diff view

#### 5.7 Testing Requirements

**Integration Tests** (`sdk/campaign_builder/tests/creature_workflow_tests.rs`):

- `test_full_creation_workflow` - New creature from template to save
- `test_full_editing_workflow` - Load, edit, save existing creature
- `test_registry_to_asset_navigation` - Switch modes correctly
- `test_undo_redo_full_session` - Undo/redo through editing session
- `test_autosave_recovery` - Autosave and recover work

**Manual E2E Tests**:

- Create creature from template → Edit → Save → Reload
- Edit existing creature → Add meshes → Undo → Redo → Save
- Switch between registry and asset mode multiple times
- Autosave triggers and recovery works
- Keyboard shortcuts function correctly

#### 5.8 Deliverables

- [ ] Unified workflow with clear mode switching
- [ ] Enhanced preview with overlays and snapshots
- [ ] Keyboard shortcuts for all common operations
- [ ] Context menus for mesh list and preview
- [ ] Undo/redo integration for all edit operations
- [ ] Auto-save and crash recovery system
- [ ] Integration tests with complete workflows
- [ ] Documentation in `docs/how-to/creature_editor_workflows.md`

#### 5.9 Success Criteria

- Can complete full creature creation workflow without confusion
- Mode switching is intuitive and clear
- Keyboard shortcuts improve editing efficiency
- Undo/redo works for all operations
- Auto-save prevents data loss
- Preview provides all necessary visual feedback
- Users can efficiently create and edit creatures

---

## Cross-Cutting Concerns

### Documentation Updates

**How-To Guides** (`docs/how-to/`):

- `manage_creature_registry.md` - Registry management operations
- `edit_creature_assets.md` - Asset editing workflows
- `use_creature_templates.md` - Template system usage
- `advanced_mesh_editing.md` - Mesh vertex/index/normal editing
- `creature_editor_workflows.md` - Complete workflows

**Reference Documentation** (`docs/reference/`):

- Update `architecture.md` Section 7 with creature file structure
- Document ID category ranges and validation rules
- Document `CreatureReference` vs `CreatureDefinition` distinction

**Explanation Documentation** (`docs/explanation/`):

- `implementations.md` - Update with creature editor implementation
- Document dual-level editing architecture (registry + assets)

### Validation Strategy

**Registry-Level Validation**:

- All IDs unique within registry
- All IDs within category range
- All file paths exist
- All referenced files parseable
- All referenced files contain matching ID

**Asset-Level Validation**:

- Mesh vertices non-empty
- Mesh indices valid (within vertex range)
- Mesh indices form valid triangles (groups of 3)
- Normals count matches vertices (if present)
- Transforms count matches meshes count
- No NaN or Inf values in floats

**Cross-Validation**:

- Asset file ID matches registry reference ID
- No orphaned asset files (in assets/ but not in registry)
- No missing asset files (in registry but file doesn't exist)

### Error Handling

**Registry Operations**:

```rust
pub enum RegistryError {
    FileNotFound(PathBuf),
    ParseError(String),
    DuplicateId(CreatureId),
    InvalidReference(String),
    IoError(std::io::Error),
}
```

**Asset Operations**:

```rust
pub enum AssetError {
    FileNotFound(PathBuf),
    ParseError(String),
    ValidationError(Vec<MeshError>),
    IoError(std::io::Error),
    IdMismatch { file_id: CreatureId, registry_id: CreatureId },
}
```

**User-Facing Error Messages**:

- Show clear, actionable error messages in UI
- Provide "Fix" buttons where possible
- Log detailed errors to console for debugging

### Performance Considerations

**Preview Rendering**:

- Cache mesh generation for unchanged meshes
- Use LOD for complex creatures (>1000 triangles)
- Debounce preview updates (50ms delay after edit)

**Large Creature Handling**:

- Lazy-load creature assets (don't load all on startup)
- Paginate registry list (show 50 at a time)
- Virtual scrolling for large mesh lists

**File I/O**:

- Async file operations for large assets
- Progress indicators for slow operations
- Background validation thread

---

## Migration Path

### Existing Creatures

All 48 existing creature files already follow new structure:

- ✅ Individual `.ron` files in `assets/creatures/`
- ✅ `CreatureDefinition` with embedded meshes
- ✅ Registry in `data/creatures.ron` with references

**No migration needed** - system is already using new structure.

### User Experience

**First-Time Users**:

- Tutorial overlay on first launch
- Tooltips on all controls
- Example creatures pre-loaded
- Template browser as entry point

**Existing SDK Users**:

- Mode indicator makes dual-level editing clear
- Breadcrumb navigation shows context
- Keyboard shortcuts document available via `F1`

---

## Risk Mitigation

### Technical Risks

**Risk**: Preview rendering performance with complex creatures

- **Mitigation**: LOD system, mesh caching, progressive rendering

**Risk**: File synchronization between registry and assets

- **Mitigation**: Atomic save operations, validation before save, auto-fix tools

**Risk**: Undo/redo system complexity with nested data

- **Mitigation**: Clone entire creature state for undo snapshots

### User Experience Risks

**Risk**: Confusion between registry mode and asset mode

- **Mitigation**: Clear mode indicators, breadcrumb nav, persistent toolbar

**Risk**: Accidental data loss from unsaved changes

- **Mitigation**: Auto-save, unsaved changes indicator, confirm on exit

**Risk**: Overwhelming UI for simple tasks

- **Mitigation**: Collapsible panels, sensible defaults, template shortcuts

---

## Implementation Order Summary

**Week 1-2: Phase 1** - Registry Management UI

- Solid foundation for managing creature references
- ID management and category system
- Validation framework

**Week 3-4: Phase 2** - Asset Editor UI

- Core editing functionality
- 3D preview integration
- Mesh manipulation

**Week 5: Phase 3** - Template System

- Template browser
- Template application workflows
- Production templates

**Week 6: Phase 4** - Advanced Tools

- Vertex/index editing
- Validation tools
- Import/export

**Week 7: Phase 5** - Integration & Polish

- Workflow refinement
- Undo/redo
- Auto-save
- Final testing

---

## Success Metrics

**Functional Completeness**:

- [ ] Can manage all 48 existing creatures without errors
- [ ] Can create new creature from template in <2 minutes
- [ ] Can edit existing creature and save changes
- [ ] Can add/remove/modify meshes
- [ ] Preview updates reflect all changes
- [ ] Validation catches all issues before save

**Quality Metrics**:

- [ ] All cargo checks pass (fmt, clippy, test)
- [ ] > 80% test coverage for new code
- [ ] Zero panics or unwraps in production code
- [ ] All error paths handled gracefully

**User Experience**:

- [ ] Mode switching is intuitive
- [ ] Keyboard shortcuts improve efficiency
- [ ] Auto-save prevents data loss
- [ ] Documentation covers all workflows
- [ ] UI responsive (<16ms frame time)

**Documentation**:

- [ ] All how-to guides complete
- [ ] Reference docs updated
- [ ] Implementation notes in `implementations.md`
- [ ] Inline doc comments for all public APIs

---

## Conclusion

This plan provides a comprehensive path to full creature editing capabilities in the Campaign Builder SDK. The dual-level architecture (registry management + asset editing) respects the current file structure while providing powerful editing tools. The template system accelerates creature creation, while advanced mesh tools enable precise control. Integration with existing SDK features (undo/redo, validation, auto-save) ensures consistency with the rest of the builder.

**Key Innovations**:

1. **Dual-Level Editing** - Clear separation between registry (references) and assets (mesh data)
2. **Template-First Workflow** - Templates with live turntable previews as primary entry point for new users
3. **Real-Time Preview** - Embedded Bevy rendering for immediate visual feedback in both editor and template browser
4. **Category-Based ID System** - Automatic ID management based on creature type
5. **Comprehensive Validation** - Multi-level validation prevents errors at save time
6. **Flexible Mesh Editing** - Both inline table editing and modal dialogs for vertex manipulation
7. **RON-Native Workflow** - Import/export in campaign-native RON format

**Next Steps**:

1. Review plan with stakeholders
2. Begin Phase 1 implementation (Registry Management UI)
3. Iterate based on user feedback after Phase 2
4. Expand template library based on user needs
5. Consider animation system integration (Phase 6 from original plan)
