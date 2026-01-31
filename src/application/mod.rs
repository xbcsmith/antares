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

pub mod dialogue;
pub mod menu;
pub mod quests;
pub mod resources;
pub mod save_game;

use crate::application::menu::MenuState;
use crate::domain::character::{Party, Roster};
use crate::domain::party_manager::{PartyManagementError, PartyManager};
use crate::domain::types::{GameTime, InnkeeperId};
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
    /// Menu system (character management, inventory)
    Menu(MenuState),
    /// NPC dialogue and interactions
    Dialogue(crate::application::dialogue::DialogueState),
    /// Inn party management interface
    InnManagement(InnManagementState),
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

        // Basic Phase 1 validation: ensure core content groups are present
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
        let starting_food = campaign.config.starting_food;

        let mut party = Party::new();
        party.gold = starting_gold;
        party.food = starting_food;

        // Preserve campaign-specific game configuration for state
        let campaign_config = campaign.game_config.clone();

        let mut state = Self {
            campaign: Some(campaign),
            world: World::new(),
            roster: Roster::new(),
            party,
            active_spells: ActiveSpells::new(),
            config: campaign_config,
            mode: GameMode::Exploration,
            time: GameTime::new(1, 6, 0), // Day 1, 6:00 AM
            quests: QuestLog::new(),
            encountered_characters: std::collections::HashSet::new(),
        };

        // Phase 2: Initialize roster from content database (premade characters)
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
    /// let loader = CampaignLoader::new("campaigns");
    /// let campaign = loader.load_campaign("tutorial")?;
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
            let character =
                def.instantiate(&content_db.races, &content_db.classes, &content_db.items)?;

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
    /// let loader = CampaignLoader::new("campaigns");
    /// let campaign = loader.load_campaign("tutorial")?;
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
    /// let loader = CampaignLoader::new("campaigns");
    /// let campaign = loader.load_campaign("tutorial")?;
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
    /// let loader = CampaignLoader::new("campaigns");
    /// let campaign = loader.load_campaign("tutorial")?;
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

    /// Gets the current inn ID from game state
    ///
    /// Returns the inn/town ID where the party is currently located, if any.
    /// This is used to determine where dismissed characters should be stored.
    ///
    /// # Returns
    ///
    /// Returns `Some(InnkeeperId)` if party is at an inn, `None` otherwise.
    ///
    /// # Note
    ///
    /// This is a placeholder implementation. Full implementation requires
    /// innkeeper-based location system to be completed.
    pub fn current_inn_id(&self) -> Option<InnkeeperId> {
        // TODO: Implement once innkeeper-based location system is complete
        // For now, return None as a safe default
        None
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
    /// let loader = CampaignLoader::new("campaigns");
    /// let campaign = loader.load_campaign("tutorial")?;
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
    /// let loader = CampaignLoader::new("campaigns");
    /// let campaign = loader.load_campaign("tutorial")?;
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
        let character =
            char_def.instantiate(&content_db.races, &content_db.classes, &content_db.items)?;

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
            if let Some(monster_group) =
                crate::domain::world::random_encounter(&self.world, &mut rng)
            {
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

                // Enter combat with prepared combat state
                self.mode = GameMode::Combat(cs);

                // Combat occurred instead of triggering a tile event; return early.
                return Ok(());
            }
        }

        // No random encounter (or a tile event exists) - handle tile event as before
        let ev = crate::domain::world::trigger_event(&mut self.world, position)
            .map_err(MoveHandleError::Event)?;

        match ev {
            crate::domain::world::EventResult::Encounter { monster_group } => {
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

                // Enter combat with prepared combat state
                self.mode = GameMode::Combat(cs);
            }

            crate::domain::world::EventResult::NpcDialogue { npc_id } => {
                // Start dialogue mode (dialogue state may need NPC context)
                let _ = npc_id; // TODO: pass npc_id into dialogue state when implemented
                self.mode = GameMode::Dialogue(crate::application::dialogue::DialogueState::new());
            }

            _ => {
                // Other events (treasure, teleport, trap, etc.) are handled elsewhere or are no-ops here
            }
        }

        Ok(())
    }

    /// Exits combat mode and returns to exploration
    pub fn exit_combat(&mut self) {
        self.mode = GameMode::Exploration;
    }

    /// Enters menu mode
    pub fn enter_menu(&mut self) {
        let prev = self.mode.clone();
        self.mode = GameMode::Menu(MenuState::new(prev));
    }

    /// Enters dialogue mode
    pub fn enter_dialogue(&mut self) {
        self.mode = GameMode::Dialogue(crate::application::dialogue::DialogueState::new());
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
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("campaigns");
        let campaign = loader
            .load_campaign("tutorial")
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
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("campaigns");
        let campaign = loader
            .load_campaign("tutorial")
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
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("campaigns");
        let campaign = loader
            .load_campaign("tutorial")
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
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("campaigns");
        let campaign = loader
            .load_campaign("tutorial")
            .expect("Failed to load tutorial campaign");

        let (state, content_db) = GameState::new_game(campaign).expect("new_game failed");

        let expected = content_db.characters.premade_characters().count();
        assert_eq!(state.roster.characters.len(), expected);
    }

    #[test]
    fn test_initialize_roster_applies_class_modifiers() {
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("campaigns");
        let campaign = loader
            .load_campaign("tutorial")
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
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("campaigns");
        let campaign = loader
            .load_campaign("tutorial")
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
        let loader = crate::sdk::campaign_loader::CampaignLoader::new("campaigns");
        let campaign = loader
            .load_campaign("tutorial")
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
            },
            assets: crate::sdk::campaign_loader::CampaignAssets {
                tilesets: "tilesets".to_string(),
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
            },
            assets: crate::sdk::campaign_loader::CampaignAssets {
                tilesets: "tilesets".to_string(),
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

    // ===== Phase 2: Party Management Integration Tests =====

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

    // ===== Phase 5: Persistence & Save Game Integration Tests =====

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
}
