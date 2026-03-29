# Unified `antares-sdk` CLI Consolidation Plan

## Overview

The `src/bin/` directory currently contains 10 separate binaries — the game
itself plus 9 SDK/developer tools. These tools have inconsistent CLI patterns
(some use `clap`, some use raw `env::args`, some have no args at all),
duplicated helper code, and varying levels of correctness. One binary
(`update_tutorial_maps`) is a completed one-time migration tool that should be
deleted outright.

This plan consolidates the 8 remaining SDK tools into a single `antares-sdk`
binary with `clap` subcommands, giving developers a consistent UX:

```text
antares-sdk names --theme fantasy --number 5
antares-sdk class [FILE]
antares-sdk map validate map_1.ron map_2.ron
antares-sdk campaign validate campaigns/tutorial
```

## Current State Analysis

### Binary Inventory

| Binary                       | In Cargo.toml | CLI Lib          | Interaction     | Lines | Verdict       |
| ---------------------------- | ------------- | ---------------- | --------------- | ----- | ------------- |
| `antares.rs`                 | ✅             | `clap`           | Bevy game loop  | ~580  | **KEEP AS-IS** |
| `campaign_validator.rs`      | ✅             | `clap`           | One-shot        | ~390  | **MERGE**     |
| `class_editor.rs`            | ✅             | raw `env::args`  | Interactive REPL | ~840  | **MERGE**     |
| `race_editor.rs`             | ✅             | raw `env::args`  | Interactive REPL | ~840  | **MERGE**     |
| `item_editor.rs`             | ✅             | raw `env::args`  | Interactive REPL | ~1810 | **MERGE**     |
| `name_gen.rs`                | ✅             | `clap`           | One-shot        | ~120  | **MERGE**     |
| `map_builder.rs`             | ❌ (auto)      | None (rustyline) | Interactive REPL | ~840  | **MERGE**     |
| `validate_map.rs`            | ❌ (auto)      | raw `env::args`  | One-shot        | ~260  | **MERGE**     |
| `generate_terrain_textures.rs` | ✅           | None             | One-shot        | ~1460 | **MERGE**     |
| `update_tutorial_maps.rs`    | ✅             | None             | One-shot        | ~270  | **DELETE**     |

### Identified Issues

1. **Inconsistent CLI patterns** — 3 binaries use `clap`, 5 use raw
   `env::args`, and 2 have no CLI args at all.
2. **Duplicated helper code** — `truncate()`, `filter_valid_proficiencies()`,
   and `input_multistring_values()` are copy-pasted across class, race, and
   item editors.
3. **Hardcoded validation data** — `validate_map.rs` has
   `const VALID_MONSTER_IDS: &[u8] = &[1..10]` and similar for items, with
   TODO comments acknowledging they should be loaded dynamically.
4. **Dead code in map builder** — The `npc` command is deprecated and just
   prints a deprecation notice.
5. **Inconsistent RON serialization** — `class_editor` uses
   `struct_names(true)` while `race_editor` uses `struct_names(false)`.
6. **Race editor bypasses database loader** — `class_editor` uses
   `ClassDatabase::load_from_file()` but `race_editor` does raw
   `ron::from_str()` directly.
7. **Stale one-time migration tool** — `update_tutorial_maps.rs` creates
   `.bak` files and hardcodes map-specific visual metadata for 5 specific
   maps. This job is done.
8. **`generate_terrain_textures` has no output dir flag** — Output path is
   hardcoded to `CARGO_MANIFEST_DIR`.

## Target UX

```text
antares-sdk — Antares RPG SDK Command-Line Tools

USAGE:
    antares-sdk <COMMAND>

COMMANDS:
    names       Generate fantasy character names
    class       Interactive class definition editor
    race        Interactive race definition editor
    item        Interactive item definition editor
    map         Map creation and validation tools
    campaign    Campaign-level validation
    textures    Generate placeholder terrain textures
    help        Print help

SUBCOMMAND EXAMPLES:
    antares-sdk names --theme fantasy --number 5
    antares-sdk names --theme star --number 10 --lore
    antares-sdk class [FILE]                        # default: data/classes.ron
    antares-sdk race [FILE]                         # default: data/races.ron
    antares-sdk item [FILE]                         # default: data/items.ron
    antares-sdk map build                           # interactive REPL
    antares-sdk map validate map_1.ron map_2.ron
    antares-sdk campaign validate campaigns/tutorial
    antares-sdk campaign validate --all
    antares-sdk textures generate
    antares-sdk textures generate --output-dir /tmp/textures
```

## Proposed File Structure

```text
src/bin/
├── antares.rs                    # KEEP — the game binary (unchanged)
└── antares_sdk.rs                # NEW — unified entry point, clap dispatch

src/sdk/cli/                      # NEW module under existing src/sdk/
├── mod.rs                        # Re-exports subcommand modules
├── editor_helpers.rs             # Shared helpers extracted from editors
├── names.rs                      # From name_gen.rs
├── class_editor.rs               # From class_editor.rs
├── race_editor.rs                # From race_editor.rs
├── item_editor.rs                # From item_editor.rs
├── map_builder.rs                # From map_builder.rs
├── map_validator.rs              # From validate_map.rs (fixed)
├── campaign_validator.rs         # From campaign_validator.rs
└── texture_generator.rs          # From generate_terrain_textures.rs
```

The thin `src/bin/antares_sdk.rs` entry point will contain only the top-level
`clap` enum and dispatch:

```text
#[derive(Parser)]
enum Cli {
    Names(NamesArgs),
    Class(ClassArgs),
    Race(RaceArgs),
    Item(ItemArgs),
    Map(MapCommand),
    Campaign(CampaignCommand),
    Textures(TexturesCommand),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse() {
        Cli::Names(args) => cli::names::run(args),
        Cli::Class(args) => cli::class_editor::run(args.file),
        ...
    }
}
```

Each submodule in `src/sdk/cli/` exposes a `pub fn run(...)` entry point.

## Implementation Phases

### Phase 1: Delete Dead Weight and Scaffold (Low Risk)

**Estimated effort**: Small

#### 1.1 Delete `update_tutorial_maps.rs`

This is a completed one-time migration tool that:
- Creates `.bak` files
- Hardcodes visual metadata for 5 specific tutorial maps
- References "Phase 5" in its module doc
- Has `#[allow(clippy::too_many_arguments)]` on both core functions

Remove the file and its `[[bin]]` entry from `Cargo.toml`.

#### 1.2 Create the Module Scaffold

Create `src/sdk/cli/mod.rs` with submodule declarations. Create
`src/bin/antares_sdk.rs` with the top-level `clap` dispatch enum. Add the
new `[[bin]]` entry to `Cargo.toml`:

```text
[[bin]]
name = "antares-sdk"
path = "src/bin/antares_sdk.rs"
```

#### 1.3 Migrate `name_gen.rs` (Proof of Concept)

This is the smallest binary (~120 lines) and already uses `clap`. Migration
steps:

1. Move the `ThemeArg` enum, `Args` struct, `print_header()`, and generation
   logic into `src/sdk/cli/names.rs`.
2. Expose `pub fn run(args: NamesArgs) -> Result<(), Box<dyn Error>>`.
3. Wire the `Names` variant in `antares_sdk.rs` to call it.
4. Delete `src/bin/name_gen.rs` and its `[[bin]]` entry.
5. Run quality gates.

#### 1.4 Migrate `campaign_validator.rs`

Already uses `clap`. Similar migration pattern:

1. Move validation logic and `ValidationReport` handling into
   `src/sdk/cli/campaign_validator.rs`.
2. Wire the `Campaign` variant with subcommand `Validate`.
3. Delete the old binary.

#### 1.5 Testing Requirements

- All existing tests from migrated binaries must move with them and continue
  to pass.
- Add a smoke test that the `antares-sdk` binary parses `--help` without
  error.

#### 1.6 Success Criteria

- `update_tutorial_maps` binary is gone.
- `antares-sdk names` and `antares-sdk campaign validate` work identically
  to the old standalone binaries.
- All quality gates pass.

---

### Phase 2: Migrate One-Shot Tools (Low Risk)

**Estimated effort**: Small–Medium

#### 2.1 Migrate `validate_map.rs` → `antares-sdk map validate`

1. Move to `src/sdk/cli/map_validator.rs`.
2. **Fix hardcoded IDs**: Replace `const VALID_MONSTER_IDS` and
   `const VALID_ITEM_IDS` with dynamic loading from the campaign's data
   files. Accept an optional `--campaign-dir` flag to locate the data. If
   not provided, skip ID validation with a warning.
3. Replace raw `env::args` with `clap` subcommand args.
4. Fix the event summary `match` to use named counters for all event
   variants instead of lumping `Furniture`, `Container`, `DroppedItem`,
   `LockedDoor`, `LockedContainer`, `EnterInn`, and `RecruitableCharacter`
   into the `signs` or `dialogues` buckets.

#### 2.2 Migrate `generate_terrain_textures.rs` → `antares-sdk textures generate`

1. Move to `src/sdk/cli/texture_generator.rs`.
2. Add `--output-dir <PATH>` CLI arg (default: `assets/textures/` relative
   to current directory).
3. No antares library imports — this module is self-contained.

#### 2.3 Testing Requirements

- Existing tests from both binaries migrate with them.
- Add test for dynamic ID loading (mock campaign data directory).
- Add test for `--output-dir` flag parsing.

#### 2.4 Success Criteria

- `antares-sdk map validate` loads IDs dynamically when `--campaign-dir`
  is provided.
- `antares-sdk textures generate --output-dir /tmp/test` writes to the
  specified directory.
- All quality gates pass.

---

### Phase 3: Migrate Interactive Editors (Medium Risk)

**Estimated effort**: Medium

This is the largest phase. The three data editors (class, race, item) and the
map builder are all interactive REPL-style tools. The migration pattern is the
same for each:

1. Extract the editor struct and its `impl` block into the corresponding
   `src/sdk/cli/` module.
2. Refactor `main()` into `pub fn run(file: PathBuf) -> Result<...>`.
3. Wire the subcommand in `antares_sdk.rs`.
4. Delete the old `src/bin/` file and its `[[bin]]` entry.

#### 3.1 Extract Shared Editor Helpers

Before migrating the editors, create `src/sdk/cli/editor_helpers.rs` with
the functions duplicated across all three:

| Function                       | Found In                        | Action     |
| ------------------------------ | ------------------------------- | ---------- |
| `truncate(s, max_len)`         | class, race, item editors       | Extract    |
| `filter_valid_proficiencies()` | class, race editors             | Extract    |
| `input_multistring_values()`   | class, race, item editors       | Extract    |
| `parse_multistring_input()`    | class, race editors (test only) | Extract    |
| `read_line()` / `prompt()`     | all editors (inline)            | Extract    |
| `STANDARD_PROFICIENCY_IDS`     | class, race editors             | Extract    |

#### 3.2 Migrate `class_editor.rs` → `antares-sdk class`

1. Move to `src/sdk/cli/class_editor.rs`.
2. Replace duplicated helpers with imports from `editor_helpers`.
3. Replace raw `env::args` with a `clap` struct:

```text
#[derive(Args)]
struct ClassArgs {
    /// Path to classes RON file
    #[arg(default_value = "data/classes.ron")]
    file: PathBuf,
}
```

#### 3.3 Migrate `race_editor.rs` → `antares-sdk race`

1. Move to `src/sdk/cli/race_editor.rs`.
2. **Fix**: Use `RaceDatabase::load_from_file()` instead of raw
   `ron::from_str()` for loading (matching the class editor pattern).
3. **Fix**: Normalize RON serialization config to match the class editor
   (`struct_names` setting should be consistent project-wide).
4. Replace duplicated helpers with imports from `editor_helpers`.

#### 3.4 Migrate `item_editor.rs` → `antares-sdk item`

1. Move to `src/sdk/cli/item_editor.rs`.
2. Replace duplicated helpers with imports from `editor_helpers`.
3. The `#[allow(deprecated)]` annotations on `Item` construction should be
   tracked and removed once the upstream `food` field migration completes
   (Game Cleanup Plan Phase 1.3).

#### 3.5 Migrate `map_builder.rs` → `antares-sdk map build`

1. Move to `src/sdk/cli/map_builder.rs`.
2. **Remove** the dead `npc` command entirely (currently just prints a
   deprecation notice).
3. Wire as a sub-subcommand under `Map`:

```text
#[derive(Subcommand)]
enum MapCommand {
    /// Interactive map builder REPL
    Build,
    /// Validate map RON files
    Validate(ValidateArgs),
}
```

4. The `rustyline` dependency stays — it's needed for the interactive REPL
   with history support.

#### 3.6 Testing Requirements

- All existing tests from all 4 editors migrate with them.
- Shared helper tests live in `editor_helpers.rs`.
- Verify that `truncate()`, `filter_valid_proficiencies()`, and
  `input_multistring_values()` behave identically to their per-editor
  originals.

#### 3.7 Success Criteria

- `antares-sdk class`, `race`, `item`, and `map build` work identically
  to the old standalone binaries.
- No duplicated helper functions remain.
- All editors use consistent RON serialization settings.
- `map_builder` `npc` command is gone.
- All quality gates pass.

---

### Phase 4: Cleanup and Polish (Low Risk)

**Estimated effort**: Small

#### 4.1 Remove Old Binaries and Cargo.toml Entries

Delete all old `src/bin/*.rs` files except `antares.rs`. Remove their
`[[bin]]` entries from `Cargo.toml`. The final state should be:

```text
[[bin]]
name = "antares"
path = "src/bin/antares.rs"

[[bin]]
name = "antares-sdk"
path = "src/bin/antares_sdk.rs"
```

#### 4.2 Add `--campaign` Flag to Editors

Add an optional `--campaign <DIR>` flag to the class, race, and item editors
so they can resolve file paths relative to a campaign directory:

```text
antares-sdk class --campaign campaigns/tutorial
# Equivalent to: antares-sdk class campaigns/tutorial/data/classes.ron
```

#### 4.3 Add Top-Level `--verbose` and `--quiet` Flags

Wire `tracing` subscriber initialization into the `antares-sdk` entry point
so all subcommands get consistent logging:

```text
antares-sdk --verbose campaign validate campaigns/tutorial
antares-sdk --quiet names --theme fantasy --number 100
```

#### 4.4 Update Documentation

- Update `docs/tutorials/name_generator_quickstart.md` to reference
  `antares-sdk names` instead of `cargo run --example name_generator_example`.
- Add a `docs/how-to/sdk_cli_usage.md` guide with examples for all
  subcommands.
- Update `docs/explanation/implementations.md` with the consolidation
  summary.

#### 4.5 Testing Requirements

- Add integration test that invokes `antares-sdk --help` and verifies all
  subcommands are listed.
- Add integration test that invokes each subcommand with `--help` and
  verifies it does not error.

#### 4.6 Success Criteria

- Only 2 binaries exist: `antares` (the game) and `antares-sdk` (all tools).
- `cargo run --bin antares-sdk -- --help` lists all subcommands.
- All quality gates pass.
- Documentation references the new command names.

## Migration Path for Users

| Old Command                                            | New Command                                             |
| ------------------------------------------------------ | ------------------------------------------------------- |
| `cargo run --bin name_gen -- -n 5 -t fantasy`          | `cargo run --bin antares-sdk -- names -n 5 -t fantasy`  |
| `cargo run --bin class_editor`                         | `cargo run --bin antares-sdk -- class`                  |
| `cargo run --bin class_editor -- path/to/classes.ron`  | `cargo run --bin antares-sdk -- class path/to/classes.ron` |
| `cargo run --bin race_editor`                          | `cargo run --bin antares-sdk -- race`                   |
| `cargo run --bin item_editor`                          | `cargo run --bin antares-sdk -- item`                   |
| `cargo run --bin map_builder`                          | `cargo run --bin antares-sdk -- map build`              |
| `cargo run --bin validate_map -- map.ron`              | `cargo run --bin antares-sdk -- map validate map.ron`   |
| `cargo run --bin campaign_validator -- campaigns/tutorial` | `cargo run --bin antares-sdk -- campaign validate campaigns/tutorial` |
| `cargo run --bin generate_terrain_textures`            | `cargo run --bin antares-sdk -- textures generate`      |
| `cargo run --bin update_tutorial_maps`                 | *(deleted — no replacement needed)*                     |

## What Gets Fixed Along the Way

| Issue                                    | Current State                                      | Fix                                          |
| ---------------------------------------- | -------------------------------------------------- | -------------------------------------------- |
| Hardcoded monster/item IDs               | `const VALID_MONSTER_IDS: &[u8] = &[1..10]`       | Load dynamically from campaign data          |
| Dead `npc` command in map builder        | Prints deprecation notice                          | Remove entirely                              |
| Inconsistent RON serialization           | class=`struct_names(true)`, race=`struct_names(false)` | Normalize to one convention             |
| Inconsistent CLI arg parsing             | 3 use `clap`, 5 use raw `env::args`, 2 use nothing | All use `clap` subcommands                  |
| Duplicated helper functions              | `truncate()`, `filter_valid_proficiencies()`, etc. | Extract to `editor_helpers.rs`               |
| Race editor bypasses database loader     | Raw `ron::from_str` instead of database loader     | Use `RaceDatabase::load_from_file()`         |
| Texture generator output dir hardcoded   | `CARGO_MANIFEST_DIR`                               | Add `--output-dir` flag                      |
| Event summary lumps unrelated variants   | `Furniture`, `Container`, etc. counted as `signs`  | Separate counters for each variant           |

## Dependencies Impact

**No new dependencies needed.** All currently used crates stay:

| Crate       | Used By              | Status    |
| ----------- | -------------------- | --------- |
| `clap`      | name_gen, campaign_validator, antares | Already in `Cargo.toml` |
| `rustyline`  | map_builder          | Already in `Cargo.toml` |
| `image`     | generate_terrain_textures | Already in `Cargo.toml` |
| `ron`       | All editors, validators   | Already in `Cargo.toml` |
| `serde`     | All editors, validators   | Already in `Cargo.toml` |
| `serde_json` | campaign_validator (JSON output) | Already in `Cargo.toml` |

## Appendix A: Files Deleted

| File                                  | Reason                                          |
| ------------------------------------- | ----------------------------------------------- |
| `src/bin/update_tutorial_maps.rs`     | One-time migration, already completed           |
| `src/bin/campaign_validator.rs`       | Merged into `antares-sdk campaign validate`     |
| `src/bin/class_editor.rs`            | Merged into `antares-sdk class`                 |
| `src/bin/race_editor.rs`             | Merged into `antares-sdk race`                  |
| `src/bin/item_editor.rs`             | Merged into `antares-sdk item`                  |
| `src/bin/name_gen.rs`                | Merged into `antares-sdk names`                 |
| `src/bin/map_builder.rs`             | Merged into `antares-sdk map build`             |
| `src/bin/validate_map.rs`            | Merged into `antares-sdk map validate`          |
| `src/bin/generate_terrain_textures.rs` | Merged into `antares-sdk textures generate`   |

## Appendix B: Files Created

| File                               | Purpose                                    |
| ---------------------------------- | ------------------------------------------ |
| `src/bin/antares_sdk.rs`           | Unified entry point with `clap` dispatch   |
| `src/sdk/cli/mod.rs`              | Module root — re-exports subcommand modules |
| `src/sdk/cli/editor_helpers.rs`   | Shared helpers for interactive editors     |
| `src/sdk/cli/names.rs`            | Name generator subcommand                  |
| `src/sdk/cli/class_editor.rs`     | Class editor subcommand                    |
| `src/sdk/cli/race_editor.rs`      | Race editor subcommand                     |
| `src/sdk/cli/item_editor.rs`      | Item editor subcommand                     |
| `src/sdk/cli/map_builder.rs`      | Map builder REPL subcommand                |
| `src/sdk/cli/map_validator.rs`    | Map validator subcommand                   |
| `src/sdk/cli/campaign_validator.rs` | Campaign validator subcommand            |
| `src/sdk/cli/texture_generator.rs` | Texture generator subcommand              |
