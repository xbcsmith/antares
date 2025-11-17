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

pub mod save_game;

use crate::domain::character::{Party, Roster};
use crate::domain::types::GameTime;
use crate::domain::world::World;
use crate::sdk::campaign_loader::Campaign;
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameMode {
    /// Exploring the world, moving through maps
    Exploration,
    /// Turn-based tactical combat
    Combat,
    /// Menu system (character management, inventory)
    Menu,
    /// NPC dialogue and interactions
    Dialogue,
}

// ===== Active Spell Effects =====

/// Party-wide active spell effects (separate from character conditions)
///
/// Each field represents duration remaining (0 = not active)
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl Default for ActiveSpells {
    fn default() -> Self {
        Self::new()
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
        self.objectives.push(QuestObjective {
            description,
            completed: false,
        });
    }

    /// Returns true if all objectives are completed
    pub fn is_completed(&self) -> bool {
        !self.objectives.is_empty() && self.objectives.iter().all(|obj| obj.completed)
    }
}

/// Quest log tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestLog {
    /// Active quests
    pub active_quests: Vec<Quest>,
    /// Completed quest IDs
    pub completed_quests: Vec<String>,
}

impl QuestLog {
    /// Creates a new empty quest log
    pub fn new() -> Self {
        Self {
            active_quests: Vec::new(),
            completed_quests: Vec::new(),
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
}

impl Default for QuestLog {
    fn default() -> Self {
        Self::new()
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
    /// Current game mode
    pub mode: GameMode,
    /// Game time
    pub time: GameTime,
    /// Quest log
    pub quests: QuestLog,
}

impl GameState {
    /// Creates a new game state with default values (no campaign)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::{GameState, GameMode};
    ///
    /// let state = GameState::new();
    /// assert_eq!(state.mode, GameMode::Exploration);
    /// assert_eq!(state.time.day, 1);
    /// assert!(state.campaign.is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            campaign: None,
            world: World::new(),
            roster: Roster::new(),
            party: Party::new(),
            active_spells: ActiveSpells::new(),
            mode: GameMode::Exploration,
            time: GameTime::new(1, 6, 0), // Day 1, 6:00 AM
            quests: QuestLog::new(),
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
    /// let loader = CampaignLoader::new("campaigns");
    /// let campaign = loader.load_campaign("tutorial")?;
    /// let state = GameState::new_game(campaign);
    ///
    /// assert!(state.campaign.is_some());
    /// assert_eq!(state.party.gold, state.campaign.as_ref().unwrap().config.starting_gold);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_game(campaign: Campaign) -> Self {
        let starting_gold = campaign.config.starting_gold;
        let starting_food = campaign.config.starting_food;

        let mut party = Party::new();
        party.gold = starting_gold;
        party.food = starting_food;

        Self {
            campaign: Some(campaign),
            world: World::new(),
            roster: Roster::new(),
            party,
            active_spells: ActiveSpells::new(),
            mode: GameMode::Exploration,
            time: GameTime::new(1, 6, 0), // Day 1, 6:00 AM
            quests: QuestLog::new(),
        }
    }

    /// Enters combat mode
    pub fn enter_combat(&mut self) {
        self.mode = GameMode::Combat;
    }

    /// Exits combat mode and returns to exploration
    pub fn exit_combat(&mut self) {
        self.mode = GameMode::Exploration;
    }

    /// Enters menu mode
    pub fn enter_menu(&mut self) {
        self.mode = GameMode::Menu;
    }

    /// Enters dialogue mode
    pub fn enter_dialogue(&mut self) {
        self.mode = GameMode::Dialogue;
    }

    /// Returns to exploration mode
    pub fn return_to_exploration(&mut self) {
        self.mode = GameMode::Exploration;
    }

    /// Advances game time by the specified number of minutes
    pub fn advance_time(&mut self, minutes: u32) {
        self.time.advance_minutes(minutes);
        // Tick active spell durations
        for _ in 0..minutes {
            self.active_spells.tick();
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
    use crate::domain::character::{Alignment, Character, Class, Race, Sex};

    #[test]
    fn test_game_state_creation() {
        let state = GameState::new();
        assert_eq!(state.mode, GameMode::Exploration);
        assert!(state.party.is_empty());
        assert_eq!(state.time.day, 1);
    }

    #[test]
    fn test_state_transition_preserves_party() {
        let mut state = GameState::new();

        // Add a character to the party
        let hero = Character::new(
            "Hero".to_string(),
            Race::Human,
            Class::Knight,
            Sex::Male,
            Alignment::Good,
        );
        state.party.add_member(hero).unwrap();

        // Transition to combat
        state.enter_combat();
        assert_eq!(state.mode, GameMode::Combat);
        assert_eq!(state.party.size(), 1);

        // Exit combat
        state.exit_combat();
        assert_eq!(state.mode, GameMode::Exploration);
        assert_eq!(state.party.size(), 1);
    }

    #[test]
    fn test_game_modes() {
        let mut state = GameState::new();

        state.enter_menu();
        assert_eq!(state.mode, GameMode::Menu);

        state.enter_dialogue();
        assert_eq!(state.mode, GameMode::Dialogue);

        state.return_to_exploration();
        assert_eq!(state.mode, GameMode::Exploration);
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

        state.advance_time(5);
        assert_eq!(state.active_spells.light, 5);
        assert_eq!(state.time.minute, 5);
    }
}
