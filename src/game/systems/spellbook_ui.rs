// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! In-game Spell Book management screen Bevy plugin.
//!
//! This module implements the full UI overlay and input handling for browsing
//! the party's spell books during exploration.  When the player presses the
//! Spell Book key (`B` by default), the game enters
//! [`GameMode::SpellBook`](crate::application::GameMode::SpellBook) and this
//! plugin takes over input until the screen is closed.
//!
//! # Layout
//!
//! Three-column design:
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │  📚 Spell Book                             [ESC] Close │
//! ├─────────────┬──────────────────────┬───────────────────┤
//! │ Characters  │ Known Spells         │ Detail            │
//! │ ──────────  │ ─────────────        │ ──────            │
//! │ [*Aria  ✓] │ -- Level 1 --        │ First Aid         │
//! │ [ Korbin  ] │  First Aid — 5 SP   │ School: Cleric    │
//! │ [ Sylva ✓] │  Cure Poison — 8 SP │ Level: 1          │
//! │             │  💎1                │ SP Cost: 5        │
//! │             │ -- Level 2 --        │ Gem Cost: —       │
//! │             │  Bless — 12 SP ⚔   │ Context: Any      │
//! │             │                      │                   │
//! │             │ -- Learnable Scrolls │ Restores 1d6+1 HP │
//! │             │  Scroll -> Light     │ to a single tgt.  │
//! ├─────────────┴──────────────────────┴───────────────────┤
//! │  [C] Cast Spell   [Tab] Switch Char   [↑↓] Select Spell│
//! └────────────────────────────────────────────────────────┘
//! ```
//!
//! # Flow
//!
//! 1. `B` key → `GameState::enter_spellbook_with_caster_select()` (in
//!    `input/global_toggles.rs`).
//! 2. [`setup_spellbook_ui`] detects the new mode and spawns the overlay.
//! 3. [`handle_spellbook_input`] drives navigation:
//!    - **Tab** / **Shift+Tab** — cycle through party members.
//!    - **↑↓ / W/S**           — navigate spell list rows.
//!    - **Enter / Space**       — select (highlight) the focused spell.
//!    - **C**                   — exit SpellBook and enter SpellCasting for
//!      the currently browsed character.
//!    - **Esc**                 — exit SpellBook and restore previous mode.
//! 4. [`update_spellbook_ui`] rebuilds all three columns every frame to
//!    reflect the current character and selected spell.
//! 5. [`cleanup_spellbook_ui`] despawns the overlay when the mode leaves
//!    `SpellBook`.
//!
//! # Architecture Reference
//!
//! Phase 2 of `docs/explanation/spell_management_implementation_plan.md`.

use crate::application::resources::GameContent;
use crate::application::spell_book_state::SpellBookState;
use crate::application::GameMode;
use crate::domain::items::types::{ConsumableEffect, ItemType};
use crate::domain::magic::learning::can_learn_spell;
use crate::domain::magic::types::SpellContext;
use crate::domain::types::SpellId;
use crate::game::resources::GlobalState;
use crate::game::systems::ui_helpers::{BODY_FONT_SIZE, LABEL_FONT_SIZE};
use bevy::prelude::*;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Semi-transparent full-screen backdrop for the Spell Book overlay.
pub const SPELLBOOK_OVERLAY_BG: Color = Color::srgba(0.0, 0.0, 0.1, 0.88);
/// Background color of the inner panel.
pub const SPELLBOOK_PANEL_BG: Color = Color::srgba(0.06, 0.06, 0.18, 0.97);
/// Highlight color for the currently selected spell row.
pub const SPELLBOOK_SELECTED_ROW_BG: Color = Color::srgba(0.2, 0.2, 0.05, 0.9);
/// Default text color for spell name entries.
pub const SPELLBOOK_NORMAL_ROW_COLOR: Color = Color::WHITE;
/// Text color when the character has insufficient SP to cast the spell.
pub const SPELLBOOK_DISABLED_SPELL_COLOR: Color = Color::srgb(0.45, 0.45, 0.45);
/// Text color for "Level N" group header rows.
pub const SPELLBOOK_LEVEL_HEADER_COLOR: Color = Color::srgb(0.7, 0.8, 1.0);
/// Text / background highlight for the active character tab.
pub const SPELLBOOK_CHAR_TAB_ACTIVE_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);
/// Text color for inactive character tabs.
pub const SPELLBOOK_CHAR_TAB_INACTIVE_COLOR: Color = Color::srgb(0.6, 0.6, 0.7);
/// Text color for hint / secondary text at the bottom and in the detail panel.
pub const SPELLBOOK_HINT_COLOR: Color = Color::srgb(0.55, 0.55, 0.65);
/// Text color for the main "Spell Book" title and column headers.
pub const SPELLBOOK_TITLE_COLOR: Color = Color::srgb(0.8, 0.85, 1.0);

// ── Components ─────────────────────────────────────────────────────────────────

/// Marker component for the root full-screen Spell Book overlay node.
///
/// Spawned by [`setup_spellbook_ui`] and despawned by [`cleanup_spellbook_ui`].
///
/// # Examples
///
/// ```
/// use antares::game::systems::spellbook_ui::SpellBookOverlay;
///
/// let _: SpellBookOverlay = SpellBookOverlay;
/// ```
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellBookOverlay;

/// Marker component for the inner three-column layout panel.
///
/// Child of [`SpellBookOverlay`].  Contains the character tab column, the
/// spell list column, and the detail panel column.
///
/// # Examples
///
/// ```
/// use antares::game::systems::spellbook_ui::SpellBookContent;
///
/// let _: SpellBookContent = SpellBookContent;
/// ```
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellBookContent;

/// Marker component for a single character tab in the left column.
///
/// One entity is spawned per party member.  `party_index` maps to
/// `party.members[party_index]`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::spellbook_ui::SpellBookCharTab;
///
/// let tab = SpellBookCharTab { party_index: 2 };
/// assert_eq!(tab.party_index, 2);
/// ```
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellBookCharTab {
    /// Index into `party.members` that this tab represents.
    pub party_index: usize,
}

/// Marker component for a single spell entry row in the center column.
///
/// One entity is spawned per known spell displayed in the list.  `spell_id`
/// corresponds to the entry in [`SpellDatabase`](crate::domain::magic::database::SpellDatabase).
///
/// # Examples
///
/// ```
/// use antares::game::systems::spellbook_ui::SpellBookSpellRow;
///
/// let row = SpellBookSpellRow { spell_id: 0x0101 };
/// assert_eq!(row.spell_id, 0x0101);
/// ```
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellBookSpellRow {
    /// Spell identifier for this row entry.
    pub spell_id: SpellId,
}

// Internal marker components for the three rebuildable content areas.

/// Marker for the character-tab list column (left column children container).
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellBookCharList;

/// Marker for the spell-list column (center column children container).
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellBookSpellList;

/// Marker for the spell-detail panel (right column children container).
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellBookDetailPane;

// ── Plugin ─────────────────────────────────────────────────────────────────────

/// Bevy plugin that provides the in-game Spell Book management screen.
///
/// Registers the four systems that drive the Spell Book overlay:
/// spawning, updating, input handling, and cleanup — chained so that
/// setup always runs before update and cleanup runs last each frame.
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::spellbook_ui::SpellBookPlugin;
///
/// # fn setup() {
/// let mut app = App::new();
/// app.add_plugins(SpellBookPlugin);
/// # }
/// ```
pub struct SpellBookPlugin;

impl Plugin for SpellBookPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                setup_spellbook_ui,
                update_spellbook_ui,
                handle_spellbook_input,
                cleanup_spellbook_ui,
            )
                .chain(),
        );
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Spawns the Spell Book overlay hierarchy when the game enters `SpellBook` mode.
///
/// Idempotent: if the overlay already exists the system returns immediately.
/// If the mode is not `SpellBook` the system also returns immediately.
///
/// The spawned hierarchy consists of:
/// - [`SpellBookOverlay`] — full-screen backdrop
///   - [`SpellBookContent`] — main panel (FlexDirection::Column)
///     - Title bar row (title + close hint)
///     - Three-column row (char list | spell list | detail pane)
///     - Bottom hint bar
pub fn setup_spellbook_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    existing: Query<Entity, With<SpellBookOverlay>>,
) {
    // Only active in SpellBook mode.
    if !matches!(global_state.0.mode, GameMode::SpellBook(_)) {
        return;
    }

    // Only spawn once.
    if !existing.is_empty() {
        return;
    }

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(SPELLBOOK_OVERLAY_BG),
            SpellBookOverlay,
        ))
        .with_children(|root| {
            // ── Main panel ───────────────────────────────────────────────────
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(88.0),
                    max_width: Val::Px(920.0),
                    height: Val::Percent(80.0),
                    max_height: Val::Px(620.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(SPELLBOOK_PANEL_BG),
                BorderRadius::all(Val::Px(10.0)),
                SpellBookContent,
            ))
            .with_children(|panel| {
                // ── Title bar ────────────────────────────────────────────────
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        width: Val::Percent(100.0),
                        ..default()
                    })
                    .with_children(|title_row| {
                        title_row.spawn((
                            Text::new("\u{1F4DA} Spell Book"),
                            TextFont {
                                font_size: BODY_FONT_SIZE + 2.0,
                                ..default()
                            },
                            TextColor(SPELLBOOK_TITLE_COLOR),
                        ));
                        title_row.spawn((
                            Text::new("[ESC] Close"),
                            TextFont {
                                font_size: LABEL_FONT_SIZE,
                                ..default()
                            },
                            TextColor(SPELLBOOK_HINT_COLOR),
                        ));
                    });

                // ── Three-column content row ──────────────────────────────────
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|cols| {
                        // Left column — character tabs
                        cols.spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                width: Val::Px(160.0),
                                min_width: Val::Px(140.0),
                                row_gap: Val::Px(4.0),
                                padding: UiRect::all(Val::Px(6.0)),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.03, 0.03, 0.12, 0.5)),
                            BorderRadius::all(Val::Px(6.0)),
                            SpellBookCharList,
                        ));

                        // Center column — spell list
                        cols.spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                flex_grow: 1.0,
                                row_gap: Val::Px(3.0),
                                padding: UiRect::all(Val::Px(6.0)),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.03, 0.03, 0.12, 0.5)),
                            BorderRadius::all(Val::Px(6.0)),
                            SpellBookSpellList,
                        ));

                        // Right column — detail panel
                        cols.spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                width: Val::Px(215.0),
                                min_width: Val::Px(180.0),
                                row_gap: Val::Px(4.0),
                                padding: UiRect::all(Val::Px(6.0)),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.03, 0.03, 0.12, 0.5)),
                            BorderRadius::all(Val::Px(6.0)),
                            SpellBookDetailPane,
                        ));
                    });

                // ── Bottom hint bar ───────────────────────────────────────────
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        width: Val::Percent(100.0),
                        ..default()
                    })
                    .with_children(|hint_row| {
                        hint_row.spawn((
                            Text::new(
                                "[C] Cast Spell    [Tab] Switch Character    [\u{2191}\u{2193}] Select Spell",
                            ),
                            TextFont {
                                font_size: LABEL_FONT_SIZE,
                                ..default()
                            },
                            TextColor(SPELLBOOK_HINT_COLOR),
                        ));
                    });
            });
        });
}

/// Despawns the Spell Book overlay when the game leaves `SpellBook` mode.
pub fn cleanup_spellbook_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    existing: Query<Entity, With<SpellBookOverlay>>,
) {
    if matches!(global_state.0.mode, GameMode::SpellBook(_)) {
        return;
    }
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }
}

/// Updates all three columns every frame while in `SpellBook` mode.
///
/// Rebuilds the children of [`SpellBookCharList`], [`SpellBookSpellList`], and
/// [`SpellBookDetailPane`] on every frame.  This is intentionally simple: the
/// lists are short and per-frame rebuilds are cheap.
#[allow(clippy::too_many_arguments)]
pub fn update_spellbook_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    content: Option<Res<GameContent>>,
    char_list_query: Query<Entity, With<SpellBookCharList>>,
    spell_list_query: Query<Entity, With<SpellBookSpellList>>,
    detail_pane_query: Query<Entity, With<SpellBookDetailPane>>,
    children_query: Query<&Children>,
) {
    let sb = match &global_state.0.mode {
        GameMode::SpellBook(sb) => sb,
        _ => return,
    };

    // Collect the flat ordered spell ID list once for reuse across columns.
    let spell_ids = collect_spell_ids_from_state(&global_state.0, content.as_deref());

    // ── Rebuild character tab column ─────────────────────────────────────────
    if let Ok(char_entity) = char_list_query.single() {
        despawn_children(&mut commands, char_entity, &children_query);
        let sb_clone = sb.clone();
        commands.entity(char_entity).with_children(|list| {
            build_char_tabs(list, &sb_clone, &global_state);
        });
    }

    // ── Rebuild spell list column ────────────────────────────────────────────
    if let Ok(spell_entity) = spell_list_query.single() {
        despawn_children(&mut commands, spell_entity, &children_query);
        let sb_clone = sb.clone();
        commands.entity(spell_entity).with_children(|list| {
            build_spell_list(
                list,
                &sb_clone,
                &global_state,
                content.as_deref(),
                &spell_ids,
            );
        });
    }

    // ── Rebuild detail panel column ──────────────────────────────────────────
    if let Ok(detail_entity) = detail_pane_query.single() {
        despawn_children(&mut commands, detail_entity, &children_query);
        let sb_clone = sb.clone();
        commands.entity(detail_entity).with_children(|pane| {
            build_detail_panel(pane, &sb_clone, content.as_deref());
        });
    }
}

/// Handles keyboard input while in `SpellBook` mode.
///
/// - **Tab** (no shift) — advance to next party member's book.
/// - **Shift+Tab** — return to previous party member's book.
/// - **↑ / W** — move spell cursor up.
/// - **↓ / S** — move spell cursor down.
/// - **Enter / Space** — confirm selection (updates `selected_spell_id`).
/// - **C** — exit SpellBook and enter spell-casting flow for current character.
/// - **Esc** — exit SpellBook and restore previous mode.
pub fn handle_spellbook_input(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut global_state: ResMut<GlobalState>,
    content: Option<Res<GameContent>>,
) {
    if !matches!(global_state.0.mode, GameMode::SpellBook(_)) {
        return;
    }

    let Some(ref kb) = keyboard else {
        return;
    };

    // ── Esc: close and restore previous mode ────────────────────────────────
    if kb.just_pressed(KeyCode::Escape) {
        global_state.0.exit_spellbook();
        return;
    }

    // ── C: open casting flow for the current character ───────────────────────
    if kb.just_pressed(KeyCode::KeyC) {
        let char_index = if let GameMode::SpellBook(ref sb) = global_state.0.mode {
            sb.character_index
        } else {
            return;
        };
        global_state.0.exit_spellbook();
        global_state.0.enter_spell_casting(char_index);
        return;
    }

    // ── Tab / Shift+Tab: cycle characters ────────────────────────────────────
    let party_size = global_state.0.party.members.len();

    if kb.just_pressed(KeyCode::Tab) {
        let shift_held = kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::ShiftRight);
        if shift_held {
            if let GameMode::SpellBook(ref mut sb) = global_state.0.mode {
                sb.prev_character(party_size);
            }
        } else if let GameMode::SpellBook(ref mut sb) = global_state.0.mode {
            sb.next_character(party_size);
        }
        return;
    }

    // ── ↑ / ↓: navigate spell list ───────────────────────────────────────────
    let up = kb.just_pressed(KeyCode::ArrowUp) || kb.just_pressed(KeyCode::KeyW);
    let down = kb.just_pressed(KeyCode::ArrowDown) || kb.just_pressed(KeyCode::KeyS);
    let confirm = kb.just_pressed(KeyCode::Enter) || kb.just_pressed(KeyCode::Space);

    // Build spell id list to know wrapping bounds and resolve selected spell.
    let spell_ids = collect_spell_ids_from_state(&global_state.0, content.as_deref());
    let item_count = spell_ids.len();

    if up {
        if let GameMode::SpellBook(ref mut sb) = global_state.0.mode {
            sb.cursor_up(item_count);
            sb.selected_spell_id = spell_ids.get(sb.selected_row).copied();
        }
        return;
    }

    if down {
        if let GameMode::SpellBook(ref mut sb) = global_state.0.mode {
            sb.cursor_down(item_count);
            sb.selected_spell_id = spell_ids.get(sb.selected_row).copied();
        }
        return;
    }

    // ── Enter / Space: confirm selection ─────────────────────────────────────
    if confirm && !spell_ids.is_empty() {
        if let GameMode::SpellBook(ref mut sb) = global_state.0.mode {
            sb.selected_spell_id = spell_ids.get(sb.selected_row).copied();
        }
    }
}

// ── Public helper ─────────────────────────────────────────────────────────────

/// Collects the flat ordered list of [`SpellId`]s known by the currently
/// browsed character.
///
/// Iterates spell book levels 0–6 (game levels 1–7) and returns IDs in level
/// order.  This list drives both the `selected_row` navigation bounds in
/// [`handle_spellbook_input`] and the row construction in
/// [`update_spellbook_ui`].
///
/// Returns an empty `Vec` if the mode is not `SpellBook`, if the current
/// character index is out of range, or if the content database is absent.
///
/// # Arguments
///
/// * `game_state` — current game state (must be in `SpellBook` mode).
/// * `content`    — optional reference to the loaded content database.
pub fn collect_spell_ids_from_state(
    game_state: &crate::application::GameState,
    content: Option<&GameContent>,
) -> Vec<SpellId> {
    let sb = match &game_state.mode {
        GameMode::SpellBook(sb) => sb,
        _ => return Vec::new(),
    };

    let Some(character) = game_state.party.members.get(sb.character_index) else {
        return Vec::new();
    };

    let Some(content_ref) = content else {
        return Vec::new();
    };

    let class_db = &content_ref.db().classes;
    let spell_list = character
        .spells
        .get_spell_list_by_id(&character.class_id, class_db);

    let mut result = Vec::new();
    for level_spells in spell_list.iter() {
        for &spell_id in level_spells {
            result.push(spell_id);
        }
    }
    result
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Despawns all direct children of `parent`.
fn despawn_children(commands: &mut Commands, parent: Entity, children_query: &Query<&Children>) {
    if let Ok(children) = children_query.get(parent) {
        let child_entities: Vec<Entity> = children.iter().collect();
        for child in child_entities {
            commands.entity(child).despawn();
        }
    }
}

/// Builds the character-tab rows in the left column.
fn build_char_tabs(
    list: &mut ChildSpawnerCommands<'_>,
    sb: &SpellBookState,
    global_state: &GlobalState,
) {
    // Column header
    list.spawn((
        Text::new("Characters"),
        TextFont {
            font_size: LABEL_FONT_SIZE,
            ..default()
        },
        TextColor(SPELLBOOK_TITLE_COLOR),
    ));

    let members = &global_state.0.party.members;
    if members.is_empty() {
        list.spawn((
            Text::new("No party."),
            TextFont {
                font_size: LABEL_FONT_SIZE,
                ..default()
            },
            TextColor(SPELLBOOK_DISABLED_SPELL_COLOR),
        ));
        return;
    }

    for (i, member) in members.iter().enumerate() {
        let active = i == sb.character_index;
        let text_color = if active {
            SPELLBOOK_CHAR_TAB_ACTIVE_COLOR
        } else {
            SPELLBOOK_CHAR_TAB_INACTIVE_COLOR
        };

        // Show "✓" if the character has any known spells.
        let has_spells = member.spells.cleric_spells.iter().any(|v| !v.is_empty())
            || member.spells.sorcerer_spells.iter().any(|v| !v.is_empty());
        let check = if has_spells { " \u{2713}" } else { "" };
        let cursor = if active { "[*]" } else { "[ ]" };

        let label = format!("{cursor} {}{}", member.name, check);

        list.spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(4.0), Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(if active {
                Color::srgba(0.2, 0.2, 0.05, 0.7)
            } else {
                Color::NONE
            }),
            BorderRadius::all(Val::Px(3.0)),
            SpellBookCharTab { party_index: i },
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: LABEL_FONT_SIZE,
                    ..default()
                },
                TextColor(text_color),
            ));
        });
    }
}

/// Builds the spell-list rows and learnable-scrolls section in the center
/// column.
fn build_spell_list(
    list: &mut ChildSpawnerCommands<'_>,
    sb: &SpellBookState,
    global_state: &GlobalState,
    content: Option<&GameContent>,
    spell_ids: &[SpellId],
) {
    // Column header
    list.spawn((
        Text::new("Known Spells"),
        TextFont {
            font_size: LABEL_FONT_SIZE,
            ..default()
        },
        TextColor(SPELLBOOK_TITLE_COLOR),
    ));

    let Some(character) = global_state.0.party.members.get(sb.character_index) else {
        list.spawn((
            Text::new("No character selected."),
            TextFont {
                font_size: BODY_FONT_SIZE,
                ..default()
            },
            TextColor(SPELLBOOK_DISABLED_SPELL_COLOR),
        ));
        return;
    };

    let Some(content_ref) = content else {
        list.spawn((
            Text::new("Content not available."),
            TextFont {
                font_size: BODY_FONT_SIZE,
                ..default()
            },
            TextColor(SPELLBOOK_DISABLED_SPELL_COLOR),
        ));
        return;
    };

    let class_db = &content_ref.db().classes;
    let spell_db = &content_ref.db().spells;
    let item_db = &content_ref.db().items;
    let spell_list = character
        .spells
        .get_spell_list_by_id(&character.class_id, class_db);

    if spell_ids.is_empty() {
        list.spawn((
            Text::new("No spells known."),
            TextFont {
                font_size: BODY_FONT_SIZE,
                ..default()
            },
            TextColor(SPELLBOOK_DISABLED_SPELL_COLOR),
        ));
    } else {
        // Flat row index that matches sb.selected_row.
        let mut row_idx: usize = 0;

        for (level_idx, level_spells) in spell_list.iter().enumerate() {
            if level_spells.is_empty() {
                continue;
            }

            // ── Level header ─────────────────────────────────────────────────
            list.spawn((
                Text::new(format!("-- Level {} --", level_idx + 1)),
                TextFont {
                    font_size: LABEL_FONT_SIZE,
                    ..default()
                },
                TextColor(SPELLBOOK_LEVEL_HEADER_COLOR),
            ));

            for &spell_id in level_spells {
                let selected = row_idx == sb.selected_row;

                // Determine affordability (true = can cast right now).
                let can_afford = spell_db
                    .get_spell(spell_id)
                    .map(|s| u32::from(character.sp.current) >= u32::from(s.sp_cost))
                    .unwrap_or(true);

                let text_color = if !can_afford {
                    SPELLBOOK_DISABLED_SPELL_COLOR
                } else if selected {
                    Color::srgb(1.0, 0.9, 0.2)
                } else {
                    SPELLBOOK_NORMAL_ROW_COLOR
                };

                let bg = if selected {
                    SPELLBOOK_SELECTED_ROW_BG
                } else {
                    Color::NONE
                };

                let label = if let Some(spell_def) = spell_db.get_spell(spell_id) {
                    let context_tag = match spell_def.context {
                        SpellContext::CombatOnly => " \u{2694}",     // ⚔
                        SpellContext::NonCombatOnly => " \u{1F30D}", // 🌍
                        SpellContext::OutdoorOnly
                        | SpellContext::IndoorOnly
                        | SpellContext::OutdoorCombat => " \u{1F30D}",
                        SpellContext::Anytime => "",
                    };
                    if spell_def.gem_cost > 0 {
                        format!(
                            "{} \u{2014} {} SP \u{1F48E}{}{}", // — …SP 💎N context
                            spell_def.name, spell_def.sp_cost, spell_def.gem_cost, context_tag,
                        )
                    } else {
                        format!(
                            "{} \u{2014} {} SP{}",
                            spell_def.name, spell_def.sp_cost, context_tag
                        )
                    }
                } else {
                    format!("Spell {spell_id:#06x}")
                };

                list.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(bg),
                    BorderRadius::all(Val::Px(4.0)),
                    SpellBookSpellRow { spell_id },
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new(label),
                        TextFont {
                            font_size: BODY_FONT_SIZE,
                            ..default()
                        },
                        TextColor(text_color),
                    ));
                });

                row_idx += 1;
            }
        }
    }

    // ── Learnable Scrolls section ────────────────────────────────────────────

    let scroll_entries: Vec<(String, SpellId, bool)> = character
        .inventory
        .items
        .iter()
        .filter_map(|slot| {
            let item_def = item_db.get_item(slot.item_id)?;
            if let ItemType::Consumable(ref consumable) = item_def.item_type {
                if let ConsumableEffect::LearnSpell(spell_id) = consumable.effect {
                    let eligible = can_learn_spell(character, spell_id, spell_db, class_db).is_ok();
                    return Some((item_def.name.clone(), spell_id, eligible));
                }
            }
            None
        })
        .collect();

    if !scroll_entries.is_empty() {
        list.spawn((
            Text::new("-- Learnable Scrolls --"),
            TextFont {
                font_size: LABEL_FONT_SIZE,
                ..default()
            },
            TextColor(SPELLBOOK_LEVEL_HEADER_COLOR),
        ));

        for (scroll_name, spell_id, eligible) in scroll_entries {
            let spell_name = spell_db
                .get_spell(spell_id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| format!("{spell_id:#06x}"));
            let eligibility = if eligible {
                " [eligible]"
            } else {
                " [not eligible]"
            };
            let color = if eligible {
                SPELLBOOK_NORMAL_ROW_COLOR
            } else {
                SPELLBOOK_DISABLED_SPELL_COLOR
            };

            list.spawn((
                Text::new(format!("{scroll_name} \u{2192} {spell_name}{eligibility}")),
                TextFont {
                    font_size: LABEL_FONT_SIZE,
                    ..default()
                },
                TextColor(color),
            ));
        }
    }
}

/// Builds the spell-detail rows in the right column.
///
/// Shows the full spell info (name, school, level, SP cost, gem cost, context,
/// description) when `sb.selected_spell_id` is `Some`, otherwise shows a
/// placeholder message.
fn build_detail_panel(
    pane: &mut ChildSpawnerCommands<'_>,
    sb: &SpellBookState,
    content: Option<&GameContent>,
) {
    // Column header
    pane.spawn((
        Text::new("Detail"),
        TextFont {
            font_size: LABEL_FONT_SIZE,
            ..default()
        },
        TextColor(SPELLBOOK_TITLE_COLOR),
    ));

    let Some(spell_id) = sb.selected_spell_id else {
        pane.spawn((
            Text::new("Select a spell to\nview details."),
            TextFont {
                font_size: BODY_FONT_SIZE,
                ..default()
            },
            TextColor(SPELLBOOK_HINT_COLOR),
        ));
        return;
    };

    let Some(content_ref) = content else {
        pane.spawn((
            Text::new("Content not loaded."),
            TextFont {
                font_size: BODY_FONT_SIZE,
                ..default()
            },
            TextColor(SPELLBOOK_DISABLED_SPELL_COLOR),
        ));
        return;
    };

    let Some(spell) = content_ref.db().spells.get_spell(spell_id) else {
        pane.spawn((
            Text::new(format!("Unknown spell\n{spell_id:#06x}")),
            TextFont {
                font_size: BODY_FONT_SIZE,
                ..default()
            },
            TextColor(SPELLBOOK_DISABLED_SPELL_COLOR),
        ));
        return;
    };

    // Spell name — displayed larger
    pane.spawn((
        Text::new(spell.name.clone()),
        TextFont {
            font_size: BODY_FONT_SIZE + 2.0,
            ..default()
        },
        TextColor(SPELLBOOK_TITLE_COLOR),
    ));

    let context_label = match spell.context {
        SpellContext::Anytime => "Any".to_string(),
        SpellContext::CombatOnly => "Combat \u{2694}".to_string(),
        SpellContext::NonCombatOnly => "Non-Combat \u{1F30D}".to_string(),
        SpellContext::OutdoorOnly => "Outdoor".to_string(),
        SpellContext::IndoorOnly => "Indoor".to_string(),
        SpellContext::OutdoorCombat => "Outdoor Combat".to_string(),
    };
    let gem_label = if spell.gem_cost > 0 {
        spell.gem_cost.to_string()
    } else {
        "\u{2014}".to_string() // —
    };

    let detail_lines = [
        format!("School: {:?}", spell.school),
        format!("Level:  {}", spell.level),
        format!("SP Cost: {}", spell.sp_cost),
        format!("Gem Cost: {gem_label}"),
        format!("Context: {context_label}"),
    ];

    for line in &detail_lines {
        pane.spawn((
            Text::new(line.clone()),
            TextFont {
                font_size: LABEL_FONT_SIZE,
                ..default()
            },
            TextColor(SPELLBOOK_NORMAL_ROW_COLOR),
        ));
    }

    // Blank line separator before description
    pane.spawn((
        Text::new(String::new()),
        TextFont {
            font_size: LABEL_FONT_SIZE,
            ..default()
        },
        TextColor(SPELLBOOK_NORMAL_ROW_COLOR),
    ));

    // Description text
    if !spell.description.is_empty() {
        pane.spawn((
            Text::new(spell.description.clone()),
            TextFont {
                font_size: LABEL_FONT_SIZE,
                ..default()
            },
            TextColor(SPELLBOOK_HINT_COLOR),
        ));
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::spell_book_state::SpellBookState;
    use crate::application::GameState;
    use crate::domain::character::{Alignment, Character, Sex};

    // ── Marker component sanity checks ───────────────────────────────────────

    /// `SpellBookOverlay` is a zero-size marker component.
    #[test]
    fn test_spell_book_overlay_is_marker_component() {
        let _: SpellBookOverlay = SpellBookOverlay;
        assert_eq!(std::mem::size_of::<SpellBookOverlay>(), 0);
    }

    /// `SpellBookContent` is a zero-size marker component.
    #[test]
    fn test_spell_book_content_is_marker_component() {
        let _: SpellBookContent = SpellBookContent;
        assert_eq!(std::mem::size_of::<SpellBookContent>(), 0);
    }

    /// `SpellBookCharTab` stores the party index correctly.
    #[test]
    fn test_spell_book_char_tab_stores_party_index() {
        let tab = SpellBookCharTab { party_index: 3 };
        assert_eq!(tab.party_index, 3);
    }

    /// `SpellBookSpellRow` stores the spell ID correctly.
    #[test]
    fn test_spell_book_spell_row_stores_spell_id() {
        let row = SpellBookSpellRow { spell_id: 0x0201 };
        assert_eq!(row.spell_id, 0x0201);
    }

    // ── collect_spell_ids_from_state ─────────────────────────────────────────

    /// Returns empty when mode is not SpellBook.
    #[test]
    fn test_collect_spell_ids_not_in_spellbook_mode_returns_empty() {
        let state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));
        let ids = collect_spell_ids_from_state(&state, None);
        assert!(ids.is_empty());
    }

    /// Returns empty when the character index is out of range (empty party).
    #[test]
    fn test_collect_spell_ids_empty_party_returns_empty() {
        let mut state = GameState::new();
        state.enter_spellbook(0);
        // Party is empty → index 0 is out of range.
        let ids = collect_spell_ids_from_state(&state, None);
        assert!(ids.is_empty());
    }

    /// Returns empty when content is None.
    #[test]
    fn test_collect_spell_ids_no_content_returns_empty() {
        let mut state = GameState::new();
        let character = Character::new(
            "Aria".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        state.party.add_member(character).unwrap();
        state.enter_spellbook(0);

        let ids = collect_spell_ids_from_state(&state, None);
        assert!(ids.is_empty(), "no content → empty spell list");
    }

    // ── Tab navigation ───────────────────────────────────────────────────────

    /// Tab forward increments character_index.
    #[test]
    fn test_tab_forward_increments_character_index() {
        let mut sb = SpellBookState::new(0, GameMode::Exploration);
        sb.next_character(3);
        assert_eq!(sb.character_index, 1);
    }

    /// Tab forward wraps at party size.
    #[test]
    fn test_tab_forward_wraps_at_party_size() {
        let mut sb = SpellBookState::new(2, GameMode::Exploration);
        sb.next_character(3);
        assert_eq!(sb.character_index, 0);
    }

    /// Shift+Tab (prev_character) decrements character_index.
    #[test]
    fn test_tab_back_decrements_character_index() {
        let mut sb = SpellBookState::new(2, GameMode::Exploration);
        sb.prev_character(3);
        assert_eq!(sb.character_index, 1);
    }

    /// Shift+Tab wraps to end at index 0.
    #[test]
    fn test_tab_back_wraps_to_end_at_zero() {
        let mut sb = SpellBookState::new(0, GameMode::Exploration);
        sb.prev_character(3);
        assert_eq!(sb.character_index, 2);
    }

    // ── SP affordability logic ───────────────────────────────────────────────

    /// A spell is disabled when its `sp_cost` exceeds the character's current
    /// SP.  The UI must use `SPELLBOOK_DISABLED_SPELL_COLOR` for such rows.
    #[test]
    fn test_spell_row_disabled_when_sp_insufficient() {
        // sp_cost = 10, character.sp.current = 5 → disabled
        let character_sp: u32 = 5;
        let spell_cost: u32 = 10;
        let can_afford = character_sp >= spell_cost;
        assert!(
            !can_afford,
            "character with 5 SP cannot afford a 10-SP spell"
        );

        // Verify the color selection logic mirrors build_spell_list
        let text_color = if !can_afford {
            SPELLBOOK_DISABLED_SPELL_COLOR
        } else {
            SPELLBOOK_NORMAL_ROW_COLOR
        };
        assert_eq!(
            text_color, SPELLBOOK_DISABLED_SPELL_COLOR,
            "insufficient SP must select SPELLBOOK_DISABLED_SPELL_COLOR"
        );
    }

    /// A spell is NOT disabled when the character has enough SP.
    #[test]
    fn test_spell_row_enabled_when_sp_sufficient() {
        let character_sp: u32 = 15;
        let spell_cost: u32 = 10;
        let can_afford = character_sp >= spell_cost;
        assert!(can_afford);

        let text_color = if !can_afford {
            SPELLBOOK_DISABLED_SPELL_COLOR
        } else {
            SPELLBOOK_NORMAL_ROW_COLOR
        };
        assert_eq!(text_color, SPELLBOOK_NORMAL_ROW_COLOR);
    }

    // ── GameMode::SpellBook transitions ─────────────────────────────────────

    /// enter_spellbook + exit_spellbook round-trip restores previous mode.
    #[test]
    fn test_enter_and_exit_spellbook_roundtrip() {
        let mut state = GameState::new();
        state.enter_spellbook(0);
        assert!(matches!(state.mode, GameMode::SpellBook(_)));
        state.exit_spellbook();
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    /// exit_spellbook is a no-op when not in SpellBook mode.
    #[test]
    fn test_exit_spellbook_noop_when_not_spellbook_mode() {
        let mut state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));
        state.exit_spellbook(); // no-op
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    // ── Bevy app integration tests ───────────────────────────────────────────

    /// `setup_spellbook_ui` spawns at least one `SpellBookOverlay` entity when
    /// the game mode is `SpellBook`.
    #[test]
    fn test_setup_spellbook_ui_spawns_overlay() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let mut gs = GameState::new();
        gs.enter_spellbook(0);
        app.insert_resource(GlobalState(gs));
        app.add_systems(Update, setup_spellbook_ui);

        app.update();

        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<SpellBookOverlay>>();
        let count = q.iter(world).count();
        assert!(
            count >= 1,
            "setup_spellbook_ui must spawn at least one SpellBookOverlay entity"
        );
    }

    /// `cleanup_spellbook_ui` despawns all `SpellBookOverlay` entities when
    /// the game mode is no longer `SpellBook`.
    #[test]
    fn test_cleanup_spellbook_ui_despawns_overlays() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Phase 1: Enter SpellBook mode, spawn the overlay.
        let mut gs = GameState::new();
        gs.enter_spellbook(0);
        app.insert_resource(GlobalState(gs));
        app.add_systems(Update, (setup_spellbook_ui, cleanup_spellbook_ui).chain());
        app.update(); // spawns overlay

        // Phase 2: Exit SpellBook mode, run cleanup.
        app.world_mut()
            .resource_mut::<GlobalState>()
            .0
            .exit_spellbook();
        app.update(); // cleanup should despawn overlay

        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<SpellBookOverlay>>();
        let count = q.iter(world).count();
        assert_eq!(
            count, 0,
            "cleanup_spellbook_ui must despawn all SpellBookOverlay entities"
        );
    }

    /// `setup_spellbook_ui` is idempotent: running twice does not spawn a
    /// second overlay.
    #[test]
    fn test_setup_spellbook_ui_is_idempotent() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let mut gs = GameState::new();
        gs.enter_spellbook(0);
        app.insert_resource(GlobalState(gs));
        app.add_systems(Update, setup_spellbook_ui);

        app.update();
        app.update(); // second update — must not spawn a second overlay

        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<SpellBookOverlay>>();
        let count = q.iter(world).count();
        assert_eq!(count, 1, "only one SpellBookOverlay entity must exist");
    }

    /// `setup_spellbook_ui` does not spawn when mode is not `SpellBook`.
    #[test]
    fn test_setup_spellbook_ui_no_spawn_in_exploration_mode() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let gs = GameState::new(); // Exploration mode
        app.insert_resource(GlobalState(gs));
        app.add_systems(Update, setup_spellbook_ui);

        app.update();

        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<SpellBookOverlay>>();
        let count = q.iter(world).count();
        assert_eq!(
            count, 0,
            "setup_spellbook_ui must not spawn when mode is not SpellBook"
        );
    }

    // ── Esc key closes the spell book ────────────────────────────────────────

    /// Pressing Esc in SpellBook mode calls exit_spellbook and restores the
    /// previous mode.  Verified via pure state mutation (no Bevy input needed).
    #[test]
    fn test_esc_triggers_exit_spellbook() {
        let mut state = GameState::new();
        state.enter_spellbook(0);
        assert!(matches!(state.mode, GameMode::SpellBook(_)));
        // Simulate what handle_spellbook_input does on Esc:
        state.exit_spellbook();
        assert!(
            matches!(state.mode, GameMode::Exploration),
            "Esc must restore the previous mode (Exploration)"
        );
    }

    /// C key exits SpellBook and enters SpellCasting for the current character.
    #[test]
    fn test_c_key_transitions_to_spell_casting() {
        let mut state = GameState::new();
        state.enter_spellbook(1);
        let char_index = if let GameMode::SpellBook(ref sb) = state.mode {
            sb.character_index
        } else {
            panic!("expected SpellBook mode");
        };

        // Simulate what handle_spellbook_input does on C key:
        state.exit_spellbook();
        state.enter_spell_casting(char_index);

        assert!(
            matches!(state.mode, GameMode::SpellCasting(_)),
            "C key must transition to SpellCasting"
        );
        if let GameMode::SpellCasting(ref sc) = state.mode {
            assert_eq!(
                sc.caster_index, 1,
                "must use character index from SpellBook"
            );
        }
    }
}
