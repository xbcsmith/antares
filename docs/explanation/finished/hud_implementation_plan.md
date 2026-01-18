# HUD Feature Implementation Plan

## Overview

Add a comprehensive party HUD to the Antares game engine that displays:

- Character names and portraits (placeholder initially)
- HP bars with **exact HP values** (current/max format like "45/100 HP")
- Active condition indicators with color-coded status
- Compass showing party facing direction (N/E/S/W)

### Design Goals

- **Compact layout**: Minimal vertical screen space usage
- **6 characters max**: Supports full party of 6 in a single horizontal strip
- **Screen efficiency**: Thin bar design to preserve gameplay viewport
- **Navigation awareness**: Compass display shows party facing direction during exploration

The HUD will enhance the existing UI system in `src/game/systems/ui.rs` with a
dedicated module providing richer visual feedback on party status.

## HUD Mockup

![HUD Mockup](hud_mockup.png)

## Current State Analysis

### Existing Infrastructure

| Component                 | Location                           | Purpose                                                                                  |
| ------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------- |
| `UiPlugin`                | `src/game/systems/ui.rs#L9-16`     | Basic party panel with name, HP, SP, level, class, race                                  |
| `Condition` bitflags      | `src/domain/character.rs#L432-506` | Status flags: FINE, ASLEEP, BLINDED, POISONED, PARALYZED, DEAD, etc.                     |
| `ActiveCondition`         | `src/domain/conditions.rs#L66`     | Data-driven conditions with duration and magnitude                                       |
| `ConditionDefinition`     | `src/domain/conditions.rs#L49`     | Condition metadata (name, description, effects, icon_id)                                 |
| `Character` struct        | `src/domain/character.rs#L870-920` | Contains `portrait_id: String`, `hp: AttributePair16`, `conditions`, `active_conditions` |
| `PARTY_MAX_SIZE` constant | `src/domain/character.rs#L101`     | Defined as `6` - maximum party members                                                   |
| `AttributePair16`         | `src/domain/character.rs`          | Contains `.current` and `.base` fields for HP values                                     |

### Identified Issues

1. **Minimal Condition Display**: Current UI shows only HP/SP values with no condition indicators
2. **No Color Coding**: No visual differentiation for health states or conditions
3. **No Portrait Support**: `portrait_id` field exists but is unused in the HUD
4. **Limited Visual Hierarchy**: All party members displayed identically regardless of status
5. **Magic Numbers**: Color values and thresholds not extracted as constants

---

## Architecture Compliance Check

**MANDATORY**: Before implementing, verify against `docs/reference/architecture.md`:

| Architecture Element | Location                       | Usage in HUD                                          |
| -------------------- | ------------------------------ | ----------------------------------------------------- |
| `PARTY_MAX_SIZE`     | `src/domain/character.rs#L101` | Loop bound for rendering party members (value: 6)     |
| `Character` struct   | `src/domain/character.rs#L870` | Read `name`, `hp`, `conditions`, `portrait_id` fields |
| `Condition` bitflags | `src/domain/character.rs#L432` | Check condition states with `.has()` method           |
| `ActiveCondition`    | `src/domain/conditions.rs#L66` | Display active condition durations                    |
| `AttributePair16`    | `src/domain/character.rs`      | Access `.current` and `.base` for HP values           |

**Verification Steps**:

1. Read `docs/reference/architecture.md` Section 4 (Core Data Structures) BEFORE coding
2. Ensure NO modifications to `Character`, `Condition`, or `AttributePair16` structs
3. Use EXACT type names as defined in architecture
4. Reference `PARTY_MAX_SIZE` constant, do not hardcode the value 6

---

## UI Technology Decision

> [!IMPORTANT] > **Use Native Bevy UI (`bevy_ui`) for Game HUD**
>
> The game HUD uses Bevy's native retained-mode UI system, NOT `bevy_egui`.
>
> - `bevy_ui`: For game engine UI (HUD, health bars, condition indicators)
> - `bevy_egui`: Reserved for SDK/editor tools (campaign builder, debug panels)

### Rationale

| Aspect          | `bevy_ui` (Native)                       | `bevy_egui`                     |
| --------------- | ---------------------------------------- | ------------------------------- |
| **Mode**        | Retained (built once, updated on change) | Immediate (rebuilt every frame) |
| **Performance** | Better for static HUDs                   | Overhead for fixed elements     |
| **Integration** | Seamless with Bevy ECS                   | Requires bridge crate           |
| **Layout**      | Taffy flexbox engine                     | egui layout                     |
| **Game Style**  | Custom, game-appropriate visuals         | Desktop application aesthetic   |
| **Use Case**    | Health bars, minimaps, score displays    | Debug tools, editor windows     |

---

## Prerequisites

**[VERIFY]** Bevy dependency exists in `Cargo.toml`:

```bash
grep "^bevy" Cargo.toml
```

Expected output: `bevy = { version = "0.17", default-features = true }`

**[VERIFY]** Required constants exist:

```bash
grep "pub const PARTY_MAX_SIZE" src/domain/character.rs
```

Expected output: `pub const PARTY_MAX_SIZE: usize = 6;`

**[VERIFY]** `Condition` methods exist:

```bash
grep -A 5 "impl Condition" src/domain/character.rs | grep "pub fn has"
```

Expected: `pub fn has(&self, flag: u8) -> bool`

---

## Implementation Decisions (FINAL)

These decisions are **FINAL** - do not deviate:

1. **HUD Position**: Bottom of screen using `PositionType::Absolute`
2. **Condition Display**: Use emoji + text labels (e.g., "💀 Dead", "☠️ Poisoned")
3. **Game Mode**: Single HUD design for all modes (combat, exploration, menu)
4. **Portrait Support**: Deferred to future work - Phase 3 is NOT in scope
5. **Color Constants**: ALL color values MUST be extracted as named constants
6. **Testing**: Unit tests are MANDATORY for all public functions
7. **Helper Function**: Use `spawn_character_card` for cleaner code and future reusability

---

## Implementation Phases

**Phase Overview:**

- **Phase 1**: Core HUD Module - Character cards with HP bars, conditions, and names
- **Phase 2**: Enhanced Condition Display - Priority-based condition display with multiple condition counting
- **Phase 3**: Compass Display - Navigation compass showing party facing direction (N/E/S/W)
- **Phase 4**: Portrait Support - DEFERRED for future implementation

### Phase 1: Core HUD Module

#### 1.1 Create HUD Module File

**[NEW FILE]** `src/game/systems/hud.rs`

**MANDATORY**: Add SPDX header as first lines:

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0
```

**Required imports** (add these after SPDX header):

```rust
use crate::domain::character::{Character, Condition, PARTY_MAX_SIZE};
use crate::domain::conditions::ActiveCondition;
use crate::game::resources::GlobalState;
use bevy::prelude::*;
```

> [!NOTE]
> This module uses **native Bevy UI** (`bevy_ui`), NOT `bevy_egui`.
> The `bevy_egui` crate is reserved for SDK/editor tools only.

**Constants to define** (extract ALL color values and thresholds):

```rust
// HP bar colors
pub const HP_HEALTHY_COLOR: Color = Color::srgb(0.39, 0.78, 0.39);
pub const HP_INJURED_COLOR: Color = Color::srgb(0.90, 0.71, 0.20);
pub const HP_CRITICAL_COLOR: Color = Color::srgb(0.86, 0.20, 0.20);
pub const HP_DEAD_COLOR: Color = Color::srgb(0.31, 0.31, 0.31);

// Condition colors
pub const CONDITION_POISONED_COLOR: Color = Color::srgb(0.20, 0.71, 0.20);
pub const CONDITION_PARALYZED_COLOR: Color = Color::srgb(0.39, 0.39, 0.78);
pub const CONDITION_BUFFED_COLOR: Color = Color::srgb(0.78, 0.71, 0.39);

// HP thresholds
pub const HP_HEALTHY_THRESHOLD: f32 = 0.75;
pub const HP_CRITICAL_THRESHOLD: f32 = 0.25;

// Layout constants
pub const HUD_PANEL_HEIGHT: Val = Val::Px(80.0);
pub const CHARACTER_CARD_WIDTH: Val = Val::Px(120.0);
pub const HP_BAR_HEIGHT: Val = Val::Px(16.0);
pub const CARD_PADDING: Val = Val::Px(8.0);
```

**Marker Components** (for querying HUD entities):

```rust
/// Marker component for the HUD root container
#[derive(Component)]
pub struct HudRoot;

/// Marker component for a character card in the HUD
#[derive(Component)]
pub struct CharacterCard {
    pub party_index: usize,
}

/// Marker component for HP bar background
#[derive(Component)]
pub struct HpBarBackground;

/// Marker component for HP bar fill (the colored portion)
#[derive(Component)]
pub struct HpBarFill {
    pub party_index: usize,
}

/// Marker component for HP text label
#[derive(Component)]
pub struct HpText {
    pub party_index: usize,
}

/// Marker component for condition text label
#[derive(Component)]
pub struct ConditionText {
    pub party_index: usize,
}

/// Marker component for character name label
#[derive(Component)]
pub struct CharacterNameText {
    pub party_index: usize,
}
```

**Plugin definition**:

```rust
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud)
           .add_systems(Update, update_hud);
    }
}
```

**Complete function implementations**:

````rust
/// Sets up the HUD UI hierarchy (runs once at startup)
///
/// Creates the HUD container and character card slots using Bevy's
/// native UI system with flexbox layout.
///
/// # Arguments
/// * `commands` - Bevy command buffer for spawning entities
fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                height: HUD_PANEL_HEIGHT,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
            HudRoot,
        ))
        .with_children(|parent| {
            for party_index in 0..PARTY_MAX_SIZE {
                spawn_character_card(parent, party_index);
            }
        });
}

/// Spawns a single character card UI entity hierarchy
///
/// Creates all child entities for one character's HUD display:
/// - Name text
/// - HP bar (background + fill)
/// - HP text
/// - Condition text
///
/// This is extracted as a helper for:
/// - Cleaner code organization
/// - Future reusability (character selection, inn screens, etc.)
/// - Easier testing and maintenance
/// - Supports dynamic spawning if needed later
///
/// # Arguments
/// * `parent` - Parent entity builder (from `.with_children()`)
/// * `party_index` - Index in party (0-5)
///
/// # Returns
/// Entity ID of the spawned character card root
///
/// # Examples
///
/// ```
/// commands.spawn(hud_root()).with_children(|parent| {
///     for i in 0..PARTY_MAX_SIZE {
///         spawn_character_card(parent, i);
///     }
/// });
/// ```
fn spawn_character_card(parent: &mut ChildBuilder, party_index: usize) -> Entity {
    parent
        .spawn((
            Node {
                width: CHARACTER_CARD_WIDTH,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(CARD_PADDING),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
            BorderRadius::all(Val::Px(4.0)),
            CharacterCard { party_index },
        ))
        .with_children(|card| {
            // Character name text
            card.spawn((
                Text::new(""),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                CharacterNameText { party_index },
            ));

            // HP bar container (background)
            card.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: HP_BAR_HEIGHT,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 1.0)),
                HpBarBackground,
            ))
            .with_children(|bar| {
                // HP bar fill (the colored part that changes width)
                bar.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(HP_HEALTHY_COLOR),
                    HpBarFill { party_index },
                ));
            });

            // HP text ("45/100 HP")
            card.spawn((
                Text::new(""),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                HpText { party_index },
            ));

            // Condition text ("☠️ Poisoned")
            card.spawn((
                Text::new(""),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                ConditionText { party_index },
            ));
        })
        .id()
}

/// Updates HUD elements based on current party state
///
/// This system runs every frame to sync UI with game state.
/// Updates HP bars, HP text, condition text, and character names.
///
/// # Arguments
/// * `global_state` - Game state containing party data
/// * `hp_bar_query` - Query for HP bar fill entities
/// * `hp_text_query` - Query for HP text entities
/// * `condition_text_query` - Query for condition text entities
/// * `name_text_query` - Query for character name text entities
fn update_hud(
    global_state: Res<GlobalState>,
    mut hp_bar_query: Query<(&HpBarFill, &mut Node, &mut BackgroundColor)>,
    mut hp_text_query: Query<(&HpText, &mut Text)>,
    mut condition_text_query: Query<(&ConditionText, &mut Text, &mut TextColor)>,
    mut name_text_query: Query<(&CharacterNameText, &mut Text)>,
) {
    let party = &global_state.0.party;

    // Update HP bars
    for (hp_bar, mut node, mut bg_color) in hp_bar_query.iter_mut() {
        if let Some(character) = party.members.get(hp_bar.party_index) {
            let hp_percent = character.hp.current as f32 / character.hp.base as f32;
            node.width = Val::Percent(hp_percent * 100.0);
            *bg_color = BackgroundColor(hp_bar_color(hp_percent));
        } else {
            // No character in this slot - hide bar
            node.width = Val::Px(0.0);
        }
    }

    // Update HP text
    for (hp_text, mut text) in hp_text_query.iter_mut() {
        if let Some(character) = party.members.get(hp_text.party_index) {
            **text = format_hp_display(character.hp.current, character.hp.base);
        } else {
            **text = String::new();
        }
    }

    // Update condition text
    for (condition_text, mut text, mut text_color) in condition_text_query.iter_mut() {
        if let Some(character) = party.members.get(condition_text.party_index) {
            let (cond_str, color) =
                get_priority_condition(&character.conditions, &character.active_conditions);
            **text = cond_str;
            *text_color = TextColor(color);
        } else {
            **text = String::new();
        }
    }

    // Update character names
    for (name_text, mut text) in name_text_query.iter_mut() {
        if let Some(character) = party.members.get(name_text.party_index) {
            **text = format!("{}. {}", name_text.party_index + 1, character.name);
        } else {
            **text = String::new();
        }
    }
}

/// Returns HP bar color based on health percentage
///
/// Uses threshold constants to determine color.
///
/// # Arguments
/// * `hp_percent` - Current HP as percentage (0.0 to 1.0)
///
/// # Returns
/// Bevy Color for the HP bar
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::hp_bar_color;
/// use bevy::prelude::Color;
///
/// let color = hp_bar_color(0.80);
/// // Returns HP_HEALTHY_COLOR
/// ```
pub fn hp_bar_color(hp_percent: f32) -> Color {
    if hp_percent >= HP_HEALTHY_THRESHOLD {
        HP_HEALTHY_COLOR
    } else if hp_percent >= HP_CRITICAL_THRESHOLD {
        HP_INJURED_COLOR
    } else {
        HP_CRITICAL_COLOR
    }
}

/// Formats HP display as "current/max HP"
///
/// # Arguments
/// * `current` - Current HP value
/// * `max` - Maximum HP value
///
/// # Returns
/// Formatted string like "45/100 HP"
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::format_hp_display;
///
/// let display = format_hp_display(45, 100);
/// assert_eq!(display, "45/100 HP");
/// ```
pub fn format_hp_display(current: u16, max: u16) -> String {
    format!("{}/{} HP", current, max)
}

/// Returns the highest priority condition for display
///
/// Priority order (highest to lowest):
/// - DEAD/STONE/ERADICATED (100)
/// - UNCONSCIOUS (90)
/// - PARALYZED (80)
/// - POISONED (70)
/// - DISEASED (60)
/// - BLINDED (50)
/// - SILENCED (40)
/// - ASLEEP (30)
/// - Active buffs (10)
/// - FINE (0)
///
/// # Arguments
/// * `conditions` - Character's condition bitflags
/// * `active_conditions` - List of active condition effects
///
/// # Returns
/// Tuple of (condition_text, condition_color)
///
/// # Examples
///
/// ```
/// use antares::domain::character::Condition;
/// use antares::game::systems::hud::get_priority_condition;
///
/// let mut conditions = Condition::new();
/// conditions.add(Condition::DEAD);
/// let (text, color) = get_priority_condition(&conditions, &[]);
/// assert!(text.contains("Dead"));
/// ```
pub fn get_priority_condition(
    conditions: &Condition,
    active_conditions: &[ActiveCondition],
) -> (String, Color) {
    if conditions.has(Condition::DEAD) {
        return ("💀 Dead".to_string(), HP_DEAD_COLOR);
    }
    if conditions.has(Condition::STONE) {
        return ("🗿 Stone".to_string(), HP_DEAD_COLOR);
    }
    if conditions.has(Condition::ERADICATED) {
        return ("💀 Eradicated".to_string(), HP_DEAD_COLOR);
    }
    if conditions.has(Condition::UNCONSCIOUS) {
        return ("💤 Unconscious".to_string(), CONDITION_PARALYZED_COLOR);
    }
    if conditions.has(Condition::PARALYZED) {
        return ("⚡ Paralyzed".to_string(), CONDITION_PARALYZED_COLOR);
    }
    if conditions.has(Condition::POISONED) {
        return ("☠️ Poisoned".to_string(), CONDITION_POISONED_COLOR);
    }
    if conditions.has(Condition::DISEASED) {
        return ("🤢 Diseased".to_string(), CONDITION_POISONED_COLOR);
    }
    if conditions.has(Condition::BLINDED) {
        return ("👁️ Blind".to_string(), HP_INJURED_COLOR);
    }
    if conditions.has(Condition::SILENCED) {
        return ("🔇 Silenced".to_string(), HP_INJURED_COLOR);
    }
    if conditions.has(Condition::ASLEEP) {
        return ("😴 Asleep".to_string(), CONDITION_PARALYZED_COLOR);
    }
    if !active_conditions.is_empty() {
        return ("✨ Buffed".to_string(), CONDITION_BUFFED_COLOR);
    }
    ("✓ OK".to_string(), HP_HEALTHY_COLOR)
}
````

---

#### 1.2 Register HUD Module

**[MODIFY]** `src/game/systems/mod.rs`

Add module declaration:

```rust
pub mod hud;
```

Location: Add after existing module declarations (after `pub mod ui;` if present)

---

#### 1.3 Integrate HUD Plugin

**[MODIFY]** `src/game/mod.rs`

Add import:

```rust
use systems::hud::HudPlugin;
```

Update plugin registration (find the `.add_plugins(UiPlugin)` line):

Change from:

```rust
.add_plugins(UiPlugin)
```

To:

```rust
.add_plugins((UiPlugin, HudPlugin))
```

---

#### 1.4 Testing Requirements

**MANDATORY**: Add unit tests for ALL public functions (AGENTS.md Rule 4).

**[MODIFY]** `src/game/systems/hud.rs`

Add test module at end of file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Helper to compare colors (Bevy Color may have floating point precision differences)
    fn colors_approx_equal(a: Color, b: Color) -> bool {
        let a_rgba = a.to_srgba();
        let b_rgba = b.to_srgba();
        (a_rgba.red - b_rgba.red).abs() < 0.01
            && (a_rgba.green - b_rgba.green).abs() < 0.01
            && (a_rgba.blue - b_rgba.blue).abs() < 0.01
    }

    #[test]
    fn test_hp_bar_color_healthy() {
        let color = hp_bar_color(0.80);
        assert!(colors_approx_equal(color, HP_HEALTHY_COLOR));
    }

    #[test]
    fn test_hp_bar_color_injured() {
        let color = hp_bar_color(0.50);
        assert!(colors_approx_equal(color, HP_INJURED_COLOR));
    }

    #[test]
    fn test_hp_bar_color_critical() {
        let color = hp_bar_color(0.15);
        assert!(colors_approx_equal(color, HP_CRITICAL_COLOR));
    }

    #[test]
    fn test_hp_bar_color_boundary_healthy() {
        let color = hp_bar_color(HP_HEALTHY_THRESHOLD);
        assert!(colors_approx_equal(color, HP_HEALTHY_COLOR));
    }

    #[test]
    fn test_hp_bar_color_boundary_critical() {
        let color = hp_bar_color(HP_CRITICAL_THRESHOLD);
        assert!(colors_approx_equal(color, HP_INJURED_COLOR));
    }

    #[test]
    fn test_format_hp_display() {
        let display = format_hp_display(45, 100);
        assert_eq!(display, "45/100 HP");
    }

    #[test]
    fn test_format_hp_display_full() {
        let display = format_hp_display(100, 100);
        assert_eq!(display, "100/100 HP");
    }

    #[test]
    fn test_format_hp_display_zero() {
        let display = format_hp_display(0, 100);
        assert_eq!(display, "0/100 HP");
    }

    #[test]
    fn test_get_priority_condition_dead() {
        let mut conditions = Condition::new();
        conditions.add(Condition::DEAD);
        let (text, color) = get_priority_condition(&conditions, &[]);
        assert!(text.contains("Dead"));
        assert!(colors_approx_equal(color, HP_DEAD_COLOR));
    }

    #[test]
    fn test_get_priority_condition_poisoned() {
        let mut conditions = Condition::new();
        conditions.add(Condition::POISONED);
        let (text, color) = get_priority_condition(&conditions, &[]);
        assert!(text.contains("Poison"));
        assert!(colors_approx_equal(color, CONDITION_POISONED_COLOR));
    }

    #[test]
    fn test_get_priority_condition_paralyzed() {
        let mut conditions = Condition::new();
        conditions.add(Condition::PARALYZED);
        let (text, color) = get_priority_condition(&conditions, &[]);
        assert!(text.contains("Paralyzed"));
        assert!(colors_approx_equal(color, CONDITION_PARALYZED_COLOR));
    }

    #[test]
    fn test_get_priority_condition_fine() {
        let conditions = Condition::new();
        let (text, color) = get_priority_condition(&conditions, &[]);
        assert!(text.contains("OK"));
        assert!(colors_approx_equal(color, HP_HEALTHY_COLOR));
    }

    #[test]
    fn test_get_priority_condition_multiple() {
        // Dead takes priority over poisoned
        let mut conditions = Condition::new();
        conditions.add(Condition::DEAD);
        conditions.add(Condition::POISONED);
        let (text, _) = get_priority_condition(&conditions, &[]);
        assert!(text.contains("Dead"));
    }
}
```

**Automated Verification** (run in order):

```bash
# 1. Format code
cargo fmt --all

# 2. Check compilation
cargo check --all-targets --all-features

# 3. Run linter (must have ZERO warnings)
cargo clippy --all-targets --all-features -- -D warnings

# 4. Run HUD-specific tests
cargo test hud::tests --all-features

# 5. Run all tests to ensure no regressions
cargo test --all-features
```

**Expected Results**:

- `cargo fmt --all` → No output (all files formatted)
- `cargo check` → "Finished" with 0 errors
- `cargo clippy` → "Finished" with 0 warnings
- `cargo test hud::tests` → All HUD tests pass
- `cargo test` → All existing tests still pass

---

#### 1.5 Deliverables

Track completion:

- [ ] `src/game/systems/hud.rs` - New file created with SPDX header
- [ ] All constants defined (`HP_HEALTHY_COLOR`, etc.)
- [ ] All public functions implemented with doc comments
- [ ] `HudPlugin` struct implemented
- [ ] `setup_hud` function implemented
- [ ] `spawn_character_card` helper function implemented
- [ ] `update_hud` function implemented
- [ ] `hp_bar_color` function implemented
- [ ] `format_hp_display` function implemented
- [ ] `get_priority_condition` function implemented
- [ ] Test module added with 13+ tests
- [ ] `src/game/systems/mod.rs` - Module export added
- [ ] `src/game/mod.rs` - Plugin registration updated
- [ ] All tests pass (`cargo test`)
- [ ] Zero clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo fmt --all`)

---

#### 1.6 Success Criteria

Verify ALL of these:

1. ✅ `cargo check --all-targets --all-features` completes with 0 errors
2. ✅ `cargo clippy --all-targets --all-features -- -D warnings` shows 0 warnings
3. ✅ `cargo test hud::tests` shows 13+ tests passing
4. ✅ `cargo test --all-features` shows all existing tests still passing
5. ✅ HUD compiles without errors
6. ✅ SPDX header present in `src/game/systems/hud.rs`
7. ✅ All constants extracted (no inline color values)
8. ✅ All public functions have `///` doc comments with examples
9. ✅ `PARTY_MAX_SIZE` constant referenced (not hardcoded value 6)
10. ✅ `Character` struct fields accessed correctly (`.hp.current`, `.hp.base`)
11. ✅ `Condition::has()` method used for condition checks
12. ✅ No modifications to core domain structs
13. ✅ `spawn_character_card` helper function implemented and called from `setup_hud`
14. ✅ Bevy 0.17 API used correctly (Node, Text, TextFont, TextColor components)

---

### Phase 2: Enhanced Condition Display

#### 2.1 Add Condition Priority Constants

**[MODIFY]** `src/game/systems/hud.rs`

Add condition priority constants after color constants:

```rust
// Condition priority values (higher = more severe)
pub const PRIORITY_DEAD: u8 = 100;
pub const PRIORITY_UNCONSCIOUS: u8 = 90;
pub const PRIORITY_PARALYZED: u8 = 80;
pub const PRIORITY_POISONED: u8 = 70;
pub const PRIORITY_DISEASED: u8 = 60;
pub const PRIORITY_BLINDED: u8 = 50;
pub const PRIORITY_SILENCED: u8 = 40;
pub const PRIORITY_ASLEEP: u8 = 30;
pub const PRIORITY_BUFFED: u8 = 10;
pub const PRIORITY_FINE: u8 = 0;
```

---

#### 2.2 Add Condition Priority Function

**[MODIFY]** `src/game/systems/hud.rs`

Add new function after `get_priority_condition`:

````rust
/// Returns numeric priority for a condition
///
/// Higher values = more severe conditions displayed first
///
/// # Arguments
/// * `conditions` - Character's condition bitflags
///
/// # Returns
/// Priority value (0-100)
///
/// # Examples
///
/// ```
/// use antares::domain::character::Condition;
/// use antares::game::systems::hud::get_condition_priority;
///
/// let mut conditions = Condition::new();
/// conditions.add(Condition::DEAD);
/// assert_eq!(get_condition_priority(&conditions), PRIORITY_DEAD);
/// ```
pub fn get_condition_priority(conditions: &Condition) -> u8 {
    if conditions.has(Condition::DEAD)
        || conditions.has(Condition::STONE)
        || conditions.has(Condition::ERADICATED)
    {
        return PRIORITY_DEAD;
    }
    if conditions.has(Condition::UNCONSCIOUS) {
        return PRIORITY_UNCONSCIOUS;
    }
    if conditions.has(Condition::PARALYZED) {
        return PRIORITY_PARALYZED;
    }
    if conditions.has(Condition::POISONED) {
        return PRIORITY_POISONED;
    }
    if conditions.has(Condition::DISEASED) {
        return PRIORITY_DISEASED;
    }
    if conditions.has(Condition::BLINDED) {
        return PRIORITY_BLINDED;
    }
    if conditions.has(Condition::SILENCED) {
        return PRIORITY_SILENCED;
    }
    if conditions.has(Condition::ASLEEP) {
        return PRIORITY_ASLEEP;
    }
    PRIORITY_FINE
}
````

---

#### 2.3 Add Multiple Condition Counter

**[MODIFY]** `src/game/systems/hud.rs`

Add helper function to count active conditions:

````rust
/// Counts number of active negative conditions
///
/// Used to display "+N conditions" when multiple exist
///
/// # Arguments
/// * `conditions` - Character's condition bitflags
///
/// # Returns
/// Count of active conditions (0-8)
///
/// # Examples
///
/// ```
/// use antares::domain::character::Condition;
/// use antares::game::systems::hud::count_conditions;
///
/// let mut conditions = Condition::new();
/// conditions.add(Condition::POISONED);
/// conditions.add(Condition::BLINDED);
/// assert_eq!(count_conditions(&conditions), 2);
/// ```
pub fn count_conditions(conditions: &Condition) -> u8 {
    let mut count = 0;
    if conditions.has(Condition::ASLEEP) {
        count += 1;
    }
    if conditions.has(Condition::BLINDED) {
        count += 1;
    }
    if conditions.has(Condition::SILENCED) {
        count += 1;
    }
    if conditions.has(Condition::DISEASED) {
        count += 1;
    }
    if conditions.has(Condition::POISONED) {
        count += 1;
    }
    if conditions.has(Condition::PARALYZED) {
        count += 1;
    }
    if conditions.has(Condition::UNCONSCIOUS) {
        count += 1;
    }
    if conditions.has(Condition::DEAD) {
        count += 1;
    }
    count
}
````

**Update `update_hud` function** to display multiple condition count:

Modify the condition text update section:

```rust
// Update condition text
for (condition_text, mut text, mut text_color) in condition_text_query.iter_mut() {
    if let Some(character) = party.members.get(condition_text.party_index) {
        let (cond_str, color) =
            get_priority_condition(&character.conditions, &character.active_conditions);
        let count = count_conditions(&character.conditions);

        // If multiple conditions, append count
        let display_text = if count > 1 {
            format!("{} +{}", cond_str, count - 1)
        } else {
            cond_str
        };

        **text = display_text;
        *text_color = TextColor(color);
    } else {
        **text = String::new();
    }
}
```

---

#### 2.4 Testing Requirements

**[MODIFY]** `src/game/systems/hud.rs` test module

Add tests for new functions:

```rust
#[test]
fn test_get_condition_priority_dead() {
    let mut conditions = Condition::new();
    conditions.add(Condition::DEAD);
    assert_eq!(get_condition_priority(&conditions), PRIORITY_DEAD);
}

#[test]
fn test_get_condition_priority_poisoned() {
    let mut conditions = Condition::new();
    conditions.add(Condition::POISONED);
    assert_eq!(get_condition_priority(&conditions), PRIORITY_POISONED);
}

#[test]
fn test_get_condition_priority_fine() {
    let conditions = Condition::new();
    assert_eq!(get_condition_priority(&conditions), PRIORITY_FINE);
}

#[test]
fn test_count_conditions_none() {
    let conditions = Condition::new();
    assert_eq!(count_conditions(&conditions), 0);
}

#[test]
fn test_count_conditions_single() {
    let mut conditions = Condition::new();
    conditions.add(Condition::POISONED);
    assert_eq!(count_conditions(&conditions), 1);
}

#[test]
fn test_count_conditions_multiple() {
    let mut conditions = Condition::new();
    conditions.add(Condition::POISONED);
    conditions.add(Condition::BLINDED);
    conditions.add(Condition::SILENCED);
    assert_eq!(count_conditions(&conditions), 3);
}
```

**Automated Verification**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test hud::tests --all-features
cargo test --all-features
```

---

#### 2.5 Deliverables

Track completion:

- [ ] Priority constants added (`PRIORITY_DEAD`, etc.)
- [ ] `get_condition_priority` function implemented with doc comments
- [ ] `count_conditions` function implemented with doc comments
- [ ] `update_hud` modified to show multiple condition count
- [ ] 6+ new tests added for priority and counting functions
- [ ] All tests pass (`cargo test hud::tests`)
- [ ] Zero clippy warnings
- [ ] Code formatted

---

#### 2.6 Success Criteria

Verify ALL of these:

1. ✅ `cargo test hud::tests` shows 19+ tests passing (13 from Phase 1 + 6 new)
2. ✅ Priority constants defined (no magic numbers)
3. ✅ `get_condition_priority` returns correct values for all condition types
4. ✅ `count_conditions` counts multiple conditions correctly
5. ✅ Multiple condition display shows "+N" suffix (e.g., "☠️ Poisoned +2")
6. ✅ All new functions have doc comments with examples
7. ✅ Zero clippy warnings
8. ✅ All existing tests still pass

---

### Phase 3: Compass Display

#### 3.1 Add Compass Constants

**[MODIFY]** `src/game/systems/hud.rs`

Add compass display constants after existing layout constants:

```rust
// Compass display constants
pub const COMPASS_SIZE: f32 = 48.0;
pub const COMPASS_BORDER_WIDTH: f32 = 2.0;
pub const COMPASS_BACKGROUND_COLOR: Color = Color::srgba(0.1, 0.1, 0.1, 0.9);
pub const COMPASS_BORDER_COLOR: Color = Color::srgba(0.4, 0.4, 0.4, 1.0);
pub const COMPASS_TEXT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);
pub const COMPASS_FONT_SIZE: f32 = 24.0;
```

---

#### 3.2 Add Compass Components

**[MODIFY]** `src/game/systems/hud.rs`

Add component markers after existing component definitions:

```rust
/// Marker component for the compass container
#[derive(Component)]
pub struct CompassRoot;

/// Marker component for the compass direction text
#[derive(Component)]
pub struct CompassText;
```

---

#### 3.3 Add Compass Helper Functions

**[MODIFY]** `src/game/systems/hud.rs`

Add helper functions before the test module:

````rust
/// Converts Direction enum to display string
///
/// # Arguments
/// * `direction` - The cardinal direction from World state
///
/// # Returns
/// Single character string: "N", "E", "S", or "W"
///
/// # Examples
///
/// ```
/// use antares::domain::world::Direction;
/// use antares::game::systems::hud::direction_to_string;
///
/// assert_eq!(direction_to_string(&Direction::North), "N");
/// assert_eq!(direction_to_string(&Direction::East), "E");
/// ```
pub fn direction_to_string(direction: &Direction) -> String {
    match direction {
        Direction::North => "N".to_string(),
        Direction::East => "E".to_string(),
        Direction::South => "S".to_string(),
        Direction::West => "W".to_string(),
    }
}
````

---

#### 3.4 Add Compass to HUD Setup

**[MODIFY]** `src/game/systems/hud.rs` - `setup_hud` function

Add compass spawning after the character cards loop:

```rust
// Spawn compass display
commands
    .spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            top: Val::Px(20.0),
            width: Val::Px(COMPASS_SIZE),
            height: Val::Px(COMPASS_SIZE),
            border: UiRect::all(Val::Px(COMPASS_BORDER_WIDTH)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(COMPASS_BACKGROUND_COLOR),
        BorderColor(COMPASS_BORDER_COLOR),
        CompassRoot,
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("N"),
            TextFont {
                font_size: COMPASS_FONT_SIZE,
                ..default()
            },
            TextColor(COMPASS_TEXT_COLOR),
            CompassText,
        ));
    });
```

---

#### 3.5 Add Compass Update System

**[MODIFY]** `src/game/systems/hud.rs`

Add new system function after `update_hud`:

```rust
/// Updates compass direction display
///
/// Queries the World resource to get current party_facing and updates
/// the compass text to show N/E/S/W
fn update_compass(
    world: Res<World>,
    mut compass_query: Query<&mut Text, With<CompassText>>,
) {
    if let Ok(mut text) = compass_query.get_single_mut() {
        **text = direction_to_string(&world.party_facing);
    }
}
```

---

#### 3.6 Register Compass Update System

**[MODIFY]** `src/game/systems/hud.rs` - `HudPlugin` implementation

Update the plugin to include compass update system:

```rust
impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud)
            .add_systems(Update, (update_hud, update_compass));
    }
}
```

---

#### 3.7 Add Required Import

**[MODIFY]** `src/game/systems/hud.rs` - imports section

Add Direction import:

```rust
use crate::domain::world::{Direction, World};
```

---

#### 3.8 Testing Requirements

**[MODIFY]** `src/game/systems/hud.rs` test module

Add tests for compass functionality:

```rust
#[test]
fn test_direction_to_string_north() {
    assert_eq!(direction_to_string(&Direction::North), "N");
}

#[test]
fn test_direction_to_string_east() {
    assert_eq!(direction_to_string(&Direction::East), "E");
}

#[test]
fn test_direction_to_string_south() {
    assert_eq!(direction_to_string(&Direction::South), "S");
}

#[test]
fn test_direction_to_string_west() {
    assert_eq!(direction_to_string(&Direction::West), "W");
}

#[test]
fn test_compass_constants_valid() {
    assert!(COMPASS_SIZE > 0.0);
    assert!(COMPASS_BORDER_WIDTH > 0.0);
    assert!(COMPASS_FONT_SIZE > 0.0);
}
```

**Automated Verification**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test hud::tests --all-features
cargo test --all-features
```

---

#### 3.9 Deliverables

Track completion:

- [ ] Compass constants added (`COMPASS_SIZE`, `COMPASS_BACKGROUND_COLOR`, etc.)
- [ ] `CompassRoot` and `CompassText` components defined
- [ ] `direction_to_string` function implemented with doc comments
- [ ] Compass spawned in `setup_hud` function
- [ ] `update_compass` system implemented
- [ ] `HudPlugin` updated to register `update_compass` system
- [ ] `Direction` and `World` imports added
- [ ] 5+ new tests added for compass functionality
- [ ] All tests pass (`cargo test hud::tests`)
- [ ] Zero clippy warnings
- [ ] Code formatted

---

#### 3.10 Success Criteria

Verify ALL of these:

1. ✅ `cargo test hud::tests` shows 24+ tests passing (19 from Phases 1-2 + 5 new)
2. ✅ Compass displays in top-right corner of screen
3. ✅ Compass shows correct direction letter (N/E/S/W)
4. ✅ Compass updates when `World.party_facing` changes
5. ✅ `direction_to_string` returns correct string for all Direction enum variants
6. ✅ All compass constants defined (no magic numbers)
7. ✅ `update_compass` system registered in HudPlugin
8. ✅ All new functions have doc comments with examples
9. ✅ Zero clippy warnings
10. ✅ All existing tests still pass
11. ✅ No modifications to core domain structs (Direction, World)
12. ✅ Bevy 0.17 API used correctly (Node, Text, TextFont, TextColor components)

---

### Phase 4: Portrait Support (DEFERRED)

**STATUS**: This phase is NOT included in current implementation scope.

**DECISION**: Portrait rendering will be implemented in a separate task after Phase 3 completion.

**DO NOT IMPLEMENT** this phase unless explicitly instructed by the user.

**Future Requirements** (for reference only):

- Portrait asset loading system in `data/assets/portraits/`
- Bevy `Image` asset integration
- Render portraits in character cards (add to `spawn_character_card`)
- Fallback placeholder for missing portraits
- `Character.portrait_id` field utilization

---

## Verification Summary

### Automated Quality Checks

Run ALL commands in sequence after each phase:

```bash
# 1. Format all code
cargo fmt --all

# 2. Verify compilation
cargo check --all-targets --all-features

# 3. Lint with zero warnings
cargo clippy --all-targets --all-features -- -D warnings

# 4. Run HUD tests
cargo test hud::tests --all-features -- --nocapture

# 5. Run all tests
cargo test --all-features

# 6. Run condition-specific tests
cargo test condition --all-features -- --nocapture

# 7. Run character tests
cargo test character --all-features -- --nocapture
```

### Integration Verification

To verify HUD displays correctly (after Phase 1 completion):

```bash
# Run the game
cargo run --bin antares

# Expected behavior:
# - HUD appears at bottom of screen
# - Party members displayed horizontally
# - Each card shows name, HP bar, HP text, condition
# - Colors match health percentage
# - No compilation errors or warnings
```

### Test Coverage Requirements

Minimum test counts:

- Phase 1: 13+ tests for core functions
- Phase 2: 19+ tests (13 existing + 6 new)

All tests MUST pass with zero failures.

---

## Implementation Notes

### Common Pitfalls to Avoid

1. ❌ **DO NOT** hardcode the value `6` - use `PARTY_MAX_SIZE` constant
2. ❌ **DO NOT** use inline color values - define all colors as constants
3. ❌ **DO NOT** modify `Character`, `Condition`, or `AttributePair16` structs
4. ❌ **DO NOT** skip SPDX header in new files
5. ❌ **DO NOT** skip unit tests (required by AGENTS.md Rule 4)
6. ❌ **DO NOT** use `unwrap()` without justification
7. ❌ **DO NOT** ignore clippy warnings
8. ❌ **DO NOT** inline all spawn logic - use `spawn_character_card` helper
9. ❌ **DO NOT** use wrong Bevy 0.17 API (verify Node, Text, TextFont, TextColor usage)

### Architecture Adherence

- Reference `PARTY_MAX_SIZE` from `src/domain/character.rs#L101`
- Use `Condition::has()` method for condition checks
- Access HP via `character.hp.current` and `character.hp.base`
- No direct modification of domain layer structs
- Respect layer boundaries (UI → Domain, not Domain → UI)

### Bevy 0.17 Specifics

- Use `Node` component for layout (replaces `Style` in older versions)
- Use `Text::new("string")` to create text
- Use `TextFont` component for font size and styling
- Use `TextColor(Color)` component for text color (separate from Text)
- Use `BackgroundColor(Color)` for UI element backgrounds
- Use `Val::Percent()` and `Val::Px()` for sizing
- Query with `&mut Text` and deref with `**text` to modify content

### Code Quality Standards

- All public functions MUST have `///` doc comments
- All doc comments MUST include `# Examples` section
- All color values MUST be named constants
- All magic numbers MUST be extracted as constants
- All tests MUST use descriptive names: `test_{function}_{condition}_{expected}`
- Zero tolerance for clippy warnings
- Use `spawn_character_card` helper for maintainability
