# Engine Hard-Coded Domain Elements — discovery & plan

Last updated: 2025-12-03

Purpose
-------
This document sketches an inventory of hard-coded (non-test) domain elements in the "Game Engine" portion of the project — specifically, hard-coded references to classes, items, spells, and other domain concepts that reduce the system's ability to be fully data-driven. For each occurrence I found, I summarize the issue and provide recommended remediation steps and priorities.

Notes & scope
-------------
- Focus: Non-test code in the Game Engine (primarily `src/domain/*`, `src/game/*`, `src/application/*`).
- Excludes test modules, SDK editors, and CLI admin tools (though many of these share similar patterns).
- The goal is to locate and document places that:
  - branch on `Class` enums or other static enums for game logic,
  - hard-code mappings (class → HP die, class → spell school or spell access),
  - assume a fixed set of classes by bit position (Disablement),
  - refer to specific numeric item/spell ids in the code (not in data files).
- This is a code-level audit (not exhaustive dynamic analysis). I gathered the primary hotspots and recommended changes for each.

Executive Summary of Findings
-----------------------------
- Core class-logic still relies on `Class` enum patterns and `match` / `if` logic:
  - `SpellBook::get_spell_list` & `get_spell_list_mut` rely on `Class` values.
  - Spellcasting logic (`can_class_cast_school`, `get_required_level_for_spell`, `calculate_spell_points`) uses `Class` enum patterns for behavior.
  - `roll_hp_gain` uses a `match` to set HP dice based on class.
- `Disablement` bit flags and constants are implemented in code and referenced in several places, which yields tightly coupled assumptions between bit positions and class IDs.
- Several domain `enum` types are defined statically (so extension requires code updates):
  - `Class`, `Race`, `Sex`, `Alignment` in `src/domain/character.rs`.
  - `ItemType`, `AccessorySlot`, `AmmoType`, etc., in `src/domain/items/types.rs`.
  - `SpellSchool` and `SpellStat` in `src/domain/classes.rs`.
- Most of the item/monster/spell content is already data-driven via RON files; the engine code still has some static behavior depending on enum values (this is where code must be refactored to use runtime class metadata).
- There are no major hard-coded numeric item/spell ids in runtime engine code that I found (those mostly appear in data files), but it's worth scanning more thoroughly if any exist.

Findings (Detailed)
-------------------

1) `SpellBook::get_spell_list` & `get_spell_list_mut` — static mapping
- File: src/domain/character.rs
- Code sample:
```antares/src/domain/character.rs#L680-694
    pub fn get_spell_list(&self, class: Class) -> &[Vec<SpellId>; 7] {
        match class {
            Class::Cleric | Class::Paladin => &self.cleric_spells,
            Class::Sorcerer | Class::Archer => &self.sorcerer_spells,
            _ => &self.sorcerer_spells, // Default to empty
        }
    }
```
- Why this is hard-coded:
  - The code assumes specific named classes map to specific spell lists. Adding new classes or swapping a class to a different spell school requires code changes.
- Recommendation:
  - Replace with runtime lookup: `ClassDefinition` (from `ClassDatabase`) should specify which spell school the class uses (and/or a `SpellListId`), and `SpellBook` should use that mapping at runtime.
  - Example: `get_spell_list(&self, class_id: ClassId, class_db: &ClassDatabase) -> &[Vec<SpellId>]`.

2) Class-based spell rules - `can_class_cast_school`, `get_required_level_for_spell`, `calculate_spell_points`
- File: src/domain/magic/casting.rs
- Code samples:
```antares/src/domain/magic/casting.rs#L250-255
    matches!(
        (class, school),
        (Class::Cleric, SpellSchool::Cleric)
            | (Class::Paladin, SpellSchool::Cleric)
            | (Class::Sorcerer, SpellSchool::Sorcerer)
            | (Class::Archer, SpellSchool::Sorcerer)
```

```antares/src/domain/magic/casting.rs#L291-304
    match class {
        Class::Cleric | Class::Sorcerer => base_required,
        Class::Paladin | Class::Archer => base_required.max(3),
        _ => 999,
    }
```

```antares/src/domain/magic/casting.rs#L337-347
    match character.class {
        Class::Cleric | Class::Paladin => calculate_sp_from_stat(personality),
        Class::Sorcerer | Class::Archer => calculate_sp_from_stat(intellect),
        _ => 0,
    }
```
- Why this is hard-coded:
  - Uses the `Class` enum directly to decide class-spell relationships, spell access progression, and SP calculation method.
- Recommendation:
  - Use the `ClassDefinition` to read `spell_school` and `spell_stat` (already present in the RON structure). Replace logic to:
    - Use `class_def.spell_school` rather than hard-coded `Class` enum membership,
    - Use `class_def.spell_stat` to calculate SP,
    - Use class metadata to derive `required_level` or store `spell_access` rules in a data file.

3) Class-based HP progression — `roll_hp_gain`
- File: src/domain/progression.rs
- Code sample:
```antares/src/domain/progression.rs#L236-246
pub fn roll_hp_gain(class: Class, rng: &mut impl Rng) -> u16 {
    let dice = match class {
        Class::Knight => DiceRoll::new(1, 10, 0),  // 1d10
        Class::Paladin => DiceRoll::new(1, 8, 0),  // 1d8
        Class::Archer => DiceRoll::new(1, 8, 0),   // 1d8
        Class::Cleric => DiceRoll::new(1, 6, 0),   // 1d6
        Class::Sorcerer => DiceRoll::new(1, 4, 0), // 1d4
        Class::Robber => DiceRoll::new(1, 6, 0),   // 1d6
    };
    dice.roll(rng).max(1) as u16
}
```
- Why this is hard-coded:
  - HP dice are directly matched to class enums, which makes adding classes or changing HP dice a code edit.
- Recommendation:
  - Use `ClassDefinition.hp_die` (already exists in `ClassDefinition` RON) for rolling HP. Replace this function with a simple call to `class_db.get_class(class_id).hp_die`.

4) Disablement bit constants & mask — `Disablement` structure
- File: src/domain/items/types.rs
- Code sample (excerpt):
```antares/src/domain/items/types.rs#L308-368
pub struct Disablement(pub u8);

impl Disablement {
    pub const ALL: Self = Self(0xFF);
    pub const NONE: Self = Self(0x00);
    // Class flags
    pub const KNIGHT: u8 = 0b0000_0001;
    pub const PALADIN: u8 = 0b0000_0010;
    pub const ARCHER: u8 = 0b0000_0100;
    pub const CLERIC: u8 = 0b0000_1000;
    pub const SORCERER: u8 = 0b0001_0000;
    pub const ROBBER: u8 = 0b0010_0000;
    pub const GOOD: u8 = 0b0100_0000;
    pub const EVIL: u8 = 0b1000_0000;
    // ...
}
```
- And `Item` uses the mask:
```antares/src/domain/items/types.rs#L392-432
pub struct Item {
    // ...
    /// Class/alignment restrictions
    pub disablements: Disablement,
    // ...
}
```
- Why this is hard-coded:
  - The engine and editors rely on fixed bit positions for classes & alignments. The mapping from bit to class index is static in `Disablement` constants.
  - This couples class definitions (in `data/classes.ron`) to specific bit indices—fragile if classes are added, reorder, or removed.
- Recommendation:
  - Use dynamic approach:
    - At minimum, maintain `Disablement` as a mask with dynamic mapping via `ClassDefinition.disablement_bit_index`. But since `Disablement` is a `u8`, you're limited to 8 bits and the code forbids more than 6 classes and 2 alignment flags.
    - Better: Migrate towards `Vec<ClassId>` or `HashSet<ClassId>` for item restrictions. Alternatively, expand mask to `u16/ u32` if only more bits are needed.
    - Update editors to use `ClassDatabase` to build checkboxes rather than hardcoding the 6 classes.

5) `Class` enum is static (root cause)
- File: src/domain/character.rs
- Code sample (the enum definition):
```antares/src/domain/character.rs#L620-642
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Class {
    Knight,
    Paladin,
    Archer,
    Cleric,
    Sorcerer,
    Robber,
}
```
- Why this is hard-coded:
  - The runtime uses a static enum for classes. To support dynamic classes from data (RON `ClassDefinition`), either `Class` enum must be kept in sync with data or character objects should use `ClassId` (string) that maps to `ClassDefinition`. The latter is required for a truly flexible data-driven engine.
- Recommendation:
  - Migrate characters to use `ClassId` (string or index referring to `ClassDefinition`) rather than the static `Class` enum. Provide a transitional mapping approach: keep the `Class` enum while supporting `ClassId` (`ClassDefinition.id`) in save/load; update domain code to work with `ClassId` and `ClassDatabase` for behavior.

Other Hard-coded concerns & checks
---------------------------------
- Multiple functions and logic may rely on `==` or `match` checks with enum values. Examples:
  - `casting::can_class_cast_school` and `get_required_level_for_spell`.
  - `progression::roll_hp_gain`.
  - `character::SpellBook` categorization of `class` into `cleric_spells` and `sorcerer_spells`.

- Equipment handling and item type checks are done via `ItemType` enum pattern matching (`is_weapon`, `is_armor`, etc). These `ItemType` enums reflect categories rather than list content. That is acceptable as categories; however, the overhead for adding new item categories would require code updates but is less painful than adding new classes.

Why these hard-coded patterns are problematic
-------------------------------------------
- Data-driven philosophy expects:
  - Behavior and rules should be expressed in data (RON files) wherever reasonable.
  - Adding or changing classes should be done by editing data (e.g., `data/classes.ron`) and not code.
- Problems introduced by the current approach:
  - Developer friction: Adding a new class requires code edits in multiple places (enums, `match` statements, editor code).
  - Migration hazards: Changing `Disablement` bit allocations leads to broken item masks unless a migration script is run.
  - Inconsistency: Some behavior is data-driven by `ClassDefinition`, but other behavior is still keyed to static `Class` enum or constants.
  - Testing limitations: If new classes are added, many code paths may not be covered by tests unless the code is refactored to read behavior from data.

Remediation road map (recommended)
---------------------------------
Priority levels: P0 = high, P1 = medium, P2 = low

P0 - Centralize & add `ClassId` model
- Replace `Character::class: Class` (enum) with `Character.class_id: ClassId` (string alias) — this is non-trivial and impacts serialization and code paths, but it unlocks flexibility.
- Provide conversion between enum-based `Class` and `ClassId` for compatibility.
- Ensure `Character` objects always look up dynamic behaviour via `ClassDatabase::get_class_by_id(&class_id)`.

P0 - Centralize class logic and policy checks
- Implement domain-level helpers and remove multiple `match`-based patterns:
  - `Disablement::can_use(class_id: ClassId, alignment: Alignment)` that uses `ClassDefinition.disablement_bit_index`.
  - `ClassDefinition`-driven functions for `hp_die`, `spell_school`, `spell_stat`.
  - `Item::can_be_equipped_by` or `Character::can_equip_item` methods that use dynamic class metadata.

P1 - Replace `match Class` with dynamic lookups
- Rework places found above to query `ClassDatabase`:
  - Replace `SpellBook::get_spell_list` with something like `SpellBook::get_spell_list_for_class(class_id, &class_db)` that returns the RON-specified spell school and so forth.
  - Move `roll_hp_gain` to a function that reads `HP Die` from `ClassDefinition`.
  - Replace `calculate_spell_points` to look at `class_def.spell_stat`.

P1 - Replace `Disablement` mask with dynamic representation
- Option A (less disruptive): Keep `Disablement` mask but make `Disablement` functions operate on dynamic `class.disablement_bit_index` rather than `Disablement::KNIGHT` constants. Add `Disablement::can_use_class_id(&self, class_id, &class_db)` that checks the mask using the class's assigned bit index. Update editors to present class names based on `ClassDatabase`.
- Option B (more dynamic): Move to `Vec<ClassId>` or `HashSet<ClassId>` at item-level for restrictions. Add migration helper to translate legacy mask to ID list.

P2 - Convert editors to dynamic lists
- CLI and SDK editors must not hard-code class names; update them to read from `ClassDatabase`.
- Replace `Disablement` UI checkboxes with a dynamic list of class names in the editor.

P1/P2 - Add integration & regression tests
- Tests should ensure:
  - `ClassDefinition` data drives behavior (HP die, spells, SP calculation).
  - Items' `disablements` map to the actual classes from `data/classes.ron`, not a hard-coded `Class` enum order.
  - Migrations from any legacy `u8` masks to new representation produce equivalent results.

Other considerations
--------------------
- Keep compatibility: When making big changes, provide compatibility options (read old masks and new sets, create migration scripts).
- API changes: If `Character` uses `ClassId` strings, API functions that previously accepted `Class` enum or `&Class` must be migrated to the new type or provide compatibility wrappers.
- Documentation updates: Update `docs/reference/architecture.md` and `docs/explanation/*` to reflect the new dynamic approach and the rationale.

Appendix: Quick list of key engine files to review
--------------------------------------------------
- `src/domain/character.rs` (enum `Class`, `SpellBook` mapping)
- `src/domain/classes.rs` (class RON handling; `ClassDefinition` structure)
- `src/domain/items/types.rs` (hard-coded `Disablement` bit flags; `Item` struct)
- `src/domain/magic/casting.rs` (spell access / spell level logic: `can_class_cast_school`, `get_required_level_for_spell`, `calculate_spell_points` etc.)
- `src/domain/progression.rs` (`roll_hp_gain` and `level_up` behavior mapping classes to HP die)
- `src/domain/combat/` (scan for class-specific logic & special-case abilities)
- `src/application` (if any game flow or save/load code references a static `Class` enum)

Next steps (suggested)
----------------------
1. Approve the proposed remediation (P0 → P2).
2. Implement `ClassId` approach (P0) and add unit & integration tests.
3. Add `Disablement::can_use` wrappers to the domain (P0).
4. Incrementally move every `match` / `if` on `Class` to use `ClassDefinition` lookups in tests (P1).
5. Update editors to be dynamic and switch to `ClassId` approach (P1 / P2).
6. Add migration scripts / backward compatibility layers for existing save files.

If you'd like, I can:
- Prepare a PR outline & patch to centralize `Disablement::can_use(ClassId, Alignment) -> bool` and convert a single `match` site (e.g. `roll_hp_gain`) into a `ClassDefinition` data-driven call as a proof-of-concept.
- Create a list of all `match` / `if` occurrences that are class-based specifically (including code lines / files) for a more exhaustive audit beyond this initial pass.
- Draft a migration script to convert `disablements: u8` masks to new representation (mask expand or `Vec<ClassId>`).

This document aims to be a practical starting point to remove hard-coded domain assumptions across the Game Engine.
