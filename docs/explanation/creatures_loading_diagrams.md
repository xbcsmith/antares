# Creatures Loading Architecture - Visual Diagrams

## Diagram 1: Two-Step Registry Loading Pattern

### Game (Correct)

```
┌─────────────────────────────────────────────────────────────┐
│ Campaign Root: campaigns/tutorial/                          │
└─────────────────────────────────────────────────────────────┘
                            │
                ┌───────────┴───────────┐
                │                       │
        ┌───────▼───────┐      ┌────────▼────────┐
        │  data/        │      │  assets/        │
        │ creatures.ron │      │ creatures/      │
        └───────┬───────┘      └────────┬────────┘
                │                       │
                │                       │
         (Step 1: Read)          (Step 2: Read each)
                │                       │
                ▼                       │
        ┌──────────────────┐           │
        │ Parse as         │           │
        │ Vec<             │           │
        │ CreatureRef>     │           │
        │                  │           │
        │ [Ref 1: id=1     │           │
        │  Ref 2: id=2     │           │
        │  Ref 3: id=3]    │           │
        └──────┬───────────┘           │
               │                       │
         (Step 3: Extract filepath)    │
               │                       │
        ┌──────▼──────────────────────┐│
        │ For each reference:         ││
        │ - Get filepath              ││
        │ - Load file                 ││
        │ - Parse as                  ││
        │   CreatureDefinition        ││
        │ - Validate ID match         ││
        │ - Add to collection         ││
        └──────┬───────────────────────┘│
               │                        │
               │◄───────────────────────┘
               │
        ┌──────▼──────────────────────┐
        │ Loaded Creatures:           │
        │ [Creature 1 (goblin),       │
        │  Creature 2 (dragon),       │
        │  Creature 3 (skeleton)]     │
        └─────────────────────────────┘
```

### Campaign Builder (Current - Broken)

```
┌──────────────────────────────────┐
│ campaigns/tutorial/              │
│ data/creatures.ron               │
└──────────┬───────────────────────┘
           │
      (Read)
           │
           ▼
    ┌─────────────────────┐
    │ File contains:      │
    │ Vec<               │
    │ CreatureReference> │
    │                    │
    │ fields:            │
    │ - id               │
    │ - name             │
    │ - filepath  ◄──────┼─ This field!
    └─────────────────────┘
           │
    (Try to parse)
           │
           ▼
    ┌─────────────────────┐
    │ Expects:            │
    │ Vec<               │
    │ CreatureDef>       │
    │                    │
    │ fields:            │
    │ - id               │
    │ - name             │
    │ - meshes           │
    │ - transforms       │
    │ - scale            │
    │ - color_tint       │
    └─────────────────────┘
           │
        ❌ MISMATCH!
           │
           ▼
    ┌─────────────────────┐
    │ Parse Error:        │
    │ Unknown field       │
    │ "filepath" in       │
    │ CreatureDefinition  │
    │                     │
    │ ➜ Creatures: empty  │
    │ ➜ Editor: blank     │
    └─────────────────────┘
```

### Campaign Builder (After Fix)

```
┌──────────────────────────────────────────────────────┐
│ campaigns/tutorial/                                  │
│ data/creatures.ron ──────────────┐                  │
│ assets/creatures/*.ron ──────────┤                  │
└──────────────────────────────────┼──────────────────┘
                                   │
                       ┌───────────┴─────────┐
                       │                     │
                    (Step 1)            (Step 2-3)
                       │                     │
                       ▼                     │
            ┌────────────────────┐          │
            │ Parse Registry:    │          │
            │ Vec<              │          │
            │ CreatureReference>│          │
            │                  │          │
            │ [                │          │
            │ Ref {            │          │
            │   id: 1,        │          │
            │   name: "Goblin",│          │
            │   filepath:     │──────────┐│
            │ "assets/creatures/│         ││
            │  goblin.ron"    │         ││
            │ },              │         ││
            │ Ref { ... },    │         ││
            │ Ref { ... }     │         ││
            │ ]               │         ││
            └────────────────────┘      ││
                       │                ││
          ┌────────────┴────────────┐  ││
          │                         │  ││
        Parse for                   │  ││
       each ref:                    │  ││
          │                         │  ││
          ▼                         │  ││
    ┌──────────────────┐           │  ││
    │ Extract filepath │◄──────────┤──┘│
    │ from reference   │           │   │
    └──────┬───────────┘           │   │
           │                       │   │
           ▼                       │   │
    ┌──────────────────┐           │   │
    │ Build full path: │           │   │
    │ campaign_root +  │           │   │
    │ filepath         │           │   │
    │                  │           │   │
    │ Result:          │           │   │
    │ campaigns/       │◄──────────┤───┘
    │ tutorial/assets/ │           │
    │ creatures/       │           │
    │ goblin.ron       │           │
    └──────┬───────────┘           │
           │                       │
      (Read file)                  │
           │                       │
           ▼                       │
    ┌──────────────────────┐      │
    │ Parse as:            │      │
    │ CreatureDefinition { │      │
    │   id: 1,             │      │
    │   name: "Goblin",    │      │
    │   meshes: [...],     │      │
    │   transforms: [...], │      │
    │   scale: 1.0,        │      │
    │   color_tint: None   │      │
    │ }                    │      │
    └──────┬───────────────┘      │
           │                      │
      Validate ID match           │
           │                      │
      reference.id (1)            │
           == creature.id (1) ✓   │
           │                      │
           ▼                      │
    ┌──────────────────┐         │
    │ Add to           │         │
    │ creatures_list   │         │
    └──────┬───────────┘         │
           │                     │
           └─────────────────────┘
                (repeat for all)
                     │
                     ▼
            ┌─────────────────────┐
            │ Loaded Creatures:   │
            │ [Goblin (id=1),     │
            │  Dragon (id=2),     │
            │  Skeleton (id=3),   │
            │  ... 37 more]       │
            │                     │
            │ ✓ Status:           │
            │ "Loaded 40 creatures"│
            └─────────────────────┘
```

---

## Diagram 2: File Structure Comparison

### Old Approach (What Campaign Builder Expected)

```
campaigns/tutorial/
│
└── data/
    └── creatures.ron (MONOLITHIC)
        │
        └── [
            CreatureDefinition {
              id: 1,
              name: "Goblin",
              meshes: [...huge mesh data...],
              transforms: [...],
            },
            CreatureDefinition {
              id: 2,
              name: "Dragon",
              meshes: [...huge mesh data...],
              transforms: [...],
            },
            CreatureDefinition {
              id: 3,
              name: "Skeleton",
              meshes: [...huge mesh data...],
              transforms: [...],
            },
            ... (40 creatures total)
          ]

File Size: ~200KB (all data in one file)
Issues:
  - Hard to edit individual creatures
  - Merge conflicts in version control
  - Must load all creatures even if need few
  - Not modular
```

### New Approach (What Game Actually Uses)

```
campaigns/tutorial/
│
├── data/
│   └── creatures.ron (REGISTRY - Lightweight!)
│       │
│       └── [
│           CreatureReference {
│             id: 1,
│             name: "Goblin",
│             filepath: "assets/creatures/goblin.ron"
│           },
│           CreatureReference {
│             id: 2,
│             name: "Dragon",
│             filepath: "assets/creatures/dragon.ron"
│           },
│           CreatureReference {
│             id: 3,
│             name: "Skeleton",
│             filepath: "assets/creatures/skeleton.ron"
│           },
│           ... (40 references)
│         ]
│
│   Registry Size: ~2KB
│
└── assets/creatures/
    ├── goblin.ron
    │   └── CreatureDefinition { id: 1, ... meshes ... }
    │   Size: ~5KB
    │
    ├── dragon.ron
    │   └── CreatureDefinition { id: 2, ... meshes ... }
    │   Size: ~8KB
    │
    ├── skeleton.ron
    │   └── CreatureDefinition { id: 3, ... meshes ... }
    │   Size: ~4KB
    │
    └── ... (37 more individual files)

Total Size: ~2KB (registry) + ~200KB (individual files)
            = ~202KB

Advantages:
  ✓ Easy to edit individual creatures
  ✓ No merge conflicts
  ✓ Load only needed creatures
  ✓ Highly modular
  ✓ Scales to thousands of creatures
  ✓ Version control friendly
```

---

## Diagram 3: Data Type Relationship

```
┌─────────────────────────────────────────────────────────────┐
│                    CREATURE SYSTEM                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌───────────────────┐       ┌──────────────────────┐     │
│  │ CreatureReference │       │ CreatureDefinition   │     │
│  ├───────────────────┤       ├──────────────────────┤     │
│  │ - id: u32         │       │ - id: u32            │     │
│  │ - name: String    │◄──┐   │ - name: String       │     │
│  │ - filepath:       │   │   │ - meshes: Vec<...>   │     │
│  │   String          │   │   │ - mesh_transforms... │     │
│  │                   │   │   │ - scale: f32         │     │
│  │ "Points to"       │   │   │ - color_tint: Opt... │     │
│  └───────────────────┘   │   │                      │     │
│         │                │   │ "Full Definition"    │     │
│         │                │   └──────────────────────┘     │
│         │                │            │                   │
│     Stored in:           │        Stored in:              │
│         │                └────────────────┐               │
│    campaigns/tutorial/        campaigns/tutorial/          │
│    data/creatures.ron         assets/creatures/*.ron       │
│         │                            │                     │
│    Registry Index            Individual Files              │
│    (~2KB, lightweight)        (~200KB, heavy)              │
│                                                             │
│  RELATIONSHIP:                                             │
│  Each CreatureReference points to ONE CreatureDefinition  │
│  via the filepath field.                                  │
│                                                             │
│  LOADING:                                                  │
│  1. Load all CreatureReference entries                    │
│  2. For each reference, load the CreatureDefinition       │
│  3. Validate that IDs match                               │
│  4. Combine into collection                               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Diagram 4: Campaign Builder Fix Flow

```
┌──────────────────────────────────────────────────────────────┐
│ Campaign Builder Startup                                     │
└──────────────────────────────────────────────────────────────┘
                          │
                    User opens campaign
                          │
                          ▼
            ┌─────────────────────────────┐
            │ load_creatures() called      │
            └──────────┬──────────────────┘
                       │
          ┌────────────┴─────────────────┐
          │                              │
      (Before Fix)                  (After Fix)
          │                              │
          ▼                              ▼
   ┌──────────────┐            ┌──────────────────┐
   │ Read file    │            │ Read file        │
   └──────┬───────┘            └──────┬───────────┘
          │                           │
          ▼                           ▼
   ┌──────────────┐        ┌─────────────────────┐
   │ Try parse as │        │ Parse as            │
   │ CreatureDef  │        │ Vec<CreatureRef>    │
   │ (WRONG TYPE!)│        │ (CORRECT TYPE)      │
   └──────┬───────┘        └──────┬──────────────┘
          │                       │
          ▼                       ▼
   ┌──────────────┐        ┌─────────────────────┐
   │ ❌ PARSE FAIL│        │ ✓ Parse success     │
   │              │        │ Got 40 references   │
   │ Error:       │        └──────┬──────────────┘
   │ Unknown field│               │
   │ "filepath"   │        ┌──────▼────────────────┐
   └──────┬───────┘        │ For each reference:   │
          │                │ 1. Extract filepath  │
          ▼                │ 2. Load file         │
   ┌──────────────┐        │ 3. Parse as          │
   │ creatures: [] │        │    CreatureDef      │
   │              │        │ 4. Validate ID      │
   │ Editor shows │        │ 5. Add to list      │
   │ empty list   │        └──────┬───────────────┘
   │              │               │
   │ ❌ BROKEN    │               ▼
   │              │        ┌──────────────────┐
   │ Status:      │        │ creatures: [40]  │
   │ "Failed to   │        │                  │
   │  parse:      │        │ Editor shows     │
   │  unknown     │        │ full list        │
   │  field..."   │        │                  │
   └──────────────┘        │ ✓ WORKING        │
                           │                  │
                           │ Status:          │
                           │ "Loaded 40       │
                           │  creatures"      │
                           └──────────────────┘
```

---

## Diagram 5: Save Operation Before and After

### Before Fix (Single File)

```
User clicks Save in Creatures Editor
        │
        ▼
┌──────────────────────┐
│ save_creatures()     │
└──────┬───────────────┘
       │
       ▼
┌──────────────────────────────────────────┐
│ Serialize self.creatures vector to RON   │
│                                          │
│ [                                        │
│   CreatureDefinition { id: 1, ... },    │
│   CreatureDefinition { id: 2, ... },    │
│   CreatureDefinition { id: 3, ... },    │
│   ...                                    │
│ ]                                        │
└──────┬───────────────────────────────────┘
       │
       ▼
┌────────────────────────────────────────┐
│ Write to creatures.ron                 │
│                                        │
│ Result: One HUGE file (~200KB)         │
│                                        │
│ Issues:                                │
│ - Individual creatures not in separate │
│   files                                │
│ - Can't control creature mapping from  │
│   registry                             │
│ - Not aligned with game loading        │
└────────────────────────────────────────┘
```

### After Fix (Registry + Individual Files)

```
User clicks Save in Creatures Editor
        │
        ▼
┌────────────────────────────┐
│ save_creatures()           │
│ (FIXED VERSION)            │
└──────┬─────────────────────┘
       │
       ├─── Step 1: Create Registry ───┐
       │                               │
       ▼                               │
┌────────────────────────────┐        │
│ For each creature:         │        │
│ Create CreatureReference { │        │
│   id: creature.id,         │        │
│   name: creature.name,     │        │
│   filepath: format!(       │        │
│     "assets/creatures/     │        │
│      {}.ron", name)        │        │
│ }                          │        │
│                            │        │
│ Result:                    │        │
│ Vec<CreatureReference> [40]│        │
└──────┬─────────────────────┘        │
       │                              │
       ▼                              │
┌──────────────────────────────────┐ │
│ Write registry to creatures.ron   │ │
│                                  │ │
│ File: data/creatures.ron         │ │
│ Size: ~2KB                       │ │
│                                  │ │
│ Content: [                       │ │
│   CreatureReference {            │ │
│     id: 1,                       │ │
│     name: "Goblin",              │ │
│     filepath: "assets/creatures/ │ │
│                 goblin.ron"      │ │
│   },                             │ │
│   ...                            │ │
│ ]                                │ │
└──────┬───────────────────────────┘ │
       │                              │
       ├─── Step 2: Write Individual Files ──┐
       │                                     │
       ▼                                     │
┌──────────────────────────────────────┐   │
│ For each creature:                   │   │
│ Serialize CreatureDefinition to RON  │   │
│                                      │   │
│ Write to:                            │   │
│ assets/creatures/{name}.ron          │   │
│                                      │   │
│ Files created:                       │   │
│ - goblin.ron (~5KB)                 │   │
│ - dragon.ron (~8KB)                 │   │
│ - skeleton.ron (~4KB)               │   │
│ - ... (37 more files)               │   │
└──────┬───────────────────────────────┘   │
       │                                    │
       ▼                                    │
┌──────────────────────────────────────┐
│ SAVE COMPLETE                        │
│                                      │
│ Result:                              │
│ ✓ Registry: data/creatures.ron       │
│            (~2KB)                    │
│ ✓ Individual files: assets/creatures │
│            (~200KB total)            │
│                                      │
│ Status: "Saved 40 creatures          │
│          (registry + 40 files)"      │
│                                      │
│ Next load will read registry first,  │
│ then load individual files.          │
└──────────────────────────────────────┘
```

---

## Diagram 6: Error Handling

```
┌──────────────────────────────────────────────────────────┐
│ Loading Creatures - Error Scenarios                      │
└──────────────────────────────────────────────────────────┘

SCENARIO 1: Missing Creature File
┌─────────────────────────────────────────────────────────┐
│ Registry: Reference to assets/creatures/goblin.ron      │
│ File: DOES NOT EXIST                                    │
│                                                         │
│ Load attempt:                                           │
│ fs::read_to_string("...goblin.ron")                    │
│   ↓ Error: No such file or directory                   │
│                                                         │
│ Response:                                               │
│ load_errors.push(                                       │
│   "Failed to read assets/creatures/goblin.ron: ..."    │
│ )                                                       │
│                                                         │
│ Result:                                                 │
│ - Goblin NOT loaded                                     │
│ - Other creatures continue loading                      │
│ - Status shows: "Loaded 39 creatures with 1 error"     │
│ - Error logged for debugging                           │
└─────────────────────────────────────────────────────────┘

SCENARIO 2: Invalid RON Syntax
┌─────────────────────────────────────────────────────────┐
│ Registry: Reference to assets/creatures/dragon.ron      │
│ File: EXISTS but has INVALID RON                        │
│                                                         │
│ Content: {                                              │
│   id: 2,                                                │
│   name: "Dragon"                                        │
│   meshes: [ INVALID_SYNTAX }    ← Missing bracket       │
│                                                         │
│ Load attempt:                                           │
│ ron::from_str::<CreatureDefinition>(content)           │
│   ↓ Error: Unexpected token (parse error)              │
│                                                         │
│ Response:                                               │
│ load_errors.push(                                       │
│   "Failed to parse assets/creatures/dragon.ron: ..."   │
│ )                                                       │
│                                                         │
│ Result:                                                 │
│ - Dragon NOT loaded                                     │
│ - Other creatures continue loading                      │
│ - Status shows: "Loaded 39 creatures with 1 error"     │
│ - Error details printed to console                     │
└─────────────────────────────────────────────────────────┘

SCENARIO 3: ID Mismatch
┌─────────────────────────────────────────────────────────┐
│ Registry: CreatureReference {                           │
│   id: 3,                                                │
│   filepath: "assets/creatures/skeleton.ron"            │
│ }                                                       │
│                                                         │
│ File: assets/creatures/skeleton.ron contains:          │
│ CreatureDefinition {                                    │
│   id: 99,    ← MISMATCH! Registry says 3              │
│   ...                                                   │
│ }                                                       │
│                                                         │
│ Load attempt:                                           │
│ if creature.id (99) != reference.id (3)               │
│   ↓ Validation fails                                   │
│                                                         │
│ Response:                                               │
│ load_errors.push(                                       │
│   "ID mismatch for assets/creatures/skeleton.ron: " +   │
│   "registry=3, file=99"                               │
│ )                                                       │
│                                                         │
│ Result:                                                 │
│ - Skeleton NOT loaded                                   │
│ - Other creatures continue loading                      │
│ - Status shows: "Loaded 39 creatures with 1 error"     │
│ - Game will reject this on startup (validation)        │
└─────────────────────────────────────────────────────────┘

SUMMARY: Graceful Error Handling
┌──────────────────────────────────────────────────┐
│ ✓ Continue loading other creatures              │
│ ✓ Collect all errors into list                  │
│ ✓ Report count and details at end               │
│ ✓ Show in UI: "Loaded N with M errors"          │
│ ✓ Print detailed errors to console              │
│ ✓ User can fix one file and reload              │
└──────────────────────────────────────────────────┘
```

---

## Summary of Diagrams

| Diagram | Shows | Purpose |
|---------|-------|---------|
| **1** | Two-step loading process | Understand why fix is needed |
| **2** | File structure before/after | Visualize modular architecture |
| **3** | Data type relationship | See how types connect |
| **4** | Campaign builder flow | Understand control flow |
| **5** | Save operation | See how both files are created |
| **6** | Error handling | Understand failure modes |

These diagrams help understand the creatures loading issue at a glance.
