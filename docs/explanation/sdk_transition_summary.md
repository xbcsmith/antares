# SDK Transition Plan Summary

## Executive Summary

This document summarizes the transition from the Map Content Implementation Plan to the full SDK implementation. The Map Content Plan serves as the cornerstone of the SDK, with additional tools and infrastructure built on top.

## Strategic Vision

**Current State**: Antares is an MM1-inspired RPG with some data-driven content (items, monsters, spells).

**Target State**: Antares becomes a general-purpose RPG engine with SDK tooling that enables modders to create custom campaigns without touching Rust code.

**Key Principle**: Map Content Plan executes first, unchanged. SDK enhances rather than replaces.

## Implementation Sequence

### Phase 0: Foundation (3 weeks)

**Execute**: `map_content_implementation_plan_v2.md` Phases 1-3

**Deliverables**:
- Map RON format documentation
- Map Builder CLI tool (MVP)
- Starter content (town, dungeon, forest)
- Map validation utility

**Status**: Prerequisite for all SDK work

### Phase 1-2: Data-Driven Core (8-11 days)

**Phase 1**: Classes (5-7 days)
- Migrate `Class` enum to `data/classes.ron`
- Create `ClassDatabase` loader
- Refactor progression, magic, items systems

**Phase 2**: Races (3-4 days)
- Migrate `Race` enum to `data/races.ron`
- Create `RaceDatabase` loader
- Apply stat modifiers during character creation

### Phase 3-4: SDK Infrastructure (6-8 days)

**Phase 3**: SDK Foundation (4-5 days)
- `ContentDatabase` - unified content access
- `Validator` - cross-reference checking
- Serialization helpers
- Template functions

**Phase 4**: Enhanced Map Builder (2-3 days)
- Integrate SDK validation into existing tool
- Add ID suggestions on errors
- Add content browser (`list` command)

### Phase 5-7: Content Editors (8-11 days)

**Phase 5**: Class/Race Editor (3-4 days)
- Interactive CLI for class definitions
- Interactive CLI for race definitions

**Phase 6**: Campaign Validator (2-3 days)
- Standalone validation tool
- CI/CD integration support

**Phase 7**: Item Editor (3-4 days)
- Interactive item creation/editing
- Class restriction selector

### Phase 8-9: Documentation & Polish (7-9 days)

**Phase 8**: Documentation (4-5 days)
- SDK API reference
- Campaign creation guide
- Tool usage guides
- Example campaign

**Phase 9**: Integration & Polish (3-4 days)
- Cross-tool integration
- Error message improvements
- Performance optimization
- Final QA

## Timeline

| Milestone | Duration | Cumulative |
|-----------|----------|------------|
| Map Content Plan | 3 weeks | 3 weeks |
| Data-Driven Core | 8-11 days | ~5 weeks |
| SDK Infrastructure | 6-8 days | ~6.5 weeks |
| Content Editors | 8-11 days | ~8 weeks |
| Docs & Polish | 7-9 days | ~10 weeks |
| **Total** | **~10 weeks** | **10 weeks** |

*Assumes part-time development (~20 hours/week)*

## Key Architectural Changes

### What Stays the Same

- Core game engine logic
- Combat system
- World system
- Existing data formats (items, monsters, spells)
- Type alias system (`ItemId`, `SpellId`, etc.)

### What Changes

**Before**:
```rust
pub enum Class {
    Knight, Paladin, Archer, Cleric, Sorcerer, Robber,
}

pub struct Character {
    pub class: Class,  // Enum reference
    // ...
}
```

**After**:
```rust
pub type ClassId = String;

pub struct Character {
    pub class_id: ClassId,  // String reference
    // ...
}

// data/classes.ron contains definitions
```

### Breaking Changes

- `Character::class` becomes `Character::class_id`
- Functions taking `Class` parameter now take `&ClassId` + `&ClassDatabase`
- Similar changes for `Race` → `RaceId`

### Migration Strategy

- Provide conversion utilities
- Document all breaking changes
- Update all tests incrementally
- Support both APIs temporarily (if needed)

## Tool Architecture

```text
antares/
├── src/
│   ├── domain/          # Core game engine (unchanged)
│   ├── application/     # Use cases (unchanged)
│   └── sdk/             # NEW: SDK module
│       ├── database.rs  # ContentDatabase
│       ├── validation.rs
│       ├── serialization.rs
│       └── templates.rs
│
├── data/                # Game content (RON files)
│   ├── classes.ron      # NEW: Class definitions
│   ├── races.ron        # NEW: Race definitions
│   ├── items.ron        # Existing
│   ├── monsters.ron     # Existing
│   ├── spells.ron       # Existing
│   └── maps/            # From Map Content Plan
│
├── tools/               # NEW: SDK tools
│   ├── map-builder/     # From Map Content Plan (enhanced)
│   ├── class-editor/    # NEW
│   ├── race-editor/     # NEW
│   ├── item-editor/     # NEW
│   └── campaign-validator/  # NEW
│
└── campaigns/           # NEW: Example campaigns
    ├── might_and_magic_1/   # Reference content
    └── example_tutorial/    # Learning example
```

## Success Criteria

### Technical

- ✅ All cargo quality checks pass (test, clippy, fmt)
- ✅ >80% code coverage for SDK modules
- ✅ Campaign loads in <2 seconds (100 maps)
- ✅ Validation completes in <5 seconds (100 maps)
- ✅ No behavioral changes to core engine

### User Experience

- ✅ External tester creates campaign using only docs
- ✅ Non-programmer can use all tools
- ✅ Documentation rated "clear and complete"
- ✅ Tools prevent common mistakes
- ✅ Error messages provide actionable guidance

### Strategic

- ✅ Modding enabled without Rust knowledge
- ✅ Multiple campaigns possible with same engine
- ✅ Community can create custom content
- ✅ Engine separated from game content

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Performance degradation | Medium | Profile early, optimize hot paths, cache lookups |
| Complex refactoring introduces bugs | High | Comprehensive tests, incremental changes |
| Tools too hard to use | High | Early user testing, iterate on UX |
| Documentation lags implementation | Medium | Write docs as you build, not after |
| Scope creep | Medium | Strict phase boundaries, defer enhancements |

## Decision Points

### After Map Content Plan (Week 3)

**Question**: Proceed with full SDK or minimal enhancements?

**Go/No-Go Criteria**:
- Map Builder workflow validates architecture ✓
- Manual RON editing is painful ✓
- Interest in modding support ✓
- Bandwidth for 7-8 more weeks ✓

### After Data-Driven Classes (Week 5)

**Question**: Continue with full tool suite or stop at validation?

**Evaluate**:
- Refactoring impact acceptable?
- Database pattern working well?
- Time investment justified?

### After SDK Foundation (Week 6)

**Question**: Build all editors or focus on polish?

**Options**:
- **Full SDK**: All tools (Phases 5-7)
- **Minimal SDK**: Just validator + docs
- **Targeted SDK**: Map Builder + validator only

## Post-SDK Roadmap

**Potential Future Work** (not in this plan):
- GUI-based editors (Tauri/egui)
- Lua/Rhai scripting for events
- Online mod repository
- Visual map editor with graphics
- Balance analyzer tools
- Procedural content generators

## References

- **Full Plan**: `docs/explanation/sdk_implementation_plan.md`
- **Map Plan**: `docs/explanation/map_content_implementation_plan_v2.md`
- **Architecture**: `docs/reference/architecture.md`
- **Agent Rules**: `AGENTS.md`

## Conclusion

The SDK implementation builds naturally on the Map Content Plan foundation. By maintaining the map tool as the cornerstone and adding complementary editors, Antares transforms from a specific game into a flexible RPG engine while preserving the core gameplay systems.

**Key Takeaway**: Execute Map Content Plan first to validate the architecture with real content creation. Only after that foundation is solid should SDK tooling be built on top.

**Next Action**: Complete Map Content Plan Phases 1-3, then reassess SDK viability based on actual experience.
