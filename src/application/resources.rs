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
use crate::domain::validation::ValidationError;
use crate::sdk::database::ContentDatabase;
use bevy::prelude::*;

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
}
