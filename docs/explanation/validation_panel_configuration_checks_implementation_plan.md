## Plan: Validation UI & Configuration Checks

Improve validation panel layout to match the Assets Manager, add campaign configuration checks, and preserve behavior of showing only categories with results. The UI will be made table-like and scrollable; configuration checks will generate `ValidationResult`s in `ValidationCategory::Configuration`.

**Steps:**
1. Rework validation panel layout to table/grid; add vertical scroll area.
2. Add `Configuration` validation checks in `validate_campaign` (starting map, party/roster, starting level, etc.).
3. Wire checks to push `ValidationResult::Configuration` results into `self.validation_errors`.
4. Add unit + integration tests; update docs and examples.
5. Final polish, performance checks, and CI validation.

**Open Questions:**
1. Should configuration checks be `Error` (blocking), `Warning`, or `Info` in general?
2. Should empty categories show (Option A: always show / Option B: only categories with results)? (You selected B.)
3. Any additional campaign config rules required (e.g., max values)?

---

# Validation UI & Campaign Configuration Checks Implementation Plan

## Overview
Add meaningful configuration validations (starting map existence, party/roster limits, starting level constraints) that produce `ValidationCategory::Configuration` results and upgrade the validation panel UI to match the Assets Manager’s table layout including a vertical scroll area. This will provide clearer, consistent validation UX and ensure the Configuration category appears when necessary.

## Current State Analysis

### Existing Infrastructure
- Validation data model: `sdk/campaign_builder/src/validation.rs` defines `ValidationCategory`, `ValidationSeverity`, `ValidationResult`, helpers like `ValidationSummary::from_results` and `group_results_by_category`.
- Validation engine: `sdk/campaign_builder/src/main.rs` exposes multiple `validate_*` functions (e.g., `validate_item_ids`, `validate_spell_ids`, `validate_monster_ids`) and a `validate_campaign` method that aggregates results into `self.validation_errors`.
- UI: `sdk/campaign_builder/src/main.rs::show_validation_panel` composes the validation UI using group_results_by_category and currently displays categories only when there are results.
- Validation tests: Basic tests already exist in `main.rs` (e.g., `test_validation_empty_id`), but campaign configuration checks are currently missing.

### Identified Issues
- The validation panel’s layout is not consistent with the Assets Manager grid and lacks a proper scroll area — readability issues and no consistent column layout.
- `ValidationCategory::Configuration` is not used by existing validators, so “Campaign Configuration” won’t show because there are no results for it.
- Tests for configuration checks don’t exist yet; we must add them.
- Need to preserve current UX preference: only show categories that have results.

---

## Implementation Phases

### Phase 1: Core (Validation UI layout & Foundation)
#### 1.1 Foundation Work
- Implement a `validation_panel_layout` using table/grid patterns modeled on `Assets Manager` output.
- Identify the shared UI helpers in `sdk/campaign_builder/src/ui_helpers.rs` to reuse table header and grid layout code.

#### 1.2 Add Foundation Functionality
- Replace the validation panel layout in `sdk/campaign_builder/src/main.rs::show_validation_panel`:
  - Use a `egui` scroll area with a constrained `max_height` and `striped(true)` grid rows.
  - Create column headers (Status / Message / File) with aligned widths.
  - Keep the `Re-validate` and filter controls at the top, as current.
- Maintain existing summary badges:
  - Use `ValidationSummary::from_results(&self.validation_errors)` for counts and status.
- Group results using `validation::group_results_by_category`.

#### 1.3 Integrate Foundation Work
- Render UI per category (only categories with results):
  - For each `(category, results)` display a category header (icon and count) and the table/grid of results.
  - Ensure “passed” severity is optionally shown only if toggled (default to hidden if no results).
- Add a `validation_panel_scroll` `egui::ScrollArea::vertical()` with `id_salt` to the panel.
- Keep `self.validation_errors` as the single source of truth for all categories.

#### 1.4 Testing Requirements
- Visual manual verification to ensure UI layout matches Assets Manager grid:
  - Column alignment, icons, padding.
  - Scrollbar shows as expected and the table uses a consistent layout for rows.
  - Check for proper `striped` rows and accessible color choices.

#### 1.5 Deliverables
- `sdk/campaign_builder/src/main.rs` validation panel uses table layout and scroll area.
- New `ui_helpers` table helper function(s) created if not available:
  - e.g., `ui_helpers::show_table_header(...)`.

#### 1.6 Success Criteria
- Validation panel visually matches Assets Manager’s structure and grid.
- The panel is scrollable, column-aligned, and uses icons for severity.
- No UI regressions in existing validation display.

---

### Phase 2: Feature Implementation (Campaign Configuration Checks)
#### 2.1 Feature Work
- Add `Configuration` checks to `validate_campaign` in `main.rs`:
  - Add `ValidationResult::error(validation::ValidationCategory::Configuration, "...")` or `warning` depending on rule severity.
  - Example checks:
    - Starting Map exists -> Error (if invalid)
    - `starting_level` must be between 1 and `max_level` -> Error
    - `max_roster_size >= max_party_size` -> Error if not
    - Starting resources constraints (starting_gold/starting_gems/starting_food) -> Warning or Info
- Use the `ValidationSeverity` types as appropriate: `Error` for blocking issues, `Warning` for user attention.

#### 2.2 Integrate Feature
- Ensure configuration checks run in `validate_campaign()` along with `validate_item_ids`, `validate_monster_ids`, etc.
- Add `ValidationResult::passed` checks only if you want the UI to show passed checks later. (Default: do not add passed unless requested)

#### 2.3 Configuration Updates
- Add relevant configuration details to `docs/explanation` — mention what configuration checks are enforced (starting map, levels, party/roster, resource bounds).
- Identify any configurable thresholds (e.g., maximum levels) and add them to `config` or `docs`.

#### 2.4 Testing Requirements
- Unit tests in `sdk/campaign_builder/src/main.rs`:
  - `test_validation_starting_map_missing` — ensure missing map triggers `ValidationCategory::Configuration` error.
  - `test_validation_roster_size_less_than_party` — ensures error generated and category is Configuration.
  - `test_validation_starting_level_invalid` — for boundaries.
- Integration: Add tests verifying that category `Configuration` appears in `grouped` results once a configuration error is present.

#### 2.5 Deliverables
- New validation checks in `validate_campaign()` for `ValidationCategory::Configuration`.
- Corresponding unit tests to validate behavior and error messages.
- Updated validation tests/CI entries to run them.

#### 2.6 Success Criteria
- When campaign configuration fails checks, "Campaign Configuration" appears in UI with the new errors or warnings.
- Tests assert `ValidationResult` with `ValidationCategory::Configuration` are present and have correct severity and messages.

---

### Phase 3: UI Behavior & Polishing
#### 3.1 Feature Work
- Add filters and controls to the new validation table:
  - Filter to show “Errors only” or “Warnings only” (future enhancement).
  - Optional “Show passed checks”.
  - Tooltips for severity icons and file path (hover to show the absolute path).
- Make file paths clickable, possibly with command to open the file or highlight the asset in the Asset Manager.

#### 3.2 Integrate Feature
- Add optional `ui_helpers` style function to display severity icons and file link actions (potentially reused by Asset Manager).
- Ensure consistent look & feel across other panels (e.g., Assets Manager).

#### 3.3 Configuration Updates
- Add or update `docs/explanation/implementations.md` with a summary of the new validation checks and UI layout.

#### 3.4 Testing Requirements
- Visual check for correct filter behavior.
- Add unit/integration tests for clickable file path behavior (if supported by test harness).
- Add UI scenario test verifying that the filter affects the table correctly.

#### 3.5 Deliverables
- Filters and options available on validation panel.
- Accessibility updates and minor UX improvements aligned with the Assets Manager.

#### 3.6 Success Criteria
- Users can easily find configuration errors and filter them.
- UI resembles the Assets Manager and is consistent across the toolkit.

---

### Phase 4: Documentation, QA, and Developer Guidance
#### 4.1 Feature Work
- Update `sdk` docs:
  - `docs/explanation/validation_panel_configuration_checks_implementation_plan.md` (this file).
  - Update `docs/explanation/implementations.md` and `docs/explanation` references describing the new checks and the Validation panel design.

#### 4.2 Integrate Feature
- Add unit tests and CI checks to ensure:
  - `validate_campaign()` yields `Configuration` category results when appropriate.
  - Validation UI is rendered without errors.
  - No regressions in existing validations.

#### 4.3 Configuration Updates
- Add optional `UI` config or `validation` toggles in `main.rs` if user wants to enable/disable specific checks in the future.

#### 4.4 Testing Requirements
- Run `cargo fmt`, `cargo check`, `cargo clippy`, and `cargo test` across `sdk/campaign_builder`.
- Create test cases for new configuration checks, and visually verify the validation panel.

#### 4.5 Deliverables
- Updated docs and tests, CI passing.
- QA sign-off for the new validation UI and checks.

#### 4.6 Success Criteria
- All unit and integration tests for validation checks pass.
- UX review sign-off confirmed that validation panel matches the preferred assets manager layout and Behavior.

---

## Notes on Implementation & Style
- Keep the default behavior: only categories with results should show in the validation panel (as requested).
- Add configuration checks that are meaningful (blocking checks should use `Error`, less severe checks should use `Warning`).
- Make sure to re-use UI helper components if available and keep new UI code consistent with `ui_helpers.rs`.
- Add tests and maintainers documentation to prevent developers from inadvertently reintroducing the `auto-select` or incomplete UI formatting issues.

## Final Remarks
This plan is written to be iterative and phased, focusing on the UI first (matching Assets Manager), then adding meaningful `Configuration` validations and tests. Once you’re satisfied with the plan, I’ll proceed to implementation in the defined phases and report modifications and tests for review.
