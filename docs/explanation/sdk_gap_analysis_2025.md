# SDK Gap Analysis and Roadmap - January 2025

## Executive Summary

This document provides a comprehensive analysis of the current SDK implementation status against the target architecture defined in `sdk_and_campaign_architecture.md`. The analysis identifies critical gaps in the Campaign Builder GUI and game engine integration, and defines a clear roadmap (Phases 10-15) to achieve the complete modding SDK vision.

**Key Findings:**

- ✅ SDK backend infrastructure is complete and solid (Phases 0-3, 8-9)
- ⚠️ Campaign Builder GUI is 40% complete (Phase 2 foundation only)
- ❌ Data editors are non-functional placeholders (Phase 3 needed)
- ❌ Game engine cannot load campaigns (Phase 14 CRITICAL)
- 📊 Estimated 15 weeks remaining to complete full SDK vision

---

## Current State Assessment

### What's Complete ✅

#### 1. SDK Backend Infrastructure (100%)

**Location:** `src/sdk/`

All backend modules are implemented, tested, and documented:

- `campaign_loader.rs` - Campaign loading with metadata validation
- `campaign_packager.rs` - Campaign packaging for distribution (.zip creation)
- `database.rs` - Unified content database (items, spells, monsters, classes, races, quests, dialogue)
- `validation.rs` - Cross-reference validation and balance checking
- `serialization.rs` - RON format helpers and utilities
- `templates.rs` - Pre-built content templates
- `quest_editor.rs` - Quest validation and analysis helpers
- `dialogue_editor.rs` - Dialogue tree validation and analysis
- `map_editor.rs` - Map validation and content browsing
- `error_formatter.rs` - Enhanced error messages with actionable suggestions
- `cache.rs` - Performance optimization through intelligent caching
- `tool_config.rs` - Shared configuration system for SDK tools

**Status:** All modules have >80% test coverage, pass all quality gates, and are fully documented.

#### 2. Domain Layer (100%)

**Location:** `src/domain/`

Complete game systems with proper data structures:

- `quest.rs` - Quest system with stages, objectives, conditions, rewards
- `dialogue.rs` - Dialogue system with trees, nodes, choices, branching
- `classes.rs` - Data-driven class system
- `character.rs` - Character system with quest flags
- `items/` - Item system with types and effects
- `combat/` - Combat mechanics
- `magic/` - Spell system
- `world/` - World and map structures

**Status:** Architecture-compliant, properly tested, ready for use.

#### 3. Documentation (100%)

**Location:** `docs/`

Complete documentation suite per Phase 8:

- `docs/reference/sdk_api.md` (938 lines) - Complete SDK API reference
- `docs/tutorials/creating_campaigns.md` (815 lines) - Step-by-step campaign creation
- `docs/how-to/using_sdk_tools.md` (804 lines) - Tool usage guides
- `docs/explanation/modding_guide.md` (767 lines) - Comprehensive modding guide
- `campaigns/tutorial/` - Complete working example campaign

**Status:** All documentation validated, example campaign loads successfully.

#### 4. Campaign Builder Foundation (40%)

**Location:** `sdk/campaign_builder/`

Phases 0-2 complete:

- ✅ Phase 0: Framework validation (egui confirmed, works without GPU)
- ✅ Phase 1: Core campaign system backend
- ✅ Phase 2: Foundation UI
  - Full metadata editor (campaign ID, name, version, author, etc.)
  - Campaign configuration editor (starting conditions, difficulty, rules)
  - Real file I/O (save/load campaign.ron in RON format)
  - Validation UI with error/warning display
  - Unsaved changes tracking with warning dialogs
  - File structure browser

**Status:** Foundation is solid, but content editors are missing.

---

### Critical Gaps ❌

#### 1. Campaign Builder Data Editors (Phase 3) - PLACEHOLDER ONLY

**Impact:** Users cannot create game content in the GUI

**Current State:**

- `show_items_editor()` - Shows placeholder text only, no CRUD operations
- `show_spells_editor()` - Shows placeholder text only, no CRUD operations
- `show_monsters_editor()` - Shows placeholder text only, no CRUD operations

**What's Missing:**

- Add/Edit/Delete functionality for items, spells, monsters
- Search and filter capabilities
- Form-based editors with validation
- Preview panels showing formatted data
- Integration with `ContentDatabase` for loading/saving
- Real-time validation with error display

**User Impact:** Cannot create campaigns without manually editing RON files, defeating the purpose of a visual editor.

#### 2. Map Editor Integration (Phase 4) - NOT STARTED

**Impact:** Map creation requires external CLI tool

**Current State:**

- Map Builder exists as standalone CLI tool
- Not integrated into Campaign Builder GUI
- `show_maps_editor()` is placeholder only

**What's Missing:**

- Map editor component rewritten for egui
- Grid-based tile placement UI
- Event editor integrated into map view
- Map list view with thumbnail previews
- Map interconnection visualizer
- Validation with content database

**User Impact:** Workflow is disjointed, users must switch between GUI and CLI.

#### 3. Quest & Dialogue Tools (Phase 5) - NOT STARTED

**Impact:** Cannot create quests or dialogue trees

**Current State:**

- `show_quests_editor()` is placeholder only
- No dialogue tree editor exists
- Backend helpers exist but no UI

**What's Missing:**

- Visual quest designer with stage/objective builder
- Dialogue tree editor (list-based navigation)
- Quest-dialogue integration
- Prerequisite chain editor
- Condition and action configuration
- Validation integration

**User Impact:** Complex quests and branching dialogues impossible to create.

#### 4. Distribution Tools (Phase 6) - NOT STARTED

**Impact:** No workflow for testing or sharing campaigns

**Current State:**

- `CampaignPackager` exists in backend but not integrated
- No test play functionality
- No asset manager

**What's Missing:**

- Export wizard with campaign packaging
- Test play button (launches game with campaign)
- Asset manager UI for images/music/sounds
- Campaign import functionality

**User Impact:** Cannot easily share campaigns or test them in-game.

#### 5. Game Engine Integration (Phase 14) - CRITICAL GAP

**Impact:** GAME CANNOT LOAD CAMPAIGNS AT ALL

**Current State:**

- `GameState` in `src/application/mod.rs` has no campaign support
- Main game CLI has no `--campaign` flag
- Game loads hardcoded data files only
- Save games don't store campaign reference

**What's Missing:**

- `campaign: Option<Campaign>` field in `GameState`
- CLI arguments: `--campaign <id>`, `--list-campaigns`, `--validate-campaign`
- Campaign data loading (items, spells, monsters from campaign paths)
- Save game format with campaign reference
- Error handling for missing/invalid campaigns

**User Impact:** CAMPAIGNS CREATED IN CAMPAIGN BUILDER CANNOT BE PLAYED. This is the most critical gap - without Phase 14, the SDK is non-functional.

---

## Roadmap: Phases 10-15

### Phase 10: Campaign Builder GUI - Data Editors (Phase 3)

**Duration:** 2-3 weeks
**Priority:** HIGH
**Depends On:** Phase 2 complete

**Goal:** Implement full CRUD editors for Items, Spells, and Monsters

**Deliverables:**

1. Items Editor
   - Add/Edit/Delete items with type selection (Weapon, Armor, Consumable, Quest)
   - Stats editor for each item type
   - Disablement flags (class restrictions)
   - Search/filter by name, type, level
   - Preview panel with formatted display

2. Spells Editor
   - Add/Edit/Delete spells
   - School selection (Cleric/Sorcerer) and level
   - Target type and effect configuration
   - SP cost editor
   - Filter by school and level

3. Monsters Editor
   - Add/Edit/Delete monsters
   - Stats editor (HP, AC, damage, level)
   - Loot table builder with drop rates
   - Special abilities (regeneration, advancement)
   - Balance validation

4. Shared Components
   - Reusable search/filter UI
   - Sortable data tables
   - Form validation with inline errors
   - Confirmation dialogs

**Success Criteria:**

- ✅ Create/edit/delete 100+ items without touching RON files
- ✅ Search works instantly on large datasets
- ✅ Validation catches invalid references in real-time
- ✅ All quality gates pass (fmt, clippy, tests)

---

### Phase 11: Campaign Builder GUI - Map Editor Integration (Phase 4)

**Duration:** 2 weeks
**Priority:** MEDIUM
**Depends On:** Phase 10 complete

**Goal:** Rewrite map editor as egui component within Campaign Builder

**Deliverables:**

1. Map Editor Component (`map_editor_component.rs`)
   - Grid-based tile placement UI
   - Zoom and pan controls
   - Event placement and configuration
   - Map properties editor

2. Map List View
   - All maps with thumbnail previews
   - Add/Edit/Delete maps
   - Map interconnection visualizer
   - Validation integration

3. Event Editor
   - Dropdown selectors using validated content
   - Item/Monster/Spell selectors from databases
   - Quest integration

**Success Criteria:**

- ✅ Create maps entirely in GUI
- ✅ Events validate against content databases
- ✅ Map connections visualized
- ✅ No need for CLI map_builder tool

---

### Phase 12: Campaign Builder GUI - Quest & Dialogue Tools (Phase 5)

**Duration:** 2-3 weeks
**Priority:** MEDIUM
**Depends On:** Phase 10 complete

**Goal:** Implement visual quest designer and dialogue tree editor (list-based)

**Deliverables:**

1. Quest Designer
   - Quest list view with status
   - Stage editor (add/edit/delete stages)
   - Objective builder (kill, collect, visit, talk)
   - Prerequisite chain editor
   - Reward configuration

2. Dialogue Tree Editor
   - Tree list view
   - Node list (list-based navigation, not graph)
   - Choice editor with target nodes
   - Condition builder (quest flags, inventory, stats)
   - Action configuration (start quest, give items, set flags)

3. Integration
   - Quest editor can select dialogue trees
   - Dialogue editor can start/complete quests
   - Cross-reference validation

**Success Criteria:**

- ✅ Create multi-stage quests with branching
- ✅ Create dialogue trees with conditions
- ✅ Quest-dialogue integration works
- ✅ List-based UI handles moderate complexity (10-20 nodes)

**Note:** Node-graph visualization deferred to Phase 15.

---

### Phase 13: Campaign Builder GUI - Distribution Tools (Phase 6)

**Duration:** 1-2 weeks
**Priority:** LOW
**Depends On:** Phases 10-12 complete

**Goal:** Integrate campaign packaging, testing, and asset management

**Deliverables:**

1. Export Wizard
   - Multi-step campaign packaging process
   - Full validation before export
   - File selection (data, maps, assets)
   - .zip creation using `CampaignPackager`
   - Version bumping helper

2. Test Play Integration
   - Launch game button with `--campaign` flag
   - Output capture (game logs in GUI)
   - Quick test workflow (save → launch → return)

3. Asset Manager
   - Asset browser (files in campaign directory)
   - Upload assets (drag-and-drop or file picker)
   - Organize into subdirectories
   - Preview (images, text files)
   - Validation (check referenced assets exist)

4. Campaign Import
   - Import .zip campaigns
   - Extract to campaigns directory
   - Validation on import

**Success Criteria:**

- ✅ Export campaign as distributable .zip
- ✅ Test play launches game successfully
- ✅ Asset manager handles images/music/sounds
- ✅ Imported campaigns work correctly

---

### Phase 14: Game Engine Campaign Integration (CRITICAL)

**Duration:** 1-2 weeks
**Priority:** CRITICAL
**Depends On:** SDK backend (Phase 3), Campaign Builder Phase 2

**Goal:** Enable game engine to load and play custom campaigns

**Rationale:** Without this phase, campaigns created in Campaign Builder CANNOT BE PLAYED. This is the most critical gap in the SDK.

**Deliverables:**

1. GameState Integration (`src/application/mod.rs`)
   - Add `campaign: Option<Campaign>` field
   - Modify `new_game()` to accept campaign parameter
   - Use campaign config for starting conditions

2. Main Game CLI
   - Add CLI argument parser (clap)
   - Support `--campaign <id>` flag
   - Support `--list-campaigns` flag
   - Support `--validate-campaign <id>` flag
   - Use `CampaignLoader` to load campaign

3. Campaign Data Loading
   - Load items from `campaign.items_file`
   - Load spells from `campaign.spells_file`
   - Load monsters from `campaign.monsters_file`
   - Load classes from `campaign.classes_file`
   - Load races from `campaign.races_file`
   - Load maps from `campaign.maps_dir`
   - Fallback to core content if no campaign

4. Save Game Format
   - Add `campaign_reference: Option<CampaignReference>`
   - Store campaign ID, version, name
   - Verify campaign on load
   - Handle missing/changed campaigns

5. Error Handling
   - User-friendly messages for missing campaigns
   - List available campaigns on error
   - Campaign validation error display
   - Save game campaign mismatch warnings

**Success Criteria:**

- ✅ Launch game with `antares --campaign tutorial`
- ✅ Campaign config applied (starting gold, map, position)
- ✅ Campaign data loaded correctly
- ✅ Save games preserve campaign reference
- ✅ Loaded games restore campaign
- ✅ Core game works without campaign (backward compatible)

**CRITICAL:** This phase unblocks the entire SDK. Without it, the SDK is a content creation tool with no way to use the content.

---

### Phase 15: Polish & Advanced Features (Phase 7)

**Duration:** 2-3 weeks
**Priority:** LOW
**Depends On:** Phases 10-14 complete

**Goal:** User experience improvements and advanced features

**Deliverables:**

1. Undo/Redo System
   - Command pattern for all operations
   - Undo/redo stack (max 50 actions)
   - Keyboard shortcuts (Ctrl+Z, Ctrl+Y)
   - Works in all editors

2. Template System
   - Pre-built templates (items, monsters, quests, dialogue)
   - User can save custom templates
   - Template browser with preview
   - "New from Template" in all editors

3. Node-Graph Dialogue Visualizer
   - Visual node graph (upgrade from list-based)
   - Drag-and-drop node positioning
   - Auto-layout algorithm
   - Export as image

4. Advanced Validation
   - Balance analyzer (party power vs monsters)
   - Loot economy checker
   - Quest dependency graph
   - Unreachable content detector
   - Difficulty curve analyzer

5. Accessibility
   - Keyboard navigation for all editors
   - High contrast theme
   - Font size adjustment
   - Tooltips for all controls

6. Performance
   - Lazy loading for large lists
   - Virtual scrolling
   - Background validation
   - Incremental saving

**Success Criteria:**

- ✅ Undo/redo works reliably
- ✅ Templates speed up content creation
- ✅ Node-graph handles 50+ nodes
- ✅ Large campaigns (1000+ items) load in <2 seconds
- ✅ Keyboard-only navigation works

---

## Timeline and Critical Path

### Completed (16 weeks / ~4 months)

- ✅ Phases 0-3: Backend SDK infrastructure
- ✅ Phase 8: Documentation
- ✅ Phase 9: CLI tools integration
- ✅ Campaign Builder Phases 0-2: Foundation UI

### Remaining (15 weeks / ~4 months)

| Phase | Description | Weeks | Priority | Status |
|-------|-------------|-------|----------|--------|
| Phase 10 | Data Editors | 2-3 | HIGH | 🔲 Not Started |
| Phase 11 | Map Integration | 2 | MEDIUM | 🔲 Not Started |
| Phase 12 | Quest/Dialogue | 2-3 | MEDIUM | 🔲 Not Started |
| Phase 13 | Distribution | 1-2 | LOW | 🔲 Not Started |
| Phase 14 | Game Integration | 1-2 | **CRITICAL** | 🔲 Not Started |
| Phase 15 | Polish | 2-3 | LOW | 🔲 Not Started |

**Total:** 31 weeks (~8 months from project start)

### Critical Path (Recommended Order)

1. **Phase 10** (Data Editors) - Foundation for all content creation
2. **Phase 14** (Game Integration) - Unblocks playable campaigns **[CRITICAL]**
3. **Phase 12** (Quest/Dialogue) - Completes content creation loop
4. **Phase 11** (Map Integration) - Can work in parallel with Phase 12
5. **Phase 13** (Distribution) - Enables sharing
6. **Phase 15** (Polish) - Improves UX but not blocking

**Alternative Path (Fastest to Playable):**

1. Phase 10 (Data Editors) - 3 weeks
2. Phase 14 (Game Integration) - 2 weeks
3. **MILESTONE:** Campaigns playable (5 weeks total)
4. Phases 11-13 (Complete SDK) - 6 weeks
5. Phase 15 (Polish) - 3 weeks

---

## Key Decisions

### Decision 1: Phase Priority

**Question:** Implement Campaign Builder phases 3-6 sequentially, or prioritize game integration?

**Decision:** Phase 10 (Data Editors) → Phase 14 (Game Integration) → Phases 11-13

**Rationale:**
- Need functional data editors before campaigns are useful
- Game integration unblocks playability (critical milestone)
- Map/Quest/Distribution can follow

### Decision 2: Map Editor Strategy

**Question:** Rewrite CLI map_builder as egui component, or wrap existing CLI tool?

**Decision:** Rewrite as egui component within Campaign Builder

**Rationale:**
- Better UX with integrated workflow
- Consistent with other editors
- Avoids subprocess complexity and platform issues
- Enables preview panel and validation

### Decision 3: Dialogue Tree UI Approach

**Question:** Simple list-based editor or full node-graph visualization?

**Decision:** List-based in Phase 12, node-graph in Phase 15

**Rationale:**
- List-based is faster to implement (MVP)
- Matches architecture Phase 5 scope
- Node-graph is polish, not blocking
- Can upgrade UI without changing backend

---

## Success Metrics

### Phase 10 Complete (Data Editors)

- [ ] User creates 10+ items, 10+ spells, 5+ monsters entirely in GUI
- [ ] Zero manual RON file editing required
- [ ] Validation catches all invalid references
- [ ] Search works on 100+ items instantly
- [ ] All quality gates pass

### Phase 14 Complete (Game Integration) - CRITICAL MILESTONE

- [ ] User exports campaign from Campaign Builder
- [ ] User launches `antares --campaign tutorial`
- [ ] Game loads with tutorial campaign data
- [ ] Starting conditions from campaign applied
- [ ] User saves game, campaign reference preserved
- [ ] User loads saved game, campaign restored
- [ ] Error messages guide user if campaign missing

### All Phases Complete (Full SDK Vision)

- [ ] Complete campaign creation workflow in GUI (no CLI needed)
- [ ] Campaigns playable in game engine
- [ ] Distribution and sharing workflow functional
- [ ] Advanced features working (undo/redo, templates, visualizers)
- [ ] Documentation updated with new phases
- [ ] Example campaign demonstrates all features
- [ ] SDK vision from `sdk_and_campaign_architecture.md` achieved

---

## Impact Assessment

### For Users (Campaign Creators)

**Current State:**
- Can define campaign metadata
- Must manually edit RON files for content
- Cannot test campaigns (game doesn't load them)
- Documentation exists but tools don't match

**After Phase 10:**
- ✅ Can create items, spells, monsters in GUI
- ❌ Still can't test campaigns

**After Phase 14:**
- ✅ Can create and TEST campaigns
- ✅ Full content creation loop working
- ⚠️ Map/quest/dialogue still manual or limited

**After Phase 15:**
- ✅ Complete GUI workflow
- ✅ Advanced features for power users
- ✅ Distribution and sharing easy
- ✅ Professional-grade modding SDK

### For The Project

**Current State:**
- Strong backend foundation
- Good documentation
- Incomplete user-facing tools

**After Completing Phases 10-15:**
- ✅ Competitive with commercial modding SDKs
- ✅ Lowers barrier to entry for modders
- ✅ Enables community content creation
- ✅ Extends game lifespan through mods
- ✅ Establishes Antares as modding-friendly platform

---

## Risk Assessment

### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| egui limitations for complex editors | Medium | Medium | Prototype first, fallback options available |
| Large campaign performance | Medium | High | Phase 9 caching already implemented |
| Game integration breaking changes | Low | High | Backward compatibility required, thorough testing |
| Cross-platform issues | Low | Medium | Testing on Linux/Mac/Windows |

### Project Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Scope creep in Phase 15 | High | Medium | Define MVP clearly, defer nice-to-haves |
| Phase 14 delayed | Medium | Critical | Prioritize early, allocate extra time |
| User testing reveals UX issues | Medium | Medium | Iterate on feedback, Phase 15 for polish |

---

## References

- **Updated Plan:** `docs/explanation/sdk_implementation_plan.md` (phases 10-15 added)
- **Architecture:** `docs/explanation/sdk_and_campaign_architecture.md`
- **Implementation Log:** `docs/explanation/implementations.md`
- **Campaign Builder Status:** `sdk/campaign_builder/README.md`
- **Framework Decision:** `sdk/campaign_builder/FRAMEWORK_DECISION.md`
- **Core Architecture:** `docs/reference/architecture.md`

---

## Conclusion

The SDK implementation is **50% complete** with a solid backend foundation. The critical gaps are:

1. **Data editors in Campaign Builder** (Phase 10) - Users can't create content
2. **Game engine integration** (Phase 14) - Campaigns can't be played **[MOST CRITICAL]**
3. **Quest/Dialogue tools** (Phase 12) - Complex content impossible
4. **Map editor integration** (Phase 11) - Workflow disjointed
5. **Distribution tools** (Phase 13) - Sharing difficult

**Recommended Next Steps:**

1. Start Phase 10 (Data Editors) immediately - 3 weeks to functional content creation
2. Follow with Phase 14 (Game Integration) - 2 weeks to playable campaigns
3. **MILESTONE:** At week 5, SDK becomes functional (campaigns can be created AND played)
4. Continue with Phases 11-13 to complete toolkit
5. Polish with Phase 15 for professional-grade UX

**Timeline:** 15 weeks (~4 months) to complete full SDK vision from current state.

**Critical Success Factor:** Phase 14 must be completed for SDK to be functional. Without game integration, Campaign Builder is just a metadata editor - campaigns cannot be played.

---

**Document Version:** 1.0
**Last Updated:** January 2025
**Status:** Current
**Next Review:** After Phase 10 completion
