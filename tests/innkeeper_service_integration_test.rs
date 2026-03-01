// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration Tests: Innkeeper Service Flow (Phase 6)
//!
//! End-to-end tests covering the innkeeper rest service flow.  These tests
//! are deliberately scoped to the *service transaction* path (gold deducted,
//! HP/SP restored) and must not break the existing party-management behaviour
//! that is tested in `innkeeper_party_management_integration_test.rs`.
//!
//! # Test Coverage
//!
//! - `test_innkeeper_rest_service_heals_party` – party at partial HP/SP, gold deducted,
//!   all members fully restored
//! - `test_innkeeper_rest_insufficient_gold` – `InsufficientGold` when gold is 0
//! - `test_existing_inn_party_management_unaffected` – regression guard confirming
//!   the core inn workflow from `test_complete_inn_workflow` still passes after
//!   Phase 6 changes

use antares::application::dialogue::DialogueState;
use antares::application::save_game::SaveGameManager;
use antares::application::{GameMode, GameState, InnManagementState};
use antares::domain::character::{Alignment, Character, CharacterLocation, Sex};
use antares::domain::inventory::{ServiceCatalog, ServiceEntry};
use antares::domain::transactions::{consume_service, TransactionError};
use antares::domain::world::npc::NpcDefinition;
use antares::domain::world::npc_runtime::NpcRuntimeState;
use tempfile::TempDir;

// ============================================================================
// Shared Test Helpers
// ============================================================================

/// Build a `Character` with explicit `base_hp`, `current_hp`, `base_sp`, and
/// `current_sp` so that rest-service assertions have predictable targets.
fn character_at_partial_resources(
    name: &str,
    base_hp: u16,
    current_hp: u16,
    base_sp: u16,
    current_sp: u16,
) -> Character {
    let mut c = Character::new(
        name.to_string(),
        "human".to_string(),
        "knight".to_string(),
        Sex::Male,
        Alignment::Good,
    );
    c.hp.base = base_hp;
    c.hp.current = current_hp;
    c.sp.base = base_sp;
    c.sp.current = current_sp;
    c
}

/// Build a `ServiceCatalog` containing a single `"rest"` service at the given
/// gold cost.
fn rest_catalog(cost: u32) -> ServiceCatalog {
    let mut catalog = ServiceCatalog::new();
    catalog
        .services
        .push(ServiceEntry::new("rest", cost, "Rest and recover fully"));
    catalog
}

/// Build an `NpcRuntimeState` for an innkeeper NPC (no stock, services only).
fn innkeeper_runtime(npc_id: &str) -> NpcRuntimeState {
    NpcRuntimeState::new(npc_id.to_string())
}

// ============================================================================
// 6.3  Innkeeper Rest Service – Heals Full Party
// ============================================================================

/// Party members are at partial HP and SP with sufficient gold.  After calling
/// `consume_service` with `"rest"` every member must have HP and SP restored
/// to their `base` values and gold must be deducted by exactly the service cost.
#[test]
fn test_innkeeper_rest_service_heals_party() {
    // ── Arrange ─────────────────────────────────────────────────────────────
    let service_cost: u32 = 30;

    let npc_def = NpcDefinition::innkeeper("innkeeper_hana", "Hana's Rest", "hana.png");
    let mut npc_runtime = innkeeper_runtime(&npc_def.id);
    let catalog = rest_catalog(service_cost);

    let mut game = GameState::new();
    game.party.gold = 150;

    // Two party members – both wounded and low on spell points
    let mut member_a = character_at_partial_resources("Warrior", 40, 12, 0, 0);
    let mut member_b = character_at_partial_resources("Mage", 20, 5, 30, 7);

    // Preconditions
    assert!(
        member_a.hp.current < member_a.hp.base,
        "precondition: member_a must be wounded"
    );
    assert!(
        member_b.hp.current < member_b.hp.base,
        "precondition: member_b must be wounded"
    );
    assert!(
        member_b.sp.current < member_b.sp.base,
        "precondition: member_b must have reduced SP"
    );

    // ── Act ──────────────────────────────────────────────────────────────────
    let outcome = consume_service(
        &mut game.party,
        &mut vec![&mut member_a, &mut member_b],
        &mut npc_runtime,
        &catalog,
        "rest",
    )
    .expect("consume_service(rest) must succeed when party has sufficient gold");

    // ── Assert: service outcome ───────────────────────────────────────────────
    assert_eq!(
        outcome.service_id, "rest",
        "outcome must record the correct service ID"
    );
    assert_eq!(
        outcome.gold_paid, service_cost,
        "outcome must record the correct gold cost"
    );
    assert_eq!(outcome.gems_paid, 0, "rest service must not cost gems");

    // ── Assert: HP and SP fully restored ─────────────────────────────────────
    assert_eq!(
        member_a.hp.current, member_a.hp.base,
        "member_a HP must be fully restored to base ({}) after rest",
        member_a.hp.base
    );
    assert_eq!(
        member_b.hp.current, member_b.hp.base,
        "member_b HP must be fully restored to base ({}) after rest",
        member_b.hp.base
    );
    assert_eq!(
        member_b.sp.current, member_b.sp.base,
        "member_b SP must be fully restored to base ({}) after rest",
        member_b.sp.base
    );

    // ── Assert: gold deducted ─────────────────────────────────────────────────
    assert_eq!(
        game.party.gold,
        150 - service_cost,
        "party gold must be reduced by the service cost ({} → {})",
        150,
        150 - service_cost
    );

    // ── Assert: service recorded in NPC runtime ───────────────────────────────
    assert!(
        npc_runtime.services_consumed.contains(&"rest".to_string()),
        "NPC runtime must record that rest was consumed"
    );
}

// ============================================================================
// 6.3  Innkeeper Rest – Insufficient Gold
// ============================================================================

/// Party gold is zero.  `consume_service` must return
/// `TransactionError::InsufficientGold`, no HP or SP must change, and gold
/// must remain at zero.
#[test]
fn test_innkeeper_rest_insufficient_gold() {
    // ── Arrange ─────────────────────────────────────────────────────────────
    let service_cost: u32 = 50;

    let npc_def = NpcDefinition::innkeeper("innkeeper_pete", "Pete's Place", "pete.png");
    let mut npc_runtime = innkeeper_runtime(&npc_def.id);
    let catalog = rest_catalog(service_cost);

    let mut game = GameState::new();
    game.party.gold = 0; // Cannot afford any service

    let mut member = character_at_partial_resources("BrokeSword", 30, 5, 10, 2);

    let hp_before = member.hp.current;
    let sp_before = member.sp.current;

    // ── Act ──────────────────────────────────────────────────────────────────
    let result = consume_service(
        &mut game.party,
        &mut vec![&mut member],
        &mut npc_runtime,
        &catalog,
        "rest",
    );

    // ── Assert ───────────────────────────────────────────────────────────────
    assert!(
        result.is_err(),
        "consume_service must fail when party has 0 gold"
    );
    assert!(
        matches!(
            result,
            Err(TransactionError::InsufficientGold { have: 0, need })
            if need == service_cost
        ),
        "error must be InsufficientGold {{ have: 0, need: {} }}, got: {:?}",
        service_cost,
        result
    );

    // Gold must remain at 0
    assert_eq!(
        game.party.gold, 0,
        "party gold must not change after a failed service call"
    );

    // HP and SP must be unchanged
    assert_eq!(
        member.hp.current, hp_before,
        "character HP must not change when service fails due to insufficient gold"
    );
    assert_eq!(
        member.sp.current, sp_before,
        "character SP must not change when service fails due to insufficient gold"
    );

    // No service must be recorded
    assert!(
        npc_runtime.services_consumed.is_empty(),
        "no service must be recorded in NPC runtime after a failed transaction"
    );
}

// ============================================================================
// 6.3  Regression Guard – Existing Inn Party Management Unaffected
// ============================================================================

/// Re-runs the core scenario from `test_complete_inn_workflow` in
/// `innkeeper_party_management_integration_test.rs` using the current
/// codebase.  Any regression introduced by Phase 6 changes will surface here.
///
/// Flow:
///  1. Create `GameState` with a 3-member party.
///  2. Enter `Dialogue` mode with an innkeeper as speaker.
///  3. Transition to `InnManagement` mode.
///  4. Remove one party member.
///  5. Return to `Exploration` mode.
///  6. Verify the change persisted, then save/load and confirm again.
#[test]
fn test_existing_inn_party_management_unaffected() {
    // ── Step 1: Create game state with 3-member party ─────────────────────────
    let mut game = GameState::new();

    for i in 0..3_usize {
        let member = Character::new(
            format!("Hero{}", i + 1),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        game.party
            .add_member(member)
            .expect("adding party member must succeed");
    }

    assert_eq!(
        game.party.members.len(),
        3,
        "precondition: party must have 3 members"
    );
    assert_eq!(
        game.roster.characters.len(),
        0,
        "precondition: roster must be empty"
    );

    // ── Step 2: Enter Dialogue mode with innkeeper as speaker ─────────────────
    let speaker_npc_id = Some("innkeeper_regression_001".to_string());
    game.mode = GameMode::Dialogue(DialogueState::start(
        999_u16,
        1_u16,
        None,
        speaker_npc_id.clone(),
    ));

    if let GameMode::Dialogue(ref dialogue_state) = game.mode {
        assert_eq!(
            dialogue_state.active_tree_id,
            Some(999_u16),
            "dialogue tree ID must be set correctly"
        );
        assert_eq!(
            dialogue_state.speaker_npc_id, speaker_npc_id,
            "speaker NPC ID must match the innkeeper"
        );
    } else {
        panic!("Expected GameMode::Dialogue after entering dialogue");
    }

    // ── Step 3: Transition to InnManagement mode ──────────────────────────────
    game.mode = GameMode::InnManagement(InnManagementState {
        current_inn_id: "innkeeper_regression_001".to_string(),
        selected_party_slot: None,
        selected_roster_slot: None,
    });

    if let GameMode::InnManagement(ref inn_state) = game.mode {
        assert_eq!(
            inn_state.current_inn_id, "innkeeper_regression_001",
            "InnManagement must reference the correct innkeeper"
        );
    } else {
        panic!("Expected GameMode::InnManagement after transition");
    }

    // ── Step 4: Remove a party member (simulate dismissing Hero2) ────────────
    let removed = game.party.remove_member(1);
    assert!(
        removed.is_some(),
        "removing member at index 1 must succeed when party has 3 members"
    );
    assert_eq!(
        game.party.members.len(),
        2,
        "party must have 2 members after removing one"
    );

    // ── Step 5: Return to exploration ────────────────────────────────────────
    game.return_to_exploration();
    assert_eq!(
        game.mode,
        GameMode::Exploration,
        "mode must be Exploration after return_to_exploration()"
    );

    // ── Step 6a: Verify change persisted in memory ───────────────────────────
    assert_eq!(
        game.party.members.len(),
        2,
        "party size must still be 2 after returning to exploration"
    );
    assert_eq!(
        game.party.members[0].name, "Hero1",
        "Hero1 must remain at index 0"
    );
    assert_eq!(
        game.party.members[1].name, "Hero3",
        "Hero3 must have shifted to index 1 after Hero2 was removed"
    );

    // ── Step 6b: Save and reload – confirm no regression ─────────────────────
    // Also add the removed character to the roster so we can confirm roster
    // integrity survives the round-trip.
    let dismissed = Character::new(
        "Hero2".to_string(),
        "human".to_string(),
        "knight".to_string(),
        Sex::Male,
        Alignment::Good,
    );
    game.roster
        .add_character(
            dismissed,
            CharacterLocation::AtInn("innkeeper_regression_001".to_string()),
        )
        .expect("adding Hero2 to roster must succeed");

    let temp_dir = TempDir::new().unwrap();
    let manager = SaveGameManager::new(temp_dir.path()).unwrap();
    manager
        .save("regression_inn_test", &game)
        .expect("save must succeed");

    let loaded = manager
        .load("regression_inn_test")
        .expect("load must succeed");

    assert_eq!(
        loaded.mode,
        GameMode::Exploration,
        "mode must still be Exploration after save/load"
    );
    assert_eq!(
        loaded.party.members.len(),
        2,
        "party must still have 2 members after save/load"
    );
    assert_eq!(
        loaded.party.members[0].name, "Hero1",
        "Hero1 must be at index 0 after save/load"
    );
    assert_eq!(
        loaded.party.members[1].name, "Hero3",
        "Hero3 must be at index 1 after save/load"
    );
    assert_eq!(
        loaded.roster.characters.len(),
        1,
        "roster must contain exactly 1 character (Hero2) after save/load"
    );
    assert_eq!(
        loaded.roster.characters[0].name, "Hero2",
        "Hero2 must be in the roster after save/load"
    );
}
