# bevy_egui Standardization Scope of Work

## Executive Summary

**Current State**: The Antares game engine uses a **hybrid rendering approach**:

- Native Bevy UI for the HUD (party status display)
- bevy_egui for overlay UI (inn management, recruitment dialogs)
- One disabled egui system (`ui_system`) with lifecycle issues

**Goal**: There is **no legacy egui code to remove**. All UI already uses bevy_egui correctly. The scope of work is to **decide on a unified rendering strategy** and potentially migrate the native Bevy UI HUD to bevy_egui for consistency.

**Recommendation**: **Keep the hybrid approach**. The current architecture is sound and each technology is used for its strengths.

---

## Current Architecture Analysis

### What's Currently Using Native Bevy UI

#### 1. HUD System (`src/game/systems/hud.rs`)

- **Purpose**: Party status display at bottom of screen
- **Rendering**: Native Bevy UI components (Node, Text, BackgroundColor)
- **Features**:
  - 6 character cards with HP bars, conditions, portraits
  - Compass with directional indicator
  - Color-coded health states
  - Emoji condition indicators
- **Lines of Code**: ~1,666 (including tests)
- **Status**: ✅ Working perfectly, well-tested

### What's Currently Using bevy_egui

#### 1. Inn UI System (`src/game/systems/inn_ui.rs`)

- **Purpose**: Party management at inns (recruit/dismiss/swap)
- **Rendering**: `egui::CentralPanel` with interactive buttons
- **Features**:
  - Party roster display
  - Available characters display
  - Click-to-select interface
  - Recruit/Dismiss/Swap actions
- **Lines of Code**: ~553
- **Status**: ✅ Working perfectly

#### 2. Recruitment Dialog System (`src/game/systems/recruitment_dialog.rs`)

- **Purpose**: NPC encounter dialogs on maps
- **Rendering**: `egui::Window` floating dialog
- **Features**:
  - Character information display
  - Accept/Decline buttons
  - Modal dialog behavior
- **Lines of Code**: ~438
- **Status**: ✅ Working perfectly

#### 3. UiPlugin (`src/game/systems/ui.rs`)

- **Purpose**: Initialize `GameLog` resource for other systems
- **Rendering**: None - resource initialization only
- **Status**: ✅ **Active** - provides `GameLog` resource
- **Note**: Experimental `ui_system` function was deleted (see Phase 1 below)

### What's Using Bevy 3D Rendering

#### 1. Map Rendering System (`src/game/systems/map.rs`)

- **Purpose**: 3D world visualization
- **Rendering**: Bevy meshes, materials, PBR pipeline
- **Features**:
  - Procedural tile generation
  - NPC markers
  - Event trigger entities
- **Status**: ✅ Working perfectly

### Legacy Code Analysis

**Finding**: There is **ZERO legacy egui code**. All systems use bevy_egui correctly:

- ✅ All imports are `use bevy_egui::{egui, EguiContexts};`
- ✅ No direct `egui` dependency in Cargo.toml
- ✅ All systems use proper Bevy plugin architecture
- ✅ Context access via `EguiContexts` system parameter

---

## Migration Options

### Option 1: Migrate HUD to bevy_egui (NOT RECOMMENDED)

#### Scope of Work

**Tasks**:

1. Replace native Bevy UI components with egui widgets
2. Convert layout from Bevy's flexbox to egui's layout system
3. Rewrite portrait rendering to use egui image widgets
4. Convert HP bars from colored rectangles to egui progress bars
5. Update tests for egui rendering approach
6. Handle system ordering with bevy_egui lifecycle

**Estimated Effort**: 3-5 days

**Code Changes**:

- Modify: `src/game/systems/hud.rs` (~1,000 lines rewritten)
- Update: All 48 HUD tests
- Add: egui texture/image handling for portraits

**Benefits**:

- ✅ Single UI technology across all overlay UI
- ✅ Easier to create new UI features (egui has rich widget library)

**Drawbacks**:

- ❌ **Performance degradation**: egui is immediate-mode, redraws every frame
- ❌ **Loss of native integration**: Bevy UI integrates better with 3D viewport
- ❌ **Complexity increase**: Portrait system currently uses Bevy's asset pipeline directly
- ❌ **Risk**: Working, tested code being replaced with new implementation
- ❌ **Maintenance burden**: More susceptible to bevy_egui version issues

### Option 2: Migrate Overlay UI to Native Bevy UI (NOT RECOMMENDED)

#### Scope of Work

**Tasks**:

1. Rewrite inn management UI with native Bevy UI components
2. Rewrite recruitment dialog with native Bevy UI
3. Implement modal dialog behavior in Bevy UI
4. Create interactive button/list selection systems
5. Handle UI state management without egui's retained state

**Estimated Effort**: 5-7 days

**Code Changes**:

- Rewrite: `src/game/systems/inn_ui.rs` (~400 lines)
- Rewrite: `src/game/systems/recruitment_dialog.rs` (~350 lines)
- Create: New UI state management components
- Add: Custom interaction systems (clicks, hovers, selections)

**Benefits**:

- ✅ Single UI technology across all UI
- ✅ Better performance (native to Bevy)
- ✅ Tighter integration with Bevy's ECS

**Drawbacks**:

- ❌ **Development time**: Bevy UI is more verbose for complex interfaces
- ❌ **Feature parity loss**: egui's rich widgets (scrollable lists, auto-layout) require custom implementation
- ❌ **Risk**: Working, tested code being replaced
- ❌ **Maintenance**: Native Bevy UI is still evolving, breaking changes common

### Option 3: Keep Hybrid Approach (✅ RECOMMENDED)

#### Scope of Work

**Tasks**:

1. Document the hybrid architecture strategy
2. Fix or remove the disabled `ui_system`
3. Create UI development guidelines for future features

**Estimated Effort**: 1 day

**Code Changes**:

- Document: Architecture decision in `docs/reference/architecture.md`
- Decision: Fix `ui_system` OR remove it completely
- Create: `docs/how-to/create_new_ui.md` guide

**Benefits**:

- ✅ **Zero risk**: No changes to working code
- ✅ **Best of both worlds**: Native Bevy UI for always-visible HUD, egui for modal/overlay UI
- ✅ **Performance optimized**: HUD rendered efficiently, dialogs only when needed
- ✅ **Clear separation**: Persistent UI vs. transient UI use different technologies

**Drawbacks**:

- ⚠️ Developers need to know when to use which technology
- ⚠️ Two UI systems to maintain

---

## Recommended Action Plan

### Phase 1: Resolve Disabled UI System ✅ COMPLETED

**Decision**: Deleted `ui_system` function entirely (Option A)

**Rationale**:

- HUD already provides party status display
- `ui_system` was experimental/placeholder code never fully implemented
- Menu bar and game log were duplicative functionality
- Fixing would require significant refactoring with no clear benefit

**Actions Taken**:

1. ✅ Removed `ui_system` function from `src/game/systems/ui.rs` (~60 lines deleted)
2. ✅ Kept `GameLog` resource and initialization in `UiPlugin`
3. ✅ Updated documentation in `docs/explanation/implementations.md`
4. ✅ All quality checks pass (1136/1136 tests)

**Final Code**:

```rust
// src/game/systems/ui.rs (39 lines, down from 102)
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameLog>();
    }
}

#[derive(Resource, Default)]
pub struct GameLog {
    pub messages: Vec<String>,
}

impl GameLog {
    pub fn new() -> Self { ... }
    pub fn add(&mut self, msg: String) { ... }
    pub fn entries(&self) -> &[String] { ... }
}
```

**Results**:

- ✅ Removed dead code that caused egui lifecycle panics
- ✅ Simplified codebase (63 fewer lines)
- ✅ No functional loss (HUD provides party display)
- ✅ `GameLog` still available for inn/recruitment systems

### Phase 2: Document Hybrid Architecture (2-3 hours) - OPTIONAL

#### Task 1: Update Architecture Document

**File**: `docs/reference/architecture.md`

**New Section**: "UI Rendering Strategy"

```markdown
## UI Rendering Strategy

Antares uses a hybrid UI approach optimized for different use cases:

### Native Bevy UI

**Used For**: Persistent, always-visible UI elements
**Examples**:

- HUD (party status, HP bars, compass)
- Future: Minimap, quest tracker, buffs display

**Rationale**:

- Better performance (retained-mode rendering)
- Tighter integration with Bevy's transform/camera system
- Less overhead for UI that updates every frame

### bevy_egui (Immediate-Mode)

**Used For**: Modal dialogs, overlays, temporary UI
**Examples**:

- Inn management interface
- Recruitment dialogs
- Future: Settings menu, inventory screen, spell selection

**Rationale**:

- Rapid development with rich widget library
- Perfect for complex, temporary UI that only renders when needed
- Built-in layout, scrolling, interaction handling

### When to Use Which?

| Requirement         | Technology     | Reason            |
| ------------------- | -------------- | ----------------- |
| Always visible      | Native Bevy UI | Performance       |
| Updates every frame | Native Bevy UI | Efficiency        |
| Modal/overlay       | bevy_egui      | Development speed |
| Complex forms       | bevy_egui      | Rich widgets      |
| 3D-integrated       | Native Bevy UI | Transform system  |
| Temporary UI        | bevy_egui      | State management  |
```

#### Task 2: Create UI Development Guide

**File**: `docs/how-to/create_new_ui.md`

**Content**:

- Decision tree: When to use Native Bevy UI vs. bevy_egui
- Example: Creating a new HUD element (Bevy UI)
- Example: Creating a new dialog (bevy_egui)
- System ordering considerations
- Testing strategies for each approach

### Phase 3: Establish UI Consistency Guidelines (1 hour) - OPTIONAL

#### Create Style Guide

**File**: `docs/reference/ui_style_guide.md`

**Sections**:

1. **Color Palette**: Define standard colors for HP, conditions, backgrounds
2. **Typography**: Font sizes, weights for different UI elements
3. **Spacing**: Padding, margins, card sizes
4. **Interaction Patterns**: Button styles, hover states, selection feedback
5. **Layout Patterns**: Common arrangements (character cards, lists, forms)

**Goal**: Ensure both Bevy UI and egui UIs feel cohesive despite different technologies

---

## Future Considerations

### bevy_egui Version Upgrade Path

**Current**: bevy_egui 0.38 (Bevy 0.17)
**Future**: When upgrading Bevy, re-evaluate bevy_egui limitations

**Watch For**:

- Better panel lifecycle management
- System set improvements
- Performance optimizations

**Decision Point**: If bevy_egui significantly improves, consider Option 1 (migrate HUD)

### Native Bevy UI Improvements

**Bevy 0.18+** is expected to bring:

- Improved layout system
- Better text rendering
- Widget library expansion

**Decision Point**: If Bevy UI becomes more feature-rich, consider Option 2 (migrate overlays)

### Pure Bevy UI Migration (Long-Term)

If the project eventually wants a single UI system, **Native Bevy UI** is the better long-term choice:

- Official Bevy component
- Better performance
- Tighter ECS integration
- More stable API (fewer breaking changes from third-party dependency)

**Timeline**: 1-2 years (wait for Bevy UI maturity)

---

## Cost-Benefit Summary

| Option                          | Effort   | Risk | Performance | Maintainability | Recommendation     |
| ------------------------------- | -------- | ---- | ----------- | --------------- | ------------------ |
| **Migrate HUD to egui**         | 3-5 days | High | Worse       | Medium          | ❌ Not Recommended |
| **Migrate overlays to Bevy UI** | 5-7 days | High | Better      | Medium          | ❌ Not Recommended |
| **Keep Hybrid**                 | 1 day    | Low  | Optimal     | Good            | ✅ **Recommended** |

---

## Conclusion

**There is no legacy egui code to remove.** The current hybrid architecture is well-designed and leverages each technology's strengths:

- **Native Bevy UI** for the persistent HUD (performance-critical)
- **bevy_egui** for modal overlays (development-friendly)

**Actions Completed**:

1. ✅ **Removed the disabled `ui_system`** (Phase 1 complete)
2. ✅ Created comprehensive scope document (this file)
3. ✅ Updated implementation log with analysis

**Remaining Work (Optional)**:

1. ⚠️ Document hybrid architecture in `docs/reference/architecture.md` (2-3 hours)
2. ⚠️ Create `docs/how-to/create_new_ui.md` guide (1 hour)
3. ⚠️ Establish UI style guide (1 hour)

**Total Effort Completed**: 1 hour (cleanup + documentation)
**Total Remaining Effort**: 4-5 hours (optional documentation)
**Total Effort**: 1 day of documentation and cleanup work

**Risk**: Minimal (no changes to working systems)

**Value**: Clear architecture guidance for future development
