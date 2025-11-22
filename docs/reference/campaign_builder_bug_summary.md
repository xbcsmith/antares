# Campaign Builder Critical Bugs - Quick Reference

**Status**: 🔴 CRITICAL - Blocks user adoption
**Date**: 2025-01-25
**Priority**: P0 - Fix before Phase 3B

---

## Three Critical Bugs Identified

### Bug #1: Items and Monsters Not Saved 💾

**Symptom**: Users add items/monsters → Save campaign → Close → Reopen → Data is gone

**Root Cause**: `do_save_campaign()` only saves campaign.ron metadata, not data files

**Fix Location**: `sdk/campaign_builder/src/main.rs:1105-1128`

**Fix**: Call `save_items()`, `save_monsters()`, `save_spells()`, `save_quests()`, `save_dialogues_to_file()` inside `do_save_campaign()`

**Test**: Add item → Save → Close app → Reopen → Verify item persists

---

### Bug #2: UI ID Clashes Between Tabs 🎛️

**Symptom**: Dropdowns freeze/stop working when switching between Items/Monsters/Spells tabs

**Root Cause**: egui combo boxes use `ComboBox::from_label()` with same labels, causing ID collisions

**Fix Locations**:
- Items tab: `src/main.rs:~2245`
- Spells tab: `src/main.rs:~3116`
- Monsters tab: `src/main.rs:~3529`

**Fix**: Replace `from_label("text")` with `from_id_salt("unique_id_string")`

**Example**:
```rust
// WRONG:
egui::ComboBox::from_label("Filter")

// CORRECT:
egui::ComboBox::from_id_salt("items_filter_combo")
```

**Test**: Click Items dropdown → Switch to Monsters → Back to Items → Verify dropdown still works

---

### Bug #3: Map Terrain and Wall Reset Each Other 🗺️

**Symptom**: Select Grass terrain → Select Normal wall → Grass selection disappears

**Root Cause**: `EditorTool` enum can only hold one tool: `PaintTerrain(type)` OR `PaintWall(type)`

**Fix Location**: `sdk/campaign_builder/src/map_editor.rs:47-62` (enum), `978-1106` (UI)

**Fix Strategy**:
1. Add `selected_terrain: TerrainType` field to `MapEditorState`
2. Add `selected_wall: WallType` field to `MapEditorState`
3. Change enum to `EditorTool::PaintTile` (no payload)
4. Paint tiles with BOTH terrain AND wall

**Test**: Select Grass + Normal wall → Paint tile → Verify tile has BOTH properties

---

## Quick Fix Checklist

- [ ] **Bug #1**: Update `do_save_campaign()` to save all data files
- [ ] **Bug #2**: Replace `from_label()` with `from_id_salt()` in all combo boxes
- [ ] **Bug #3**: Refactor map editor to separate terrain/wall selections
- [ ] Run unit tests: `cd sdk/campaign_builder && cargo test --lib`
- [ ] Run clippy: `cargo clippy --all-targets -- -D warnings`
- [ ] Run manual tests from `docs/how-to/test_campaign_builder_ui.md`
- [ ] Verify all three bugs are fixed
- [ ] Update `implementations.md` to mark bugs as resolved

---

## Documentation

| Document | Purpose |
|----------|---------|
| `docs/how-to/fix_campaign_builder_bugs.md` | Step-by-step fix implementation with code |
| `docs/how-to/test_campaign_builder_ui.md` | Comprehensive testing procedures (10 test suites) |
| `docs/explanation/implementations.md` | Implementation tracking (updated with bug status) |
| `docs/explanation/campaign_builder_completion_plan.md` | Roadmap (Phase 3B blocked until bugs fixed) |

---

## Impact

### Before Fixes
- ❌ Data loss on save/load
- ❌ UI freezes when switching tabs
- ❌ Cannot paint terrain + wall together
- ❌ Users frustrated, cannot use tool
- ❌ Blocks Phase 3B implementation

### After Fixes
- ✅ Data persists reliably
- ✅ UI responsive across all tabs
- ✅ Intuitive map editing
- ✅ Users can create campaigns confidently
- ✅ Ready for Phase 3B development

---

## Timeline Estimate

- **Bug #1 Fix**: 30 minutes (code) + 30 minutes (testing) = 1 hour
- **Bug #2 Fix**: 45 minutes (code) + 30 minutes (testing) = 1.25 hours
- **Bug #3 Fix**: 2 hours (refactor) + 1 hour (testing) = 3 hours
- **Regression Testing**: 2 hours (full manual suite)

**Total**: ~7-8 hours to fix and verify all three bugs

---

## Success Criteria

✅ **All three bugs fixed and verified**
✅ **Campaign save/load works 100% reliably**
✅ **No UI freezes or crashes**
✅ **Map editor allows terrain + wall painting**
✅ **All unit tests pass**
✅ **All manual tests pass**
✅ **Zero regressions**
✅ **Ready to proceed with Phase 3B**

---

**NEXT ACTION**: Apply fixes from `docs/how-to/fix_campaign_builder_bugs.md`, verify with tests, then continue Phase 3B implementation.
