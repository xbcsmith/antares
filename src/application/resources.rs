// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Application-level resources exposed to Bevy systems
//!
//! This module defines a thin resource wrapper around the SDK's
//! `ContentDatabase`, making campaign content available to ECS systems
//! via Bevy's resource mechanism.
//!
//! It also contains application-layer enforcement helpers for campaign rules
//! such as permadeath that must be checked before domain operations are
//! performed.

use crate::application::GameState;
use crate::domain::campaign::CampaignConfig;
use crate::domain::levels::LevelDatabase;
use crate::domain::progression::{
    check_level_up_with_db, level_up_and_grant_spells_with_level_db, ProgressionError,
};
use crate::domain::types::SpellId;
use crate::domain::validation::ValidationError;
use crate::sdk::database::ContentDatabase;
use bevy::prelude::*;
use rand::Rng;
use thiserror::Error;

/// Wrapper resource exposing campaign content as a Bevy resource.
///
/// Systems can fetch this resource to query items, spells, maps, and
/// other campaign data loaded by the SDK.
///
/// # Examples
///
/// ```no_run
/// use antares::application::resources::GameContent;
/// use antares::sdk::database::ContentDatabase;
///
/// let db = ContentDatabase::new();
/// let content = GameContent::new(db);
/// assert_eq!(content.db().classes.all_classes().count(), 0);
/// ```
#[derive(Resource, Debug, Clone)]
pub struct GameContent(pub ContentDatabase);

impl GameContent {
    /// Create a new `GameContent` resource from a `ContentDatabase`.
    pub fn new(db: ContentDatabase) -> Self {
        Self(db)
    }

    /// Immutable access to the underlying `ContentDatabase`.
    pub fn db(&self) -> &ContentDatabase {
        &self.0
    }

    /// Mutable access to the underlying `ContentDatabase`.
    pub fn db_mut(&mut self) -> &mut ContentDatabase {
        &mut self.0
    }
}

/// Returns `Ok(())` if the campaign allows resurrection, or a
/// [`ValidationError::PreconditionFailed`] when permadeath is enabled.
///
/// This helper must be called by any application-layer or game-system code
/// that is about to apply a `ConsumableEffect::Resurrect` or cast a spell
/// with `resurrect_hp: Some(_)`.  The domain layer does **not** check
/// permadeath — enforcement is the caller's responsibility.
///
/// # Arguments
///
/// * `config` — the active campaign's [`CampaignConfig`].
///
/// # Errors
///
/// Returns [`ValidationError::PreconditionFailed`] with a human-readable
/// message when `config.permadeath == true`.
///
/// # Examples
///
/// ```
/// use antares::application::resources::check_permadeath_allows_resurrection;
/// use antares::domain::campaign::CampaignConfig;
///
/// // Default config has permadeath == false → resurrection allowed.
/// let config = CampaignConfig::default();
/// assert!(check_permadeath_allows_resurrection(&config).is_ok());
///
/// // Permadeath config → resurrection blocked.
/// let mut pd_config = CampaignConfig::default();
/// pd_config.permadeath = true;
/// assert!(check_permadeath_allows_resurrection(&pd_config).is_err());
/// ```
pub fn check_permadeath_allows_resurrection(
    config: &CampaignConfig,
) -> Result<(), ValidationError> {
    if config.permadeath {
        Err(ValidationError::PreconditionFailed(
            "Resurrection is not allowed in this campaign (permadeath enabled).".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Errors returned by [`perform_training_service`].
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TrainingError {
    /// The specified NPC is not flagged as a trainer.
    #[error("NPC '{0}' is not a trainer")]
    NotATrainer(String),

    /// The target character is dead or does not yet have enough XP for a level-up.
    #[error("Character is not eligible for level-up (dead or insufficient XP)")]
    CharacterNotEligible,

    /// The party does not have enough gold to pay the training fee.
    #[error("Insufficient gold: need {need}, have {have}")]
    InsufficientGold { need: u32, have: u32 },

    /// The underlying domain-level level-up operation failed.
    #[error("Level-up failed: {0}")]
    LevelUpFailed(#[from] ProgressionError),
}

/// Performs a trainer NPC level-up service for the party member at `party_index`.
///
/// This function is the authoritative application-layer entry point for the
/// NPC trainer flow.  It enforces all preconditions atomically before modifying
/// any state.
///
/// ## Steps performed
///
/// 1. Looks up the NPC by `npc_id` in `db.npcs`.
/// 2. Verifies the NPC has `is_trainer == true`.
/// 3. Verifies the party member at `party_index` is alive and eligible for
///    level-up via [`check_level_up_with_db`].
/// 4. Computes the training fee via
///    [`NpcDefinition::training_fee_for_level`][crate::domain::world::npc::NpcDefinition::training_fee_for_level].
/// 5. Checks `party.gold >= fee`; returns [`TrainingError::InsufficientGold`] if not.
/// 6. Deducts gold and calls [`level_up_and_grant_spells_with_level_db`]
///    using `db.classes` and `db.spells`.
/// 7. Returns `Ok((hp_gained, spells_granted))`.
///
/// # Arguments
///
/// * `game_state`   — Mutable game state; party gold and character data are
///   modified on success.
/// * `npc_id`       — String ID of the trainer NPC.
/// * `party_index`  — Index into `game_state.party.members`.
/// * `level_db`     — Optional per-class XP threshold table (`None` = formula fallback).
/// * `rng`          — Random-number generator for HP rolls.
/// * `db`           — Content database for NPC, class, and spell lookups.
///
/// # Returns
///
/// `Ok((hp_gained, spells_granted))` on success.
///
/// # Errors
///
/// Returns [`TrainingError`] when any precondition fails.
///
/// # Examples
///
/// ```
/// use antares::application::GameState;
/// use antares::application::resources::perform_training_service;
/// use antares::domain::character::{Alignment, Character, Sex};
/// use antares::domain::progression::award_experience;
/// use antares::domain::world::npc::NpcDefinition;
/// use antares::sdk::database::ContentDatabase;
/// use antares::domain::classes::ClassDatabase;
///
/// let mut state = GameState::new();
/// state.campaign_config.level_up_mode =
///     antares::domain::campaign::LevelUpMode::NpcTrainer;
///
/// // Add a trainer NPC to the content DB
/// let mut db = ContentDatabase::new();
/// db.classes = ClassDatabase::load_from_file("data/classes.ron").unwrap();
/// let trainer = NpcDefinition::trainer("trainer_bob", "Trainer Bob", "bob.png", 100);
/// db.npcs.add_npc(trainer).unwrap();
///
/// // Add a knight with enough XP for level 2 (1000 XP)
/// let mut knight = Character::new(
///     "Sir Lancelot".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// knight.hp.base = 30;
/// knight.hp.current = 30;
/// award_experience(&mut knight, 1_000).unwrap();
/// state.party.members.push(knight);
/// state.party.gold = 500;
///
/// let mut rng = rand::rng();
///
/// let result = perform_training_service(
///     &mut state,
///     "trainer_bob",
///     0,
///     None,
///     &mut rng,
///     &db,
/// );
///
/// assert!(result.is_ok());
/// assert_eq!(state.party.members[0].level, 2);
/// assert_eq!(state.party.gold, 400); // 500 - 100 fee
/// ```
pub fn perform_training_service(
    game_state: &mut GameState,
    npc_id: &str,
    party_index: usize,
    level_db: Option<&LevelDatabase>,
    rng: &mut impl Rng,
    db: &ContentDatabase,
) -> Result<(u16, Vec<SpellId>), TrainingError> {
    // Step 1: Look up and validate the NPC.
    let npc = db
        .npcs
        .get_npc(npc_id)
        .ok_or_else(|| TrainingError::NotATrainer(npc_id.to_string()))?;

    // Step 2: Verify the NPC is a trainer.
    if !npc.is_trainer {
        return Err(TrainingError::NotATrainer(npc_id.to_string()));
    }

    // Clone what we need before the mutable borrow of game_state.
    let npc_clone = npc.clone();

    // Step 3: Verify eligibility — alive and enough XP.
    let member = game_state
        .party
        .members
        .get(party_index)
        .ok_or(TrainingError::CharacterNotEligible)?;

    if !member.is_alive() || !check_level_up_with_db(member, level_db) {
        return Err(TrainingError::CharacterNotEligible);
    }

    let current_level = member.level;

    // Step 4: Compute the training fee for the character's current level.
    let fee = npc_clone.training_fee_for_level(current_level, &game_state.campaign_config);

    // Step 5: Check gold.
    let gold = game_state.party.gold;
    if gold < fee {
        return Err(TrainingError::InsufficientGold {
            need: fee,
            have: gold,
        });
    }

    // Step 6: Deduct gold and apply the level-up using class/spell DBs from content.
    game_state.party.gold = gold.saturating_sub(fee);

    let max_level = game_state.campaign_config.max_party_level;
    let result = level_up_and_grant_spells_with_level_db(
        &mut game_state.party.members[party_index],
        &db.classes,
        &db.spells,
        level_db,
        max_level,
        rng,
    )?;

    Ok(result)
}

/// Performs a priest resurrection service for the party member at `character_index`.
///
/// This function is the authoritative application-layer entry point for the
/// NPC temple resurrection flow.  It enforces all preconditions before
/// modifying any state so that every failure is clean and atomic.
///
/// ## Steps performed
///
/// 1. Looks up the NPC by `npc_id` in `content.npcs`.
/// 2. Verifies the NPC has a `"resurrect"` service entry in its
///    `service_catalog` and reads the gold/gem cost from it.
/// 3. Checks campaign permadeath via
///    [`check_permadeath_allows_resurrection`].
/// 4. Verifies the target character exists and is dead
///    (`conditions.is_dead() == true`).  Characters with `STONE` or
///    `ERADICATED` conditions are not considered dead by `is_dead()` and
///    will cause this step to fail.
/// 5. Verifies the party has enough gold (≥ `service.cost`).
/// 6. Verifies the party has enough gems (≥ `service.gem_cost`).
/// 7. Deducts gold and gems from the party.
/// 8. Calls [`crate::domain::resources::revive_from_dead`] with `hp = 1`.
///
/// # Arguments
///
/// * `game_state` — Mutable game state (party gold/gems and character
///   conditions are modified on success).
/// * `npc_id` — String ID of the priest NPC offering the service
///   (e.g. `"temple_priest"`).
/// * `character_index`  — Index into `game_state.party.members`.
/// * `content`          — Content database for NPC lookup.
///
/// # Returns
///
/// `Ok(())` on success; the character at `character_index` is revived with
/// 1 HP and the party's gold/gems are reduced by the service cost.
///
/// # Errors
///
/// Returns a [`ValidationError`] for any of the following:
///
/// | Condition                                              | Error contains         |
/// |--------------------------------------------------------|------------------------|
/// | `npc_id` not in `content.npcs`                        | `"not found"`          |
/// | NPC has no `service_catalog` or no `"resurrect"` entry| `"resurrect"`        |
/// | Campaign has `permadeath == true`                      | `"permadeath"`         |
/// | No party member at `character_index`                   | `"No party member"`    |
/// | Target character is not dead                           | `"not dead"`           |
/// | Party gold < service cost                              | `"Insufficient gold"`  |
/// | Party gems < service gem cost                          | `"Insufficient gems"`  |
///
/// # Examples
///
/// ```
/// use antares::application::GameState;
/// use antares::application::resources::perform_resurrection_service;
/// use antares::sdk::database::{ContentDatabase, NpcDatabase};
/// use antares::domain::world::npc::NpcDefinition;
/// use antares::domain::inventory::{ServiceCatalog, ServiceEntry};
/// use antares::domain::character::{Character, Condition, Sex, Alignment};
///
/// // Build a minimal content database with a priest NPC
/// let mut db = ContentDatabase::new();
/// let mut priest = NpcDefinition::new("temple_priest", "Temple Priest", "priest.png");
/// priest.is_priest = true;
/// let mut catalog = ServiceCatalog::new();
/// catalog.services.push(ServiceEntry::with_gem_cost(
///     "resurrect", 500, 1, "Raise a dead party member",
/// ));
/// priest.service_catalog = Some(catalog);
/// db.npcs.add_npc(priest).unwrap();
///
/// // Put a dead character in the party
/// let mut state = GameState::new();
/// let mut hero = Character::new(
///     "Sir Lancelot".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
/// hero.hp.current = 0;
/// hero.conditions.add(Condition::DEAD);
/// hero.add_condition(antares::domain::conditions::ActiveCondition::new(
///     "dead".to_string(),
///     antares::domain::conditions::ConditionDuration::Permanent,
/// ));
/// state.party.members.push(hero);
/// state.party.gold = 1000;
/// state.party.gems = 5;
///
/// assert!(perform_resurrection_service(&mut state, "temple_priest", 0, &db).is_ok());
/// assert_eq!(state.party.gold, 500); // 1000 - 500
/// assert_eq!(state.party.gems, 4);   // 5 - 1
/// assert!(state.party.members[0].conditions.is_fine() ||
///         state.party.members[0].hp.current > 0);
/// ```
pub fn perform_resurrection_service(
    game_state: &mut GameState,
    npc_id: &str,
    character_index: usize,
    content: &ContentDatabase,
) -> Result<(), ValidationError> {
    // Step 1 & 2: Look up NPC and clone the resurrect service entry.
    // The clone releases the immutable borrow of `content` before we touch
    // the mutable `game_state` below.
    let service = {
        let npc = content.npcs.get_npc(npc_id).ok_or_else(|| {
            ValidationError::NotFound(format!("NPC '{}' not found in content database", npc_id))
        })?;

        let catalog = npc.service_catalog.as_ref().ok_or_else(|| {
            ValidationError::NotFound(format!("NPC '{}' does not offer any services", npc_id))
        })?;

        catalog
            .get_service("resurrect")
            .ok_or_else(|| {
                ValidationError::NotFound(format!(
                    "NPC '{}' does not offer the 'resurrect' service",
                    npc_id
                ))
            })?
            .clone()
    };

    // Step 3: Enforce campaign permadeath rule.
    check_permadeath_allows_resurrection(&game_state.campaign_config)?;

    // Step 4: Verify the target exists and is dead (read-only borrow ends
    // at the closing brace so we can mutably borrow `game_state` below).
    {
        let character = game_state
            .party
            .members
            .get(character_index)
            .ok_or_else(|| {
                ValidationError::NotFound(format!("No party member at index {}", character_index))
            })?;

        if !character.conditions.is_dead() {
            return Err(ValidationError::PreconditionFailed(format!(
                "Party member '{}' at index {} is not dead",
                character.name, character_index
            )));
        }
    }

    // Step 5: Check gold.
    if game_state.party.gold < service.cost {
        return Err(ValidationError::InsufficientResources(format!(
            "Insufficient gold: resurrection costs {} gold but the party only has {}",
            service.cost, game_state.party.gold
        )));
    }

    // Step 6: Check gems.
    if game_state.party.gems < service.gem_cost {
        return Err(ValidationError::InsufficientResources(format!(
            "Insufficient gems: resurrection costs {} gem(s) but the party only has {}",
            service.gem_cost, game_state.party.gems
        )));
    }

    // Step 7: Deduct resources.
    game_state.party.gold -= service.cost;
    game_state.party.gems -= service.gem_cost;

    // Step 8: Revive the character with 1 HP.
    let character = &mut game_state.party.members[character_index];
    crate::domain::resources::revive_from_dead(character, 1);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::campaign::CampaignConfig;
    use crate::domain::character::{Alignment, Character, Condition, Sex};
    use crate::domain::inventory::{ServiceCatalog, ServiceEntry};
    use crate::domain::world::npc::NpcDefinition;

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Builds a `ContentDatabase` with a single priest NPC that offers the
    /// `"resurrect"` service for `cost` gold and `gem_cost` gems.
    fn content_with_priest(cost: u32, gem_cost: u32) -> ContentDatabase {
        let mut db = ContentDatabase::new();
        let mut priest = NpcDefinition::new("temple_priest", "Temple Priest", "priest.png");
        priest.is_priest = true;
        let mut catalog = ServiceCatalog::new();
        catalog.services.push(ServiceEntry::with_gem_cost(
            "resurrect",
            cost,
            gem_cost,
            "Raise a dead party member",
        ));
        priest.service_catalog = Some(catalog);
        db.npcs.add_npc(priest).expect("add_npc should not fail");
        db
    }

    /// Builds a `ContentDatabase` with a priest NPC that has a catalog but
    /// does NOT include a `"resurrect"` service.
    fn content_with_priest_no_resurrect() -> ContentDatabase {
        let mut db = ContentDatabase::new();
        let mut priest = NpcDefinition::new("temple_priest", "Temple Priest", "priest.png");
        priest.is_priest = true;
        let mut catalog = ServiceCatalog::new();
        catalog
            .services
            .push(ServiceEntry::new("heal_all", 50, "Heal all party members"));
        priest.service_catalog = Some(catalog);
        db.npcs.add_npc(priest).expect("add_npc should not fail");
        db
    }

    /// Creates a dead `Character` and pushes it into `game_state.party`.
    fn add_dead_member(game_state: &mut GameState, name: &str) {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};
        let mut c = Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.base = 20;
        c.hp.current = 0;
        c.conditions.add(Condition::DEAD);
        c.add_condition(ActiveCondition::new(
            "dead".to_string(),
            ConditionDuration::Permanent,
        ));
        game_state.party.members.push(c);
    }

    /// Creates a living `Character` and pushes it into `game_state.party`.
    fn add_living_member(game_state: &mut GameState, name: &str) {
        let mut c = Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.base = 20;
        c.hp.current = 15;
        game_state.party.members.push(c);
    }

    // ── existing tests ────────────────────────────────────────────────────────

    #[test]
    fn test_game_content_new() {
        let db = ContentDatabase::new();
        let resource = GameContent::new(db);
        // Basic smoke test: empty content database has zero classes
        assert_eq!(resource.db().classes.all_classes().count(), 0);
    }

    /// `check_permadeath_allows_resurrection` must return `Ok` when permadeath
    /// is disabled (the default).
    #[test]
    fn test_permadeath_allows_resurrection_by_default() {
        let config = CampaignConfig::default();
        assert!(
            check_permadeath_allows_resurrection(&config).is_ok(),
            "resurrection must be allowed when permadeath == false"
        );
    }

    /// `check_permadeath_allows_resurrection` must return `Err` when permadeath
    /// is enabled.
    #[test]
    fn test_permadeath_blocks_resurrection() {
        let config = CampaignConfig {
            permadeath: true,
            ..CampaignConfig::default()
        };
        let result = check_permadeath_allows_resurrection(&config);
        assert!(
            result.is_err(),
            "resurrection must be blocked when permadeath == true"
        );
        assert!(
            result.unwrap_err().to_string().contains("permadeath"),
            "error message must mention permadeath"
        );
    }

    // ── perform_resurrection_service tests ───────────────────────────────────

    /// Happy path: gold and gems are deducted and the dead character is revived
    /// to 1 HP.
    #[test]
    fn test_perform_resurrection_service_success() {
        let content = content_with_priest(500, 1);
        let mut state = GameState::new();
        state.party.gold = 1_000;
        state.party.gems = 5;
        add_dead_member(&mut state, "Aldric");

        let result = perform_resurrection_service(&mut state, "temple_priest", 0, &content);
        assert!(result.is_ok(), "expected Ok, got {:?}", result);

        // Gold and gems deducted
        assert_eq!(state.party.gold, 500, "500 gold should have been deducted");
        assert_eq!(state.party.gems, 4, "1 gem should have been deducted");

        // Character revived to 1 HP and no longer dead
        let hero = &state.party.members[0];
        assert!(
            hero.hp.current > 0,
            "revived character must have hp.current > 0"
        );
        assert!(
            !hero.conditions.is_dead(),
            "revived character must not be dead"
        );
    }

    /// Returns `Err` when the party has less gold than the service cost.
    #[test]
    fn test_perform_resurrection_service_insufficient_gold() {
        let content = content_with_priest(500, 1);
        let mut state = GameState::new();
        state.party.gold = 100; // not enough
        state.party.gems = 5;
        add_dead_member(&mut state, "Aldric");

        let result = perform_resurrection_service(&mut state, "temple_priest", 0, &content);
        assert!(result.is_err(), "expected Err for insufficient gold");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Insufficient gold"),
            "error must mention 'Insufficient gold', got: {}",
            msg
        );

        // State must be unchanged
        assert_eq!(state.party.gold, 100);
        assert_eq!(state.party.gems, 5);
        assert!(state.party.members[0].conditions.is_dead());
    }

    /// Returns `Err` when the party has fewer gems than the service gem cost.
    #[test]
    fn test_perform_resurrection_service_insufficient_gems() {
        let content = content_with_priest(500, 3);
        let mut state = GameState::new();
        state.party.gold = 1_000;
        state.party.gems = 2; // not enough
        add_dead_member(&mut state, "Aldric");

        let result = perform_resurrection_service(&mut state, "temple_priest", 0, &content);
        assert!(result.is_err(), "expected Err for insufficient gems");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Insufficient gems"),
            "error must mention 'Insufficient gems', got: {}",
            msg
        );

        // Gold must not have been spent
        assert_eq!(state.party.gold, 1_000);
        assert!(state.party.members[0].conditions.is_dead());
    }

    /// Returns `Err` when the target character is alive (not dead).
    #[test]
    fn test_perform_resurrection_service_target_not_dead() {
        let content = content_with_priest(500, 1);
        let mut state = GameState::new();
        state.party.gold = 1_000;
        state.party.gems = 5;
        add_living_member(&mut state, "Aldric"); // alive, not dead

        let result = perform_resurrection_service(&mut state, "temple_priest", 0, &content);
        assert!(result.is_err(), "expected Err when target is not dead");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("not dead"),
            "error must mention 'not dead', got: {}",
            msg
        );

        // Resources unchanged
        assert_eq!(state.party.gold, 1_000);
        assert_eq!(state.party.gems, 5);
    }

    /// Returns `Err` when the NPC ID is not in the content database.
    #[test]
    fn test_perform_resurrection_service_npc_not_found() {
        let content = ContentDatabase::new(); // empty DB
        let mut state = GameState::new();
        state.party.gold = 1_000;
        state.party.gems = 5;
        add_dead_member(&mut state, "Aldric");

        let result = perform_resurrection_service(&mut state, "nonexistent_priest", 0, &content);
        assert!(result.is_err(), "expected Err for unknown NPC");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("not found"),
            "error must mention 'not found', got: {}",
            msg
        );
    }

    /// Returns `Err` when the NPC exists but has no `"resurrect"` service.
    #[test]
    fn test_perform_resurrection_service_no_resurrect_service() {
        let content = content_with_priest_no_resurrect();
        let mut state = GameState::new();
        state.party.gold = 1_000;
        state.party.gems = 5;
        add_dead_member(&mut state, "Aldric");

        let result = perform_resurrection_service(&mut state, "temple_priest", 0, &content);
        assert!(
            result.is_err(),
            "expected Err when NPC lacks resurrect service"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("resurrect"),
            "error must mention 'resurrect', got: {}",
            msg
        );
    }

    /// Returns `Err` when the campaign has `permadeath == true`.
    #[test]
    fn test_perform_resurrection_service_permadeath() {
        let content = content_with_priest(500, 1);
        let mut state = GameState::new();
        state.campaign_config.permadeath = true;
        state.party.gold = 1_000;
        state.party.gems = 5;
        add_dead_member(&mut state, "Aldric");

        let result = perform_resurrection_service(&mut state, "temple_priest", 0, &content);
        assert!(result.is_err(), "expected Err when permadeath is enabled");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("permadeath"),
            "error must mention 'permadeath', got: {}",
            msg
        );

        // Character must still be dead; resources unchanged
        assert!(state.party.members[0].conditions.is_dead());
        assert_eq!(state.party.gold, 1_000);
    }

    /// Zero-cost service (cost == 0, gem_cost == 0) succeeds and deducts nothing.
    #[test]
    fn test_perform_resurrection_service_zero_cost_succeeds() {
        let content = content_with_priest(0, 0);
        let mut state = GameState::new();
        state.party.gold = 0;
        state.party.gems = 0;
        add_dead_member(&mut state, "Aldric");

        let result = perform_resurrection_service(&mut state, "temple_priest", 0, &content);
        assert!(
            result.is_ok(),
            "zero-cost resurrection must succeed even with empty treasury"
        );
        assert_eq!(state.party.gold, 0);
        assert_eq!(state.party.gems, 0);
        assert!(!state.party.members[0].conditions.is_dead());
    }

    // ── Training Service Tests ────────────────────────────────────────────────

    fn make_trainer_db() -> ContentDatabase {
        let mut db = ContentDatabase::new();
        db.classes = crate::domain::classes::ClassDatabase::load_from_file("data/classes.ron")
            .expect("classes.ron must exist for training tests");
        db.spells = crate::sdk::database::SpellDatabase::load_from_file("data/spells.ron")
            .unwrap_or_default();
        let trainer = crate::domain::world::npc::NpcDefinition::trainer(
            "trainer_bob",
            "Trainer Bob",
            "bob.png",
            100, // 100 gold base fee
        );
        db.npcs.add_npc(trainer).unwrap();
        db
    }

    fn make_knight_with_xp(xp: u64) -> crate::domain::character::Character {
        let mut c = crate::domain::character::Character::new(
            "Sir Lancelot".to_string(),
            "human".to_string(),
            "knight".to_string(),
            crate::domain::character::Sex::Male,
            crate::domain::character::Alignment::Good,
        );
        c.hp.base = 30;
        c.hp.current = 30;
        c.experience = xp;
        c
    }

    #[test]
    fn test_perform_training_service_success() {
        let mut state = GameState::new();
        state.campaign_config.level_up_mode = crate::domain::campaign::LevelUpMode::NpcTrainer;
        state.party.members.push(make_knight_with_xp(1_000));
        state.party.gold = 500;

        let db = make_trainer_db();
        let mut rng = rand::rng();

        let result = perform_training_service(&mut state, "trainer_bob", 0, None, &mut rng, &db);

        assert!(
            result.is_ok(),
            "training should succeed: {:?}",
            result.err()
        );
        let (hp_gained, _) = result.unwrap();
        assert!(hp_gained > 0, "knight must gain HP on level-up");
        assert_eq!(state.party.members[0].level, 2);
        assert_eq!(state.party.gold, 400); // 500 - 100 fee (100 * 1.0 * 1)
    }

    #[test]
    fn test_perform_training_service_insufficient_gold() {
        let mut state = GameState::new();
        state.campaign_config.level_up_mode = crate::domain::campaign::LevelUpMode::NpcTrainer;
        state.party.members.push(make_knight_with_xp(1_000));
        state.party.gold = 50; // Less than the 100 fee

        let db = make_trainer_db();
        let mut rng = rand::rng();

        let result = perform_training_service(&mut state, "trainer_bob", 0, None, &mut rng, &db);

        assert!(matches!(
            result,
            Err(TrainingError::InsufficientGold {
                need: 100,
                have: 50
            })
        ));
        assert_eq!(
            state.party.members[0].level, 1,
            "level must remain 1 on failure"
        );
        assert_eq!(state.party.gold, 50, "gold must not be deducted on failure");
    }

    #[test]
    fn test_perform_training_service_character_not_eligible() {
        let mut state = GameState::new();
        state.campaign_config.level_up_mode = crate::domain::campaign::LevelUpMode::NpcTrainer;
        state.party.members.push(make_knight_with_xp(0)); // No XP — not eligible
        state.party.gold = 500;

        let db = make_trainer_db();
        let mut rng = rand::rng();

        let result = perform_training_service(&mut state, "trainer_bob", 0, None, &mut rng, &db);

        assert!(
            matches!(result, Err(TrainingError::CharacterNotEligible)),
            "should be CharacterNotEligible, got {:?}",
            result
        );
    }

    #[test]
    fn test_perform_training_service_not_a_trainer() {
        let mut state = GameState::new();
        state.party.members.push(make_knight_with_xp(1_000));
        state.party.gold = 500;

        // NPC exists but is NOT a trainer
        let mut db = ContentDatabase::new();
        let mut npc =
            crate::domain::world::npc::NpcDefinition::new("plain_npc", "Plain NPC", "plain.png");
        npc.is_trainer = false;
        db.npcs.add_npc(npc).unwrap();

        let mut rng = rand::rng();

        let result = perform_training_service(&mut state, "plain_npc", 0, None, &mut rng, &db);

        assert!(
            matches!(result, Err(TrainingError::NotATrainer(_))),
            "should be NotATrainer, got {:?}",
            result
        );
    }

    #[test]
    fn test_perform_training_service_deducts_correct_fee_at_level_5() {
        let mut state = GameState::new();
        state.campaign_config.level_up_mode = crate::domain::campaign::LevelUpMode::NpcTrainer;

        // Character at level 5 with enough XP for level 6
        // Level 6 threshold (formula): 1000 * 5^1.5 ≈ 11180
        let mut knight = make_knight_with_xp(12_000);
        knight.level = 5;
        // Must set experience >= level-6 threshold
        state.party.members.push(knight);
        state.party.gold = 10_000;

        let db = make_trainer_db(); // base fee = 100
        let mut rng = rand::rng();

        let result = perform_training_service(&mut state, "trainer_bob", 0, None, &mut rng, &db);

        assert!(
            result.is_ok(),
            "training level 5→6 should succeed: {:?}",
            result.err()
        );
        // fee = 100 * 1.0 * 5 = 500
        assert_eq!(
            state.party.gold, 9_500,
            "expected 500 gold deducted for level-5 training"
        );
        assert_eq!(state.party.members[0].level, 6);
    }
}
