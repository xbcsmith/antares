# Map Content Plan to SDK Bridge Document

## Purpose

This document explains how the Map Content Implementation Plan (v2) serves as the cornerstone for the SDK, and what minimal changes are needed to make it SDK-ready.

## TL;DR

**The Map Content Plan remains 95% unchanged.** The SDK adds optional enhancements that don't break existing functionality.

## Map Plan Status: Cornerstone Architecture

The Map Content Implementation Plan v2 is **architecturally perfect** for SDK integration because it already follows best practices:

1. ✅ Uses RON format for data files
2. ✅ Validates content with clear error messages
3. ✅ Provides CLI tool for content creation
4. ✅ Documents format specifications
5. ✅ Includes test content (starter maps)
6. ✅ Follows architecture.md patterns

**No major refactoring needed!**

## Timeline Integration

```text
Week 1-3:  Map Content Plan Phase 1-3 (EXECUTE AS-IS)
           ├─ Phase 1: Documentation & Foundation
           ├─ Phase 2: Map Builder Tool
           └─ Phase 3: Starter Content

Week 4:    Evaluate SDK viability
           └─ Decision point: Continue to full SDK?

Week 5+:   SDK Phases 1-9 (IF APPROVED)
           └─ Map Builder becomes flagship SDK tool
```

## What Changes in the Map Plan: NOTHING CRITICAL

### Phase 1: Documentation & Foundation

**Status**: ✅ Execute exactly as written

**Changes**: None

**Rationale**: Documentation and validation utilities work standalone and enhance SDK

### Phase 2: Map Builder Tool

**Status**: ✅ Execute exactly as written

**Changes**: Optional enhancement in SDK Phase 4 (week 6+)

**Original Map Builder** (from plan):
```rust
struct MapBuilder {
    map: Map,
}

impl MapBuilder {
    fn new() -> Self { ... }
    fn create_map(...) { ... }
    fn load_map(...) { ... }
    fn set_tile(...) { ... }
    fn add_event(...) { ... }
    fn save_map(...) { ... }
}
```

**SDK-Enhanced Map Builder** (optional, later):
```rust
struct MapBuilder {
    map: Map,
    content_db: Option<ContentDatabase>,  // NEW: optional SDK integration
}

impl MapBuilder {
    // All original methods stay EXACTLY the same
    fn new() -> Self { ... }
    fn create_map(...) { ... }
    // ... etc ...

    // NEW: Additional SDK-powered features
    fn validate_with_sdk(&self) { ... }  // Enhanced validation
    fn list_available_content(&self) { ... }  // Browse content
}
```

**Key Point**: Original functionality preserved, SDK features are additive.

### Phase 3: Starter Content

**Status**: ✅ Execute exactly as written

**Changes**: None

**Note**: Starter content becomes reference implementation for SDK

## SDK Enhancement Points (Added Later)

These enhancements happen **after** Map Content Plan completion:

### Enhancement 1: Smart Validation (SDK Phase 4)

**When**: Week 6 (after SDK Foundation complete)

**What**: Map Builder optionally loads `ContentDatabase` for enhanced validation

**Impact**:
- Map Builder works standalone (no SDK required)
- With SDK, provides better error messages
- Zero breaking changes

**Code Addition**:
```rust
// In MapBuilder::new()
fn new() -> Self {
    let content_db = ContentDatabase::load_core().ok();  // Try to load, ignore errors
    Self { map: Map::default(), content_db }
}
```

### Enhancement 2: ID Suggestions (SDK Phase 4)

**When**: Week 6

**What**: When user enters invalid monster/item ID, suggest valid alternatives

**Impact**: Better UX, no functional changes

**Code Addition**:
```rust
// In MapBuilder::add_event()
if let Some(db) = &self.content_db {
    if db.monsters.get_monster(monster_id).is_none() {
        println!("❌ Monster ID {} not found", monster_id);
        println!("💡 Available monsters:");
        for m in db.monsters.all_monsters().iter().take(10) {
            println!("   {} - {}", m.id, m.name);
        }
    }
}
```

### Enhancement 3: Content Browser (SDK Phase 4)

**When**: Week 6

**What**: Add `list` command to browse available content

**Impact**: New feature, doesn't affect existing commands

**Code Addition**:
```rust
// In MapBuilder::process_command()
match parts[0] {
    // ... all existing commands stay ...

    "list" => {  // NEW command
        if let Some(db) = &self.content_db {
            match parts.get(1) {
                Some(&"monsters") => { /* show monsters */ }
                Some(&"items") => { /* show items */ }
                _ => println!("Usage: list <monsters|items>"),
            }
        } else {
            println!("SDK not loaded - basic mode");
        }
    }
}
```

## File Structure Evolution

### After Map Content Plan (Week 3)

```text
antares/
├── src/domain/          # Core engine
├── data/
│   ├── items.ron
│   ├── monsters.ron
│   ├── spells.ron
│   └── maps/            # NEW: From map plan
│       ├── starter_town.ron
│       ├── starter_dungeon.ron
│       └── forest_area.ron
│
├── tools/
│   └── map-builder/     # NEW: From map plan
│
└── docs/
    └── how_to/
        └── using_map_builder.md  # NEW: From map plan
```

### After SDK Implementation (Week 10)

```text
antares/
├── src/
│   ├── domain/          # Core engine (unchanged)
│   └── sdk/             # NEW: SDK module
│
├── data/
│   ├── classes.ron      # NEW: SDK Phase 1
│   ├── races.ron        # NEW: SDK Phase 2
│   ├── items.ron
│   ├── monsters.ron
│   ├── spells.ron
│   └── maps/            # From map plan
│
├── tools/
│   ├── map-builder/     # From map plan (enhanced)
│   ├── class-editor/    # NEW: SDK Phase 5
│   ├── race-editor/     # NEW: SDK Phase 5
│   ├── item-editor/     # NEW: SDK Phase 7
│   └── campaign-validator/  # NEW: SDK Phase 6
│
└── campaigns/           # NEW: SDK Phase 8
    └── might_and_magic_1/
```

## Backward Compatibility Strategy

### Map Builder Backward Compatibility

**Goal**: Map Builder must work with or without SDK

**Implementation**:
```rust
impl MapBuilder {
    fn validate(&self) {
        match &self.content_db {
            Some(db) => self.validate_with_sdk(db),  // Enhanced validation
            None => self.validate_basic(),            // Original validation
        }
    }

    fn validate_basic(&self) {
        // Original VALID_MONSTER_IDS check from map plan
        const VALID_MONSTER_IDS: &[u8] = &[1, 2, 3, 4, 5];
        // ... original validation logic ...
    }

    fn validate_with_sdk(&self, db: &ContentDatabase) {
        // Enhanced validation using SDK
        let errors = db.validate();
        // ... show detailed errors ...
    }
}
```

**Result**:
- Works standalone (no dependencies on SDK)
- Works enhanced (with SDK installed)
- Zero breaking changes

## Testing Strategy

### Map Plan Tests (Week 1-3)

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_load_starter_town() {
        let map = Map::load("data/maps/starter_town.ron").unwrap();
        assert_eq!(map.width, 20);
        assert_eq!(map.height, 20);
    }

    #[test]
    fn test_validate_basic() {
        let builder = MapBuilder::new();
        builder.create_map("test", 10, 10);
        builder.validate_basic();  // Should pass
    }
}
```

### SDK Enhancement Tests (Week 6+)

```rust
#[cfg(test)]
mod sdk_tests {
    #[test]
    fn test_map_builder_without_sdk() {
        // Simulate SDK not available
        let builder = MapBuilder {
            map: Map::default(),
            content_db: None
        };

        // Should still work
        builder.validate();  // Falls back to basic validation
    }

    #[test]
    fn test_map_builder_with_sdk() {
        let db = ContentDatabase::load_core().unwrap();
        let builder = MapBuilder {
            map: Map::default(),
            content_db: Some(db)
        };

        // Should use enhanced validation
        builder.validate();
    }
}
```

## Documentation Updates

### During Map Plan (Week 3)

Create:
- `docs/how_to/using_map_builder.md` (as planned)
- `docs/reference/map_format.md` (as planned)

### During SDK (Week 9)

Update:
- `docs/how_to/using_map_builder.md` - Add SDK features section
- Add `docs/tutorials/campaign_creation_guide.md` - Full workflow

Example SDK enhancement to existing doc:

```markdown
# Using Map Builder

## Basic Usage (No SDK Required)

[... existing documentation ...]

## Advanced Usage (With SDK)

If you have the SDK installed, Map Builder provides enhanced features:

### Smart Validation

Validates against actual content database:
```
> validate
✅ Map validated successfully
   - All monster IDs exist
   - All item IDs exist
   - No orphaned events
```

### Content Browser

Browse available content:
```
> list monsters
Available Monsters:
  1 - Goblin (HP: 8)
  2 - Skeleton (HP: 12)
  3 - Orc (HP: 15)
  ...
```
```

## Migration Checklist

### When Starting Map Plan

- [ ] No changes needed - execute plan as written
- [ ] Focus on Map Builder functionality only
- [ ] Don't worry about SDK integration yet

### After Map Plan Completion (Week 3)

- [ ] Map Builder works standalone ✓
- [ ] Starter content loads correctly ✓
- [ ] Validation catches common errors ✓
- [ ] Documentation complete ✓
- [ ] Decision point: Proceed to SDK? ✓

### When Starting SDK (Week 4+)

- [ ] Create `src/sdk/` module
- [ ] Don't modify Map Builder yet
- [ ] Build SDK foundation first

### When Enhancing Map Builder (Week 6)

- [ ] Add `content_db: Option<ContentDatabase>` field
- [ ] Keep all existing methods unchanged
- [ ] Add new SDK-powered methods
- [ ] Add fallback for when SDK not available
- [ ] Update tests to cover both modes
- [ ] Update documentation with SDK features

## Common Questions

### Q: Do I need to change my Map Builder code for SDK?

**A**: No, not initially. Execute Map Content Plan as-is. SDK enhancements come later (week 6+) and are optional additions.

### Q: Will the Map Builder break if SDK isn't installed?

**A**: No. Map Builder works standalone. SDK features are optional enhancements that gracefully degrade if not available.

### Q: Do I need to learn SDK to use Map Builder?

**A**: No. Map Builder works perfectly without SDK knowledge. SDK just adds nice-to-have features like smart suggestions.

### Q: Can I stop after Map Content Plan?

**A**: Yes! Map Content Plan is complete and functional on its own. SDK is an optional extension, not a requirement.

### Q: What if I want only some SDK tools?

**A**: SDK is modular. You can implement just the validator, or just the class editor, without building everything.

## Decision Tree

```text
Week 3: Map Plan Complete
    │
    ├─ Is manual RON editing painful?
    │   └─ NO → Stop here, ship Map Builder as-is
    │   └─ YES → Continue
    │
    ├─ Do you want modding support?
    │   └─ NO → Stop here, ship Map Builder as-is
    │   └─ YES → Continue
    │
    ├─ Can you invest 7+ more weeks?
    │   └─ NO → Add just validator (2 weeks)
    │   └─ YES → Full SDK
    │
    └─ Proceed to SDK Phase 1
```

## Success Metrics

### Map Plan Success (Week 3)

- ✅ Map Builder creates valid RON files
- ✅ Starter maps load in game engine
- ✅ Validation catches common errors
- ✅ Documentation clear and complete

### SDK Integration Success (Week 10)

- ✅ Map Builder still works standalone
- ✅ SDK features enhance without breaking
- ✅ Backward compatibility maintained
- ✅ External user can create campaign

## Conclusion

The Map Content Implementation Plan is **perfectly positioned** to become the SDK's cornerstone because it already embodies SDK principles:

1. **Data-driven**: Uses RON format
2. **Validated**: Checks content correctness
3. **Documented**: Clear format specifications
4. **Tooled**: CLI for content creation
5. **Modular**: Clean separation of concerns

**Execute the Map Plan as-is, then evaluate SDK viability based on real experience.**

## References

- **Map Plan**: `docs/explanation/map_content_implementation_plan_v2.md`
- **SDK Plan**: `docs/explanation/sdk_implementation_plan.md`
- **SDK Summary**: `docs/explanation/sdk_transition_summary.md`
- **Architecture**: `docs/reference/architecture.md`
