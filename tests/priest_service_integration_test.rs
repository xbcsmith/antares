// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration Tests: Priest Service Flow (Phase 6)
//!
//! End-to-end tests covering the complete priest service flow from party state
//! construction through service consumption through save/load round-trip
//! verification.
//!
//! # Test Coverage
//!
//! - `test_priest_heal_all_flow` – character at partial HP, gold deducted, HP restored
//! - `test_priest_resurrect_flow` – dead character resurrected, HP set to 1, gold deducted
//! - `test_priest_service_insufficient_gold` – `InsufficientGold` error, no state change
//! - `test_priest_service_save_load_preserves_state` – post-service state survives round-trip

use antares::application::save_game::SaveGameManager;
use antares::application::GameState;
use antares::domain::character::{Alignment, Character, Condition, Sex};
use antares::domain::inventory::{ServiceCatalog, ServiceEntry};
use antares::domain::transactions::{consume_service, TransactionError};
use antares::domain::world::npc::NpcDefinition;
use antares::domain::world::npc_runtime::NpcRuntimeState;
use tempfile::TempDir;

// ============================================================================
// Shared Test Helpers
// ============================================================================

/// Build a `Character` with specified `base_hp` and `current_hp`.
///
/// The `base_hp` represents the fully-healed maximum; `current_hp` represents
/// the character's health at the time of the test.
fn character_with_hp(base_hp: u16, current_hp: u16) -> Character {
    let mut c = Character::new(
        "TestPriest".to_string(),
        "human".to_string(),
        "cleric".to_string(),
        Sex::Female,
        Alignment::Good,
    );
    c.hp.base = base_hp;
    c.hp.current = current_hp;
    c
}

/// Build a `ServiceCatalog` containing a single heal-all service at the given
/// gold cost.
fn catalog_with_service(service_id: &str, cost: u32, description: &str) -> ServiceCatalog {
    let mut catalog = ServiceCatalog::new();
    catalog
        .services
        .push(ServiceEntry::new(service_id, cost, description));
    catalog
}

/// Build a standard `NpcRuntimeState` for a priest NPC.
fn priest_runtime(npc_id: &str) -> NpcRuntimeState {
    NpcRuntimeState::new(npc_id.to_string())
}

// ============================================================================
// 6.2  Priest Heal-All Flow – End-to-End
// ============================================================================

/// Character at partial HP, party has sufficient gold.  After calling
/// `consume_service` with `"heal_all"` the character's HP must be fully
/// restored and the service cost must be deducted from party gold.
#[test]
fn test_priest_heal_all_flow() {
    // ── Arrange ─────────────────────────────────────────────────────────────
    let service_cost: u32 = 50;
    let base_hp: u16 = 30;
    let wounded_hp: u16 = 8;

    let npc_def = NpcDefinition::priest("high_priest_sol", "High Priest Sol", "sol.png");
    let mut npc_runtime = priest_runtime(&npc_def.id);
    let catalog = catalog_with_service("heal_all", service_cost, "Fully restore HP");

    let mut game = GameState::new();
    game.party.gold = 100;

    let mut character = character_with_hp(base_hp, wounded_hp);

    assert_eq!(
        character.hp.current, wounded_hp,
        "precondition: character must start with reduced HP"
    );
    assert!(
        character.hp.current < character.hp.base,
        "precondition: current HP must be below base HP"
    );

    // ── Act ──────────────────────────────────────────────────────────────────
    let outcome = consume_service(
        &mut game.party,
        &mut vec![&mut character],
        &mut npc_runtime,
        &catalog,
        "heal_all",
    )
    .expect("consume_service must succeed when party has sufficient gold");

    // ── Assert ───────────────────────────────────────────────────────────────
    assert_eq!(
        outcome.gold_paid, service_cost,
        "outcome must record the correct gold cost"
    );
    assert_eq!(outcome.gems_paid, 0, "heal_all should not cost gems");
    assert_eq!(
        outcome.service_id, "heal_all",
        "outcome must record the correct service ID"
    );

    assert_eq!(
        character.hp.current, base_hp,
        "character HP must be fully restored to base ({}) after heal_all",
        base_hp
    );

    assert_eq!(
        game.party.gold,
        100 - service_cost,
        "party gold must be reduced by the service cost ({} → {})",
        100,
        100 - service_cost
    );

    // The service must be recorded in the NPC runtime
    assert!(
        npc_runtime
            .services_consumed
            .contains(&"heal_all".to_string()),
        "NPC runtime must record that heal_all was consumed"
    );
}

// ============================================================================
// 6.2  Priest Resurrect Flow – End-to-End
// ============================================================================

/// Character is dead (`Condition::DEAD`).  After calling `consume_service`
/// with `"resurrect"` all conditions must be cleared, HP must be exactly 1,
/// and gold must be deducted.
#[test]
fn test_priest_resurrect_flow() {
    // ── Arrange ─────────────────────────────────────────────────────────────
    let service_cost: u32 = 200;
    let base_hp: u16 = 25;

    let npc_def = NpcDefinition::priest("priest_luna", "Priest Luna", "luna.png");
    let mut npc_runtime = priest_runtime(&npc_def.id);
    let catalog = catalog_with_service("resurrect", service_cost, "Bring the dead back to life");

    let mut game = GameState::new();
    game.party.gold = 500;

    // Create a dead character: HP is 0, DEAD condition is set
    let mut dead_character = character_with_hp(base_hp, 0);
    dead_character.conditions.add(Condition::DEAD);

    assert!(
        dead_character.conditions.has(Condition::DEAD),
        "precondition: character must have the DEAD condition"
    );
    assert_eq!(
        dead_character.hp.current, 0,
        "precondition: dead character must have 0 current HP"
    );

    // ── Act ──────────────────────────────────────────────────────────────────
    let outcome = consume_service(
        &mut game.party,
        &mut vec![&mut dead_character],
        &mut npc_runtime,
        &catalog,
        "resurrect",
    )
    .expect("resurrect must succeed when party has sufficient gold");

    // ── Assert ───────────────────────────────────────────────────────────────
    assert_eq!(
        outcome.gold_paid, service_cost,
        "outcome must record the correct resurrection cost"
    );
    assert_eq!(
        outcome.service_id, "resurrect",
        "outcome must record the correct service ID"
    );

    assert!(
        dead_character.conditions.is_fine(),
        "all conditions must be cleared after resurrection (was DEAD)"
    );
    assert!(
        !dead_character.conditions.has(Condition::DEAD),
        "DEAD condition must be removed after resurrection"
    );
    assert_eq!(
        dead_character.hp.current, 1,
        "resurrected character must have exactly 1 HP"
    );

    assert_eq!(
        game.party.gold,
        500 - service_cost,
        "party gold must be reduced by the resurrection cost ({} → {})",
        500,
        500 - service_cost
    );

    assert!(
        npc_runtime
            .services_consumed
            .contains(&"resurrect".to_string()),
        "NPC runtime must record that resurrect was consumed"
    );
}

// ============================================================================
// 6.2  Insufficient Gold – Error Path
// ============================================================================

/// Party gold is less than the service cost.  The call must return
/// `TransactionError::InsufficientGold`, character HP must remain unchanged,
/// and party gold must remain unchanged.
#[test]
fn test_priest_service_insufficient_gold() {
    // ── Arrange ─────────────────────────────────────────────────────────────
    let service_cost: u32 = 100;
    let party_gold: u32 = 30; // Insufficient

    let npc_def = NpcDefinition::priest("friar_mike", "Friar Mike", "mike.png");
    let mut npc_runtime = priest_runtime(&npc_def.id);
    let catalog = catalog_with_service("heal_all", service_cost, "Full heal");

    let mut game = GameState::new();
    game.party.gold = party_gold;

    let base_hp: u16 = 40;
    let wounded_hp: u16 = 15;
    let mut character = character_with_hp(base_hp, wounded_hp);

    // ── Act ──────────────────────────────────────────────────────────────────
    let result = consume_service(
        &mut game.party,
        &mut vec![&mut character],
        &mut npc_runtime,
        &catalog,
        "heal_all",
    );

    // ── Assert ───────────────────────────────────────────────────────────────
    assert!(
        result.is_err(),
        "consume_service must fail when party cannot afford the service"
    );
    assert!(
        matches!(
            result,
            Err(TransactionError::InsufficientGold { have, need })
            if have == party_gold && need == service_cost
        ),
        "error must be InsufficientGold with correct have/need values, got: {:?}",
        result
    );

    // Gold must be unchanged
    assert_eq!(
        game.party.gold, party_gold,
        "party gold must not change when the service call fails"
    );

    // HP must be unchanged
    assert_eq!(
        character.hp.current, wounded_hp,
        "character HP must not change when the service call fails due to insufficient gold"
    );

    // No service must be recorded in NPC runtime
    assert!(
        npc_runtime.services_consumed.is_empty(),
        "no service must be recorded in NPC runtime after a failed transaction"
    );
}

// ============================================================================
// 6.2  Save/Load Preserves Post-Service State
// ============================================================================

/// After a priest service is consumed, save the game and reload it.  Verify
/// that the party state (gold, HP) and NPC runtime (services_consumed) are
/// correctly restored from disk.
#[test]
fn test_priest_service_save_load_preserves_state() {
    // ── Arrange ─────────────────────────────────────────────────────────────
    let service_cost: u32 = 75;
    let base_hp: u16 = 50;
    let wounded_hp: u16 = 10;
    let starting_gold: u32 = 200;

    let npc_def = NpcDefinition::priest("elder_priest", "Elder Priest", "elder.png");
    let mut npc_runtime = priest_runtime(&npc_def.id);
    let catalog = catalog_with_service("heal_all", service_cost, "Complete restoration");

    let mut game = GameState::new();
    game.party.gold = starting_gold;

    let mut character = character_with_hp(base_hp, wounded_hp);

    // ── Act: consume the service ──────────────────────────────────────────────
    consume_service(
        &mut game.party,
        &mut vec![&mut character],
        &mut npc_runtime,
        &catalog,
        "heal_all",
    )
    .expect("service consumption must succeed");

    let expected_gold = starting_gold - service_cost;

    // Sanity-check pre-save state
    assert_eq!(
        game.party.gold, expected_gold,
        "gold must be deducted before save"
    );
    assert_eq!(
        character.hp.current, base_hp,
        "HP must be restored before save"
    );

    // ── Act: persist everything and round-trip through save/load ─────────────
    game.npc_runtime.insert(npc_runtime);
    game.party
        .add_member(character)
        .expect("party must accept the character");

    let temp_dir = TempDir::new().unwrap();
    let manager = SaveGameManager::new(temp_dir.path()).unwrap();
    manager.save("priest_service_test", &game).unwrap();

    let loaded = manager.load("priest_service_test").unwrap();

    // ── Assert: post-load state matches post-service state ───────────────────
    assert_eq!(
        loaded.party.gold, expected_gold,
        "party gold ({}) must survive the save/load round-trip",
        expected_gold
    );
    assert_eq!(
        loaded.party.members.len(),
        1,
        "party must still contain one member after load"
    );
    assert_eq!(
        loaded.party.members[0].hp.current, base_hp,
        "fully-healed HP ({}) must survive the save/load round-trip",
        base_hp
    );
    assert_eq!(
        loaded.party.members[0].hp.base, base_hp,
        "HP base value must be unchanged after save/load"
    );

    // NPC runtime consumed services must persist
    let loaded_runtime = loaded
        .npc_runtime
        .get(&"elder_priest".to_string())
        .expect("elder_priest NPC runtime must be present after load");

    assert!(
        loaded_runtime
            .services_consumed
            .contains(&"heal_all".to_string()),
        "services_consumed must contain 'heal_all' after save/load round-trip"
    );
    assert_eq!(
        loaded_runtime.services_consumed.len(),
        1,
        "services_consumed must have exactly one entry after save/load"
    );
}
