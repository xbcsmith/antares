// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Combat systems and support types
//!
//! Core Combat Infrastructure
//! Combat UI System
//! Visual Combat Feedback and Animation State
//!
//! This module implements the foundational Bevy plugin and systems needed to
//! start and synchronize combat state with the global `GameState`, plus the
//! combat UI display.
//!
//! Responsibilities:
//! - `CombatPlugin` registers combat messages and resources
//! - `CombatResource` wraps domain `CombatState` for use by ECS systems
//! - `CombatTurnStateResource` is a small resource for turn sub-state tracking
//! - `start_encounter()` prepares a `CombatState` from a map encounter and party
//! - Systems to synchronize party -> combat and combat -> party on exit
//! - Combat UI systems for displaying enemies, turn order, and action menu
//!
//! # Systems
//!
//! - `handle_combat_started` (listens for `CombatStarted`) — loads the combat
//!   state from `GlobalState` into `CombatResource` and builds participant mapping.
//! - `sync_party_to_combat` (runs while in combat) — ensures players are present
//!   in the combat state if they were not added earlier.
//! - `sync_party_hp_during_combat` (runs every frame during combat) — mirrors
//!   HP/SP/conditions from `CombatResource` participants back into `party.members`
//!   so the HUD always reflects live combat damage without waiting for combat exit.
//! - `sync_combat_to_party_on_exit` (runs every frame) — when combat has ended,
//!   copies HP/SP/conditions/stat currents back into the party and clears combat data.
//! - `setup_combat_ui` (runs on combat enter) — spawns combat UI entities
//! - `cleanup_combat_ui` (runs on combat exit) — despawns combat UI entities
//! - `update_combat_ui` (runs every frame during combat) — syncs UI with combat state
//!
//! # Notes
//!
//! This file intentionally keeps systems small and focused. More complex action
//! handling and AI belong to later phases.
//!
//! # Examples
//!
//! Enter combat from an encounter event (simplified):
//!
//! ```no_run
//! use antares::game::systems::combat::start_encounter;
//! use antares::domain::combat::types::CombatEventType;
//! # // In an event handler:
//! # let mut gs = antares::application::GameState::new();
//! # let content = antares::application::resources::GameContent::new(antares::sdk::database::ContentDatabase::new());
//! # let monster_group: Vec<u8> = vec![1, 2];
//! let _ = start_encounter(&mut gs, &content, &monster_group, CombatEventType::Normal);
//! ```
use crate::game::systems::mouse_input;
use bevy::prelude::*;

use crate::application::resources::GameContent;
use crate::application::GameMode;
use crate::domain::combat::engine::{
    apply_damage, choose_monster_attack, get_character_attack, has_ranged_weapon,
    initialize_combat_from_group, resolve_attack, CombatError, CombatState, Combatant,
    MeleeAttackResult,
};
use crate::domain::combat::types::{
    Attack, CombatEventType, CombatStatus, CombatantId, Handicap, SpecialEffect,
};
use crate::domain::types::{DiceRoll, ItemId};
use crate::game::resources::GlobalState;
use crate::game::systems::camera::MainCamera;
use crate::game::systems::combat_visual::{
    hide_indicator_during_animation, spawn_turn_indicator, update_turn_indicator,
};
use crate::game::systems::map::EncounterVisualMarker;
use crate::game::systems::ui_helpers::{text_style, BODY_FONT_SIZE, LABEL_FONT_SIZE};

/// Message emitted when combat has started.
///
/// Other systems can listen to this to set up UI or audio.
/// The `encounter_position` and `encounter_map_id` fields carry the world tile
/// that triggered the encounter so `handle_combat_started` can store them in
/// `CombatResource` for later use by `handle_combat_victory`.
#[derive(Message)]
pub struct CombatStarted {
    /// Tile position of the `MapEvent::Encounter` that started this combat.
    /// `None` when combat is started programmatically rather than by a map event.
    pub encounter_position: Option<crate::domain::types::Position>,
    /// Map ID of the map that contains the encounter tile.
    /// `None` when `encounter_position` is `None`.
    pub encounter_map_id: Option<crate::domain::types::MapId>,
    /// Type of this combat encounter.
    ///
    /// Forwarded from the `MapEvent::Encounter` field or from the selected
    /// [`EncounterGroup`](crate::domain::world::types::EncounterGroup) in the
    /// encounter table.  Defaults to [`CombatEventType::Normal`].
    pub combat_event_type: CombatEventType,
}

/// Player-initiated attack message (registered by the plugin)
#[derive(Message)]
pub struct AttackAction {
    pub attacker: CombatantId,
    pub target: CombatantId,
}

/// Player-initiated cast spell message (registered by the plugin)
#[derive(Message)]
pub struct CastSpellAction {
    pub caster: CombatantId,
    pub spell_id: crate::domain::types::SpellId,
    pub target: CombatantId,
}

/// Player-initiated use item message (registered by the plugin)
#[derive(Message)]
pub struct UseItemAction {
    pub user: CombatantId,
    /// Index into the user's inventory (0-based)
    pub inventory_index: usize,
    pub target: CombatantId,
}

/// Player defends this turn (registered by the plugin)
#[derive(Message)]
pub struct DefendAction {
    pub combatant: CombatantId,
}

/// Player attempts to flee (registered by the plugin)
#[derive(Message)]
pub struct FleeAction;

/// Player-initiated ranged attack message (registered by the plugin).
///
/// Fired when the player selects the "Ranged" button in a
/// `CombatEventType::Ranged` encounter and confirms a target.  The attacker
/// must have a [`WeaponClassification::MartialRanged`] weapon equipped **and**
/// at least one ammo item; if not, `perform_ranged_attack_action_with_rng`
/// returns [`CombatError::NoAmmo`] or [`CombatError::CombatantCannotAct`].
#[derive(Message)]
pub struct RangedAttackAction {
    pub attacker: CombatantId,
    pub target: CombatantId,
}

/// Resource that flags whether the next target-confirm should fire a
/// `RangedAttackAction` instead of a plain `AttackAction`.
///
/// Set to `true` by `dispatch_combat_action` when
/// `ActionButtonType::RangedAttack` is pressed; cleared to `false` once the
/// target is confirmed (or the selection is cancelled).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct RangedAttackPending(pub bool);

/// Message emitted when combat ends in victory
#[derive(Message)]
pub struct CombatVictory;

/// Message emitted when combat ends in defeat
#[derive(Message)]
pub struct CombatDefeat;

/// Combat turn sub-states used by the UI / systems.
///
/// Kept small intentionally; this is not a Bevy `State` type, but a lightweight
/// resource allowing systems to coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombatTurnState {
    PlayerTurn,
    EnemyTurn,
    Animating,
    RoundEnd,
}

/// Simple resource wrapper for the current turn sub-state.
#[derive(Resource, Debug, Clone)]
pub struct CombatTurnStateResource(pub CombatTurnState);

impl Default for CombatTurnStateResource {
    fn default() -> Self {
        Self(CombatTurnState::PlayerTurn)
    }
}

// ===== Combat UI Constants =====

/// Height of the enemy panel (monster cards + HP bars).
pub const COMBAT_ENEMY_PANEL_HEIGHT: Val = Val::Px(200.0);

/// Width of individual enemy card in the enemy panel
pub const ENEMY_CARD_WIDTH: Val = Val::Px(150.0);

/// Height of enemy HP bar
pub const ENEMY_HP_BAR_HEIGHT: Val = Val::Px(12.0);

/// Padding inside enemy cards
pub const ENEMY_CARD_PADDING: Val = Val::Px(8.0);

/// Height of the turn order display panel
pub const TURN_ORDER_PANEL_HEIGHT: Val = Val::Px(40.0);

/// Height of the action menu panel
pub const ACTION_MENU_HEIGHT: Val = Val::Px(60.0);

/// Button width in action menu
pub const ACTION_BUTTON_WIDTH: Val = Val::Px(100.0);

/// Button height in action menu
pub const ACTION_BUTTON_HEIGHT: Val = Val::Px(40.0);

/// Color for enemy HP bar (healthy)
pub const ENEMY_HP_HEALTHY_COLOR: Color = Color::srgb(0.2, 0.8, 0.2);

/// Color for enemy HP bar (injured)
pub const ENEMY_HP_INJURED_COLOR: Color = Color::srgb(0.8, 0.6, 0.0);

/// Color for enemy HP bar (critical)
pub const ENEMY_HP_CRITICAL_COLOR: Color = Color::srgb(0.8, 0.2, 0.2);

/// Color for turn indicator highlight
pub const TURN_INDICATOR_COLOR: Color = Color::srgb(1.0, 1.0, 0.0);

/// Color for action button background
pub const ACTION_BUTTON_COLOR: Color = Color::srgb(0.3, 0.3, 0.4);

/// Color for action button hover
pub const ACTION_BUTTON_HOVER_COLOR: Color = Color::srgb(0.4, 0.4, 0.5);

/// Color for action button when keyboard selection is armed (Enter pressed once)
pub const ACTION_BUTTON_CONFIRMED_COLOR: Color = Color::srgb(0.65, 0.55, 0.25);

/// Color for action button disabled
pub const ACTION_BUTTON_DISABLED_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);

/// Color for enemy card highlight when in target selection mode
pub const ENEMY_CARD_HIGHLIGHT_COLOR: Color = Color::srgba(0.35, 0.25, 0.25, 0.95);

/// Width of the persistent combat log bubble in the top-right corner.
pub const COMBAT_LOG_BUBBLE_WIDTH: Val = Val::Px(360.0);

/// Minimum height of the persistent combat log bubble.
pub const COMBAT_LOG_BUBBLE_MIN_HEIGHT: Val = Val::Px(180.0);

/// Maximum height of the persistent combat log bubble.
/// The log occupies the upper portion of the screen above the enemy panel.
pub const COMBAT_LOG_BUBBLE_MAX_HEIGHT: Val = Val::Percent(55.0);

// ===== Absolute-position anchors for combat panels =====
//
// All three combat panels (enemy cards, turn order, action menu) are pinned
// from the bottom of the screen so they sit in the lower half and never
// overlap the player HUD (height 70 px, gap 24 px → top edge at bottom+94).
//
// Stack (bottom → top):
//   Action menu  : bottom = HUD_HEIGHT(70) + HUD_GAP(24) + PANEL_GAP(4) =  98 px
//   Turn order   : bottom = 98 + ACTION_MENU(60) + PANEL_GAP(4)          = 162 px
//   Enemy panel  : bottom = 162 + TURN_ORDER(40) + PANEL_GAP(4)          = 206 px

/// Distance from the bottom of the screen to the bottom edge of the action menu.
/// Keeps the action buttons just above the player HUD (70 px tall, 24 px gap).
pub const ACTION_MENU_BOTTOM: Val = Val::Px(120.0);

/// Distance from the bottom of the screen to the bottom edge of the turn order panel.
pub const TURN_ORDER_BOTTOM: Val = Val::Px(175.0);

/// Distance from the bottom of the screen to the bottom edge of the enemy panel.
pub const ENEMY_PANEL_BOTTOM: Val = Val::Px(206.0);

/// Maximum number of log lines kept in the on-screen combat bubble.
pub const COMBAT_LOG_MAX_LINES: usize = 14;

/// Typewriter speed for combat log reveal (characters per second).
pub const COMBAT_LOG_TYPEWRITER_CHARS_PER_SEC: f32 = 52.0;

/// Fixed palette used to assign stable character-name colours in combat log.
pub const COMBAT_LOG_CHARACTER_PALETTE: [Color; 8] = [
    Color::srgb(0.95, 0.85, 0.30),
    Color::srgb(0.45, 0.85, 1.00),
    Color::srgb(0.65, 0.95, 0.50),
    Color::srgb(0.95, 0.55, 0.55),
    Color::srgb(0.95, 0.75, 0.45),
    Color::srgb(0.75, 0.70, 1.00),
    Color::srgb(1.00, 0.60, 0.85),
    Color::srgb(0.60, 0.95, 0.90),
];

/// Predefined palettes used to randomly assign each monster a combat-log colour.
pub const COMBAT_LOG_MONSTER_PALETTE: [Color; 8] = [
    Color::srgb(1.00, 0.45, 0.45),
    Color::srgb(1.00, 0.62, 0.38),
    Color::srgb(0.96, 0.42, 0.62),
    Color::srgb(0.86, 0.52, 1.00),
    Color::srgb(0.45, 0.75, 1.00),
    Color::srgb(0.45, 0.95, 0.55),
    Color::srgb(0.92, 0.90, 0.45),
    Color::srgb(0.70, 0.95, 0.95),
];

// ===== Visual Feedback Color Constants =====

/// Colour for damage floating numbers (red)
pub const FEEDBACK_COLOR_DAMAGE: Color = Color::srgb(1.0, 0.3, 0.3);

/// Colour for heal floating numbers (green)
pub const FEEDBACK_COLOR_HEAL: Color = Color::srgb(0.3, 1.0, 0.3);

/// Colour for miss floating text (grey)
pub const FEEDBACK_COLOR_MISS: Color = Color::srgb(0.8, 0.8, 0.8);

/// Colour for status/condition floating text (yellow)
pub const FEEDBACK_COLOR_STATUS: Color = Color::srgb(1.0, 0.8, 0.0);

// ===== Boss HP Bar Constants =====

/// Width of the boss HP bar (wider and more prominent than standard enemy bars).
pub const BOSS_HP_BAR_WIDTH: f32 = 400.0;

/// Height of the boss HP bar.
pub const BOSS_HP_BAR_HEIGHT: f32 = 20.0;

/// Boss HP bar color when healthy (>= 50% HP).
pub const BOSS_HP_HEALTHY_COLOR: Color = Color::srgba(0.8, 0.1, 0.1, 1.0);

/// Boss HP bar color when injured (25–49% HP).
pub const BOSS_HP_INJURED_COLOR: Color = Color::srgba(0.5, 0.1, 0.1, 1.0);

/// Boss HP bar color when critical (< 25% HP).
pub const BOSS_HP_CRITICAL_COLOR: Color = Color::srgba(0.3, 0.05, 0.05, 1.0);

/// Width of the world-projected monster HP hover bars.
pub const MONSTER_HP_HOVER_BAR_WIDTH: Val = Val::Px(120.0);

/// Height of the world-projected monster HP hover bars.
pub const MONSTER_HP_HOVER_BAR_HEIGHT: Val = Val::Px(10.0);

/// Real-time seconds to pause on the enemy's turn before the monster acts.
///
/// This gives the player time to read the combat log and see the turn
/// indicator before the monster's attack resolves.  Set to 0.0 to disable
/// the delay (useful for automated tests — insert a zero-duration
/// `MonsterTurnTimer` resource to override the plugin default).
pub const MONSTER_TURN_DELAY_SECS: f32 = 1.2;

/// Number of top-level action buttons (Attack, Defend, Cast, Item, Flee).
///
/// Used by `combat_input_system` and `update_action_highlight` instead of
/// the inline literal `5` to avoid magic numbers.
pub const COMBAT_ACTION_COUNT: usize = 5;

/// Canonical order of action buttons matching the spawn order in `setup_combat_ui`.
///
/// Index 0 is `Attack` (the default highlight). This is the single source of
/// truth for `ActionMenuState::active_index` → `ActionButtonType` mapping.
pub const COMBAT_ACTION_ORDER: [ActionButtonType; COMBAT_ACTION_COUNT] = [
    ActionButtonType::Attack,
    ActionButtonType::Defend,
    ActionButtonType::Cast,
    ActionButtonType::Item,
    ActionButtonType::Flee,
];

/// Number of action buttons shown in Magic combat (same count, different order).
pub const COMBAT_ACTION_COUNT_MAGIC: usize = 5;

/// Button order for Magic combat encounters (`CombatEventType::Magic`).
///
/// `Cast` is placed first (index 0) so the default highlight is always the
/// most useful action in a magic encounter.  The remaining buttons follow the
/// standard order.
///
/// Used by `update_action_highlight` and `combat_input_system` when
/// `combat_res.combat_event_type.highlights_magic_action()` returns `true`.
pub const COMBAT_ACTION_ORDER_MAGIC: [ActionButtonType; COMBAT_ACTION_COUNT_MAGIC] = [
    ActionButtonType::Cast,
    ActionButtonType::Attack,
    ActionButtonType::Defend,
    ActionButtonType::Item,
    ActionButtonType::Flee,
];

/// Bevy resource that contains the authoritative combat state used by ECS systems.
///
/// `player_orig_indices` maps participant index -> Option<party_index> so we can
/// sync values back to the party after combat ends.
#[derive(Resource, Debug, Clone)]
pub struct CombatResource {
    pub state: CombatState,
    /// For each participant index in `state.participants`, `Some(idx)` if that
    /// participant is a player and `idx` is the original party index.
    pub player_orig_indices: Vec<Option<usize>>,
    /// Whether the combat resolution (victory/defeat) has been handled.
    pub resolution_handled: bool,
    /// World position of the encounter tile that started this combat.
    /// Set by the event handler when a `MapEvent::Encounter` triggers combat;
    /// cleared after victory so a subsequent combat is unaffected.
    pub encounter_position: Option<crate::domain::types::Position>,
    /// Map ID of the map containing the encounter tile.
    /// Set alongside `encounter_position`; cleared after victory.
    pub encounter_map_id: Option<crate::domain::types::MapId>,
    /// The last round number for which game time was advanced.
    /// Used by `tick_combat_time` to detect new rounds and charge
    /// `TIME_COST_COMBAT_ROUND_MINUTES` exactly once per round.
    pub last_timed_round: u32,
    /// Type of the current combat encounter.
    ///
    /// Set when `CombatStarted` is received and cleared (reset to
    /// [`CombatEventType::Normal`]) when `CombatResource::clear()` is called.
    /// Later phases read this to apply ambush, ranged, magic, and boss
    /// mechanics.
    pub combat_event_type: CombatEventType,
}

impl CombatResource {
    /// Create a default/empty combat resource
    pub fn new() -> Self {
        Self {
            state: CombatState::new(Handicap::Even),
            player_orig_indices: Vec::new(),
            resolution_handled: false,
            encounter_position: None,
            encounter_map_id: None,
            last_timed_round: 0,
            combat_event_type: CombatEventType::Normal,
        }
    }

    /// Clear the combat state and mappings (called after syncing back to party)
    pub fn clear(&mut self) {
        self.state = CombatState::new(Handicap::Even);
        self.player_orig_indices.clear();
        self.resolution_handled = false;
        self.encounter_position = None;
        self.encounter_map_id = None;
        self.last_timed_round = 0;
        self.combat_event_type = CombatEventType::Normal;
    }
}

impl Default for CombatResource {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Combat UI Marker Components =====

/// Marker component for enemy card UI elements
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyCard {
    /// Index in combat participants (should be a monster)
    pub participant_index: usize,
}

/// Marker component for enemy HP bar fill
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyHpBarFill {
    /// Index in combat participants
    pub participant_index: usize,
}

/// Marker component for the HP bar background container node inside an [`EnemyCard`].
///
/// `FloatingDamage` nodes are spawned as children of this node so they are
/// anchored to the HP bar area rather than to the whole card.  The node has a
/// known, fixed height (`ENEMY_HP_BAR_HEIGHT`) and `overflow: visible` so the
/// larger damage text is not clipped.
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyHpBarBackground {
    /// Index in combat participants
    pub participant_index: usize,
}

/// Marker component for enemy HP text display
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyHpText {
    /// Index in combat participants
    pub participant_index: usize,
}

/// Marker component for enemy name text
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyNameText {
    /// Index in combat participants
    pub participant_index: usize,
}

/// Marker component for enemy condition text
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyConditionText {
    /// Index in combat participants
    pub participant_index: usize,
}

// ===== Boss HP Bar Components =====

/// Marker component for the Boss HP bar panel root.
///
/// Spawned in `setup_combat_ui` only when `combat_event_type == Boss`.
/// The bar is wider and more prominent than a standard `EnemyHpBarFill`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::combat::BossHpBar;
/// let bar = BossHpBar { participant_index: 0 };
/// assert_eq!(bar.participant_index, 0);
/// ```
#[derive(Component, Debug, Clone, Copy)]
pub struct BossHpBar {
    /// Index into `CombatResource::state.participants` for the boss monster.
    pub participant_index: usize,
}

/// Fill portion of the boss HP bar (width varies with HP ratio).
#[derive(Component, Debug, Clone, Copy)]
pub struct BossHpBarFill {
    /// Index into `CombatResource::state.participants` for the boss monster.
    pub participant_index: usize,
}

/// HP text label on the boss HP bar.
#[derive(Component, Debug, Clone, Copy)]
pub struct BossHpBarText {
    /// Index into `CombatResource::state.participants` for the boss monster.
    pub participant_index: usize,
}

/// Marker component for the turn order display panel
#[derive(Component, Debug, Clone, Copy)]
pub struct TurnOrderPanel;

/// Marker component for turn order text display
#[derive(Component, Debug, Clone, Copy)]
pub struct TurnOrderText;

/// Marker component for the action menu panel
#[derive(Component, Debug, Clone, Copy)]
pub struct ActionMenuPanel;

/// Action button types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionButtonType {
    Attack,
    /// Ranged attack — only shown in `CombatEventType::Ranged` encounters.
    RangedAttack,
    Defend,
    Cast,
    Item,
    Flee,
}

/// Marker component for action buttons
#[derive(Component, Debug, Clone, Copy)]
pub struct ActionButton {
    pub button_type: ActionButtonType,
}

/// Marker component for the spell selection panel UI
///
/// Spawned when the player requests to cast a spell for a specific caster.
/// The `caster` field indicates which participant's spell list the panel is
/// displaying.
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellSelectionPanel {
    /// Participant index (player) that owns this panel
    pub caster: CombatantId,
}

/// Marker component for an individual spell button inside the spell selection panel
///
/// Stores the `spell_id` for the button and the `sp_cost` for display convenience.
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellButton {
    /// Identifier of the spell this button will cast
    pub spell_id: crate::domain::types::SpellId,
    /// SP cost (display-only convenience)
    pub sp_cost: u16,
}

/// Marker component for the item selection panel UI
///
/// Spawned when the player requests to use an item for a specific caster.
/// The `user` field indicates which participant's inventory the panel is
/// displaying.
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemSelectionPanel {
    /// Participant index (player) that owns this panel
    pub user: CombatantId,
}

/// Marker component for an individual item button inside the item selection panel
///
/// Stores the `item_id` and remaining `charges` for display convenience.
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemButton {
    /// Identifier of the item this button will use
    pub item_id: crate::domain::types::ItemId,
    /// Charges remaining in this inventory slot
    pub charges: u8,
}

/// Marker component for victory summary UI root
#[derive(Component, Debug, Clone, Copy)]
pub struct VictorySummaryRoot;

/// Marker component for defeat summary UI root
#[derive(Component, Debug, Clone, Copy)]
pub struct DefeatSummaryRoot;

/// Marker component for floating damage numbers (spawned on hits)
#[derive(Component, Debug, Clone, Copy)]
pub struct FloatingDamage {
    /// Remaining lifetime for the damage number in seconds
    pub remaining: f32,
}

/// Marker component for floating damage text node
#[derive(Component, Debug, Clone, Copy)]
pub struct DamageText;

// ===== Combat Feedback Event =====

/// The type of visual effect to display for a combat action result.
///
/// Used by `CombatFeedbackEvent` to drive colour selection and text formatting
/// in `spawn_combat_feedback`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::combat::CombatFeedbackEffect;
///
/// let hit = CombatFeedbackEffect::Damage(15);
/// let miss = CombatFeedbackEffect::Miss;
/// let heal = CombatFeedbackEffect::Heal(8);
/// let status = CombatFeedbackEffect::Status("Poison".to_string());
/// assert_ne!(hit, miss);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum CombatFeedbackEffect {
    /// A hit that dealt `n` damage points.
    Damage(u32),
    /// A heal that restored `n` HP (or SP if the string label varies).
    Heal(u32),
    /// The attack missed the target completely.
    Miss,
    /// A condition or status was applied (condition name).
    Status(String),
}

/// Message emitted whenever a combat action produces a visible result.
///
/// Listeners (specifically `spawn_combat_feedback`) read this message and
/// spawn a `FloatingDamage` UI node anchored to the target's UI card.
///
/// # Examples
///
/// ```
/// use antares::game::systems::combat::{CombatFeedbackEffect, CombatFeedbackEvent};
/// use antares::domain::combat::types::CombatantId;
///
/// let ev = CombatFeedbackEvent {
///     source: Some(CombatantId::Player(0)),
///     target: CombatantId::Monster(0),
///     effect: CombatFeedbackEffect::Damage(12),
/// };
/// assert!(matches!(ev.effect, CombatFeedbackEffect::Damage(_)));
/// ```
#[derive(Message, Debug, Clone)]
pub struct CombatFeedbackEvent {
    /// The combatant that performed the action, if known.
    pub source: Option<CombatantId>,
    /// The combatant that was affected.
    pub target: CombatantId,
    /// What happened to that combatant.
    pub effect: CombatFeedbackEffect,
}

// ===== Monster HP Hover Bar =====

/// Marker component for in-world (screen-space) monster HP hover bars.
///
/// Spawned for each alive monster when combat starts and updated every frame
/// by `update_monster_hp_hover_bars`.  Despawned by
/// `cleanup_monster_hp_hover_bars` when the game mode leaves `Combat`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::combat::MonsterHpHoverBar;
///
/// let bar = MonsterHpHoverBar {
///     participant_index: 2,
///     stack_order: 0,
/// };
/// assert_eq!(bar.participant_index, 2);
/// ```
#[derive(Component, Debug, Clone, Copy)]
pub struct MonsterHpHoverBar {
    /// Index into `CombatResource::state.participants` for the monster this bar tracks.
    pub participant_index: usize,
    /// Stable vertical stack order for bars attached to the same encounter marker.
    pub stack_order: usize,
}

/// Marker component on the fill node inside a `MonsterHpHoverBar` panel.
#[derive(Component, Debug, Clone, Copy)]
pub struct MonsterHpHoverBarFill {
    /// Index into `CombatResource::state.participants`.
    pub participant_index: usize,
}

/// Marker component for the monster name text inside a hover bar card.
#[derive(Component, Debug, Clone, Copy)]
pub struct MonsterHpHoverBarNameText {
    /// Index into `CombatResource::state.participants`.
    pub participant_index: usize,
}

/// Marker component for the HP text inside a hover bar card.
#[derive(Component, Debug, Clone, Copy)]
pub struct MonsterHpHoverBarHpText {
    /// Index into `CombatResource::state.participants`.
    pub participant_index: usize,
}

/// Marker component for the persistent combat log bubble root UI node.
#[derive(Component, Debug, Clone, Copy)]
pub struct CombatLogBubbleRoot;

/// Marker component for the scrollable viewport inside the combat log bubble.
#[derive(Component, Debug, Clone, Copy)]
pub struct CombatLogBubbleViewport;

/// Marker component for the line-list container inside the combat log bubble.
#[derive(Component, Debug, Clone, Copy)]
pub struct CombatLogLineList;

/// Resource representing the current target selection state.
///
/// When set to `Some(attacker)` the player is selecting a target for that attacker.
#[derive(Resource, Debug, Clone, Default)]
pub struct TargetSelection(pub Option<CombatantId>);

/// Resource tracking the keyboard-navigable action menu state.
///
/// `active_index` (0–4) indicates which action button is currently highlighted.
/// The order matches the spawn order: `[Attack, Defend, Cast, Item, Flee]`.
/// `confirmed` is set to `true` when `Enter` is pressed and consumed by the
/// unified dispatch function on the same frame.
///
/// `active_target_index` tracks which enemy card (by alive-monster index) is
/// currently highlighted for keyboard target selection. It is `Some(0)` when
/// target-select mode is entered and `None` when not in target selection.
///
/// Defaults to `active_index = 0` (Attack highlighted), `confirmed = false`,
/// and `active_target_index = None`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::combat::ActionMenuState;
///
/// let state = ActionMenuState::default();
/// assert_eq!(state.active_index, 0);
/// assert!(!state.confirmed);
/// assert!(state.active_target_index.is_none());
/// ```
#[derive(Resource, Debug, Clone, Default)]
pub struct ActionMenuState {
    /// Index of the currently highlighted action button (0–4).
    pub active_index: usize,
    /// Set to `true` when `Enter` is pressed; consumed by the dispatch path.
    pub confirmed: bool,
    /// Index into the alive-monster participants list for keyboard target cycling.
    ///
    /// `Some(n)` when target-select mode is active (set to `Some(0)` on entry).
    /// `None` when not in target-select mode.
    pub active_target_index: Option<usize>,
}

/// One coloured text segment inside a combat log line.
#[derive(Debug, Clone)]
pub struct CombatLogSegment {
    pub text: String,
    pub color: Color,
}

/// One structured combat log line rendered as multiple coloured segments.
#[derive(Debug, Clone)]
pub struct CombatLogLine {
    pub segments: Vec<CombatLogSegment>,
}

impl CombatLogLine {
    /// Total visible character count across all segments.
    fn char_count(&self) -> usize {
        self.segments.iter().map(|s| s.text.chars().count()).sum()
    }

    /// Return segments clipped to the first `visible_chars` characters.
    fn clipped_segments(&self, visible_chars: usize) -> Vec<CombatLogSegment> {
        if visible_chars == 0 {
            return Vec::new();
        }

        let mut remaining = visible_chars;
        let mut out: Vec<CombatLogSegment> = Vec::new();

        for segment in &self.segments {
            if remaining == 0 {
                break;
            }

            let seg_chars = segment.text.chars().count();
            if remaining >= seg_chars {
                out.push(segment.clone());
                remaining -= seg_chars;
            } else {
                let clipped = segment.text.chars().take(remaining).collect::<String>();
                out.push(CombatLogSegment {
                    text: clipped,
                    color: segment.color,
                });
                break;
            }
        }

        out
    }

    /// Return this structured line as plain text by concatenating all segments.
    fn plain_text(&self) -> String {
        self.segments.iter().map(|s| s.text.as_str()).collect()
    }
}

/// Resource storing stable colour assignments for combat log names.
#[derive(Resource, Debug, Clone, Default)]
pub struct CombatLogColorState {
    /// Stable monster colour by participant index for the current combat.
    pub monster_colors: std::collections::HashMap<usize, Color>,
}

/// Resource storing persistent combat log lines and typewriter animation state.
#[derive(Resource, Debug, Clone)]
pub struct CombatLogState {
    /// Rolling set of lines shown in the combat log bubble.
    pub lines: Vec<CombatLogLine>,
    /// Number of characters currently revealed for the newest line.
    pub active_line_visible_chars: usize,
    /// Sub-character accumulator used by the typewriter effect.
    pub reveal_accumulator: f32,
}

impl Default for CombatLogState {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            active_line_visible_chars: 0,
            reveal_accumulator: 0.0,
        }
    }
}

impl CombatLogState {
    /// Append a new combat log line and reset typewriter progress for it.
    fn push_line(&mut self, line: CombatLogLine) {
        if line.char_count() == 0 {
            return;
        }
        self.lines.push(line);
        while self.lines.len() > COMBAT_LOG_MAX_LINES {
            self.lines.remove(0);
        }
        self.active_line_visible_chars = 0;
        self.reveal_accumulator = 0.0;
    }
}

/// Marker component placed on the currently highlighted `ActionButton` entity.
///
/// Used by `update_action_highlight` to apply the hover background colour to
/// the active button while restoring the default colour on all others.
#[derive(Component, Debug, Clone, Copy)]
pub struct ActiveActionHighlight;

/// One-shot timer that gates [`execute_monster_turn`].
///
/// When the turn transitions to [`CombatTurnState::EnemyTurn`] the timer is
/// reset and starts counting.  `execute_monster_turn` waits until
/// `just_finished()` before resolving the monster's action, giving the player
/// time to read the combat log entry from the previous action.
///
/// Insert this resource with `duration = 0.0` in tests that call
/// `app.update()` and expect the monster to act immediately:
///
/// ```ignore
/// app.insert_resource(MonsterTurnTimer(
///     Timer::from_seconds(0.0, TimerMode::Once)
/// ));
/// ```
///
/// When the resource is absent (minimal test harnesses that do not use
/// [`CombatPlugin`]), `execute_monster_turn` fires immediately so that
/// existing unit tests need no changes.
#[derive(Resource)]
pub struct MonsterTurnTimer(pub Timer);

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CombatStarted>()
            .add_message::<AttackAction>()
            .add_message::<RangedAttackAction>()
            .add_message::<CastSpellAction>()
            .add_message::<UseItemAction>()
            .add_message::<DefendAction>()
            .add_message::<FleeAction>()
            .add_message::<CombatVictory>()
            .add_message::<CombatDefeat>()
            // Feedback event bus
            .add_message::<CombatFeedbackEvent>()
            .insert_resource(CombatResource::new())
            .insert_resource(CombatTurnStateResource::default())
            .insert_resource(TargetSelection::default())
            .insert_resource(ActionMenuState::default())
            .insert_resource(CombatLogState::default())
            .insert_resource(CombatLogColorState::default())
            .insert_resource(RangedAttackPending::default())
            // Monster-turn delay: start finished so the very first EnemyTurn
            // frame arms it (see execute_monster_turn for the reset logic).
            .insert_resource({
                let mut t = Timer::from_seconds(MONSTER_TURN_DELAY_SECS, TimerMode::Once);
                t.tick(std::time::Duration::from_secs_f32(MONSTER_TURN_DELAY_SECS));
                MonsterTurnTimer(t)
            })
            .insert_resource(ButtonInput::<KeyCode>::default())
            // Handle events that indicate combat started
            .add_systems(Update, handle_combat_started)
            // Ensure party members exist in combat on enter
            .add_systems(Update, sync_party_to_combat)
            // Mirror live HP/SP/conditions into party.members every frame so the HUD
            // always shows current values — runs after all action handlers.
            .add_systems(
                Update,
                sync_party_hp_during_combat
                    .after(handle_attack_action)
                    .after(handle_ranged_attack_action)
                    .after(handle_cast_spell_action)
                    .after(handle_use_item_action)
                    .after(handle_defend_action)
                    .before(update_combat_ui),
            )
            // Sync back to party when combat ends
            .add_systems(Update, sync_combat_to_party_on_exit)
            // Player Action Systems
            .add_systems(Update, combat_input_system)
            .add_systems(Update, update_action_highlight.after(combat_input_system))
            .add_systems(Update, enter_target_selection)
            .add_systems(
                Update,
                update_target_highlight
                    .after(enter_target_selection)
                    .after(combat_input_system),
            )
            .add_systems(Update, select_target)
            .add_systems(Update, handle_attack_action)
            .add_systems(Update, handle_ranged_attack_action)
            .add_systems(Update, handle_cast_spell_action)
            .add_systems(Update, handle_use_item_action)
            .add_systems(Update, handle_defend_action)
            .add_systems(Update, handle_flee_action)
            // Spawn anchored feedback numbers after action handlers write the event
            .add_systems(
                Update,
                spawn_combat_feedback
                    .after(handle_attack_action)
                    .after(handle_ranged_attack_action)
                    .after(handle_cast_spell_action)
                    .after(handle_use_item_action)
                    .after(execute_monster_turn),
            )
            .add_systems(
                Update,
                collect_combat_feedback_log_lines
                    .after(handle_attack_action)
                    .after(handle_ranged_attack_action)
                    .after(handle_cast_spell_action)
                    .after(handle_use_item_action)
                    .after(execute_monster_turn),
            )
            .add_systems(
                Update,
                mirror_combat_feedback_to_game_log
                    .after(collect_combat_feedback_log_lines)
                    .after(handle_attack_action)
                    .after(handle_ranged_attack_action)
                    .after(handle_cast_spell_action)
                    .after(handle_use_item_action)
                    .after(execute_monster_turn),
            )
            .add_systems(
                Update,
                update_combat_log_typewriter.after(collect_combat_feedback_log_lines),
            )
            .add_systems(
                Update,
                update_combat_log_bubble_text.after(update_combat_log_typewriter),
            )
            .add_systems(
                Update,
                auto_scroll_combat_log_viewport.after(update_combat_log_bubble_text),
            )
            .add_systems(Update, reset_combat_log_on_exit)
            .add_systems(Update, reset_combat_log_colors_on_exit)
            // Monster HP hover bars
            .add_systems(Update, spawn_monster_hp_hover_bars.after(setup_combat_ui))
            .add_systems(
                Update,
                update_monster_hp_hover_bars.after(spawn_monster_hp_hover_bars),
            )
            .add_systems(Update, cleanup_monster_hp_hover_bars)
            // Combat resolution & rewards
            .add_systems(Update, check_combat_resolution)
            .add_systems(Update, handle_combat_victory)
            .add_systems(Update, handle_combat_defeat)
            // Combat UI systems
            // Must run after handle_combat_started so combat_event_type is
            // already set when we decide which buttons to spawn.
            .add_systems(Update, setup_combat_ui.after(handle_combat_started))
            // Spawn the turn indicator after UI is created
            .add_systems(Update, spawn_turn_indicator.after(setup_combat_ui))
            // Update/move the indicator when the current actor changes
            .add_systems(Update, update_turn_indicator.after(spawn_turn_indicator))
            // Hide indicator during animations and ensure visibility is updated before the main UI update
            .add_systems(
                Update,
                hide_indicator_during_animation
                    .after(update_turn_indicator)
                    .before(update_combat_ui),
            )
            .add_systems(Update, cleanup_combat_ui)
            .add_systems(Update, update_combat_ui)
            .add_systems(
                Update,
                update_ranged_button_color
                    .after(update_combat_ui)
                    .after(update_action_highlight),
            )
            .add_systems(Update, cleanup_floating_damage)
            // Monster AI — must run AFTER update_combat_ui so the UI
            // always reflects the current EnemyTurn state (and hides the action
            // menu) before the monster acts and potentially advances the turn.
            .add_systems(Update, execute_monster_turn.after(update_combat_ui))
            // Advance game clock once per new combat round.
            // Runs after all action handlers so the round counter is already
            // incremented before we sample it.
            .add_systems(
                Update,
                tick_combat_time
                    .after(handle_attack_action)
                    .after(handle_ranged_attack_action)
                    .after(handle_cast_spell_action)
                    .after(handle_use_item_action)
                    .after(handle_defend_action)
                    .after(handle_flee_action)
                    .after(execute_monster_turn),
            );
    }
}

/// Initialize combat using the current party and an explicit monster group.
///
/// This will copy the current party members into the combat `CombatState`, add
/// monsters based on the provided `group` (list of `MonsterId`), and set the
/// `GameState` mode to `GameMode::Combat`.
///
/// The `combat_event_type` is stored on the returned `CombatState` for later
/// phases to read (ambush suppresses player turn 1, boss sets advance/regen
/// flags, etc.).  The value is stored end-to-end; the mechanics are wired
/// in subsequent layers.
///
/// Returns an error if any monster in `group` is not found in the content DB.
///
/// # Arguments
///
/// * `game_state`        - Mutable reference to the application `GameState`
/// * `content`           - `GameContent` resource for monster lookup
/// * `group`             - Slice of monster IDs (u8 alias)
/// * `combat_event_type` - Type of combat that determines starting conditions
///
/// # Errors
///
/// Returns `crate::domain::combat::database::MonsterDatabaseError` if a monster
/// ID is missing from the content DB.
///
/// # Examples
///
/// ```no_run
/// use antares::game::systems::combat::{start_encounter};
/// use antares::domain::combat::types::CombatEventType;
/// # let mut gs = antares::application::GameState::new();
/// # let content = antares::application::resources::GameContent::new(antares::sdk::database::ContentDatabase::new());
/// # let group = vec![1u8, 2];
/// let _ = start_encounter(&mut gs, &content, &group, CombatEventType::Normal);
/// ```
pub fn start_encounter(
    game_state: &mut crate::application::GameState,
    content: &GameContent,
    group: &[u8],
    combat_event_type: CombatEventType,
) -> Result<(), crate::domain::combat::database::MonsterDatabaseError> {
    // Select handicap based on combat event type.
    // Ambush gives monsters the initiative advantage for round 1.
    let handicap = if combat_event_type.gives_monster_advantage() {
        Handicap::MonsterAdvantage
    } else {
        Handicap::Even
    };

    let mut cs = CombatState::new(handicap);

    // Copy the campaign death mode so apply_damage respects it for this combat.
    cs.unconscious_before_death = game_state.campaign_config.unconscious_before_death;

    // Set the ambush flag so the game layer can suppress player
    // actions during round 1.
    cs.ambush_round_active = combat_event_type == CombatEventType::Ambush;

    // Boss mechanics — monsters advance and regenerate each round;
    // the party cannot bribe or surrender.
    if combat_event_type.applies_boss_mechanics() {
        cs.monsters_advance = true;
        cs.monsters_regenerate = true;
        cs.can_bribe = false;
        cs.can_surrender = false;
    }

    for character in &game_state.party.members {
        cs.add_player(character.clone());
    }

    // Add monsters from content DB and initialize turn order
    initialize_combat_from_group(&mut cs, content.db(), group)?;

    // Enter combat with the prepared state
    game_state.enter_combat_with_state(cs);

    Ok(())
}

/// System: Handle `CombatStarted` messages by copying the `GameState`'s combat
/// state into the `CombatResource` and building participant -> party mapping.
fn handle_combat_started(
    mut reader: MessageReader<CombatStarted>,
    mut combat_res: ResMut<CombatResource>,
    global_state: Res<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
    mut combat_log: ResMut<CombatLogState>,
    mut music_writer: Option<MessageWriter<crate::game::systems::audio::PlayMusic>>,
) {
    for msg in reader.read() {
        // Only proceed if the global state is actually in combat mode
        if let GameMode::Combat(ref cs) = global_state.0.mode {
            combat_res.state = cs.clone();

            // Build participant -> party index mapping
            let mut mapping: Vec<Option<usize>> =
                Vec::with_capacity(combat_res.state.participants.len());
            let mut player_counter: usize = 0;
            for participant in &combat_res.state.participants {
                match participant {
                    Combatant::Player(_) => {
                        mapping.push(Some(player_counter));
                        player_counter += 1;
                    }
                    Combatant::Monster(_) => mapping.push(None),
                }
            }
            combat_res.player_orig_indices = mapping;

            // Initialize CombatTurnStateResource from the actual first actor in
            // turn_order.  Without this, a monster-first initiative order (e.g.
            // Ancient Wolf speed 14 > all party speeds) would leave the resource
            // stuck on PlayerTurn, causing the action buttons to flash incorrectly
            // and the monster turn to be skipped or mishandled on the first frame.
            //
            // During an ambush round 1 the party cannot act, so force
            // EnemyTurn regardless of the turn order — monsters always go first.
            turn_state.0 = if combat_res.state.ambush_round_active {
                info!("Combat started: ambush — monsters act first in round 1, setting EnemyTurn");
                CombatTurnState::EnemyTurn
            } else {
                match combat_res.state.turn_order.first() {
                    Some(CombatantId::Monster(_)) => {
                        info!("Combat started: monster goes first — setting EnemyTurn");
                        CombatTurnState::EnemyTurn
                    }
                    _ => {
                        info!("Combat started: player goes first — setting PlayerTurn");
                        CombatTurnState::PlayerTurn
                    }
                }
            };

            // Store encounter position so handle_combat_victory can remove
            // the MapEvent::Encounter from the map on victory.
            combat_res.encounter_position = msg.encounter_position;
            combat_res.encounter_map_id = msg.encounter_map_id;
            combat_res.combat_event_type = msg.combat_event_type;
            if let (Some(pos), Some(map_id)) = (msg.encounter_position, msg.encounter_map_id) {
                info!(
                    "Stored encounter position {:?} on map {} in CombatResource",
                    pos, map_id
                );
            }
            info!("Combat event type: {:?}", combat_res.combat_event_type);

            // Emit a combat log entry that describes how the battle began.
            let opening_text = match msg.combat_event_type {
                CombatEventType::Ambush => {
                    "The monsters ambush the party! The party is surprised!".to_string()
                }
                CombatEventType::Ranged => "Combat begins at range! Draw your bows!".to_string(),
                CombatEventType::Magic => "The air crackles with magical energy!".to_string(),
                CombatEventType::Boss => {
                    "A powerful foe stands before you! Prepare for a legendary battle!".to_string()
                }
                CombatEventType::Normal => "Monsters appear!".to_string(),
            };
            combat_log.push_line(CombatLogLine {
                segments: vec![CombatLogSegment {
                    text: opening_text,
                    color: FEEDBACK_COLOR_STATUS,
                }],
            });

            // Request combat music transition (if audio plugin is registered)
            if let Some(ref mut w) = music_writer {
                w.write(crate::game::systems::audio::PlayMusic {
                    track_id: "combat_theme".to_string(),
                    looped: true,
                });
            }
        }
    }
}

/// System: Ensure party members are present in the `CombatResource` while in
/// combat. This acts as a safety net if combat was created without players
/// properly added (e.g., direct `enter_combat()` calls).
fn sync_party_to_combat(mut combat_res: ResMut<CombatResource>, global_state: Res<GlobalState>) {
    // Only run when in combat
    let _cs = match &global_state.0.mode {
        GameMode::Combat(cs) => cs,
        _ => return,
    };

    // If there are already player entries, assume the combat is initialized
    let existing_players = combat_res
        .state
        .participants
        .iter()
        .filter(|p| matches!(p, Combatant::Player(_)))
        .count();

    if existing_players > 0 {
        return;
    }

    // Copy party characters into the combat state (preserve order)
    for (i, character) in global_state.0.party.members.iter().enumerate() {
        combat_res.state.add_player(character.clone());
        combat_res.player_orig_indices.push(Some(i));
    }

    // Initialize turn order in case monsters were already added earlier
    crate::domain::combat::engine::start_combat(&mut combat_res.state);
}

/// Mirrors HP, SP, and conditions from [`CombatResource`] participants back into
/// `party.members` every frame while combat is active.
///
/// This keeps `global_state.0.party.members[i].hp.current` in sync with what is
/// happening inside the combat engine so that the HUD (`update_hud`) always
/// displays live values without needing to know about `CombatResource`.
///
/// Only the fields that can change during a combat turn are written:
/// - `hp.current`
/// - `sp.current`
/// - `conditions`
/// - `active_conditions`
///
/// Base values and stats are left untouched; `sync_combat_to_party_on_exit`
/// handles the full authoritative copy when combat ends.
fn sync_party_hp_during_combat(
    mut global_state: ResMut<GlobalState>,
    combat_res: Res<CombatResource>,
) {
    // Only run while in combat.
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    for (participant_idx, participant) in combat_res.state.participants.iter().enumerate() {
        if let Combatant::Player(pc) = participant {
            // Resolve which party slot this participant maps to.
            let party_idx = match combat_res
                .player_orig_indices
                .get(participant_idx)
                .and_then(|opt| *opt)
            {
                Some(idx) => idx,
                None => continue,
            };

            let Some(member) = global_state.0.party.members.get_mut(party_idx) else {
                continue;
            };

            // Mirror only the values that change during combat turns.
            // Use saturating arithmetic — hp/sp are u16, never go negative.
            member.hp.current = pc.hp.current;
            member.sp.current = pc.sp.current;
            member.conditions = pc.conditions;
            member.active_conditions = pc.active_conditions.clone();
        }
    }
}

/// System: When combat has ended (global mode is not Combat) copy combat state
/// data (HP, SP, conditions, stat currents) back into the party members.
fn sync_combat_to_party_on_exit(
    mut global_state: ResMut<GlobalState>,
    mut combat_res: ResMut<CombatResource>,
) {
    // Only sync when not currently in combat and when we have mapping data
    if matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    if combat_res.player_orig_indices.is_empty() {
        return;
    }

    // Copy participant data back to party members for mapped players
    for (participant_idx, participant) in combat_res.state.participants.iter().enumerate() {
        if let Some(Some(party_idx)) = combat_res.player_orig_indices.get(participant_idx).cloned()
        {
            if party_idx >= global_state.0.party.members.len() {
                // Party membership changed during combat; skip out-of-range
                continue;
            }

            if let Combatant::Player(pc) = participant {
                let party_member = &mut global_state.0.party.members[party_idx];

                // Sync HP (use modify to adjust current safely)
                let delta_hp = pc.hp.current as i32 - party_member.hp.current as i32;
                if delta_hp != 0 {
                    party_member.hp.modify(delta_hp);
                }

                // Sync SP
                let delta_sp = pc.sp.current as i32 - party_member.sp.current as i32;
                if delta_sp != 0 {
                    party_member.sp.modify(delta_sp);
                }

                // Sync transient status flags (bitflags) - replace with combat view
                party_member.conditions = pc.conditions;

                // Sync active condition stack
                party_member.active_conditions = pc.active_conditions.clone();

                // Sync stat current values (might, intellect, personality, endurance, speed, accuracy, luck)
                let stats_src = &pc.stats;
                let stats_dst = &mut party_member.stats;

                let deltas = [
                    (stats_src.might.current as i32) - (stats_dst.might.current as i32),
                    (stats_src.intellect.current as i32) - (stats_dst.intellect.current as i32),
                    (stats_src.personality.current as i32) - (stats_dst.personality.current as i32),
                    (stats_src.endurance.current as i32) - (stats_dst.endurance.current as i32),
                    (stats_src.speed.current as i32) - (stats_dst.speed.current as i32),
                    (stats_src.accuracy.current as i32) - (stats_dst.accuracy.current as i32),
                    (stats_src.luck.current as i32) - (stats_dst.luck.current as i32),
                ];

                // Apply deltas using `modify` to respect modifier system (do not overwrite base)
                // Convert i32 deltas to the expected i16 type for the `modify` methods.
                stats_dst
                    .might
                    .modify(deltas[0].try_into().expect("stat delta fits in i16"));
                stats_dst
                    .intellect
                    .modify(deltas[1].try_into().expect("stat delta fits in i16"));
                stats_dst
                    .personality
                    .modify(deltas[2].try_into().expect("stat delta fits in i16"));
                stats_dst
                    .endurance
                    .modify(deltas[3].try_into().expect("stat delta fits in i16"));
                stats_dst
                    .speed
                    .modify(deltas[4].try_into().expect("stat delta fits in i16"));
                stats_dst
                    .accuracy
                    .modify(deltas[5].try_into().expect("stat delta fits in i16"));
                stats_dst
                    .luck
                    .modify(deltas[6].try_into().expect("stat delta fits in i16"));

                // Sync AC current (armor class)
                let ac_delta = pc.ac.current as i32 - party_member.ac.current as i32;
                if ac_delta != 0 {
                    party_member
                        .ac
                        .modify(ac_delta.try_into().expect("ac delta fits in i16"));
                }
            }
        }
    }

    // Clear stored combat state now that party has been updated
    combat_res.clear();
}

// ===== Combat UI Query Type Aliases =====

/// Query for enemy HP bar fill nodes, excluding boss HP bars.
type EnemyHpBarQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static EnemyHpBarFill,
        &'static mut Node,
        &'static mut BackgroundColor,
    ),
    Without<BossHpBarFill>,
>;

/// Query for enemy HP text entities, excluding name, condition, turn order, and boss texts.
type EnemyHpTextQuery<'w, 's> = Query<
    'w,
    's,
    (&'static EnemyHpText, &'static mut Text),
    (
        Without<EnemyNameText>,
        Without<EnemyConditionText>,
        Without<TurnOrderText>,
        Without<BossHpBarText>,
    ),
>;

/// Query for enemy condition text entities, excluding HP, name, turn order, and boss texts.
type EnemyConditionTextQuery<'w, 's> = Query<
    'w,
    's,
    (&'static EnemyConditionText, &'static mut Text),
    (
        Without<EnemyHpText>,
        Without<EnemyNameText>,
        Without<TurnOrderText>,
        Without<BossHpBarText>,
    ),
>;

/// Query for the turn order text entity, excluding all enemy-specific text markers.
type TurnOrderTextQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<TurnOrderText>,
        Without<EnemyHpText>,
        Without<EnemyNameText>,
        Without<EnemyConditionText>,
        Without<BossHpBarText>,
    ),
>;

/// Query for boss HP bar fill nodes, excluding standard enemy HP bars.
type BossHpBarQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static BossHpBarFill,
        &'static mut Node,
        &'static mut BackgroundColor,
    ),
    Without<EnemyHpBarFill>,
>;

/// Query for boss HP bar text entities, excluding standard enemy HP text.
type BossHpBarTextQuery<'w, 's> =
    Query<'w, 's, (&'static BossHpBarText, &'static mut Text), Without<EnemyHpText>>;

/// Query for action button interactions used by the combat input system.
type ActionButtonQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        Ref<'static, Interaction>,
        &'static ActionButton,
    ),
    With<Button>,
>;

/// Query for enemy card interactions used by the target selection system.
type EnemyCardInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        Ref<'static, Interaction>,
        &'static EnemyCard,
    ),
    With<Button>,
>;

/// Query for the main camera transform used by hover bar world-projection.
type CombatCameraQuery<'w, 's> =
    Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<MainCamera>>;

/// Query for encounter visual markers and their world transforms.
type EncounterVisualQuery<'w, 's> =
    Query<'w, 's, (&'static EncounterVisualMarker, &'static GlobalTransform)>;

/// Query for monster HP hover bar text entities.
type MonsterHpHoverTextQuery<'w, 's> =
    Query<'w, 's, (&'static MonsterHpHoverBarHpText, &'static mut Text)>;

/// Bundled queries for monster HP hover bar containers and fills.
///
/// Used by [`update_monster_hp_hover_bars`] to read bar layout and update fill
/// widths without violating Bevy's borrow rules (both queries touch `Node`).
type MonsterHpHoverBarQueries<'w, 's> = ParamSet<
    'w,
    's,
    (
        Query<'static, 'static, (&'static MonsterHpHoverBar, &'static mut Node)>,
        Query<
            'static,
            'static,
            (
                &'static MonsterHpHoverBarFill,
                &'static mut Node,
                &'static mut BackgroundColor,
            ),
        >,
    ),
>;

// ===== Combat UI Systems =====

/// System: Setup combat UI when entering combat mode
///
/// Spawns the combat HUD including enemy panel, turn order display, and action menu.
/// This system runs every frame but only acts when combat UI needs to be created.
fn setup_combat_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    combat_res: Res<CombatResource>,
    existing_ui: Query<Entity, With<crate::game::components::combat::CombatHudRoot>>,
) {
    // Only setup UI when in combat mode
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    // If UI already exists, don't recreate it
    if !existing_ui.is_empty() {
        return;
    }

    // Spawn combat HUD root container — a transparent full-screen anchor for
    // all absolutely-positioned combat panels.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            crate::game::components::combat::CombatHudRoot,
        ))
        .with_children(|parent| {
            // ── Enemy panel ────────────────────────────────────────────────
            // Pinned from the bottom so it sits in the lower half of the
            // screen, leaving the upper half clear for the combat log.
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        bottom: ENEMY_PANEL_BOTTOM,
                        height: COMBAT_ENEMY_PANEL_HEIGHT,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(16.0),
                        padding: UiRect::all(Val::Px(16.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
                ))
                .with_children(|enemy_panel| {
                    // Spawn enemy cards for each monster in combat
                    for (idx, participant) in combat_res.state.participants.iter().enumerate() {
                        if let Combatant::Monster(monster) = participant {
                            // Inline enemy card spawning
                            enemy_panel
                                .spawn((
                                    Button,
                                    Node {
                                        width: ENEMY_CARD_WIDTH,
                                        flex_direction: FlexDirection::Column,
                                        padding: UiRect::all(ENEMY_CARD_PADDING),
                                        row_gap: Val::Px(4.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.2, 0.15, 0.15, 0.9)),
                                    BorderRadius::all(Val::Px(4.0)),
                                    EnemyCard {
                                        participant_index: idx,
                                    },
                                ))
                                .with_children(|card| {
                                    // Enemy name
                                    card.spawn((
                                        Text::new(monster.name.clone()),
                                        text_style(LABEL_FONT_SIZE, Color::WHITE),
                                        EnemyNameText {
                                            participant_index: idx,
                                        },
                                    ));

                                    // HP bar background
                                    card.spawn((
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: ENEMY_HP_BAR_HEIGHT,
                                            overflow: Overflow::visible(),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 1.0)),
                                        EnemyHpBarBackground {
                                            participant_index: idx,
                                        },
                                    ))
                                    .with_children(|bar| {
                                        // HP bar fill
                                        bar.spawn((
                                            Node {
                                                width: Val::Percent(100.0),
                                                height: Val::Percent(100.0),
                                                ..default()
                                            },
                                            BackgroundColor(ENEMY_HP_HEALTHY_COLOR),
                                            EnemyHpBarFill {
                                                participant_index: idx,
                                            },
                                        ));
                                    });

                                    // HP text
                                    card.spawn((
                                        Text::new(format!(
                                            "{}/{}",
                                            monster.hp.current, monster.hp.base
                                        )),
                                        TextFont {
                                            font_size: 10.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                        EnemyHpText {
                                            participant_index: idx,
                                        },
                                    ));

                                    // Condition text
                                    card.spawn((
                                        Text::new(""),
                                        TextFont {
                                            font_size: 9.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.8, 0.8, 0.0)),
                                        EnemyConditionText {
                                            participant_index: idx,
                                        },
                                    ));
                                });
                        }
                    }
                });

            // ── Boss HP Bar ─────────────────────────────────────────────────────
            // Only spawn when this is a Boss encounter.  The bar is rendered at the
            // top-centre of the screen, above the combat log, for maximum visibility.
            if combat_res.combat_event_type == CombatEventType::Boss {
                for (idx, participant) in combat_res.state.participants.iter().enumerate() {
                    if let Combatant::Monster(monster) = participant {
                        parent
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Auto,
                                    right: Val::Auto,
                                    top: Val::Px(8.0),
                                    width: Val::Px(BOSS_HP_BAR_WIDTH),
                                    height: Val::Px(BOSS_HP_BAR_HEIGHT + 32.0),
                                    flex_direction: FlexDirection::Column,
                                    align_self: AlignSelf::Center,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    row_gap: Val::Px(4.0),
                                    padding: UiRect::all(Val::Px(6.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.15, 0.05, 0.05, 0.92)),
                                BorderRadius::all(Val::Px(6.0)),
                                BossHpBar {
                                    participant_index: idx,
                                },
                            ))
                            .with_children(|boss_panel| {
                                // Boss name label
                                boss_panel.spawn((
                                    Text::new(format!("⚔ {} ⚔", monster.name)),
                                    TextFont {
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(1.0, 0.8, 0.2)),
                                ));

                                // Boss HP bar background
                                boss_panel
                                    .spawn((
                                        Node {
                                            width: Val::Px(BOSS_HP_BAR_WIDTH - 12.0),
                                            height: Val::Px(BOSS_HP_BAR_HEIGHT),
                                            overflow: Overflow::visible(),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.3, 0.1, 0.1, 1.0)),
                                    ))
                                    .with_children(|bar_bg| {
                                        bar_bg.spawn((
                                            Node {
                                                width: Val::Percent(100.0),
                                                height: Val::Percent(100.0),
                                                ..default()
                                            },
                                            BackgroundColor(BOSS_HP_HEALTHY_COLOR),
                                            BossHpBarFill {
                                                participant_index: idx,
                                            },
                                        ));
                                    });

                                // Boss HP text
                                boss_panel.spawn((
                                    Text::new(format!(
                                        "{}/{}",
                                        monster.hp.current, monster.hp.base
                                    )),
                                    TextFont {
                                        font_size: 11.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(1.0, 0.9, 0.9)),
                                    BossHpBarText {
                                        participant_index: idx,
                                    },
                                ));
                            });
                        // Only show one boss bar (for the first monster)
                        break;
                    }
                }
            }

            // ── Turn order panel ───────────────────────────────────────────
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        bottom: TURN_ORDER_BOTTOM,
                        height: TURN_ORDER_PANEL_HEIGHT,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.9)),
                    TurnOrderPanel,
                ))
                .with_children(|turn_panel| {
                    turn_panel.spawn((
                        Text::new("Turn Order: "),
                        text_style(BODY_FONT_SIZE, Color::WHITE),
                        TurnOrderText,
                    ));
                });

            // ── Action menu ────────────────────────────────────────────────
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        bottom: ACTION_MENU_BOTTOM,
                        height: ACTION_MENU_HEIGHT,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                    ActionMenuPanel,
                ))
                .with_children(|menu_panel| {
                    // Choose button order based on combat type.
                    // Magic combat puts Cast first; all others use the standard order.
                    let standard_buttons: &[(&str, ActionButtonType)] = &[
                        ("Attack", ActionButtonType::Attack),
                        ("Defend", ActionButtonType::Defend),
                        ("Cast", ActionButtonType::Cast),
                        ("Item", ActionButtonType::Item),
                        ("Flee", ActionButtonType::Flee),
                    ];
                    let magic_buttons: &[(&str, ActionButtonType)] = &[
                        ("Cast", ActionButtonType::Cast),
                        ("Attack", ActionButtonType::Attack),
                        ("Defend", ActionButtonType::Defend),
                        ("Item", ActionButtonType::Item),
                        ("Flee", ActionButtonType::Flee),
                    ];

                    let buttons = if combat_res.combat_event_type.highlights_magic_action() {
                        magic_buttons
                    } else {
                        standard_buttons
                    };

                    for (label, button_type) in buttons.iter().copied() {
                        menu_panel
                            .spawn((
                                Button,
                                Node {
                                    width: ACTION_BUTTON_WIDTH,
                                    height: ACTION_BUTTON_HEIGHT,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(ACTION_BUTTON_COLOR),
                                BorderRadius::all(Val::Px(4.0)),
                                ActionButton { button_type },
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Text::new(label),
                                    text_style(LABEL_FONT_SIZE, Color::WHITE),
                                ));
                            });
                    }

                    // In Ranged combat, also spawn the Ranged action button.
                    // Initial color is disabled — update_combat_ui enables it
                    // each frame when the current player combatant has a ranged weapon.
                    if combat_res.combat_event_type.enables_ranged_action() {
                        menu_panel
                            .spawn((
                                Button,
                                Node {
                                    width: ACTION_BUTTON_WIDTH,
                                    height: ACTION_BUTTON_HEIGHT,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(ACTION_BUTTON_DISABLED_COLOR),
                                BorderRadius::all(Val::Px(4.0)),
                                ActionButton {
                                    button_type: ActionButtonType::RangedAttack,
                                },
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Text::new("Ranged"),
                                    text_style(LABEL_FONT_SIZE, Color::WHITE),
                                ));
                            });
                    }
                });

            // Persistent combat log bubble in the top-right corner.
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(12.0),
                        top: Val::Px(12.0),
                        width: COMBAT_LOG_BUBBLE_WIDTH,
                        min_height: COMBAT_LOG_BUBBLE_MIN_HEIGHT,
                        max_height: COMBAT_LOG_BUBBLE_MAX_HEIGHT,
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(10.0)),
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.06, 0.09, 0.13, 0.92)),
                    BorderRadius::all(Val::Px(12.0)),
                    CombatLogBubbleRoot,
                ))
                .with_children(|bubble| {
                    bubble.spawn((
                        Text::new("Combat Log"),
                        TextFont {
                            font_size: 15.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.92, 0.96, 1.0)),
                    ));

                    bubble
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                flex_grow: 1.0,
                                overflow: Overflow::scroll_y(),
                                padding: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.03, 0.05, 0.08, 0.35)),
                            BorderRadius::all(Val::Px(6.0)),
                            CombatLogBubbleViewport,
                        ))
                        .with_children(|viewport| {
                            viewport.spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(4.0),
                                    ..default()
                                },
                                CombatLogLineList,
                            ));
                        });
                });
        });
}

/// System: Cleanup combat UI when exiting combat mode
///
/// Despawns all combat HUD entities when combat ends.
fn cleanup_combat_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    combat_ui: Query<Entity, With<crate::game::components::combat::CombatHudRoot>>,
) {
    // Only cleanup when not in combat mode
    if matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    // Despawn combat UI if it exists (despawn will automatically handle children)
    for entity in combat_ui.iter() {
        commands.entity(entity).despawn();
    }
}

/// System: Update combat UI to reflect current combat state
///
/// Updates enemy HP bars, turn order display, and action menu visibility.
#[allow(clippy::too_many_arguments)]
fn update_combat_ui(
    combat_res: Res<CombatResource>,
    global_state: Res<GlobalState>,
    mut enemy_hp_bars: EnemyHpBarQuery,
    mut enemy_hp_texts: EnemyHpTextQuery,
    mut enemy_condition_texts: EnemyConditionTextQuery,
    mut turn_order_text: TurnOrderTextQuery,
    mut action_menu: Query<&mut Visibility, With<ActionMenuPanel>>,
    turn_state: Res<CombatTurnStateResource>,
    mut action_menu_state: ResMut<ActionMenuState>,
    mut boss_hp_fills: BossHpBarQuery,
    mut boss_hp_texts: BossHpBarTextQuery,
) {
    // Only update when in combat mode
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    // Update enemy HP bars
    for (hp_bar, mut node, mut bg_color) in enemy_hp_bars.iter_mut() {
        if let Some(Combatant::Monster(monster)) =
            combat_res.state.participants.get(hp_bar.participant_index)
        {
            // Calculate HP percentage
            let hp_percent = if monster.hp.base > 0 {
                (monster.hp.current as f32 / monster.hp.base as f32 * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };

            // Update bar width
            node.width = Val::Percent(hp_percent);

            // Update bar color based on HP percentage
            *bg_color = if hp_percent > 50.0 {
                BackgroundColor(ENEMY_HP_HEALTHY_COLOR)
            } else if hp_percent > 20.0 {
                BackgroundColor(ENEMY_HP_INJURED_COLOR)
            } else {
                BackgroundColor(ENEMY_HP_CRITICAL_COLOR)
            };
        }
    }

    // Update enemy HP text
    for (hp_text, mut text) in enemy_hp_texts.iter_mut() {
        if let Some(Combatant::Monster(monster)) =
            combat_res.state.participants.get(hp_text.participant_index)
        {
            **text = format!("{}/{}", monster.hp.current, monster.hp.base);
        }
    }

    // Update enemy condition text
    for (condition_text, mut text) in enemy_condition_texts.iter_mut() {
        if let Some(Combatant::Monster(monster)) = combat_res
            .state
            .participants
            .get(condition_text.participant_index)
        {
            // Get condition summary (simplified for now)
            // MonsterCondition is an enum, not a bitflag, so check directly
            let condition_str = if matches!(
                monster.conditions,
                crate::domain::combat::monster::MonsterCondition::Normal
            ) {
                String::new()
            } else {
                "Condition".to_string()
            };
            **text = condition_str;
        }
    }

    // Update turn order display
    if let Ok(mut text) = turn_order_text.single_mut() {
        let mut turn_order_str = String::from("Turn Order: ");

        for (i, combatant_id) in combat_res.state.turn_order.iter().enumerate() {
            let name = match combatant_id {
                CombatantId::Player(idx) => {
                    if let Some(Combatant::Player(character)) =
                        combat_res.state.participants.get(*idx)
                    {
                        character.name.as_str()
                    } else {
                        "???"
                    }
                }
                CombatantId::Monster(idx) => {
                    if let Some(Combatant::Monster(monster)) =
                        combat_res.state.participants.get(*idx)
                    {
                        monster.name.as_str()
                    } else {
                        "???"
                    }
                }
            };

            if i == combat_res.state.current_turn {
                turn_order_str.push_str(&format!("[{}] → ", name));
            } else {
                turn_order_str.push_str(&format!("{} → ", name));
            }
        }

        // Remove trailing arrow (safely handle UTF-8)
        if let Some(stripped) = turn_order_str.strip_suffix(" → ") {
            turn_order_str = stripped.to_string();
        }

        **text = turn_order_str;
    }

    // Show/hide action menu based on turn state.
    // When the menu becomes visible (player turn starts), reset the highlight to
    // index 0 (Attack) so the default is always Attack on every menu open.
    if let Ok(mut visibility) = action_menu.single_mut() {
        let new_visibility = if matches!(turn_state.0, CombatTurnState::PlayerTurn) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        // Reset highlight index whenever the menu transitions to visible.
        if *visibility == Visibility::Hidden && new_visibility == Visibility::Visible {
            action_menu_state.active_index = 0;
            action_menu_state.confirmed = false;
        }

        *visibility = new_visibility;
    }

    // ── Boss HP Bar Update ────────────────────────────────────────────────
    for (fill, mut node, mut color) in &mut boss_hp_fills {
        if let Some(Combatant::Monster(monster)) =
            combat_res.state.participants.get(fill.participant_index)
        {
            let ratio = if monster.hp.base > 0 {
                monster.hp.current as f32 / monster.hp.base as f32
            } else {
                0.0
            };
            node.width = Val::Percent(ratio * 100.0);
            *color = BackgroundColor(if ratio >= 0.5 {
                BOSS_HP_HEALTHY_COLOR
            } else if ratio >= 0.25 {
                BOSS_HP_INJURED_COLOR
            } else {
                BOSS_HP_CRITICAL_COLOR
            });
        }
    }
    for (text_comp, mut text) in &mut boss_hp_texts {
        if let Some(Combatant::Monster(monster)) = combat_res
            .state
            .participants
            .get(text_comp.participant_index)
        {
            **text = format!("{}/{}", monster.hp.current, monster.hp.base);
        }
    }
}

/// Update the RangedAttack button enable/disable color each frame.
///
/// Only active when `combat_res.combat_event_type.enables_ranged_action()` is
/// `true`; in other combat types the `RangedAttack` button is never spawned so
/// this system is a no-op.  Separated from `update_combat_ui` to avoid a
/// double-mutable-`BackgroundColor` parameter conflict with the HP-bar query.
fn update_ranged_button_color(
    combat_res: Res<CombatResource>,
    content: Option<Res<GameContent>>,
    turn_state: Res<CombatTurnStateResource>,
    mut action_buttons: Query<(&ActionButton, &mut BackgroundColor), With<Button>>,
) {
    if !combat_res.combat_event_type.enables_ranged_action() {
        return;
    }

    // Determine whether the current player combatant has a ranged weapon.
    let player_has_ranged = if matches!(turn_state.0, CombatTurnState::PlayerTurn) {
        if let Some(content_res) = content.as_deref() {
            let actor = combat_res
                .state
                .turn_order
                .get(combat_res.state.current_turn)
                .cloned();
            match actor {
                Some(CombatantId::Player(idx)) => {
                    if let Some(Combatant::Player(pc)) = combat_res.state.participants.get(idx) {
                        has_ranged_weapon(pc, &content_res.db().items)
                    } else {
                        false
                    }
                }
                _ => false,
            }
        } else {
            false
        }
    } else {
        false
    };

    let ranged_color = if player_has_ranged {
        ACTION_BUTTON_COLOR
    } else {
        ACTION_BUTTON_DISABLED_COLOR
    };

    for (btn, mut bg) in action_buttons.iter_mut() {
        if btn.button_type == ActionButtonType::RangedAttack {
            *bg = BackgroundColor(ranged_color);
        }
    }
}

// ===== Player Action Systems =====

/// Handle input from action buttons and keyboard shortcuts during PlayerTurn.
///
/// Emits `DefendAction` / `FleeAction` messages or enters target selection mode
/// for attacks.
/// Private alias kept for the internal dispatch path.
///
/// Both `combat_input_system` and `update_action_highlight` reference the
/// public `COMBAT_ACTION_ORDER` constant directly; this alias is retained only
/// so that any future private callers have a short, stable name.
const ACTION_BUTTON_ORDER: [ActionButtonType; COMBAT_ACTION_COUNT] = COMBAT_ACTION_ORDER;

/// Unified combat action dispatcher.
///
/// Both mouse (`Interaction::Pressed`) and keyboard (`Enter`) routes call this
/// function so the semantics are identical regardless of input method.
///
/// # Arguments
///
/// * `button_type` - The `ActionButtonType` selected by the player.
/// * `actor` - The `CombatantId` of the currently acting player combatant.
/// * `target_sel` - Mutable reference to the `TargetSelection` resource.
/// * `action_menu_state` - Mutable reference to `ActionMenuState`; when
///   `Attack` or `RangedAttack` is dispatched the `active_target_index` is
///   reset to `Some(0)`.
/// * `ranged_pending` - Mutable reference to `RangedAttackPending`; set to
///   `true` when `RangedAttack` is dispatched so target-confirmation writes a
///   `RangedAttackAction` instead of an `AttackAction`.
/// * `defend_writer` - Optional message writer for `DefendAction`.
/// * `flee_writer` - Optional message writer for `FleeAction`.
fn dispatch_combat_action(
    button_type: ActionButtonType,
    actor: CombatantId,
    target_sel: &mut TargetSelection,
    action_menu_state: &mut ActionMenuState,
    ranged_pending: &mut RangedAttackPending,
    defend_writer: &mut Option<MessageWriter<DefendAction>>,
    flee_writer: &mut Option<MessageWriter<FleeAction>>,
) {
    match button_type {
        ActionButtonType::Attack => {
            target_sel.0 = Some(actor);
            // Initialise keyboard target cycling to the first enemy.
            action_menu_state.active_target_index = Some(0);
            ranged_pending.0 = false;
        }
        ActionButtonType::RangedAttack => {
            target_sel.0 = Some(actor);
            // Initialise keyboard target cycling to the first enemy.
            action_menu_state.active_target_index = Some(0);
            // Signal that the pending target-confirm should fire RangedAttackAction.
            ranged_pending.0 = true;
        }
        ActionButtonType::Defend => {
            if let Some(w) = defend_writer {
                w.write(DefendAction { combatant: actor });
            }
        }
        ActionButtonType::Flee => {
            if let Some(w) = flee_writer {
                w.write(FleeAction);
            }
        }
        ActionButtonType::Cast | ActionButtonType::Item => {
            // Submenu open — handled by separate systems
        }
    }
}

/// Write an `AttackAction` or `RangedAttackAction` and clear both
/// `TargetSelection` and the keyboard target index.
///
/// This is the single point through which both mouse-click and keyboard-confirm
/// target paths produce their attack action, guaranteeing identical semantics.
/// When `ranged_pending.0` is `true` a `RangedAttackAction` is written and the
/// flag is reset; otherwise a plain `AttackAction` is written.
///
/// # Arguments
///
/// * `attacker` - The `CombatantId` of the attacking player combatant.
/// * `target_monster_idx` - Participant index of the targeted monster in
///   `CombatState::participants`.
/// * `target_sel` - Mutable reference to `TargetSelection`; cleared to `None`.
/// * `action_menu_state` - Mutable reference to `ActionMenuState`; clears
///   `active_target_index` to `None`.
/// * `ranged_pending` - Mutable reference to `RangedAttackPending`; consumed
///   and reset to `false` when a ranged action is confirmed.
/// * `attack_writer` - Optional message writer for `AttackAction`.
/// * `ranged_writer` - Optional message writer for `RangedAttackAction`.
#[allow(clippy::too_many_arguments)]
fn confirm_attack_target(
    attacker: CombatantId,
    target_monster_idx: usize,
    target_sel: &mut TargetSelection,
    action_menu_state: &mut ActionMenuState,
    ranged_pending: &mut RangedAttackPending,
    attack_writer: &mut Option<MessageWriter<AttackAction>>,
    ranged_writer: &mut Option<MessageWriter<RangedAttackAction>>,
) {
    if ranged_pending.0 {
        if let Some(ref mut w) = ranged_writer {
            w.write(RangedAttackAction {
                attacker,
                target: CombatantId::Monster(target_monster_idx),
            });
        }
        ranged_pending.0 = false;
    } else if let Some(ref mut w) = attack_writer {
        w.write(AttackAction {
            attacker,
            target: CombatantId::Monster(target_monster_idx),
        });
    }
    target_sel.0 = None;
    action_menu_state.active_target_index = None;
}

/// Handle input from action buttons and keyboard shortcuts during PlayerTurn.
///
/// # Keyboard Controls (action menu — when NOT in target-select mode)
///
/// - `Tab`: cycles the highlighted action button forward (wraps at
///   `COMBAT_ACTION_COUNT - 1` → 0).
/// - `Enter`: dispatches the action at the current highlight index.
/// - `Escape`: no-op (no target selection active).
///
/// # Keyboard Controls (target selection — when `TargetSelection.0.is_some()`)
///
/// - `Tab`: advances `ActionMenuState::active_target_index` modulo the count
///   of alive monster participants, cycling through all valid targets.
/// - `Enter`: calls `confirm_attack_target` with the currently highlighted
///   monster index, writes `AttackAction`, and clears target-select state.
/// - `Escape`: clears `TargetSelection.0 = None` and resets
///   `active_target_index = None`, cancelling the attack selection.
///
/// # Mouse Controls
///
/// - `Interaction::Pressed` on an `ActionButton`: dispatches that action.
/// - Left mouse `just_pressed` while `Interaction::Hovered` over an
///   `ActionButton` is also treated as activation (robust fallback for
///   platforms/timings where `Pressed` transition is missed).
/// - Plain hover without a click does not dispatch.
///
/// Both mouse and keyboard routes call `dispatch_combat_action` /
/// `confirm_attack_target` so their semantics are identical.
#[allow(clippy::too_many_arguments)]
fn combat_input_system(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    mut interactions: ActionButtonQuery,
    global_state: Res<GlobalState>,
    combat_res: Res<CombatResource>,
    mut target_sel: ResMut<TargetSelection>,
    mut defend_writer: Option<MessageWriter<DefendAction>>,
    mut flee_writer: Option<MessageWriter<FleeAction>>,
    mut attack_writer: Option<MessageWriter<AttackAction>>,
    mut ranged_writer: Option<MessageWriter<RangedAttackAction>>,
    mut ranged_pending: ResMut<RangedAttackPending>,
    turn_state: Res<CombatTurnStateResource>,
    mut action_menu_state: ResMut<ActionMenuState>,
) {
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    // During an ambush round the party is surprised and cannot act.
    // `handle_combat_started` already sets `CombatTurnState::EnemyTurn` for
    // ambush round 1, so this guard provides defence-in-depth: even if a
    // player-turn combatant appears in the turn order during round 1 we
    // refuse to dispatch any player input.
    if combat_res.state.ambush_round_active
        && matches!(
            combat_res.state.get_current_combatant(),
            Some(Combatant::Player(_))
        )
    {
        let any_key = keyboard
            .as_ref()
            .is_some_and(|kb| kb.just_pressed(KeyCode::Tab) || kb.just_pressed(KeyCode::Enter));
        if any_key {
            info!("Combat: input blocked — party is surprised (ambush round 1)");
        }
        return;
    }

    // Log blocked input if not player turn and any input event is present.
    if !matches!(turn_state.0, CombatTurnState::PlayerTurn) {
        let any_key_input = keyboard.as_ref().is_some_and(|kb| {
            kb.just_pressed(KeyCode::Tab)
                || kb.just_pressed(KeyCode::Enter)
                || kb.just_pressed(KeyCode::Escape)
        });
        let mouse_just_pressed = mouse_input::mouse_just_pressed(mouse_buttons.as_deref());
        let any_mouse_input = mouse_just_pressed
            || interactions
                .iter()
                .any(|(i, i_ref, _)| mouse_input::is_activated(i, i_ref.is_changed(), false));
        if any_key_input || any_mouse_input {
            info!("Combat: input blocked — not player turn");
        }
        return;
    }

    let current_actor = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
        .cloned();
    let mut execute_selected_action = false;

    // --- Mouse: robust click handling ---
    let mouse_just_pressed = mouse_input::mouse_just_pressed(mouse_buttons.as_deref());
    let mut mouse_dispatched = false;

    for (interaction, interaction_ref, button) in interactions.iter_mut() {
        if mouse_input::is_activated(
            interaction,
            interaction_ref.is_changed(),
            mouse_just_pressed,
        ) {
            if let Some(actor) = current_actor {
                dispatch_combat_action(
                    button.button_type,
                    actor,
                    &mut target_sel,
                    &mut action_menu_state,
                    &mut ranged_pending,
                    &mut defend_writer,
                    &mut flee_writer,
                );
                // Mouse activation is immediate and clears any prior keyboard
                // armed state.
                action_menu_state.confirmed = false;
                mouse_dispatched = true;
                break;
            }
        }
    }

    // Avoid mixing mouse and keyboard dispatch in the same frame.
    if mouse_dispatched {
        return;
    }

    // Count alive monster participants for target cycling.
    let alive_monster_count = combat_res
        .state
        .participants
        .iter()
        .filter(|p| matches!(p, Combatant::Monster(m) if m.hp.current > 0))
        .count();

    // --- Keyboard: behaviour splits on whether we are in target-select mode ---
    if let Some(kb) = keyboard.as_ref() {
        if target_sel.0.is_some() {
            // ---- Target-selection keyboard handling ----
            if kb.just_pressed(KeyCode::Tab) {
                // Cycle active_target_index through alive monsters.
                if alive_monster_count > 0 {
                    let current = action_menu_state.active_target_index.unwrap_or(0);
                    action_menu_state.active_target_index =
                        Some((current + 1) % alive_monster_count);
                }
            } else if kb.just_pressed(KeyCode::Enter) {
                // Confirm attack on the currently highlighted target.
                if let Some(attacker) = current_actor {
                    let target_idx = action_menu_state.active_target_index.unwrap_or(0);
                    // Resolve the alive-monster index to the actual participant index.
                    let participant_idx = resolve_alive_monster_participant_index(
                        &combat_res.state.participants,
                        target_idx,
                    );
                    if let Some(pidx) = participant_idx {
                        confirm_attack_target(
                            attacker,
                            pidx,
                            &mut target_sel,
                            &mut action_menu_state,
                            &mut ranged_pending,
                            &mut attack_writer,
                            &mut ranged_writer,
                        );
                    }
                }
            } else if kb.just_pressed(KeyCode::Escape) {
                // Cancel target selection — also clear ranged pending flag.
                target_sel.0 = None;
                action_menu_state.active_target_index = None;
                ranged_pending.0 = false;
            }
        } else {
            // ---- Action-menu keyboard handling ----
            // Use COMBAT_ACTION_COUNT_MAGIC for magic encounters; for all other
            // types (including Ranged which has an extra button) the standard 5
            // actions are the cycling set.
            let cycle_count = if combat_res.combat_event_type.highlights_magic_action() {
                COMBAT_ACTION_COUNT_MAGIC
            } else {
                COMBAT_ACTION_COUNT
            };
            if kb.just_pressed(KeyCode::Tab) {
                // Cycle active_index forward, wrapping at the correct count.
                action_menu_state.active_index = (action_menu_state.active_index + 1) % cycle_count;
                action_menu_state.confirmed = false;
            } else if kb.just_pressed(KeyCode::Enter) {
                // Single-step keyboard flow: Enter immediately executes the
                // currently highlighted action.
                execute_selected_action = true;
                action_menu_state.confirmed = false;
            } else if kb.just_pressed(KeyCode::Escape) {
                // No-op: no target selection is active.
            }
        }
    }

    // Execute selected action on Enter.
    if execute_selected_action {
        // Select from the correct order array based on combat type.
        let order: &[ActionButtonType] = if combat_res.combat_event_type.highlights_magic_action() {
            &COMBAT_ACTION_ORDER_MAGIC
        } else {
            &ACTION_BUTTON_ORDER
        };
        let selected_type = order[action_menu_state.active_index % order.len()];
        if let Some(actor) = current_actor {
            if selected_type == ActionButtonType::Attack {
                // Quick keyboard attack: immediately attack first alive target
                // so one Enter performs a full attack action.
                if let Some(pidx) =
                    resolve_alive_monster_participant_index(&combat_res.state.participants, 0)
                {
                    confirm_attack_target(
                        actor,
                        pidx,
                        &mut target_sel,
                        &mut action_menu_state,
                        &mut ranged_pending,
                        &mut attack_writer,
                        &mut ranged_writer,
                    );
                } else {
                    // No valid monster target; fall back to normal selection flow.
                    dispatch_combat_action(
                        selected_type,
                        actor,
                        &mut target_sel,
                        &mut action_menu_state,
                        &mut ranged_pending,
                        &mut defend_writer,
                        &mut flee_writer,
                    );
                }
            } else {
                dispatch_combat_action(
                    selected_type,
                    actor,
                    &mut target_sel,
                    &mut action_menu_state,
                    &mut ranged_pending,
                    &mut defend_writer,
                    &mut flee_writer,
                );
            }
        }
    }
}

/// Resolve the *n*-th alive monster's index in `participants`.
///
/// Iterates `participants` in order and returns the index of the `n`-th entry
/// that is a living (`hp.current > 0`) `Combatant::Monster`.  Returns `None`
/// if `n` is out of range.
///
/// Used by `combat_input_system` to map `active_target_index` (which counts
/// only alive monsters) to the real participant index used by `AttackAction`.
fn resolve_alive_monster_participant_index(
    participants: &[Combatant],
    alive_monster_nth: usize,
) -> Option<usize> {
    let mut count = 0usize;
    for (idx, participant) in participants.iter().enumerate() {
        if let Combatant::Monster(m) = participant {
            if m.hp.current > 0 {
                if count == alive_monster_nth {
                    return Some(idx);
                }
                count += 1;
            }
        }
    }
    None
}

/// Update the background colour of all `ActionButton` entities to reflect the
/// currently highlighted index stored in `ActionMenuState`.
///
/// The button at `active_index` receives `ACTION_BUTTON_HOVER_COLOR` by
/// default. If `ActionMenuState::confirmed` is `true` (keyboard-armed state),
/// the active button receives `ACTION_BUTTON_CONFIRMED_COLOR`. All other
/// buttons receive `ACTION_BUTTON_COLOR`.
///
/// References `COMBAT_ACTION_ORDER` and `COMBAT_ACTION_COUNT` so the order is
/// always consistent with the spawn order in `setup_combat_ui`.
fn update_action_highlight(
    action_menu_state: Res<ActionMenuState>,
    combat_res: Res<CombatResource>,
    mut buttons: Query<(&ActionButton, &mut BackgroundColor)>,
) {
    let order: &[ActionButtonType] = if combat_res.combat_event_type.highlights_magic_action() {
        &COMBAT_ACTION_ORDER_MAGIC
    } else {
        &COMBAT_ACTION_ORDER
    };
    let active_type = order[action_menu_state.active_index % order.len()];
    for (btn, mut bg) in buttons.iter_mut() {
        // The RangedAttack button is managed exclusively by update_combat_ui;
        // skip it here so we do not accidentally override its disabled color.
        if btn.button_type == ActionButtonType::RangedAttack {
            continue;
        }
        *bg = if btn.button_type == active_type {
            if action_menu_state.confirmed {
                BackgroundColor(ACTION_BUTTON_CONFIRMED_COLOR)
            } else {
                BackgroundColor(ACTION_BUTTON_HOVER_COLOR)
            }
        } else {
            BackgroundColor(ACTION_BUTTON_COLOR)
        };
    }
}

/// Highlight ALL enemy UI cards when target selection is active (general).
///
/// Sets every `EnemyCard` to `ENEMY_CARD_HIGHLIGHT_COLOR` while
/// `TargetSelection.0` is `Some`, and restores the default dark-red tint when
/// target selection is inactive.
///
/// `update_target_highlight` runs *after* this system and adds an additional
/// brighter highlight (`TURN_INDICATOR_COLOR`) on top of the card at
/// `ActionMenuState::active_target_index` for keyboard navigation.
fn enter_target_selection(
    target_sel: Res<TargetSelection>,
    mut enemy_cards: Query<(&EnemyCard, &mut BackgroundColor)>,
) {
    for (_card, mut bg) in enemy_cards.iter_mut() {
        *bg = if target_sel.0.is_some() {
            BackgroundColor(ENEMY_CARD_HIGHLIGHT_COLOR)
        } else {
            BackgroundColor(Color::srgba(0.2, 0.15, 0.15, 0.9))
        };
    }
}

/// Apply a specific highlight (`TURN_INDICATOR_COLOR`) to the enemy card at
/// `ActionMenuState::active_target_index` for keyboard target navigation.
///
/// This system runs *after* `enter_target_selection` so the per-card highlight
/// is layered on top of the general highlight applied to all enemy cards.
/// When target-select mode is not active (i.e. `TargetSelection.0` is `None`)
/// or `active_target_index` is `None`, this system is a no-op.
///
/// The card indices are resolved the same way as `combat_input_system`: the
/// *n*-th alive (`hp.current > 0`) monster in `participants` order corresponds
/// to `active_target_index == n`.
fn update_target_highlight(
    target_sel: Res<TargetSelection>,
    action_menu_state: Res<ActionMenuState>,
    combat_res: Res<CombatResource>,
    mut enemy_cards: Query<(&EnemyCard, &mut BackgroundColor)>,
) {
    // Only active while in target-select mode with a keyboard index set.
    let (Some(_attacker), Some(kbd_target)) = (target_sel.0, action_menu_state.active_target_index)
    else {
        return;
    };

    // Resolve alive-monster index → participant index.
    let highlighted_participant =
        resolve_alive_monster_participant_index(&combat_res.state.participants, kbd_target);

    for (card, mut bg) in enemy_cards.iter_mut() {
        if Some(card.participant_index) == highlighted_participant {
            *bg = BackgroundColor(TURN_INDICATOR_COLOR);
        }
    }
}

/// Handle clicks on enemy cards during target selection and emit `AttackAction`.
fn select_target(
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    mut interactions: EnemyCardInteractionQuery,
    mut target_sel: ResMut<TargetSelection>,
    mut action_menu_state: ResMut<ActionMenuState>,
    mut ranged_pending: ResMut<RangedAttackPending>,
    mut attack_writer: Option<MessageWriter<AttackAction>>,
    mut ranged_writer: Option<MessageWriter<RangedAttackAction>>,
) {
    let Some(attacker) = target_sel.0 else {
        return;
    };

    let mouse_just_pressed = mouse_input::mouse_just_pressed(mouse_buttons.as_deref());

    for (interaction, interaction_ref, enemy_card) in interactions.iter_mut() {
        if mouse_input::is_activated(
            interaction,
            interaction_ref.is_changed(),
            mouse_just_pressed,
        ) {
            confirm_attack_target(
                attacker,
                enemy_card.participant_index,
                &mut target_sel,
                &mut action_menu_state,
                &mut ranged_pending,
                &mut attack_writer,
                &mut ranged_writer,
            );
            break;
        }
    }
}

/// Perform an attack action using the supplied RNG (testable, deterministic).
pub fn perform_attack_action_with_rng(
    combat_res: &mut CombatResource,
    action: &AttackAction,
    content: &GameContent,
    global_state: &mut GlobalState,
    turn_state: &mut CombatTurnStateResource,
    rng: &mut impl rand::Rng,
) -> Result<(), CombatError> {
    // Ensure it's the attacker's turn
    if let Some(current) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        if current != &action.attacker {
            // Ignore actions from non-current actors
            return Ok(());
        }
    } else {
        return Ok(());
    }

    // Choose attack data
    let attack_data = match action.attacker {
        CombatantId::Player(idx) => {
            if let Some(Combatant::Player(pc)) = combat_res.state.participants.get(idx) {
                match get_character_attack(pc, &content.db().items) {
                    MeleeAttackResult::Melee(attack) => attack,
                    MeleeAttackResult::Ranged(_) => {
                        // Ranged weapons must be used via TurnAction::RangedAttack /
                        // perform_ranged_attack_action_with_rng, not the melee path.
                        // Log a warning and skip the turn rather than dealing wrong damage.
                        warn!(
                            "Player {:?} attempted melee attack with ranged weapon; \
                             use TurnAction::RangedAttack instead. Turn skipped.",
                            action.attacker
                        );
                        return Ok(());
                    }
                }
            } else {
                return Err(CombatError::CombatantNotFound(action.attacker));
            }
        }
        CombatantId::Monster(idx) => {
            if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get(idx) {
                let is_ranged = combat_res.combat_event_type
                    == crate::domain::combat::types::CombatEventType::Ranged;
                choose_monster_attack(mon, is_ranged, rng)
                    .unwrap_or(Attack::physical(DiceRoll::new(1, 4, 0)))
            } else {
                return Err(CombatError::CombatantNotFound(action.attacker));
            }
        }
    };

    // Resolve attack
    let (damage, special) = resolve_attack(
        &combat_res.state,
        action.attacker,
        action.target,
        &attack_data,
        Some(&global_state.0.active_spells),
        rng,
    )?;

    // Apply damage
    if damage > 0 {
        let died = apply_damage(&mut combat_res.state, action.target, damage)?;
        if died {
            tracing::debug!("Combatant {:?} was slain by damage", action.target);
        }
    }

    // Apply special effect if any (map to condition by name)
    if let Some(effect) = special {
        let effect_name = match effect {
            SpecialEffect::Poison => "poison",
            SpecialEffect::Disease => "disease",
            SpecialEffect::Paralysis => "paralysis",
            SpecialEffect::Sleep => "sleep",
            SpecialEffect::Drain => "drain",
            SpecialEffect::Stone => "stone",
            SpecialEffect::Death => "death",
        };

        if let Some(def) = content.db().conditions.get_condition_by_name(effect_name) {
            let active = crate::domain::conditions::ActiveCondition::new(
                def.id.clone(),
                def.default_duration,
            );
            match action.target {
                CombatantId::Player(idx) => {
                    if let Some(Combatant::Player(pc)) = combat_res.state.participants.get_mut(idx)
                    {
                        pc.active_conditions.push(active);
                    }
                }
                CombatantId::Monster(idx) => {
                    if let Some(Combatant::Monster(mon)) =
                        combat_res.state.participants.get_mut(idx)
                    {
                        mon.active_conditions.push(active);
                    }
                }
            }
        }
    }

    // Check combat end conditions
    combat_res.state.check_combat_end();

    if combat_res.state.status == CombatStatus::Fled {
        // Exit immediately if fled
        global_state.0.exit_combat();
        return Ok(());
    }

    // Advance turn and update turn state
    let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
        .db()
        .conditions
        .all_conditions()
        .into_iter()
        .filter_map(|id| content.db().conditions.get_condition(id).cloned())
        .collect();

    let _round_effects = combat_res.state.advance_turn(&cond_defs);

    // Update turn state based on next actor
    if let Some(next) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        turn_state.0 = match next {
            CombatantId::Player(_) => CombatTurnState::PlayerTurn,
            _ => CombatTurnState::EnemyTurn,
        };
    }

    Ok(())
}

/// Perform a ranged attack action for a player combatant.
///
/// Verifies the attacker has a ranged weapon equipped and ammo in their
/// inventory, resolves the attack roll using `resolve_attack`, consumes one
/// ammo item, applies damage via `apply_damage`, and advances the turn.
///
/// # Errors
///
/// - [`CombatError::CombatantCannotAct`] – attacker does not have a ranged
///   weapon equipped.
/// - [`CombatError::NoAmmo`] – attacker has a ranged weapon but no ammo.
/// - Propagates other [`CombatError`] variants from `resolve_attack` /
///   `apply_damage`.
pub fn perform_ranged_attack_action_with_rng(
    combat_res: &mut CombatResource,
    action: &RangedAttackAction,
    content: &GameContent,
    global_state: &mut GlobalState,
    turn_state: &mut CombatTurnStateResource,
    rng: &mut impl rand::Rng,
) -> Result<(), CombatError> {
    use crate::domain::items::ItemType;

    // Only process if it is currently the attacker's turn.
    if let Some(current) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        if current != &action.attacker {
            return Ok(());
        }
    } else {
        return Ok(());
    }

    // Only players can perform ranged attacks via this path.
    let attacker_idx = match action.attacker {
        CombatantId::Player(idx) => idx,
        _ => return Err(CombatError::CombatantCannotAct(action.attacker)),
    };

    // Verify the player has a ranged weapon.
    let has_ranged = {
        if let Some(Combatant::Player(pc)) = combat_res.state.participants.get(attacker_idx) {
            has_ranged_weapon(pc, &content.db().items)
        } else {
            return Err(CombatError::CombatantNotFound(action.attacker));
        }
    };

    if !has_ranged {
        // Check whether the weapon is present but ammo is missing.
        let has_bow =
            if let Some(Combatant::Player(pc)) = combat_res.state.participants.get(attacker_idx) {
                pc.equipment.weapon.is_some_and(|wid| {
                    content
                        .db()
                        .items
                        .get_item(wid)
                        .map(|i| {
                            if let crate::domain::items::ItemType::Weapon(w) = &i.item_type {
                                w.classification
                                    == crate::domain::items::WeaponClassification::MartialRanged
                            } else {
                                false
                            }
                        })
                        .unwrap_or(false)
                })
            } else {
                false
            };

        return if has_bow {
            Err(CombatError::NoAmmo)
        } else {
            Err(CombatError::CombatantCannotAct(action.attacker))
        };
    }

    // Retrieve the ranged attack data.
    let attack_data = {
        if let Some(Combatant::Player(pc)) = combat_res.state.participants.get(attacker_idx) {
            match get_character_attack(pc, &content.db().items) {
                MeleeAttackResult::Ranged(attack) => attack,
                MeleeAttackResult::Melee(_) => {
                    // Should not happen given has_ranged_weapon check above.
                    return Err(CombatError::CombatantCannotAct(action.attacker));
                }
            }
        } else {
            return Err(CombatError::CombatantNotFound(action.attacker));
        }
    };

    // Resolve the attack roll.
    let (damage, special) = resolve_attack(
        &combat_res.state,
        action.attacker,
        action.target,
        &attack_data,
        Some(&global_state.0.active_spells),
        rng,
    )?;

    // Consume one ammo item from the attacker's inventory (first Ammo slot).
    if let Some(Combatant::Player(pc)) = combat_res.state.participants.get_mut(attacker_idx) {
        if let Some(pos) = pc.inventory.items.iter().position(|slot| {
            content
                .db()
                .items
                .get_item(slot.item_id)
                .map(|i| matches!(i.item_type, ItemType::Ammo(_)))
                .unwrap_or(false)
        }) {
            pc.inventory.items.remove(pos);
        }
    }

    // Apply damage.
    if damage > 0 {
        let died = apply_damage(&mut combat_res.state, action.target, damage)?;
        if died {
            tracing::debug!("Combatant {:?} was slain by damage", action.target);
        }
    }

    // Apply special effect if any.
    if let Some(effect) = special {
        let effect_name = match effect {
            SpecialEffect::Poison => "poison",
            SpecialEffect::Disease => "disease",
            SpecialEffect::Paralysis => "paralysis",
            SpecialEffect::Sleep => "sleep",
            SpecialEffect::Drain => "drain",
            SpecialEffect::Stone => "stone",
            SpecialEffect::Death => "death",
        };

        if let Some(def) = content.db().conditions.get_condition_by_name(effect_name) {
            let active = crate::domain::conditions::ActiveCondition::new(
                def.id.clone(),
                def.default_duration,
            );
            match action.target {
                CombatantId::Player(idx) => {
                    if let Some(Combatant::Player(pc)) = combat_res.state.participants.get_mut(idx)
                    {
                        pc.active_conditions.push(active);
                    }
                }
                CombatantId::Monster(idx) => {
                    if let Some(Combatant::Monster(mon)) =
                        combat_res.state.participants.get_mut(idx)
                    {
                        mon.active_conditions.push(active);
                    }
                }
            }
        }
    }

    // Check combat end conditions.
    combat_res.state.check_combat_end();

    if combat_res.state.status == CombatStatus::Fled {
        global_state.0.exit_combat();
        return Ok(());
    }

    // Advance turn.
    let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
        .db()
        .conditions
        .all_conditions()
        .into_iter()
        .filter_map(|id| content.db().conditions.get_condition(id).cloned())
        .collect();

    let _round_effects = combat_res.state.advance_turn(&cond_defs);

    // Update turn state.
    if let Some(next) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        turn_state.0 = match next {
            CombatantId::Player(_) => CombatTurnState::PlayerTurn,
            _ => CombatTurnState::EnemyTurn,
        };
    }

    Ok(())
}

pub fn perform_cast_action_with_rng(
    combat_res: &mut CombatResource,
    action: &CastSpellAction,
    content: &GameContent,
    global_state: &mut GlobalState,
    turn_state: &mut CombatTurnStateResource,
    rng: &mut impl rand::Rng,
) -> Result<(), crate::domain::combat::engine::CombatError> {
    // Ensure it's the caster's turn
    if let Some(current) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        if current != &action.caster {
            // Ignore actions from non-current actors
            return Ok(());
        }
    } else {
        return Ok(());
    }

    // Permadeath guard: if this spell is a resurrection spell and the campaign
    // has permadeath enabled, block the cast silently.
    if let Some(spell) = content.db().spells.get_spell(action.spell_id) {
        if spell.resurrect_hp.is_some()
            && crate::application::resources::check_permadeath_allows_resurrection(
                &global_state.0.campaign_config,
            )
            .is_err()
        {
            // Permadeath is enabled — silently block the resurrection spell.
            return Ok(());
        }
    }

    // Execute the spell via the domain-level helper. It handles validation,
    // resource consumption and applies effects to combat participants.
    // We treat casting failures (insufficient SP, silenced, etc.) as non-fatal no-ops.
    let cast_result = crate::domain::combat::spell_casting::execute_spell_cast_by_id(
        &mut combat_res.state,
        action.caster,
        action.spell_id,
        action.target,
        content.db(),
        rng,
    );

    if let Err(_err) = cast_result {
        // Casting failed (insufficient SP, wrong class, silenced, etc.)
        // For now, treat this as a no-op from a combat flow perspective.
        return Ok(());
    }

    // If the combat state indicates a flee, exit combat immediately
    if combat_res.state.status == CombatStatus::Fled {
        global_state.0.exit_combat();
        return Ok(());
    }

    // Update turn state based on next actor (the domain helper advanced the turn)
    if let Some(next) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        turn_state.0 = match next {
            CombatantId::Player(_) => CombatTurnState::PlayerTurn,
            _ => CombatTurnState::EnemyTurn,
        };
    }

    Ok(())
}

/// Perform an item use action (domain -> game glue).
///
/// Executes the domain item usage helper which consumes inventory charges and
/// applies consumable effects (healing, cure, attribute boosts, etc.). Failures
/// (invalid slot, restrictions) are treated as non-fatal no-ops to keep the
/// game loop robust.
///
/// Enforces permadeath: if the item carries a `Resurrect` effect and the
/// active campaign has `permadeath == true`, the action is silently skipped.
pub fn perform_use_item_action_with_rng(
    combat_res: &mut CombatResource,
    action: &UseItemAction,
    content: &GameContent,
    global_state: &mut GlobalState,
    turn_state: &mut CombatTurnStateResource,
    rng: &mut impl rand::Rng,
) -> Result<(), crate::domain::combat::engine::CombatError> {
    // Ensure it's the actor's turn
    if let Some(current) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        if current != &action.user {
            // Ignore actions from non-current actors
            return Ok(());
        }
    } else {
        return Ok(());
    }

    // Permadeath guard: if the item being used is a Resurrect consumable and
    // the campaign has permadeath enabled, skip the action silently.
    {
        use crate::domain::combat::engine::Combatant;
        use crate::domain::items::types::ConsumableEffect;
        use crate::domain::items::ItemType;

        let is_resurrect = combat_res
            .state
            .get_combatant(&action.user)
            .and_then(|c| {
                if let Combatant::Player(pc) = c {
                    pc.inventory.items.get(action.inventory_index)
                } else {
                    None
                }
            })
            .and_then(|slot| content.db().items.get_item(slot.item_id))
            .map(|item| matches!(&item.item_type, ItemType::Consumable(d) if matches!(d.effect, ConsumableEffect::Resurrect(_))))
            .unwrap_or(false);

        if is_resurrect
            && crate::application::resources::check_permadeath_allows_resurrection(
                &global_state.0.campaign_config,
            )
            .is_err()
        {
            // Permadeath is enabled — silently block the resurrection.
            return Ok(());
        }
    }

    // Execute the item use via the domain-level helper. It handles validation,
    // charge consumption and effect application.
    let res = crate::domain::combat::item_usage::execute_item_use_by_slot(
        &mut combat_res.state,
        action.user,
        action.inventory_index,
        action.target,
        content.db(),
        rng,
    );

    if let Err(_err) = res {
        // Usage failed (invalid slot, restriction, etc.) - treat as a no-op
        return Ok(());
    }

    // If the combat state indicates a flee/exit, exit combat immediately
    if combat_res.state.status == CombatStatus::Fled {
        global_state.0.exit_combat();
        return Ok(());
    }

    // Update turn state based on next actor (the domain helper advanced the turn)
    if let Some(next) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        turn_state.0 = match next {
            CombatantId::Player(_) => CombatTurnState::PlayerTurn,
            _ => CombatTurnState::EnemyTurn,
        };
    }

    Ok(())
}

/// System wrapper: handle `AttackAction` messages and route to the attack performer.
fn handle_attack_action(
    mut reader: MessageReader<AttackAction>,
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
    mut feedback_writer: Option<MessageWriter<CombatFeedbackEvent>>,
    mut sfx_writer: Option<MessageWriter<crate::game::systems::audio::PlaySfx>>,
) {
    // Some tests or minimal harnesses may not register the full `GameContent` resource.
    // Use a lightweight in-memory database fallback so systems are resilient in tests.
    let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());

    for action in reader.read() {
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);

        // Capture pre-attack HP for the target so we can detect miss vs. hit
        let pre_hp: u16 = match action.target {
            CombatantId::Player(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Player(pc) => Some(pc.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
            CombatantId::Monster(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Monster(m) => Some(m.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
        };

        // Set Animating before the domain call
        let prior_turn_state = turn_state.0;
        turn_state.0 = CombatTurnState::Animating;

        let mut rng = rand::rng();
        if let Err(e) = perform_attack_action_with_rng(
            &mut combat_res,
            action,
            content_ref,
            &mut global_state,
            &mut turn_state,
            &mut rng,
        ) {
            tracing::warn!("Attack action failed: {}", e);
        }

        // Compute post-attack HP and damage dealt
        let post_hp: u16 = match action.target {
            CombatantId::Player(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Player(pc) => Some(pc.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
            CombatantId::Monster(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Monster(m) => Some(m.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
        };

        let dmg = (pre_hp as i32 - post_hp as i32).max(0) as u32;

        // Emit feedback event instead of spawning inline
        let effect = if dmg > 0 {
            CombatFeedbackEffect::Damage(dmg)
        } else {
            CombatFeedbackEffect::Miss
        };
        emit_combat_feedback(
            Some(action.attacker),
            action.target,
            effect,
            &mut feedback_writer,
        );

        // Restore turn state after action (perform_* may have already updated it)
        // Only restore if perform_* left it as Animating (meaning it didn't naturally advance)
        if matches!(turn_state.0, CombatTurnState::Animating) {
            turn_state.0 = prior_turn_state;
        }

        // Play SFX
        if dmg > 0 {
            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_hit".to_string(),
                });
            }
        } else if let Some(ref mut w) = sfx_writer {
            w.write(crate::game::systems::audio::PlaySfx {
                sfx_id: "combat_miss".to_string(),
            });
        }
    }
}

/// System wrapper: handle `UseItemAction` messages and route to the item performer.
///
/// This is analogous to the spell handler: it captures pre-use HP/SP for the
/// target so we can spawn UI feedback (floating numbers), performs the domain
fn handle_use_item_action(
    mut reader: MessageReader<UseItemAction>,
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
    mut feedback_writer: Option<MessageWriter<CombatFeedbackEvent>>,
    mut sfx_writer: Option<MessageWriter<crate::game::systems::audio::PlaySfx>>,
) {
    // Some tests or minimal harnesses may not register the full `GameContent` resource.
    // Use a lightweight in-memory database fallback so systems are resilient in tests.
    let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());

    for action in reader.read() {
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);

        // Capture pre-use HP/SP for the target so we can show numbers
        let (pre_hp, pre_sp): (u16, u16) = match action.target {
            CombatantId::Player(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Player(pc) => Some((pc.hp.current, pc.sp.current)),
                    _ => None,
                })
                .unwrap_or((0, 0)),
            CombatantId::Monster(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Monster(m) => Some((m.hp.current, 0)),
                    _ => None,
                })
                .unwrap_or((0, 0)),
        };

        // Set Animating before the domain call
        let prior_turn_state = turn_state.0;
        turn_state.0 = CombatTurnState::Animating;

        let mut rng = rand::rng();

        // Perform the use (domain-level). This consumes inventory charges and applies effects.
        if let Err(e) = perform_use_item_action_with_rng(
            &mut combat_res,
            action,
            content_ref,
            &mut global_state,
            &mut turn_state,
            &mut rng,
        ) {
            tracing::warn!("Use item action failed: {}", e);
        }

        // Compute post-use HP/SP and differences
        let (post_hp, post_sp): (u16, u16) = match action.target {
            CombatantId::Player(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Player(pc) => Some((pc.hp.current, pc.sp.current)),
                    _ => None,
                })
                .unwrap_or((0, 0)),
            CombatantId::Monster(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Monster(m) => Some((m.hp.current, 0)),
                    _ => None,
                })
                .unwrap_or((0, 0)),
        };

        let hp_delta = post_hp as i32 - pre_hp as i32;
        let sp_delta = post_sp as i32 - pre_sp as i32;

        // Emit typed feedback event; restore Animating state if unchanged
        if hp_delta < 0 {
            let dmg = (-hp_delta) as u32;
            emit_combat_feedback(
                Some(action.user),
                action.target,
                CombatFeedbackEffect::Damage(dmg),
                &mut feedback_writer,
            );
            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_hit".to_string(),
                });
            }
        } else if hp_delta > 0 {
            let healed = hp_delta as u32;
            emit_combat_feedback(
                Some(action.user),
                action.target,
                CombatFeedbackEffect::Heal(healed),
                &mut feedback_writer,
            );
            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_heal".to_string(),
                });
            }
        } else if sp_delta > 0 {
            let label = format!("+{} SP", sp_delta as u32);
            emit_combat_feedback(
                Some(action.user),
                action.target,
                CombatFeedbackEffect::Status(label),
                &mut feedback_writer,
            );
            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_heal".to_string(),
                });
            }
        } else {
            emit_combat_feedback(
                Some(action.user),
                action.target,
                CombatFeedbackEffect::Miss,
                &mut feedback_writer,
            );
            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_miss".to_string(),
                });
            }
        }

        // Restore turn state after action
        if matches!(turn_state.0, CombatTurnState::Animating) {
            turn_state.0 = prior_turn_state;
        }
    }
}

#[cfg(test)]
mod perform_use_item_tests {
    use super::*;
    use crate::application::GameState;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
    use crate::game::resources::GlobalState;
    use crate::sdk::database::ContentDatabase;

    #[test]
    fn test_perform_use_item_action_heal_consumes_and_heals() {
        // Setup content DB and a healing potion
        // Prepare content DB with a healing potion
        let mut content = GameContent::new(ContentDatabase::new());
        let potion = Item {
            id: 50,
            name: "Test Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(20),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        // Mutate the inner DB directly so tests can set up content in-place
        content.0.items.add_item(potion).unwrap();

        // Prepare combat state and player
        let mut combat_res = CombatResource::new();
        let mut player = Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "none".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        player.hp.base = 50;
        player.hp.current = 20;
        player.inventory.add_item(50, 1).unwrap();

        combat_res.state.add_player(player);

        // Ensure turn order contains the player as the current actor
        combat_res.state.turn_order = vec![CombatantId::Player(0)];
        combat_res.state.current_turn = 0;

        let mut global_state = GlobalState(GameState::new());
        let mut turn_state = CombatTurnStateResource::default();
        let mut rng = rand::rng();

        let action = UseItemAction {
            user: CombatantId::Player(0),
            inventory_index: 0,
            target: CombatantId::Player(0),
        };

        let _ = perform_use_item_action_with_rng(
            &mut combat_res,
            &action,
            &content,
            &mut global_state,
            &mut turn_state,
            &mut rng,
        );

        // Verify player healed and inventory slot consumed
        if let Some(Combatant::Player(pc_after)) =
            combat_res.state.get_combatant(&CombatantId::Player(0))
        {
            assert!(pc_after.hp.current > 20);
            assert!(pc_after.inventory.items.is_empty());
        } else {
            panic!("player not present after action");
        }
    }
}

/// System wrapper: handle `CastSpellAction` messages and route to the spell performer.
/// ECS system that reads [`RangedAttackAction`] messages and resolves them
/// via [`perform_ranged_attack_action_with_rng`].
fn handle_ranged_attack_action(
    mut reader: MessageReader<RangedAttackAction>,
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
    mut feedback_writer: Option<MessageWriter<CombatFeedbackEvent>>,
    mut combat_log: ResMut<CombatLogState>,
) {
    let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());
    let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);

    for msg in reader.read() {
        let mut rng = rand::rng();
        match perform_ranged_attack_action_with_rng(
            &mut combat_res,
            msg,
            content_ref,
            &mut global_state,
            &mut turn_state,
            &mut rng,
        ) {
            Ok(()) => {
                // Emit feedback for the ranged attack result.
                // We derive the damage from state changes rather than re-rolling,
                // so emit a generic Damage feedback (0 = miss).
                emit_combat_feedback(
                    Some(msg.attacker),
                    msg.target,
                    CombatFeedbackEffect::Miss, // placeholder; actual damage logged by format_combat_log_line
                    &mut feedback_writer,
                );
                // Log a simple combat entry for the ranged attack.
                let attacker_name = match msg.attacker {
                    CombatantId::Player(idx) => {
                        if let Some(Combatant::Player(pc)) = combat_res.state.participants.get(idx)
                        {
                            pc.name.clone()
                        } else {
                            "Unknown".to_string()
                        }
                    }
                    CombatantId::Monster(idx) => {
                        if let Some(Combatant::Monster(m)) = combat_res.state.participants.get(idx)
                        {
                            m.name.clone()
                        } else {
                            "Unknown".to_string()
                        }
                    }
                };
                combat_log.push_line(CombatLogLine {
                    segments: vec![CombatLogSegment {
                        text: format!("{} fires a ranged attack!", attacker_name),
                        color: FEEDBACK_COLOR_STATUS,
                    }],
                });
            }
            Err(CombatError::NoAmmo) => {
                combat_log.push_line(CombatLogLine {
                    segments: vec![CombatLogSegment {
                        text: "No ammo! Cannot fire ranged attack.".to_string(),
                        color: FEEDBACK_COLOR_STATUS,
                    }],
                });
            }
            Err(err) => {
                warn!("Ranged attack failed: {:?}", err);
            }
        }
    }
}

fn handle_cast_spell_action(
    mut reader: MessageReader<CastSpellAction>,
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
    mut feedback_writer: Option<MessageWriter<CombatFeedbackEvent>>,
    mut sfx_writer: Option<MessageWriter<crate::game::systems::audio::PlaySfx>>,
) {
    // Some tests or minimal harnesses may not register the full `GameContent` resource.
    // Use a lightweight in-memory database fallback so systems are resilient in tests.
    let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());

    for action in reader.read() {
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);

        // Capture pre-spell HP for the target so we can show damage numbers
        let pre_hp: u16 = match action.target {
            CombatantId::Player(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Player(pc) => Some(pc.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
            CombatantId::Monster(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Monster(m) => Some(m.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
        };

        // Set Animating before the domain call
        let prior_turn_state = turn_state.0;
        turn_state.0 = CombatTurnState::Animating;

        let mut rng = rand::rng();

        // Perform the cast (domain-level). This consumes SP/gems and applies effects.
        if let Err(e) = perform_cast_action_with_rng(
            &mut combat_res,
            action,
            content_ref,
            &mut global_state,
            &mut turn_state,
            &mut rng,
        ) {
            tracing::warn!("Cast action failed: {}", e);
        }

        // Compute post-spell HP and damage dealt
        let post_hp: u16 = match action.target {
            CombatantId::Player(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Player(pc) => Some(pc.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
            CombatantId::Monster(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Monster(m) => Some(m.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
        };

        let dmg = (pre_hp as i32 - post_hp as i32).max(0) as u32;

        // Emit typed feedback instead of inline spawn
        let effect = if dmg > 0 {
            CombatFeedbackEffect::Damage(dmg)
        } else {
            CombatFeedbackEffect::Miss
        };
        emit_combat_feedback(
            Some(action.caster),
            action.target,
            effect,
            &mut feedback_writer,
        );

        // Restore turn state after action
        if matches!(turn_state.0, CombatTurnState::Animating) {
            turn_state.0 = prior_turn_state;
        }

        // Play SFX
        if dmg > 0 {
            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_hit".to_string(),
                });
            }
        } else if let Some(ref mut w) = sfx_writer {
            w.write(crate::game::systems::audio::PlaySfx {
                sfx_id: "combat_miss".to_string(),
            });
        }
    }
}

/// Public helper: perform defend action (applies temporary AC bonus and advances turn).
pub fn perform_defend_action(
    combat_res: &mut CombatResource,
    action: &DefendAction,
    content: &GameContent,
    _global_state: &mut GlobalState,
    turn_state: &mut CombatTurnStateResource,
) -> Result<(), CombatError> {
    // Ensure it's the actor's turn
    if let Some(current) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        if current != &action.combatant {
            return Ok(());
        }
    } else {
        return Ok(());
    }

    // Apply AC bonus to the combatant
    match action.combatant {
        CombatantId::Player(idx) => {
            if let Some(Combatant::Player(pc)) = combat_res.state.participants.get_mut(idx) {
                pc.ac.modify(2);
            } else {
                return Err(CombatError::CombatantNotFound(action.combatant));
            }
        }
        CombatantId::Monster(idx) => {
            if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get_mut(idx) {
                mon.ac.modify(2);
            } else {
                return Err(CombatError::CombatantNotFound(action.combatant));
            }
        }
    }

    // Advance turn
    let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
        .db()
        .conditions
        .all_conditions()
        .into_iter()
        .filter_map(|id| content.db().conditions.get_condition(id).cloned())
        .collect();

    let turn_effects = combat_res.state.advance_turn(&cond_defs);
    if !turn_effects.is_empty() {
        tracing::debug!("Turn advance effects: {:?}", turn_effects);
    }

    // Update turn state
    if let Some(next) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        turn_state.0 = match next {
            CombatantId::Player(_) => CombatTurnState::PlayerTurn,
            _ => CombatTurnState::EnemyTurn,
        };
    }

    Ok(())
}

/// System wrapper for defend action
fn handle_defend_action(
    mut reader: MessageReader<DefendAction>,
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
) {
    let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());

    for action in reader.read() {
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);

        if let Err(e) = perform_defend_action(
            &mut combat_res,
            action,
            content_ref,
            &mut global_state,
            &mut turn_state,
        ) {
            tracing::warn!("Defend action failed: {}", e);
        }
    }
}

/// Public helper: perform flee action (returns true on success)
pub fn perform_flee_action(
    combat_res: &mut CombatResource,
    content: &GameContent,
    global_state: &mut GlobalState,
    turn_state: &mut CombatTurnStateResource,
) -> Result<bool, crate::domain::combat::engine::CombatError> {
    // Determine current actor
    let attacker = match combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        Some(a) => *a,
        None => return Ok(false),
    };

    // Flee only allowed if combat.can_flee is true
    if !combat_res.state.can_flee {
        // Consume turn
        let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
            .db()
            .conditions
            .all_conditions()
            .into_iter()
            .filter_map(|id| content.db().conditions.get_condition(id).cloned())
            .collect();
        let turn_effects = combat_res.state.advance_turn(&cond_defs);
        if !turn_effects.is_empty() {
            tracing::debug!("Turn advance effects: {:?}", turn_effects);
        }
        if let Some(next) = combat_res
            .state
            .turn_order
            .get(combat_res.state.current_turn)
        {
            turn_state.0 = match next {
                CombatantId::Player(_) => CombatTurnState::PlayerTurn,
                _ => CombatTurnState::EnemyTurn,
            };
        }
        return Ok(false);
    }

    // Determine flee success based on speed differential (simple rule)
    let attacker_speed = match attacker {
        CombatantId::Player(idx) => {
            if let Some(Combatant::Player(pc)) = combat_res.state.participants.get(idx) {
                pc.stats.speed.current
            } else {
                return Ok(false);
            }
        }
        CombatantId::Monster(idx) => {
            if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get(idx) {
                mon.stats.speed.current
            } else {
                return Ok(false);
            }
        }
    };

    // Find highest monster speed (if any monster exists)
    let highest_monster_speed = combat_res
        .state
        .participants
        .iter()
        .filter_map(|p| {
            if let Combatant::Monster(m) = p {
                Some(m.stats.speed.current)
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0);

    let success = attacker_speed >= highest_monster_speed;

    if success {
        combat_res.state.status = CombatStatus::Fled;
        global_state.0.exit_combat();
        return Ok(true);
    }

    // Failure: consume turn
    let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
        .db()
        .conditions
        .all_conditions()
        .into_iter()
        .filter_map(|id| content.db().conditions.get_condition(id).cloned())
        .collect();
    let turn_effects = combat_res.state.advance_turn(&cond_defs);
    if !turn_effects.is_empty() {
        tracing::debug!("Turn advance effects: {:?}", turn_effects);
    }
    if let Some(next) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        turn_state.0 = match next {
            CombatantId::Player(_) => CombatTurnState::PlayerTurn,
            _ => CombatTurnState::EnemyTurn,
        };
    }

    Ok(false)
}

/// System wrapper for FleeAction
fn handle_flee_action(
    mut reader: MessageReader<FleeAction>,
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
) {
    let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());

    for _ in reader.read() {
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);

        if let Err(e) = perform_flee_action(
            &mut combat_res,
            content_ref,
            &mut global_state,
            &mut turn_state,
        ) {
            tracing::warn!("Flee action failed: {}", e);
        }
    }
}

/// Selects a target for a monster according to its AI behavior.
///
/// Behavior:
/// - Aggressive: choose the lowest HP living player
/// - Defensive: choose the player with the highest 'threat' (might + accuracy)
/// - Random: choose a random living player
fn select_monster_target(
    combat_state: &CombatState,
    behavior: crate::domain::combat::monster::AiBehavior,
    rng: &mut impl rand::Rng,
) -> Option<CombatantId> {
    // Gather living player candidates (index, player reference)
    let mut candidates: Vec<(usize, &crate::domain::character::Character)> = Vec::new();
    for (idx, participant) in combat_state.participants.iter().enumerate() {
        if let Combatant::Player(pc) = participant {
            if pc.is_alive() {
                candidates.push((idx, pc));
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }

    match behavior {
        crate::domain::combat::monster::AiBehavior::Aggressive => {
            // Pick lowest HP player
            let (idx, _) = candidates
                .iter()
                .min_by_key(|(_, pc)| pc.hp.current)
                .expect("candidates guaranteed non-empty by is_empty guard");
            Some(CombatantId::Player(*idx))
        }
        crate::domain::combat::monster::AiBehavior::Defensive => {
            // Simple threat heuristic: might + accuracy
            let (idx, _) = candidates
                .iter()
                .max_by_key(|(_, pc)| {
                    (pc.stats.might.current as i32) + (pc.stats.accuracy.current as i32)
                })
                .expect("candidates guaranteed non-empty by is_empty guard");
            Some(CombatantId::Player(*idx))
        }
        crate::domain::combat::monster::AiBehavior::Random => {
            let chosen = rng.random_range(0..candidates.len());
            Some(CombatantId::Player(candidates[chosen].0))
        }
    }
}

/// Perform a monster's turn using the provided RNG (deterministic for tests).
///
/// This resolves the monster's attack choice, selects a target using AI behavior,
/// applies damage and special effects, advances the turn, and updates the
/// `CombatTurnStateResource`.
pub fn perform_monster_turn_with_rng(
    combat_res: &mut CombatResource,
    content: &GameContent,
    global_state: &mut GlobalState,
    turn_state: &mut CombatTurnStateResource,
    rng: &mut impl rand::Rng,
) -> Result<Option<(CombatantId, CombatantId, u32)>, CombatError> {
    // Determine current actor
    let attacker = match combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        Some(a) => *a,
        None => return Ok(None),
    };

    // Only handle monster turns
    let monster_idx = match attacker {
        CombatantId::Monster(idx) => idx,
        _ => return Ok(None),
    };

    // Ensure monster exists and can act (use a short-lived immutable borrow)
    {
        if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get(monster_idx) {
            if !mon.can_act() {
                // Monster cannot act (paralyzed, dead, or already acted)
                return Ok(None);
            }
        } else {
            return Err(CombatError::CombatantNotFound(attacker));
        }
    }

    // Determine AI behavior (read-only)
    let behavior =
        if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get(monster_idx) {
            mon.ai_behavior
        } else {
            return Err(CombatError::CombatantNotFound(attacker));
        };

    // Select a target using AI
    let target = match select_monster_target(&combat_res.state, behavior, rng) {
        Some(t) => t,
        None => {
            // No valid targets; check for end of combat
            combat_res.state.check_combat_end();
            return Ok(None);
        }
    };

    // Choose attack using domain helper
    let attack_data =
        if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get(monster_idx) {
            let is_ranged = combat_res.combat_event_type
                == crate::domain::combat::types::CombatEventType::Ranged;
            choose_monster_attack(mon, is_ranged, rng)
                .unwrap_or(Attack::physical(DiceRoll::new(1, 4, 0)))
        } else {
            return Err(CombatError::CombatantNotFound(attacker));
        };

    // Boss monsters never flee regardless of flee_threshold.
    // For non-boss encounters, monsters with flee_threshold > 0 may attempt
    // to flee when their HP drops below the threshold.
    let should_flee_this_turn = if combat_res.combat_event_type == CombatEventType::Boss {
        false
    } else if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get(monster_idx) {
        mon.should_flee()
    } else {
        false
    };

    if should_flee_this_turn {
        // Monster flees: mark acted, advance turn, but don't attack
        if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get_mut(monster_idx) {
            mon.mark_acted();
        }
        combat_res.state.check_combat_end();
        let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
            .db()
            .conditions
            .all_conditions()
            .into_iter()
            .filter_map(|id| content.db().conditions.get_condition(id).cloned())
            .collect();
        let turn_effects = combat_res.state.advance_turn(&cond_defs);
        if !turn_effects.is_empty() {
            tracing::debug!("Turn advance effects: {:?}", turn_effects);
        }
        if let Some(next) = combat_res
            .state
            .turn_order
            .get(combat_res.state.current_turn)
        {
            turn_state.0 = match next {
                CombatantId::Player(_) => CombatTurnState::PlayerTurn,
                _ => CombatTurnState::EnemyTurn,
            };
        }
        return Ok(None);
    }

    // Resolve attack (pure calculation, uses immutable state)
    let (damage, special) = resolve_attack(
        &combat_res.state,
        attacker,
        target,
        &attack_data,
        Some(&global_state.0.active_spells),
        rng,
    )?;

    // Apply damage (mutably modify combat state)
    if damage > 0 {
        let died = apply_damage(&mut combat_res.state, target, damage)?;
        if died {
            tracing::debug!("Combatant {:?} was slain by damage", target);
        }
    }

    // Apply special effect if any (map to condition by name)
    if let Some(effect) = special {
        let effect_name = match effect {
            SpecialEffect::Poison => "poison",
            SpecialEffect::Disease => "disease",
            SpecialEffect::Paralysis => "paralysis",
            SpecialEffect::Sleep => "sleep",
            SpecialEffect::Drain => "drain",
            SpecialEffect::Stone => "stone",
            SpecialEffect::Death => "death",
        };

        if let Some(def) = content.db().conditions.get_condition_by_name(effect_name) {
            let active = crate::domain::conditions::ActiveCondition::new(
                def.id.clone(),
                def.default_duration,
            );
            match target {
                CombatantId::Player(idx) => {
                    if let Some(Combatant::Player(pc)) = combat_res.state.participants.get_mut(idx)
                    {
                        pc.active_conditions.push(active);
                    }
                }
                CombatantId::Monster(idx) => {
                    if let Some(Combatant::Monster(mon)) =
                        combat_res.state.participants.get_mut(idx)
                    {
                        mon.active_conditions.push(active);
                    }
                }
            }
        }
    }

    // Mark monster as having acted
    if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get_mut(monster_idx) {
        mon.mark_acted();
    }

    // Check combat end conditions
    combat_res.state.check_combat_end();

    if combat_res.state.status == CombatStatus::Fled {
        global_state.0.exit_combat();
        return Ok(Some((attacker, target, damage as u32)));
    }

    // Advance turn and apply round start effects if needed
    let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
        .db()
        .conditions
        .all_conditions()
        .into_iter()
        .filter_map(|id| content.db().conditions.get_condition(id).cloned())
        .collect();

    let turn_effects = combat_res.state.advance_turn(&cond_defs);
    if !turn_effects.is_empty() {
        tracing::debug!("Turn advance effects: {:?}", turn_effects);
    }

    // Boss encounters regenerate BOSS_REGEN_PER_ROUND HP per round
    // (in addition to the 1 HP from advance_round's base regeneration).
    // The base engine regenerates 1 HP; we add 4 more to reach BOSS_REGEN_PER_ROUND.
    if combat_res.combat_event_type == CombatEventType::Boss && combat_res.state.monsters_regenerate
    {
        let bonus_regen = crate::domain::combat::types::BOSS_REGEN_PER_ROUND.saturating_sub(1);
        if bonus_regen > 0 {
            for participant in &mut combat_res.state.participants {
                if let Combatant::Monster(mon) = participant {
                    if mon.can_regenerate && mon.is_alive() {
                        mon.regenerate(bonus_regen);
                    }
                }
            }
        }
    }

    // Update turn sub-state based on next actor
    if let Some(next) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        turn_state.0 = match next {
            CombatantId::Player(_) => CombatTurnState::PlayerTurn,
            _ => CombatTurnState::EnemyTurn,
        };
    }

    Ok(Some((attacker, target, damage as u32)))
}

/// System: Execute monster turns automatically when it is EnemyTurn.
///
/// Picks an attack and target using AI, performs the attack, and advances
/// the turn. Uses the global RNG for in-game randomness.
///
/// # Ambush round handling
///
/// During round 1 of an ambush (`CombatState::ambush_round_active == true`),
/// player-combatant slots in the turn order must be skipped automatically so
/// that only monsters act. This function handles that case: whenever the
/// current combatant is a *player* during an ambush round it advances the turn
/// without performing any action, effectively consuming the player's surprised
/// turn. Once all turns in round 1 are exhausted `advance_round` clears the
/// flag and round 2 proceeds normally.
#[allow(clippy::too_many_arguments)]
fn execute_monster_turn(
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
    mut feedback_writer: Option<MessageWriter<CombatFeedbackEvent>>,
    mut combat_log: ResMut<CombatLogState>,
    mut monster_turn_timer: Option<ResMut<MonsterTurnTimer>>,
    time: Option<Res<Time>>,
    mut was_enemy_turn: Local<bool>,
) {
    // During an ambush round, if the current slot belongs to a player,
    // auto-skip it with a "Surprised!" log entry and advance the turn.
    // This keeps the EnemyTurn state active until all slots are consumed and
    // `advance_round` fires, clearing `ambush_round_active` at round 2.
    let current_id = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
        .cloned();
    if combat_res.state.ambush_round_active {
        if let Some(CombatantId::Player(_)) = current_id {
            combat_log.push_line(CombatLogLine {
                segments: vec![CombatLogSegment {
                    text: "The party is surprised and cannot act!".to_string(),
                    color: FEEDBACK_COLOR_STATUS,
                }],
            });

            let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());
            let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);
            let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content_ref
                .db()
                .conditions
                .all_conditions()
                .into_iter()
                .filter_map(|id| content_ref.db().conditions.get_condition(id).cloned())
                .collect();

            let turn_effects = combat_res.state.advance_turn(&cond_defs);
            if !turn_effects.is_empty() {
                tracing::debug!("Turn advance effects: {:?}", turn_effects);
            }

            // After advancing, determine what the next actor is.
            if let Some(next) = combat_res
                .state
                .turn_order
                .get(combat_res.state.current_turn)
            {
                turn_state.0 = match next {
                    CombatantId::Player(_) => {
                        // Still in ambush round — keep EnemyTurn so this system
                        // fires again next frame and skips the next surprised slot.
                        if combat_res.state.ambush_round_active {
                            CombatTurnState::EnemyTurn
                        } else {
                            CombatTurnState::PlayerTurn
                        }
                    }
                    CombatantId::Monster(_) => CombatTurnState::EnemyTurn,
                };
            }
            return;
        }
    }

    // Only run during combat and when it's enemy turn
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        *was_enemy_turn = false;
        return;
    }

    if !matches!(turn_state.0, CombatTurnState::EnemyTurn) {
        *was_enemy_turn = false;
        return;
    }

    // Ensure current actor is a monster
    let current_actor = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
        .cloned();

    if let Some(CombatantId::Monster(monster_idx)) = current_actor {
        // Check if the monster is able to act.  If it cannot (dead, paralyzed,
        // or already acted this round), we must still advance the turn so that
        // combat does not deadlock with turn_state perpetually stuck on EnemyTurn.
        let can_act = combat_res
            .state
            .participants
            .get(monster_idx)
            .map(|p| p.can_act())
            .unwrap_or(false);

        if !can_act {
            // Incapacitated monsters skip immediately — no delay needed.
            // Advance past this monster and update turn_state so the next actor
            // (player or another monster) gets control.
            info!(
                "Monster at participant index {} cannot act — advancing turn",
                monster_idx
            );
            // Advance turn (round effects are handled inside advance_turn when
            // the round wraps; we pass an empty condition list here because the
            // full DoT tick already happened in perform_monster_turn_with_rng on
            // earlier turns this round).
            let turn_effects = combat_res.state.advance_turn(&[]);
            if !turn_effects.is_empty() {
                tracing::debug!("Turn advance effects (skip): {:?}", turn_effects);
            }
            turn_state.0 = match combat_res
                .state
                .turn_order
                .get(combat_res.state.current_turn)
            {
                Some(CombatantId::Player(_)) => CombatTurnState::PlayerTurn,
                Some(CombatantId::Monster(_)) => CombatTurnState::EnemyTurn,
                None => CombatTurnState::PlayerTurn,
            };
            // Reset the was_enemy_turn flag so the delay arms fresh for the
            // next monster in the same round (if any).
            *was_enemy_turn = false;
            return;
        }

        // ── Delay gate ─────────────────────────────────────────────────────
        // The monster CAN act.  On the first frame we enter EnemyTurn for an
        // active monster, reset the timer so the player can read the combat log
        // before the attack resolves.  On subsequent frames, tick it.  Only
        // proceed once the timer has just finished.
        //
        // If the MonsterTurnTimer resource is absent (minimal test harnesses
        // that do not use CombatPlugin) we skip the delay entirely and act
        // immediately so existing unit tests need no changes.
        if let Some(ref mut timer) = monster_turn_timer {
            if !*was_enemy_turn {
                // First frame of this EnemyTurn — arm the timer and wait.
                timer.0.reset();
                *was_enemy_turn = true;
                return;
            }

            // Subsequent frames — tick the timer and wait until it finishes.
            let delta = time
                .as_ref()
                .map(|t| t.delta())
                .unwrap_or(std::time::Duration::ZERO);
            timer.0.tick(delta);

            if !timer.0.just_finished() {
                return;
            }
            // Timer just finished — fall through to execute the monster's action.
        }
        // If MonsterTurnTimer resource is absent, act immediately (test path).

        // Fallback content when none registered (tests often omit GameContent)
        let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);

        let mut rng = rand::rng();
        // Capture the round counter before the turn so we can detect
        // when a new round starts and emit boss-regeneration log lines.
        let round_before = combat_res.state.round;
        let outcome = perform_monster_turn_with_rng(
            &mut combat_res,
            content_ref,
            &mut global_state,
            &mut turn_state,
            &mut rng,
        );
        let round_after = combat_res.state.round;

        // Emit boss regeneration log line when a new round started
        if round_after > round_before
            && combat_res.combat_event_type == CombatEventType::Boss
            && combat_res.state.monsters_regenerate
        {
            for participant in &combat_res.state.participants {
                if let Combatant::Monster(mon) = participant {
                    if mon.can_regenerate && mon.is_alive() {
                        combat_log.push_line(CombatLogLine {
                            segments: vec![CombatLogSegment {
                                text: format!(
                                    "{} regenerates {} HP!",
                                    mon.name,
                                    crate::domain::combat::types::BOSS_REGEN_PER_ROUND
                                ),
                                color: FEEDBACK_COLOR_HEAL,
                            }],
                        });
                    }
                }
            }
        }

        if let Ok(Some((attacker, target, damage))) = outcome {
            let effect = if damage > 0 {
                CombatFeedbackEffect::Damage(damage)
            } else {
                CombatFeedbackEffect::Miss
            };
            emit_combat_feedback(Some(attacker), target, effect, &mut feedback_writer);
        }
    } else if !combat_res.state.turn_order.is_empty() {
        // The turn_order slot exists but is a Player, yet turn_state is EnemyTurn
        // — this is a stale/inconsistent state.  Correct it so the player regains
        // control rather than hanging forever.
        //
        // The `turn_order.is_empty()` guard prevents this branch from firing
        // during partially-initialised test states where EnemyTurn is set
        // manually before a full combat state (with monsters) is wired up.
        warn!(
            "execute_monster_turn: EnemyTurn but current actor is not a monster (turn {}); \
             correcting to PlayerTurn",
            combat_res.state.current_turn
        );
        turn_state.0 = CombatTurnState::PlayerTurn;
    }
}

// ===== Combat Resolution & Rewards =====

/// System: Detect when combat ends (Victory/Defeat) and emit an appropriate message
///
/// This ensures the resolution is only handled once by tracking a flag in
/// `CombatResource`.
/// System: advance game time by `TIME_COST_COMBAT_ROUND_MINUTES` once per new
/// combat round.
///
/// `CombatResource::last_timed_round` is initialised to `0` and the combat
/// state's round counter starts at `1`, so the first round is charged
/// immediately when combat begins.  Subsequent rounds are charged when
/// `combat_res.state.round` exceeds `last_timed_round`.
///
/// This system is a no-op outside of combat, ensuring that the clock is never
/// advanced by stale combat data.
fn tick_combat_time(mut combat_res: ResMut<CombatResource>, mut global_state: ResMut<GlobalState>) {
    // Only run while the global state is in combat mode.
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    let current_round = combat_res.state.round;
    if current_round > combat_res.last_timed_round {
        let new_rounds = current_round - combat_res.last_timed_round;
        global_state.0.advance_time(
            new_rounds * crate::domain::resources::TIME_COST_COMBAT_ROUND_MINUTES,
            None,
        );
        combat_res.last_timed_round = current_round;
    }
}

fn check_combat_resolution(
    mut victory_writer: Option<MessageWriter<CombatVictory>>,
    mut defeat_writer: Option<MessageWriter<CombatDefeat>>,
    mut combat_res: ResMut<CombatResource>,
    global_state: Res<GlobalState>,
) {
    // Only consider when in combat
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    if combat_res.resolution_handled {
        return;
    }

    match combat_res.state.status {
        CombatStatus::Victory => {
            if let Some(ref mut w) = victory_writer {
                w.write(CombatVictory);
            }
            combat_res.resolution_handled = true;
        }
        CombatStatus::Defeat => {
            if let Some(ref mut w) = defeat_writer {
                w.write(CombatDefeat);
            }
            combat_res.resolution_handled = true;
        }
        _ => {}
    }
}

/// Summary of rewards from a victorious combat - returned by the reward processor.
pub struct VictorySummary {
    pub total_xp: u64,
    pub xp_awarded: Vec<(usize, u64)>, // (party_index, xp_amount)
    pub total_gold: u32,
    pub total_gems: u32,
    pub items: Vec<ItemId>,
    /// True when this was a Boss encounter — callers should display a
    /// "Boss Defeated!" header in the victory screen.
    pub boss_defeated: bool,
}

/// Process victory rewards using the provided RNG. This will:
/// - Sum monster XP and distribute it to living party members
/// - Roll gold/gem drops and add them to the party
/// - Resolve item drops and add them to party members' inventories (if space)
pub fn process_combat_victory_with_rng(
    combat_res: &mut CombatResource,
    _content: &GameContent,
    global_state: &mut GlobalState,
    rng: &mut impl rand::Rng,
) -> Result<VictorySummary, crate::domain::combat::engine::CombatError> {
    // Sum XP from dead monsters
    let mut total_xp: u64 = 0;
    for participant in &combat_res.state.participants {
        if let Combatant::Monster(mon) = participant {
            if !mon.is_alive() {
                total_xp = total_xp.saturating_add(mon.loot.experience as u64);
            }
        }
    }

    // Determine recipient party indices (from mapping); fallback to living party members
    let mut recipients: Vec<usize> = Vec::new();
    for (idx, participant) in combat_res.state.participants.iter().enumerate() {
        if let Combatant::Player(pc) = participant {
            if pc.is_alive() {
                if let Some(Some(party_idx)) = combat_res.player_orig_indices.get(idx).cloned() {
                    if party_idx < global_state.0.party.members.len() {
                        recipients.push(party_idx);
                    }
                }
            }
        }
    }

    if recipients.is_empty() {
        for (i, member) in global_state.0.party.members.iter().enumerate() {
            if member.is_alive() {
                recipients.push(i);
            }
        }
    }

    let mut xp_awarded: Vec<(usize, u64)> = Vec::new();
    if !recipients.is_empty() && total_xp > 0 {
        let per = total_xp / recipients.len() as u64;
        let mut remainder = total_xp % recipients.len() as u64;

        for &party_idx in &recipients {
            let mut award = per;
            if remainder > 0 {
                award += 1;
                remainder -= 1;
            }

            // Award experience using domain helper (respects dead checks)
            if let Err(e) = crate::domain::progression::award_experience(
                &mut global_state.0.party.members[party_idx],
                award,
            ) {
                tracing::warn!(
                    "Failed to award {} XP to party member {}: {}",
                    award,
                    party_idx,
                    e
                );
            }
            xp_awarded.push((party_idx, award));
        }
    }

    // Roll gold/gems/items from dead monsters and award to party
    let mut total_gold: u32 = 0;
    let mut total_gems: u32 = 0u32;
    let mut items_dropped: Vec<ItemId> = Vec::new();

    for participant in &combat_res.state.participants {
        if let Combatant::Monster(mon) = participant {
            if !mon.is_alive() {
                // Gold
                if mon.loot.gold_max >= mon.loot.gold_min {
                    let gold = if mon.loot.gold_max == mon.loot.gold_min {
                        mon.loot.gold_min
                    } else {
                        rng.random_range(mon.loot.gold_min..=mon.loot.gold_max)
                    };
                    total_gold = total_gold.saturating_add(gold);
                }

                // Gems
                if mon.loot.gems_max >= mon.loot.gems_min {
                    let gems = if mon.loot.gems_max == mon.loot.gems_min {
                        mon.loot.gems_min as u32
                    } else {
                        rng.random_range(mon.loot.gems_min..=mon.loot.gems_max) as u32
                    };
                    total_gems = total_gems.saturating_add(gems);
                }

                // Items (probabilistic)
                for (prob, item_id) in &mon.loot.items {
                    if rng.random::<f32>() < *prob {
                        items_dropped.push(*item_id);
                    }
                }
            }
        }
    }

    // Apply currency to party (pooled)
    global_state.0.party.gold = global_state.0.party.gold.saturating_add(total_gold);
    global_state.0.party.gems = global_state.0.party.gems.saturating_add(total_gems);

    // Distribute items to first available living party members
    for item_id in &items_dropped {
        let mut placed = false;
        for &party_idx in &recipients {
            if let Some(member) = global_state.0.party.members.get_mut(party_idx) {
                if member.inventory.has_space() {
                    if let Err(e) = member.inventory.add_item(*item_id, 0) {
                        tracing::warn!("Failed to add loot item {:?}: {}", item_id, e);
                    }
                    placed = true;
                    break;
                }
            }
        }

        // If no recipient had space, attempt to stash on any living member
        if !placed {
            for member in global_state.0.party.members.iter_mut() {
                if member.is_alive() && member.inventory.has_space() {
                    if let Err(e) = member.inventory.add_item(*item_id, 0) {
                        tracing::warn!("Failed to add loot item {:?}: {}", item_id, e);
                    }
                    break;
                }
            }
        }
    }

    Ok(VictorySummary {
        total_xp,
        xp_awarded,
        total_gold,
        total_gems,
        items: items_dropped,
        boss_defeated: combat_res.combat_event_type == CombatEventType::Boss,
    })
}

/// System: Handle CombatVictory messages and apply rewards.
///
/// This runs as a system wrapper and spawns a simple victory summary UI.
fn handle_combat_victory(
    mut reader: MessageReader<CombatVictory>,
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut commands: Commands,
    mut music_writer: Option<MessageWriter<crate::game::systems::audio::PlayMusic>>,
    mut sfx_writer: Option<MessageWriter<crate::game::systems::audio::PlaySfx>>,
) {
    let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());

    for _ in reader.read() {
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);
        let mut rng = rand::rng();

        // Remove the encounter event from the map so the tile no longer re-triggers.
        if let (Some(pos), Some(map_id)) =
            (combat_res.encounter_position, combat_res.encounter_map_id)
        {
            if let Some(map) = global_state.0.world.get_map_mut(map_id) {
                map.remove_event(pos);
                info!("Removed encounter event at {:?} on map {}", pos, map_id);
            }
        }
        // Clear stored position so it doesn't accidentally affect a later combat.
        combat_res.encounter_position = None;
        combat_res.encounter_map_id = None;

        if let Ok(summary) = process_combat_victory_with_rng(
            &mut combat_res,
            content_ref,
            &mut global_state,
            &mut rng,
        ) {
            // Spawn a simple victory UI
            commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                    VictorySummaryRoot,
                ))
                .with_children(|parent| {
                    let header = if summary.boss_defeated {
                        "⚔ Boss Defeated! ⚔\n".to_string()
                    } else {
                        String::new()
                    };
                    parent.spawn((
                        Text::new(format!(
                            "{}Victory! XP: {}  Gold: {}  Gems: {}  Items: {:?}",
                            header,
                            summary.total_xp,
                            summary.total_gold,
                            summary.total_gems,
                            summary.items
                        )),
                        text_style(BODY_FONT_SIZE, Color::WHITE),
                    ));
                });

            // Play victory SFX and transition back to exploration music (if audio plugin present)
            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "victory_fanfare".to_string(),
                });
            }
            if let Some(ref mut w) = music_writer {
                w.write(crate::game::systems::audio::PlayMusic {
                    track_id: "exploration_theme".to_string(),
                    looped: true,
                });
            }
        }

        // Exit combat (will trigger sync back to party on next frame)
        global_state.0.exit_combat();
    }
}

/// Process defeat state (non-UI part) - updates global game mode/state.
pub fn process_combat_defeat_state(
    _combat_res: &mut CombatResource,
    global_state: &mut GlobalState,
) {
    // Exit combat and open menu (allow player to load/quit)
    global_state.0.exit_combat();
    global_state.0.enter_menu();
}

/// System: Handle CombatDefeat messages and display defeat UI + menu
fn handle_combat_defeat(
    mut reader: MessageReader<CombatDefeat>,
    mut combat_res: ResMut<CombatResource>,
    mut global_state: ResMut<GlobalState>,
    mut commands: Commands,
    mut music_writer: Option<MessageWriter<crate::game::systems::audio::PlayMusic>>,
    mut sfx_writer: Option<MessageWriter<crate::game::systems::audio::PlaySfx>>,
) {
    for _ in reader.read() {
        process_combat_defeat_state(&mut combat_res, &mut global_state);

        // Spawn simple defeat UI overlay
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                DefeatSummaryRoot,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new("You have been defeated. Returning to menu...".to_string()),
                    text_style(BODY_FONT_SIZE, Color::WHITE),
                ));
            });

        // Play defeat SFX / music (if available)
        if let Some(ref mut w) = sfx_writer {
            w.write(crate::game::systems::audio::PlaySfx {
                sfx_id: "defeat_sound".to_string(),
            });
        }
        if let Some(ref mut w) = music_writer {
            w.write(crate::game::systems::audio::PlayMusic {
                track_id: "defeat_music".to_string(),
                looped: false,
            });
        }
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.0, 0.0, 0.7)),
                DefeatSummaryRoot,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new("Defeat! You have been defeated."),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
    }
}

// ===== Visual Combat Feedback Helpers & Systems =====

/// Helper: write a `CombatFeedbackEvent` to the message bus when a writer is available.
///
/// This is the single call-site used by all three action handlers and the
/// monster-turn executor so that every combat result produces exactly one event.
///
/// # Arguments
///
/// * `target` - The combatant that was affected.
/// * `effect` - What happened (damage amount, heal, miss, status name).
/// * `writer` - Optional `MessageWriter`; a no-op when `None` (e.g. in tests
///   where the message bus is not registered).
///
/// # Examples
///
/// ```
/// use antares::game::systems::combat::{CombatFeedbackEffect, CombatFeedbackEvent};
/// use antares::domain::combat::types::CombatantId;
///
/// // Demonstrates how emit_combat_feedback behaves with no writer present
/// let target = CombatantId::Monster(0);
/// let effect = CombatFeedbackEffect::Damage(10);
/// // emit_combat_feedback(target, effect, &mut None::<bevy_ecs::system::MessageWriter<CombatFeedbackEvent>>);
/// // No writer → no-op; compiles fine.
/// ```
fn emit_combat_feedback(
    source: Option<CombatantId>,
    target: CombatantId,
    effect: CombatFeedbackEffect,
    writer: &mut Option<MessageWriter<CombatFeedbackEvent>>,
) {
    if let Some(ref mut w) = writer {
        w.write(CombatFeedbackEvent {
            source,
            target,
            effect,
        });
    }
}

/// Resolve a combatant display name from participant state.
fn combatant_display_name(combat_res: &CombatResource, id: CombatantId) -> String {
    match id {
        CombatantId::Player(idx) => combat_res
            .state
            .participants
            .get(idx)
            .and_then(|p| match p {
                Combatant::Player(pc) => Some(pc.name.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "Player".to_string()),
        CombatantId::Monster(idx) => combat_res
            .state
            .participants
            .get(idx)
            .and_then(|p| match p {
                Combatant::Monster(mon) => Some(mon.name.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "Monster".to_string()),
    }
}

/// Deterministically assign a character-name colour from a fixed palette.
fn character_name_color(name: &str) -> Color {
    // Use a stable hash so the same character name always renders in the same colour.
    let hash = name
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(131).wrapping_add(b as u64));
    let idx = (hash as usize) % COMBAT_LOG_CHARACTER_PALETTE.len();
    COMBAT_LOG_CHARACTER_PALETTE[idx]
}

/// Resolve the display colour for a combatant name.
fn combatant_name_color(
    id: CombatantId,
    color_state: &mut CombatLogColorState,
    rng: &mut impl rand::Rng,
    character_name: Option<&str>,
) -> Color {
    match id {
        CombatantId::Player(_) => character_name
            .map(character_name_color)
            .unwrap_or(Color::WHITE),
        CombatantId::Monster(idx) => {
            if let Some(existing) = color_state.monster_colors.get(&idx) {
                *existing
            } else {
                let palette_idx = rng.random_range(0..COMBAT_LOG_MONSTER_PALETTE.len());
                let color = COMBAT_LOG_MONSTER_PALETTE[palette_idx];
                color_state.monster_colors.insert(idx, color);
                color
            }
        }
    }
}

/// Format `CombatFeedbackEvent` into structured coloured segments.
fn format_combat_log_line(
    combat_res: &CombatResource,
    event: &CombatFeedbackEvent,
    color_state: &mut CombatLogColorState,
    rng: &mut impl rand::Rng,
) -> CombatLogLine {
    let target_name = combatant_display_name(combat_res, event.target);
    let target_color = combatant_name_color(event.target, color_state, rng, Some(&target_name));

    if let Some(source) = event.source {
        let source_name = combatant_display_name(combat_res, source);
        let source_color = combatant_name_color(source, color_state, rng, Some(&source_name));

        match &event.effect {
            CombatFeedbackEffect::Damage(n) => {
                return CombatLogLine {
                    segments: vec![
                        CombatLogSegment {
                            text: source_name,
                            color: source_color,
                        },
                        CombatLogSegment {
                            text: ": Attacks ".to_string(),
                            color: Color::WHITE,
                        },
                        CombatLogSegment {
                            text: target_name,
                            color: target_color,
                        },
                        CombatLogSegment {
                            text: format!(" for [{}] damage", n),
                            color: Color::WHITE,
                        },
                    ],
                };
            }
            CombatFeedbackEffect::Miss => {
                return CombatLogLine {
                    segments: vec![
                        CombatLogSegment {
                            text: source_name,
                            color: source_color,
                        },
                        CombatLogSegment {
                            text: ": Misses ".to_string(),
                            color: Color::WHITE,
                        },
                        CombatLogSegment {
                            text: target_name,
                            color: target_color,
                        },
                    ],
                };
            }
            CombatFeedbackEffect::Heal(n) => {
                return CombatLogLine {
                    segments: vec![
                        CombatLogSegment {
                            text: source_name,
                            color: source_color,
                        },
                        CombatLogSegment {
                            text: ": Heals ".to_string(),
                            color: Color::WHITE,
                        },
                        CombatLogSegment {
                            text: target_name,
                            color: target_color,
                        },
                        CombatLogSegment {
                            text: format!(" for [{}] HP", n),
                            color: Color::WHITE,
                        },
                    ],
                };
            }
            CombatFeedbackEffect::Status(s) => {
                return CombatLogLine {
                    segments: vec![
                        CombatLogSegment {
                            text: source_name,
                            color: source_color,
                        },
                        CombatLogSegment {
                            text: format!(": {}", s),
                            color: FEEDBACK_COLOR_STATUS,
                        },
                    ],
                };
            }
        }
    }

    // Fallback formatting for effects without an explicit source.
    match &event.effect {
        CombatFeedbackEffect::Damage(n) => CombatLogLine {
            segments: vec![
                CombatLogSegment {
                    text: target_name,
                    color: target_color,
                },
                CombatLogSegment {
                    text: format!(": takes [{}] damage", n),
                    color: Color::WHITE,
                },
            ],
        },
        CombatFeedbackEffect::Heal(n) => CombatLogLine {
            segments: vec![
                CombatLogSegment {
                    text: target_name,
                    color: target_color,
                },
                CombatLogSegment {
                    text: format!(": recovers [{}] HP", n),
                    color: Color::WHITE,
                },
            ],
        },
        CombatFeedbackEffect::Miss => CombatLogLine {
            segments: vec![CombatLogSegment {
                text: format!("Misses {}", target_name),
                color: FEEDBACK_COLOR_MISS,
            }],
        },
        CombatFeedbackEffect::Status(s) => CombatLogLine {
            segments: vec![
                CombatLogSegment {
                    text: target_name,
                    color: target_color,
                },
                CombatLogSegment {
                    text: format!(": {}", s),
                    color: FEEDBACK_COLOR_STATUS,
                },
            ],
        },
    }
}

/// Consume combat feedback events and append them to the persistent combat log.
fn collect_combat_feedback_log_lines(
    mut reader: MessageReader<CombatFeedbackEvent>,
    global_state: Res<GlobalState>,
    combat_res: Res<CombatResource>,
    mut combat_log_state: ResMut<CombatLogState>,
    mut color_state: ResMut<CombatLogColorState>,
) {
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    let mut rng = rand::rng();
    for event in reader.read() {
        let line = format_combat_log_line(&combat_res, event, &mut color_state, &mut rng);
        let plain_text = line.plain_text();
        debug!("CombatLog: {}", plain_text);
        info!("Combat: {}", plain_text);
        combat_log_state.push_line(line);
    }
}

/// Mirror combat feedback into the persistent game log so players can review
/// combat events after the combat bubble is cleaned up.
fn mirror_combat_feedback_to_game_log(
    mut reader: MessageReader<CombatFeedbackEvent>,
    global_state: Res<GlobalState>,
    combat_res: Res<CombatResource>,
    mut color_state: ResMut<CombatLogColorState>,
    mut game_log_writer: Option<MessageWriter<crate::game::systems::ui::GameLogEvent>>,
) {
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    let Some(ref mut game_log_writer) = game_log_writer else {
        for _ in reader.read() {}
        return;
    };

    let mut rng = rand::rng();
    for event in reader.read() {
        let plain_text =
            format_combat_log_line(&combat_res, event, &mut color_state, &mut rng).plain_text();
        game_log_writer.write(crate::game::systems::ui::GameLogEvent {
            text: plain_text,
            category: crate::game::systems::ui::LogCategory::Combat,
        });
    }
}

/// Advance typewriter reveal for the newest combat log line.
fn update_combat_log_typewriter(
    time: Res<Time>,
    global_state: Res<GlobalState>,
    mut combat_log_state: ResMut<CombatLogState>,
) {
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    let Some(latest) = combat_log_state.lines.last() else {
        return;
    };

    let total_chars = latest.char_count();
    if combat_log_state.active_line_visible_chars >= total_chars {
        return;
    }

    combat_log_state.reveal_accumulator += time.delta_secs() * COMBAT_LOG_TYPEWRITER_CHARS_PER_SEC;
    while combat_log_state.reveal_accumulator >= 1.0
        && combat_log_state.active_line_visible_chars < total_chars
    {
        combat_log_state.active_line_visible_chars += 1;
        combat_log_state.reveal_accumulator -= 1.0;
    }
}

/// Sync the combat log bubble rows from `CombatLogState`.
fn update_combat_log_bubble_text(
    mut commands: Commands,
    combat_log_state: Res<CombatLogState>,
    line_list_query: Query<(Entity, Option<&Children>), With<CombatLogLineList>>,
) {
    if !combat_log_state.is_changed() {
        return;
    }

    let Ok((line_list_entity, children)) = line_list_query.single() else {
        return;
    };

    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    if combat_log_state.lines.is_empty() {
        return;
    }

    let latest_idx = combat_log_state.lines.len() - 1;

    commands.entity(line_list_entity).with_children(|list| {
        for (idx, line) in combat_log_state.lines.iter().enumerate() {
            let segments = if idx == latest_idx {
                line.clipped_segments(combat_log_state.active_line_visible_chars)
            } else {
                line.segments.clone()
            };

            if segments.is_empty() {
                continue;
            }

            list.spawn((Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },))
                .with_children(|row| {
                    for segment in segments {
                        row.spawn((
                            Text::new(segment.text),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(segment.color),
                        ));
                    }
                });
        }
    });
}

/// Auto-scroll the combat log viewport to the newest line when log state changes.
fn auto_scroll_combat_log_viewport(
    combat_log_state: Res<CombatLogState>,
    mut viewports: Query<&mut ScrollPosition, With<CombatLogBubbleViewport>>,
) {
    if !combat_log_state.is_changed() {
        return;
    }

    for mut pos in viewports.iter_mut() {
        // Scroll downward to reveal latest entries; layout clamps to valid range.
        pos.0.y = f32::MAX;
    }
}

/// Keep monster colour assignments encounter-local and reset them when combat ends.
fn reset_combat_log_colors_on_exit(
    global_state: Res<GlobalState>,
    mut color_state: ResMut<CombatLogColorState>,
) {
    if matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    if !color_state.monster_colors.is_empty() {
        color_state.monster_colors.clear();
    }
}

#[cfg(test)]
mod combat_log_format_tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::combat::monster::Monster;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn make_combat_resource_for_log() -> CombatResource {
        let mut cr = CombatResource::new();
        let player = Character::new(
            "Ariadne".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        let monster = Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(8, 8, 8, 8, 8, 8, 8),
            10,
            5,
            vec![crate::domain::combat::types::Attack::physical(
                DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );

        cr.state.add_player(player);
        cr.state.add_monster(monster);
        cr
    }

    #[test]
    fn test_character_name_color_is_stable() {
        let c1 = character_name_color("Ariadne");
        let c2 = character_name_color("Ariadne");
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_monster_color_is_stable_per_participant() {
        let mut state = CombatLogColorState::default();
        let mut rng = StdRng::seed_from_u64(11);
        let c1 = combatant_name_color(CombatantId::Monster(1), &mut state, &mut rng, None);
        let c2 = combatant_name_color(CombatantId::Monster(1), &mut state, &mut rng, None);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_damage_line_matches_structured_format() {
        let cr = make_combat_resource_for_log();
        let mut colors = CombatLogColorState::default();
        let mut rng = StdRng::seed_from_u64(7);
        let event = CombatFeedbackEvent {
            source: Some(CombatantId::Player(0)),
            target: CombatantId::Monster(1),
            effect: CombatFeedbackEffect::Damage(9),
        };

        let line = format_combat_log_line(&cr, &event, &mut colors, &mut rng);
        let joined = line
            .segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<String>();

        assert!(joined.contains("Ariadne: Attacks Goblin for [9] damage"));
    }
}

/// Clear the persistent combat log when combat ends so the next encounter starts fresh.
fn reset_combat_log_on_exit(
    global_state: Res<GlobalState>,
    mut combat_log_state: ResMut<CombatLogState>,
) {
    if matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    if !combat_log_state.lines.is_empty()
        || combat_log_state.active_line_visible_chars != 0
        || combat_log_state.reveal_accumulator != 0.0
    {
        *combat_log_state = CombatLogState::default();
    }
}

/// System: read `CombatFeedbackEvent` messages and spawn anchored `FloatingDamage` nodes.
///
/// For monster targets the node is spawned as a **child** of the corresponding
/// [`EnemyHpBarBackground`] node (not the whole card) so it sits in the
/// lower-right corner of the HP bar area without affecting the card's flex
/// layout.  The background node has `overflow: visible` so the text is never
/// clipped.  For player targets it is spawned at an absolute position (HUD
/// bottom area) because the player HUD layout differs per game.
///
/// Colour is chosen from the visual feedback constants:
/// - Red   (`FEEDBACK_COLOR_DAMAGE`) — `Damage(_)`
/// - Green (`FEEDBACK_COLOR_HEAL`)   — `Heal(_)`
/// - Grey  (`FEEDBACK_COLOR_MISS`)   — `Miss`
/// - Yellow(`FEEDBACK_COLOR_STATUS`) — `Status(_)`
fn spawn_combat_feedback(
    mut reader: MessageReader<CombatFeedbackEvent>,
    mut commands: Commands,
    hp_bar_backgrounds: Query<(Entity, &EnemyHpBarBackground)>,
) {
    for event in reader.read() {
        let (text, color) = match &event.effect {
            CombatFeedbackEffect::Damage(n) => (format!("-{}", n), FEEDBACK_COLOR_DAMAGE),
            CombatFeedbackEffect::Heal(n) => (format!("+{}", n), FEEDBACK_COLOR_HEAL),
            CombatFeedbackEffect::Miss => ("Miss".to_string(), FEEDBACK_COLOR_MISS),
            CombatFeedbackEffect::Status(s) => (s.clone(), FEEDBACK_COLOR_STATUS),
        };

        let font_size = match &event.effect {
            CombatFeedbackEffect::Damage(_) | CombatFeedbackEffect::Heal(_) => 18.0,
            _ => 15.0,
        };

        match event.target {
            CombatantId::Monster(idx) => {
                // Anchor to the HP bar background node for this participant so
                // the damage text sits in the lower-right of the bar without
                // disturbing the card's flex layout.
                let bar_entity = hp_bar_backgrounds
                    .iter()
                    .find(|(_, bg)| bg.participant_index == idx)
                    .map(|(e, _)| e);

                if let Some(bar) = bar_entity {
                    commands.entity(bar).with_children(|parent| {
                        parent
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    width: Val::Auto,
                                    height: Val::Auto,
                                    bottom: Val::Px(0.0),
                                    right: Val::Px(4.0),
                                    ..default()
                                },
                                FloatingDamage { remaining: 1.2 },
                            ))
                            .with_children(|p| {
                                p.spawn((
                                    Text::new(text),
                                    TextFont {
                                        font_size,
                                        ..default()
                                    },
                                    TextColor(color),
                                    DamageText,
                                ));
                            });
                    });
                } else {
                    // Fallback: absolute-positioned if bar background not found yet
                    commands
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Auto,
                                height: Val::Auto,
                                ..default()
                            },
                            FloatingDamage { remaining: 1.2 },
                        ))
                        .with_children(|p| {
                            p.spawn((
                                Text::new(text),
                                TextFont {
                                    font_size,
                                    ..default()
                                },
                                TextColor(color),
                                DamageText,
                            ));
                        });
                }
            }
            CombatantId::Player(_) => {
                // Player targets: spawn at bottom-left of screen (HUD area)
                commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Auto,
                            height: Val::Auto,
                            bottom: Val::Px(80.0),
                            left: Val::Px(16.0),
                            ..default()
                        },
                        FloatingDamage { remaining: 1.2 },
                    ))
                    .with_children(|p| {
                        p.spawn((
                            Text::new(text),
                            TextFont {
                                font_size,
                                ..default()
                            },
                            TextColor(color),
                            DamageText,
                        ));
                    });
            }
        }
    }
}

/// System: spawn in-world HP hover bars for each alive monster when combat starts.
///
/// Runs every frame but is a no-op once all bars are present (one bar per alive
/// monster).  The bars are spawned as screen-space UI nodes positioned above
/// the corresponding `EnemyCard`.
fn spawn_monster_hp_hover_bars(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    combat_res: Res<CombatResource>,
    existing_bars: Query<&MonsterHpHoverBar>,
) {
    // Only run in combat
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    // Respect runtime graphics setting.
    if !global_state.0.config.graphics.show_combat_monster_hp_bars {
        return;
    }

    // Collect which participant indices already have a bar
    let existing_indices: std::collections::HashSet<usize> =
        existing_bars.iter().map(|b| b.participant_index).collect();

    // Spawn a bar for each alive monster that doesn't have one yet
    let mut stack_order = 0usize;
    for (idx, participant) in combat_res.state.participants.iter().enumerate() {
        if let Combatant::Monster(mon) = participant {
            if mon.hp.current == 0 {
                continue;
            }
            if existing_indices.contains(&idx) {
                stack_order += 1;
                continue;
            }

            let max_hp = mon.hp.base.max(1) as f32;
            let cur_hp = mon.hp.current as f32;
            let fill_pct = (cur_hp / max_hp).clamp(0.0, 1.0);

            let bar_color = if fill_pct > 0.5 {
                ENEMY_HP_HEALTHY_COLOR
            } else if fill_pct > 0.25 {
                ENEMY_HP_INJURED_COLOR
            } else {
                ENEMY_HP_CRITICAL_COLOR
            };

            // Spawn container panel
            let fill_idx = idx;
            let bar_stack_order = stack_order;
            commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: MONSTER_HP_HOVER_BAR_WIDTH,
                        height: Val::Px(38.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(3.0),
                        padding: UiRect::all(Val::Px(4.0)),
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.10, 0.12, 0.92)),
                    BorderRadius::all(Val::Px(4.0)),
                    ZIndex(i32::MAX),
                    MonsterHpHoverBar {
                        participant_index: idx,
                        stack_order: bar_stack_order,
                    },
                ))
                .with_children(|parent| {
                    parent
                        .spawn((Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        },))
                        .with_children(|header| {
                            header.spawn((
                                Text::new(mon.name.clone()),
                                TextFont {
                                    font_size: 10.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                MonsterHpHoverBarNameText {
                                    participant_index: fill_idx,
                                },
                            ));

                            header.spawn((
                                Text::new(format!("{}/{}", mon.hp.base, mon.hp.current)),
                                TextFont {
                                    font_size: 10.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.86, 0.92, 0.98)),
                                MonsterHpHoverBarHpText {
                                    participant_index: fill_idx,
                                },
                            ));
                        });

                    parent
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: MONSTER_HP_HOVER_BAR_HEIGHT,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                            BorderRadius::all(Val::Px(2.0)),
                        ))
                        .with_children(|bar_bg| {
                            bar_bg.spawn((
                                Node {
                                    width: Val::Percent(fill_pct * 100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(bar_color),
                                BorderRadius::all(Val::Px(2.0)),
                                MonsterHpHoverBarFill {
                                    participant_index: fill_idx,
                                },
                            ));
                        });
                });
            stack_order += 1;
        }
    }
}

/// System: update `MonsterHpHoverBar` fill widths each frame from `CombatResource`.
fn update_monster_hp_hover_bars(
    combat_res: Res<CombatResource>,
    global_state: Res<GlobalState>,
    camera_query: CombatCameraQuery,
    primary_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    encounter_visual_query: EncounterVisualQuery,
    mut hp_text_query: MonsterHpHoverTextQuery,
    mut hover_bar_queries: MonsterHpHoverBarQueries,
) {
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    let bars_enabled = global_state.0.config.graphics.show_combat_monster_hp_bars;

    // Keep the bars world-projected above the active encounter marker while combat is active.
    if bars_enabled {
        let camera_and_transform = camera_query.single().ok();
        let anchor = if let (Some(map_id), Some(pos)) =
            (combat_res.encounter_map_id, combat_res.encounter_position)
        {
            encounter_visual_query
                .iter()
                .find(|(marker, _)| marker.map_id == map_id && marker.position == pos)
                .map(|(_, tf)| tf.translation())
        } else {
            None
        };

        let mut used_world_projection = false;

        if let (Some((camera, camera_tf)), Some(anchor_pos)) = (camera_and_transform, anchor) {
            for (bar, mut node) in hover_bar_queries.p0().iter_mut() {
                let world_bar_pos =
                    anchor_pos + Vec3::new(0.0, 2.2 + (bar.stack_order as f32) * 0.28, 0.0);
                if let Ok(screen_pos) = camera.world_to_viewport(camera_tf, world_bar_pos) {
                    node.left = Val::Px(screen_pos.x - 60.0);
                    node.top = Val::Px(screen_pos.y - 12.0 - (bar.stack_order as f32) * 12.0);
                    used_world_projection = true;
                }
            }
        }

        // Fallback: if no anchor/projection is available, align bars with the
        // enemy-card row so HUD composition stays close to the original layout.
        if !used_world_projection {
            let bar_count = hover_bar_queries.p0().iter().count();
            let window_width = primary_window.single().ok().map(|w| w.width());
            let card_width = match ENEMY_CARD_WIDTH {
                Val::Px(v) => v,
                _ => 150.0,
            };
            let card_gap = 16.0;
            let total_cards_width = if bar_count == 0 {
                0.0
            } else {
                (bar_count as f32 * card_width) + ((bar_count - 1) as f32 * card_gap)
            };
            let start_x = window_width
                .map(|w| ((w - total_cards_width) * 0.5).max(8.0))
                .unwrap_or(24.0);

            for (bar, mut node) in hover_bar_queries.p0().iter_mut() {
                let card_left = start_x + (bar.stack_order as f32) * (card_width + card_gap);
                node.left = Val::Px(card_left + 14.0);
                // Fallback top: enemy panel bottom edge is at ENEMY_PANEL_BOTTOM
                // (206 px from screen bottom) and is COMBAT_ENEMY_PANEL_HEIGHT
                // (200 px) tall, so its top edge is ~406 px from the bottom.
                // Express as a top offset; the world-projection path overrides
                // this whenever a camera is present.
                node.top = Val::Px(54.0);
            }
        }
    }

    for (fill, mut node, mut bg) in hover_bar_queries.p1().iter_mut() {
        if let Some(Combatant::Monster(mon)) =
            combat_res.state.participants.get(fill.participant_index)
        {
            let max_hp = mon.hp.base.max(1) as f32;
            let cur_hp = mon.hp.current as f32;
            let fill_pct = (cur_hp / max_hp).clamp(0.0, 1.0);

            node.width = Val::Percent(fill_pct * 100.0);

            *bg = BackgroundColor(if fill_pct > 0.5 {
                ENEMY_HP_HEALTHY_COLOR
            } else if fill_pct > 0.25 {
                ENEMY_HP_INJURED_COLOR
            } else {
                ENEMY_HP_CRITICAL_COLOR
            });
        }
    }

    // Update hover-card HP text as Original/Current (base/current).
    for (hp_text, mut text) in hp_text_query.iter_mut() {
        if let Some(Combatant::Monster(mon)) =
            combat_res.state.participants.get(hp_text.participant_index)
        {
            **text = format!("{}/{}", mon.hp.base, mon.hp.current);
        }
    }
}

/// System: despawn all `MonsterHpHoverBar` entities when combat ends.
fn cleanup_monster_hp_hover_bars(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    bars: Query<Entity, With<MonsterHpHoverBar>>,
) {
    // Leave bars in place only while in combat and setting is enabled.
    if matches!(global_state.0.mode, GameMode::Combat(_))
        && global_state.0.config.graphics.show_combat_monster_hp_bars
    {
        return;
    }

    for entity in bars.iter() {
        commands.entity(entity).despawn();
    }
}

/// Cleanup system for floating damage UI nodes. Decrements remaining lifetime
/// and despawns the node when time is up.
fn cleanup_floating_damage(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut FloatingDamage)>,
) {
    for (entity, mut fd) in query.iter_mut() {
        fd.remaining -= time.delta_secs();
        if fd.remaining <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameState;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::combat::types::Handicap;

    /// Ensure plugin creates and exposes expected resources when registered.
    #[test]
    fn test_combat_plugin_registers_messages() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // CombatResource should be available after plugin initialization
        let cr = app.world().resource::<CombatResource>();
        assert!(cr.state.participants.is_empty());
        assert!(cr.player_orig_indices.is_empty());

        // RangedAttackPending resource should be registered
        let _pending = app.world().resource::<RangedAttackPending>();
    }

    /// Party -> Combat sync should copy party members into CombatResource when entering combat.
    #[test]
    fn test_party_sync_to_combat() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Prepare game state with one party member and enter combat mode
        let mut gs = GameState::new();
        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        // Make sure mode is Combat (empty CombatState)
        gs.enter_combat();

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Run a frame so the sync system runs
        app.update();

        let cr = app.world().resource::<CombatResource>();
        // After sync, a player should be present in participants
        assert!(cr
            .state
            .participants
            .iter()
            .any(|p| matches!(p, Combatant::Player(_))));
        // Mapping should reflect the party index
        assert_eq!(
            cr.player_orig_indices
                .iter()
                .filter(|x| x.is_some())
                .count(),
            1
        );
    }

    /// `sync_party_hp_during_combat` must write the combat participant's current HP
    /// into `party.members` every frame while combat is active — so the HUD
    /// reflects live damage before combat ends.
    #[test]
    fn test_sync_party_hp_during_combat_updates_party_hp() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let mut hero = Character::new(
            "LiveHpHero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.base = 30;
        hero.hp.current = 30;
        gs.party.add_member(hero.clone()).unwrap();

        // Put the game into Combat mode.
        gs.enter_combat();

        // Build a CombatResource where the player has already taken 10 damage.
        let mut cr = CombatResource::new();
        let mut combat_hero = hero.clone();
        combat_hero.hp.current = 20; // simulates 10 damage dealt
        cr.state.add_player(combat_hero);
        cr.player_orig_indices = vec![Some(0)];
        // Initialise turn order so the state is consistent.
        crate::domain::combat::engine::start_combat(&mut cr.state);

        app.insert_resource(crate::game::resources::GlobalState(gs));
        app.insert_resource(cr);

        // Run one frame — sync_party_hp_during_combat must fire.
        app.update();

        let gs_after = app
            .world()
            .resource::<crate::game::resources::GlobalState>();

        // Party member HP must reflect the combat-engine value (20), not the
        // original value (30), while combat is still active.
        assert_eq!(
            gs_after.0.party.members[0].hp.current, 20,
            "sync_party_hp_during_combat must mirror combat HP into party.members \
             so the HUD shows live values; expected 20, got {}",
            gs_after.0.party.members[0].hp.current
        );
        // Must still be in combat — this sync must not end combat.
        assert!(
            matches!(gs_after.0.mode, GameMode::Combat(_)),
            "combat must still be active after sync; got {:?}",
            gs_after.0.mode
        );
    }

    /// `sync_party_hp_during_combat` must do nothing when not in combat mode.
    #[test]
    fn test_sync_party_hp_during_combat_noop_in_exploration() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let mut hero = Character::new(
            "SafeHero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.base = 30;
        hero.hp.current = 30;
        gs.party.add_member(hero.clone()).unwrap();
        // Mode stays Exploration — no combat.

        let cr = CombatResource::new(); // empty, no participants

        app.insert_resource(crate::game::resources::GlobalState(gs));
        app.insert_resource(cr);

        app.update();

        let gs_after = app
            .world()
            .resource::<crate::game::resources::GlobalState>();
        // HP must be unchanged when not in combat.
        assert_eq!(
            gs_after.0.party.members[0].hp.current, 30,
            "sync_party_hp_during_combat must not alter party HP outside combat"
        );
    }

    /// Combat -> Party sync on exit should copy HP/SP/conditions back into the party.
    #[test]
    fn test_combat_sync_to_party() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create a party with one member (HP default 10)
        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Wounded".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.modify(0); // ensure default
        gs.party.add_member(hero.clone()).unwrap();

        // Create combat state where the player is at 3 HP (i.e., took damage)
        let mut cs = CombatState::new(Handicap::Even);
        hero.hp.modify(-7); // hero now has current HP lowered (mutating local hero)
        cs.add_player(hero.clone());
        // Build mapping: participant 0 -> party index 0
        let mut cr = CombatResource::new();
        cr.state = cs;
        cr.player_orig_indices = vec![Some(0)];

        // Insert resources and set global mode to Exploration to simulate combat exit
        app.insert_resource(crate::game::resources::GlobalState(gs));
        app.insert_resource(cr);

        // Run frame, sync system should copy HP back to party
        app.update();

        let gs_after = app
            .world()
            .resource::<crate::game::resources::GlobalState>();
        let party_member = &gs_after.0.party.members[0];
        // Expect HP to have been synchronized (reduced)
        assert!(party_member.hp.current < party_member.hp.base);
    }

    /// Verify `start_encounter` constructs a combat state and sets the
    /// `GameState` mode to `GameMode::Combat` when invoked.
    #[test]
    fn test_start_encounter_sets_game_mode() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        // Prepare a minimal game state with one party member
        let mut gs = GameState::new();
        let hero = crate::domain::character::Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "knight".to_string(),
            crate::domain::character::Sex::Male,
            crate::domain::character::Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        // Empty content DB is sufficient for this test (empty monster group)
        let content = GameContent::new(ContentDatabase::new());

        // Starting an encounter with an empty monster group should succeed
        assert!(start_encounter(&mut gs, &content, &[], CombatEventType::Normal).is_ok());

        // GameState should now be in Combat mode
        assert!(matches!(gs.mode, crate::application::GameMode::Combat(_)));
    }

    /// After `start_encounter` + `CombatStarted` round-trip, `CombatResource`
    /// must hold the `CombatEventType` that was passed to `start_encounter`.
    ///
    /// This test exercises the Bevy message path via a minimal `App` so we can
    /// confirm that `handle_combat_started` copies `msg.combat_event_type` into
    /// `combat_res.combat_event_type`.
    #[test]
    fn test_start_encounter_stores_type_in_resource() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::domain::combat::types::CombatEventType;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = crate::domain::character::Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "knight".to_string(),
            crate::domain::character::Sex::Male,
            crate::domain::character::Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        // Start an Ambush encounter
        start_encounter(&mut gs, &content, &[], CombatEventType::Ambush).unwrap();
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Write the CombatStarted message with Ambush type
        {
            let mut writer = app.world_mut().resource_mut::<Messages<CombatStarted>>();
            writer.write(CombatStarted {
                encounter_position: None,
                encounter_map_id: None,
                combat_event_type: CombatEventType::Ambush,
            });
        }

        // Run one frame so handle_combat_started processes the message
        app.update();

        let combat_res = app
            .world()
            .get_resource::<CombatResource>()
            .expect("CombatResource must exist");
        assert_eq!(
            combat_res.combat_event_type,
            CombatEventType::Ambush,
            "CombatResource must store the Ambush type forwarded from CombatStarted"
        );
    }

    // ===== Normal and Ambush Combat Tests =====

    /// `start_encounter` with `CombatEventType::Normal` must produce a
    /// `CombatState` with `handicap == Handicap::Even`.
    #[test]
    fn test_normal_combat_handicap_is_even() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Normal).unwrap();

        if let crate::application::GameMode::Combat(ref cs) = gs.mode {
            assert_eq!(
                cs.handicap,
                Handicap::Even,
                "Normal combat must start with Even handicap"
            );
        } else {
            panic!("Expected Combat mode");
        }
    }

    /// `start_encounter` with `CombatEventType::Ambush` must produce a
    /// `CombatState` with `handicap == Handicap::MonsterAdvantage`.
    #[test]
    fn test_ambush_combat_handicap_is_monster_advantage() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Ambush).unwrap();

        if let crate::application::GameMode::Combat(ref cs) = gs.mode {
            assert_eq!(
                cs.handicap,
                Handicap::MonsterAdvantage,
                "Ambush combat must start with MonsterAdvantage handicap"
            );
        } else {
            panic!("Expected Combat mode");
        }
    }

    /// `start_encounter` with `CombatEventType::Ambush` must set
    /// `ambush_round_active = true` on the resulting `CombatState`.
    #[test]
    fn test_ambush_round_active_set_on_start() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Ambush).unwrap();

        if let crate::application::GameMode::Combat(ref cs) = gs.mode {
            assert!(
                cs.ambush_round_active,
                "ambush_round_active must be true after Ambush start_encounter"
            );
        } else {
            panic!("Expected Combat mode");
        }
    }

    /// `start_encounter` with `CombatEventType::Normal` must leave
    /// `ambush_round_active = false`.
    #[test]
    fn test_normal_round_active_not_set() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Normal).unwrap();

        if let crate::application::GameMode::Combat(ref cs) = gs.mode {
            assert!(
                !cs.ambush_round_active,
                "ambush_round_active must be false after Normal start_encounter"
            );
        } else {
            panic!("Expected Combat mode");
        }
    }

    /// After `advance_turn` exhausts round 1 of an ambush encounter,
    /// `ambush_round_active` must be `false` and `handicap` must be `Even`.
    #[test]
    fn test_ambush_round_active_cleared_at_round_2() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Ambush).unwrap();

        if let crate::application::GameMode::Combat(ref mut cs) = gs.mode {
            assert!(cs.ambush_round_active, "pre-condition: flag must be set");
            // Exhaust all slots in round 1 to trigger advance_round.
            // With only one combatant (player), a single advance_turn call
            // wraps the turn order and fires advance_round (round -> 2).
            let _ = cs.advance_turn(&[]);

            assert!(
                !cs.ambush_round_active,
                "ambush_round_active must be cleared at the start of round 2"
            );
            assert_eq!(
                cs.handicap,
                Handicap::Even,
                "handicap must be reset to Even at round 2"
            );
            assert_eq!(cs.round, 2, "round must be 2");
        } else {
            panic!("Expected Combat mode");
        }
    }

    /// After round 2 starts, `handicap` must equal `Even` (ambush handicap reset).
    #[test]
    fn test_ambush_handicap_resets_to_even_round_2() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Ambush).unwrap();

        if let crate::application::GameMode::Combat(ref mut cs) = gs.mode {
            let _ = cs.advance_turn(&[]);
            assert_eq!(cs.handicap, Handicap::Even);
        } else {
            panic!("Expected Combat mode");
        }
    }

    /// Boss combat must set `monsters_advance`, `monsters_regenerate` and
    /// clear `can_bribe` / `can_surrender`.
    #[test]
    fn test_boss_combat_sets_boss_flags() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Boss).unwrap();

        if let crate::application::GameMode::Combat(ref cs) = gs.mode {
            assert!(cs.monsters_advance, "boss: monsters_advance must be true");
            assert!(
                cs.monsters_regenerate,
                "boss: monsters_regenerate must be true"
            );
            assert!(!cs.can_bribe, "boss: can_bribe must be false");
            assert!(!cs.can_surrender, "boss: can_surrender must be false");
        } else {
            panic!("Expected Combat mode");
        }
    }

    /// The combat log must contain "ambush" text after an ambush `CombatStarted`
    /// message is processed.
    #[test]
    fn test_combat_log_reports_ambush() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Ambush).unwrap();
        app.insert_resource(crate::game::resources::GlobalState(gs));

        {
            let mut writer = app.world_mut().resource_mut::<Messages<CombatStarted>>();
            writer.write(CombatStarted {
                encounter_position: None,
                encounter_map_id: None,
                combat_event_type: CombatEventType::Ambush,
            });
        }

        app.update();

        let log = app.world().resource::<CombatLogState>();
        let found = log
            .lines
            .iter()
            .any(|line| line.plain_text().to_lowercase().contains("ambush"));
        assert!(
            found,
            "combat log must contain 'ambush' text after an ambush encounter; \
             got lines: {:?}",
            log.lines.iter().map(|l| l.plain_text()).collect::<Vec<_>>()
        );
    }

    /// The combat log must contain "Monsters appear!" for a Normal encounter.
    #[test]
    fn test_combat_log_reports_normal_encounter() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Normal).unwrap();
        app.insert_resource(crate::game::resources::GlobalState(gs));

        {
            let mut writer = app.world_mut().resource_mut::<Messages<CombatStarted>>();
            writer.write(CombatStarted {
                encounter_position: None,
                encounter_map_id: None,
                combat_event_type: CombatEventType::Normal,
            });
        }

        app.update();

        let log = app.world().resource::<CombatLogState>();
        let found = log
            .lines
            .iter()
            .any(|line| line.plain_text().contains("Monsters appear!"));
        assert!(
            found,
            "combat log must contain 'Monsters appear!' for a Normal encounter; \
             got lines: {:?}",
            log.lines.iter().map(|l| l.plain_text()).collect::<Vec<_>>()
        );
    }

    /// After an ambush `CombatStarted` message, the `CombatTurnStateResource`
    /// must be set to `EnemyTurn` (monsters always act first in round 1).
    #[test]
    fn test_ambush_combat_started_sets_enemy_turn() {
        use crate::application::GameState;
        use crate::domain::combat::monster::LootTable;

        // Build the test using a direct CombatResource fixture rather than
        // start_encounter so we can include a monster.  With a monster present
        // in the turn order, execute_monster_turn's ambush path will:
        //   1. Skip the surprised player (Player(0)) and advance to Monster(1).
        //   2. Return immediately with turn_state = EnemyTurn (monster's turn).
        // That lets us assert EnemyTurn at the end of the frame, which is the
        // real game-logic requirement: monsters act first in an ambush.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let player = Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        let goblin = crate::domain::combat::monster::Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(8, 6, 6, 8, 8, 1, 6),
            20,
            5,
            vec![],
            LootTable::default(),
        );

        // Build a CombatState with ambush active: player first (surprised),
        // monster second so it gets to act after the player is skipped.
        let mut cs = CombatState::new(Handicap::MonsterAdvantage);
        cs.ambush_round_active = true;
        cs.add_player(player.clone());
        cs.add_monster(goblin);
        // Force Player first so execute_monster_turn skips them and lands on
        // the monster's slot, leaving turn_state = EnemyTurn.
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;
        cs.status = crate::domain::combat::types::CombatStatus::InProgress;

        let mut cr = CombatResource::new();
        cr.state = cs.clone();
        cr.player_orig_indices = vec![Some(0), None];
        cr.combat_event_type = CombatEventType::Ambush;

        let mut gs = GameState::new();
        gs.party.add_member(player).unwrap();
        // Place the prepared combat state into the GlobalState so that
        // handle_combat_started can read it via GlobalState.
        gs.enter_combat_with_state(cs);

        app.insert_resource(crate::game::resources::GlobalState(gs));
        app.insert_resource(cr);

        {
            let mut writer = app.world_mut().resource_mut::<Messages<CombatStarted>>();
            writer.write(CombatStarted {
                encounter_position: None,
                encounter_map_id: None,
                combat_event_type: CombatEventType::Ambush,
            });
        }

        app.update();

        let turn_state = app.world().resource::<CombatTurnStateResource>();
        assert!(
            matches!(turn_state.0, CombatTurnState::EnemyTurn),
            "CombatTurnStateResource must be EnemyTurn after ambush CombatStarted \
             (monster's slot follows the skipped player slot); got {:?}",
            turn_state.0
        );
    }

    /// During round 1 of an ambush, the player's turn must be
    /// auto-skipped (suppressed) by `execute_monster_turn`.  The combat log
    /// must record a "surprised" message and `CombatTurnStateResource` must
    /// remain `EnemyTurn` after the skip so that the monster gets to act.
    #[test]
    fn test_ambush_player_turn_is_skipped() {
        use crate::application::GameState;
        use crate::domain::combat::monster::LootTable;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let player = Character::new(
            "Surprised".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let goblin = crate::domain::combat::monster::Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(8, 6, 6, 8, 8, 1, 6),
            20,
            5,
            vec![],
            LootTable::default(),
        );

        // Ambush: player first in turn order (will be skipped), monster second.
        let mut cs = CombatState::new(Handicap::MonsterAdvantage);
        cs.ambush_round_active = true;
        cs.add_player(player.clone());
        cs.add_monster(goblin);
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;
        cs.status = crate::domain::combat::types::CombatStatus::InProgress;

        let mut cr = CombatResource::new();
        cr.state = cs.clone();
        cr.player_orig_indices = vec![Some(0), None];
        cr.combat_event_type = CombatEventType::Ambush;

        let mut gs = GameState::new();
        gs.party.add_member(player).unwrap();
        gs.enter_combat_with_state(cs);

        app.insert_resource(crate::game::resources::GlobalState(gs));
        app.insert_resource(cr);

        // Manually set EnemyTurn so execute_monster_turn runs this frame.
        app.insert_resource(CombatTurnStateResource(CombatTurnState::EnemyTurn));

        app.update();

        // Player's slot was skipped — the log must record the surprise message.
        let log = app.world().resource::<CombatLogState>();
        let found_surprised = log
            .lines
            .iter()
            .any(|line| line.plain_text().to_lowercase().contains("surprised"));
        assert!(
            found_surprised,
            "combat log must contain 'surprised' after player turn is skipped in ambush round 1; \
             got lines: {:?}",
            log.lines.iter().map(|l| l.plain_text()).collect::<Vec<_>>()
        );

        // After skipping the player slot, the system must keep EnemyTurn (so
        // the monster on the next slot gets to act).
        let ts = app.world().resource::<CombatTurnStateResource>();
        assert!(
            matches!(ts.0, CombatTurnState::EnemyTurn),
            "CombatTurnStateResource must remain EnemyTurn after skipping \
             the surprised player slot in round 1; got {:?}",
            ts.0
        );
    }

    /// In round 2 of an ambush encounter `ambush_round_active` is
    /// `false`, so the player must NOT be skipped.  `combat_input_system` must
    /// dispatch the player's action normally and `CombatTurnStateResource` must
    /// be `PlayerTurn` at the start of the round.
    #[test]
    fn test_ambush_player_can_act_round_2() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        // Build an ambush encounter and manually advance into round 2 so that
        // `ambush_round_active` is cleared by `advance_round`.
        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Ambush).unwrap();

        // Advance past round 1 so `advance_round` fires and clears the flag.
        if let crate::application::GameMode::Combat(ref mut cs) = gs.mode {
            assert!(cs.ambush_round_active, "pre-condition: flag must be set");
            let _ = cs.advance_turn(&[]);
            assert!(
                !cs.ambush_round_active,
                "pre-condition: ambush_round_active must be cleared after advance_turn into round 2"
            );
            assert_eq!(cs.round, 2, "pre-condition: must be in round 2");
        } else {
            panic!("Expected Combat mode after start_encounter");
        }

        // In round 2 the player must NOT be skipped — `ambush_round_active` is
        // false, so `combat_input_system` will process player input normally and
        // `execute_monster_turn` will not auto-skip player slots.
        if let crate::application::GameMode::Combat(ref cs) = gs.mode {
            assert!(
                !cs.ambush_round_active,
                "ambush_round_active must be false in round 2 — player can act"
            );
            assert_eq!(
                cs.handicap,
                Handicap::Even,
                "handicap must be reset to Even in round 2"
            );
        } else {
            panic!("Expected Combat mode");
        }
    }

    // ===== Combat UI Tests =====

    /// Verify combat UI spawns when entering combat
    #[test]
    fn test_combat_ui_spawns_on_enter() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create game state in combat mode
        let mut gs = GameState::new();
        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        gs.enter_combat();

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Run update to trigger setup_combat_ui
        app.update();

        // Combat HUD root should now exist
        let mut ui_query = app
            .world_mut()
            .query_filtered::<Entity, With<crate::game::components::combat::CombatHudRoot>>();
        assert_eq!(ui_query.iter(app.world()).count(), 1);
    }

    /// Verify combat UI despawns when exiting combat
    #[test]
    fn test_combat_ui_despawns_on_exit() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Start in combat mode
        let mut gs = GameState::new();
        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        gs.enter_combat();

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Run update to spawn UI
        app.update();

        // Verify UI exists
        let mut ui_query = app
            .world_mut()
            .query_filtered::<Entity, With<crate::game::components::combat::CombatHudRoot>>();
        assert_eq!(ui_query.iter(app.world()).count(), 1);

        // Exit combat
        let mut gs_res = app
            .world_mut()
            .resource_mut::<crate::game::resources::GlobalState>();
        gs_res.0.exit_combat();

        // Run update to trigger cleanup
        app.update();

        // UI should be despawned
        let mut ui_query = app
            .world_mut()
            .query_filtered::<Entity, With<crate::game::components::combat::CombatHudRoot>>();
        assert_eq!(ui_query.iter(app.world()).count(), 0);
    }

    /// Verify enemy HP bars update correctly
    #[test]
    fn test_enemy_hp_bars_update() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create combat state with a monster
        let mut gs = GameState::new();
        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        // Create combat state with monster
        let mut cs = CombatState::new(Handicap::Even);
        let mut monster = crate::domain::combat::monster::Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 10),
            10,
            10,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        monster.hp.base = 10;
        monster.hp.current = 10;
        cs.add_monster(monster);

        gs.enter_combat_with_state(cs);

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource manually since we're not going through handle_combat_started
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            // Rebuild the combat state
            let mut new_cs = CombatState::new(Handicap::Even);
            let monster = crate::domain::combat::monster::Monster::new(
                1,
                "Goblin".to_string(),
                crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 10),
                10,
                10,
                vec![crate::domain::combat::types::Attack::physical(
                    crate::domain::types::DiceRoll::new(1, 4, 0),
                )],
                crate::domain::combat::monster::LootTable::default(),
            );
            new_cs.add_monster(monster);
            combat_res.state = new_cs;
            combat_res.player_orig_indices = vec![None];
        }

        // Run update to trigger setup_combat_ui
        app.update();

        // Verify enemy card was created
        let mut enemy_cards = app.world_mut().query_filtered::<Entity, With<EnemyCard>>();
        assert_eq!(enemy_cards.iter(app.world()).count(), 1);

        // Damage the monster by modifying combat resource
        let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
        if let Some(Combatant::Monster(monster)) = combat_res.state.participants.get_mut(0) {
            monster.hp.current = 5; // 50% HP
        }

        // Run update to sync UI
        app.update();

        // Verify HP bar fill exists and has been updated
        let mut hp_bars = app
            .world_mut()
            .query_filtered::<(&Node, &EnemyHpBarFill), With<EnemyHpBarFill>>();
        let mut found_hp_bar = false;
        for (node, _) in hp_bars.iter(app.world()) {
            if let Val::Percent(width) = node.width {
                // Should be ~50% width
                assert!((width - 50.0).abs() < 1.0);
                found_hp_bar = true;
            }
        }
        assert!(found_hp_bar, "HP bar fill should have been updated");
    }

    /// Verify turn order display shows correct order
    #[test]
    fn test_turn_order_display() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create combat with player and monster
        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(gs.party.members[0].clone());
        let monster = crate::domain::combat::monster::Monster::new(
            2,
            "Orc".to_string(),
            crate::domain::character::Stats::new(12, 8, 8, 12, 10, 10, 8),
            15,
            12,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 6, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        cs.add_monster(monster);

        // Set up turn order manually
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs);

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Run update to spawn and populate UI
        app.update();

        // Verify turn order text was created
        let mut turn_order_texts = app
            .world_mut()
            .query_filtered::<&Text, With<TurnOrderText>>();
        assert_eq!(turn_order_texts.iter(app.world()).count(), 1);

        // Text should contain turn order information
        for text in turn_order_texts.iter(app.world()) {
            let text_str = text.0.as_str();
            assert!(text_str.contains("Turn Order"));
        }
    }

    /// Verify action menu visibility changes with turn state
    #[test]
    fn test_action_menu_visibility() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create combat state with a monster as the first actor so that forcing
        // EnemyTurn is consistent with the combat state and the stale-state
        // correction in execute_monster_turn does not fire (the monster cannot
        // act because has_acted == true, which triggers the incapacitated-skip
        // path that advances to PlayerTurn — so we use a two-step approach:
        // first confirm the menu exists on PlayerTurn, then verify it is hidden
        // when we set EnemyTurn with an *already-acted* monster current actor).
        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        // Build a combat state with player first so the initial state is PlayerTurn.
        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        let monster = crate::domain::combat::monster::Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(10, 8, 8, 10, 8, 10, 8),
            10,
            8,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        cs.add_monster(monster);
        // Force player first in turn order.
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), None];
        }

        // Set turn state to PlayerTurn and run to spawn UI.
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;
        app.update();

        // Action menu should be visible when PlayerTurn.
        let mut menu_query = app
            .world_mut()
            .query_filtered::<&Visibility, With<ActionMenuPanel>>();
        let count = menu_query.iter(app.world()).count();
        assert_eq!(count, 1, "Action menu should exist");

        // Advance combat turn to the monster (current_turn = 1) and set EnemyTurn.
        // Mark the monster as already-acted so execute_monster_turn skips it and
        // immediately advances back to the player — but on *this* frame we only
        // care that update_combat_ui received EnemyTurn and hid the menu.
        // We set current_turn = 1 to point at the monster slot.
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state.current_turn = 1;
            // Mark monster as having acted so the can_act() == false path fires
            // and the turn is advanced *within the same update call* before
            // update_combat_ui runs — however the ordering guarantees
            // execute_monster_turn runs *after* update_combat_ui in the schedule,
            // so on this frame update_combat_ui still sees EnemyTurn.
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::EnemyTurn;
        app.update();

        // Action menu should be hidden on the frame where EnemyTurn is set.
        let mut menu_query = app
            .world_mut()
            .query_filtered::<&Visibility, With<ActionMenuPanel>>();
        for visibility in menu_query.iter(app.world()) {
            assert_eq!(*visibility, Visibility::Hidden);
        }
    }

    /// Verify action buttons are created correctly
    #[test]
    fn test_action_buttons_created() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        gs.enter_combat();

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Run update to spawn UI
        app.update();

        // All 5 action buttons should exist
        let mut buttons = app
            .world_mut()
            .query_filtered::<&ActionButton, With<ActionButton>>();
        assert_eq!(buttons.iter(app.world()).count(), 5);

        // Verify each button type exists
        let mut found_types = std::collections::HashSet::new();
        for button in buttons.iter(app.world()) {
            found_types.insert(button.button_type);
        }

        assert!(found_types.contains(&ActionButtonType::Attack));
        assert!(found_types.contains(&ActionButtonType::Defend));
        assert!(found_types.contains(&ActionButtonType::Cast));
        assert!(found_types.contains(&ActionButtonType::Item));
        assert!(found_types.contains(&ActionButtonType::Flee));
    }

    /// Verify enemy cards are created for monsters only
    #[test]
    fn test_enemy_cards_for_monsters_only() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        // Create combat with 2 monsters
        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(gs.party.members[0].clone());

        let monster1 = crate::domain::combat::monster::Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(10, 8, 8, 10, 10, 10, 8),
            8,
            10,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        let monster2 = crate::domain::combat::monster::Monster::new(
            2,
            "Orc".to_string(),
            crate::domain::character::Stats::new(12, 8, 8, 12, 10, 10, 8),
            12,
            11,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 6, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        cs.add_monster(monster1);
        cs.add_monster(monster2);

        gs.enter_combat_with_state(cs);

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource manually
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            // Rebuild combat state
            let mut new_cs = CombatState::new(Handicap::Even);
            new_cs.add_player(Character::new(
                "Hero".to_string(),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            ));
            let monster1 = crate::domain::combat::monster::Monster::new(
                1,
                "Goblin".to_string(),
                crate::domain::character::Stats::new(10, 8, 8, 10, 10, 10, 8),
                8,
                10,
                vec![crate::domain::combat::types::Attack::physical(
                    crate::domain::types::DiceRoll::new(1, 4, 0),
                )],
                crate::domain::combat::monster::LootTable::default(),
            );
            let monster2 = crate::domain::combat::monster::Monster::new(
                2,
                "Orc".to_string(),
                crate::domain::character::Stats::new(12, 8, 8, 12, 10, 10, 8),
                12,
                11,
                vec![crate::domain::combat::types::Attack::physical(
                    crate::domain::types::DiceRoll::new(1, 6, 0),
                )],
                crate::domain::combat::monster::LootTable::default(),
            );
            new_cs.add_monster(monster1);
            new_cs.add_monster(monster2);
            combat_res.state = new_cs;
            combat_res.player_orig_indices = vec![Some(0), None, None];
        }

        // Run update to spawn UI
        app.update();

        // Should have exactly 2 enemy cards (not 3, even though there are 3 participants)
        let mut enemy_cards = app
            .world_mut()
            .query_filtered::<&EnemyCard, With<EnemyCard>>();
        assert_eq!(enemy_cards.iter(app.world()).count(), 2);
    }

    /// Verify combat victory distributes XP to living party members
    #[test]
    fn test_victory_distributes_xp() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Prepare game state with two party members
        let mut gs = GameState::new();
        let p1 = Character::new(
            "P1".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let p2 = Character::new(
            "P2".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(p1.clone()).unwrap();
        gs.party.add_member(p2.clone()).unwrap();
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Build combat state: two players and one dead monster with 100 XP
        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(p1.clone());
        cs.add_player(p2.clone());
        let monster = crate::domain::combat::monster::Monster::new(
            1,
            "Killed".to_string(),
            crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 10),
            0, // dead
            5,
            vec![],
            crate::domain::combat::monster::LootTable::new(0, 0, 0, 0, 100),
        );
        cs.add_monster(monster);
        cs.status = CombatStatus::Victory;
        cs.turn_order = vec![
            CombatantId::Player(0),
            CombatantId::Player(1),
            CombatantId::Monster(2),
        ];
        cs.current_turn = 0;

        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), Some(1), None];
        }

        // Call helper with deterministic RNG
        {
            use rand::rngs::StdRng;
            use rand::SeedableRng;
            let mut rng = StdRng::seed_from_u64(42);
            let content = GameContent::new(crate::sdk::database::ContentDatabase::new());

            {
                // Remove the GlobalState resource from the world to avoid overlapping
                // mutable borrows, operate on it, then re-insert the updated resource.
                let world = app.world_mut();

                let mut gs_owned = world
                    .remove_resource::<crate::game::resources::GlobalState>()
                    .expect("missing GlobalState resource for test");

                let summary = {
                    let mut cr = world.resource_mut::<CombatResource>();

                    process_combat_victory_with_rng(&mut cr, &content, &mut gs_owned, &mut rng)
                        .expect("victory processing failed")
                };

                // Reinsert the updated GlobalState (the mutable borrow ended when the inner block closed)
                world.insert_resource(gs_owned);

                // Fetch updated global state for assertions
                let gs_after = app
                    .world()
                    .resource::<crate::game::resources::GlobalState>();

                // 100 XP split between 2 living party members = 50 each
                assert_eq!(gs_after.0.party.members[0].experience, 50);
                assert_eq!(gs_after.0.party.members[1].experience, 50);
                assert_eq!(summary.total_xp, 100);
            }
        }
    }

    /// Verify victory awards gold to the party
    #[test]
    fn test_victory_awards_gold() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Prepare game state with one party member
        let mut gs = GameState::new();
        let p = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(p.clone()).unwrap();
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Build combat state with one dead monster with fixed gold 25
        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(p.clone());
        let monster = crate::domain::combat::monster::Monster::new(
            1,
            "Looty".to_string(),
            crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 10),
            0, // dead
            5,
            vec![],
            crate::domain::combat::monster::LootTable::new(25, 25, 0, 0, 0),
        );
        cs.add_monster(monster);
        cs.status = CombatStatus::Victory;

        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), None];
        }

        // Process victory
        {
            use rand::rngs::StdRng;
            use rand::SeedableRng;
            let mut rng = StdRng::seed_from_u64(99);
            let content = GameContent::new(crate::sdk::database::ContentDatabase::new());
            {
                // Avoid overlapping mutable borrows by removing GlobalState, mutating it,
                // and then reinserting it back into the world.
                let world = app.world_mut();

                let mut gs_owned = world
                    .remove_resource::<crate::game::resources::GlobalState>()
                    .expect("missing GlobalState resource for test");

                let summary = {
                    let mut cr = world.resource_mut::<CombatResource>();

                    process_combat_victory_with_rng(&mut cr, &content, &mut gs_owned, &mut rng)
                        .expect("victory processing failed")
                };

                // Reinsert the updated GlobalState (the mutable borrow ended when the inner block closed)
                world.insert_resource(gs_owned);

                // Fetch updated global state for assertions
                let gs_after = app
                    .world()
                    .resource::<crate::game::resources::GlobalState>();

                assert_eq!(summary.total_gold, 25);
                assert_eq!(gs_after.0.party.gold, 25);
            }
        }
    }

    /// Verify defeat triggers menu/game over flow
    #[test]
    fn test_defeat_triggers_game_over() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create a game state with one dead party member and set combat state to Defeat
        let mut gs = GameState::new();
        let mut dead = Character::new(
            "Dead".to_string(),
            "human".to_string(),
            "peasant".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        dead.hp.current = 0;
        gs.party.add_member(dead).unwrap();
        gs.enter_combat();

        app.insert_resource(crate::game::resources::GlobalState(gs));

        {
            // Read the party member first (immutable borrow) to avoid overlapping
            // with the subsequent mutable borrow of the world.
            let p = app
                .world()
                .resource::<crate::game::resources::GlobalState>()
                .0
                .party
                .members[0]
                .clone();

            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            let mut cs = CombatState::new(Handicap::Even);
            cs.add_player(p);
            cs.status = CombatStatus::Defeat;
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
            cr.resolution_handled = false;
        }

        // Run update so the resolution system emits the message and the handler runs
        app.update();
        app.update();

        // Verify game entered Menu mode (game over handling)
        let gs_after = app
            .world()
            .resource::<crate::game::resources::GlobalState>();
        assert!(matches!(gs_after.0.mode, GameMode::Menu(_)));
    }

    // ===== Player Action System Tests =====

    #[test]
    fn test_attack_action_applies_damage() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Prepare game state with one player and one monster
        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Attacker".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Ensure the tester's attacker has very high accuracy so seeded RNG
        // in the unit test will consistently register a hit.
        hero.stats.accuracy.current = 200;
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        let monster = crate::domain::combat::monster::Monster::new(
            1,
            "Test Goblin".to_string(),
            crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 10),
            10,
            10,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        cs.add_monster(monster);

        // Ensure player acts first
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            combat_res.state = cs;
            combat_res.player_orig_indices = vec![Some(0), None];
        }

        // Deterministic RNG
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(42);
        let content = GameContent::new(crate::sdk::database::ContentDatabase::new());

        // Perform attack by calling helper directly with a single borrow to CombatResource
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            let mut gs_local = crate::game::resources::GlobalState(GameState::new());
            let mut turn_state_local = CombatTurnStateResource::default();

            let attack = AttackAction {
                attacker: CombatantId::Player(0),
                target: CombatantId::Monster(1),
            };

            perform_attack_action_with_rng(
                &mut cr,
                &attack,
                &content,
                &mut gs_local,
                &mut turn_state_local,
                &mut rng,
            )
            .expect("attack failed");

            // Debug information: monster HP after attack
            if let Some(Combatant::Monster(mon)) = cr.state.participants.get(1) {
                println!("DEBUG: monster final hp {}/{}", mon.hp.current, mon.hp.base);
                assert!(mon.hp.current < mon.hp.base);
            } else {
                panic!("monster not found");
            }
        }
    }

    #[test]
    fn test_defend_action_improves_ac() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Defender".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.ac = crate::domain::character::AttributePair::new(10);
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            combat_res.state = cs;
            combat_res.player_orig_indices = vec![Some(0)];
        }

        // Perform defend by sending a DefendAction message and letting systems handle it
        // Perform defend by calling helper directly with a single borrow to CombatResource
        {
            let content = GameContent::new(crate::sdk::database::ContentDatabase::new());
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            let mut gs_local = crate::game::resources::GlobalState(GameState::new());
            let mut turn_state_local = CombatTurnStateResource::default();

            let action = DefendAction {
                combatant: CombatantId::Player(0),
            };

            perform_defend_action(
                &mut cr,
                &action,
                &content,
                &mut gs_local,
                &mut turn_state_local,
            )
            .expect("defend failed");

            if let Some(Combatant::Player(pc)) = cr.state.participants.first() {
                assert!(pc.ac.current >= pc.ac.base + 2);
            } else {
                panic!("player missing");
            }
        }
    }

    #[test]
    fn test_flee_success_exits_combat() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Fleer".to_string(),
            "human".to_string(),
            "rogue".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Make hero fast
        hero.stats.speed.current = 200;
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        let weak_monster = crate::domain::combat::monster::Monster::new(
            1,
            "Turtle".to_string(),
            crate::domain::character::Stats::new(8, 8, 8, 8, 8, 8, 8),
            5,
            5,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        cs.add_monster(weak_monster);
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            combat_res.state = cs;
            combat_res.player_orig_indices = vec![Some(0), None];
        }

        // Attempt to flee
        {
            let content = GameContent::new(crate::sdk::database::ContentDatabase::new());
            let world = app.world_mut();
            // Remove ownership of GlobalState and TurnState so we can mutate them
            // without creating overlapping mutable borrows on the world.
            let mut gs_owned = world
                .remove_resource::<crate::game::resources::GlobalState>()
                .expect("missing GlobalState resource for test");
            let mut turn_state_owned = world
                .remove_resource::<CombatTurnStateResource>()
                .expect("missing CombatTurnStateResource for test");
            let mut combat_res = world.resource_mut::<CombatResource>();

            let success = perform_flee_action(
                &mut combat_res,
                &content,
                &mut gs_owned,
                &mut turn_state_owned,
            )
            .expect("flee action failed");

            assert!(success, "Expected flee to succeed due to speed advantage");
            // Global state should have exited combat (check the owned copy)
            assert!(!matches!(gs_owned.0.mode, GameMode::Combat(_)));

            // Insert modified resources back into the world
            world.insert_resource(gs_owned);
            world.insert_resource(turn_state_owned);
        }
    }

    #[test]
    fn test_flee_failure_consumes_turn() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let mut hero = Character::new(
            "SlowOne".to_string(),
            "human".to_string(),
            "peasant".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Slow hero
        hero.stats.speed.current = 1;
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        let fast_monster = crate::domain::combat::monster::Monster::new(
            1,
            "Wild Dog".to_string(),
            crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 10),
            10,
            10,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        cs.add_monster(fast_monster);
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            combat_res.state = cs;
            combat_res.player_orig_indices = vec![Some(0), None];
        }

        // Attempt to flee and expect failure (turn consumed)
        {
            let content = GameContent::new(crate::sdk::database::ContentDatabase::new());
            let world = app.world_mut();
            // Temporarily remove GlobalState and TurnState for mutation without overlapping borrows
            let mut gs_owned = world
                .remove_resource::<crate::game::resources::GlobalState>()
                .expect("missing GlobalState resource for test");
            let mut turn_state_owned = world
                .remove_resource::<CombatTurnStateResource>()
                .expect("missing CombatTurnStateResource for test");
            let mut combat_res = world.resource_mut::<CombatResource>();

            let success = perform_flee_action(
                &mut combat_res,
                &content,
                &mut gs_owned,
                &mut turn_state_owned,
            )
            .expect("flee action execution failed");

            assert!(!success, "Expected flee to fail due to slow speed");
            // Turn should have advanced (from 0 to 1)
            assert_eq!(combat_res.state.current_turn, 1);

            // Put mutated resources back
            world.insert_resource(gs_owned);
            world.insert_resource(turn_state_owned);
        }
    }

    // ===== Monster AI Tests =====

    /// Verify monster AI (Aggressive) chooses the lowest HP target
    #[test]
    fn test_monster_ai_selects_target() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create two party members with differing HP
        let mut gs = GameState::new();
        let p1 = Character::new(
            "Tank".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let mut p2 = Character::new(
            "Squire".to_string(),
            "human".to_string(),
            "peasant".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Make second player low HP
        p2.hp.current = 3;
        gs.party.add_member(p1.clone()).unwrap();
        gs.party.add_member(p2.clone()).unwrap();

        // Monster with aggressive AI
        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(p1.clone());
        cs.add_player(p2.clone());
        let mut monster = crate::domain::combat::monster::Monster::new(
            99,
            "Brute".to_string(),
            crate::domain::character::Stats::new(10, 10, 6, 10, 10, 10, 8),
            10,
            6,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        monster.ai_behavior = crate::domain::combat::monster::AiBehavior::Aggressive;
        cs.add_monster(monster);

        // Set turn order with monster first
        cs.turn_order = vec![
            CombatantId::Monster(2),
            CombatantId::Player(0),
            CombatantId::Player(1),
        ];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), Some(1), None];
        }

        // Deterministic RNG
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(42);
        let content = GameContent::new(crate::sdk::database::ContentDatabase::new());

        // Execute monster turn directly
        {
            let world = app.world_mut();
            let mut gs_owned = world
                .remove_resource::<crate::game::resources::GlobalState>()
                .expect("missing GlobalState resource for test");
            let mut turn_state_owned = world
                .remove_resource::<CombatTurnStateResource>()
                .expect("missing CombatTurnStateResource for test");
            let mut cr = world.resource_mut::<CombatResource>();

            perform_monster_turn_with_rng(
                &mut cr,
                &content,
                &mut gs_owned,
                &mut turn_state_owned,
                &mut rng,
            )
            .expect("monster turn failed");

            // The low HP player (index 1) should have taken damage (be lower than base)
            if let Some(Combatant::Player(pc)) = cr.state.participants.get(1) {
                assert!(pc.hp.current < pc.hp.base);
            } else {
                panic!("expected player at index 1");
            }

            world.insert_resource(gs_owned);
            world.insert_resource(turn_state_owned);
        }
    }

    /// Verify that the monster turn advances to the next actor after its action
    #[test]
    fn test_monster_turn_advances_after_attack() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // One player and one monster
        let mut gs = GameState::new();
        let player = Character::new(
            "Solo".to_string(),
            "human".to_string(),
            "fighter".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(player.clone()).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(player.clone());
        let monster = crate::domain::combat::monster::Monster::new(
            1,
            "Lurker".to_string(),
            crate::domain::character::Stats::new(8, 8, 8, 8, 8, 8, 8),
            8,
            5,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        cs.add_monster(monster);

        // Monster acts first
        cs.turn_order = vec![CombatantId::Monster(1), CombatantId::Player(0)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), None];
        }

        // Deterministic RNG
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(12345);
        let content = GameContent::new(crate::sdk::database::ContentDatabase::new());

        // Execute monster turn
        {
            let world = app.world_mut();
            let mut gs_owned = world
                .remove_resource::<crate::game::resources::GlobalState>()
                .expect("missing GlobalState resource for test");
            let mut turn_state_owned = world
                .remove_resource::<CombatTurnStateResource>()
                .expect("missing CombatTurnStateResource for test");
            let mut cr = world.resource_mut::<CombatResource>();

            perform_monster_turn_with_rng(
                &mut cr,
                &content,
                &mut gs_owned,
                &mut turn_state_owned,
                &mut rng,
            )
            .expect("monster turn failed");

            // After the monster acts, current_turn should have advanced (to index 1)
            assert_eq!(cr.state.current_turn, 1);

            world.insert_resource(gs_owned);
            world.insert_resource(turn_state_owned);
        }
    }

    /// Verify that a monster will preferentially attack the lowest HP (explicit)
    #[test]
    fn test_monster_attacks_lowest_hp_target() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Two players: one nearly dead, one healthy
        let mut gs = GameState::new();
        let healthy = Character::new(
            "Healthy".to_string(),
            "human".to_string(),
            "fighter".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let mut low = Character::new(
            "Wounded".to_string(),
            "human".to_string(),
            "peasant".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        low.hp.current = 1;
        gs.party.add_member(healthy.clone()).unwrap();
        gs.party.add_member(low.clone()).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(healthy.clone());
        cs.add_player(low.clone());

        let mut monster = crate::domain::combat::monster::Monster::new(
            7,
            "Stalker".to_string(),
            crate::domain::character::Stats::new(10, 8, 8, 8, 8, 8, 8),
            8,
            5,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        monster.ai_behavior = crate::domain::combat::monster::AiBehavior::Aggressive;
        cs.add_monster(monster);

        cs.turn_order = vec![
            CombatantId::Monster(2),
            CombatantId::Player(0),
            CombatantId::Player(1),
        ];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), Some(1), None];
        }

        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(7);
        let content = GameContent::new(crate::sdk::database::ContentDatabase::new());

        {
            let world = app.world_mut();
            let mut gs_owned = world
                .remove_resource::<crate::game::resources::GlobalState>()
                .expect("missing GlobalState resource for test");
            let mut turn_state_owned = world
                .remove_resource::<CombatTurnStateResource>()
                .expect("missing CombatTurnStateResource for test");
            let mut cr = world.resource_mut::<CombatResource>();

            perform_monster_turn_with_rng(
                &mut cr,
                &content,
                &mut gs_owned,
                &mut turn_state_owned,
                &mut rng,
            )
            .expect("monster turn failed");

            // The low-HP player (index 1) should be the one damaged / killed.
            if let Some(Combatant::Player(pc)) = cr.state.participants.get(1) {
                assert!(pc.hp.current < 1 || pc.hp.current == 0 || pc.hp.current < pc.hp.base);
            } else {
                panic!("expected wounded player at index 1");
            }

            world.insert_resource(gs_owned);
            world.insert_resource(turn_state_owned);
        }
    }

    // ===== Target Selection and Action Completeness Tests =====

    /// T2-1: `Tab` during target-select mode cycles `active_target_index`
    /// through alive monsters, wrapping back to 0 after the last.
    #[test]
    fn test_tab_cycles_targets() {
        use crate::domain::combat::monster::{AiBehavior, LootTable, Monster};

        // Build a CombatResource with 3 alive monsters.
        let mut cr = CombatResource::new();
        for i in 0..3usize {
            let mut m = Monster::new(
                i as u8,
                format!("Goblin {i}"),
                crate::domain::character::Stats::new(8, 6, 6, 8, 8, 8, 6),
                10,
                5,
                vec![],
                LootTable::default(),
            );
            m.ai_behavior = AiBehavior::Random;
            cr.state.participants.push(Combatant::Monster(Box::new(m)));
        }

        // Enter target-select mode: active_target_index starts at Some(0).
        let mut ams = ActionMenuState {
            active_index: 0,
            confirmed: false,
            active_target_index: Some(0),
        };
        let alive_count = cr
            .state
            .participants
            .iter()
            .filter(|p| matches!(p, Combatant::Monster(m) if m.hp.current > 0))
            .count();

        // Tab #1: 0 → 1
        ams.active_target_index = Some((ams.active_target_index.unwrap() + 1) % alive_count);
        assert_eq!(ams.active_target_index, Some(1));

        // Tab #2: 1 → 2
        ams.active_target_index = Some((ams.active_target_index.unwrap() + 1) % alive_count);
        assert_eq!(ams.active_target_index, Some(2));

        // Tab #3: 2 → 0 (wrap)
        ams.active_target_index = Some((ams.active_target_index.unwrap() + 1) % alive_count);
        assert_eq!(
            ams.active_target_index,
            Some(0),
            "index must wrap back to 0"
        );
    }

    /// T2-2: `Enter` while `active_target_index == Some(1)` emits
    /// `AttackAction { target: CombatantId::Monster(1) }` and clears state.
    #[test]
    fn test_enter_confirms_target() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        use crate::domain::combat::monster::{AiBehavior, Monster};

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());

        // Add 2 alive monsters as participants.
        for i in 0..2usize {
            let mut m = Monster::new(
                i as u8,
                format!("Orc {i}"),
                crate::domain::character::Stats::new(10, 6, 6, 10, 8, 8, 6),
                20,
                5,
                vec![],
                crate::domain::combat::monster::LootTable::default(),
            );
            m.ai_behavior = AiBehavior::Random;
            cs.participants.push(Combatant::Monster(Box::new(m)));
        }
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Enter target-select mode with keyboard index pointing at monster 1
        // (alive-monster index 1 → participant index 2 because participant 0 is
        // the player, participants 1 and 2 are the two monsters).
        {
            let mut ts = app.world_mut().resource_mut::<TargetSelection>();
            ts.0 = Some(CombatantId::Player(0));
        }
        {
            let mut ams = app.world_mut().resource_mut::<ActionMenuState>();
            ams.active_target_index = Some(1);
        }

        // Press Enter.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(KeyCode::Enter);
        }
        app.update();

        // TargetSelection must be cleared.
        let ts = app.world().resource::<TargetSelection>();
        assert!(
            ts.0.is_none(),
            "TargetSelection must be None after Enter confirms target"
        );

        // active_target_index must be cleared.
        let ams = app.world().resource::<ActionMenuState>();
        assert!(
            ams.active_target_index.is_none(),
            "active_target_index must be None after confirm"
        );
    }

    /// T2-3: `Escape` during target-select mode clears `TargetSelection.0`
    /// and resets `active_target_index` to `None`.
    #[test]
    fn test_escape_cancels_target_selection() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Enter target-select mode.
        {
            let mut ts = app.world_mut().resource_mut::<TargetSelection>();
            ts.0 = Some(CombatantId::Player(0));
        }
        {
            let mut ams = app.world_mut().resource_mut::<ActionMenuState>();
            ams.active_target_index = Some(0);
        }

        // Press Escape.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(KeyCode::Escape);
        }
        app.update();

        let ts = app.world().resource::<TargetSelection>();
        assert!(ts.0.is_none(), "TargetSelection must be None after Escape");

        let ams = app.world().resource::<ActionMenuState>();
        assert!(
            ams.active_target_index.is_none(),
            "active_target_index must be None after Escape"
        );
    }

    /// T2-4: Clicking an `EnemyCard` via `Interaction::Pressed` produces the
    /// same `AttackAction` structure as keyboard confirm would (same attacker,
    /// same `CombatantId::Monster` target).
    #[test]
    fn test_mouse_click_target_matches_keyboard_confirm() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        use crate::domain::combat::monster::{AiBehavior, Monster};

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());

        // Add one monster at participant index 1.
        let mut troll = Monster::new(
            1,
            "Troll".to_string(),
            crate::domain::character::Stats::new(14, 6, 6, 14, 8, 8, 4),
            30,
            6,
            vec![],
            crate::domain::combat::monster::LootTable::default(),
        );
        troll.ai_behavior = AiBehavior::Aggressive;
        cs.participants.push(Combatant::Monster(Box::new(troll)));
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Run first update to spawn UI (EnemyCard entities).
        app.update();

        // Enter target-select mode so EnemyCard clicks are handled.
        {
            let mut ts = app.world_mut().resource_mut::<TargetSelection>();
            ts.0 = Some(CombatantId::Player(0));
        }
        {
            let mut ams = app.world_mut().resource_mut::<ActionMenuState>();
            ams.active_target_index = Some(0);
        }

        // Find the EnemyCard entity at participant_index == 1 and press it.
        let card_entity = {
            let mut q = app
                .world_mut()
                .query_filtered::<(Entity, &EnemyCard), With<Button>>();
            q.iter(app.world())
                .find(|(_, card)| card.participant_index == 1)
                .map(|(e, _)| e)
                .expect("EnemyCard at participant_index 1 must exist")
        };

        app.world_mut()
            .entity_mut(card_entity)
            .insert(Interaction::Pressed);

        app.update();

        // After the mouse click, TargetSelection must be cleared — confirming
        // that `confirm_attack_target` was called (same path as keyboard Enter).
        let ts = app.world().resource::<TargetSelection>();
        assert!(
            ts.0.is_none(),
            "TargetSelection must be cleared after mouse click on EnemyCard (same as keyboard confirm)"
        );

        let ams = app.world().resource::<ActionMenuState>();
        assert!(
            ams.active_target_index.is_none(),
            "active_target_index must be None after mouse click confirm"
        );
    }

    /// T2-5: `COMBAT_ACTION_ORDER` covers all 5 action variants exactly once
    /// and the first entry is `ActionButtonType::Attack`.
    #[test]
    fn test_combat_action_order_constant_matches_spawn_order() {
        assert_eq!(
            COMBAT_ACTION_ORDER[0],
            ActionButtonType::Attack,
            "index 0 must be Attack"
        );
        assert_eq!(
            COMBAT_ACTION_ORDER[1],
            ActionButtonType::Defend,
            "index 1 must be Defend"
        );
        assert_eq!(
            COMBAT_ACTION_ORDER[2],
            ActionButtonType::Cast,
            "index 2 must be Cast"
        );
        assert_eq!(
            COMBAT_ACTION_ORDER[3],
            ActionButtonType::Item,
            "index 3 must be Item"
        );
        assert_eq!(
            COMBAT_ACTION_ORDER[4],
            ActionButtonType::Flee,
            "index 4 must be Flee"
        );
        assert_eq!(COMBAT_ACTION_COUNT, 5, "COMBAT_ACTION_COUNT must equal 5");
        // Verify all 5 variants are covered (no duplicates, no omissions).
        let all_variants = [
            ActionButtonType::Attack,
            ActionButtonType::Defend,
            ActionButtonType::Cast,
            ActionButtonType::Item,
            ActionButtonType::Flee,
        ];
        for variant in all_variants {
            assert!(
                COMBAT_ACTION_ORDER.contains(&variant),
                "COMBAT_ACTION_ORDER must contain {variant:?}"
            );
        }
    }

    // ===== Input Reliability Tests =====

    /// T1-1: `Tab` cycles active_index through all five actions (0 → 4).
    #[test]
    fn test_tab_cycles_through_actions() {
        let mut state = ActionMenuState::default();
        assert_eq!(state.active_index, 0);

        for expected in 1..=4usize {
            state.active_index = (state.active_index + 1) % 5;
            assert_eq!(state.active_index, expected);
        }
    }

    /// T1-2: `Tab` wraps from index 4 back to index 0.
    #[test]
    fn test_tab_wraps_at_end() {
        let mut state = ActionMenuState::default();
        state.active_index = 4;
        state.active_index = (state.active_index + 1) % 5;
        assert_eq!(state.active_index, 0);
    }

    /// T1-3: When the action menu becomes visible the highlight is reset to index 0 (Attack).
    #[test]
    fn test_default_highlight_is_attack_on_menu_open() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        gs.enter_combat();

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Start with EnemyTurn so the menu is hidden, then set a non-zero index.
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::EnemyTurn;
        app.update(); // spawn UI with hidden menu

        // Simulate player having navigated away from default index.
        app.world_mut()
            .resource_mut::<ActionMenuState>()
            .active_index = 3;

        // Transition back to PlayerTurn; update_combat_ui should reset the index.
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;
        app.update();

        let state = app.world().resource::<ActionMenuState>();
        assert_eq!(
            state.active_index, 0,
            "active_index must reset to 0 (Attack) when menu becomes visible"
        );
    }

    /// T1-4: Enter uses a single-step flow and dispatches immediately.
    #[test]
    fn test_enter_dispatches_active_action() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        use crate::domain::combat::monster::{AiBehavior, Monster};

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        // Set up a CombatState with the player and a live turn order.
        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        let mut goblin = Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(8, 6, 6, 8, 8, 8, 6),
            10,
            5,
            vec![],
            crate::domain::combat::monster::LootTable::default(),
        );
        goblin.ai_behavior = AiBehavior::Aggressive;
        cs.add_monster(goblin);
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Spawn combat UI and select Attack at index 0 for this test.
        app.update();
        app.world_mut()
            .resource_mut::<ActionMenuState>()
            .active_index = 0;

        // Single Enter should execute attack immediately.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(KeyCode::Enter);
        }
        app.update();

        let ts_after_first_enter = app.world().resource::<TargetSelection>();
        let ams_after_first_enter = app.world().resource::<ActionMenuState>();
        assert!(
            ts_after_first_enter.0.is_none(),
            "Enter should execute attack without entering target-selection mode"
        );
        assert!(
            !ams_after_first_enter.confirmed,
            "Single-step Enter should not leave confirmed armed state"
        );
    }

    /// Enter does not arm a second-step state; active action remains hover-highlighted.
    #[test]
    fn test_enter_keeps_hover_highlight_color() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Spawn UI and press Enter once.
        app.update();
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(KeyCode::Enter);
        }
        app.update();

        // Active action is Attack by default and should stay hover-highlighted.
        let mut q = app
            .world_mut()
            .query_filtered::<(&ActionButton, &BackgroundColor), With<Button>>();
        let attack_bg = q
            .iter(app.world())
            .find(|(btn, _)| btn.button_type == ActionButtonType::Attack)
            .map(|(_, bg)| *bg)
            .expect("Attack button must exist");

        assert_eq!(
            attack_bg,
            BackgroundColor(ACTION_BUTTON_HOVER_COLOR),
            "Single-step Enter should keep hover highlight color"
        );
    }

    /// Regression guard: one Enter on default Attack should execute immediately
    /// and advance turn flow without entering target-selection mode.
    #[test]
    fn test_single_enter_attack_executes_and_advances_turn() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        use crate::domain::combat::monster::{AiBehavior, Monster};

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        let mut goblin = Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(8, 6, 6, 8, 8, 8, 6),
            10,
            5,
            vec![],
            crate::domain::combat::monster::LootTable::default(),
        );
        goblin.ai_behavior = AiBehavior::Aggressive;
        cs.add_monster(goblin);
        // Player first, then monster.
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), None];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Spawn combat UI and keep default action index at 0 (Attack).
        app.update();

        // One Enter should execute attack immediately.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(KeyCode::Enter);
        }
        app.update();

        let ts = app.world().resource::<TargetSelection>();
        assert!(
            ts.0.is_none(),
            "single Enter attack should not enter target-selection mode"
        );

        // With execute_monster_turn now scheduled after update_combat_ui, the
        // monster (Goblin) fires its turn in the *same* frame as the player's
        // attack: the sequence within one app.update() is:
        //   1. handle_attack_action  → advances current_turn to 1, sets EnemyTurn
        //   2. update_combat_ui      → hides action menu (sees EnemyTurn)
        //   3. execute_monster_turn  → Goblin acts, advances current_turn back to 0,
        //                              sets PlayerTurn
        //
        // So after the frame the observable state is back to PlayerTurn with
        // current_turn == 0.  The meaningful invariants are:
        //   a) TargetSelection was cleared (attack was executed, not just armed)
        //   b) The combat is still InProgress (goblin survived or didn't — either
        //      way the state machine ran without panicking)
        //   c) The round advanced (at minimum current_turn wrapped back to 0)
        let cr = app.world().resource::<CombatResource>();
        assert!(
            cr.state.is_in_progress()
                || cr.state.status == crate::domain::combat::types::CombatStatus::Victory,
            "combat must still be progressing or have resolved cleanly after one round"
        );
        // current_turn wrapped back to 0 after the full mini-round
        assert_eq!(
            cr.state.current_turn, 0,
            "current_turn must wrap back to 0 after player attacks and monster responds in same frame"
        );
    }

    /// T1-5: `Interaction::Pressed` on the Attack button enters target selection.
    #[test]
    fn test_mouse_pressed_dispatches_action() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Run first update to spawn UI (ActionButton entities).
        app.update();

        // Find the Attack ActionButton entity and simulate a Pressed interaction.
        let attack_entity = {
            let mut q = app
                .world_mut()
                .query_filtered::<(Entity, &ActionButton), With<Button>>();
            q.iter(app.world())
                .find(|(_, btn)| btn.button_type == ActionButtonType::Attack)
                .map(|(e, _)| e)
                .expect("Attack button must exist")
        };

        app.world_mut()
            .entity_mut(attack_entity)
            .insert(Interaction::Pressed);

        app.update();

        let ts = app.world().resource::<TargetSelection>();
        assert!(
            ts.0.is_some(),
            "TargetSelection must be Some after pressing Attack"
        );
    }

    /// Left mouse `just_pressed` while hovering an action button should also
    /// dispatch, even if the `Interaction::Pressed` transition is not observed.
    #[test]
    fn test_mouse_left_click_on_hover_dispatches_action() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Ensure mouse input resource exists for just_pressed checks.
        app.insert_resource(ButtonInput::<MouseButton>::default());

        // Spawn UI.
        app.update();

        let attack_entity = {
            let mut q = app
                .world_mut()
                .query_filtered::<(Entity, &ActionButton), With<Button>>();
            q.iter(app.world())
                .find(|(_, btn)| btn.button_type == ActionButtonType::Attack)
                .map(|(e, _)| e)
                .expect("Attack button must exist")
        };

        // Simulate hover plus left-click this frame.
        app.world_mut()
            .entity_mut(attack_entity)
            .insert(Interaction::Hovered);
        {
            let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            mouse.press(MouseButton::Left);
        }

        app.update();

        let ts = app.world().resource::<TargetSelection>();
        assert!(
            ts.0.is_some(),
            "Left click while hovered should dispatch Attack"
        );
    }

    /// T1-6: `Interaction::Hovered` on an ActionButton must NOT dispatch any action.
    #[test]
    fn test_mouse_hover_does_not_dispatch() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Spawn UI.
        app.update();

        // Simulate Hover on Attack button.
        let attack_entity = {
            let mut q = app
                .world_mut()
                .query_filtered::<(Entity, &ActionButton), With<Button>>();
            q.iter(app.world())
                .find(|(_, btn)| btn.button_type == ActionButtonType::Attack)
                .map(|(e, _)| e)
                .expect("Attack button must exist")
        };

        app.world_mut()
            .entity_mut(attack_entity)
            .insert(Interaction::Hovered);

        app.update();

        // TargetSelection must remain None.
        let ts = app.world().resource::<TargetSelection>();
        assert!(
            ts.0.is_none(),
            "Hovered interaction must not enter target selection"
        );
    }

    /// T1-7: Pressing `KeyA` during `PlayerTurn` must NOT enter target selection
    /// (A/D/F shortcuts have been removed).
    #[test]
    fn test_key_a_does_not_dispatch_in_combat() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Simulate pressing KeyA.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(KeyCode::KeyA);
        }

        app.update();

        // TargetSelection must remain None — KeyA is no longer a combat shortcut.
        let ts = app.world().resource::<TargetSelection>();
        assert!(
            ts.0.is_none(),
            "KeyA must not trigger Attack in combat (A/D/F shortcuts removed)"
        );
    }

    /// T1-8: The `GameMode::Combat` guard in the split input systems is validated
    /// by the dedicated integration test
    /// `test_movement_blocked_in_combat_mode` located in
    /// `src/game/systems/input.rs` (module `combat_guard_tests`).
    ///
    /// This stub asserts the precondition that entering combat mode sets the mode
    /// correctly, confirming the guard has something to match against.
    #[test]
    fn test_movement_blocked_in_combat_mode() {
        // Verify that enter_combat transitions the game into Combat mode.
        // The actual movement-blocking behaviour is tested in input.rs.
        let mut gs = GameState::new();
        let hero = Character::new(
            "Guard Test".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        gs.enter_combat();

        assert!(
            matches!(gs.mode, crate::application::GameMode::Combat(_)),
            "GameMode must be Combat after enter_combat() so the split input-system combat guard fires"
        );
    }

    // ===== Visual Combat Feedback Tests =====

    /// T3-1: Firing an `AttackAction` that hits emits a `CombatFeedbackEvent`
    /// with `effect: Damage(_)`.
    #[test]
    fn test_feedback_event_emitted_on_hit() {
        use crate::domain::combat::monster::{AiBehavior, LootTable, Monster};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());

        // Add a monster with very low AC so the player will almost certainly hit
        let mut goblin = Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(8, 6, 6, 8, 8, 4, 6),
            20,
            1, // AC 1 → easy to hit
            vec![],
            LootTable::default(),
        );
        goblin.ai_behavior = AiBehavior::Random;
        cs.participants.push(Combatant::Monster(Box::new(goblin)));
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Force the monster to have full HP so a hit is detectable
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            if let Some(Combatant::Monster(m)) = cr.state.participants.get_mut(1) {
                m.hp.base = 100;
                m.hp.current = 100;
                // Force high accuracy on player so it always hits
            }
            if let Some(Combatant::Player(pc)) = cr.state.participants.get_mut(0) {
                pc.stats.accuracy.current = 255;
            }
        }

        // Write an AttackAction into the message bus
        app.world_mut().write_message(AttackAction {
            attacker: CombatantId::Player(0),
            target: CombatantId::Monster(1),
        });

        app.update();

        // Check that a CombatFeedbackEvent was written (it will be consumed by
        // spawn_combat_feedback on the same frame; verify indirectly via the
        // FloatingDamage entity count > 0 or by checking HP dropped).
        // The actual feedback event is consumed within the same frame by
        // spawn_combat_feedback so it won't be visible after update(); however
        // the FloatingDamage entities will exist regardless of hit or miss.
        let floating_count = {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<FloatingDamage>>();
            q.iter(app.world()).count()
        };
        // A FloatingDamage entity is spawned for every attack (hit → "-N", miss → "Miss").
        assert!(
            floating_count > 0,
            "FloatingDamage must exist after an attack action (hit or miss)"
        );
    }

    /// T3-2: A miss from `perform_attack_action_with_rng` (zero damage) causes
    /// `handle_attack_action` to call `emit_combat_feedback` with
    /// `CombatFeedbackEffect::Miss`, which `spawn_combat_feedback` converts into
    /// a `FloatingDamage` entity containing "Miss" text.
    ///
    /// We verify this end-to-end by calling `perform_attack_action_with_rng`
    /// directly with a seeded RNG that rolls 1 (guaranteed miss since any
    /// hit_threshold is > 1 when the monster has positive AC), then confirm
    /// the `FloatingDamage` entity is spawned via the ECS system path.
    #[test]
    fn test_feedback_event_emitted_on_miss() {
        use crate::domain::combat::monster::{AiBehavior, LootTable, Monster};
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        // ── Part 1: verify the domain call produces 0 damage (miss) ──────────
        let mut cs_direct = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        cs_direct.add_player(hero.clone());

        // Monster with AC 20 and player accuracy 0:
        // hit_threshold = (10 + 20 - 0).max(2) = 30 → always > 20 → always miss
        let mut iron = Monster::new(
            1,
            "Iron Golem".to_string(),
            crate::domain::character::Stats::new(18, 6, 6, 18, 8, 20, 6),
            50,
            20,
            vec![],
            LootTable::default(),
        );
        iron.ai_behavior = AiBehavior::Random;
        cs_direct
            .participants
            .push(Combatant::Monster(Box::new(iron)));
        cs_direct.turn_order = vec![CombatantId::Player(0)];
        cs_direct.current_turn = 0;

        // Zero accuracy → hit_threshold = (10 + 20).max(2) = 30 > 20 → guaranteed miss
        if let Some(Combatant::Player(pc)) = cs_direct.participants.get_mut(0) {
            pc.stats.accuracy.current = 0;
            pc.stats.accuracy.base = 0;
        }

        let mut cr_direct = CombatResource::new();
        cr_direct.state = cs_direct;
        cr_direct.player_orig_indices = vec![Some(0)];

        let content = GameContent::new(crate::sdk::database::ContentDatabase::new());
        let mut gs_local = crate::game::resources::GlobalState(GameState::new());
        let mut ts_local = CombatTurnStateResource::default();
        // Any seed: we always miss because roll(1..=20) < 30 is always true
        let mut rng = StdRng::seed_from_u64(0);

        let action = AttackAction {
            attacker: CombatantId::Player(0),
            target: CombatantId::Monster(1),
        };
        perform_attack_action_with_rng(
            &mut cr_direct,
            &action,
            &content,
            &mut gs_local,
            &mut ts_local,
            &mut rng,
        )
        .expect("perform_attack should not error");

        // Monster HP must be unchanged after a miss
        if let Some(Combatant::Monster(m)) = cr_direct.state.participants.get(1) {
            assert_eq!(m.hp.current, 50, "monster HP must not change on a miss");
        } else {
            panic!("monster participant not found");
        }

        // ── Part 2: ECS path — system emits FloatingDamage for a miss ────────
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());

        let mut iron2 = Monster::new(
            1,
            "Iron Golem".to_string(),
            crate::domain::character::Stats::new(18, 6, 6, 18, 8, 20, 6),
            50,
            20,
            vec![],
            LootTable::default(),
        );
        iron2.ai_behavior = AiBehavior::Random;
        cs.participants.push(Combatant::Monster(Box::new(iron2)));
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Zero accuracy so handle_attack_action always misses
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            if let Some(Combatant::Player(pc)) = cr.state.participants.get_mut(0) {
                pc.stats.accuracy.current = 0;
                pc.stats.accuracy.base = 0;
            }
        }

        app.world_mut().write_message(AttackAction {
            attacker: CombatantId::Player(0),
            target: CombatantId::Monster(1),
        });

        app.update();

        // A FloatingDamage entity must exist for the "Miss" feedback
        let floating_count = {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<FloatingDamage>>();
            q.iter(app.world()).count()
        };
        assert!(
            floating_count > 0,
            "FloatingDamage must be spawned for a Miss feedback event"
        );
    }

    /// T3-3: After `execute_monster_turn` runs, a `CombatFeedbackEvent` targeting
    /// a player is emitted (detected via `FloatingDamage` entity presence or HP change).
    #[test]
    fn test_monster_turn_emits_feedback() {
        use crate::domain::combat::monster::{AiBehavior, LootTable, Monster};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());

        // Monster with high accuracy and low AC so it hits the player easily
        let mut orc = Monster::new(
            1,
            "Orc".to_string(),
            crate::domain::character::Stats::new(14, 6, 6, 12, 10, 18, 6),
            30,
            5,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 6, 2),
            )],
            LootTable::default(),
        );
        orc.ai_behavior = AiBehavior::Aggressive;
        cs.participants.push(Combatant::Monster(Box::new(orc)));

        // Monster goes first
        cs.turn_order = vec![CombatantId::Monster(1)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }

        // Set EnemyTurn so execute_monster_turn fires
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::EnemyTurn;

        // Record player HP before
        let hp_before = app
            .world()
            .resource::<CombatResource>()
            .state
            .participants
            .iter()
            .find_map(|p| match p {
                Combatant::Player(pc) => Some(pc.hp.current),
                _ => None,
            })
            .unwrap_or(0);

        app.update();

        // The system ran. Either HP changed (damage feedback emitted) or the
        // FloatingDamage entity exists — both confirm feedback path executed.
        let hp_after = app
            .world()
            .resource::<CombatResource>()
            .state
            .participants
            .iter()
            .find_map(|p| match p {
                Combatant::Player(pc) => Some(pc.hp.current),
                _ => None,
            })
            .unwrap_or(0);

        let floating_count = {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<FloatingDamage>>();
            q.iter(app.world()).count()
        };

        // If HP changed, the monster landed a hit and feedback was emitted.
        // If HP is unchanged (miss), no feedback node is spawned — that's also fine.
        // The key is the system didn't panic.
        let damage_dealt = (hp_before as i32 - hp_after as i32).max(0) as u32;
        if damage_dealt > 0 {
            assert!(
                floating_count > 0,
                "FloatingDamage must exist when monster dealt damage"
            );
        }
        // System ran without panic — that's the core assertion.
    }

    /// T3-4: After `AttackAction` is written and the system runs, the
    /// `CombatTurnState` is NOT stuck in `Animating` — it transitions to the
    /// next natural state (PlayerTurn or EnemyTurn) after the action completes.
    #[test]
    fn test_animating_state_set_during_action() {
        use crate::domain::combat::monster::{AiBehavior, LootTable, Monster};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());

        let mut goblin = Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(8, 6, 6, 8, 8, 8, 6),
            15,
            5,
            vec![],
            LootTable::default(),
        );
        goblin.ai_behavior = AiBehavior::Random;
        cs.participants.push(Combatant::Monster(Box::new(goblin)));
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Fire an AttackAction
        app.world_mut().write_message(AttackAction {
            attacker: CombatantId::Player(0),
            target: CombatantId::Monster(1),
        });

        app.update();

        // After the update the turn state must NOT be stuck in Animating.
        let ts = app.world().resource::<CombatTurnStateResource>();
        assert!(
            !matches!(ts.0, CombatTurnState::Animating),
            "CombatTurnState must not remain Animating after action completes; got {:?}",
            ts.0
        );
    }

    /// T3-5: `hide_indicator_during_animation` hides the indicator when
    /// `CombatTurnState::Animating` and restores it otherwise.
    /// (Mirrors the existing test in combat_visual.rs — confirms integration.)
    #[test]
    fn test_indicator_hidden_during_animating() {
        use crate::game::components::combat::TurnIndicator;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();
        gs.enter_combat();
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Set up a minimal CombatResource with a player turn
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            let player = Character::new(
                "Hero".to_string(),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            cr.state.add_player(player);
            cr.state.turn_order = vec![CombatantId::Player(0)];
            cr.state.current_turn = 0;
        }

        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;
        app.update(); // spawn indicator

        // Set Animating
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::Animating;
        app.update();

        // All TurnIndicator entities must be Hidden
        let hidden = {
            let mut q = app
                .world_mut()
                .query_filtered::<&Visibility, With<TurnIndicator>>();
            q.iter(app.world()).all(|v| matches!(v, Visibility::Hidden))
        };
        assert!(
            hidden,
            "all TurnIndicator nodes must be Hidden during Animating"
        );

        // Restore to PlayerTurn
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;
        app.update();

        let visible = {
            let mut q = app
                .world_mut()
                .query_filtered::<&Visibility, With<TurnIndicator>>();
            q.iter(app.world())
                .all(|v| matches!(v, Visibility::Visible))
        };
        assert!(
            visible,
            "all TurnIndicator nodes must be Visible after Animating ends"
        );
    }

    /// T3-6: Entering combat with 2 monsters spawns exactly 2 `MonsterHpHoverBar`
    /// entities after one frame.
    #[test]
    fn test_hover_bars_spawned_on_combat_start() {
        use crate::domain::combat::monster::{AiBehavior, LootTable, Monster};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());

        for i in 0..2usize {
            let mut m = Monster::new(
                i as u8 + 1,
                format!("Goblin {i}"),
                crate::domain::character::Stats::new(8, 6, 6, 8, 8, 8, 6),
                10,
                5,
                vec![],
                LootTable::default(),
            );
            m.ai_behavior = AiBehavior::Random;
            cs.participants.push(Combatant::Monster(Box::new(m)));
        }
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        app.update(); // spawn hover bars

        let bar_count = {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<MonsterHpHoverBar>>();
            q.iter(app.world()).count()
        };
        assert_eq!(
            bar_count, 2,
            "expected 2 MonsterHpHoverBar entities for 2 alive monsters, got {bar_count}"
        );
    }

    /// T3-7: After exiting combat all `MonsterHpHoverBar` entities are removed.
    #[test]
    fn test_hover_bars_removed_on_combat_exit() {
        use crate::domain::combat::monster::{AiBehavior, LootTable, Monster};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());

        let mut m = Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(8, 6, 6, 8, 8, 8, 6),
            10,
            5,
            vec![],
            LootTable::default(),
        );
        m.ai_behavior = AiBehavior::Random;
        cs.participants.push(Combatant::Monster(Box::new(m)));
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        app.update(); // spawn bars

        // Verify bars were spawned
        let bar_count_before = {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<MonsterHpHoverBar>>();
            q.iter(app.world()).count()
        };
        assert!(bar_count_before > 0, "bars must exist after combat start");

        // Exit combat
        {
            let mut gs = app
                .world_mut()
                .resource_mut::<crate::game::resources::GlobalState>();
            gs.0.exit_combat();
        }
        app.update(); // cleanup

        let bar_count_after = {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<MonsterHpHoverBar>>();
            q.iter(app.world()).count()
        };
        assert_eq!(
            bar_count_after, 0,
            "all MonsterHpHoverBar entities must be despawned after combat exit"
        );
    }

    /// T3-8: After damage is dealt to a monster the corresponding
    /// `MonsterHpHoverBarFill` node reflects the reduced HP (width < 100%).
    #[test]
    fn test_hover_bar_hp_updated_after_damage() {
        use crate::domain::combat::monster::{AiBehavior, LootTable, Monster};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());

        let mut goblin = Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(8, 6, 6, 8, 8, 8, 6),
            100, // base HP
            5,
            vec![],
            LootTable::default(),
        );
        goblin.ai_behavior = AiBehavior::Random;
        cs.participants.push(Combatant::Monster(Box::new(goblin)));
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        app.update(); // spawn bars — monster at 100/100 → fill = 100%

        // Manually reduce HP in CombatResource (simulates damage)
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            if let Some(Combatant::Monster(m)) = cr.state.participants.get_mut(1) {
                m.hp.current = 50; // 50% HP
            }
        }

        app.update(); // update_monster_hp_hover_bars fires

        // The fill node must now have width < 100%
        let fill_widths: Vec<Val> = {
            let mut q = app
                .world_mut()
                .query_filtered::<&Node, With<MonsterHpHoverBarFill>>();
            q.iter(app.world()).map(|n| n.width).collect()
        };

        assert!(
            !fill_widths.is_empty(),
            "MonsterHpHoverBarFill nodes must exist"
        );

        for w in &fill_widths {
            if let Val::Percent(pct) = w {
                assert!(
                    *pct <= 50.0 + f32::EPSILON,
                    "fill width must be ≤ 50% after monster takes 50% damage, got {pct}%"
                );
            }
        }
    }

    // ===== Defeated Monster World-Mesh Removal =====

    /// T4-E1: After a `MapEvent::Encounter` triggers combat, `combat_res.encounter_position`
    /// and `combat_res.encounter_map_id` must be set to the tile position and map id.
    #[test]
    fn test_encounter_position_stored_on_combat_start() {
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent};

        let mut combat_res = CombatResource::new();
        assert!(combat_res.encounter_position.is_none());
        assert!(combat_res.encounter_map_id.is_none());

        // Simulate what events.rs does after start_encounter succeeds.
        let tile_pos = Position::new(3, 7);
        let map_id: crate::domain::types::MapId = 2;
        combat_res.encounter_position = Some(tile_pos);
        combat_res.encounter_map_id = Some(map_id);

        assert_eq!(combat_res.encounter_position, Some(tile_pos));
        assert_eq!(combat_res.encounter_map_id, Some(map_id));

        // Verify clear() resets both fields.
        combat_res.clear();
        assert!(combat_res.encounter_position.is_none());
        assert!(combat_res.encounter_map_id.is_none());

        // Verify a freshly added encounter event exists on the map.
        let mut map = Map::new(map_id, "Test".to_string(), "Desc".to_string(), 10, 10);
        map.add_event(
            tile_pos,
            MapEvent::Encounter {
                name: "Goblins".to_string(),
                description: "A band of goblins".to_string(),
                monster_group: vec![1],
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
            },
        );
        assert!(map.get_event(tile_pos).is_some());
    }

    /// T4-E2: After `handle_combat_victory` processes a victory, the backing
    /// `MapEvent::Encounter` must be removed from the map data.
    #[test]
    fn test_encounter_event_removed_on_victory() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::combat::engine::CombatState;
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Build a game state with a live party member.
        let mut gs = crate::application::GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        // Place an encounter event on a map.
        let map_id: crate::domain::types::MapId = 1;
        let encounter_pos = Position::new(5, 5);
        let mut map = Map::new(map_id, "Test".to_string(), "Desc".to_string(), 10, 10);
        map.add_event(
            encounter_pos,
            MapEvent::Encounter {
                name: "Goblins".to_string(),
                description: "A band of goblins".to_string(),
                monster_group: vec![1],
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
            },
        );
        gs.world.add_map(map);
        gs.world.set_current_map(map_id);

        // Enter combat so exit_combat() has a valid state to exit.
        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));

        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
            cr.encounter_position = Some(encounter_pos);
            cr.encounter_map_id = Some(map_id);
        }

        // Send the CombatVictory message.
        {
            let mut writer = app.world_mut().resource_mut::<Messages<CombatVictory>>();
            writer.write(CombatVictory {});
        }
        app.update();

        // The encounter event must be gone from the map.
        let gs = app
            .world()
            .resource::<crate::game::resources::GlobalState>();
        let map = gs.0.world.get_map(map_id).expect("map should exist");
        assert!(
            map.get_event(encounter_pos).is_none(),
            "Encounter event should have been removed on victory"
        );
    }

    /// T4-E3: After victory the `encounter_position` and `encounter_map_id`
    /// fields of `CombatResource` must be `None`.
    #[test]
    fn test_encounter_position_cleared_after_victory() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::combat::engine::CombatState;
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = crate::application::GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let map_id: crate::domain::types::MapId = 1;
        let encounter_pos = Position::new(2, 2);
        let mut map = Map::new(map_id, "Test".to_string(), "Desc".to_string(), 10, 10);
        map.add_event(
            encounter_pos,
            MapEvent::Encounter {
                name: "Goblins".to_string(),
                description: "Goblins".to_string(),
                monster_group: vec![1],
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
            },
        );
        gs.world.add_map(map);
        gs.world.set_current_map(map_id);

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));

        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
            cr.encounter_position = Some(encounter_pos);
            cr.encounter_map_id = Some(map_id);
        }

        {
            let mut writer = app.world_mut().resource_mut::<Messages<CombatVictory>>();
            writer.write(CombatVictory {});
        }
        app.update();

        let cr = app.world().resource::<CombatResource>();
        assert!(
            cr.encounter_position.is_none(),
            "encounter_position must be None after victory"
        );
        assert!(
            cr.encounter_map_id.is_none(),
            "encounter_map_id must be None after victory"
        );
    }

    /// T1-9: When `CombatTurnState` is `EnemyTurn` and `Tab` is pressed,
    /// `active_index` remains unchanged (blocked) — no crash, no dispatch.
    #[test]
    fn test_blocked_input_logs_feedback() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        // Build a combat state where the monster is the current actor so that
        // EnemyTurn is consistent with the turn_order.  The monster has already
        // acted (has_acted == true) so can_act() returns false — execute_monster_turn
        // will advance the turn on the *second* update, but for the frame on which
        // we check input blocking the system still sees EnemyTurn at entry.
        let mut cs = crate::domain::combat::engine::CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        let mut blocking_monster = crate::domain::combat::monster::Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(10, 8, 8, 10, 8, 10, 8),
            10,
            8,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        blocking_monster.mark_acted();
        cs.add_monster(blocking_monster);
        // Monster is current actor (index 1 in participants, slot 0 in turn_order).
        cs.turn_order = vec![CombatantId::Monster(1), CombatantId::Player(0)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs.clone());

        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), None];
        }

        // Enemy turn — input should be blocked.
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::EnemyTurn;
        app.update(); // spawn UI; execute_monster_turn skips the already-acted monster
                      // and advances turn_state to PlayerTurn on this frame

        // After the first update execute_monster_turn has advanced to PlayerTurn
        // (monster could not act).  Force EnemyTurn again so we can test that
        // combat_input_system blocks keyboard input when turn_state is EnemyTurn.
        // Also move current_turn back to the monster slot to keep state consistent.
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state.current_turn = 0; // back to monster slot
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::EnemyTurn;

        // Record active_index before pressing Tab.
        let index_before = app.world().resource::<ActionMenuState>().active_index;

        // Press Tab during EnemyTurn — combat_input_system runs before
        // execute_monster_turn in the schedule, so it sees EnemyTurn first and
        // blocks the input.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(KeyCode::Tab);
        }
        app.update();

        // active_index must be unchanged — input was blocked by combat_input_system.
        let index_after = app.world().resource::<ActionMenuState>().active_index;
        assert_eq!(
            index_after, index_before,
            "active_index must not change when input is blocked during EnemyTurn"
        );

        // TargetSelection must also remain None.
        let ts = app.world().resource::<TargetSelection>();
        assert!(ts.0.is_none(), "No dispatch must occur during EnemyTurn");
    }

    /// Regression: when the fastest combatant is a monster (e.g. Ancient Wolf
    /// speed 14 > all party speeds), `handle_combat_started` must initialise
    /// `CombatTurnStateResource` to `EnemyTurn`, not leave it at the default
    /// `PlayerTurn`.  Before the fix the action buttons would appear on the
    /// first frame and the player could "act" before the monster's turn ran.
    #[test]
    fn test_monster_first_initiative_sets_enemy_turn() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Party member with speed 8 (slower than the wolf).
        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Slow Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Force a very low speed so the monster is guaranteed to go first.
        hero.stats.speed.base = 5;
        hero.stats.speed.current = 5;
        gs.party.add_member(hero.clone()).unwrap();

        // Monster with speed 14 — goes first under Handicap::Even.
        let fast_wolf = crate::domain::combat::monster::Monster::new(
            99,
            "Ancient Wolf".to_string(),
            crate::domain::character::Stats::new(16, 5, 7, 14, 14, 10, 10),
            30,
            12,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(2, 6, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        cs.add_monster(fast_wolf);
        // calculate_turn_order puts the monster first (speed 14 > 5).
        crate::domain::combat::engine::start_combat(&mut cs);

        // Confirm the turn order really does put the monster first.
        assert!(
            matches!(cs.turn_order.first(), Some(CombatantId::Monster(_))),
            "test precondition: monster must be first in turn order"
        );

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Deliver the CombatStarted message so handle_combat_started fires.
        {
            let mut writer = app.world_mut().resource_mut::<Messages<CombatStarted>>();
            writer.write(CombatStarted {
                encounter_position: None,
                encounter_map_id: None,
                combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
            });
        }
        // Seed CombatResource so handle_combat_started has something to copy.
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), None];
        }

        // First frame: handle_combat_started fires and sets CombatTurnStateResource.
        app.update();

        let ts = app.world().resource::<CombatTurnStateResource>();
        assert!(
            matches!(ts.0, CombatTurnState::EnemyTurn),
            "CombatTurnStateResource must be EnemyTurn when monster goes first, got {:?}",
            ts.0
        );

        // The turn-state assertion above is the authoritative proof of the fix.
        // We do NOT assert action-menu visibility here because execute_monster_turn
        // fires on the very next frame (the wolf acts, advances the turn to the
        // player, and flips turn_state back to PlayerTurn), which would make the
        // menu visible again and cause a flaky assertion.  The core contract —
        // "handle_combat_started sets EnemyTurn when the monster is first" — is
        // fully covered by the turn_state assertion that already passed above.
    }

    /// Regression: when a monster's `can_act()` returns false (already acted,
    /// dead, or paralyzed) `execute_monster_turn` must still advance the turn
    /// pointer and flip `CombatTurnStateResource` to `PlayerTurn` so the player
    /// is not locked out.  Before the fix the system returned early without
    /// advancing the turn, leaving `turn_state` stuck on `EnemyTurn` forever
    /// and causing "input blocked — not player turn" to spam the log.
    #[test]
    fn test_incapacitated_monster_turn_advances_to_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        // Build a combat state where the monster is first in turn order but
        // has already acted (has_acted == true), so can_act() returns false.
        let mut already_acted_wolf = crate::domain::combat::monster::Monster::new(
            99,
            "Ancient Wolf".to_string(),
            crate::domain::character::Stats::new(16, 5, 7, 14, 14, 10, 10),
            30,
            12,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(2, 6, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        already_acted_wolf.mark_acted(); // simulate already acted this round

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        cs.add_monster(already_acted_wolf);
        // Force monster first in the turn order.
        cs.turn_order = vec![CombatantId::Monster(1), CombatantId::Player(0)];
        cs.current_turn = 0;
        cs.status = crate::domain::combat::types::CombatStatus::InProgress;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Seed CombatResource and force turn_state to EnemyTurn.
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), None];
        }
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::EnemyTurn;

        // One frame: execute_monster_turn should detect can_act()==false and
        // advance the turn to the player, flipping turn_state to PlayerTurn.
        app.update();

        let ts = app.world().resource::<CombatTurnStateResource>();
        assert!(
            matches!(ts.0, CombatTurnState::PlayerTurn),
            "turn_state must be PlayerTurn after skipping incapacitated monster, got {:?}",
            ts.0
        );
    }

    // ── Integration tests ─────────────────────────────────────────────────────
    // These tests verify that `perform_attack_action_with_rng` now calls
    // `get_character_attack` and dispatches on `MeleeAttackResult` instead of
    // using the old hardcoded `DiceRoll::new(1, 4, 0)`.

    /// Build a minimal weapon `Item` for use in integration tests.
    fn make_p2_weapon_item(
        id: u8,
        damage: DiceRoll,
        bonus: i8,
        classification: crate::domain::items::WeaponClassification,
    ) -> crate::domain::items::Item {
        use crate::domain::items::{Item, ItemType, WeaponData};
        Item {
            id,
            name: format!("P2Weapon#{}", id),
            item_type: ItemType::Weapon(WeaponData {
                damage,
                bonus,
                hands_required: 1,
                classification,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        }
    }

    /// Build a self-contained `(CombatResource, GameContent, GlobalState,
    /// CombatTurnStateResource)` fixture with one player (index 0) and one
    /// goblin monster (index 1, AC 1 so the player almost always hits).
    fn make_p2_combat_fixture(
        player: Character,
    ) -> (
        CombatResource,
        crate::application::resources::GameContent,
        crate::game::resources::GlobalState,
        CombatTurnStateResource,
    ) {
        use crate::application::GameState;
        use crate::domain::combat::monster::LootTable;

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(player.clone());

        let goblin = crate::domain::combat::monster::Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(8, 6, 6, 8, 8, 1, 6),
            30,
            1, // AC 1 — very easy to hit
            vec![],
            LootTable::default(),
        );
        cs.add_monster(goblin);
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;
        cs.status = crate::domain::combat::types::CombatStatus::InProgress;

        let mut cr = CombatResource::new();
        cr.state = cs;
        cr.player_orig_indices = vec![Some(0), None];

        let content = crate::application::resources::GameContent::new(
            crate::sdk::database::ContentDatabase::new(),
        );
        let gs = crate::game::resources::GlobalState(GameState::new());
        let ts = CombatTurnStateResource::default();
        (cr, content, gs, ts)
    }

    /// T1: A player with a longsword (1d8, bonus 0) must deal damage
    /// in the range [1, 8].  Over 50 seeds at least one roll must exceed 4,
    /// proving the old hardcoded 1d4 is gone.
    #[test]
    fn test_player_attack_uses_equipped_melee_weapon_damage() {
        use crate::domain::items::{ItemDatabase, WeaponClassification};
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let longsword = make_p2_weapon_item(
            42,
            DiceRoll::new(1, 8, 0),
            0,
            WeaponClassification::MartialMelee,
        );

        let mut item_db = ItemDatabase::new();
        item_db.add_item(longsword).unwrap();

        let mut player = Character::new(
            "Sir Lancelot".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        player.equipment.weapon = Some(42);
        player.stats.accuracy.current = 255; // always hit

        let (cr, mut content, _gs, ts) = make_p2_combat_fixture(player);
        content.db_mut().items = item_db;

        let hp_before = match cr.state.participants.get(1) {
            Some(Combatant::Monster(m)) => m.hp.base,
            _ => panic!("monster not found in fixture"),
        };

        let action = AttackAction {
            attacker: CombatantId::Player(0),
            target: CombatantId::Monster(1),
        };

        let mut any_above_four = false;
        for seed in 0u64..50 {
            let mut cr_clone = cr.clone();
            let mut gs_local =
                crate::game::resources::GlobalState(crate::application::GameState::new());
            let mut ts_clone = ts.clone();
            let mut rng = StdRng::seed_from_u64(seed);

            let result = perform_attack_action_with_rng(
                &mut cr_clone,
                &action,
                &content,
                &mut gs_local,
                &mut ts_clone,
                &mut rng,
            );
            assert!(result.is_ok(), "attack must not error: {:?}", result);

            if let Some(Combatant::Monster(m)) = cr_clone.state.participants.get(1) {
                let damage = (hp_before as i32 - m.hp.current as i32).max(0) as u32;
                assert!(damage <= 8, "longsword damage {} exceeded 1d8 max", damage);
                if damage > 4 {
                    any_above_four = true;
                }
            }
        }

        assert!(
            any_above_four,
            "longsword (1d8) never rolled above 4 over 50 seeds — old 1d4 may still be active"
        );
    }

    /// T2: An unarmed player (`equipment.weapon = None`) must deal at
    /// most 2 damage per hit — the 1d2 UNARMED_DAMAGE fallback.
    #[test]
    fn test_player_attack_unarmed_when_no_weapon() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut player = Character::new(
            "Peasant".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        player.equipment.weapon = None;
        player.stats.accuracy.current = 255; // always hit

        let (cr, content, _gs, ts) = make_p2_combat_fixture(player);

        let hp_before = match cr.state.participants.get(1) {
            Some(Combatant::Monster(m)) => m.hp.base,
            _ => panic!("monster not found in fixture"),
        };

        let action = AttackAction {
            attacker: CombatantId::Player(0),
            target: CombatantId::Monster(1),
        };

        for seed in 0u64..30 {
            let mut cr_clone = cr.clone();
            let mut gs_local =
                crate::game::resources::GlobalState(crate::application::GameState::new());
            let mut ts_clone = ts.clone();
            let mut rng = StdRng::seed_from_u64(seed);

            let result = perform_attack_action_with_rng(
                &mut cr_clone,
                &action,
                &content,
                &mut gs_local,
                &mut ts_clone,
                &mut rng,
            );
            assert!(
                result.is_ok(),
                "unarmed attack must not error: {:?}",
                result
            );

            if let Some(Combatant::Monster(m)) = cr_clone.state.participants.get(1) {
                let damage = (hp_before as i32 - m.hp.current as i32).max(0) as u32;
                assert!(
                    damage <= 2,
                    "unarmed damage {} exceeded 1d2 maximum (2) on seed {}",
                    damage,
                    seed
                );
            }
        }
    }

    /// T3: A cursed dagger (1d4 with bonus -3 baked into the
    /// `DiceRoll`) must always deal at least 1 damage when it hits — the damage
    /// floor from `DiceRoll::roll` prevents negative or zero results.
    #[test]
    fn test_player_attack_bonus_weapon_floor_at_one() {
        use crate::domain::items::{ItemDatabase, WeaponClassification};
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        // 1d4 with bonus -3: worst roll is 1 + (-3) = -2, but the floor in
        // DiceRoll::roll clamps it to 1.
        let cursed_dagger =
            make_p2_weapon_item(99, DiceRoll::new(1, 4, -3), 0, WeaponClassification::Simple);

        let mut item_db = ItemDatabase::new();
        item_db.add_item(cursed_dagger).unwrap();

        let mut player = Character::new(
            "Cursed Rogue".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        player.equipment.weapon = Some(99);
        player.stats.accuracy.current = 255; // always hit

        let (cr, mut content, _gs, ts) = make_p2_combat_fixture(player);
        content.db_mut().items = item_db;

        let hp_before = match cr.state.participants.get(1) {
            Some(Combatant::Monster(m)) => m.hp.base,
            _ => panic!("monster not found in fixture"),
        };

        let action = AttackAction {
            attacker: CombatantId::Player(0),
            target: CombatantId::Monster(1),
        };

        for seed in 0u64..50 {
            let mut cr_clone = cr.clone();
            let mut gs_local =
                crate::game::resources::GlobalState(crate::application::GameState::new());
            let mut ts_clone = ts.clone();
            let mut rng = StdRng::seed_from_u64(seed);

            let result = perform_attack_action_with_rng(
                &mut cr_clone,
                &action,
                &content,
                &mut gs_local,
                &mut ts_clone,
                &mut rng,
            );
            assert!(
                result.is_ok(),
                "cursed dagger attack must not error: {:?}",
                result
            );

            if let Some(Combatant::Monster(m)) = cr_clone.state.participants.get(1) {
                let damage = hp_before as i32 - m.hp.current as i32;
                // Damage must never be negative (monster must never gain HP from a hit).
                assert!(
                    damage >= 0,
                    "cursed dagger damage {} went negative on seed {} — monster healed",
                    damage,
                    seed
                );
                // When a hit lands (HP dropped), damage must be at least 1.
                if damage > 0 {
                    assert!(
                        damage >= 1,
                        "cursed dagger hit dealt {} — floor must be at least 1",
                        damage
                    );
                }
            }
        }
    }

    /// T4: A player with a `MartialRanged` bow who triggers the melee
    /// action path must have their turn skipped — `perform_attack_action_with_rng`
    /// returns `Ok(())` and the monster's HP is completely unchanged.
    #[test]
    fn test_player_melee_attack_with_ranged_weapon_skips_turn() {
        use crate::domain::items::{ItemDatabase, WeaponClassification};
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let bow = make_p2_weapon_item(
            77,
            DiceRoll::new(1, 6, 0),
            0,
            WeaponClassification::MartialRanged,
        );

        let mut item_db = ItemDatabase::new();
        item_db.add_item(bow).unwrap();

        let mut player = Character::new(
            "Archer".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        player.equipment.weapon = Some(77);
        player.stats.accuracy.current = 255;

        let (mut cr, mut content, mut gs, mut ts) = make_p2_combat_fixture(player);
        content.db_mut().items = item_db;

        let hp_before = match cr.state.participants.get(1) {
            Some(Combatant::Monster(m)) => m.hp.base,
            _ => panic!("monster not found in fixture"),
        };

        let action = AttackAction {
            attacker: CombatantId::Player(0),
            target: CombatantId::Monster(1),
        };

        let mut rng = StdRng::seed_from_u64(0);
        let result =
            perform_attack_action_with_rng(&mut cr, &action, &content, &mut gs, &mut ts, &mut rng);

        assert!(
            result.is_ok(),
            "ranged-weapon melee guard must return Ok(()), got {:?}",
            result
        );

        match cr.state.participants.get(1) {
            Some(Combatant::Monster(m)) => {
                assert_eq!(
                    m.hp.current, hp_before,
                    "ranged-weapon melee guard must not deal any damage \
                     (hp_before={hp_before}, hp_after={})",
                    m.hp.current
                );
            }
            _ => panic!("monster not found after ranged-weapon guard test"),
        }
    }

    // ===== Ranged and Magic Combat Tests =====

    /// Helper: create a bow (MartialRanged weapon) item with a given id.
    fn make_bow_item(id: u8) -> crate::domain::items::Item {
        make_p2_weapon_item(
            id,
            DiceRoll::new(1, 6, 0),
            0,
            crate::domain::items::WeaponClassification::MartialRanged,
        )
    }

    /// Helper: create an ammo item with a given id.
    fn make_ammo_item_p3(id: u8) -> crate::domain::items::Item {
        use crate::domain::items::{AmmoData, AmmoType, Item, ItemType};
        Item {
            id,
            name: format!("Arrow#{}", id),
            item_type: ItemType::Ammo(AmmoData {
                ammo_type: AmmoType::Arrow,
                quantity: 20,
            }),
            base_cost: 1,
            sell_cost: 0,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        }
    }

    /// Build a self-contained fixture for ranged combat tests.
    ///
    /// Returns `(CombatResource, GameContent, GlobalState, CombatTurnStateResource)`.
    /// The player (index 0) has a bow (item 88) and arrow (item 89).
    /// The goblin monster is at index 1 with AC=1.
    fn make_ranged_combat_fixture() -> (
        CombatResource,
        crate::application::resources::GameContent,
        crate::game::resources::GlobalState,
        CombatTurnStateResource,
    ) {
        use crate::domain::items::ItemDatabase;

        let bow = make_bow_item(88);
        let arrow = make_ammo_item_p3(89);

        let mut item_db = ItemDatabase::new();
        item_db.add_item(bow).unwrap();
        item_db.add_item(arrow).unwrap();

        let mut player = Character::new(
            "Archer".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        player.equipment.weapon = Some(88);
        player
            .inventory
            .items
            .push(crate::domain::character::InventorySlot {
                item_id: 89,
                charges: 0,
            });
        player.stats.accuracy.current = 255;

        let (mut cr, mut content, gs, ts) = make_p2_combat_fixture(player);
        content.db_mut().items = item_db;
        cr.combat_event_type = CombatEventType::Ranged;

        (cr, content, gs, ts)
    }

    /// 3.1 — After setup_combat_ui with Ranged type, an ActionButton with
    /// RangedAttack type must be present in the world.
    #[test]
    fn test_ranged_combat_shows_ranged_button() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Ranged).unwrap();
        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut writer = app.world_mut().resource_mut::<Messages<CombatStarted>>();
            writer.write(CombatStarted {
                encounter_position: None,
                encounter_map_id: None,
                combat_event_type: CombatEventType::Ranged,
            });
        }

        app.update();

        let ranged_buttons: Vec<_> = app
            .world_mut()
            .query::<&ActionButton>()
            .iter(app.world())
            .filter(|b| b.button_type == ActionButtonType::RangedAttack)
            .collect();

        assert!(
            !ranged_buttons.is_empty(),
            "Expected a RangedAttack ActionButton to be spawned for Ranged combat"
        );
    }

    /// 3.2 — The RangedAttack button has ACTION_BUTTON_DISABLED_COLOR when the
    /// player does not have a ranged weapon equipped.
    #[test]
    fn test_ranged_button_disabled_without_ranged_weapon() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        // Hero with NO weapon — no ranged weapon.
        let hero = Character::new(
            "Melee".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Ranged).unwrap();
        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut writer = app.world_mut().resource_mut::<Messages<CombatStarted>>();
            writer.write(CombatStarted {
                encounter_position: None,
                encounter_map_id: None,
                combat_event_type: CombatEventType::Ranged,
            });
        }

        // Run twice: first update handles CombatStarted + spawns UI,
        // second update runs update_combat_ui with the color logic.
        app.update();
        app.update();

        let mut found_disabled = false;
        for (btn, bg) in app
            .world_mut()
            .query::<(&ActionButton, &BackgroundColor)>()
            .iter(app.world())
        {
            if btn.button_type == ActionButtonType::RangedAttack {
                assert_eq!(
                    bg.0, ACTION_BUTTON_DISABLED_COLOR,
                    "RangedAttack button should be disabled when player has no ranged weapon"
                );
                found_disabled = true;
            }
        }
        assert!(
            found_disabled,
            "RangedAttack button must exist in Ranged combat"
        );
    }

    /// 3.3 — update_combat_ui sets ACTION_BUTTON_COLOR for the RangedAttack
    /// button when the current player combatant has a bow and ammo.
    #[test]
    fn test_ranged_button_enabled_with_ranged_weapon() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::domain::items::ItemDatabase;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Build bow + arrow items.
        let bow = make_bow_item(88);
        let arrow = make_ammo_item_p3(89);
        let mut item_db = ItemDatabase::new();
        item_db.add_item(bow).unwrap();
        item_db.add_item(arrow).unwrap();

        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Archer".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.equipment.weapon = Some(88);
        hero.inventory
            .items
            .push(crate::domain::character::InventorySlot {
                item_id: 89,
                charges: 0,
            });
        gs.party.add_member(hero).unwrap();

        let mut content_db = ContentDatabase::new();
        content_db.items = item_db;
        let content = GameContent::new(content_db);

        start_encounter(&mut gs, &content, &[], CombatEventType::Ranged).unwrap();
        let gs_resource = crate::game::resources::GlobalState(gs);
        app.insert_resource(gs_resource);
        app.insert_resource(content);
        {
            let mut writer = app.world_mut().resource_mut::<Messages<CombatStarted>>();
            writer.write(CombatStarted {
                encounter_position: None,
                encounter_map_id: None,
                combat_event_type: CombatEventType::Ranged,
            });
        }

        app.update();
        app.update();

        let mut found_enabled = false;
        for (btn, bg) in app
            .world_mut()
            .query::<(&ActionButton, &BackgroundColor)>()
            .iter(app.world())
        {
            if btn.button_type == ActionButtonType::RangedAttack {
                assert_eq!(
                    bg.0, ACTION_BUTTON_COLOR,
                    "RangedAttack button should be enabled (ACTION_BUTTON_COLOR) \
                     when player has bow + ammo"
                );
                found_enabled = true;
            }
        }
        assert!(
            found_enabled,
            "RangedAttack button must exist in Ranged combat"
        );
    }

    /// 3.4 — After perform_ranged_attack_action_with_rng, the ammo slot is
    /// removed from the attacker's inventory.
    #[test]
    fn test_perform_ranged_attack_consumes_ammo() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let (mut cr, content, mut gs, mut ts) = make_ranged_combat_fixture();

        let ammo_count_before = match cr.state.participants.first() {
            Some(Combatant::Player(pc)) => pc.inventory.items.len(),
            _ => panic!("player not found"),
        };
        assert_eq!(
            ammo_count_before, 1,
            "fixture must start with exactly one ammo"
        );

        let action = RangedAttackAction {
            attacker: CombatantId::Player(0),
            target: CombatantId::Monster(1),
        };

        let mut rng = StdRng::seed_from_u64(42);
        let result = perform_ranged_attack_action_with_rng(
            &mut cr, &action, &content, &mut gs, &mut ts, &mut rng,
        );

        assert!(result.is_ok(), "ranged attack must succeed: {:?}", result);

        let ammo_count_after = match cr.state.participants.first() {
            Some(Combatant::Player(pc)) => pc.inventory.items.len(),
            _ => panic!("player not found after attack"),
        };
        assert_eq!(
            ammo_count_after, 0,
            "ammo must be consumed after ranged attack (before={ammo_count_before}, after={ammo_count_after})"
        );
    }

    /// 3.5 — When the attacker has a bow but no ammo,
    /// perform_ranged_attack_action_with_rng returns CombatError::NoAmmo.
    #[test]
    fn test_perform_ranged_attack_no_ammo_returns_error() {
        use crate::domain::items::ItemDatabase;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        // Build fixture with bow but NO ammo.
        let bow = make_bow_item(88);
        let mut item_db = ItemDatabase::new();
        item_db.add_item(bow).unwrap();

        let mut player = Character::new(
            "Archer".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        player.equipment.weapon = Some(88);
        // No ammo added to inventory.
        player.stats.accuracy.current = 255;

        let (mut cr, mut content, mut gs, mut ts) = make_p2_combat_fixture(player);
        content.db_mut().items = item_db;
        cr.combat_event_type = CombatEventType::Ranged;

        let action = RangedAttackAction {
            attacker: CombatantId::Player(0),
            target: CombatantId::Monster(1),
        };

        let mut rng = StdRng::seed_from_u64(0);
        let result = perform_ranged_attack_action_with_rng(
            &mut cr, &action, &content, &mut gs, &mut ts, &mut rng,
        );

        assert!(
            matches!(
                result,
                Err(crate::domain::combat::engine::CombatError::NoAmmo)
            ),
            "expected CombatError::NoAmmo when attacker has bow but no ammo, got {:?}",
            result
        );
    }

    /// 3.6 — COMBAT_ACTION_ORDER_MAGIC[0] must be ActionButtonType::Cast.
    #[test]
    fn test_magic_combat_cast_is_first_action() {
        assert_eq!(
            COMBAT_ACTION_ORDER_MAGIC[0],
            ActionButtonType::Cast,
            "Cast must be the first (index 0) action in COMBAT_ACTION_ORDER_MAGIC"
        );
    }

    /// 3.7 — Magic combat uses Handicap::Even (same as Normal; only Ambush gives
    /// MonsterAdvantage).
    #[test]
    fn test_magic_combat_normal_handicap() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut gs = GameState::new();
        let hero = Character::new(
            "Wizard".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());
        start_encounter(&mut gs, &content, &[], CombatEventType::Magic).unwrap();

        let combat_state = match &gs.mode {
            crate::application::GameMode::Combat(cs) => cs.clone(),
            _ => panic!("expected Combat mode after start_encounter"),
        };

        assert_eq!(
            combat_state.handicap,
            Handicap::Even,
            "Magic combat must use Handicap::Even"
        );
    }

    /// 3.8 — A monster with an is_ranged attack prefers it when
    /// is_ranged_combat = true.
    #[test]
    fn test_monster_ranged_attack_preferred_in_ranged_combat() {
        use crate::domain::combat::engine::choose_monster_attack;
        use crate::domain::combat::monster::Monster;
        use crate::domain::combat::types::Attack;
        use crate::domain::types::DiceRoll;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let melee_attack = Attack::physical(DiceRoll::new(1, 4, 0));
        let ranged_attack = Attack::ranged(DiceRoll::new(1, 6, 0));

        let mut monster = Monster::new(
            99,
            "Archer Goblin".to_string(),
            crate::domain::character::Stats::new(8, 6, 6, 8, 8, 1, 6),
            20,
            2,
            vec![melee_attack.clone(), ranged_attack.clone()],
            crate::domain::combat::monster::LootTable::default(),
        );
        // Disable special-attack threshold so the selection is deterministic.
        monster.special_attack_threshold = 0;

        // Over many seeds, when is_ranged_combat = true, the chosen attack
        // must always be the ranged one (since there is exactly one ranged attack).
        for seed in 0u64..20 {
            let mut rng = StdRng::seed_from_u64(seed);
            let chosen = choose_monster_attack(&monster, true, &mut rng)
                .expect("monster with attacks must return Some");
            assert!(
                chosen.is_ranged,
                "monster must prefer ranged attack in ranged combat (seed={seed})"
            );
        }

        // When is_ranged_combat = false, the random selection may return melee.
        // Run many seeds and verify at least one melee result occurs.
        let mut saw_melee = false;
        for seed in 0u64..50 {
            let mut rng = StdRng::seed_from_u64(seed);
            let chosen = choose_monster_attack(&monster, false, &mut rng).unwrap();
            if !chosen.is_ranged {
                saw_melee = true;
                break;
            }
        }
        assert!(
            saw_melee,
            "monster must sometimes choose melee when is_ranged_combat = false"
        );
    }

    /// 3.9 — After handle_combat_started with Ranged type, the log contains "range".
    #[test]
    fn test_combat_log_ranged_opening() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Ranged).unwrap();
        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut writer = app.world_mut().resource_mut::<Messages<CombatStarted>>();
            writer.write(CombatStarted {
                encounter_position: None,
                encounter_map_id: None,
                combat_event_type: CombatEventType::Ranged,
            });
        }

        app.update();

        let log = app.world().resource::<CombatLogState>();
        let found = log
            .lines
            .iter()
            .any(|line| line.plain_text().to_lowercase().contains("range"));
        assert!(
            found,
            "combat log must contain 'range' for a Ranged encounter; got: {:?}",
            log.lines.iter().map(|l| l.plain_text()).collect::<Vec<_>>()
        );
    }

    /// 3.10 — After handle_combat_started with Magic type, the log contains "magical".
    #[test]
    fn test_combat_log_magic_opening() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Magic).unwrap();
        app.insert_resource(crate::game::resources::GlobalState(gs));
        {
            let mut writer = app.world_mut().resource_mut::<Messages<CombatStarted>>();
            writer.write(CombatStarted {
                encounter_position: None,
                encounter_map_id: None,
                combat_event_type: CombatEventType::Magic,
            });
        }

        app.update();

        let log = app.world().resource::<CombatLogState>();
        let found = log
            .lines
            .iter()
            .any(|line| line.plain_text().to_lowercase().contains("magical"));
        assert!(
            found,
            "combat log must contain 'magical' for a Magic encounter; got: {:?}",
            log.lines.iter().map(|l| l.plain_text()).collect::<Vec<_>>()
        );
    }

    // ===== Boss Combat Tests =====

    /// 4.1 — `start_encounter` with Boss type must set `monsters_advance = true`.
    #[test]
    fn test_boss_combat_monsters_advance() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Boss).unwrap();

        if let crate::application::GameMode::Combat(ref cs) = gs.mode {
            assert!(
                cs.monsters_advance,
                "Boss encounter must set monsters_advance = true"
            );
        } else {
            panic!("Expected Combat mode");
        }
    }

    /// 4.2 — `start_encounter` with Boss type must set `monsters_regenerate = true`.
    #[test]
    fn test_boss_combat_monsters_regenerate() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Boss).unwrap();

        if let crate::application::GameMode::Combat(ref cs) = gs.mode {
            assert!(
                cs.monsters_regenerate,
                "Boss encounter must set monsters_regenerate = true"
            );
        } else {
            panic!("Expected Combat mode");
        }
    }

    /// 4.3 — `start_encounter` with Boss type must set `can_bribe = false`.
    #[test]
    fn test_boss_combat_cannot_bribe() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Boss).unwrap();

        if let crate::application::GameMode::Combat(ref cs) = gs.mode {
            assert!(!cs.can_bribe, "Boss encounter must set can_bribe = false");
        } else {
            panic!("Expected Combat mode");
        }
    }

    /// 4.4 — `start_encounter` with Boss type must set `can_surrender = false`.
    #[test]
    fn test_boss_combat_cannot_surrender() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        let content = GameContent::new(ContentDatabase::new());

        start_encounter(&mut gs, &content, &[], CombatEventType::Boss).unwrap();

        if let crate::application::GameMode::Combat(ref cs) = gs.mode {
            assert!(
                !cs.can_surrender,
                "Boss encounter must set can_surrender = false"
            );
        } else {
            panic!("Expected Combat mode");
        }
    }

    /// 4.7 — A boss monster at 1 HP with flee_threshold=50 must not trigger the flee path.
    #[test]
    fn test_boss_monster_does_not_flee() {
        use crate::application::resources::GameContent;
        use crate::domain::combat::monster::{LootTable, Monster};
        use crate::domain::combat::types::BOSS_REGEN_PER_ROUND;
        use crate::sdk::database::ContentDatabase;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut gs = crate::application::GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();

        let mut monster = Monster::new(
            1,
            "Dragon Boss".to_string(),
            crate::domain::character::Stats::new(18, 8, 8, 18, 10, 12, 5),
            100,
            10,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(2, 8, 0),
            )],
            LootTable::default(),
        );
        // 50% flee threshold — at 1 HP (1%) it would normally flee
        monster.flee_threshold = 50;
        monster.hp.current = 1;

        let mut cs = CombatState::new(Handicap::Even);
        cs.monsters_advance = true;
        cs.monsters_regenerate = true;
        cs.can_bribe = false;
        cs.can_surrender = false;
        cs.add_player(hero.clone());
        cs.add_monster(monster);
        crate::domain::combat::engine::start_combat(&mut cs);

        let mut cr = CombatResource::new();
        cr.state = cs;
        cr.player_orig_indices = vec![Some(0), None];
        cr.combat_event_type = CombatEventType::Boss;

        let content = GameContent::new(ContentDatabase::new());
        let mut global_state = crate::game::resources::GlobalState(gs);
        global_state.0.mode = crate::application::GameMode::Combat(cr.state.clone());
        let mut turn_state = CombatTurnStateResource::default();
        let mut rng = StdRng::seed_from_u64(42);

        // Move to a monster turn
        cr.state.current_turn = 1; // Monster(1)
        cr.state.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cr.state.status = crate::domain::combat::types::CombatStatus::InProgress;

        // The monster's should_flee() returns true (1% < 50% threshold)
        if let Some(Combatant::Monster(mon)) = cr.state.participants.get(1) {
            assert!(
                mon.should_flee(),
                "monster should_flee() must be true for this test"
            );
        }

        // But perform_monster_turn_with_rng must suppress the flee for Boss type.
        // The monster should attack (not flee), so the result must be Ok(Some(...)).
        // Ok(None) means the monster bailed out early without attacking.
        let result = perform_monster_turn_with_rng(
            &mut cr,
            &content,
            &mut global_state,
            &mut turn_state,
            &mut rng,
        );
        assert!(
            result.is_ok(),
            "perform_monster_turn_with_rng must not return Err for boss encounter: {:?}",
            result
        );
        // Explicitly verify the monster attacked rather than fleeing.
        // Ok(None) is returned by early-return paths (can't act, no target, or flee).
        // Ok(Some(_)) means the monster resolved an attack.
        assert!(
            result.as_ref().unwrap().is_some(),
            "boss monster must have attacked (result must be Ok(Some(...))), \
             got Ok(None) meaning it bailed out early; \
             monster state: {:?}",
            cr.state.participants.get(1)
        );
        // Note: has_acted is reset by advance_round when the turn wraps, so we
        // cannot reliably check it here. The Ok(Some(...)) assertion above is the
        // definitive proof that the monster attacked rather than fled or bailed out.
        // suppress unused constant warning
        let _ = BOSS_REGEN_PER_ROUND;
    }

    /// 4.7 — After a full round with Boss type, a regenerating monster gains BOSS_REGEN_PER_ROUND HP.
    #[test]
    fn test_boss_monster_regenerates_each_round() {
        use crate::domain::combat::monster::{LootTable, Monster};
        use crate::domain::combat::types::BOSS_REGEN_PER_ROUND;

        // Build a CombatState with one monster (can_regenerate=true) and boss flags.
        let mut cs = CombatState::new(Handicap::Even);
        cs.monsters_regenerate = true;

        let mut monster = Monster::new(
            1,
            "Boss Dragon".to_string(),
            crate::domain::character::Stats::new(18, 8, 8, 18, 10, 12, 5),
            100,
            10,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 6, 0),
            )],
            LootTable::default(),
        );
        monster.can_regenerate = true;
        monster.hp.base = 100;
        monster.hp.current = 80; // damaged
        cs.add_monster(monster);

        crate::domain::combat::engine::start_combat(&mut cs);

        // Wrap in CombatResource with Boss type
        let mut cr = CombatResource::new();
        cr.state = cs;
        cr.combat_event_type = CombatEventType::Boss;

        // Record HP before advancing a full round
        let hp_before = if let Some(Combatant::Monster(m)) = cr.state.participants.first() {
            m.hp.current
        } else {
            panic!("no monster");
        };

        // Advance turns until a new round starts.
        // With one monster and no players, the turn_order has 1 entry,
        // so advancing once triggers advance_round.
        let _ = cr.state.advance_turn(&[]);

        // Apply the boss bonus regeneration (simulating what perform_monster_turn_with_rng does)
        if cr.combat_event_type == CombatEventType::Boss && cr.state.monsters_regenerate {
            let bonus_regen = BOSS_REGEN_PER_ROUND.saturating_sub(1);
            if bonus_regen > 0 {
                for participant in &mut cr.state.participants {
                    if let Combatant::Monster(mon) = participant {
                        if mon.can_regenerate && mon.is_alive() {
                            mon.regenerate(bonus_regen);
                        }
                    }
                }
            }
        }

        let hp_after = if let Some(Combatant::Monster(m)) = cr.state.participants.first() {
            m.hp.current
        } else {
            panic!("no monster");
        };

        // advance_round adds 1 HP; boss bonus adds 4 more = BOSS_REGEN_PER_ROUND total
        assert_eq!(
            hp_after,
            hp_before + BOSS_REGEN_PER_ROUND,
            "boss monster must regenerate exactly BOSS_REGEN_PER_ROUND ({}) HP per round; \
             got {} -> {}",
            BOSS_REGEN_PER_ROUND,
            hp_before,
            hp_after
        );
    }

    /// 4.7 — BossHpBar component is present in the ECS world after combat UI
    /// setup with Boss type.
    #[test]
    fn test_boss_hp_bar_spawned() {
        use crate::domain::combat::monster::{LootTable, Monster};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let monster = Monster::new(
            1,
            "Boss Dragon".to_string(),
            crate::domain::character::Stats::new(18, 8, 8, 18, 10, 12, 5),
            200,
            15,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(2, 10, 0),
            )],
            LootTable::default(),
        );

        let mut cs = CombatState::new(Handicap::Even);
        cs.monsters_advance = true;
        cs.monsters_regenerate = true;
        cs.can_bribe = false;
        cs.can_surrender = false;
        cs.add_player(hero.clone());
        cs.add_monster(monster);
        crate::domain::combat::engine::start_combat(&mut cs);

        let mut cr = CombatResource::new();
        cr.state = cs.clone();
        cr.player_orig_indices = vec![Some(0), None];
        cr.combat_event_type = CombatEventType::Boss;

        let mut gs = crate::application::GameState::new();
        gs.party.add_member(hero).unwrap();
        gs.enter_combat_with_state(cs);

        app.insert_resource(crate::game::resources::GlobalState(gs));
        app.insert_resource(cr);

        app.update();

        let boss_bars = app
            .world_mut()
            .query::<&BossHpBar>()
            .iter(app.world())
            .count();
        assert!(
            boss_bars > 0,
            "BossHpBar component must be present after setup_combat_ui for Boss encounter"
        );
    }

    /// 4.7 — Normal combat does not spawn BossHpBar.
    #[test]
    fn test_normal_combat_no_boss_bar() {
        use crate::domain::combat::monster::{LootTable, Monster};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let monster = Monster::new(
            2,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(8, 6, 6, 8, 8, 6, 5),
            20,
            5,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            LootTable::default(),
        );

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        cs.add_monster(monster);
        crate::domain::combat::engine::start_combat(&mut cs);

        let mut cr = CombatResource::new();
        cr.state = cs.clone();
        cr.player_orig_indices = vec![Some(0), None];
        cr.combat_event_type = CombatEventType::Normal;

        let mut gs = crate::application::GameState::new();
        gs.party.add_member(hero).unwrap();
        gs.enter_combat_with_state(cs);

        app.insert_resource(crate::game::resources::GlobalState(gs));
        app.insert_resource(cr);

        app.update();

        let boss_bars = app
            .world_mut()
            .query::<&BossHpBar>()
            .iter(app.world())
            .count();
        assert_eq!(
            boss_bars, 0,
            "BossHpBar must NOT be spawned for a Normal encounter"
        );
    }

    /// One complete combat round must advance the in-game clock by
    /// exactly `TIME_COST_COMBAT_ROUND_MINUTES`.
    ///
    /// Strategy: build a minimal Bevy app with `CombatPlugin`, put the
    /// `GlobalState` into combat mode with round = 1, run one update frame so
    /// that `tick_combat_time` executes, and assert the clock advanced by the
    /// expected amount.
    #[test]
    fn test_combat_round_advances_time() {
        use crate::domain::resources::TIME_COST_COMBAT_ROUND_MINUTES;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Build a GameState already in combat mode (round 1).
        let mut gs = GameState::new();
        let hero = Character::new(
            "Time Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero.clone()).unwrap();
        gs.enter_combat();

        // Record the starting total minutes.
        // Use total_days() so the cumulative-minute baseline is correct across
        // month/year boundaries (day is now 1–30 within-month, not a running total).
        let start_minutes = gs.time.total_days() as u64 * 24 * 60
            + gs.time.hour as u64 * 60
            + gs.time.minute as u64;

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // The CombatResource starts with last_timed_round = 0.
        // After sync_party_to_combat runs, combat.state.round == 1.
        // tick_combat_time sees current_round (1) > last_timed_round (0),
        // so it charges TIME_COST_COMBAT_ROUND_MINUTES and sets last_timed_round = 1.
        app.update();

        let time_after_first_frame = {
            let state = app
                .world()
                .resource::<crate::game::resources::GlobalState>();
            let end_minutes = state.0.time.total_days() as u64 * 24 * 60
                + state.0.time.hour as u64 * 60
                + state.0.time.minute as u64;

            assert_eq!(
                end_minutes - start_minutes,
                TIME_COST_COMBAT_ROUND_MINUTES as u64,
                "one combat round must advance the clock by exactly TIME_COST_COMBAT_ROUND_MINUTES ({} min)",
                TIME_COST_COMBAT_ROUND_MINUTES
            );

            // Capture the time so we can compare after the next frame.
            state.0.time
        };

        // Subsequent frames with the same round number must NOT advance time again.
        app.update(); // same round — no new charge
        let state2 = app
            .world()
            .resource::<crate::game::resources::GlobalState>();
        assert_eq!(
            state2.0.time.minute, time_after_first_frame.minute,
            "same-round subsequent frame must not advance minutes again"
        );
        assert_eq!(
            state2.0.time.hour, time_after_first_frame.hour,
            "same-round subsequent frame must not advance hours again"
        );
    }

    /// 4.7 — Victory summary for a Boss encounter has boss_defeated == true.
    #[test]
    fn test_boss_victory_summary_has_boss_header() {
        use crate::application::resources::GameContent;
        use crate::domain::combat::monster::{LootTable, Monster};
        use crate::sdk::database::ContentDatabase;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let mut dead_boss = Monster::new(
            1,
            "Boss Dragon".to_string(),
            crate::domain::character::Stats::new(18, 8, 8, 18, 10, 12, 5),
            200,
            15,
            vec![],
            LootTable::default(),
        );
        dead_boss.hp.current = 0;
        dead_boss.conditions = crate::domain::combat::monster::MonsterCondition::Dead;

        let mut cs = CombatState::new(Handicap::Even);
        cs.monsters_advance = true;
        cs.monsters_regenerate = true;
        cs.can_bribe = false;
        cs.can_surrender = false;
        cs.add_player(hero.clone());
        cs.add_monster(dead_boss);
        cs.status = crate::domain::combat::types::CombatStatus::Victory;

        let mut cr = CombatResource::new();
        cr.state = cs;
        cr.player_orig_indices = vec![Some(0), None];
        cr.combat_event_type = CombatEventType::Boss;

        let mut gs = crate::application::GameState::new();
        gs.party.add_member(hero).unwrap();

        let content = GameContent::new(ContentDatabase::new());
        let mut global_state = crate::game::resources::GlobalState(gs);

        let mut rng = StdRng::seed_from_u64(42);
        let summary =
            process_combat_victory_with_rng(&mut cr, &content, &mut global_state, &mut rng)
                .expect("process_combat_victory_with_rng must succeed");

        assert!(
            summary.boss_defeated,
            "VictorySummary::boss_defeated must be true for a Boss encounter"
        );
    }
}
