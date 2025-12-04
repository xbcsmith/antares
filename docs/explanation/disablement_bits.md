# disablement bits — implementation & impact

Last updated: 2025-12-03

This document explains the `disablement_bit` concept, how it is implemented, where it is used, what impacts changes would have, and recommended steps for safe modification and migration.

Summary
-------
- `disablement_bit` is a per-class index (0..=7) stored on a class definition and used to map that class to a bit in an item-level `Disablement` bitmask.
- Items declare a numeric `disablements` mask (u8 bitmask) that lists classes and alignments that are allowed to use the item.
- Class bits (0..5) map to classes (Knight, Paladin, Archer, Cleric, Sorcerer, Robber) and bits 6 & 7 are reserved for alignments (GOOD and EVIL).
- Editors (SDK & CLI) use these masks to display and edit class restrictions. The runtime currently has helpers for class & alignment checks but lacks a single, canonical enforcement function across the codebase.

Terminology
-----------
- disablement_bit: An index 0..=7 in class definitions indicating which bit in a bitmask belongs to the class.
- disablements (u8): An item-level mask where each bit indicates that class/alignment can use the item (1 = can use, 0 = cannot).
- class bit: A single bit corresponding to a specific class flag in the mask.
- alignment bit: A bit used to restrict to Good/Evil alignments.

Key Code & Data Locations
-------------------------
- Disablement bitmask type:
```antares/src/domain/items/types.rs#L308-368
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disablement(pub u8);

impl Disablement {
    pub const ALL: Self = Self(0xFF);
    pub const NONE: Self = Self(0x00);

    pub const KNIGHT: u8 = 0b0000_0001;
    pub const PALADIN: u8 = 0b0000_0010;
    pub const ARCHER: u8 = 0b0000_0100;
    pub const CLERIC: u8 = 0b0000_1000;
    pub const SORCERER: u8 = 0b0001_0000;
    pub const ROBBER: u8 = 0b0010_0000;
    pub const GOOD: u8 = 0b0100_0000;
    pub const EVIL: u8 = 0b1000_0000;

    pub fn can_use_class(&self, class_bit: u8) -> bool { (self.0 & class_bit) != 0 }
    pub fn good_only(&self) -> bool { (self.0 & Self::GOOD) != 0 }
    pub fn evil_only(&self) -> bool { (self.0 & Self::EVIL) != 0 }

    pub const fn from_index(index: u8) -> Self { Self(1u8 << (index & 0x07)) }
    pub const fn mask_from_index(index: u8) -> u8 { 1u8 << (index & 0x07) }
    pub fn to_index(&self) -> Option<u8> { /* returns index if single-bit only */ }
}
```

- Item struct (stores the mask):
```antares/src/domain/items/types.rs#L392-432
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    // ...
    /// Class/alignment restrictions
    pub disablements: Disablement,
    // ...
}
```

- ClassDefinition (index and mask)
```antares/src/domain/classes.rs#L120-204
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassDefinition {
    // ...
    /// Bit index for item disablement checking.
    #[serde(rename = "disablement_bit")]
    pub disablement_bit_index: u8,
    // ...
}

impl ClassDefinition {
    pub fn disablement_mask(&self) -> u8 {
        1 << self.disablement_bit_index
    }
}
```

- Example data file usage
  - class bit indices in `data/classes.ron`:
```antares/data/classes.ron#L20-66
(
    id: "knight",
    // ...
    disablement_bit: 0,
    // ...
),
(
    id: "paladin",
    // ...
    disablement_bit: 1,
    // ...
),
```
  - item masks are numeric:
```antares/data/items.ron#L1-40
(
    id: 1,
    name: "Club",
    disablements: (255), // All classes
),
(
    id: 3,
    name: "Short Sword",
    disablements: (63),  // bits 0..5 set (class bits)
),
```

- Editors & tooling (reads, presents, and sets bits)
  - SDK editor toggles the bits:
```antares/sdk/campaign_builder/src/items_editor.rs#L1086-1136
for (flag, name) in &classes {
    let mut can_use = disablement.can_use_class(*flag);
    if ui.checkbox(&mut can_use, *name).changed() {
        if can_use { disablement.0 |= *flag; } else { disablement.0 &= !*flag; }
    }
}
```

  - CLI editor composes the flags via OR:
```antares/src/bin/item_editor.rs#L372-440
let mut flags = 0u8;
if knight { flags |= Disablement::KNIGHT; }
if paladin { flags |= Disablement::PALADIN; }
// ...
Disablement(flags)
```

  - Class editor allocates the next free bit when adding classes:
```antares/src/bin/class_editor.rs#L524-532
fn get_next_disablement_bit(&self) -> u8 {
    let mut used_bits = [false; 8];
    for class in &self.classes {
        if (class.disablement_bit_index as usize) < 8 {
            used_bits[class.disablement_bit_index as usize] = true;
        }
    }
    for (idx, &used) in used_bits.iter().enumerate() {
        if !used { return idx as u8; }
    }
    0 // fallback
}
```

Behavior & Enforcement
----------------------
- At present:
  - Helper methods exist: `Disablement::can_use_class(flag)` and `good_only()/evil_only()`.
  - There isn't a single domain-level `Disablement::can_use(class, alignment)` wrapper consistently used by runtime logic. The architecture documentation suggests a canonical `can_use` that accepts a `Class` and `Alignment` and returns boolean.
- For consistent enforcement:
  - Implement domain-level `Disablement::can_use(Class, Alignment)` with unit tests.
  - Expose a `Character::can_equip(item, &class_db)` or `Item::can_be_used_by(character, &class_db)` to centralize equip/use checks.

Why this matters (Risks)
------------------------
- Data mapping & migrations:
  - Existing `data/items.ron` uses numeric masks. If bit index semantics change, all item masks will be corrupted and require data migration.
- Limited capacity:
  - `Disablement` is a `u8` (8 bits). Adding more classes or more complex restrictions requires type changes (`u16`, `u32`) or switching to a dynamic representation (list / set of `ClassId`).
- Hard-coded UI:
  - Many editors present a static class list and rely on specific bit positions for their checkboxes. Adding classes or re-ordering bits require updates in all tools.
- `Class` enum vs dynamic class definitions:
  - The runtime uses a `Class` enum (`character.rs`) while `ClassDefinition` is data-driven via RON. Adding or changing classes may force code updates to the enum or moving to a dynamic class model.

Recommended change paths (safe modification)
-------------------------------------------
If you plan to change bit semantics or representation:

1. Centralize the check first:
   - Add `Disablement::can_use(Class, Alignment)` and use it in a `Character::can_equip` or `Item::can_be_used_by` helper. This reduces duplication.

2. For a small addition of classes:
   - Upgrade `Disablement(pub u8)` to `Disablement(pub u16)` or `Disablement(pub u32)`. Update constants (shift them accordingly) and update all code, RON examples, editors, and tests.

3. For a data-driven class approach:
   - Replace the bitmask with a dynamic set (e.g., `Vec<ClassId>` or `HashSet<ClassId>`) at the item level:
     - Implement a migration tool to convert existing masks into lists of `ClassId`.
     - Update RON item format (e.g., `disablements: ["knight", "paladin"]`).
     - Update editors and CLI to show class lists dynamically from `ClassDatabase`.
     - Make sure `ClassDefinition` provides `id` → `bit` translation if you still need a mask representation for legacy purposes.
     - Add interop logic that can read both old and new formats for backward compatibility.

4. Update editors:
   - Make editors dynamically read class names from `ClassDatabase` and display them; do not hard-code class names & bit constants.
   - Update the class editor to allow choosing a bit for the class or to remove the bit concept entirely in favor of `ClassId` lists.

5. Add tests:
   - `Disablement` from/to index conversions.
   - `Disablement::can_use` behavior for combinations of class + alignment.
   - Editor round-trip (set mask in editor → save to RON → load back → same mask).
   - Data migration tests (if changing format).
   - Confirmation that UI toggles correct bits for specific classes.

Files likely to change
----------------------
- Domain:
  - `src/domain/items/types.rs` (Disablement struct & constants).
  - `src/domain/classes.rs` (disablement index & validations).
  - `src/domain` logical checks to enforce equip/use rules.

- Editors & Tools:
  - `src/bin/item_editor.rs` (CLI editing).
  - `src/bin/class_editor.rs` (class bit allocation).
  - `sdk/campaign_builder/src/items_editor.rs` (SDK UI).

- Data:
  - `data/classes.ron`, `data/items.ron` (migrate masks or change format).

- Documentation:
  - `docs/reference/architecture.md` (ensure the doc and code align).
  - This new doc (`docs/explanation/disablement_bits.md`).
  - `docs/how-to/` guides and SDK docs.

- Tests:
  - All tests that rely on specific mask values or editor state.

Migration / upgrade checklist (example)
--------------------------------------
1. Implement `Disablement::can_use` and unit tests.
2. Upgrade type (u8 → u16) and update constants + tests.
3. Update class DB & data validation to allow new bit range.
4. Update CLI & SDK editors to use dynamic class lists or to support the larger bit width.
5. Convert `data/items.ron` masks to new format (script + tests).
6. Update `data/classes.ron` if shifting bit indices or class identifiers.
7. Update all tests and CI to cover new behavior.
8. Update docs to reflect new semantics (architecture, how-to).

Examples and notes
------------------
- Current pattern examples:
  - Data-driven classes define `disablement_bit` per class in `data/classes.ron`.
  - Item RON `disablements` is a numeric mask. Editors show/allow toggling of those bits.
- A single canonical enforcement function makes it easy to ensure consistent behavior across CLI, SDK, and runtime.

Closing
-------
If you want, I can:
- Implement a domain-level `Disablement::can_use` and add unit tests,
- Add a `Character::can_equip` helper and apply it at the equip/use codepath,
- Draft a migration script to convert masks from `u8` to `u16` format or to class ID collections,
- Make SDK & CLI editors dynamic in class list generation.

For any changes beyond documentation (i.e., code or format changes), please tell me which route you'd prefer (small numeric mask expansion vs dynamic class ID set), and I’ll produce a design + migration plan and a working PR sketch.
