# Creatures Editor Issue - Documentation Index

## Quick Navigation

This is your entry point to understanding and fixing the creatures editor loading issue.

### For Different Audiences

#### 🚀 "Just Tell Me How to Fix It"
→ Read: **`docs/how-to/fix_creatures_editor_loading.md`**
- Step-by-step implementation guide
- Code snippets ready to copy-paste
- Troubleshooting section included
- ~10 minutes to read

#### 🎓 "I Want to Understand the Problem"
→ Read: **`docs/explanation/CREATURES_EDITOR_ISSUE_SUMMARY.md`**
- Executive summary
- Root cause explanation
- Why it matters
- ~5 minutes to read

#### 📚 "Show Me Everything"
→ Read in this order:
1. `CREATURES_EDITOR_ISSUE_SUMMARY.md` (what's wrong)
2. `creatures_loading_pattern_comparison.md` (how game does it)
3. `creatures_editor_loading_issue.md` (detailed analysis)
4. `creatures_loading_diagrams.md` (visual explanations)
5. `fix_creatures_editor_loading.md` (implementation)
- ~30 minutes total

#### 🔍 "I'm a Visual Learner"
→ Read: **`docs/explanation/creatures_loading_diagrams.md`**
- 6 detailed ASCII diagrams
- Before/after flow charts
- Data structure relationships
- Error handling scenarios
- ~15 minutes to read

---

## Documentation Files

### Explanation Documents (Understanding)

Located in: `docs/explanation/`

#### 1. **CREATURES_EDITOR_ISSUE_SUMMARY.md** ⭐ START HERE
- **Length**: ~5 min read
- **What it covers**:
  - One-sentence problem statement
  - Root cause explanation
  - Why the architecture exists
  - Implementation checklist
  - Success criteria
- **Best for**: Quick overview before diving deeper

#### 2. **creatures_editor_loading_issue.md** 📖 DEEP DIVE
- **Length**: ~15 min read
- **What it covers**:
  - Complete architecture analysis
  - Current vs proposed implementation
  - Data structures explained
  - Solution architecture (3 phases)
  - Implementation plan with code examples
  - Testing strategy
  - Migration path
- **Best for**: Understanding every detail before coding

#### 3. **creatures_loading_pattern_comparison.md** 🔄 PATTERN ANALYSIS
- **Length**: ~15 min read
- **What it covers**:
  - Game implementation (correct pattern)
  - Campaign builder implementation (broken)
  - Detailed code comparison
  - Data structure differences
  - File organization comparison
  - Step-by-step loading walkthrough
  - Why the pattern exists
- **Best for**: Understanding game architecture and why fix is needed

#### 4. **creatures_loading_diagrams.md** 📊 VISUAL EXPLANATIONS
- **Length**: ~15 min read
- **What it covers**:
  - 6 ASCII diagrams:
    1. Two-step registry loading
    2. File structure comparison
    3. Data type relationships
    4. Campaign builder fix flow
    5. Save operation before/after
    6. Error handling scenarios
- **Best for**: Visual learners, quick reference

---

### How-To Documents (Implementation)

Located in: `docs/how-to/`

#### 1. **fix_creatures_editor_loading.md** 🛠️ IMPLEMENTATION GUIDE
- **Length**: ~10 min read
- **What it covers**:
  - Step 1: Update `load_creatures()` function
  - Step 2: Update `save_creatures()` function
  - Step 3: Add required imports
  - Step 4: Testing instructions
  - What the fix does (before/after)
  - File structure after fix
  - Troubleshooting section
  - Code quality checks
- **Best for**: Implementing the fix

---

## The Issue at a Glance

### Problem
Campaign builder tries to parse `creatures.ron` (a lightweight registry) as if it contained full creature definitions.

### Root Cause
Game uses **registry-based loading** (two files: registry + individual definitions), but campaign builder expects **monolithic loading** (everything in one file).

### Solution
Update campaign builder's `load_creatures()` and `save_creatures()` to implement the same two-step registry pattern the game uses.

### Files to Change
- `sdk/campaign_builder/src/lib.rs` (two functions)

### Expected Result
✅ Creatures editor loads creatures successfully
✅ Users can edit creatures in the campaign builder
✅ Changes are saved correctly
✅ Tutorial campaign works as intended

---

## Reading Recommendations

### Scenario 1: "I have 5 minutes"
1. Read `CREATURES_EDITOR_ISSUE_SUMMARY.md` (5 min)
2. Done! You understand the problem

### Scenario 2: "I have 30 minutes and want to implement"
1. Read `CREATURES_EDITOR_ISSUE_SUMMARY.md` (5 min)
2. Read `fix_creatures_editor_loading.md` (10 min)
3. Read code examples and start implementing (15 min)

### Scenario 3: "I want to understand everything"
1. Read `CREATURES_EDITOR_ISSUE_SUMMARY.md` (5 min)
2. Read `creatures_loading_diagrams.md` (15 min)
3. Read `creatures_loading_pattern_comparison.md` (15 min)
4. Read `creatures_editor_loading_issue.md` (20 min)
5. Read `fix_creatures_editor_loading.md` (10 min)
6. Implement the fix (30 min)
**Total: ~95 minutes**

### Scenario 4: "I need to debug something"
1. Read `creatures_loading_diagrams.md` section "Diagram 6: Error Handling" (5 min)
2. Read `fix_creatures_editor_loading.md` section "Troubleshooting" (5 min)
3. Check error messages against documented scenarios

---

## Key Concepts

### CreatureReference
- **What**: Lightweight registry entry
- **Contains**: id, name, filepath
- **Stored in**: `creatures.ron`
- **Size**: ~50 bytes each
- **Purpose**: Points to individual creature files

### CreatureDefinition
- **What**: Full creature definition
- **Contains**: id, name, meshes, transforms, scale, color
- **Stored in**: `assets/creatures/*.ron`
- **Size**: ~5KB each (includes mesh data)
- **Purpose**: Complete visual definition

### Registry-Based Loading
- **Step 1**: Parse registry file as `Vec<CreatureReference>`
- **Step 2**: For each reference, load individual creature file
- **Step 3**: Combine all definitions
- **Step 4**: Validate (IDs must match)

---

## Implementation Checklist

- [ ] Read appropriate documentation (see scenarios above)
- [ ] Locate `load_creatures()` function in `lib.rs` (~line 1961)
- [ ] Locate `save_creatures()` function in `lib.rs` (~line 2010)
- [ ] Implement Step 1: Update `load_creatures()`
- [ ] Implement Step 2: Update `save_creatures()`
- [ ] Implement Step 3: Add `CreatureReference` import
- [ ] Run `cargo fmt --all`
- [ ] Run `cargo check --all-targets --all-features`
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Run `cargo nextest run --all-features`
- [ ] Test with tutorial campaign
- [ ] Verify creatures load (should see ~40)
- [ ] Test save and reload
- [ ] All checks pass ✓

---

## File Organization

```
docs/
├── explanation/
│   ├── CREATURES_EDITOR_DOCUMENTATION_INDEX.md ← You are here
│   ├── CREATURES_EDITOR_ISSUE_SUMMARY.md
│   ├── creatures_editor_loading_issue.md
│   ├── creatures_loading_pattern_comparison.md
│   └── creatures_loading_diagrams.md
└── how-to/
    └── fix_creatures_editor_loading.md
```

---

## Related Code References

### Game (Reference Implementation)
- **File**: `src/domain/visual/creature_database.rs`
- **Function**: `load_from_registry()`
- **Status**: ✅ Works correctly
- **Use as**: Reference for correct pattern

### Data Structures
- **File**: `src/domain/visual/mod.rs`
- **Types**: `CreatureReference`, `CreatureDefinition`
- **Status**: ✅ Correctly defined
- **Use as**: Reference for type definitions

### Campaign Builder (Needs Fixing)
- **File**: `sdk/campaign_builder/src/lib.rs`
- **Functions**: `load_creatures()`, `save_creatures()`
- **Status**: ❌ Broken
- **Action**: Apply fixes from documentation

---

## Common Questions

### Q: What's the difference between CreatureReference and CreatureDefinition?
**A**: See "Key Concepts" section above, or read `creatures_loading_pattern_comparison.md` for detailed comparison.

### Q: Why does the game use registry-based loading?
**A**: See `creatures_editor_loading_issue.md` section "Why This Pattern Exists" or `creatures_loading_pattern_comparison.md` section "Why This Pattern Exists".

### Q: How do I know if the fix worked?
**A**: See `fix_creatures_editor_loading.md` section "Step 4: Test the Fix" or `CREATURES_EDITOR_ISSUE_SUMMARY.md` section "Success Criteria".

### Q: What if creatures still don't load after the fix?
**A**: See `fix_creatures_editor_loading.md` section "Troubleshooting".

### Q: Do I need to change the game code?
**A**: No. The game already implements the correct pattern. Only campaign builder needs fixing.

### Q: Can I see code examples?
**A**: Yes. See `creatures_editor_loading_issue.md` section "Implementation Plan" or `fix_creatures_editor_loading.md` "Step-by-Step Fix".

---

## What Each Document Teaches

| Document | Teaches | Best For |
|----------|---------|----------|
| **Summary** | What's broken and why | Quick overview |
| **Diagrams** | Visual understanding | Visual learners |
| **Pattern Comparison** | How game vs builder differ | Understanding architecture |
| **Loading Issue** | Deep technical details | Complete understanding |
| **Fix Guide** | Step-by-step implementation | Implementing the fix |

---

## After Reading

Once you've read the appropriate documentation for your needs:

1. **If you're implementing**: Start with `fix_creatures_editor_loading.md`
2. **If you're reviewing**: Check `creatures_editor_loading_issue.md` for completeness
3. **If you're debugging**: Cross-reference with `creatures_loading_diagrams.md`
4. **If you're testing**: Use the "Success Criteria" in `CREATURES_EDITOR_ISSUE_SUMMARY.md`

---

## Document Stats

| Document | Lines | Read Time | Focus |
|----------|-------|-----------|-------|
| Summary | ~322 | 5 min | Overview |
| Loading Issue | ~372 | 15 min | Technical |
| Pattern Comparison | ~454 | 15 min | Analysis |
| Diagrams | ~642 | 15 min | Visual |
| Fix Guide | ~320 | 10 min | Implementation |
| **Total** | **~2,110** | **~70 min** | **Complete** |

---

## Next Steps

1. **Choose your scenario** from "Reading Recommendations" above
2. **Follow the recommended reading order**
3. **Implement the fix** using the step-by-step guide
4. **Test with the tutorial campaign**
5. **Run quality checks** (fmt, check, clippy, nextest)
6. **Verify success** using the success criteria

---

## Questions or Issues?

If something in the documentation is unclear:
1. Check the "Common Questions" section above
2. Look for similar scenarios in `creatures_loading_diagrams.md`
3. See "Troubleshooting" in `fix_creatures_editor_loading.md`
4. Review error handling in `creatures_loading_diagrams.md` "Diagram 6"

---

**Start reading now:** Pick your scenario above and begin with the first recommended document!
