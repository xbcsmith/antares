# Months and Years Time System Implementation Plan

## Overview

Extend the Antares time system from `day/hour/minute` to `year/month/day/hour/minute`.
This lets campaign authors gate events by season or year, display a full calendar date
on the HUD clock, and give long-running campaigns a richer sense of history. All
existing tests must continue to pass; save-file backward compatibility is preserved via
`#[serde(default)]`.

## Current State Analysis

### Existing Infrastructure

- **`GameTime`** in `src/domain/types.rs`: `{ day, hour, minute }` with `advance_minutes()`, `advance_hours()`, `advance_days()`, `time_of_day()`, `is_night()`, `is_day()`.
- **`TimeCondition`** in `src/domain/world/types.rs`: `DuringPeriods`, `AfterDay`, `BeforeDay`, `BetweenHours` with `is_met()`.
- **`GameState::advance_time()`** in `src/application/mod.rs`: delegates to `GameTime::advance_minutes()`, ticks spells, triggers daily restock.
- **HUD clock** in `src/game/systems/hud.rs`: `ClockDayText` marker, `update_clock()` system, `format_clock_day()` and `format_clock_time()` helpers.
- **Campaign config** in `src/sdk/campaign_loader.rs`: `CampaignConfig.starting_time: GameTime`.
- **Campaign Builder** in `sdk/campaign_builder/src/campaign_editor.rs`: `CampaignMetadataEditBuffer` with `starting_day`, `starting_hour`, `starting_minute`.
- **Campaign Builder lib** in `sdk/campaign_builder/src/lib.rs`: `CampaignMetadata.starting_time`.

### Identified Issues

- No concept of month or year; all time is tracked as an unbounded day counter.
- The HUD clock shows only "Day N" — no calendar date.
- `TimeCondition` has no month- or year-based variants; campaign authors cannot express seasonal or yearly conditions.

## Implementation Phases

### Phase 1: Core Time System (Phase A)

Extend `GameTime`, add calendar constants, update `advance_minutes()` rollover logic.

#### 1.1 Foundation Work

##### [MODIFY] `src/domain/types.rs`

1. **Add calendar constants** above `GameTime`:
   - `MONTHS_PER_YEAR: u32 = 12`
   - `DAYS_PER_MONTH: u32 = 30`
   - `DAYS_PER_YEAR: u32 = MONTHS_PER_YEAR * DAYS_PER_MONTH` (360)

2. **Extend `GameTime` struct** with `year: u32` (1-based) and `month: u32` (1-based), ordered before `day`. Both fields get `#[serde(default = "default_year")]` / `#[serde(default = "default_month")]` where defaults return `1`.

3. **Keep `GameTime::new(day, hour, minute)`** as backward-compatible constructor that sets `year = 1, month = 1`.

4. **Add `GameTime::new_full(year, month, day, hour, minute)`** constructor.

5. **Update `advance_minutes()`** so after computing `self.day += days` it also rolls:
   - While `self.day > DAYS_PER_MONTH`: `self.day -= DAYS_PER_MONTH; self.month += 1`
   - While `self.month > MONTHS_PER_YEAR`: `self.month -= MONTHS_PER_YEAR; self.year += 1`

6. **Update `advance_days()`** to call the same rollover logic (currently it does bare `self.day += days`).

7. **Add `total_days()` helper** that computes cumulative days from `(year, month, day)`:
   `(year - 1) * DAYS_PER_YEAR + (month - 1) * DAYS_PER_MONTH + day`.
   This is used by `TimeCondition::AfterDay` and `BeforeDay` so they continue to
   check total elapsed days (not day-within-month), preserving existing RON data
   behavior.

8. **Update all doctests** on `GameTime` struct and methods.

#### 1.2 Serde Backward Compatibility

Fields use `#[serde(default)]` so existing save files and RON data deserialize with `year = 1, month = 1`.

#### 1.3 Testing Requirements

- **Existing tests**: 8 `GameTime` unit tests in `src/domain/types.rs` must continue to pass as-is (the `new()` constructor still works).
- **New rollover tests**:
  - `test_advance_minutes_day_to_month_rollover` — advancing past day 30 increments `month`.
  - `test_advance_minutes_month_to_year_rollover` — advancing past month 12 increments `year`.
  - `test_advance_minutes_multi_year_rollover` — advancing a large number of minutes rolls through multiple years.
  - `test_advance_days_with_month_rollover` — `advance_days(31)` on day 1/month 1 → day 1/month 2.
  - `test_new_full_constructor` — verify all five fields.
  - `test_serde_default_year_month` — deserializing RON `(day: 5, hour: 8, minute: 0)` yields `year=1, month=1`.
  - `test_total_days_basic` — year 1, month 2, day 10 → total 40 days.
  - `test_total_days_adventure_span` — 13×28 calendar: month 2 day 10 to month 3 day 12 = 30 days elapsed.

#### 1.4 Deliverables

- [ ] Calendar constants added
- [ ] `GameTime` extended with `year` and `month`
- [ ] `new_full()` constructor
- [ ] `advance_minutes()` rolls month/year
- [ ] `advance_days()` rolls month/year
- [ ] Updated doctests pass
- [ ] New rollover unit tests pass

#### 1.5 Success Criteria

All existing `GameTime` tests pass unchanged; new tests cover minute→hour→day→month→year rollover.

### Phase 2: TimeCondition Variants (Phase B)

Add month- and year-aware event conditions.

#### 2.1 Feature Work

##### [MODIFY] `src/domain/world/types.rs`

1. **Add four new variants** to `TimeCondition`:

   - `DuringMonths(Vec<u32>)` — fires when `game_time.month` is in the list (e.g. winter = `[11, 12, 1]`)
   - `AfterYear(u32)` — fires when `game_time.year > threshold`
   - `BeforeYear(u32)` — fires when `game_time.year < threshold`
   - `BetweenYears { from: u32, to: u32 }` — fires when `from <= game_time.year <= to`

2. **Add match arms** to `is_met()` for each new variant.

3. **Update top-level enum doc comment** and doctests to include examples of the new variants.

#### 2.2 Testing Requirements

- **Existing tests**: 15+ `TimeCondition` tests in `src/domain/world/types.rs` must continue to pass.
- **New tests**:
  - `test_during_months_fires_in_winter` — month 12 matches `[11, 12, 1]`
  - `test_during_months_skips_summer` — month 6 not in `[11, 12, 1]`
  - `test_after_year_fires` — year 3 > threshold 2
  - `test_after_year_skips` — year 2 not > threshold 2
  - `test_before_year_fires` — year 1 < threshold 3
  - `test_before_year_skips` — year 3 not < threshold 3
  - `test_between_years_fires` — year 2 in [1, 3]
  - `test_between_years_skips` — year 5 not in [1, 3]
  - RON roundtrip tests for each new variant

#### 2.3 Deliverables

- [ ] Four new `TimeCondition` variants
- [ ] `is_met()` extended
- [ ] Doctests updated
- [ ] Unit tests pass
- [ ] RON roundtrip tests pass

#### 2.4 Success Criteria

All existing `TimeCondition` tests pass unchanged; new variants are fully tested for match/skip/serialization.

### Phase 3: HUD Clock Update (Phase C)

Display a full date on the HUD.

#### 3.1 Feature Work

##### [MODIFY] `src/game/systems/hud.rs`

1. **Rename** `ClockDayText` → `ClockDateText`. Update all references (marker component, queries, tests).

2. **Replace `format_clock_day(day: u32)`** with `format_clock_date(year: u32, month: u32, day: u32)`:
   - Returns `"Y{year} M{month} D{day}"` (compact format that fits the existing clock panel width).

3. **Update `update_clock()`** system to pass `game_time.year`, `game_time.month`, `game_time.day` to the new formatter.

#### 3.2 Testing Requirements

- **Existing tests**: `format_clock_day` tests will be updated to match the new signature/output.
- **New/updated tests**:
  - `test_format_clock_date_defaults` — `(1, 1, 1)` → `"Y1 M1 D1"`
  - `test_format_clock_date_large_values` — `(4, 12, 30)` → `"Y4 M12 D30"`
- Bevy ECS test `test_hud_clock_day_text_is_spawned` updated to use `ClockDateText`.

#### 3.3 Deliverables

- [ ] `ClockDayText` → `ClockDateText` rename
- [ ] `format_clock_date()` implemented
- [ ] `update_clock()` updated
- [ ] HUD tests updated

#### 3.4 Success Criteria

Clock HUD shows `Y1 M1 D{N}` at startup and properly updates on time advance.

### Phase 4: Campaign Builder & Config (Phase D)

Extend campaign starting time to include year and month.

#### 4.1 Feature Work

##### [MODIFY] `src/sdk/campaign_loader.rs`

- `default_starting_time()` returns `GameTime::new(1, 8, 0)` — no change needed since `new()` defaults `year=1, month=1`.

##### [MODIFY] `sdk/campaign_builder/src/campaign_editor.rs`

1. **Add fields** `starting_year: u32` and `starting_month: u32` to `CampaignMetadataEditBuffer`.
2. **Update `from_metadata()`** to read `m.starting_time.year` and `m.starting_time.month`.
3. **Update `apply_to()`** to call `GameTime::new_full(year, month, day, hour, minute)` with clamping (`year.max(1)`, `month.clamp(1, 12)`).

##### [MODIFY] `sdk/campaign_builder/src/lib.rs`

- `CampaignMetadata.starting_time` already uses `GameTime` — no schema change needed since the struct itself gains the fields.
- `default_starting_time()` in lib.rs returns `GameTime::new(1, 8, 0)` — inherits defaults.

#### 4.2 Testing Requirements

- **Existing tests** in `sdk/campaign_builder/src/campaign_editor.rs` must pass.
- **New tests**:
  - `test_buffer_from_metadata_copies_starting_year_month`
  - `test_buffer_apply_to_writes_starting_year_month`
  - `test_buffer_starting_time_clamps_month`

#### 4.3 Deliverables

- [ ] `starting_year` / `starting_month` in edit buffer
- [ ] `from_metadata()` / `apply_to()` updated
- [ ] Tests pass

#### 4.4 Success Criteria

Campaign Builder can round-trip year/month values; defaults correctly populate.

### Phase 5: Fixup All Call Sites & Quality Gates (Phase E)

#### 5.1 Fix All Call-Site References

Every `GameTime::new(...)` call across the codebase already works (three-arg constructor preserved). Grep for `.day` access on `GameTime` to confirm semantics still hold with the new month-based "day within month" meaning.

> **Decision**: The `day` field changes meaning from "total elapsed days" to "day within
> month (1–30)". `TimeCondition::AfterDay` and `BeforeDay` use `GameTime::total_days()`
> so they continue to check cumulative elapsed days, preserving existing behavior.
> Any other code that accessed `.day` as a cumulative counter is updated to use
> `total_days()` instead.

#### 5.2 Quality Gates

Run in order:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

#### 5.3 Documentation

Update `docs/explanation/implementations.md` with a section describing the month/year extension.

#### 5.4 Deliverables

- [ ] All quality gates pass (zero errors, zero warnings)
- [ ] `implementations.md` updated

#### 5.5 Success Criteria

Full `cargo nextest run` green. No clippy warnings.
