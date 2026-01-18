# TwoColumnLayout::show → show_split Migration Plan

Date: 2025-01-xx
Author: Antares dev (automation-assisted)

## Overview

This document describes the migration plan to replace all usages of the legacy `TwoColumnLayout::show(...)` API with the more robust `TwoColumnLayout::show_split(...)` API throughout the SDK campaign builder UI code. Additionally, it covers our choice to remove/back out of keeping the old API and replaces it with the recommended pattern to avoid lingering run-time/layout regressions.

## Why

- `TwoColumnLayout::show(...)` does not reliably invoke the developer-provided closure; a bug in the helper caused editors to render blank content.
- `TwoColumnLayout::show_split(...)` is explicit and easier to reason about: it expects two separate left & right closures, which avoids complicated borrow/lifetime issues and is robust for typical editor layout patterns.
- Since we control all in-repo consumers, we can break backward compatibility in our own code base and perform a hard migration to the `show_split` API — this simplifies the implementation and avoids complex compatibility wrappers.

## Goals

1. Replace all uses of `TwoColumnLayout::show` with `TwoColumnLayout::show_split` in `sdk/` code, tests, and compatible examples.
2. Remove or deprecate the `show` implementation and replace it with a short, explicit fallback that helps find old usage and forces migration (e.g., `panic!` or `#[deprecated]`).
3. Ensure all editors continue to render properly — specifically targeted editors: `races_editor.rs` (already migrated), `items_editor.rs`, `monsters_editor.rs`, `maps_editor.rs`, `characters_editor.rs`, `conditions_editor.rs`, `dialogue_editor.rs`, and any other editors in the `campaign_builder` SDK package.
4. Add tests and validation to verify each editor renders correctly after migration.
5. Run the CI checks (formatting/compile/lint/tests) and fix regressions.

## Scope

- Primary repo location: `antares/sdk/campaign_builder/src/`
- Files to check and update:
  - `races_editor.rs` (already migrated)
  - `items_editor.rs`
  - `monsters_editor.rs`
  - `maps_editor.rs`
  - `characters_editor.rs`
  - `classes_editor.rs`
  - `conditions_editor.rs`
  - `dialogue_editor.rs`
  - `quest_editor.rs`
  - `spells_editor.rs`
  - Any `tests` that contain sample or example code using `TwoColumnLayout::show(...)`
- UI helpers:
  - `ui_helpers.rs` (update or remove legacy `show`; add explicit deprecation or panic)
- Tests and docs:
  - `sdk/campaign_builder/src/test_utils.rs` (patterns & compliance checks)
  - `docs/*` references that discuss `show` usage or sample code

## Non-goals

- We will not try to preserve every usage pattern for `show` in the runtime; the plan intentionally breaks backward compatibility where `show` was used. Because the codebase is in-house and maintained by the team, this is acceptable.

## High-level approach

Phase 0 — Prep

- Search the entire `sdk/campaign_builder` source for occurrences of `TwoColumnLayout::new(...).show(...`.
- Compile a list of code lines & tests (including automated test cases) using it.
- If CI or local test harness is slow, target the following modules first: `races_editor`, `items_editor`, `monsters_editor`, `maps_editor`, `characters_editor`.

Phase 1 — Editor migration

- For each editor `X` that uses `.show`, convert it to `.show_split`:
  - Replace:
    ```
    TwoColumnLayout::new("id")
        .with_left_width(w)
        .show(ui, |left_ui, right_ui| {
            // left and right code in a single closure that uses left_ui and right_ui
        });
    ```
  - With:
    ```
    TwoColumnLayout::new("id")
        .with_left_width(w)
        .show_split(ui,
            |left_ui| {
                // left code moved here
            },
            |right_ui| {
                // right code moved here
            });
    ```
  - If the pre-migration code used nested `ScrollArea` calls in the closure (it often will), keep the left's `ScrollArea` lines inside the left closure and the right's inside the right closure. The split API requires explicit handling in each closure but otherwise is identical.
  - For editors that used `show` with single `ui` closure (test samples), replace them with `show_split` and add stub left/right closures in the snippet.

Phase 2 — Update test fixtures & examples

- Update `main.rs` test snippet used by `check_editor_compliance`:
  - Replace `TwoColumnLayout::new("good").show(ui, |ui| {});` with `TwoColumnLayout::new("good").show_split(ui, |left_ui| {}, |right_ui| {});`. (Adjust for readability)
- Update `test_utils.rs` patterns that match `show(...)` to accept `show_split(...)`.
- Update any documentation references or code examples in `docs/explanation/*` to use `show_split`.

Phase 3 — Remove `show` or warn users

- Replace `TwoColumnLayout::show` implementation:
  - Option A: Remove method entirely and/or add compile-time `#[deprecated]` attribute + `panic!("TwoColumnLayout::show is removed; use show_split")`.
  - Option B: Keep `show` as `#[deprecated]` and have it call `show_split` if possible — BUT since older closure types are incompatible, make it a `compile_error!()` or `panic!()` in debug so the user is forced to switch to `show_split`.
  - We prefer Option A for this migration — keep the code small and fail fast.

Phase 4 — QA and validation

- Format and lint:
  - `cargo fmt --all`
  - `cargo check --all-targets --all-features`
  - `cargo clippy --all-targets --all-features -- -D warnings` (address warnings as needed)
- Unit tests: run `cargo test -p campaign_builder` and address any compile or test failures.
- Manual UI validation (smoke test): Launch the Campaign Builder and inspect:
  - Items Editor
  - Races Editor
  - Monsters Editor
  - Maps Editor
  - Characters Editor
  - Asset Manager
  - Confirm content is visible in both left and right panes, action buttons work (Duplicate/Edit/Delete/etc.), scroll bars appear for long lists, and detail panels display correctly.
  - Confirm `Status` messages and `Open Races Editor & Load` behaviors still work.

## Detailed code migration checklist

- Convert usage pattern:
  - Search for: `TwoColumnLayout::new("...").with_left_width(...).show(ui, |left_ui, right_ui| {`
  - For each match:
    1. Separate the left and right content blocks into their own closures.
    2. Replace `show` with `show_split`.
    3. Ensure the left closure `left_ui` includes list rendering, scroll area, context menus etc.
    4. Ensure the right closure `right_ui` includes detail preview, action buttons, grid/labels etc.
    5. Keep `let mut action_to_perform` logic at the outer scope as needed, but avoid double borrowing: for closures that share control variables, prefer `Cell`, `Rc<RefCell>`, or `Atomic` where necessary for simultaneous mutation inside left & right closures.
- Tests to update:
  - `sdk/campaign_builder/src/main.rs`: test `good_content` sample
  - `sdk/campaign_builder/src/test_utils.rs`: test sample editor snippets
  - Any inline code snippets in test constants representing editor UI patterns.
- Edge-case conversions:
  - If an editor used `show` with code that expects both `left_ui` and `right_ui` simultaneously (rare), move their logic into `show_split` with explicitly separate left/right closures.
  - If a closure does operations that require both `left` and `right` Ui references simultaneously (rare), move cross-column logic to “after layout” handling, using commands / `Cell`/temporary state to capture events during layout.

## Testing details & examples

- Before → After conversion sample:
  - Before:
    ```rust
    TwoColumnLayout::new("races_layout")
      .with_left_width(250.0)
      .show(ui, |left_ui, right_ui| {
          // left_ui code (list)
          // right_ui code (detail)
      });
    ```
  - After:
    ```rust
    TwoColumnLayout::new("races_layout")
      .with_left_width(250.0)
      .show_split(ui,
        |left_ui| {
           // left list code
        },
        |right_ui| {
           // right details code
        });
    ```
- Test case: `test_two_column_layout_show_split_calls_both_closures`:
  - Confirm the `show_split` passes both closures and the content that manipulates both `Ui`s is rendered without errors.

## Validation checklist

- [ ] All editors in `sdk/campaign_builder/src` use `show_split` or equivalent.
- [ ] `TwoColumnLayout::show` is either removed or explicitly deprecated with a clear message.
- [ ] No `show` usages left in `sdk/campaign_builder` source or included code snippets in tests or docs.
- [ ] `cargo check --all-targets --all-features` completes cleanly.
- [ ] `cargo test -p campaign_builder` succeeds (or fails only for unrelated issues that pre-existed).
- [ ] UI smoke test: open Campaign Builder, navigate to all editors (Items, Spells, Races, Monsters, Maps, Characters, Assets) and verify content is displayed.
- [ ] Document the migration in `docs/explanation/implementations.md` and `CHANGELOG`.

## Rollback Plan

- If we detect problems after migration:
  1. Revert the change to the prior commit (maintain backup `.bak` files if present).
  2. Re-apply the `show_split` changes incrementally per editor and revalidate.
  3. If `TwoColumnLayout::show` needs to be re-introduced as compatibility wrapper, we can reintroduce a runtime panic earlier so devs see the failure point.

## Notes & Known Pitfalls

- When converting closures, pay attention to shared variables mutated by both left and right sections (for example `action_to_perform`/`new_selection`). Use `std::cell::Cell` or `Rc<RefCell<..>>` in the editor to avoid multi-borrow compiler errors when state is mutated inside both closures.
- `show_split` provides a much clearer API; prefer moving code blocks into explicit left or right closures. This makes the UI easier to reason about and avoids complicated borrow checker issues.
- Ensure style: follow existing UI patterns (ScrollArea, set_min_height, id_salt) for consistent behavior across editors.
- Keep `ui_helpers` a lightweight and stable set of helpers after the migration.

## Documentation & dev note

- Update docs:
  - `docs/explanation/implementations.md`
  - `docs/reference/ui_helpers.md` (if exists)
  - `docs/explanation/campaign_builder_ui_completion_plan.md` to indicate this migration
- Add a short developer note / comment in `ui_helpers.rs`:
  - Mark `show` as `#[deprecated]` if we keep it, with explicit direction to use `show_split` and show example code.

## Next actions

1. Scan codebase (`sdk/campaign_builder/src`) for `.show(` usages and convert them to `.show_split` (append the left & right logic accordingly). Start with the most obvious editors and tests (races editor already converted).
2. Run the checks, fix compile errors, adjust closures for shared state using `Cell` or `Rc<RefCell>`.
3. Update `ui_helpers.rs` to remove or deprecate `show` method; mark `show` as removed or replaced.
4. Run the full local quality checks (fmt / check / clippy / tests).
5. Run the UI smoke test: open the Campaign Builder and walk through all editors (focus on the asset manager).
6. If needed, implement added smoke tests to ensure all editors render content (e.g., `races_editor_state` shows content in `races_editor.rs`).

If you want, I’ll go through the codebase now and make the conversions and changes:

- Convert all `show` usages to `show_split`.
- Deprecate `show` in `ui_helpers.rs`.
- Add tests for `TwoColumnLayout::show_split`.
- Run `cargo fmt` / `cargo check` for the SDK package.
- If you want, I’ll also add a small debug `status_message` update to the Asset Manager as a temporary validation aid to make sure scanning is properly working in your environment.

Would you like me to proceed with those code updates now?
