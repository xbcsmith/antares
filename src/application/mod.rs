// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Application layer - Game state management and orchestration
//!
//! This module contains the application logic that coordinates
//! the domain layer components. It manages the overall game state
//! and game mode transitions.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.1 for complete specifications.

pub mod character_sheet_state;
pub mod container_inventory_state;
pub mod dialogue;
pub mod inventory_state;
pub mod menu;
pub mod merchant_inventory_state;
pub mod quests;
pub mod resources;
pub mod save_game;
pub mod spell_book_state;
pub mod spell_casting_state;

use crate::application::menu::MenuState;
use crate::application::save_game::SavedLogEntry;
use crate::domain::campaign::CampaignConfig;
use crate::domain::character::{Party, Roster};
use crate::domain::party_manager::{PartyManagementError, PartyManager};
use crate::domain::types::{GameTime, InnkeeperId, TimeOfDay};
use crate::domain::world::npc_runtime::NpcRuntimeStore;
use crate::domain::world::World;
use crate::sdk::campaign_loader::{Campaign, CampaignError};
use crate::sdk::database::ContentDatabase;
use crate::sdk::game_config::GameConfig;
use serde::{Deserialize, Serialize};

// ===== Game Mode =====

/// Game mode enum representing different states of the game
///
/// # Examples
///
/// ```
/// use antares::application::GameMode;
///
/// let mode = GameMode::Exploration;
/// assert!(matches!(mode, GameMode::Exploration));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameMode {
    /// Exploring the world, moving through maps
    Exploration,
    /// Turn-based tactical combat containing full combat state
    Combat(crate::domain::combat::engine::CombatState),
    /// Full-screen automap overlay is open
    Automap,
    /// Inventory management screen
    Inventory(crate::application::inventory_state::InventoryState),
    /// Inn party management interface
    InnManagement(InnManagementState),
    /// Menu system (character management, inventory)
    Menu(MenuState),
    /// NPC dialogue and interactions
    Dialogue(crate::application::dialogue::DialogueState),
    /// Merchant buy/sell split-screen inventory (opened with `I` during Dialogue
    /// with a merchant NPC).
    MerchantInventory(crate::application::merchant_inventory_state::MerchantInventoryState),
    /// Container interaction split-screen inventory (opened with `E` when
    /// facing a chest, crate, hole in the wall, etc.).
    ContainerInventory(crate::application::container_inventory_state::ContainerInventoryState),
    /// Priest/Temple resurrection service interface.
    ///
    /// Entered when the player interacts with an NPC that has `is_priest: true`
    /// and a `service_catalog` containing the `"resurrect"` service.  Displays
    /// dead party members and allows the player to spend gold and gems to revive
    /// them via [`crate::application::resources::perform_resurrection_service`].
    TempleService(TempleServiceState),
    /// Rest duration selection menu.
    ///
    /// Shown when the player presses R in Exploration mode.  Presents three
    /// choices: 4 hours (50% HP/SP), 8 hours (75% HP/SP), 12 hours (100%
    /// HP/SP).  Selecting a choice fires [`InitiateRestEvent`] with the
    /// chosen duration and transitions to [`GameMode::Resting`].
    /// Pressing Escape or Menu while here returns to Exploration.
    RestMenu,
    /// Party is resting — per-hour healing loop is running.
    ///
    /// Input is blocked during this mode (except `GameAction::Menu` which
    /// cancels the rest in a future enhancement). The orchestration system
    /// drives the rest sequence one hour per Bevy frame and detect encounter
    /// interruptions.
    Resting(RestState),
    /// Full-screen scrollable game log overlay.
    ///
    /// Entered by pressing the fullscreen log toggle key (default `G`) or
    /// clicking the "Game Log" header in the small log panel.  ESC returns
    /// to the previous [`GameMode`].
    GameLog,
    /// All party members are dead — the game is over.
    ///
    /// Entered when a trap, combat, or other hazard kills every living party
    /// member.  The UI should display a "Game Over" screen with options to
    /// load a save or quit.
    GameOver,
    /// Exploration-mode spell casting flow.
    ///
    /// Entered when a player opens the spell menu outside of combat (default
    /// key `C`).  The multi-step flow is tracked by
    /// [`crate::application::spell_casting_state::SpellCastingState`]:
    /// caster selection → spell selection → optional target selection → result.
    SpellCasting(crate::application::spell_casting_state::SpellCastingState),
    /// Read-only in-game Spell Book management screen.
    ///
    /// Entered when the player presses the Spell Book key (default `B`) in
    /// Exploration mode.  The player can browse each caster's known spells,
    /// view SP status, read spell descriptions, and inspect learnable scrolls
    /// in the character's inventory — without casting.
    ///
    /// Stored state is tracked by
    /// [`crate::application::spell_book_state::SpellBookState`]:
    /// which character's book is open, which spell is highlighted, and which
    /// mode to restore on close.
    SpellBook(crate::application::spell_book_state::SpellBookState),
    /// NPC trainer level-up session.
    ///
    /// Entered when a dialogue node fires [`crate::domain::dialogue::DialogueAction::OpenTraining`]
    /// for an NPC with `is_trainer: true` and the campaign uses
    /// [`crate::domain::campaign::LevelUpMode::NpcTrainer`].  The UI presents
    /// eligible party members and their training costs.
    Training(TrainingState),
    /// Read-only character stats viewer (out of combat).
    ///
    /// Entered by pressing the character sheet key (default `P`) in Exploration
    /// mode. Tab / Shift-Tab cycles through party members. Esc returns to the
    /// previous [`GameMode`].
    CharacterSheet(crate::application::character_sheet_state::CharacterSheetState),
}

// ===== Rest State =====

/// Tracks progress of an in-progress party rest sequence.
///
/// Stored inside [`GameMode::Resting`] so that the rest orchestration system
/// can advance the sequence one hour per Bevy frame and detect encounter
/// interruptions.
///
/// A save made while resting serialises this state, so loading the save
/// correctly resumes the rest sequence.
///
/// # Examples
///
/// ```
/// use antares::application::RestState;
///
/// let state = RestState::new(12);
/// assert_eq!(state.hours_requested, 12);
/// assert_eq!(state.hours_completed, 0);
/// assert!(!state.interrupted);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestState {
    /// Total hours of rest requested (e.g. 12 for a full rest).
    pub hours_requested: u32,
    /// Hours of rest completed so far.
    pub hours_completed: u32,
    /// Set when a random encounter interrupts the rest before completion.
    pub interrupted: bool,
    /// HP/SP fraction of each character's base to restore per hour tick.
    ///
    /// Derived from the chosen [`RestDuration`]: Short → 0.125, Long →
    /// 0.09375, Full → 0.08333.  Stored here so the per-hour healing loop
    /// doesn't need to recompute it and so save/load restores the correct
    /// rate when resuming a rest in progress.
    pub restore_fraction_per_hour: f32,
}

impl RestState {
    /// Creates a new `RestState` for the given number of requested hours.
    ///
    /// # Arguments
    ///
    /// * `hours` — total hours of rest to attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::RestState;
    ///
    /// let s = RestState::new(6);
    /// assert_eq!(s.hours_requested, 6);
    /// assert_eq!(s.hours_completed, 0);
    /// assert!(!s.interrupted);
    /// ```
    /// Creates a `RestState` with an explicit per-hour restore fraction.
    ///
    /// Use this constructor when initiating a rest from the rest-duration
    /// menu so the correct heal rate is stored alongside the hour count.
    ///
    /// # Arguments
    ///
    /// * `hours`                    — total in-game hours to rest.
    /// * `restore_fraction_per_hour` — HP/SP fraction to restore each hour.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::RestState;
    /// use antares::domain::resources::RestDuration;
    ///
    /// let d = RestDuration::Full;
    /// let s = RestState::with_fraction(d.hours(), d.restore_fraction_per_hour());
    /// assert_eq!(s.hours_requested, 12);
    /// assert!((s.restore_fraction_per_hour - d.restore_fraction_per_hour()).abs() < 1e-6);
    /// ```
    pub fn with_fraction(hours: u32, restore_fraction_per_hour: f32) -> Self {
        Self {
            hours_requested: hours,
            hours_completed: 0,
            interrupted: false,
            restore_fraction_per_hour,
        }
    }

    /// Creates a `RestState` for a full 12-hour rest (100% HP/SP).
    ///
    /// Convenience wrapper used by tests and legacy call sites.
    /// Prefer [`RestState::with_fraction`] for player-initiated rests so the
    /// chosen duration's heal rate is stored correctly.
    pub fn new(hours: u32) -> Self {
        use crate::domain::resources::{RestDuration, HP_RESTORE_RATE};
        // Use the correct fraction for known durations; fall back to the
        // full-rest rate for any other hour count.
        let fraction = RestDuration::from_hours(hours)
            .map(|d| d.restore_fraction_per_hour())
            .unwrap_or(HP_RESTORE_RATE);
        Self {
            hours_requested: hours,
            hours_completed: 0,
            interrupted: false,
            restore_fraction_per_hour: fraction,
        }
    }

    /// Returns `true` when all requested hours have been completed.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::RestState;
    ///
    /// let mut s = RestState::new(2);
    /// assert!(!s.is_complete());
    /// s.hours_completed = 2;
    /// assert!(s.is_complete());
    /// ```
    pub fn is_complete(&self) -> bool {
        self.hours_completed >= self.hours_requested
    }
}

/// State for inn party management mode
///
/// Tracks which inn the party is at and any active selections
/// for recruit/dismiss/swap operations.
///
/// # Examples
///
/// ```
/// use antares::application::InnManagementState;
/// use antares::domain::types::InnkeeperId;
///
/// let state = InnManagementState::new("tutorial_innkeeper_town".to_string());
/// assert_eq!(state.current_inn_id, "tutorial_innkeeper_town".to_string());
/// assert_eq!(state.selected_party_slot, None);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InnManagementState {
    /// ID of the inn currently being visited (Innkeeper NPC ID)
    pub current_inn_id: InnkeeperId,
    /// Currently selected party member slot (0-5) for swap operations
    pub selected_party_slot: Option<usize>,
    /// Currently selected roster index for swap operations
    pub selected_roster_slot: Option<usize>,
}

// ===== Temple Service State =====

/// State for an active priest/temple resurrection service session.
///
/// Stored inside [`GameMode::TempleService`] while the player is interacting
/// with a priest NPC that offers the `"resurrect"` service.
///
/// # Examples
///
/// ```
/// use antares::application::TempleServiceState;
///
/// let state = TempleServiceState::new("tutorial_priestess_town".to_string());
/// assert_eq!(state.npc_id, "tutorial_priestess_town");
/// assert!(state.selected_member_index.is_none());
/// assert!(state.status_message.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TempleServiceState {
    /// The NPC ID of the priest conducting the service (e.g., `"temple_priest"`)
    pub npc_id: String,
    /// Currently selected party-member index in the list of dead members shown by the UI
    pub selected_member_index: Option<usize>,
    /// Last status or error message to display in the UI (`None` when idle)
    pub status_message: Option<String>,
}

impl TempleServiceState {
    /// Creates a new temple service state for the given priest NPC.
    ///
    /// # Arguments
    ///
    /// * `npc_id` - The NPC ID of the priest (e.g., `"tutorial_priestess_town"`)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::TempleServiceState;
    ///
    /// let state = TempleServiceState::new("tutorial_priestess_town".to_string());
    /// assert_eq!(state.npc_id, "tutorial_priestess_town");
    /// assert!(state.selected_member_index.is_none());
    /// ```
    pub fn new(npc_id: String) -> Self {
        Self {
            npc_id,
            selected_member_index: None,
            status_message: None,
        }
    }

    /// Clears the current selection and status message.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::TempleServiceState;
    ///
    /// let mut state = TempleServiceState::new("temple_priest".to_string());
    /// state.selected_member_index = Some(0);
    /// state.status_message = Some("Resurrection complete!".to_string());
    /// state.clear();
    /// assert!(state.selected_member_index.is_none());
    /// assert!(state.status_message.is_none());
    /// ```
    pub fn clear(&mut self) {
        self.selected_member_index = None;
        self.status_message = None;
    }
}

// ===== Training State =====

/// State for an active NPC trainer level-up session.
///
/// Stored inside [`GameMode::Training`] while the player is interacting
/// with a trainer NPC that offers level-up services.
///
/// # Examples
///
/// ```
/// use antares::application::TrainingState;
///
/// let state = TrainingState::new("master_swordsman".to_string());
/// assert_eq!(state.npc_id, "master_swordsman");
/// assert!(state.eligible_member_indices.is_empty());
/// assert!(state.selected_member_index.is_none());
/// assert!(state.status_message.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingState {
    /// The NPC ID of the trainer conducting the session (e.g., `"master_swordsman"`)
    pub npc_id: String,
    /// Indices into `party.members` for members eligible for level-up
    pub eligible_member_indices: Vec<usize>,
    /// Currently selected party-member index in the eligible list
    pub selected_member_index: Option<usize>,
    /// Last status or error message to display in the UI (`None` when idle)
    pub status_message: Option<String>,
}

impl TrainingState {
    /// Creates a new training state for the given trainer NPC.
    ///
    /// # Arguments
    ///
    /// * `npc_id` - The NPC ID of the trainer (e.g., `"master_swordsman"`)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::TrainingState;
    ///
    /// let state = TrainingState::new("master_swordsman".to_string());
    /// assert_eq!(state.npc_id, "master_swordsman");
    /// assert!(state.eligible_member_indices.is_empty());
    /// ```
    pub fn new(npc_id: String) -> Self {
        Self {
            npc_id,
            eligible_member_indices: Vec::new(),
            selected_member_index: None,
            status_message: None,
        }
    }

    /// Clears the current selection and status message.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::TrainingState;
    ///
    /// let mut state = TrainingState::new("trainer".to_string());
    /// state.selected_member_index = Some(0);
    /// state.status_message = Some("Training complete!".to_string());
    /// state.clear();
    /// assert!(state.selected_member_index.is_none());
    /// assert!(state.status_message.is_none());
    /// ```
    pub fn clear(&mut self) {
        self.selected_member_index = None;
        self.status_message = None;
    }
}

impl InnManagementState {
    /// Creates a new inn management state for the given inn
    ///
    /// # Arguments
    ///
    /// * `inn_id` - The Innkeeper NPC ID (e.g., "tutorial_innkeeper_town")
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::InnManagementState;
    /// use antares::domain::types::InnkeeperId;
    ///
    /// let state = InnManagementState::new("tutorial_innkeeper_town".to_string());
    /// assert_eq!(state.current_inn_id, "tutorial_innkeeper_town".to_string());
    /// ```
    pub fn new(inn_id: InnkeeperId) -> Self {
        Self {
            current_inn_id: inn_id,
            selected_party_slot: None,
            selected_roster_slot: None,
        }
    }

    /// Clears all selections
    pub fn clear_selection(&mut self) {
        self.selected_party_slot = None;
        self.selected_roster_slot = None;
    }
}

// ===== Active Spell Effects =====

/// Party-wide active spell effects (separate from character conditions).
///
/// Each `u8` field represents the number of in-game minutes remaining for that
/// effect.  A value of `0` means the effect is not active.  Fields are
/// decremented once per minute by [`ActiveSpells::tick`], which is called
/// inside [`GameState::advance_time`] for every minute advanced.
///
/// ## Timed Resistance Potions
///
/// When a campaign author gives a consumable item the
/// `ConsumableEffect::BoostResistance` effect **and** a non-zero
/// `duration_minutes`, the exploration executor routes the use through
/// [`crate::domain::items::consumable_usage::apply_consumable_effect_exploration`],
/// which writes the clamped duration directly into the relevant field of this
/// struct (e.g. `fire_protection = 60` for a 60-minute fire potion).
///
/// During combat, [`ActiveSpells::effective_resistance`] is consulted for each
/// incoming attack.  If the matching protection field is non-zero, the character
/// receives a flat [`ACTIVE_PROTECTION_BONUS`] added to their base resistance.
///
/// ## Relationship to `ACTIVE_PROTECTION_BONUS`
///
/// [`ACTIVE_PROTECTION_BONUS`] is a compile-time constant (`25`) representing
/// the flat resistance bonus granted while a protection is active.  It is
/// intentionally fixed — campaign authors control potion potency through the
/// `amount` field in `BoostResistance`; the `active_spells` fields store only
/// the remaining duration, not the magnitude.
///
/// ## Overwrite Semantics
///
/// Applying a second resistance potion while the same protection is still active
/// **overwrites** the remaining duration rather than stacking.  This matches the
/// Might and Magic 1 design philosophy: predictable, non-stacking consumables.
///
/// # Examples
///
/// ```
/// use antares::application::{ActiveSpells, ACTIVE_PROTECTION_BONUS};
/// use antares::domain::items::types::ResistanceType;
///
/// let mut spells = ActiveSpells::new();
/// spells.fire_protection = 60;
///
/// // Protection active — returns the flat bonus.
/// assert_eq!(spells.effective_resistance(ResistanceType::Fire), ACTIVE_PROTECTION_BONUS);
///
/// // Tick 60 times to expire.
/// for _ in 0..60 { spells.tick(); }
/// assert_eq!(spells.fire_protection, 0);
/// assert_eq!(spells.effective_resistance(ResistanceType::Fire), 0);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActiveSpells {
    /// Resistance to fear effects
    pub fear_protection: u8,
    /// Cold damage reduction
    pub cold_protection: u8,
    /// Fire damage reduction
    pub fire_protection: u8,
    /// Poison resistance
    pub poison_protection: u8,
    /// Acid damage reduction
    pub acid_protection: u8,
    /// Lightning resistance
    pub electricity_protection: u8,
    /// Magic damage reduction
    pub magic_protection: u8,
    /// Illumination radius
    pub light: u8,
    /// AC bonus
    pub leather_skin: u8,
    /// Avoid ground traps
    pub levitate: u8,
    /// Water traversal
    pub walk_on_water: u8,
    /// Alert for ambushes
    pub guard_dog: u8,
    /// Mental attack resistance
    pub psychic_protection: u8,
    /// Combat bonus
    pub bless: u8,
    /// Avoid encounters
    pub invisibility: u8,
    /// AC bonus
    pub shield: u8,
    /// Greater AC bonus
    pub power_shield: u8,
    /// Negative effects
    pub cursed: u8,
}

/// Bonus resistance points granted per active protection spell/potion.
///
/// When an `ActiveSpells` protection field is non-zero, this flat bonus is
/// added to the character's current resistance for the matching damage type
/// during combat damage resolution.
pub const ACTIVE_PROTECTION_BONUS: i16 = 25;

impl ActiveSpells {
    /// Creates a new ActiveSpells with all effects inactive
    pub fn new() -> Self {
        Self {
            fear_protection: 0,
            cold_protection: 0,
            fire_protection: 0,
            poison_protection: 0,
            acid_protection: 0,
            electricity_protection: 0,
            magic_protection: 0,
            light: 0,
            leather_skin: 0,
            levitate: 0,
            walk_on_water: 0,
            guard_dog: 0,
            psychic_protection: 0,
            bless: 0,
            invisibility: 0,
            shield: 0,
            power_shield: 0,
            cursed: 0,
        }
    }

    /// Returns the effective bonus resistance for a given [`ResistanceType`]
    /// contributed by active spell/potion protections.
    ///
    /// Returns [`ACTIVE_PROTECTION_BONUS`] (25) when the mapped `active_spells`
    /// field is non-zero (i.e. the protection is currently active), and `0`
    /// when the field has ticked down to zero (expired).
    ///
    /// Callers add this value to `character.resistances.<field>.current` to
    /// obtain the total effective resistance for a given damage type.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{ActiveSpells, ACTIVE_PROTECTION_BONUS};
    /// use antares::domain::items::types::ResistanceType;
    ///
    /// let mut spells = ActiveSpells::new();
    /// // No protection active — all types return 0.
    /// assert_eq!(spells.effective_resistance(ResistanceType::Fire), 0);
    ///
    /// // Activate fire protection for 30 minutes.
    /// spells.fire_protection = 30;
    /// assert_eq!(spells.effective_resistance(ResistanceType::Fire), ACTIVE_PROTECTION_BONUS);
    ///
    /// // Cold is still inactive.
    /// assert_eq!(spells.effective_resistance(ResistanceType::Cold), 0);
    /// ```
    pub fn effective_resistance(
        &self,
        res_type: crate::domain::items::types::ResistanceType,
    ) -> i16 {
        use crate::domain::items::types::ResistanceType;
        let active = match res_type {
            ResistanceType::Fire => self.fire_protection > 0,
            ResistanceType::Cold => self.cold_protection > 0,
            ResistanceType::Electricity => self.electricity_protection > 0,
            ResistanceType::Energy => self.magic_protection > 0,
            ResistanceType::Fear => self.fear_protection > 0,
            ResistanceType::Physical => self.magic_protection > 0,
            ResistanceType::Paralysis => self.psychic_protection > 0,
            ResistanceType::Sleep => self.psychic_protection > 0,
        };
        if active {
            ACTIVE_PROTECTION_BONUS
        } else {
            0
        }
    }

    /// Decrements all active spell durations by 1 (called each turn/minute)
    pub fn tick(&mut self) {
        self.fear_protection = self.fear_protection.saturating_sub(1);
        self.cold_protection = self.cold_protection.saturating_sub(1);
        self.fire_protection = self.fire_protection.saturating_sub(1);
        self.poison_protection = self.poison_protection.saturating_sub(1);
        self.acid_protection = self.acid_protection.saturating_sub(1);
        self.electricity_protection = self.electricity_protection.saturating_sub(1);
        self.magic_protection = self.magic_protection.saturating_sub(1);
        self.light = self.light.saturating_sub(1);
        self.leather_skin = self.leather_skin.saturating_sub(1);
        self.levitate = self.levitate.saturating_sub(1);
        self.walk_on_water = self.walk_on_water.saturating_sub(1);
        self.guard_dog = self.guard_dog.saturating_sub(1);
        self.psychic_protection = self.psychic_protection.saturating_sub(1);
        self.bless = self.bless.saturating_sub(1);
        self.invisibility = self.invisibility.saturating_sub(1);
        self.shield = self.shield.saturating_sub(1);
        self.power_shield = self.power_shield.saturating_sub(1);
        self.cursed = self.cursed.saturating_sub(1);
    }

    /// Resets all active spell effects to zero (e.g. when Dispel Magic is cast).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::ActiveSpells;
    ///
    /// let mut spells = ActiveSpells::new();
    /// spells.fire_protection = 30;
    /// spells.bless = 10;
    /// spells.reset();
    /// assert_eq!(spells.fire_protection, 0);
    /// assert_eq!(spells.bless, 0);
    /// ```
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

// ===== Quest Log =====

/// Quest objective tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestObjective {
    /// Objective description
    pub description: String,
    /// Whether completed
    pub completed: bool,
    /// Map identifier for objective-related POI display, if known
    #[serde(default)]
    pub map_id: Option<crate::domain::types::MapId>,
    /// Objective position for automap / mini map POI display, if known
    #[serde(default)]
    pub position: Option<crate::domain::types::Position>,
}

/// A quest in the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    /// Quest identifier
    pub id: String,
    /// Quest name
    pub name: String,
    /// Quest description
    pub description: String,
    /// Quest objectives
    pub objectives: Vec<QuestObjective>,
}

impl Quest {
    /// Creates a new quest
    pub fn new(id: String, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            objectives: Vec::new(),
        }
    }

    /// Adds an objective to the quest
    pub fn add_objective(&mut self, description: String) {
        self.add_objective_with_location(description, None, None);
    }

    /// Adds a new objective with optional map metadata for automap POI display
    pub fn add_objective_with_location(
        &mut self,
        description: String,
        map_id: Option<crate::domain::types::MapId>,
        position: Option<crate::domain::types::Position>,
    ) {
        self.objectives.push(QuestObjective {
            description,
            completed: false,
            map_id,
            position,
        });
    }

    /// Returns true if all objectives are completed
    pub fn is_completed(&self) -> bool {
        !self.objectives.is_empty() && self.objectives.iter().all(|obj| obj.completed)
    }
}

/// Quest log tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuestLog {
    /// Active quests
    pub active_quests: Vec<Quest>,
    /// Completed quest IDs
    pub completed_quests: Vec<String>,
    /// Quest IDs that have been unlocked and are available to start.
    ///
    /// Populated by the [`crate::domain::quest::QuestReward::UnlockQuest`]
    /// reward.  When a quest completion unlocks another quest, the target
    /// quest ID is inserted here.  The quest-start flow can optionally
    /// check this set to gate quest availability.
    #[serde(default)]
    pub available_quests: std::collections::HashSet<u16>,
}

impl QuestLog {
    /// Creates a new empty quest log
    pub fn new() -> Self {
        Self {
            active_quests: Vec::new(),
            completed_quests: Vec::new(),
            available_quests: std::collections::HashSet::new(),
        }
    }

    /// Adds a new quest
    pub fn add_quest(&mut self, quest: Quest) {
        self.active_quests.push(quest);
    }

    /// Marks a quest as completed
    pub fn complete_quest(&mut self, quest_id: &str) {
        if let Some(pos) = self.active_quests.iter().position(|q| q.id == quest_id) {
            let quest = self.active_quests.remove(pos);
            self.completed_quests.push(quest.id);
        }
    }

    /// Marks a quest as available/unlocked so it can be started.
    ///
    /// Called by the [`crate::domain::quest::QuestReward::UnlockQuest`]
    /// handler when a completed quest unlocks a follow-up quest.
    ///
    /// # Arguments
    ///
    /// * `quest_id` - Domain-level quest ID to unlock.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::QuestLog;
    ///
    /// let mut log = QuestLog::new();
    /// assert!(!log.is_quest_available(5));
    /// log.unlock_quest(5);
    /// assert!(log.is_quest_available(5));
    /// ```
    pub fn unlock_quest(&mut self, quest_id: u16) {
        self.available_quests.insert(quest_id);
    }

    /// Returns `true` if the given quest has been unlocked.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::QuestLog;
    ///
    /// let mut log = QuestLog::new();
    /// log.unlock_quest(10);
    /// assert!(log.is_quest_available(10));
    /// assert!(!log.is_quest_available(11));
    /// ```
    pub fn is_quest_available(&self, quest_id: u16) -> bool {
        self.available_quests.contains(&quest_id)
    }
}

// ===== Game State =====

/// The main game state container
///
/// This holds all the game data including world, party, and game mode.
///
/// # Examples
///
/// ```
/// use antares::application::GameState;
///
/// let game_state = GameState::new();
/// assert!(game_state.party.is_empty());
/// assert_eq!(game_state.time.day, 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// Active campaign (if playing campaign mode)
    #[serde(skip)]
    pub campaign: Option<Campaign>,
    /// The game world
    pub world: World,
    /// Character roster (all created characters)
    pub roster: Roster,
    /// Active party (up to 6 members)
    pub party: Party,
    /// Active party-wide spell effects
    pub active_spells: ActiveSpells,
    /// Game configuration (graphics, audio, controls, camera)
    /// Stored per-save to allow different settings per playthrough
    #[serde(default)]
    pub config: GameConfig,
    /// Current game mode
    pub mode: GameMode,
    /// Game time
    pub time: GameTime,
    /// Quest log
    pub quests: QuestLog,
    /// Tracks which characters have been encountered on maps (prevents re-recruiting)
    #[serde(default)]
    pub encountered_characters: std::collections::HashSet<String>,
    /// Per-NPC mutable runtime state (merchant stock, consumed services).
    ///
    /// Serialised into save data so stock levels persist across save/load cycles.
    /// Initialised via `NpcRuntimeStore::initialize_merchant` for each NPC that
    /// has a `stock_template` when a new game session begins.
    #[serde(default)]
    pub npc_runtime: NpcRuntimeStore,

    /// Campaign-level gameplay rules (level-up mode, XP curve, permadeath, etc.).
    ///
    /// Populated by [`GameState::new_game`] from the campaign's `config.ron`
    /// (`LevelingConfig` — leveling curve and mode) and `campaign.ron`
    /// (`max_level`, `permadeath`).  All other fields use their
    /// [`CampaignConfig::default`] values.
    ///
    /// Uses `#[serde(default)]` so that saves created via [`GameState::new`]
    /// (tests, offline contexts) or saves predating this field load cleanly
    /// with all defaults.
    #[serde(default)]
    pub campaign_config: CampaignConfig,

    /// Persisted game log entries so the log survives save/load cycles.
    ///
    /// Populated from the live [`crate::game::systems::ui::GameLog`] resource
    /// immediately before a save is written, and used to restore that resource
    /// after a save is loaded.  Uses `#[serde(default)]` so that saves created
    /// before this field was added load cleanly with an empty log.
    #[serde(default)]
    pub game_log_entries: Vec<SavedLogEntry>,
}

/// Errors returned by `GameState::initialize_roster`.
///
/// Wraps lower-level `CharacterDefinitionError` and `CharacterError` to
/// provide a single error type usable by the application layer.
#[derive(thiserror::Error, Debug)]
pub enum RosterInitializationError {
    #[error("Character definition error: {0}")]
    CharacterDefinition(#[from] crate::domain::character_definition::CharacterDefinitionError),

    #[error("Character operation error: {0}")]
    CharacterError(#[from] crate::domain::character::CharacterError),

    #[error("Too many starting party members: {count} characters have starts_in_party=true, but max party size is {max}")]
    TooManyStartingPartyMembers { count: usize, max: usize },
}

#[derive(thiserror::Error, Debug)]
pub enum MoveHandleError {
    #[error("Movement error: {0}")]
    Movement(#[from] crate::domain::world::MovementError),

    #[error("Event error: {0}")]
    Event(#[from] crate::domain::world::EventError),

    #[error("Combat initialization error: {0}")]
    CombatInit(#[from] crate::domain::combat::database::MonsterDatabaseError),
}

/// Errors that can occur during character recruitment from map encounters
#[derive(thiserror::Error, Debug)]
pub enum RecruitmentError {
    #[error("Character '{0}' has already been encountered and cannot be recruited again")]
    AlreadyEncountered(String),

    #[error("Character definition '{0}' not found in database")]
    CharacterNotFound(String),

    #[error("Character definition error: {0}")]
    CharacterDefinition(#[from] crate::domain::character_definition::CharacterDefinitionError),

    #[error("Character operation error: {0}")]
    CharacterError(#[from] crate::domain::character::CharacterError),

    #[error("Party manager error: {0}")]
    PartyManager(#[from] crate::domain::party_manager::PartyManagementError),
}

/// Result of attempting to recruit a character from a map encounter
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecruitResult {
    /// Character was successfully added to the party
    AddedToParty,

    /// Party was full, character was sent to the specified inn (innkeeper ID)
    SentToInn(crate::domain::types::InnkeeperId),

    /// Character recruitment was declined by the player
    Declined,
}

/// Maps a trap effect name string to a [`crate::domain::character::Condition`] bitflag.
///
/// Trap events store their status effect as an `Option<String>`.  This helper
/// translates well-known effect names into the corresponding condition flag so
/// that [`GameState::move_party_and_handle_events`] and the Bevy
/// [`crate::game::systems::events::handle_events`] system can apply effects
/// uniformly.
///
/// Unknown effect names return [`crate::domain::character::Condition::FINE`]
/// (no effect) and a warning is logged.
///
/// # Examples
///
/// ```
/// use antares::application::map_effect_to_condition;
/// use antares::domain::character::Condition;
///
/// assert_eq!(map_effect_to_condition("poison"), Condition::POISONED);
/// assert_eq!(map_effect_to_condition("paralysis"), Condition::PARALYZED);
/// assert_eq!(map_effect_to_condition("unknown"), Condition::FINE);
/// ```
pub fn map_effect_to_condition(effect: &str) -> u8 {
    use crate::domain::character::Condition;
    match effect.to_ascii_lowercase().as_str() {
        "poison" | "poisoned" => Condition::POISONED,
        "paralysis" | "paralyze" | "paralyzed" => Condition::PARALYZED,
        "sleep" | "asleep" => Condition::ASLEEP,
        "blind" | "blinded" => Condition::BLINDED,
        "silence" | "silenced" => Condition::SILENCED,
        "disease" | "diseased" => Condition::DISEASED,
        "unconscious" => Condition::UNCONSCIOUS,
        "death" | "dead" => Condition::DEAD,
        "stone" | "petrify" | "petrified" => Condition::STONE,
        other => {
            tracing::warn!("Unknown trap effect '{}' — no condition applied", other);
            Condition::FINE
        }
    }
}

impl GameState {
    /// Creates a new game state with default values (no campaign)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameState;
    ///
    /// let state = GameState::new();
    /// assert!(state.party.is_empty());
    /// assert_eq!(state.time.day, 1);
    /// ```
    pub fn new() -> Self {
        Self {
            campaign: None,
            world: World::new(),
            roster: Roster::new(),
            party: Party::new(),
            active_spells: ActiveSpells::new(),
            config: GameConfig::default(),
            mode: GameMode::Exploration,
            time: GameTime::new(1, 6, 0), // Day 1, 6:00 AM
            quests: QuestLog::new(),
            encountered_characters: std::collections::HashSet::new(),
            npc_runtime: NpcRuntimeStore::new(),
            campaign_config: CampaignConfig::default(),
            game_log_entries: Vec::new(),
        }
    }

    /// Creates a new game state with a campaign
    ///
    /// This constructor applies the campaign's starting configuration:
    /// - Sets starting gold and food from campaign config
    /// - Initializes party with campaign starting position
    /// - Loads campaign-specific data (items, spells, monsters, maps)
    ///
    /// # Arguments
    ///
    /// * `campaign` - The campaign to load
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::application::GameState;
    /// use antares::sdk::campaign_loader::{Campaign, CampaignLoader};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let loader = CampaignLoader::new("data");
    /// let campaign = loader.load_campaign("test_campaign")?;
    /// // `new_game` returns a Result<(GameState, ContentDatabase), CampaignError>
    /// let (state, _content_db) = GameState::new_game(campaign)?;
    ///
    /// assert!(state.campaign.is_some());
    /// assert_eq!(state.party.gold, state.campaign.as_ref().unwrap().config.starting_gold);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_game(campaign: Campaign) -> Result<(Self, ContentDatabase), CampaignError> {
        // Load campaign content (propagates CampaignError::DatabaseError or others)
        let content_db = campaign.load_content()?;

        // Basic validation: ensure core content groups are present
        if content_db.classes.all_classes().count() == 0 {
            return Err(CampaignError::DatabaseError(
                "Classes database is empty".to_string(),
            ));
        }
        if content_db.races.all_races().count() == 0 {
            return Err(CampaignError::DatabaseError(
                "Races database is empty".to_string(),
            ));
        }
        if content_db.characters.all_characters().count() == 0 {
            return Err(CampaignError::DatabaseError(
                "Characters database is empty".to_string(),
            ));
        }

        let starting_gold = campaign.config.starting_gold;

        let mut party = Party::new();
        party.gold = starting_gold;

        // Preserve campaign-specific game configuration for state
        let campaign_config = campaign.game_config.clone();

        // Extract leveling settings and per-campaign rules BEFORE `campaign`
        // is moved into `Self` below.  These values populate `campaign_config`
        // (the domain `CampaignConfig`) so that the progression system, combat
        // XP award, and training service all read the correct runtime values
        // instead of hard-coded defaults.
        let leveling = campaign.game_config.leveling.clone();
        let campaign_permadeath = campaign.config.permadeath;
        // max_level == 0 is invalid in practice (default is 20); treat it as
        // "no explicit cap" (None) so a malformed campaign.ron can't lock all
        // leveling.
        let max_party_level =
            (campaign.config.max_level > 0).then_some(u32::from(campaign.config.max_level));

        // Initialise the game clock from the campaign's configured starting time.
        // Campaign authors set this in config.ron via `starting_time: (day: N, hour: H, minute: M)`.
        // Falls back to Day 1, 08:00 when the field is absent (serde default).
        let starting_time = campaign.config.starting_time;

        let mut state = Self {
            campaign: Some(campaign),
            world: World::new(),
            roster: Roster::new(),
            party,
            active_spells: ActiveSpells::new(),
            config: campaign_config,
            mode: GameMode::Exploration,
            time: starting_time,
            quests: QuestLog::new(),
            encountered_characters: std::collections::HashSet::new(),
            npc_runtime: NpcRuntimeStore::new(),
            // Populate from the campaign's LevelingConfig (config.ron) and
            // campaign metadata (campaign.ron).  All other fields default.
            campaign_config: CampaignConfig {
                level_up_mode: leveling.level_up_mode,
                base_xp: leveling.base_xp,
                xp_multiplier: leveling.xp_multiplier,
                training_fee_base: leveling.training_fee_base,
                training_fee_multiplier: leveling.training_fee_multiplier,
                permadeath: campaign_permadeath,
                max_party_level,
                ..CampaignConfig::default()
            },
            game_log_entries: Vec::new(),
        };

        // Initialize roster from content database (premade characters)
        state.initialize_roster(&content_db).map_err(|e| {
            CampaignError::DatabaseError(format!("Roster initialization failed: {}", e))
        })?;

        Ok((state, content_db))
    }

    /// Loads campaign content for the currently loaded campaign
    ///
    /// # Errors
    ///
    /// Returns `CampaignError` if no campaign is loaded or the campaign fails to load its content.
    pub fn load_campaign_content(&self) -> Result<ContentDatabase, CampaignError> {
        if let Some(campaign) = &self.campaign {
            campaign.load_content()
        } else {
            Err(CampaignError::InvalidStructure(
                "No campaign loaded".to_string(),
            ))
        }
    }

    /// Initializes the roster using premade character definitions from the given content database.
    ///
    /// This will instantiate each premade character using race and class definitions from the content
    /// database (which applies race/class modifiers) and insert the resulting `Character` into the
    /// game's roster. Returns an error if instantiation or roster insertion fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameState;
    /// use antares::sdk::campaign_loader::CampaignLoader;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let loader = CampaignLoader::new("data");
    /// let campaign = loader.load_campaign("test_campaign")?;
    /// let (mut state, content_db) = GameState::new_game(campaign)?;
    /// // new_game calls initialize_roster internally; alternatively you may call:
    /// // state.initialize_roster(&content_db)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn initialize_roster(
        &mut self,
        content_db: &ContentDatabase,
    ) -> Result<(), RosterInitializationError> {
        use crate::domain::character::CharacterLocation;

        // Get starting innkeeper id from campaign config (default to tutorial innkeeper)
        //
        // Prefer the configured campaign `starting_innkeeper` (string ID). If the
        // campaign is missing or the configured value is empty, fall back to the
        // tutorial innkeeper ID so behavior remains consistent for legacy tests.
        let starting_innkeeper = if let Some(c) = &self.campaign {
            if c.config.starting_innkeeper.trim().is_empty() {
                "tutorial_innkeeper_town".to_string()
            } else {
                c.config.starting_innkeeper.clone()
            }
        } else {
            "tutorial_innkeeper_town".to_string()
        };

        // Track how many characters have starts_in_party set
        let mut starting_party_count = 0;

        for def in content_db.characters.premade_characters() {
            let character = def.instantiate(
                &content_db.races,
                &content_db.classes,
                &content_db.items,
                &content_db.spells,
            )?;

            // Determine initial location
            let location = if def.starts_in_party {
                starting_party_count += 1;

                // Enforce party size limit
                if starting_party_count > crate::domain::character::Party::MAX_MEMBERS {
                    return Err(RosterInitializationError::TooManyStartingPartyMembers {
                        count: starting_party_count,
                        max: crate::domain::character::Party::MAX_MEMBERS,
                    });
                }

                // Add to active party
                self.party.add_member(character.clone())?;
                CharacterLocation::InParty
            } else {
                // Non-party premades go to starting innkeeper id
                CharacterLocation::AtInn(starting_innkeeper.clone())
            };

            // Add to roster with location tracking
            self.roster.add_character(character, location)?;
        }

        Ok(())
    }

    // ===== Party Management Operations =====

    /// Recruits a character from the roster to the active party
    ///
    /// Moves a character from inn/map storage to the active adventuring party.
    /// The character must not already be in the party, and the party must have
    /// space available.
    ///
    /// # Arguments
    ///
    /// * `roster_index` - Index of the character in the roster to recruit
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if recruitment succeeds, otherwise returns a
    /// `PartyManagementError` explaining why the operation failed.
    ///
    /// # Errors
    ///
    /// - `PartyManagementError::PartyFull` if party is at maximum size (6 members)
    /// - `PartyManagementError::AlreadyInParty` if character is already in party
    /// - `PartyManagementError::InvalidRosterIndex` if roster_index is out of bounds
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::application::GameState;
    /// use antares::sdk::campaign_loader::CampaignLoader;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let loader = CampaignLoader::new("data");
    /// let campaign = loader.load_campaign("test_campaign")?;
    /// let (mut state, _db) = GameState::new_game(campaign)?;
    ///
    /// // Recruit character at roster index 0 (if not already in party)
    /// state.recruit_character(0)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn recruit_character(&mut self, roster_index: usize) -> Result<(), PartyManagementError> {
        PartyManager::recruit_to_party(&mut self.party, &mut self.roster, roster_index)
    }

    /// Dismisses a party member to an inn
    ///
    /// Removes a character from the active party and stores them at the specified inn.
    /// The party must have at least one member remaining after dismissal.
    ///
    /// # Arguments
    ///
    /// * `party_index` - Index of the character in the party to dismiss (0-5)
    /// * `inn_id` - Town/inn ID where the character will be stored
    ///
    /// # Returns
    ///
    /// Returns the dismissed `Character` if successful, otherwise returns a
    /// `PartyManagementError`.
    ///
    /// # Errors
    ///
    /// - `PartyManagementError::PartyEmpty` if dismissing would leave party empty
    /// - `PartyManagementError::InvalidPartyIndex` if party_index is out of bounds
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::application::GameState;
    /// use antares::sdk::campaign_loader::CampaignLoader;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let loader = CampaignLoader::new("data");
    /// let campaign = loader.load_campaign("test_campaign")?;
    /// let (mut state, _db) = GameState::new_game(campaign)?;
    ///
    /// // Assuming we have multiple party members, dismiss index 0 to inn 'tutorial_innkeeper_town'
    /// if state.party.size() > 1 {
    ///     let dismissed = state.dismiss_character(0, "tutorial_innkeeper_town".to_string())?;
    ///     println!("Dismissed {}", dismissed.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn dismiss_character(
        &mut self,
        party_index: usize,
        innkeeper_id: InnkeeperId,
    ) -> Result<crate::domain::character::Character, PartyManagementError> {
        PartyManager::dismiss_to_inn(&mut self.party, &mut self.roster, party_index, innkeeper_id)
    }

    /// Swaps a party member with a roster character
    ///
    /// Atomically exchanges a character in the active party with a character
    /// from the roster. This ensures the party never becomes empty during the
    /// operation.
    ///
    /// # Arguments
    ///
    /// * `party_index` - Index of the character in the party to swap out (0-5)
    /// * `roster_index` - Index of the character in the roster to swap in
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if swap succeeds, otherwise returns a `PartyManagementError`.
    ///
    /// # Errors
    ///
    /// - `PartyManagementError::InvalidPartyIndex` if party_index is out of bounds
    /// - `PartyManagementError::InvalidRosterIndex` if roster_index is out of bounds
    /// - `PartyManagementError::AlreadyInParty` if roster character is already in party
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::application::GameState;
    /// use antares::sdk::campaign_loader::CampaignLoader;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let loader = CampaignLoader::new("data");
    /// let campaign = loader.load_campaign("test_campaign")?;
    /// let (mut state, _db) = GameState::new_game(campaign)?;
    ///
    /// // Swap party member 0 with roster character 3
    /// state.swap_party_member(0, 3)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn swap_party_member(
        &mut self,
        party_index: usize,
        roster_index: usize,
    ) -> Result<(), PartyManagementError> {
        PartyManager::swap_party_member(
            &mut self.party,
            &mut self.roster,
            party_index,
            roster_index,
        )
    }

    /// Gets the current inn ID from the party's location.
    ///
    /// Checks the following in order:
    /// 1. The tile the party is standing on for an `EnterInn` event
    /// 2. Any `EnterInn` event on the current map (nearest inn)
    /// 3. The campaign's configured starting innkeeper as a fallback
    ///
    /// # Returns
    ///
    /// Returns `Some(InnkeeperId)` if an inn can be determined, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameState;
    ///
    /// let state = GameState::new();
    /// // No campaign loaded and no inn events → None
    /// let inn_id = state.current_inn_id();
    /// // Falls back to campaign starting innkeeper or None
    /// ```
    pub fn current_inn_id(&self) -> Option<InnkeeperId> {
        if let Some(map) = self.world.get_current_map() {
            // 1. Check the party's current tile for an EnterInn event
            if let Some(crate::domain::world::MapEvent::EnterInn { innkeeper_id, .. }) =
                map.get_event(self.world.party_position)
            {
                return Some(innkeeper_id.clone());
            }

            // 2. Check any EnterInn event on the current map
            for event in map.events.values() {
                if let crate::domain::world::MapEvent::EnterInn { innkeeper_id, .. } = event {
                    return Some(innkeeper_id.clone());
                }
            }
        }

        // 3. Fall back to the campaign's starting innkeeper
        self.campaign
            .as_ref()
            .map(|c| c.config.starting_innkeeper.clone())
    }

    // ===== Map Recruitment System =====

    /// Finds the nearest inn to the party's current position
    ///
    /// This simplified implementation returns the campaign's configured
    /// `starting_innkeeper` as a fallback. A full implementation would use
    /// pathfinding across maps to locate the closest valid inn.
    ///
    /// # Returns
    ///
    /// Returns `Some(InnkeeperId)` for the nearest/default inn, or `None` if no campaign loaded.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::application::GameState;
    /// use antares::sdk::campaign_loader::CampaignLoader;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let loader = CampaignLoader::new("data");
    /// let campaign = loader.load_campaign("test_campaign")?;
    /// let (state, _db) = GameState::new_game(campaign)?;
    ///
    /// let innkeeper_id = state.find_nearest_inn();
    /// assert!(innkeeper_id.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn find_nearest_inn(&self) -> Option<InnkeeperId> {
        // Return the configured starting innkeeper ID from the loaded campaign as a fallback.
        // A full implementation would find the nearest inn using pathfinding.
        self.campaign
            .as_ref()
            .map(|c| c.config.starting_innkeeper.clone())
    }

    /// Attempts to recruit a character from a map encounter
    ///
    /// When the player encounters a recruitable NPC on the map, this method handles
    /// the recruitment logic:
    /// - Checks if character was already encountered (prevents duplicates)
    /// - Loads character definition from content database
    /// - Adds to party if space available, otherwise sends to nearest inn
    /// - Marks character as encountered to prevent re-recruitment
    ///
    /// # Arguments
    ///
    /// * `character_id` - The character definition ID (e.g., "npc_old_gareth")
    /// * `content_db` - Content database containing character definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(RecruitResult)` indicating where the character was placed:
    /// - `AddedToParty` if party had room
    /// - `SentToInn(innkeeper_id)` if party was full
    /// - `Declined` should not be returned by this method (handled by UI)
    ///
    /// # Errors
    ///
    /// Returns `RecruitmentError` if:
    /// - Character was already encountered (`AlreadyEncountered`)
    /// - Character definition not found in database (`CharacterNotFound`)
    /// - Character instantiation fails (`CharacterDefinition`)
    /// - Roster/party operations fail (`PartyManager`, `CharacterError`)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::application::GameState;
    /// use antares::sdk::campaign_loader::CampaignLoader;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let loader = CampaignLoader::new("data");
    /// let campaign = loader.load_campaign("test_campaign")?;
    /// let (mut state, content_db) = GameState::new_game(campaign)?;
    ///
    /// // Player encounters "npc_old_gareth" on the map
    /// let result = state.recruit_from_map("npc_old_gareth", &content_db)?;
    /// match result {
    ///     antares::application::RecruitResult::AddedToParty => {
    ///         println!("Character joined the party!");
    ///     }
    ///     antares::application::RecruitResult::SentToInn(innkeeper_id) => {
    ///         println!("Party full - character sent to inn {}", innkeeper_id);
    ///     }
    ///     antares::application::RecruitResult::Declined => {
    ///         // Not used in this method
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn recruit_from_map(
        &mut self,
        character_id: &str,
        content_db: &ContentDatabase,
    ) -> Result<RecruitResult, RecruitmentError> {
        use crate::domain::character::CharacterLocation;

        // Check if character was already encountered
        if self.encountered_characters.contains(character_id) {
            return Err(RecruitmentError::AlreadyEncountered(
                character_id.to_string(),
            ));
        }

        // Get character definition from database
        let char_def = content_db
            .characters
            .get_character(character_id)
            .ok_or_else(|| RecruitmentError::CharacterNotFound(character_id.to_string()))?;

        // Instantiate character from definition
        let character = char_def.instantiate(
            &content_db.races,
            &content_db.classes,
            &content_db.items,
            &content_db.spells,
        )?;

        // Mark as encountered to prevent re-recruitment
        self.encountered_characters.insert(character_id.to_string());

        // Determine where to place the character
        if self.party.size() < crate::domain::character::Party::MAX_MEMBERS {
            // Party has room - add directly to party
            self.party.add_member(character.clone())?;
            self.roster
                .add_character(character, CharacterLocation::InParty)?;

            Ok(RecruitResult::AddedToParty)
        } else {
            // Party is full - send to nearest inn (innkeeper ID)
            let inn_id = self
                .find_nearest_inn()
                .unwrap_or("tutorial_innkeeper_town".to_string()); // Fallback to tutorial innkeeper ID if no campaign

            self.roster
                .add_character(character, CharacterLocation::AtInn(inn_id.clone()))?;

            Ok(RecruitResult::SentToInn(inn_id))
        }
    }

    // ===== Game Mode Transitions =====

    /// Enters combat mode (default handicap: Even)
    ///
    /// This creates a default `CombatState` and places the game into combat mode.
    pub fn enter_combat(&mut self) {
        let cs = crate::domain::combat::engine::CombatState::new(
            crate::domain::combat::types::Handicap::Even,
        );
        self.mode = GameMode::Combat(cs);
    }

    /// Enters combat mode with a provided `CombatState`.
    pub fn enter_combat_with_state(&mut self, cs: crate::domain::combat::engine::CombatState) {
        self.mode = GameMode::Combat(cs);
    }

    /* MoveHandleError moved to module scope (see above impl) */

    /// Move the party in the given direction, trigger any tile event, and handle
    /// the result. Encounters will initialize a combat from the content database
    /// and transition the game into combat mode.
    pub fn move_party_and_handle_events(
        &mut self,
        direction: crate::domain::types::Direction,
        content: &ContentDatabase,
    ) -> Result<(), MoveHandleError> {
        // Perform the move (may return MovementError)
        let position = crate::domain::world::move_party(&mut self.world, direction)
            .map_err(MoveHandleError::Movement)?;

        // Each successful step costs time — advance before event resolution so
        // that the clock ticks even when an event fires (traps, encounters, etc.).
        self.advance_time_seconds(self.config.time.movement_step_seconds, None);

        // If there is no explicit map event at this position, first roll for a
        // random encounter (map-level encounter tables / terrain modifiers apply).
        // Tile events take precedence: if there is an event placed on the tile,
        // we will let trigger_event handle it instead.
        if self
            .world
            .get_current_map()
            .and_then(|m| m.get_event(position))
            .is_none()
        {
            let mut rng = rand::rng();
            if let Some(encounter_group) =
                crate::domain::world::random_encounter(&self.world, &mut rng)
            {
                // Build combat state and initialize from the monster group
                let mut cs = crate::domain::combat::engine::CombatState::new(
                    crate::domain::combat::types::Handicap::Even,
                );

                crate::domain::combat::engine::initialize_combat_from_group(
                    &mut cs,
                    content,
                    &encounter_group.monster_group,
                )
                .map_err(MoveHandleError::CombatInit)?;

                // Enter combat with prepared combat state
                // store the type
                let _ = encounter_group.combat_event_type;
                self.mode = GameMode::Combat(cs);

                // Combat occurred instead of triggering a tile event; return early.
                return Ok(());
            }
        }

        // No random encounter (or a tile event exists) - handle tile event as before
        let ev = crate::domain::world::trigger_event(&mut self.world, position, &self.time)
            .map_err(MoveHandleError::Event)?;

        match ev {
            crate::domain::world::EventResult::Encounter {
                monster_group,
                combat_event_type,
            } => {
                // Build combat state and initialize from the monster group
                let mut cs = crate::domain::combat::engine::CombatState::new(
                    crate::domain::combat::types::Handicap::Even,
                );

                crate::domain::combat::engine::initialize_combat_from_group(
                    &mut cs,
                    content,
                    &monster_group,
                )
                .map_err(MoveHandleError::CombatInit)?;

                // store the type
                let _ = combat_event_type;

                // Enter combat with prepared combat state
                self.mode = GameMode::Combat(cs);
            }

            crate::domain::world::EventResult::NpcDialogue { npc_id } => {
                // Start dialogue mode with NPC context so dialogue systems
                // know which NPC the party is speaking to.
                let mut dialogue_state = crate::application::dialogue::DialogueState::new();
                dialogue_state.speaker_npc_id = Some(npc_id);
                self.mode = GameMode::Dialogue(dialogue_state);
            }

            crate::domain::world::EventResult::Trap { damage, effect } => {
                // Levitate buff negates all trap effects — the party floats over
                // pit traps and ground-based hazards entirely.
                if self.active_spells.levitate > 0 {
                    tracing::info!("Trap avoided — party is levitating.");
                } else {
                    // Apply trap damage to all living party members.
                    for member in &mut self.party.members {
                        if member.is_alive() {
                            member.hp.modify(-(damage as i32));
                            if member.hp.current == 0 {
                                member
                                    .conditions
                                    .add(crate::domain::character::Condition::DEAD);
                            }
                        }
                    }

                    // Apply status effect if present.
                    if let Some(ref effect_name) = effect {
                        let flag = map_effect_to_condition(effect_name);
                        if flag != crate::domain::character::Condition::FINE {
                            for member in &mut self.party.members {
                                if member.is_alive() {
                                    member.conditions.add(flag);
                                }
                            }
                        }
                    }

                    // Check for party wipe.
                    if self.party.living_count() == 0 {
                        self.mode = GameMode::GameOver;
                    }
                }
            }

            crate::domain::world::EventResult::Treasure { loot } => {
                // Distribute loot items across party members with inventory
                // space.  Each item ID in the loot vec is an `ItemId` (u8).
                for item_byte in &loot {
                    let item_id = *item_byte as crate::domain::types::ItemId;
                    let mut placed = false;
                    for member in &mut self.party.members {
                        if member.inventory.has_space()
                            && member.inventory.add_item(item_id, 1).is_ok()
                        {
                            placed = true;
                            break;
                        }
                    }
                    if !placed {
                        tracing::warn!(
                            "Treasure item {} lost — no party member has inventory space",
                            item_id
                        );
                    }
                }
            }

            _ => {
                // Other events (teleport, sign, etc.) are handled elsewhere or are no-ops here
            }
        }

        Ok(())
    }

    /// Exits combat mode and returns to exploration
    pub fn exit_combat(&mut self) {
        self.mode = GameMode::Exploration;
    }

    /// Enters resting mode for the specified number of hours.
    ///
    /// Transitions the game to [`GameMode::Resting`] with a fresh
    /// [`RestState`].  The rest orchestration system (`process_rest`) drives
    /// the per-hour loop; callers should not call this while already in
    /// `Resting` mode.
    ///
    /// # Arguments
    ///
    /// * `hours` — number of in-game hours to rest (typically
    ///   [`crate::domain::resources::REST_DURATION_HOURS`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameState, GameMode};
    ///
    /// let mut state = GameState::new();
    /// state.enter_rest(12);
    /// assert!(matches!(state.mode, GameMode::Resting(_)));
    /// ```
    pub fn enter_rest(&mut self, hours: u32) {
        self.mode = GameMode::Resting(RestState::new(hours));
    }

    /// Transitions to the rest-duration selection menu.
    ///
    /// Called when the player presses the rest key in Exploration mode.
    /// The rest menu UI presents three choices (4 / 8 / 12 hours); selecting
    /// one calls [`enter_rest`](Self::enter_rest) with the chosen duration.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameState, GameMode};
    ///
    /// let mut state = GameState::new();
    /// state.enter_rest_menu();
    /// assert!(matches!(state.mode, GameMode::RestMenu));
    /// ```
    pub fn enter_rest_menu(&mut self) {
        self.mode = GameMode::RestMenu;
    }

    /// Enters inventory mode, storing the current mode for resume on close.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameState, GameMode};
    ///
    /// let mut state = GameState::new();
    /// state.enter_inventory();
    /// assert!(matches!(state.mode, GameMode::Inventory(_)));
    /// ```
    pub fn enter_inventory(&mut self) {
        let prev = self.mode.clone();
        let party_size = self.party.members.len();
        let mut inv_state = crate::application::inventory_state::InventoryState::new(prev);
        // Open a panel for every current party member so the grid is fully
        // populated on first open — the player should not have to Tab to see
        // their own characters.
        inv_state.open_panels = (0..party_size).collect();
        if inv_state.open_panels.is_empty() {
            inv_state.open_panels.push(0);
        }
        self.mode = GameMode::Inventory(inv_state);
    }

    /// Enters menu mode
    pub fn enter_menu(&mut self) {
        let prev = self.mode.clone();
        self.mode = GameMode::Menu(MenuState::new(prev));
    }

    /// Closes the current modal screen and restores the appropriate prior mode.
    ///
    /// Returns `true` when the current [`GameMode`] was a closeable modal and
    /// was closed successfully. Returns `false` when the current mode is not a
    /// modal-close target and callers should apply different behavior (such as
    /// opening the game menu from [`GameMode::Exploration`]).
    ///
    /// This centralizes Escape / menu-key modal-close behavior so UI systems
    /// and global input toggles do not have to duplicate the per-mode resume
    /// logic.
    ///
    /// # Modal close behavior
    ///
    /// - `Automap` → `Exploration`
    /// - `Inventory` → stored resume mode
    /// - `MerchantInventory` → stored resume mode
    /// - `ContainerInventory` → stored resume mode
    /// - `SpellBook` → stored resume mode
    /// - `SpellCasting` → stored resume mode
    /// - `Dialogue` → `Exploration`
    /// - `TempleService` → `Exploration`
    /// - `RestMenu` → `Exploration`
    /// - `GameLog` → `Exploration`
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameMode, GameState};
    ///
    /// let mut state = GameState::new();
    /// state.enter_inventory();
    ///
    /// assert!(state.close_modal());
    /// assert!(matches!(state.mode, GameMode::Exploration));
    /// ```
    pub fn close_modal(&mut self) -> bool {
        match self.mode.clone() {
            GameMode::Automap => {
                self.mode = GameMode::Exploration;
                true
            }
            GameMode::Inventory(inv_state) => {
                self.mode = inv_state.get_resume_mode();
                true
            }
            GameMode::MerchantInventory(merchant_state) => {
                self.mode = merchant_state.get_resume_mode();
                true
            }
            GameMode::ContainerInventory(container_state) => {
                self.mode = container_state.get_resume_mode();
                true
            }
            GameMode::SpellBook(spell_book_state) => {
                self.mode = spell_book_state.get_resume_mode();
                true
            }
            GameMode::SpellCasting(spell_casting_state) => {
                self.mode = spell_casting_state.get_resume_mode();
                true
            }
            GameMode::Dialogue(_) => {
                self.mode = GameMode::Exploration;
                true
            }
            GameMode::TempleService(_) => {
                self.mode = GameMode::Exploration;
                true
            }
            GameMode::Training(_) => {
                self.mode = GameMode::Exploration;
                true
            }
            GameMode::RestMenu => {
                self.mode = GameMode::Exploration;
                true
            }
            GameMode::GameLog => {
                self.mode = GameMode::Exploration;
                true
            }
            GameMode::CharacterSheet(cs_state) => {
                self.mode = cs_state.get_resume_mode();
                true
            }
            _ => false,
        }
    }

    /// Enters exploration-mode spell casting with a pre-selected caster.
    ///
    /// Stores the current mode so it can be restored when the player cancels
    /// or finishes casting.  The UI starts at the spell-selection step because
    /// the caster is already known.
    ///
    /// Use [`enter_spell_casting_with_caster_select`] when the player should
    /// choose the caster first.
    ///
    /// # Arguments
    ///
    /// * `caster_index` — index into `party.members` of the character who casts.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameState, GameMode};
    ///
    /// let mut state = GameState::new();
    /// state.enter_spell_casting(0);
    /// assert!(matches!(state.mode, GameMode::SpellCasting(_)));
    /// ```
    ///
    /// [`enter_spell_casting_with_caster_select`]: Self::enter_spell_casting_with_caster_select
    pub fn enter_spell_casting(&mut self, caster_index: usize) {
        let prev = self.mode.clone();
        self.mode = GameMode::SpellCasting(
            crate::application::spell_casting_state::SpellCastingState::new(prev, caster_index),
        );
    }

    /// Enters exploration-mode spell casting with the caster-selection step first.
    ///
    /// Use this when no specific caster is pre-selected and the player must
    /// choose which party member will cast before browsing spells.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameState, GameMode};
    /// use antares::application::spell_casting_state::SpellCastingStep;
    ///
    /// let mut state = GameState::new();
    /// state.enter_spell_casting_with_caster_select();
    /// if let GameMode::SpellCasting(sc) = &state.mode {
    ///     assert_eq!(sc.step, SpellCastingStep::SelectCaster);
    /// } else {
    ///     panic!("expected SpellCasting mode");
    /// }
    /// ```
    pub fn enter_spell_casting_with_caster_select(&mut self) {
        let prev = self.mode.clone();
        self.mode = GameMode::SpellCasting(
            crate::application::spell_casting_state::SpellCastingState::new_with_caster_select(
                prev,
            ),
        );
    }

    /// Exits exploration-mode spell casting and restores the previous mode.
    ///
    /// If the current mode is not `SpellCasting` this is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameState, GameMode};
    ///
    /// let mut state = GameState::new();
    /// state.enter_spell_casting(0);
    /// assert!(matches!(state.mode, GameMode::SpellCasting(_)));
    /// state.exit_spell_casting();
    /// assert!(matches!(state.mode, GameMode::Exploration));
    /// ```
    pub fn exit_spell_casting(&mut self) {
        if let GameMode::SpellCasting(ref sc) = self.mode.clone() {
            self.mode = sc.get_resume_mode();
        }
    }

    /// Opens the in-game Spell Book screen for the given party member.
    ///
    /// Stores the current mode so it can be restored when the player closes
    /// the Spell Book screen.
    ///
    /// # Arguments
    ///
    /// * `character_index` — index into `party.members` whose book to open.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameState, GameMode};
    ///
    /// let mut state = GameState::new();
    /// state.enter_spellbook(2);
    /// if let GameMode::SpellBook(sb) = &state.mode {
    ///     assert_eq!(sb.character_index, 2);
    /// } else {
    ///     panic!("expected SpellBook mode");
    /// }
    /// ```
    pub fn enter_spellbook(&mut self, character_index: usize) {
        let prev = self.mode.clone();
        self.mode = GameMode::SpellBook(crate::application::spell_book_state::SpellBookState::new(
            character_index,
            prev,
        ));
    }

    /// Opens the in-game Spell Book screen starting at party member 0.
    ///
    /// Convenience wrapper for `enter_spellbook(0)`.  The UI may advance to
    /// the first caster automatically when the first party member is not a
    /// spell-caster.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameState, GameMode};
    ///
    /// let mut state = GameState::new();
    /// state.enter_spellbook_with_caster_select();
    /// if let GameMode::SpellBook(sb) = &state.mode {
    ///     assert_eq!(sb.character_index, 0);
    /// } else {
    ///     panic!("expected SpellBook mode");
    /// }
    /// ```
    pub fn enter_spellbook_with_caster_select(&mut self) {
        self.enter_spellbook(0);
    }

    /// Closes the Spell Book screen and restores the mode that was active
    /// before it was opened.
    ///
    /// If the current mode is not `SpellBook`, this is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameState, GameMode};
    ///
    /// let mut state = GameState::new();
    /// state.enter_spellbook(0);
    /// assert!(matches!(state.mode, GameMode::SpellBook(_)));
    /// state.exit_spellbook();
    /// assert!(matches!(state.mode, GameMode::Exploration));
    /// ```
    pub fn exit_spellbook(&mut self) {
        if let GameMode::SpellBook(ref sb) = self.mode.clone() {
            self.mode = sb.get_resume_mode();
        }
    }

    /// Enter the character sheet screen.
    ///
    /// Stores the current mode as `previous_mode` so it can be restored on close.
    /// If already in `CharacterSheet` mode, this is a no-op (idempotent).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameMode, GameState};
    ///
    /// let mut state = GameState::new();
    /// state.enter_character_sheet();
    /// assert!(matches!(state.mode, GameMode::CharacterSheet(_)));
    /// ```
    pub fn enter_character_sheet(&mut self) {
        if matches!(self.mode, GameMode::CharacterSheet(_)) {
            return;
        }
        let prev = self.mode.clone();
        self.mode = GameMode::CharacterSheet(
            crate::application::character_sheet_state::CharacterSheetState::new(prev),
        );
    }

    /// Enters dialogue mode
    pub fn enter_dialogue(&mut self) {
        self.mode = GameMode::Dialogue(crate::application::dialogue::DialogueState::new());
    }

    /// Enters the merchant buy/sell split-screen inventory.
    ///
    /// Called when the player presses `I` while in `GameMode::Dialogue` with a
    /// merchant NPC.  The current mode (typically `Dialogue`) is stored so it
    /// can be restored when the player closes the merchant screen.
    ///
    /// # Arguments
    ///
    /// * `npc_id`   – ID of the merchant NPC being traded with.
    /// * `npc_name` – Display name shown in the right-panel header.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameState, GameMode};
    ///
    /// let mut state = GameState::new();
    /// state.enter_merchant_inventory("merchant_tom".to_string(), "Tom's Goods".to_string());
    /// assert!(matches!(state.mode, GameMode::MerchantInventory(_)));
    /// ```
    pub fn enter_merchant_inventory(&mut self, npc_id: String, npc_name: String) {
        let prev = self.mode.clone();
        let active_character_index = 0; // always start with the party leader
        self.mode = GameMode::MerchantInventory(
            crate::application::merchant_inventory_state::MerchantInventoryState::new(
                npc_id,
                npc_name,
                active_character_index,
                prev,
            ),
        );
    }

    /// Enters the container interaction split-screen inventory.
    ///
    /// Called when the player presses `E` while facing a container tile event
    /// (chest, crate, hole in the wall, etc.).  The current mode (typically
    /// `Exploration`) is stored so it can be restored when the player closes
    /// the container screen.
    ///
    /// # Arguments
    ///
    /// * `container_event_id` – The map event ID of the container.
    /// * `container_name`     – Display name shown in the right-panel header.
    /// * `items`              – Current item list inside the container.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameState, GameMode};
    ///
    /// let mut state = GameState::new();
    /// state.enter_container_inventory(
    ///     "chest_001".to_string(),
    ///     "Wooden Chest".to_string(),
    ///     vec![],
    ///     0,
    ///     0,
    /// );
    /// assert!(matches!(state.mode, GameMode::ContainerInventory(_)));
    /// ```
    pub fn enter_container_inventory(
        &mut self,
        container_event_id: String,
        container_name: String,
        items: Vec<crate::domain::character::InventorySlot>,
        gold: u32,
        gems: u32,
    ) {
        let prev = self.mode.clone();
        let active_character_index = 0;
        let mut container_state =
            crate::application::container_inventory_state::ContainerInventoryState::new(
                container_event_id,
                container_name,
                items,
                active_character_index,
                prev,
            );
        container_state.gold = gold;
        container_state.gems = gems;
        self.mode = GameMode::ContainerInventory(container_state);
    }

    /// Returns to exploration mode (or resumes previous mode when exiting menu)
    pub fn return_to_exploration(&mut self) {
        let replaced = std::mem::replace(&mut self.mode, GameMode::Exploration);
        if let GameMode::Menu(menu_state) = replaced {
            self.mode = menu_state.get_resume_mode();
        } else {
            self.mode = GameMode::Exploration;
        }
    }

    /// Advances game time by the specified number of minutes.
    ///
    /// After advancing, active spell durations are ticked and merchant NPC stock
    /// is restocked / magic slots are rotated if a new in-game day has begun.
    ///
    /// # Arguments
    ///
    /// Rests the party for the given number of hours, healing HP/SP and
    /// consistently advancing game time through [`GameState::advance_time`].
    ///
    /// Unlike calling [`crate::domain::resources::rest_party`] directly (which
    /// only restores HP/SP and consumes food), this method also ticks
    /// active-spell durations and triggers daily merchant restocking for the
    /// full rest duration via [`GameState::advance_time`].
    ///
    /// # Arguments
    ///
    /// * `hours`     - Number of hours to rest.
    /// * `templates` - Optional merchant-stock template database.  Pass `None`
    ///   in headless tests that do not load campaign data; restocking is silently
    ///   skipped in that case.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::domain::resources::ResourceError`] if the party cannot
    /// rest (no food, etc.).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameState;
    /// use antares::domain::character::{Character, Sex, Alignment};
    /// use antares::domain::resources::REST_DURATION_HOURS;
    /// use antares::domain::items::{
    ///     ItemDatabase, Item, ItemType, ConsumableData, ConsumableEffect,
    /// };
    ///
    /// // Build an item database with one food ration (id = 1, IsFood(1)).
    /// let mut item_db = ItemDatabase::new();
    /// let ration = Item {
    ///     id: 1,
    ///     name: "Food Ration".to_string(),
    ///     item_type: ItemType::Consumable(ConsumableData {
    ///         effect: ConsumableEffect::IsFood(1),
    ///         is_combat_usable: false,
    ///         duration_minutes: None,
    ///     }),
    ///     base_cost: 1,
    ///     sell_cost: 0,
    ///     alignment_restriction: None,
    ///     constant_bonus: None,
    ///     temporary_bonus: None,
    ///     spell_effect: None,
    ///     max_charges: 0,
    ///     is_cursed: false,
    ///     icon_path: None,
    ///     tags: vec![],
    ///     mesh_descriptor_override: None,
    /// };
    /// item_db.add_item(ration).unwrap();
    ///
    /// let mut state = GameState::new();
    /// let mut hero = Character::new(
    ///     "Hero".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Male,
    ///     Alignment::Good,
    /// );
    /// hero.hp.base = 20;
    /// hero.hp.current = 10;
    /// // Give the hero a food ration so rest_party does not return TooHungryToRest.
    /// hero.inventory.add_item(1, 0).unwrap();
    /// state.party.add_member(hero).unwrap();
    /// // Use a value ≤ 255 that is fully consumed by REST_DURATION_HOURS * 60 ticks.
    /// // REST_DURATION_HOURS * 60 = 720 > u8::MAX, so we use a smaller sentinel.
    /// state.active_spells.light = 60; // expires after 60 minutes (< 12 hours)
    ///
    /// state.rest_party(REST_DURATION_HOURS, &item_db, None).unwrap();
    ///
    /// // Active spell with only 60 ticks must expire during a 12-hour rest.
    /// assert_eq!(state.active_spells.light, 0);
    /// // Time advanced by REST_DURATION_HOURS hours (starting from hour 6, so 6 + 12 = 18).
    /// assert_eq!(state.time.hour, 18);
    /// ```
    pub fn rest_party(
        &mut self,
        hours: u32,
        item_db: &crate::domain::items::ItemDatabase,
        templates: Option<&crate::domain::world::npc_runtime::MerchantStockTemplateDatabase>,
    ) -> Result<(), crate::domain::resources::ResourceError> {
        // Perform HP/SP restoration and food consumption from inventories.
        // rest_party() no longer takes a game_time parameter — time advancement
        // is exclusively handled by advance_time() below so that active-spell
        // ticking and merchant restocking are never bypassed.
        crate::domain::resources::rest_party(&mut self.party, item_db, hours)?;

        // Advance the authoritative clock via the GameState path so that active
        // spells are ticked and merchant stock is restocked for the full duration.
        self.advance_time_minutes(hours * 60, templates);

        Ok(())
    }

    /// Returns the current [`TimeOfDay`] period for the game clock.
    ///
    /// This is a convenience wrapper around [`GameTime::time_of_day`] so that
    /// any system with access to [`GameState`] can query the period without
    /// having to reach into `state.time` directly.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameState;
    /// use antares::domain::types::TimeOfDay;
    ///
    /// let mut state = GameState::new();
    /// // Default start time is Day 1, 06:00 — Dawn
    /// assert_eq!(state.time_of_day(), TimeOfDay::Dawn);
    ///
    /// state.time.advance_hours(6); // advance to 12:00
    /// assert_eq!(state.time_of_day(), TimeOfDay::Afternoon);
    /// ```
    pub fn time_of_day(&self) -> TimeOfDay {
        self.time.time_of_day()
    }

    /// Advances the in-game clock by `minutes` and ticks all time-sensitive state.
    ///
    /// This is the single authoritative time-advancement path. Every minute
    /// advanced through this function also:
    ///
    /// 1. Decrements all [`ActiveSpells`] protection field counters by 1 via
    ///    [`ActiveSpells::tick`].  When a field reaches 0 the corresponding
    ///    protection (e.g. `fire_protection`) expires and the next call to
    ///    [`ActiveSpells::effective_resistance`] for that type will return 0.
    /// 2. Ticks per-character timed attribute boosts via
    ///    [`crate::domain::character::Character::tick_timed_stat_boosts_minute`]
    ///    for every party member.  Boosts whose `minutes_remaining` reaches 0
    ///    are automatically reversed — the boosted `current` attribute value is
    ///    restored to its pre-boost level.
    /// 3. Triggers NPC merchant restock (if `templates` is `Some`).
    ///
    /// Advances the in-game clock by `seconds` and ticks all time-sensitive state.
    ///
    /// This is the single authoritative time-advancement path. The clock is
    /// updated in seconds for sub-minute resolution, but spell/boost ticking
    /// only occurs at minute boundaries (Option A: per-minute ticking).
    ///
    /// For every full minute contained in `seconds`:
    ///
    /// 1. Decrements all [`ActiveSpells`] protection field counters by 1 via
    ///    [`ActiveSpells::tick`].
    /// 2. Ticks per-character timed attribute boosts via
    ///    [`crate::domain::character::Character::tick_timed_stat_boosts_minute`].
    /// 3. Triggers NPC merchant restock (if `templates` is `Some`).
    ///
    /// Sub-minute advances (e.g. 30 seconds) update the clock but do **not**
    /// trigger effect ticks, since spells and stat boosts are measured in
    /// minutes.
    ///
    /// # Arguments
    ///
    /// * `seconds`   - Number of in-game seconds to advance.
    /// * `templates` - Template database used to replenish merchant stock.
    ///   Pass `None` in contexts where the content is not available (e.g.
    ///   headless unit tests that do not load campaign data); restocking is
    ///   silently skipped in that case.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameState;
    /// use antares::domain::world::npc_runtime::MerchantStockTemplateDatabase;
    ///
    /// let mut state = GameState::new();
    /// let templates = MerchantStockTemplateDatabase::new();
    /// state.advance_time_seconds(3600, Some(&templates));
    /// assert_eq!(state.time.minute, 0);
    /// assert_eq!(state.time.second, 0);
    /// assert_eq!(state.time.hour, 7);
    /// ```
    ///
    /// Sub-minute advance (no spell ticking):
    ///
    /// ```
    /// use antares::application::GameState;
    ///
    /// let mut state = GameState::new();
    /// state.active_spells.light = 10;
    /// state.advance_time_seconds(30, None); // 30 seconds — no minute boundary crossed
    /// assert_eq!(state.time.second, 30);
    /// assert_eq!(state.active_spells.light, 10); // unchanged — sub-minute
    /// ```
    ///
    /// Timed attribute boost expiry:
    ///
    /// ```
    /// use antares::application::GameState;
    /// use antares::domain::character::{Character, Sex, Alignment};
    /// use antares::domain::items::types::AttributeType;
    ///
    /// let mut state = GameState::new();
    /// let mut hero = Character::new(
    ///     "Hero".to_string(), "human".to_string(), "knight".to_string(),
    ///     Sex::Male, Alignment::Good,
    /// );
    /// let base_might = hero.stats.might.current;
    /// hero.apply_timed_stat_boost(AttributeType::Might, 5, Some(10));
    /// state.party.add_member(hero).unwrap();
    ///
    /// // Boost expires after exactly 10 minute-ticks (600 seconds).
    /// state.advance_time_seconds(600, None);
    /// assert_eq!(state.party.members[0].stats.might.current, base_might);
    /// assert!(state.party.members[0].timed_stat_boosts.is_empty());
    /// ```
    pub fn advance_time_seconds(
        &mut self,
        seconds: u32,
        templates: Option<&crate::domain::world::npc_runtime::MerchantStockTemplateDatabase>,
    ) {
        self.time.advance_seconds(seconds);

        // Tick active spell durations and per-character timed stat boosts
        // at minute granularity (Option A). Sub-minute advances do not tick.
        let minutes_to_tick = seconds / 60;
        for _ in 0..minutes_to_tick {
            self.active_spells.tick();
            for member in &mut self.party.members {
                member.tick_timed_stat_boosts_minute();
            }
        }

        // Trigger daily restock and magic-slot rotation when templates are available.
        if let Some(tmpl) = templates {
            self.npc_runtime.tick_restock(&self.time, tmpl);
        }
    }

    /// Convenience wrapper that advances time by the given number of minutes.
    ///
    /// Delegates to [`advance_time_seconds`](Self::advance_time_seconds) with
    /// `minutes * 60`. Use this for callers that still think in minutes (e.g.
    /// rest, spell durations).
    ///
    /// # Arguments
    ///
    /// * `minutes`   - Number of in-game minutes to advance.
    /// * `templates` - Optional merchant-stock template database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameState;
    ///
    /// let mut state = GameState::new();
    /// state.advance_time_minutes(60, None);
    /// assert_eq!(state.time.hour, 7);
    /// assert_eq!(state.time.minute, 0);
    /// assert_eq!(state.time.second, 0);
    /// ```
    pub fn advance_time_minutes(
        &mut self,
        minutes: u32,
        templates: Option<&crate::domain::world::npc_runtime::MerchantStockTemplateDatabase>,
    ) {
        self.advance_time_seconds(minutes * 60, templates);
    }

    /// Ensures all merchant NPCs in the content database have runtime state initialised.
    ///
    /// This method is idempotent: if a runtime state already exists for an NPC it is
    /// left unchanged. It is used in two scenarios:
    ///
    /// 1. **New game**: after `new_game` creates the state, merchant stock is seeded
    ///    from templates.
    /// 2. **Legacy save load**: a save file created before `npc_runtime` was added will
    ///    deserialise to an empty `NpcRuntimeStore` (via `#[serde(default)]`). Calling
    ///    this method after loading such a save re-creates the merchant stock from
    ///    templates so the player can interact with merchants normally.
    ///
    /// # Arguments
    ///
    /// * `content` - The loaded content database that contains NPC definitions and
    ///   stock templates.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameState;
    /// use antares::sdk::database::ContentDatabase;
    ///
    /// let mut state = GameState::new();
    /// let content = ContentDatabase::new();
    /// // With an empty content database this is a no-op.
    /// state.ensure_npc_runtime_initialized(&content);
    /// assert!(state.npc_runtime.is_empty());
    /// ```
    pub fn ensure_npc_runtime_initialized(&mut self, content: &ContentDatabase) {
        for npc_id in content.npcs.all_npcs() {
            // Only initialise NPCs that do not yet have a runtime state.
            if self.npc_runtime.get(&npc_id).is_none() {
                if let Some(npc) = content.npcs.get_npc(&npc_id) {
                    self.npc_runtime
                        .initialize_merchant(npc, &content.npc_stock_templates);
                }
            }
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::character_definition::CharacterDefinition;

    #[test]
    fn test_game_state_creation() {
        let state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));
        assert!(state.party.is_empty());
        assert_eq!(state.time.day, 1);
    }

    #[test]
    fn test_state_transition_preserves_party() {
        let mut state = GameState::new();

        // Add a character to the party
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        state.party.add_member(hero).unwrap();

        // Transition to combat (default combat state created)
        state.enter_combat();
        assert!(matches!(state.mode, GameMode::Combat(_)));
        assert_eq!(state.party.size(), 1);

        // Exit combat
        state.exit_combat();
        assert!(matches!(state.mode, GameMode::Exploration));
        assert_eq!(state.party.size(), 1);
    }

    #[test]
    fn test_game_content_resource_creation() {
        // Ensure the GameContent resource can be created and wraps an empty DB
        let db = crate::sdk::database::ContentDatabase::new();
        let resource = crate::application::resources::GameContent::new(db);
        assert_eq!(resource.db().classes.all_classes().count(), 0);
    }

    #[test]
    fn test_load_campaign_content_success() {
        // Uses the tutorial campaign (existing in repo) to validate loading
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("data");
        let campaign = loader
            .load_campaign("test_campaign")
            .expect("Failed to load tutorial campaign");

        // GameState::new_game now returns (GameState, ContentDatabase)
        let (_state, content_db) =
            GameState::new_game(campaign).expect("Failed to initialize game with campaign");

        assert!(content_db.classes.all_classes().count() > 0);
        assert!(content_db.races.all_races().count() > 0);
        assert!(content_db.characters.all_characters().count() > 0);
    }

    #[test]
    fn test_load_campaign_content_missing_files_error() {
        // Attempting to load a non-existent campaign directory should error
        use tempfile::TempDir;
        let t = TempDir::new().unwrap();
        let p = t.path().join("nonexistent_campaign");
        let res = crate::sdk::database::ContentDatabase::load_campaign(&p);
        assert!(matches!(
            res,
            Err(crate::sdk::database::DatabaseError::CampaignNotFound(_))
        ));
    }

    #[test]
    fn test_new_game_returns_content_database() {
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("data");
        let campaign = loader
            .load_campaign("test_campaign")
            .expect("Failed to load tutorial campaign");

        let (state, content_db) = GameState::new_game(campaign).expect("new_game failed");

        assert!(state.campaign.is_some());
        assert!(content_db.classes.all_classes().count() > 0);
    }

    #[test]
    fn test_game_modes() {
        let mut state = GameState::new();

        state.enter_menu();
        assert!(matches!(state.mode, GameMode::Menu(_)));

        state.enter_dialogue();
        assert!(matches!(state.mode, GameMode::Dialogue(_)));

        state.return_to_exploration();
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_close_modal_returns_false_in_exploration() {
        let mut state = GameState::new();

        assert!(!state.close_modal());
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_close_modal_closes_inventory_to_resume_mode() {
        let mut state = GameState::new();
        state.enter_inventory();

        assert!(state.close_modal());
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_close_modal_closes_dialogue_to_exploration() {
        let mut state = GameState::new();
        state.enter_dialogue();

        assert!(state.close_modal());
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_close_modal_closes_merchant_inventory_to_resume_mode() {
        let mut state = GameState::new();
        state.mode = GameMode::MerchantInventory(
            crate::application::merchant_inventory_state::MerchantInventoryState::new(
                "merchant_tom".to_string(),
                "Tom the Merchant".to_string(),
                0,
                GameMode::Dialogue(crate::application::dialogue::DialogueState::new()),
            ),
        );

        assert!(state.close_modal());
        assert!(matches!(state.mode, GameMode::Dialogue(_)));
    }

    #[test]
    fn test_close_modal_closes_container_inventory_to_resume_mode() {
        let mut state = GameState::new();
        state.enter_container_inventory(
            "crate_01".to_string(),
            "Wooden Crate".to_string(),
            vec![],
            0,
            0,
        );

        assert!(state.close_modal());
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_close_modal_closes_spell_book_to_resume_mode() {
        let mut state = GameState::new();
        state.enter_spellbook_with_caster_select();

        assert!(state.close_modal());
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_close_modal_closes_spell_casting_to_resume_mode() {
        let mut state = GameState::new();
        state.enter_spell_casting_with_caster_select();

        assert!(state.close_modal());
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_close_modal_closes_automap_to_exploration() {
        let mut state = GameState::new();
        state.mode = GameMode::Automap;

        assert!(state.close_modal());
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_close_modal_closes_temple_service_to_exploration() {
        let mut state = GameState::new();
        state.mode = GameMode::TempleService(TempleServiceState::new("temple_priest".to_string()));

        assert!(state.close_modal());
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_close_modal_closes_rest_menu_to_exploration() {
        let mut state = GameState::new();
        state.mode = GameMode::RestMenu;

        assert!(state.close_modal());
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_close_modal_closes_game_log_to_exploration() {
        let mut state = GameState::new();
        state.mode = GameMode::GameLog;

        assert!(state.close_modal());
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_game_mode_inventory_variant_constructable() {
        use crate::application::inventory_state::InventoryState;
        let mode = GameMode::Inventory(InventoryState::default());
        assert!(
            matches!(mode, GameMode::Inventory(_)),
            "GameMode::Inventory variant must be constructable"
        );
    }

    #[test]
    fn test_enter_inventory_sets_mode() {
        let mut state = GameState::new();
        state.enter_inventory();
        assert!(
            matches!(state.mode, GameMode::Inventory(_)),
            "enter_inventory must transition mode to GameMode::Inventory"
        );
    }

    #[test]
    fn test_enter_inventory_stores_previous_mode() {
        let mut state = GameState::new();
        // Start from Exploration (default)
        assert!(matches!(state.mode, GameMode::Exploration));
        state.enter_inventory();
        if let GameMode::Inventory(inv) = &state.mode {
            assert!(
                matches!(inv.get_resume_mode(), GameMode::Exploration),
                "enter_inventory must store the previous mode for resume"
            );
        } else {
            panic!("mode must be Inventory after enter_inventory");
        }
    }

    #[test]
    fn test_active_spells_tick() {
        let mut spells = ActiveSpells::new();
        spells.light = 10;
        spells.bless = 5;

        spells.tick();
        assert_eq!(spells.light, 9);
        assert_eq!(spells.bless, 4);

        // Tick multiple times
        for _ in 0..5 {
            spells.tick();
        }
        assert_eq!(spells.light, 4);
        assert_eq!(spells.bless, 0);
    }

    #[test]
    fn test_active_spells_reset_clears_all_fields() {
        let mut spells = ActiveSpells::new();

        // Set every field to a non-zero value
        spells.fear_protection = 5;
        spells.cold_protection = 10;
        spells.fire_protection = 15;
        spells.poison_protection = 20;
        spells.acid_protection = 8;
        spells.electricity_protection = 12;
        spells.magic_protection = 7;
        spells.light = 30;
        spells.leather_skin = 6;
        spells.levitate = 9;
        spells.walk_on_water = 11;
        spells.guard_dog = 4;
        spells.psychic_protection = 3;
        spells.bless = 25;
        spells.invisibility = 14;
        spells.shield = 18;
        spells.power_shield = 2;
        spells.cursed = 1;

        spells.reset();

        // Every field must be zeroed after reset
        assert_eq!(spells.fear_protection, 0);
        assert_eq!(spells.cold_protection, 0);
        assert_eq!(spells.fire_protection, 0);
        assert_eq!(spells.poison_protection, 0);
        assert_eq!(spells.acid_protection, 0);
        assert_eq!(spells.electricity_protection, 0);
        assert_eq!(spells.magic_protection, 0);
        assert_eq!(spells.light, 0);
        assert_eq!(spells.leather_skin, 0);
        assert_eq!(spells.levitate, 0);
        assert_eq!(spells.walk_on_water, 0);
        assert_eq!(spells.guard_dog, 0);
        assert_eq!(spells.psychic_protection, 0);
        assert_eq!(spells.bless, 0);
        assert_eq!(spells.invisibility, 0);
        assert_eq!(spells.shield, 0);
        assert_eq!(spells.power_shield, 0);
        assert_eq!(spells.cursed, 0);
    }

    #[test]
    fn test_quest_completion() {
        let mut quest = Quest::new(
            "quest_1".to_string(),
            "The First Quest".to_string(),
            "Complete the tutorial".to_string(),
        );

        quest.add_objective("Find the sword".to_string());
        quest.add_objective("Defeat the goblin".to_string());

        assert!(!quest.is_completed());

        quest.objectives[0].completed = true;
        assert!(!quest.is_completed());

        quest.objectives[1].completed = true;
        assert!(quest.is_completed());
    }

    #[test]
    fn test_advance_time_ticks_spells() {
        let mut state = GameState::new();
        state.active_spells.light = 10;

        state.advance_time_minutes(5, None);
        assert_eq!(state.active_spells.light, 5);
        assert_eq!(state.time.minute, 5);
    }

    // ===== TimedStatBoost wiring tests =====

    #[test]
    fn test_advance_time_ticks_timed_stat_boosts() {
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::domain::items::types::AttributeType;

        let mut state = GameState::new();

        // Add a party member with a timed Might boost of 5 minutes
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let base_might = hero.stats.might.current;
        hero.apply_timed_stat_boost(AttributeType::Might, 8, Some(5));
        assert_eq!(hero.stats.might.current, base_might + 8);
        state.party.add_member(hero).unwrap();

        // Advance 5 minutes — boost expires exactly on the 5th tick
        state.advance_time_minutes(5, None);

        assert_eq!(
            state.party.members[0].stats.might.current, base_might,
            "Might must be restored after advancing exactly 5 minutes"
        );
        assert!(
            state.party.members[0].timed_stat_boosts.is_empty(),
            "expired boost must be removed from the list"
        );
    }

    #[test]
    fn test_advance_time_ticks_both_spells_and_boosts() {
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::domain::items::types::AttributeType;

        let mut state = GameState::new();
        state.active_spells.light = 10;

        let mut hero = Character::new(
            "Mage".to_string(),
            "human".to_string(),
            "sorcerer".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        let base_speed = hero.stats.speed.current;
        hero.apply_timed_stat_boost(AttributeType::Speed, 4, Some(10));
        state.party.add_member(hero).unwrap();

        // Advance 7 minutes — both counters should tick together
        state.advance_time_minutes(7, None);

        assert_eq!(
            state.active_spells.light, 3,
            "active_spells.light must have decremented by 7"
        );
        assert_eq!(
            state.party.members[0].stats.speed.current,
            base_speed + 4,
            "Speed boost must still be active after 7 of 10 minutes"
        );
        assert_eq!(
            state.party.members[0].timed_stat_boosts[0].minutes_remaining, 3,
            "boost must have 3 minutes remaining"
        );

        // Advance 3 more minutes — boost expires
        state.advance_time_minutes(3, None);

        assert_eq!(
            state.active_spells.light, 0,
            "active_spells.light must reach 0 after 10 total minutes"
        );
        assert_eq!(
            state.party.members[0].stats.speed.current, base_speed,
            "Speed must be restored after 10 total minutes"
        );
        assert!(state.party.members[0].timed_stat_boosts.is_empty());
    }

    #[test]
    fn test_advance_time_no_restock_without_templates() {
        use crate::domain::inventory::{MerchantStock, StockEntry};
        use crate::domain::world::npc_runtime::NpcRuntimeState;

        let mut state = GameState::new();

        // Insert a merchant with depleted stock.
        let stock = MerchantStock {
            entries: vec![StockEntry {
                item_id: 1,
                quantity: 0,
                override_price: None,
            }],
            restock_template: Some("some_template".to_string()),
        };
        let mut runtime = NpcRuntimeState::new("merchant_alice".to_string());
        runtime.stock = Some(stock);
        state.npc_runtime.insert(runtime);

        // Advance past a day boundary with None templates — must not panic and must
        // not alter stock.
        state.advance_time_minutes(1440, None); // 24 hours

        let runtime = state
            .npc_runtime
            .get(&"merchant_alice".to_string())
            .unwrap();
        assert_eq!(
            runtime
                .stock
                .as_ref()
                .unwrap()
                .get_entry(1)
                .unwrap()
                .quantity,
            0,
            "stock must be unchanged when templates is None"
        );
        assert_eq!(
            runtime.last_restock_day, 0,
            "last_restock_day must not be updated"
        );
    }

    #[test]
    fn test_advance_time_triggers_restock() {
        use crate::domain::inventory::{MerchantStock, StockEntry};
        use crate::domain::world::npc_runtime::{
            MerchantStockTemplate, MerchantStockTemplateDatabase, NpcRuntimeState,
            TemplateStockEntry,
        };

        let mut state = GameState::new();

        // Build a template database with one template.
        let mut templates = MerchantStockTemplateDatabase::new();
        templates.add(MerchantStockTemplate {
            id: "basic_shop".to_string(),
            entries: vec![TemplateStockEntry {
                item_id: 2,
                quantity: 4,
                override_price: None,
            }],
            magic_item_pool: vec![],
            magic_slot_count: 0,
            magic_refresh_days: 7,
            description: String::new(),
        });

        // Insert a merchant with depleted stock referencing that template.
        let stock = MerchantStock {
            entries: vec![StockEntry {
                item_id: 2,
                quantity: 0,
                override_price: None,
            }],
            restock_template: Some("basic_shop".to_string()),
        };
        let mut runtime = NpcRuntimeState::new("merchant_bob".to_string());
        runtime.stock = Some(stock);
        state.npc_runtime.insert(runtime);

        // GameState starts at day 1, hour 0, minute 0.
        // Advance 24 hours so a new day begins (day 2).
        state.advance_time_minutes(1440, Some(&templates));

        let runtime = state.npc_runtime.get(&"merchant_bob".to_string()).unwrap();
        assert_eq!(
            runtime
                .stock
                .as_ref()
                .unwrap()
                .get_entry(2)
                .unwrap()
                .quantity,
            4,
            "stock must be restocked after crossing a day boundary"
        );
        assert!(
            runtime.last_restock_day > 0,
            "last_restock_day must be updated"
        );
    }

    #[test]
    fn test_inn_management_state_string_id() {
        // Verify InnManagementState stores the innkeeper string ID correctly
        let state = InnManagementState::new("tutorial_innkeeper_town".to_string());
        assert_eq!(state.current_inn_id, "tutorial_innkeeper_town".to_string());
        assert_eq!(state.selected_party_slot, None);
        assert_eq!(state.selected_roster_slot, None);
    }

    #[test]
    fn test_dismiss_character_with_innkeeper_id() {
        // Dismiss via GameState should preserve innkeeper string ID on roster location
        use crate::domain::character::{Alignment, Character, CharacterLocation, Sex};

        let mut state = GameState::new();

        // Create two characters and place them at the tutorial inn
        let c1 = Character::new(
            "Warrior".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let c2 = Character::new(
            "Mage".to_string(),
            "human".to_string(),
            "sorcerer".to_string(),
            Sex::Female,
            Alignment::Good,
        );

        state
            .roster
            .add_character(
                c1.clone(),
                CharacterLocation::AtInn("tutorial_innkeeper_town".to_string()),
            )
            .unwrap();
        state
            .roster
            .add_character(
                c2.clone(),
                CharacterLocation::AtInn("tutorial_innkeeper_town".to_string()),
            )
            .unwrap();

        // Recruit both to the active party
        state.recruit_character(0).unwrap();
        state.recruit_character(1).unwrap();

        // Dismiss first member to a specific innkeeper string ID
        let dismissed = state
            .dismiss_character(0, "storybook_inn".to_string())
            .unwrap();
        assert_eq!(dismissed.name, "Warrior");

        // Ensure roster now contains an AtInn location with the expected innkeeper ID
        let found = state
            .roster
            .character_locations
            .iter()
            .any(|loc| matches!(loc, CharacterLocation::AtInn(id) if id == "storybook_inn"));
        assert!(
            found,
            "Expected roster to contain a CharacterLocation::AtInn(\"storybook_inn\")"
        );

        // Party size should have decreased by one
        assert_eq!(state.party.size(), 1);
    }

    #[test]
    fn test_recruit_from_map_sends_to_innkeeper() {
        // When party is full, recruiting from map should send character to nearest inn (string ID)
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("data");
        let campaign = loader
            .load_campaign("test_campaign")
            .expect("Failed to load tutorial campaign");

        let (mut state, content_db) = GameState::new_game(campaign).expect("new_game failed");

        // Fill party to maximum capacity
        while state.party.size() < crate::domain::character::Party::MAX_MEMBERS {
            let filler = crate::domain::character::Character::new(
                "Filler".to_string(),
                "human".to_string(),
                "knight".to_string(),
                crate::domain::character::Sex::Male,
                crate::domain::character::Alignment::Good,
            );
            state.party.add_member(filler).unwrap();
        }

        // Recruit a known recruitable character from the tutorial campaign
        let result = state
            .recruit_from_map("old_gareth", &content_db)
            .expect("recruit_from_map failed");

        match result {
            RecruitResult::SentToInn(inn_id) => {
                // Verify the roster contains a character stored at the returned innkeeper ID
                let found = state.roster.character_locations.iter().any(|loc| {
                    matches!(loc, crate::domain::character::CharacterLocation::AtInn(id) if id == &inn_id)
                });
                assert!(
                    found,
                    "Expected roster to have a character at innkeeper ID {}",
                    inn_id
                );
            }
            other => panic!("Expected SentToInn result, got {:?}", other),
        }
    }

    #[test]
    fn test_initialize_roster_loads_all_characters() {
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("data");
        let campaign = loader
            .load_campaign("test_campaign")
            .expect("Failed to load tutorial campaign");

        let (state, content_db) = GameState::new_game(campaign).expect("new_game failed");

        let expected = content_db.characters.premade_characters().count();
        assert_eq!(state.roster.characters.len(), expected);
    }

    #[test]
    fn test_initialize_roster_applies_class_modifiers() {
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("data");
        let campaign = loader
            .load_campaign("test_campaign")
            .expect("Failed to load tutorial campaign");

        let (state, _content_db) = GameState::new_game(campaign).expect("new_game failed");

        // Kira is a human knight in the tutorial data with endurance 14.
        // Her character definition has an explicit hp_base: Some(10) override,
        // which takes precedence over the calculated value (class hp_die + endurance modifier).
        // This tests that explicit overrides in character definitions are respected.
        let kira = state
            .roster
            .characters
            .iter()
            .find(|c| c.name == "Kira")
            .expect("Kira not found in roster");
        assert_eq!(kira.hp.base, 10); // Explicit override in characters.ron
    }

    #[test]
    fn test_initialize_roster_applies_race_modifiers() {
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("data");
        let campaign = loader
            .load_campaign("test_campaign")
            .expect("Failed to load tutorial campaign");

        let (state, _content_db) = GameState::new_game(campaign).expect("new_game failed");

        // Sirius is an elf sorcerer with base intellect 16 and elf +2 modifier
        let sirius = state
            .roster
            .characters
            .iter()
            .find(|c| c.name == "Sirius")
            .expect("Sirius not found in roster");
        assert_eq!(sirius.stats.intellect.base, 18);
    }

    #[test]
    fn test_initialize_roster_sets_initial_hp_sp() {
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("data");
        let campaign = loader
            .load_campaign("test_campaign")
            .expect("Failed to load tutorial campaign");

        let (state, _content_db) = GameState::new_game(campaign).expect("new_game failed");

        let sirius = state
            .roster
            .characters
            .iter()
            .find(|c| c.name == "Sirius")
            .expect("Sirius not found in roster");

        assert_eq!(sirius.sp.base, 8); // 18 intellect -> 8 SP for a pure caster
    }

    #[test]
    fn test_initialize_roster_invalid_class_id_error() {
        use crate::domain::character_definition::CharacterDefinition;

        let mut db = crate::sdk::database::ContentDatabase::new();
        let mut char_db = crate::domain::character_definition::CharacterDatabase::new();

        // Ensure race exists so class validation is exercised
        let human = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human".to_string(),
        );
        db.races.add_race(human).unwrap();

        let mut bad = CharacterDefinition::new(
            "bad_class".to_string(),
            "Bad Class".to_string(),
            "human".to_string(),
            "no_such_class".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        // Mark as premade so `initialize_roster` will attempt to instantiate it
        bad.is_premade = true;

        char_db.add_character(bad).unwrap();
        db.characters = char_db;

        let mut state = GameState::new();
        let res = state.initialize_roster(&db);

        assert!(matches!(
            res,
            Err(RosterInitializationError::CharacterDefinition(
                crate::domain::character_definition::CharacterDefinitionError::InvalidClassId { .. }
            ))
        ));
    }

    #[test]
    fn test_initialize_roster_invalid_race_id_error() {
        use crate::domain::character_definition::CharacterDefinition;

        let mut db = crate::sdk::database::ContentDatabase::new();
        let mut char_db = crate::domain::character_definition::CharacterDatabase::new();

        // Ensure class exists so race validation is exercised
        let knight = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight).unwrap();

        let mut bad = CharacterDefinition::new(
            "bad_race".to_string(),
            "Bad Race".to_string(),
            "no_such_race".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        // Mark as premade so `initialize_roster` will attempt to instantiate it
        bad.is_premade = true;

        char_db.add_character(bad).unwrap();
        db.characters = char_db;

        let mut state = GameState::new();
        let res = state.initialize_roster(&db);

        assert!(matches!(
            res,
            Err(RosterInitializationError::CharacterDefinition(
                crate::domain::character_definition::CharacterDefinitionError::InvalidRaceId { .. }
            ))
        ));
    }

    #[test]
    fn test_initialize_roster_populates_starting_party() {
        let mut db = crate::sdk::database::ContentDatabase::new();

        // Add required class and race
        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        // Create character definitions with starts_in_party
        let mut char_db = crate::domain::character_definition::CharacterDatabase::new();

        let mut kira = CharacterDefinition::new(
            "kira".to_string(),
            "Kira".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        kira.is_premade = true;
        kira.starts_in_party = true;

        let mut sage = CharacterDefinition::new(
            "sage".to_string(),
            "Sage".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        sage.is_premade = true;
        sage.starts_in_party = true;

        let mut other = CharacterDefinition::new(
            "other".to_string(),
            "Other".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Female,
            Alignment::Neutral,
        );
        other.is_premade = true;
        other.starts_in_party = false;

        char_db.add_character(kira).unwrap();
        char_db.add_character(sage).unwrap();
        char_db.add_character(other).unwrap();
        db.characters = char_db;

        // Create campaign with starting inn
        let campaign = crate::sdk::campaign_loader::Campaign {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Test".to_string(),
            engine_version: "0.1.0".to_string(),
            required_features: vec![],
            config: crate::sdk::campaign_loader::CampaignConfig {
                starting_map: 1,
                starting_position: crate::domain::types::Position::new(0, 0),
                starting_direction: crate::domain::types::Direction::North,
                starting_gold: 100,
                starting_food: 50,
                starting_innkeeper: "tutorial_innkeeper_town".to_string(),
                max_party_size: 6,
                max_roster_size: 20,
                difficulty: crate::sdk::campaign_loader::Difficulty::Normal,
                permadeath: false,
                allow_multiclassing: false,
                starting_level: 1,
                max_level: 20,
                level_up_mode: crate::domain::campaign::LevelUpMode::Auto,
                base_xp: 1000,
                xp_multiplier: 1.5,
                starting_time: crate::domain::types::GameTime::new(1, 8, 0),
            },
            data: crate::sdk::campaign_loader::CampaignData {
                items: "items.ron".to_string(),
                spells: "spells.ron".to_string(),
                monsters: "monsters.ron".to_string(),
                classes: "classes.ron".to_string(),
                races: "races.ron".to_string(),
                maps: "maps".to_string(),
                quests: "quests.ron".to_string(),
                dialogues: "dialogues.ron".to_string(),
                characters: "characters.ron".to_string(),
                creatures: "creatures.ron".to_string(),
                furniture: "data/furniture.ron".to_string(),
            },
            assets: crate::sdk::campaign_loader::CampaignAssets {
                tilesets: "tilesets".to_string(),
                audio: "audio".to_string(),
                music: "music".to_string(),
                sounds: "sounds".to_string(),
                images: "images".to_string(),
                fonts: "fonts".to_string(),
            },
            root_path: std::path::PathBuf::from("test"),
            game_config: crate::sdk::game_config::GameConfig::default(),
        };

        let mut state = GameState::new();
        state.campaign = Some(campaign);
        state.initialize_roster(&db).unwrap();

        // Verify party has 2 members (kira and sage)
        assert_eq!(state.party.size(), 2);
        let party_names: Vec<&str> = state
            .party
            .members
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert!(party_names.contains(&"Kira"));
        assert!(party_names.contains(&"Sage"));

        // Verify roster has 3 characters
        assert_eq!(state.roster.characters.len(), 3);
        assert_eq!(state.roster.character_locations.len(), 3);

        // Verify locations are correct by checking characters_in_party and characters_at_inn
        let in_party = state.roster.characters_in_party();
        assert_eq!(in_party.len(), 2);
        let party_names_roster: Vec<&str> = in_party.iter().map(|(_, c)| c.name.as_str()).collect();
        assert!(party_names_roster.contains(&"Kira"));
        assert!(party_names_roster.contains(&"Sage"));

        let at_inn = state.roster.characters_at_inn("tutorial_innkeeper_town");
        assert_eq!(at_inn.len(), 1);
        assert_eq!(at_inn[0].1.name, "Other");
    }

    #[test]
    fn test_initialize_roster_sets_party_locations() {
        use crate::domain::character::CharacterLocation;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let mut char_db = crate::domain::character_definition::CharacterDatabase::new();

        let mut hero = CharacterDefinition::new(
            "hero".to_string(),
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.is_premade = true;
        hero.starts_in_party = true;

        char_db.add_character(hero).unwrap();
        db.characters = char_db;

        let mut state = GameState::new();
        state.initialize_roster(&db).unwrap();

        // Verify location is InParty
        assert_eq!(state.roster.character_locations.len(), 1);
        assert_eq!(
            state.roster.character_locations[0],
            CharacterLocation::InParty
        );

        // Verify character is in party
        let in_party = state.roster.characters_in_party();
        assert_eq!(in_party.len(), 1);
        assert_eq!(in_party[0].1.name, "Hero");
    }

    #[test]
    fn test_initialize_roster_sets_inn_locations() {
        use crate::domain::character::CharacterLocation;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let mut char_db = crate::domain::character_definition::CharacterDatabase::new();

        let mut npc = CharacterDefinition::new(
            "npc".to_string(),
            "NPC".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Female,
            Alignment::Neutral,
        );
        npc.is_premade = true;
        npc.starts_in_party = false;

        char_db.add_character(npc).unwrap();
        db.characters = char_db;

        let campaign = crate::sdk::campaign_loader::Campaign {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Test".to_string(),
            engine_version: "0.1.0".to_string(),
            required_features: vec![],
            config: crate::sdk::campaign_loader::CampaignConfig {
                starting_map: 1,
                starting_position: crate::domain::types::Position::new(0, 0),
                starting_direction: crate::domain::types::Direction::North,
                starting_gold: 100,
                starting_food: 50,
                starting_innkeeper: "tutorial_innkeeper_town".to_string(),
                max_party_size: 6,
                max_roster_size: 20,
                difficulty: crate::sdk::campaign_loader::Difficulty::Normal,
                permadeath: false,
                allow_multiclassing: false,
                starting_level: 1,
                max_level: 20,
                level_up_mode: crate::domain::campaign::LevelUpMode::Auto,
                base_xp: 1000,
                xp_multiplier: 1.5,
                starting_time: crate::domain::types::GameTime::new(1, 8, 0),
            },
            data: crate::sdk::campaign_loader::CampaignData {
                items: "items.ron".to_string(),
                spells: "spells.ron".to_string(),
                monsters: "monsters.ron".to_string(),
                classes: "classes.ron".to_string(),
                races: "races.ron".to_string(),
                maps: "maps".to_string(),
                quests: "quests.ron".to_string(),
                dialogues: "dialogues.ron".to_string(),
                characters: "characters.ron".to_string(),
                creatures: "creatures.ron".to_string(),
                furniture: "data/furniture.ron".to_string(),
            },
            assets: crate::sdk::campaign_loader::CampaignAssets {
                tilesets: "tilesets".to_string(),
                audio: "audio".to_string(),
                music: "music".to_string(),
                sounds: "sounds".to_string(),
                images: "images".to_string(),
                fonts: "fonts".to_string(),
            },
            root_path: std::path::PathBuf::from("test"),
            game_config: crate::sdk::game_config::GameConfig::default(),
        };

        let mut state = GameState::new();
        state.campaign = Some(campaign);
        state.initialize_roster(&db).unwrap();

        // Verify location is AtInn("tutorial_innkeeper_town")
        assert_eq!(state.roster.character_locations.len(), 1);
        assert_eq!(
            state.roster.character_locations[0],
            CharacterLocation::AtInn("tutorial_innkeeper_town".to_string())
        );

        // Verify character is at tutorial innkeeper id
        let at_inn = state.roster.characters_at_inn("tutorial_innkeeper_town");
        assert_eq!(at_inn.len(), 1);
        assert_eq!(at_inn[0].1.name, "NPC");

        // Verify party is empty
        assert_eq!(state.party.size(), 0);
    }

    #[test]
    fn test_initialize_roster_party_overflow_error() {
        let mut db = crate::sdk::database::ContentDatabase::new();

        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let mut char_db = crate::domain::character_definition::CharacterDatabase::new();

        // Create 7 characters with starts_in_party = true (exceeds max of 6)
        for i in 0..7 {
            let mut char = CharacterDefinition::new(
                format!("char_{}", i),
                format!("Character {}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            char.is_premade = true;
            char.starts_in_party = true;
            char_db.add_character(char).unwrap();
        }
        db.characters = char_db;

        let mut state = GameState::new();
        let res = state.initialize_roster(&db);

        // Should error with TooManyStartingPartyMembers
        assert!(matches!(
            res,
            Err(RosterInitializationError::TooManyStartingPartyMembers { count: 7, max: 6 })
        ));
    }

    #[test]
    fn test_initialize_roster_respects_max_party_size() {
        use crate::domain::character::CharacterLocation;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let mut char_db = crate::domain::character_definition::CharacterDatabase::new();

        // Create exactly 6 characters with starts_in_party = true (at max)
        for i in 0..6 {
            let mut char = CharacterDefinition::new(
                format!("char_{}", i),
                format!("Character {}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            char.is_premade = true;
            char.starts_in_party = true;
            char_db.add_character(char).unwrap();
        }
        db.characters = char_db;

        let mut state = GameState::new();
        state.initialize_roster(&db).unwrap();

        // Verify party has exactly 6 members
        assert_eq!(state.party.size(), 6);

        // Verify all are marked as InParty
        for i in 0..6 {
            assert_eq!(
                state.roster.character_locations[i],
                CharacterLocation::InParty
            );
        }
    }

    // ===== Party Management Integration Tests =====

    #[test]
    fn test_game_state_recruit_character() {
        use crate::domain::character::CharacterLocation;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let mut char_db = crate::domain::character_definition::CharacterDatabase::new();

        // Create one character in party, one at inn
        let mut char1 = CharacterDefinition::new(
            "char1".to_string(),
            "Character 1".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        char1.is_premade = true;
        char1.starts_in_party = true;

        let mut char2 = CharacterDefinition::new(
            "char2".to_string(),
            "Character 2".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        char2.is_premade = true;
        char2.starts_in_party = false;

        char_db.add_character(char1).unwrap();
        char_db.add_character(char2).unwrap();
        db.characters = char_db;

        let mut state = GameState::new();
        state.initialize_roster(&db).unwrap();

        // Initially: 1 in party, 1 at inn
        assert_eq!(state.party.size(), 1);

        // Find the character at the inn (not already in party)
        let inn_char_index = state
            .roster
            .character_locations
            .iter()
            .enumerate()
            .find(|(_, loc)| matches!(loc, CharacterLocation::AtInn(_)))
            .map(|(idx, _)| idx)
            .expect("Should have at least one character at inn");

        // Recruit the character at the inn
        let result = state.recruit_character(inn_char_index);
        assert!(result.is_ok());
        assert_eq!(state.party.size(), 2);
        assert_eq!(
            state.roster.character_locations[inn_char_index],
            CharacterLocation::InParty
        );
    }

    #[test]
    fn test_game_state_recruit_when_party_full() {
        use crate::domain::party_manager::PartyManagementError;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let mut char_db = crate::domain::character_definition::CharacterDatabase::new();

        // Create 6 in party, 1 at inn
        for i in 0..7 {
            let mut char = CharacterDefinition::new(
                format!("char_{}", i),
                format!("Character {}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            char.is_premade = true;
            char.starts_in_party = i < 6;
            char_db.add_character(char).unwrap();
        }
        db.characters = char_db;

        let mut state = GameState::new();
        state.initialize_roster(&db).unwrap();

        // Try to recruit 7th character when party is full
        let result = state.recruit_character(6);
        assert!(matches!(result, Err(PartyManagementError::PartyFull(6))));
    }

    #[test]
    fn test_game_state_dismiss_character() {
        use crate::domain::character::CharacterLocation;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let mut char_db = crate::domain::character_definition::CharacterDatabase::new();

        // Create 2 characters in party
        // Note: CharacterDatabase uses HashMap, so iteration order is non-deterministic.
        // We must find characters by ID, not assume index order.
        for i in 0..2 {
            let mut char = CharacterDefinition::new(
                format!("char_{}", i),
                format!("Character {}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            char.is_premade = true;
            char.starts_in_party = true;
            char_db.add_character(char).unwrap();
        }
        db.characters = char_db;

        let mut state = GameState::new();
        state.initialize_roster(&db).unwrap();

        assert_eq!(state.party.size(), 2);

        // Find which character is at party index 0 (could be either due to HashMap)
        let char_at_index_0 = &state.party.members[0];
        let expected_name = char_at_index_0.name.clone();

        // Dismiss first character (index 0) to inn 'tutorial_innkeeper_town2'
        let result = state.dismiss_character(0, "tutorial_innkeeper_town2".to_string());
        assert!(result.is_ok());
        let dismissed = result.unwrap();
        assert_eq!(dismissed.name, expected_name); // Verify we got the right character
        assert_eq!(state.party.size(), 1);

        // Find the dismissed character's roster index to verify location
        let dismissed_roster_index = state
            .roster
            .characters
            .iter()
            .position(|c| c.name == expected_name)
            .expect("Dismissed character not found in roster");
        assert_eq!(
            state.roster.character_locations[dismissed_roster_index],
            CharacterLocation::AtInn("tutorial_innkeeper_town2".to_string())
        );
    }

    #[test]
    fn test_game_state_dismiss_last_member_fails() {
        use crate::domain::party_manager::PartyManagementError;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let mut char_db = crate::domain::character_definition::CharacterDatabase::new();

        let mut char1 = CharacterDefinition::new(
            "char1".to_string(),
            "Character 1".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        char1.is_premade = true;
        char1.starts_in_party = true;
        char_db.add_character(char1).unwrap();
        db.characters = char_db;

        let mut state = GameState::new();
        state.initialize_roster(&db).unwrap();

        // Try to dismiss the only party member
        let result = state.dismiss_character(0, "tutorial_innkeeper_town".to_string());
        assert!(matches!(result, Err(PartyManagementError::PartyEmpty)));
    }

    #[test]
    fn test_game_state_swap_party_member() {
        use crate::domain::character::CharacterLocation;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let mut char_db = crate::domain::character_definition::CharacterDatabase::new();

        // Create 1 in party, 1 at inn
        let mut char1 = CharacterDefinition::new(
            "warrior".to_string(),
            "Warrior".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        char1.is_premade = true;
        char1.starts_in_party = true;

        let mut char2 = CharacterDefinition::new(
            "mage".to_string(),
            "Mage".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        char2.is_premade = true;
        char2.starts_in_party = false;

        char_db.add_character(char1).unwrap();
        char_db.add_character(char2).unwrap();
        db.characters = char_db;

        let mut state = GameState::new();
        state.initialize_roster(&db).unwrap();

        // Find the character at the inn (to swap with)
        let inn_char_index = state
            .roster
            .character_locations
            .iter()
            .enumerate()
            .find(|(_, loc)| matches!(loc, CharacterLocation::AtInn(_)))
            .map(|(idx, _)| idx)
            .expect("Should have at least one character at inn");

        let inn_char_name = state.roster.characters[inn_char_index].name.clone();

        // Swap first party member with character at inn
        let result = state.swap_party_member(0, inn_char_index);
        assert!(result.is_ok());

        // Verify swap
        assert_eq!(state.party.size(), 1);
        assert_eq!(state.party.members[0].name, inn_char_name);

        assert_eq!(
            state.roster.character_locations[inn_char_index],
            CharacterLocation::InParty
        );
    }

    // ===== Persistence & Save Game Integration Tests =====

    #[test]
    fn test_full_save_load_cycle_with_recruitment() {
        use crate::application::save_game::SaveGameManager;
        use crate::domain::character::CharacterLocation;
        use tempfile::TempDir;

        // Setup save manager
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        // Create initial game state
        let mut state = GameState::new();

        // Add 4 characters: 2 in party, 2 at inn
        for i in 0..4 {
            let char_name = format!("TestChar{}", i);
            let character = Character::new(
                char_name.clone(),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );

            let location = if i < 2 {
                CharacterLocation::InParty
            } else {
                CharacterLocation::AtInn("tutorial_innkeeper_town".to_string())
            };

            state.roster.add_character(character, location).unwrap();
        }

        // Add party members
        state.party.members.push(state.roster.characters[0].clone());
        state.party.members.push(state.roster.characters[1].clone());

        // Mark one character as encountered
        state
            .encountered_characters
            .insert("npc_recruit1".to_string());

        // Save initial state
        manager.save("integration_test", &state).unwrap();

        // Load and verify
        let loaded_state = manager.load("integration_test").unwrap();

        // Verify roster size
        assert_eq!(loaded_state.roster.characters.len(), 4);
        assert_eq!(loaded_state.roster.character_locations.len(), 4);

        // Verify party members
        assert_eq!(loaded_state.party.members.len(), 2);
        assert_eq!(loaded_state.party.members[0].name, "TestChar0");
        assert_eq!(loaded_state.party.members[1].name, "TestChar1");

        // Verify locations
        assert_eq!(
            loaded_state.roster.character_locations[0],
            CharacterLocation::InParty
        );
        assert_eq!(
            loaded_state.roster.character_locations[1],
            CharacterLocation::InParty
        );
        assert_eq!(
            loaded_state.roster.character_locations[2],
            CharacterLocation::AtInn("tutorial_innkeeper_town".to_string())
        );
        assert_eq!(
            loaded_state.roster.character_locations[3],
            CharacterLocation::AtInn("tutorial_innkeeper_town".to_string())
        );

        // Verify encounter tracking
        assert!(loaded_state.encountered_characters.contains("npc_recruit1"));
    }

    #[test]
    fn test_party_management_persists_across_save() {
        use crate::application::save_game::SaveGameManager;
        use crate::domain::character::CharacterLocation;
        use tempfile::TempDir;

        // Setup
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut state = GameState::new();

        // Add 4 characters
        for i in 0..4 {
            let character = Character::new(
                format!("Char{}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            let location = if i < 2 {
                CharacterLocation::InParty
            } else {
                CharacterLocation::AtInn("tutorial_innkeeper_town".to_string())
            };
            state.roster.add_character(character, location).unwrap();
        }

        state.party.members.push(state.roster.characters[0].clone());
        state.party.members.push(state.roster.characters[1].clone());

        // Save initial state
        manager.save("swap_test", &state).unwrap();

        // Perform swap: char[1] to inn, char[2] to party
        state.roster.character_locations[1] =
            CharacterLocation::AtInn("tutorial_innkeeper_town".to_string());
        state.roster.character_locations[2] = CharacterLocation::InParty;
        state.party.members[1] = state.roster.characters[2].clone();

        // Save swapped state
        manager.save("swap_test", &state).unwrap();

        // Load and verify swapped state
        let loaded_state = manager.load("swap_test").unwrap();

        assert_eq!(loaded_state.party.members.len(), 2);
        assert_eq!(loaded_state.party.members[0].name, "Char0");
        assert_eq!(loaded_state.party.members[1].name, "Char2");

        assert_eq!(
            loaded_state.roster.character_locations[0],
            CharacterLocation::InParty
        );
        assert_eq!(
            loaded_state.roster.character_locations[1],
            CharacterLocation::AtInn("tutorial_innkeeper_town".to_string())
        );
        assert_eq!(
            loaded_state.roster.character_locations[2],
            CharacterLocation::InParty
        );
        assert_eq!(
            loaded_state.roster.character_locations[3],
            CharacterLocation::AtInn("tutorial_innkeeper_town".to_string())
        );
    }

    #[test]
    fn test_encounter_tracking_persists() {
        use crate::application::save_game::SaveGameManager;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut state = GameState::new();

        // Mark several characters as encountered
        state
            .encountered_characters
            .insert("npc_merchant".to_string());
        state
            .encountered_characters
            .insert("npc_warrior".to_string());
        state.encountered_characters.insert("npc_mage".to_string());

        // Save
        manager.save("encounter_persist_test", &state).unwrap();

        // Load
        let loaded_state = manager.load("encounter_persist_test").unwrap();

        // Verify all encounters persisted
        assert_eq!(loaded_state.encountered_characters.len(), 3);
        assert!(loaded_state.encountered_characters.contains("npc_merchant"));
        assert!(loaded_state.encountered_characters.contains("npc_warrior"));
        assert!(loaded_state.encountered_characters.contains("npc_mage"));

        // Verify preventing re-recruitment still works
        assert!(loaded_state.encountered_characters.contains("npc_merchant"));
    }

    #[test]
    fn test_save_load_with_recruited_map_character() {
        use crate::application::save_game::SaveGameManager;
        use crate::domain::character::CharacterLocation;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut state = GameState::new();

        // Initial party member
        let party_char = Character::new(
            "Initial".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        state
            .roster
            .add_character(party_char, CharacterLocation::InParty)
            .unwrap();
        state.party.members.push(state.roster.characters[0].clone());

        // Character recruited from map (now in party)
        let recruited = Character::new(
            "RecruitedNPC".to_string(),
            "elf".to_string(),
            "archer".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        state
            .roster
            .add_character(recruited, CharacterLocation::InParty)
            .unwrap();
        state.party.members.push(state.roster.characters[1].clone());

        // Mark as encountered
        state
            .encountered_characters
            .insert("npc_recruited_archer".to_string());

        // Save
        manager.save("recruited_test", &state).unwrap();

        // Load
        let loaded_state = manager.load("recruited_test").unwrap();

        // Verify recruited character is in party
        assert_eq!(loaded_state.party.members.len(), 2);
        assert_eq!(loaded_state.party.members[1].name, "RecruitedNPC");
        assert_eq!(
            loaded_state.roster.character_locations[1],
            CharacterLocation::InParty
        );

        // Verify encounter tracking prevents re-recruitment
        assert!(loaded_state
            .encountered_characters
            .contains("npc_recruited_archer"));
    }

    #[test]
    fn test_save_load_character_sent_to_inn() {
        use crate::application::save_game::SaveGameManager;
        use crate::domain::character::CharacterLocation;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut state = GameState::new();

        // Fill party (6 members)
        for i in 0..6 {
            let character = Character::new(
                format!("PartyMember{}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            state
                .roster
                .add_character(character, CharacterLocation::InParty)
                .unwrap();
            state.party.members.push(state.roster.characters[i].clone());
        }

        // Recruit character when party is full (goes to inn)
        let inn_char = Character::new(
            "InnRecruit".to_string(),
            "dwarf".to_string(),
            "cleric".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        state
            .roster
            .add_character(
                inn_char,
                CharacterLocation::AtInn("tutorial_innkeeper_town".to_string()),
            )
            .unwrap();
        state
            .encountered_characters
            .insert("npc_dwarf_cleric".to_string());

        // Save
        manager.save("inn_recruit_test", &state).unwrap();

        // Load
        let loaded_state = manager.load("inn_recruit_test").unwrap();

        // Verify character is at inn
        assert_eq!(loaded_state.roster.characters.len(), 7);
        assert_eq!(loaded_state.roster.characters[6].name, "InnRecruit");
        assert_eq!(
            loaded_state.roster.character_locations[6],
            CharacterLocation::AtInn("tutorial_innkeeper_town".to_string())
        );

        // Verify party is still full with original members
        assert_eq!(loaded_state.party.members.len(), 6);

        // Verify encounter tracking
        assert!(loaded_state
            .encountered_characters
            .contains("npc_dwarf_cleric"));
    }

    #[test]
    fn test_save_load_preserves_all_character_data() {
        use crate::application::save_game::SaveGameManager;
        use crate::domain::character::CharacterLocation;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut state = GameState::new();

        // Create character with specific stats
        let mut character = Character::new(
            "TestHero".to_string(),
            "elf".to_string(),
            "sorcerer".to_string(),
            Sex::Female,
            Alignment::Neutral,
        );
        character.level = 5;
        character.experience = 1000;
        character.hp.current = 50;
        character.sp.current = 30;

        state
            .roster
            .add_character(character, CharacterLocation::InParty)
            .unwrap();
        state.party.members.push(state.roster.characters[0].clone());

        // Save
        manager.save("detailed_test", &state).unwrap();

        // Load
        let loaded_state = manager.load("detailed_test").unwrap();

        // Verify all character details preserved
        let loaded_char = &loaded_state.roster.characters[0];
        assert_eq!(loaded_char.name, "TestHero");
        assert_eq!(loaded_char.race_id, "elf");
        assert_eq!(loaded_char.class_id, "sorcerer");
        assert_eq!(loaded_char.sex, Sex::Female);
        assert_eq!(loaded_char.alignment, Alignment::Neutral);
        assert_eq!(loaded_char.level, 5);
        assert_eq!(loaded_char.experience, 1000);
        assert_eq!(loaded_char.hp.current, 50);
        assert_eq!(loaded_char.sp.current, 30);
    }

    #[test]
    fn test_party_management_maintains_invariants() {
        use crate::domain::character::CharacterLocation;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let mut char_db = crate::domain::character_definition::CharacterDatabase::new();

        // Create 4 characters: 2 in party, 2 at inn
        for i in 0..4 {
            let mut char = CharacterDefinition::new(
                format!("char_{}", i),
                format!("Character {}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            char.is_premade = true;
            char.starts_in_party = i < 2;
            char_db.add_character(char).unwrap();
        }
        db.characters = char_db;

        let mut state = GameState::new();
        state.initialize_roster(&db).unwrap();

        // Find characters at inn (not in party)
        let inn_chars: Vec<usize> = state
            .roster
            .character_locations
            .iter()
            .enumerate()
            .filter_map(|(idx, loc)| {
                if matches!(loc, CharacterLocation::AtInn(_)) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();

        assert!(
            inn_chars.len() >= 2,
            "Need at least 2 characters at inn for test"
        );

        // Perform several operations
        state.recruit_character(inn_chars[0]).unwrap(); // Recruit first inn char
        state
            .dismiss_character(0, "tutorial_innkeeper_town".to_string())
            .unwrap(); // Dismiss first party member to inn 'tutorial_innkeeper_town'

        // Find a character at inn for swap
        let inn_char_for_swap = state
            .roster
            .character_locations
            .iter()
            .enumerate()
            .find(|(_, loc)| matches!(loc, CharacterLocation::AtInn(_)))
            .map(|(idx, _)| idx)
            .expect("Should have at least one character at inn");

        state.swap_party_member(0, inn_char_for_swap).unwrap(); // Swap party member with inn char

        // Verify invariants
        let party_count_in_roster = state
            .roster
            .character_locations
            .iter()
            .filter(|loc| **loc == CharacterLocation::InParty)
            .count();
        assert_eq!(party_count_in_roster, state.party.size());

        // Verify no duplicate InParty locations
        let in_party_indices: Vec<usize> = state
            .roster
            .character_locations
            .iter()
            .enumerate()
            .filter_map(|(idx, loc)| {
                if *loc == CharacterLocation::InParty {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(in_party_indices.len(), state.party.size());
    }

    // ===== NPC Runtime Initialization Tests =====

    /// Helper that builds a minimal `ContentDatabase` with one merchant NPC and a
    /// matching stock template.  Used by the `ensure_npc_runtime_initialized` tests.
    fn build_content_db_with_merchant() -> crate::sdk::database::ContentDatabase {
        use crate::domain::world::npc::NpcDefinition;
        use crate::domain::world::npc_runtime::{
            MerchantStockTemplate, MerchantStockTemplateDatabase, TemplateStockEntry,
        };

        let mut db = crate::sdk::database::ContentDatabase::new();

        // Build a stock template
        let mut templates = MerchantStockTemplateDatabase::new();
        templates.add(MerchantStockTemplate {
            id: "basic_goods".to_string(),
            entries: vec![TemplateStockEntry {
                item_id: 1,
                quantity: 5,
                override_price: None,
            }],
            magic_item_pool: vec![],
            magic_slot_count: 0,
            magic_refresh_days: 7,
            description: String::new(),
        });
        db.npc_stock_templates = templates;

        // Build an NPC database with one merchant referencing the template
        let mut merchant = NpcDefinition::merchant("merchant_alice", "Alice", "alice.png");
        merchant.stock_template = Some("basic_goods".to_string());
        db.npcs.add_npc(merchant).expect("add_npc should succeed");

        db
    }

    #[test]
    fn test_ensure_npc_runtime_initialized_populates_merchants() {
        // Arrange: empty npc_runtime in GameState
        let mut state = GameState::new();
        assert!(state.npc_runtime.is_empty());

        let content = build_content_db_with_merchant();

        // Act
        state.ensure_npc_runtime_initialized(&content);

        // Assert: merchant now has runtime state with stock
        let runtime = state
            .npc_runtime
            .get(&"merchant_alice".to_string())
            .expect("merchant_alice should have runtime state after initialization");

        assert!(
            runtime.stock.is_some(),
            "merchant_alice should have stock initialized from template"
        );
        assert_eq!(
            runtime
                .stock
                .as_ref()
                .unwrap()
                .get_entry(1)
                .expect("item 1 should be in stock")
                .quantity,
            5
        );
    }

    #[test]
    fn test_ensure_npc_runtime_initialized_is_idempotent() {
        // Arrange: initialize once
        let mut state = GameState::new();
        let content = build_content_db_with_merchant();

        state.ensure_npc_runtime_initialized(&content);

        // Simulate a buy: decrement the stock quantity
        {
            let runtime = state
                .npc_runtime
                .get_mut(&"merchant_alice".to_string())
                .unwrap();
            let entry = runtime
                .stock
                .as_mut()
                .unwrap()
                .get_entry_mut(1)
                .expect("item 1 should be in stock");
            entry.quantity = 2; // simulate two items bought
        }

        // Act: call again (second time should be a no-op for existing entries)
        state.ensure_npc_runtime_initialized(&content);

        // Assert: the decremented quantity is still 2 (not reset to 5)
        let runtime = state
            .npc_runtime
            .get(&"merchant_alice".to_string())
            .unwrap();
        assert_eq!(
            runtime
                .stock
                .as_ref()
                .unwrap()
                .get_entry(1)
                .unwrap()
                .quantity,
            2,
            "Second call to ensure_npc_runtime_initialized must not overwrite existing state"
        );
    }

    // ===== Time Advancement Hook Tests =====

    /// Helper: build a minimal world with a single passable 20×20 map, party at (10,10).
    fn build_world_with_map() -> crate::domain::world::World {
        use crate::domain::world::{Map, World};
        let mut world = World::new();
        let map = Map::new(1, "Test Map".to_string(), "A test map".to_string(), 20, 20);
        world.add_map(map);
        world.set_current_map(1);
        world.set_party_position(crate::domain::types::Position::new(10, 10));
        world
    }

    #[test]
    fn test_step_advances_time() {
        // A successful step must advance game time by exactly
        // config.time.movement_step_seconds (default 30 seconds).
        use crate::domain::types::Direction;
        use crate::sdk::database::ContentDatabase;

        let mut state = GameState::new();
        state.world = build_world_with_map();

        // Compute total seconds from the epoch so the baseline is correct
        // across month/year boundaries.
        let total_seconds = |s: &GameState| -> u64 {
            (s.time.total_days() as u64 - 1) * 86400
                + s.time.hour as u64 * 3600
                + s.time.minute as u64 * 60
                + s.time.second as u64
        };
        let before = total_seconds(&state);

        let content = ContentDatabase::new();
        state
            .move_party_and_handle_events(Direction::North, &content)
            .expect("move north on clear map must succeed");

        let after = total_seconds(&state);

        assert_eq!(
            after - before,
            30,
            "one step must advance time by exactly 30 seconds (default movement_step_seconds)"
        );
    }

    #[test]
    fn test_blocked_step_does_not_advance_time() {
        // Attempting to walk into a wall must NOT advance game time.
        use crate::domain::types::{Direction, Position};
        use crate::domain::world::{Map, WallType, World};
        use crate::sdk::database::ContentDatabase;

        let mut state = GameState::new();

        // Build a map with a wall directly north of the party's starting tile.
        let mut world = World::new();
        let mut map = Map::new(1, "Wall Map".to_string(), "Has a wall".to_string(), 20, 20);
        // Place a Normal wall at (10, 9) — one step North of (10, 10).
        if let Some(tile) = map.get_tile_mut(Position::new(10, 9)) {
            tile.wall_type = WallType::Normal;
            tile.blocked = true;
        }
        world.add_map(map);
        world.set_current_map(1);
        world.set_party_position(Position::new(10, 10));
        state.world = world;

        let time_before = state.time;
        let content = ContentDatabase::new();

        // Walking North should fail (blocked tile).
        let result = state.move_party_and_handle_events(Direction::North, &content);
        assert!(result.is_err(), "move into a wall must return an error");

        assert_eq!(
            state.time.second, time_before.second,
            "blocked step must not advance seconds"
        );
        assert_eq!(
            state.time.minute, time_before.minute,
            "blocked step must not advance minutes"
        );
        assert_eq!(
            state.time.hour, time_before.hour,
            "blocked step must not advance hours"
        );
        assert_eq!(
            state.time.day, time_before.day,
            "blocked step must not advance days"
        );
    }

    #[test]
    fn test_rest_advances_time_via_state() {
        // GameState::rest_party must advance time by exactly hours * 60 minutes.
        use crate::domain::items::types::{ConsumableData, ConsumableEffect};
        use crate::domain::items::{Item, ItemDatabase, ItemType};

        let mut state = GameState::new();

        // Build a food item DB and give the hero a ration.
        let mut item_db = ItemDatabase::new();
        item_db
            .add_item(Item {
                id: 1,
                name: "Food Ration".to_string(),
                item_type: ItemType::Consumable(ConsumableData {
                    effect: ConsumableEffect::IsFood(1),
                    is_combat_usable: false,
                    duration_minutes: None,
                }),
                base_cost: 5,
                sell_cost: 2,
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
            })
            .unwrap();

        // Add a party member with food rations so rest_party has someone to heal.
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Give 5 rations — more than enough for 1 member.
        for _ in 0..5 {
            hero.inventory.add_item(1, 0).unwrap();
        }
        state.party.add_member(hero).unwrap();

        let hours = 8u32;
        let initial_minute = state.time.minute;
        let initial_hour = state.time.hour;

        state
            .rest_party(hours, &item_db, None)
            .expect("rest_party must succeed with food");

        // Total minutes elapsed since the campaign start (Year 1, Month 1, Day 1, 00:00).
        // total_days() is cumulative across months/years; subtract 1 to get elapsed days.
        let elapsed_minutes = (state.time.total_days() - 1) * 24 * 60
            + state.time.hour as u32 * 60
            + state.time.minute as u32;
        let expected_elapsed = (initial_hour as u32) * 60 + (initial_minute as u32) + hours * 60;
        assert_eq!(
            elapsed_minutes,
            expected_elapsed,
            "rest_party must advance time by exactly {} hours ({} minutes)",
            hours,
            hours * 60
        );
        // Rest always advances on whole-minute boundaries, so seconds must be 0.
        assert_eq!(
            state.time.second, 0,
            "rest_party must leave seconds at 0 (minute-aligned advancement)"
        );
    }

    #[test]
    fn test_rest_ticks_active_spells() {
        // GameState::rest_party must tick active spells for the full rest duration
        // (hours * 60 ticks), ensuring that active-spell durations are not bypassed.
        use crate::domain::items::types::{ConsumableData, ConsumableEffect};
        use crate::domain::items::{Item, ItemDatabase, ItemType};
        use crate::domain::resources::REST_DURATION_HOURS;

        let mut state = GameState::new();

        // Build a food item DB and give the mage enough rations.
        let mut item_db = ItemDatabase::new();
        item_db
            .add_item(Item {
                id: 1,
                name: "Food Ration".to_string(),
                item_type: ItemType::Consumable(ConsumableData {
                    effect: ConsumableEffect::IsFood(1),
                    is_combat_usable: false,
                    duration_minutes: None,
                }),
                base_cost: 5,
                sell_cost: 2,
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
            })
            .unwrap();

        let mut hero = Character::new(
            "Mage".to_string(),
            "human".to_string(),
            "sorcerer".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        for _ in 0..5 {
            hero.inventory.add_item(1, 0).unwrap();
        }
        state.party.add_member(hero).unwrap();

        // Give a light spell that will expire during a full rest.
        // REST_DURATION_HOURS * 60 = 720, which overflows u8::MAX (255).
        // Use 240 minutes (4 hours), which is safely less than 12 * 60 and fits
        // in a u8.  After REST_DURATION_HOURS hours the spell must be fully ticked.
        let ticks: u8 = 240;
        state.active_spells.light = ticks;

        state
            .rest_party(REST_DURATION_HOURS, &item_db, None)
            .expect("rest_party must succeed with food");

        // After a full rest the light spell should have expired (0 ticks left).
        assert_eq!(
            state.active_spells.light, 0,
            "active spell must be fully ticked after a full rest of {} hours",
            REST_DURATION_HOURS
        );
    }

    // ===== GameState::time_of_day() Tests =====

    #[test]
    fn test_game_state_time_of_day_default_is_dawn() {
        // GameState::new() starts at Day 1, 06:00 — which is Dawn (05:00–07:59)
        let state = GameState::new();
        assert_eq!(
            state.time_of_day(),
            crate::domain::types::TimeOfDay::Dawn,
            "default start time (06:00) should be Dawn"
        );
    }

    #[test]
    fn test_game_state_time_of_day_delegates_to_game_time() {
        use crate::domain::types::{GameTime, TimeOfDay};

        let pairs: &[(u8, TimeOfDay)] = &[
            (5, TimeOfDay::Dawn),
            (8, TimeOfDay::Morning),
            (12, TimeOfDay::Afternoon),
            (16, TimeOfDay::Dusk),
            (19, TimeOfDay::Evening),
            (22, TimeOfDay::Night),
            (0, TimeOfDay::Night),
        ];

        for &(hour, ref expected) in pairs {
            let mut state = GameState::new();
            state.time = GameTime::new(1, hour, 0);
            assert_eq!(
                &state.time_of_day(),
                expected,
                "hour {} should map to {:?}",
                hour,
                expected
            );
        }
    }

    #[test]
    fn test_game_state_time_of_day_advances_correctly() {
        use crate::domain::types::TimeOfDay;

        // Start at 06:00 (Dawn), advance 6 hours → 12:00 (Afternoon)
        let mut state = GameState::new();
        assert_eq!(state.time_of_day(), TimeOfDay::Dawn);

        state.time.advance_hours(6);
        assert_eq!(
            state.time_of_day(),
            TimeOfDay::Afternoon,
            "06:00 + 6 hours should be Afternoon"
        );
    }

    #[test]
    fn test_game_state_time_of_day_night_via_advance_time() {
        use crate::domain::types::TimeOfDay;

        // Start at 06:00, advance 16 hours → 22:00 (Night)
        let mut state = GameState::new();
        state.advance_time_minutes(16 * 60, None);
        assert_eq!(
            state.time_of_day(),
            TimeOfDay::Night,
            "06:00 + 16 hours should be Night"
        );
    }

    // ===== ActiveSpells::effective_resistance tests =====

    #[test]
    fn test_effective_resistance_zero_when_no_protection() {
        use crate::domain::items::types::ResistanceType;

        // All active_spells fields are 0 by default — every type must return 0.
        let spells = ActiveSpells::new();
        let all_types = [
            ResistanceType::Fire,
            ResistanceType::Cold,
            ResistanceType::Electricity,
            ResistanceType::Energy,
            ResistanceType::Fear,
            ResistanceType::Physical,
            ResistanceType::Paralysis,
            ResistanceType::Sleep,
        ];
        for res_type in all_types {
            assert_eq!(
                spells.effective_resistance(res_type),
                0,
                "effective_resistance({res_type:?}) must be 0 when no protection is active"
            );
        }
    }

    #[test]
    fn test_effective_resistance_nonzero_when_active() {
        use crate::domain::items::types::ResistanceType;

        // Each of the eight ResistanceType variants must return ACTIVE_PROTECTION_BONUS
        // when its mapped ActiveSpells field is non-zero.
        //
        // Mapping (mirrors apply_consumable_effect_exploration):
        //   Fire        → fire_protection
        //   Cold        → cold_protection
        //   Electricity → electricity_protection
        //   Energy      → magic_protection
        //   Fear        → fear_protection
        //   Physical    → magic_protection  (no dedicated field)
        //   Paralysis   → psychic_protection
        //   Sleep       → psychic_protection
        type ActivateFn = fn(&mut ActiveSpells);
        let cases: &[(ResistanceType, ActivateFn)] = &[
            (ResistanceType::Fire, |s| s.fire_protection = 10),
            (ResistanceType::Cold, |s| s.cold_protection = 10),
            (ResistanceType::Electricity, |s| {
                s.electricity_protection = 10
            }),
            (ResistanceType::Energy, |s| s.magic_protection = 10),
            (ResistanceType::Fear, |s| s.fear_protection = 10),
            (ResistanceType::Physical, |s| s.magic_protection = 10),
            (ResistanceType::Paralysis, |s| s.psychic_protection = 10),
            (ResistanceType::Sleep, |s| s.psychic_protection = 10),
        ];

        for (res_type, activate) in cases {
            let mut spells = ActiveSpells::new();
            activate(&mut spells);
            assert_eq!(
                spells.effective_resistance(*res_type),
                ACTIVE_PROTECTION_BONUS,
                "effective_resistance({res_type:?}) must be {ACTIVE_PROTECTION_BONUS} when protection is active"
            );
        }
    }

    #[test]
    fn test_effective_resistance_zero_when_expired() {
        use crate::domain::items::types::ResistanceType;

        // Set fire_protection to 1, tick once — it becomes 0 — resistance bonus
        // must then be 0 again.
        let mut spells = ActiveSpells::new();
        spells.fire_protection = 1;

        assert_eq!(
            spells.effective_resistance(ResistanceType::Fire),
            ACTIVE_PROTECTION_BONUS,
            "fire resistance bonus must be non-zero while protection is active"
        );

        // One tick decrements fire_protection to 0.
        spells.tick();

        assert_eq!(
            spells.effective_resistance(ResistanceType::Fire),
            0,
            "fire resistance bonus must be 0 after protection expires (tick to 0)"
        );
    }

    // ===== End-to-end timed potion / active-spell expiry tests =====

    /// Simulates applying a 60-minute fire-resistance potion directly on
    /// `active_spells` (the same write `apply_consumable_effect_exploration`
    /// would perform) and verifies that `advance_time(60)` fully drains it.
    #[test]
    fn test_timed_resistance_potion_expires_after_advance_time() {
        use crate::application::GameState;
        use crate::domain::character::{Alignment, Character, Sex};

        let mut state = GameState::new();

        // Add a hero so the party is non-empty (mirrors real usage).
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        state.party.add_member(hero).unwrap();

        // Simulate the fire-resistance potion being applied (60-minute duration).
        state.active_spells.fire_protection = 60;
        assert!(
            state.active_spells.fire_protection > 0,
            "fire_protection must be non-zero immediately after potion use"
        );

        // Advance exactly 60 minutes — every tick drains one unit, so it must reach 0.
        state.advance_time_minutes(60, None);

        assert_eq!(
            state.active_spells.fire_protection, 0,
            "fire_protection must be 0 after advancing exactly 60 minutes"
        );
    }

    /// Verifies that a 30-minute Might boost applied via `apply_timed_stat_boost`
    /// is correctly expired (and the stat restored) after `advance_time_minutes(30)`.
    #[test]
    fn test_timed_attribute_potion_expires_after_advance_time() {
        use crate::application::GameState;
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::domain::items::types::AttributeType;

        let mut state = GameState::new();

        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let base_might = hero.stats.might.current;

        // Apply a 5-point Might boost that lasts 30 minutes.
        hero.apply_timed_stat_boost(AttributeType::Might, 5, Some(30));
        assert_eq!(
            hero.timed_stat_boosts.len(),
            1,
            "one timed boost must be registered immediately after apply"
        );
        assert_eq!(
            hero.stats.might.current,
            base_might + 5,
            "Might must be raised by 5 immediately after apply"
        );

        state.party.add_member(hero).unwrap();

        // Advance exactly 30 minutes — the boost must expire on the last tick.
        state.advance_time_minutes(30, None);

        assert!(
            state.party.members[0].timed_stat_boosts.is_empty(),
            "timed boost list must be empty after the full 30-minute duration elapses"
        );
        assert_eq!(
            state.party.members[0].stats.might.current, base_might,
            "Might must be restored to its base value after the boost expires"
        );
    }

    /// Verifies that both a timed stat boost and an active-spell protection
    /// expire when `rest_party` is called for a full rest (12 hours = 720 minutes),
    /// which is well beyond the 60-minute durations used here.
    #[test]
    fn test_timed_potion_expires_during_rest() {
        use crate::application::GameState;
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::domain::items::types::{AttributeType, ConsumableData, ConsumableEffect};
        use crate::domain::items::{Item, ItemDatabase, ItemType};
        use crate::domain::resources::REST_DURATION_HOURS;

        let mut state = GameState::new();

        // Build a minimal food-item database (required by rest_party).
        let mut item_db = ItemDatabase::new();
        item_db
            .add_item(Item {
                id: 1,
                name: "Food Ration".to_string(),
                item_type: ItemType::Consumable(ConsumableData {
                    effect: ConsumableEffect::IsFood(1),
                    is_combat_usable: false,
                    duration_minutes: None,
                }),
                base_cost: 5,
                sell_cost: 2,
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
            })
            .unwrap();

        // Build a hero with enough food rations to survive the full rest.
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        for _ in 0..5 {
            hero.inventory.add_item(1, 0).unwrap();
        }

        // Apply a 60-minute Speed boost — well within the 720-minute rest window.
        hero.apply_timed_stat_boost(AttributeType::Speed, 3, Some(60));
        let boosted_speed = hero.stats.speed.current;
        assert!(
            boosted_speed > 0,
            "boosted_speed sanity check: Speed must be positive"
        );

        state.party.add_member(hero).unwrap();

        // Simulate a cold-resistance potion (60-minute duration).
        state.active_spells.cold_protection = 60;

        // A full 12-hour rest ticks 720 minutes — both 60-minute effects must expire.
        state
            .rest_party(REST_DURATION_HOURS, &item_db, None)
            .expect("rest_party must succeed when party has sufficient food");

        assert!(
            state.party.members[0].timed_stat_boosts.is_empty(),
            "Speed boost must have expired during the 12-hour rest"
        );
        assert_eq!(
            state.active_spells.cold_protection, 0,
            "cold_protection must be 0 after the full 12-hour rest"
        );
    }

    /// Verifies that a permanent `BoostAttribute` (duration_minutes: None) applied
    /// via `apply_consumable_effect` raises the stat permanently — no timed boost
    /// is registered, and the stat is unchanged after `advance_time(999)`.
    #[test]
    fn test_permanent_attribute_potion_survives_advance_time() {
        use crate::application::GameState;
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::domain::items::consumable_usage::apply_consumable_effect;
        use crate::domain::items::types::{AttributeType, ConsumableData, ConsumableEffect};

        let mut state = GameState::new();

        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        // Apply a permanent +7 Might boost (duration_minutes: None → no timed entry).
        apply_consumable_effect(
            &mut hero,
            &ConsumableData {
                effect: ConsumableEffect::BoostAttribute(AttributeType::Might, 7),
                is_combat_usable: true,
                duration_minutes: None, // permanent — must NOT register a timed boost
            },
        );

        // Capture the boosted value before adding to the party.
        let boosted_might = hero.stats.might.current;

        // The permanent path must not register anything in the timed list.
        assert!(
            hero.timed_stat_boosts.is_empty(),
            "a permanent boost must not create a timed boost entry"
        );

        state.party.add_member(hero).unwrap();

        // Advance a large number of minutes — the permanent boost must survive.
        state.advance_time_minutes(999, None);

        assert_eq!(
            state.party.members[0].stats.might.current, boosted_might,
            "Might must remain permanently boosted after advance_time_minutes(999)"
        );
        assert!(
            state.party.members[0].timed_stat_boosts.is_empty(),
            "no timed boost entry must ever appear for a permanent boost"
        );
    }

    /// Verifies overwrite semantics: applying a second fire-resistance potion
    /// replaces the remaining duration (not adds to it), and the new 60-minute
    /// window ticks down independently from the original.
    #[test]
    fn test_second_resistance_potion_overwrites_duration() {
        use crate::application::GameState;

        let mut state = GameState::new();

        // First potion: 60 minutes of fire protection.
        state.active_spells.fire_protection = 60;

        // Advance 30 minutes — 30 minutes remain from the first potion.
        state.advance_time_minutes(30, None);
        assert_eq!(
            state.active_spells.fire_protection, 30,
            "30 minutes must remain after the first advance_time_minutes(30)"
        );

        // Second potion: overwrites with a fresh 60-minute duration (not 90).
        state.active_spells.fire_protection = 60;
        assert_eq!(
            state.active_spells.fire_protection, 60,
            "second potion must overwrite to exactly 60 (not 90)"
        );

        // Advance 30 more minutes — 30 minutes remain from the second potion.
        state.advance_time_minutes(30, None);
        assert_eq!(
            state.active_spells.fire_protection, 30,
            "30 minutes must remain from the second potion after another advance_time_minutes(30)"
        );
    }

    // ===== Phase 2: Time Advancement System Tests =====

    /// `advance_time_seconds(30)` from 12:00:00 yields 12:00:30 — sub-minute
    /// resolution works and does NOT tick spell/boost effects.
    #[test]
    fn test_advance_time_seconds_sub_minute_no_tick() {
        let mut state = GameState::new();
        state.active_spells.light = 10;
        state.time = crate::domain::types::GameTime::new_full(1, 1, 1, 12, 0);

        state.advance_time_seconds(30, None);

        assert_eq!(state.time.hour, 12);
        assert_eq!(state.time.minute, 0);
        assert_eq!(state.time.second, 30);
        // Sub-minute advance must NOT tick spells.
        assert_eq!(
            state.active_spells.light, 10,
            "sub-minute advance must not tick active spells"
        );
    }

    /// `advance_time_seconds(90)` from 12:00:30 yields 12:02:00 and ticks
    /// effects once (90 / 60 = 1 full minute).
    #[test]
    fn test_advance_time_seconds_minute_rollover() {
        let mut state = GameState::new();
        state.active_spells.light = 10;
        state.time = crate::domain::types::GameTime::new_full_with_seconds(1, 1, 1, 12, 0, 30);

        state.advance_time_seconds(90, None);

        assert_eq!(state.time.hour, 12);
        assert_eq!(state.time.minute, 2);
        assert_eq!(state.time.second, 0);
        // 90 seconds = 1 full minute tick.
        assert_eq!(
            state.active_spells.light, 9,
            "90 seconds should tick spells once (1 full minute)"
        );
    }

    /// `advance_time_minutes` delegates to `advance_time_seconds` correctly.
    #[test]
    fn test_advance_time_minutes_delegates_to_seconds() {
        let mut state_a = GameState::new();
        let mut state_b = GameState::new();
        state_a.active_spells.light = 20;
        state_b.active_spells.light = 20;

        state_a.advance_time_minutes(5, None);
        state_b.advance_time_seconds(300, None);

        assert_eq!(state_a.time.hour, state_b.time.hour);
        assert_eq!(state_a.time.minute, state_b.time.minute);
        assert_eq!(state_a.time.second, state_b.time.second);
        assert_eq!(state_a.active_spells.light, state_b.active_spells.light);
    }

    /// Movement advances time by exactly `movement_step_seconds` (default 30s)
    /// from the `TimeConfig` on the game state's config.
    #[test]
    fn test_movement_uses_config_time_step() {
        use crate::domain::types::Direction;
        use crate::sdk::database::ContentDatabase;

        let mut state = GameState::new();
        state.world = build_world_with_map();
        // Override movement_step_seconds to a custom value.
        state.config.time.movement_step_seconds = 45;

        let total_seconds = |s: &GameState| -> u64 {
            (s.time.total_days() as u64 - 1) * 86400
                + s.time.hour as u64 * 3600
                + s.time.minute as u64 * 60
                + s.time.second as u64
        };
        let before = total_seconds(&state);

        let content = ContentDatabase::new();
        state
            .move_party_and_handle_events(Direction::North, &content)
            .expect("move north on clear map must succeed");

        let after = total_seconds(&state);
        assert_eq!(
            after - before,
            45,
            "movement must advance time by exactly movement_step_seconds (custom 45)"
        );
    }

    /// `advance_time_seconds(3600)` advances the clock by exactly 1 hour and
    /// ticks effects 60 times.
    #[test]
    fn test_advance_time_seconds_full_hour() {
        let mut state = GameState::new();
        state.active_spells.light = 100;

        state.advance_time_seconds(3600, None);

        assert_eq!(state.time.hour, 7); // default start 06:00 + 1 hour
        assert_eq!(state.time.minute, 0);
        assert_eq!(state.time.second, 0);
        assert_eq!(
            state.active_spells.light, 40,
            "3600 seconds = 60 minute ticks, 100 - 60 = 40"
        );
    }

    /// Rest still works correctly via `advance_time_minutes` — hours * 60
    /// minutes is faithfully converted to seconds internally.
    #[test]
    fn test_rest_uses_advance_time_minutes_path() {
        let mut state = GameState::new();
        state.active_spells.light = 200;

        // Simulate what rest_party does internally: advance_time_minutes(hours * 60)
        let hours = 3u32;
        state.advance_time_minutes(hours * 60, None);

        assert_eq!(state.time.hour, 9); // 06:00 + 3 hours = 09:00
        assert_eq!(state.time.minute, 0);
        assert_eq!(state.time.second, 0);
        // 3 hours = 180 minute ticks. 200 - 180 = 20.
        assert_eq!(
            state.active_spells.light, 20,
            "3 hours of rest must tick spells 180 times"
        );
    }

    /// Backward-compatible deserialization: `GameTime` without `second` field
    /// defaults to 0, and `advance_time_seconds` works correctly on it.
    #[test]
    fn test_backward_compat_game_time_no_second_field() {
        use crate::domain::types::GameTime;

        // Deserialize a GameTime RON snippet that omits the `second` field.
        let ron_str = "(day: 5, hour: 8, minute: 30)";
        let time: GameTime = ron::from_str(ron_str).expect("RON deserialize must succeed");
        assert_eq!(time.day, 5);
        assert_eq!(time.hour, 8);
        assert_eq!(time.minute, 30);
        assert_eq!(time.second, 0, "missing second field must default to 0");

        // Verify advance_time_seconds works on a GameState with this time.
        let mut state = GameState::new();
        state.time = time;
        state.advance_time_seconds(45, None);
        assert_eq!(state.time.second, 45);
        assert_eq!(state.time.minute, 30);
    }

    #[test]
    fn test_map_effect_to_condition_known_effects() {
        use crate::application::map_effect_to_condition;
        use crate::domain::character::Condition;

        assert_eq!(map_effect_to_condition("poison"), Condition::POISONED);
        assert_eq!(map_effect_to_condition("poisoned"), Condition::POISONED);
        assert_eq!(map_effect_to_condition("paralysis"), Condition::PARALYZED);
        assert_eq!(map_effect_to_condition("paralyze"), Condition::PARALYZED);
        assert_eq!(map_effect_to_condition("sleep"), Condition::ASLEEP);
        assert_eq!(map_effect_to_condition("blind"), Condition::BLINDED);
        assert_eq!(map_effect_to_condition("silence"), Condition::SILENCED);
        assert_eq!(map_effect_to_condition("disease"), Condition::DISEASED);
        assert_eq!(
            map_effect_to_condition("unconscious"),
            Condition::UNCONSCIOUS
        );
        assert_eq!(map_effect_to_condition("death"), Condition::DEAD);
        assert_eq!(map_effect_to_condition("stone"), Condition::STONE);
        assert_eq!(map_effect_to_condition("petrify"), Condition::STONE);
    }

    #[test]
    fn test_map_effect_to_condition_unknown_returns_fine() {
        use crate::application::map_effect_to_condition;
        use crate::domain::character::Condition;

        assert_eq!(map_effect_to_condition("unknown"), Condition::FINE);
        assert_eq!(map_effect_to_condition("fireball"), Condition::FINE);
    }

    #[test]
    fn test_map_effect_to_condition_case_insensitive() {
        use crate::application::map_effect_to_condition;
        use crate::domain::character::Condition;

        assert_eq!(map_effect_to_condition("POISON"), Condition::POISONED);
        assert_eq!(map_effect_to_condition("Paralysis"), Condition::PARALYZED);
        assert_eq!(map_effect_to_condition("SLEEP"), Condition::ASLEEP);
    }

    #[test]
    fn test_quest_log_unlock_quest() {
        let mut log = QuestLog::new();
        assert!(!log.is_quest_available(5));
        log.unlock_quest(5);
        assert!(log.is_quest_available(5));
        assert!(!log.is_quest_available(6));
    }

    #[test]
    fn test_quest_log_unlock_quest_idempotent() {
        let mut log = QuestLog::new();
        log.unlock_quest(10);
        log.unlock_quest(10);
        assert!(log.is_quest_available(10));
        assert_eq!(log.available_quests.len(), 1);
    }

    #[test]
    fn test_quest_log_available_quests_serialization() {
        let mut log = QuestLog::new();
        log.unlock_quest(1);
        log.unlock_quest(2);

        let serialized =
            ron::ser::to_string_pretty(&log, ron::ser::PrettyConfig::default()).expect("serialize");
        let deserialized: QuestLog = ron::from_str(&serialized).expect("deserialize");
        assert!(deserialized.is_quest_available(1));
        assert!(deserialized.is_quest_available(2));
        assert!(!deserialized.is_quest_available(3));
    }

    #[test]
    fn test_quest_log_backward_compat_no_available_quests_field() {
        // Simulate a legacy save without available_quests field
        let ron_str = r#"(
            active_quests: [],
            completed_quests: [],
        )"#;
        let log: QuestLog = ron::from_str(ron_str).expect("deserialize legacy quest log");
        assert!(log.available_quests.is_empty());
    }

    #[test]
    fn test_trap_event_reduces_party_hp() {
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent, World};

        let mut state = GameState::new();
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp = crate::domain::character::AttributePair16::new(50);
        state.party.add_member(hero).unwrap();

        // Set up a world with a map containing a trap
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let trap_pos = Position::new(1, 0);
        map.add_event(
            trap_pos,
            MapEvent::Trap {
                name: "Pit Trap".to_string(),
                description: "A hidden pit".to_string(),
                damage: 10,
                effect: None,
            },
        );
        state.world = World::new();
        state.world.add_map(map);
        state.world.set_current_map(1);

        // Build a minimal content database
        let content = crate::sdk::database::ContentDatabase::new();

        let result =
            state.move_party_and_handle_events(crate::domain::types::Direction::East, &content);
        assert!(result.is_ok());

        // Hero should have taken 10 damage: 50 - 10 = 40
        assert_eq!(state.party.members[0].hp.current, 40);
    }

    #[test]
    fn test_trap_event_with_effect_applies_condition() {
        use crate::domain::character::{Alignment, Character, Condition, Sex};
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent, World};

        let mut state = GameState::new();
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp = crate::domain::character::AttributePair16::new(100);
        state.party.add_member(hero).unwrap();

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let trap_pos = Position::new(1, 0);
        map.add_event(
            trap_pos,
            MapEvent::Trap {
                name: "Poison Trap".to_string(),
                description: "".to_string(),
                damage: 5,
                effect: Some("poison".to_string()),
            },
        );
        state.world = World::new();
        state.world.add_map(map);
        state.world.set_current_map(1);

        let content = crate::sdk::database::ContentDatabase::new();
        let _ = state.move_party_and_handle_events(crate::domain::types::Direction::East, &content);

        assert_eq!(state.party.members[0].hp.current, 95);
        assert!(state.party.members[0].conditions.has(Condition::POISONED));
    }

    #[test]
    fn test_trap_kills_all_members_triggers_game_over() {
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent, World};

        let mut state = GameState::new();
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp = crate::domain::character::AttributePair16::new(5);
        state.party.add_member(hero).unwrap();

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        map.add_event(
            Position::new(1, 0),
            MapEvent::Trap {
                name: "Death Trap".to_string(),
                description: "".to_string(),
                damage: 100,
                effect: None,
            },
        );
        state.world = World::new();
        state.world.add_map(map);
        state.world.set_current_map(1);

        let content = crate::sdk::database::ContentDatabase::new();
        let _ = state.move_party_and_handle_events(crate::domain::types::Direction::East, &content);

        assert_eq!(state.party.living_count(), 0);
        assert!(matches!(state.mode, GameMode::GameOver));
    }

    #[test]
    fn test_treasure_event_distributes_items() {
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent, World};

        let mut state = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        state.party.add_member(hero).unwrap();

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        map.add_event(
            Position::new(1, 0),
            MapEvent::Treasure {
                name: "Chest".to_string(),
                description: "A shiny chest".to_string(),
                loot: vec![5, 10],
            },
        );
        state.world = World::new();
        state.world.add_map(map);
        state.world.set_current_map(1);

        let content = crate::sdk::database::ContentDatabase::new();
        let _ = state.move_party_and_handle_events(crate::domain::types::Direction::East, &content);

        // Items should be in first member's inventory
        let inv = &state.party.members[0].inventory;
        assert_eq!(inv.items.len(), 2);
        assert!(inv.items.iter().any(|slot| slot.item_id == 5));
        assert!(inv.items.iter().any(|slot| slot.item_id == 10));
    }

    #[test]
    fn test_treasure_event_consumed_after_collection() {
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent, World};

        let mut state = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        state.party.add_member(hero).unwrap();

        let treasure_pos = Position::new(1, 0);
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        map.add_event(
            treasure_pos,
            MapEvent::Treasure {
                name: "Chest".to_string(),
                description: "".to_string(),
                loot: vec![1],
            },
        );
        state.world = World::new();
        state.world.add_map(map);
        state.world.set_current_map(1);

        let content = crate::sdk::database::ContentDatabase::new();
        let _ = state.move_party_and_handle_events(crate::domain::types::Direction::East, &content);

        // Treasure event should have been consumed by trigger_event (domain layer)
        assert!(state
            .world
            .get_current_map()
            .unwrap()
            .get_event(treasure_pos)
            .is_none());
    }

    #[test]
    fn test_npc_dialogue_carries_npc_id() {
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent, World};

        let mut state = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        state.party.add_member(hero).unwrap();

        let npc_pos = Position::new(1, 0);
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        map.add_event(
            npc_pos,
            MapEvent::NpcDialogue {
                name: "Elder".to_string(),
                description: "".to_string(),
                npc_id: "village_elder".to_string(),
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
            },
        );
        state.world = World::new();
        state.world.add_map(map);
        state.world.set_current_map(1);

        let content = crate::sdk::database::ContentDatabase::new();
        let _ = state.move_party_and_handle_events(crate::domain::types::Direction::East, &content);

        // Should be in dialogue mode with npc_id set
        match &state.mode {
            GameMode::Dialogue(ds) => {
                assert_eq!(ds.speaker_npc_id, Some("village_elder".to_string()));
            }
            other => panic!("Expected Dialogue mode, got {:?}", other),
        }
    }

    #[test]
    fn test_trap_dead_members_take_no_damage() {
        use crate::domain::character::{Alignment, Character, Condition, Sex};
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent, World};

        let mut state = GameState::new();

        // Living hero
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp = crate::domain::character::AttributePair16::new(50);
        state.party.add_member(hero).unwrap();

        // Dead companion
        let mut dead_guy = Character::new(
            "DeadGuy".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        dead_guy.hp.current = 0;
        dead_guy.conditions.add(Condition::DEAD);
        state.party.add_member(dead_guy).unwrap();

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        map.add_event(
            Position::new(1, 0),
            MapEvent::Trap {
                name: "Trap".to_string(),
                description: "".to_string(),
                damage: 10,
                effect: None,
            },
        );
        state.world = World::new();
        state.world.add_map(map);
        state.world.set_current_map(1);

        let content = crate::sdk::database::ContentDatabase::new();
        let _ = state.move_party_and_handle_events(crate::domain::types::Direction::East, &content);

        // Living hero takes damage
        assert_eq!(state.party.members[0].hp.current, 40);
        // Dead member unchanged
        assert_eq!(state.party.members[1].hp.current, 0);
    }

    #[test]
    fn test_levitate_buff_skips_trap_damage() {
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent, World};

        let mut state = GameState::new();
        let mut hero = Character::new(
            "Floater".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        hero.hp = crate::domain::character::AttributePair16::new(50);
        state.party.add_member(hero).unwrap();

        // Activate the Levitate buff.
        state.active_spells.levitate = 20;

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let trap_pos = Position::new(1, 0);
        map.add_event(
            trap_pos,
            MapEvent::Trap {
                name: "Pit Trap".to_string(),
                description: "A nasty pit".to_string(),
                damage: 25,
                effect: None,
            },
        );
        state.world = World::new();
        state.world.add_map(map);
        state.world.set_current_map(1);

        let content = crate::sdk::database::ContentDatabase::new();
        let result =
            state.move_party_and_handle_events(crate::domain::types::Direction::East, &content);
        assert!(result.is_ok());

        // No damage: levitate buff must have negated the trap.
        assert_eq!(
            state.party.members[0].hp.current, 50,
            "party should take no trap damage while levitating"
        );
        // Mode must remain Exploration (no game over).
        assert!(
            matches!(state.mode, crate::application::GameMode::Exploration),
            "game mode must remain Exploration"
        );
    }

    #[test]
    fn test_levitate_buff_skips_trap_condition() {
        use crate::domain::character::{Alignment, Character, Condition, Sex};
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent, World};

        let mut state = GameState::new();
        let mut hero = Character::new(
            "Floater".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        hero.hp = crate::domain::character::AttributePair16::new(50);
        state.party.add_member(hero).unwrap();

        state.active_spells.levitate = 10;

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        map.add_event(
            Position::new(1, 0),
            MapEvent::Trap {
                name: "Poison Pit".to_string(),
                description: "".to_string(),
                damage: 15,
                effect: Some("poison".to_string()),
            },
        );
        state.world = World::new();
        state.world.add_map(map);
        state.world.set_current_map(1);

        let content = crate::sdk::database::ContentDatabase::new();
        let _ = state.move_party_and_handle_events(crate::domain::types::Direction::East, &content);

        // HP unchanged and poison condition not applied.
        assert_eq!(state.party.members[0].hp.current, 50);
        assert!(
            !state.party.members[0].conditions.has(Condition::POISONED),
            "poisoned condition must not be applied when levitating"
        );
    }

    #[test]
    fn test_trap_damage_applies_without_levitate() {
        // Regression: trap must still deal damage when levitate is NOT active.
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent, World};

        let mut state = GameState::new();
        let mut hero = Character::new(
            "Grounded".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp = crate::domain::character::AttributePair16::new(50);
        state.party.add_member(hero).unwrap();
        state.active_spells.levitate = 0; // explicitly no buff

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        map.add_event(
            Position::new(1, 0),
            MapEvent::Trap {
                name: "Pit".to_string(),
                description: "".to_string(),
                damage: 10,
                effect: None,
            },
        );
        state.world = World::new();
        state.world.add_map(map);
        state.world.set_current_map(1);

        let content = crate::sdk::database::ContentDatabase::new();
        let _ = state.move_party_and_handle_events(crate::domain::types::Direction::East, &content);

        assert_eq!(
            state.party.members[0].hp.current, 40,
            "trap must deal 10 damage when levitate is not active"
        );
    }

    #[test]
    fn test_current_inn_id_at_inn_event() {
        let mut state = GameState::new();
        // Create a map with an EnterInn event at the party's position
        let mut map = crate::domain::world::Map::new(
            1,
            "Test Town".to_string(),
            "A test town".to_string(),
            10,
            10,
        );
        map.events.insert(
            crate::domain::types::Position::new(0, 0),
            crate::domain::world::MapEvent::EnterInn {
                name: "Town Inn".to_string(),
                description: "A cozy inn".to_string(),
                innkeeper_id: "town_innkeeper".to_string(),
            },
        );
        state.world.maps.insert(1, map);
        state.world.current_map = 1;
        state.world.party_position = crate::domain::types::Position::new(0, 0);

        assert_eq!(state.current_inn_id(), Some("town_innkeeper".to_string()));
    }

    #[test]
    fn test_current_inn_id_not_at_inn_but_inn_on_map() {
        let mut state = GameState::new();
        let mut map = crate::domain::world::Map::new(
            1,
            "Test Town".to_string(),
            "A test town".to_string(),
            10,
            10,
        );
        // Inn is at position (5, 5), party is at (0, 0)
        map.events.insert(
            crate::domain::types::Position::new(5, 5),
            crate::domain::world::MapEvent::EnterInn {
                name: "Far Inn".to_string(),
                description: "An inn across town".to_string(),
                innkeeper_id: "far_innkeeper".to_string(),
            },
        );
        state.world.maps.insert(1, map);
        state.world.current_map = 1;
        state.world.party_position = crate::domain::types::Position::new(0, 0);

        assert_eq!(state.current_inn_id(), Some("far_innkeeper".to_string()));
    }

    #[test]
    fn test_current_inn_id_no_inn_on_map_no_campaign() {
        let state = GameState::new();
        // No maps loaded, no campaign → None
        assert_eq!(state.current_inn_id(), None);
    }

    #[test]
    fn test_current_inn_id_no_inn_on_map_with_campaign_fallback() {
        use crate::domain::campaign::LevelUpMode;

        let mut state = GameState::new();
        // Create a map with no inn events
        let map = crate::domain::world::Map::new(
            1,
            "Dungeon".to_string(),
            "A dark dungeon".to_string(),
            10,
            10,
        );
        state.world.maps.insert(1, map);
        state.world.current_map = 1;

        // Set up a campaign with a starting innkeeper
        state.campaign = Some(crate::sdk::campaign_loader::Campaign {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Test".to_string(),
            engine_version: "0.1.0".to_string(),
            required_features: Vec::new(),
            config: crate::sdk::campaign_loader::CampaignConfig {
                starting_map: 1,
                starting_position: crate::domain::types::Position::new(0, 0),
                starting_direction: crate::domain::types::Direction::North,
                starting_gold: 0,
                starting_food: 0,
                starting_innkeeper: "default_innkeeper".to_string(),
                max_party_size: 6,
                max_roster_size: 20,
                difficulty: crate::sdk::campaign_loader::Difficulty::Normal,
                permadeath: false,
                allow_multiclassing: false,
                starting_level: 1,
                max_level: 20,
                level_up_mode: LevelUpMode::Auto,
                base_xp: 1000,
                xp_multiplier: 1.5,
                starting_time: crate::domain::types::GameTime::new(1, 8, 0),
            },
            data: crate::sdk::campaign_loader::CampaignData {
                items: "data/items.ron".to_string(),
                spells: "data/spells.ron".to_string(),
                monsters: "data/monsters.ron".to_string(),
                classes: "data/classes.ron".to_string(),
                races: "data/races.ron".to_string(),
                maps: "data/maps".to_string(),
                quests: "data/quests.ron".to_string(),
                dialogues: "data/dialogues.ron".to_string(),
                characters: "data/characters.ron".to_string(),
                creatures: "data/creatures.ron".to_string(),
                furniture: "data/furniture.ron".to_string(),
            },
            assets: crate::sdk::campaign_loader::CampaignAssets {
                tilesets: "assets/tilesets".to_string(),
                audio: "assets/audio".to_string(),
                music: "assets/music".to_string(),
                sounds: "assets/sounds".to_string(),
                images: "assets/images".to_string(),
                fonts: "assets/fonts".to_string(),
            },
            root_path: std::path::PathBuf::new(),
            game_config: crate::sdk::game_config::GameConfig::default(),
        });

        // Should fall back to campaign starting innkeeper
        assert_eq!(
            state.current_inn_id(),
            Some("default_innkeeper".to_string())
        );
    }

    // ── SpellBook state transition tests ────────────────────────────────────

    #[test]
    fn test_enter_spellbook_sets_mode() {
        let mut state = GameState::new();
        state.enter_spellbook(2);
        assert!(
            matches!(state.mode, GameMode::SpellBook(_)),
            "enter_spellbook must transition mode to GameMode::SpellBook"
        );
    }

    #[test]
    fn test_enter_spellbook_character_index() {
        let mut state = GameState::new();
        state.enter_spellbook(2);
        if let GameMode::SpellBook(ref sb) = state.mode {
            assert_eq!(sb.character_index, 2);
        } else {
            panic!("expected SpellBook mode");
        }
    }

    #[test]
    fn test_enter_spellbook_stores_previous_mode() {
        let mut state = GameState::new();
        // Default is Exploration
        assert!(matches!(state.mode, GameMode::Exploration));
        state.enter_spellbook(0);
        if let GameMode::SpellBook(ref sb) = state.mode {
            assert!(
                matches!(sb.get_resume_mode(), GameMode::Exploration),
                "previous_mode must be Exploration"
            );
        } else {
            panic!("expected SpellBook mode");
        }
    }

    #[test]
    fn test_enter_spellbook_with_caster_select_starts_at_index_zero() {
        let mut state = GameState::new();
        state.enter_spellbook_with_caster_select();
        if let GameMode::SpellBook(ref sb) = state.mode {
            assert_eq!(
                sb.character_index, 0,
                "enter_spellbook_with_caster_select must start at index 0"
            );
        } else {
            panic!("expected SpellBook mode");
        }
    }

    #[test]
    fn test_exit_spellbook_restores_previous_mode() {
        let mut state = GameState::new();
        state.enter_spellbook(0);
        assert!(matches!(state.mode, GameMode::SpellBook(_)));
        state.exit_spellbook();
        assert!(
            matches!(state.mode, GameMode::Exploration),
            "exit_spellbook must restore Exploration mode"
        );
    }

    #[test]
    fn test_exit_spellbook_noop_when_not_in_spellbook_mode() {
        let mut state = GameState::new();
        // mode is Exploration, not SpellBook
        assert!(matches!(state.mode, GameMode::Exploration));
        state.exit_spellbook();
        // still Exploration — no-op
        assert!(
            matches!(state.mode, GameMode::Exploration),
            "exit_spellbook must be a no-op when mode is not SpellBook"
        );
    }

    #[test]
    fn test_enter_character_sheet_sets_mode() {
        let mut state = GameState::new();
        state.enter_character_sheet();
        assert!(matches!(state.mode, GameMode::CharacterSheet(_)));
    }

    #[test]
    fn test_enter_character_sheet_stores_previous_mode() {
        let mut state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));
        state.enter_character_sheet();
        if let GameMode::CharacterSheet(ref cs) = state.mode {
            assert!(
                matches!(cs.get_resume_mode(), GameMode::Exploration),
                "CharacterSheet must store Exploration as the previous mode"
            );
        } else {
            panic!("expected CharacterSheet mode");
        }
    }

    #[test]
    fn test_enter_character_sheet_idempotent() {
        let mut state = GameState::new();
        state.enter_character_sheet();
        // Capture the inner state after first call
        let first_focused_index = if let GameMode::CharacterSheet(ref cs) = state.mode {
            cs.focused_index
        } else {
            panic!("expected CharacterSheet mode");
        };
        // Second call must be a no-op
        state.enter_character_sheet();
        assert!(matches!(state.mode, GameMode::CharacterSheet(_)));
        if let GameMode::CharacterSheet(ref cs) = state.mode {
            assert_eq!(cs.focused_index, first_focused_index);
        }
    }

    #[test]
    fn test_close_modal_closes_character_sheet_to_resume_mode() {
        let mut state = GameState::new();
        state.enter_character_sheet();
        assert!(matches!(state.mode, GameMode::CharacterSheet(_)));
        let closed = state.close_modal();
        assert!(closed);
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    // ── new_game campaign_config propagation tests ────────────────────────────

    /// `GameState::new_game` must bridge `LevelingConfig` (from `config.ron`)
    /// and campaign metadata (from `campaign.ron`) into `GameState::campaign_config`
    /// so that the progression system and combat XP award use the correct
    /// runtime values instead of hard-coded defaults.
    ///
    /// The discriminating assertion is `max_party_level == Some(20)`:
    /// `CampaignConfig::default()` gives `None`, but the test campaign's
    /// `campaign.ron` sets `max_level: 20`, so a properly wired `new_game`
    /// must produce `Some(20)`.
    #[test]
    fn test_new_game_propagates_leveling_config_to_campaign_config() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let campaign_path = std::path::PathBuf::from(manifest_dir).join("data/test_campaign");
        let campaign = crate::sdk::campaign_loader::Campaign::load(&campaign_path)
            .expect("data/test_campaign must load cleanly");
        let (state, _) =
            GameState::new_game(campaign).expect("new_game must succeed for test_campaign");

        // Level-up mode — from data/test_campaign/config.ron `leveling: LevelingConfig`
        assert_eq!(
            state.campaign_config.level_up_mode,
            crate::domain::campaign::LevelUpMode::Auto,
            "level_up_mode must be Auto (from config.ron LevelingConfig)"
        );

        // XP curve fields — from data/test_campaign/config.ron
        assert_eq!(
            state.campaign_config.base_xp, 1000,
            "base_xp must be 1000 (from config.ron LevelingConfig)"
        );
        assert!(
            (state.campaign_config.xp_multiplier - 1.5).abs() < 1e-9,
            "xp_multiplier must be 1.5 (from config.ron LevelingConfig)"
        );
        assert_eq!(
            state.campaign_config.training_fee_base, 500,
            "training_fee_base must be 500 (from config.ron LevelingConfig)"
        );
        assert!(
            (state.campaign_config.training_fee_multiplier - 1.0).abs() < 1e-6,
            "training_fee_multiplier must be 1.0 (from config.ron LevelingConfig)"
        );

        // max_party_level — the discriminating assertion.
        // CampaignConfig::default() → None; campaign.ron `max_level: 20` → Some(20).
        assert_eq!(
            state.campaign_config.max_party_level,
            Some(20),
            "max_party_level must be Some(20) from campaign.ron max_level: 20, not None"
        );

        // permadeath — from data/test_campaign/campaign.ron `permadeath: false`
        assert!(
            !state.campaign_config.permadeath,
            "permadeath must be false (from campaign.ron)"
        );
    }

    /// `GameState::new_game` must set `campaign_config.max_party_level` to
    /// `None` when `campaign.config.max_level == 0`, treating 0 as "no cap".
    #[test]
    fn test_new_game_max_party_level_none_when_max_level_zero() {
        use crate::sdk::campaign_loader::Campaign;
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let campaign_path = std::path::PathBuf::from(manifest_dir).join("data/test_campaign");
        let mut campaign =
            Campaign::load(&campaign_path).expect("data/test_campaign must load cleanly");
        // Override max_level to 0 (invalid / "no cap" sentinel)
        campaign.config.max_level = 0;
        let (state, _) = GameState::new_game(campaign).expect("new_game must succeed");
        assert_eq!(
            state.campaign_config.max_party_level, None,
            "max_level == 0 must map to max_party_level == None (no cap)"
        );
    }

    #[test]
    fn test_enter_spellbook_from_automap_mode_restores_automap() {
        let mut state = GameState::new();
        state.mode = GameMode::Automap;
        state.enter_spellbook(1);
        state.exit_spellbook();
        assert!(
            matches!(state.mode, GameMode::Automap),
            "exit_spellbook must restore the mode that was active before SpellBook"
        );
    }
}
