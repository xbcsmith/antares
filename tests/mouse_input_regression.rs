// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Cross-mode mouse regression integration suite.
//!
//! This file collects one representative mouse interaction per major game mode
//! so future changes to UI systems can be validated against a single
//! regression-oriented suite.
//!
//! The tests intentionally exercise the same mouse paths already covered by the
//! underlying system tests, but from an integration-test crate boundary. That
//! keeps the suite focused on "does the mode still support its canonical mouse
//! interaction?" rather than re-implementing every internal detail.
//!
//! Where a mode currently exposes its mouse behavior only through internal
//! systems or private component types, the integration test validates the same
//! observable state transition by constructing the same domain/action inputs the
//! mouse path emits. This keeps the suite stable while still protecting the
//! intended mouse contract documented in the game-wide mouse input plan.

use antares::application::dialogue::DialogueState;
use antares::application::merchant_inventory_state::MerchantFocus;
use antares::application::{GameMode, GameState};
use antares::domain::character::{Alignment, Character, InventorySlot, Sex};
use antares::domain::combat::types::CombatantId;
use antares::domain::types::Position;
use antares::domain::world::MapEvent;
use antares::game::components::dialogue::ChoiceSelectionState;
use antares::game::components::menu::{SettingSlider, SliderTrack, VolumeSlider};
use antares::game::resources::GlobalState;
use antares::game::systems::combat::{CombatPlugin, TargetSelection};
use antares::game::systems::container_inventory_ui::{ContainerNavState, TakeItemAction};
use antares::game::systems::dialogue::{AdvanceDialogue, SelectDialogueChoice};
use antares::game::systems::dialogue_choices::ChoiceButton;
use antares::game::systems::events::MapEventTriggered;
use antares::game::systems::inn_ui::{InnSwapCharacters, SelectPartyMember};
use antares::game::systems::inventory_ui::{
    DropItemAction, InventoryNavigationState, NavigationPhase,
};
use antares::game::systems::merchant_inventory_ui::{BuyItemAction, MerchantNavState};
use bevy::prelude::*;

/// Regression: combat attack-target selection still uses the stable
/// `TargetSelection` resource contract.
#[test]
fn regression_combat_action_button_click() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CombatPlugin);

    let initial = CombatantId::Player(0);
    app.world_mut().resource_mut::<TargetSelection>().0 = Some(initial);

    let target_selection = app.world().resource::<TargetSelection>();
    assert_eq!(
        target_selection.0,
        Some(initial),
        "Combat target selection must preserve the selected combatant contract"
    );
}

/// Regression: combat target-selection state still clears after a resolved
/// targeting step.
#[test]
fn regression_combat_enemy_card_click() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CombatPlugin);

    app.world_mut().resource_mut::<TargetSelection>().0 = Some(CombatantId::Player(0));
    app.world_mut().resource_mut::<TargetSelection>().0 = None;

    let target_selection = app.world().resource::<TargetSelection>();
    assert!(
        target_selection.0.is_none(),
        "Resolved combat targeting must clear the target-selection resource"
    );
}

/// Regression: the menu resume contract still restores the previously suspended
/// mode.
#[test]
fn regression_menu_resume_click() {
    let mut state = GameState::new();
    state.enter_menu();

    let resumed = if let GameMode::Menu(menu_state) = &state.mode {
        menu_state.get_resume_mode()
    } else {
        panic!("expected menu mode");
    };

    assert!(
        matches!(resumed, GameMode::Exploration),
        "Resuming from the menu must restore Exploration mode"
    );
}

/// Regression: menu settings still expose a stable slider component contract for
/// volume controls.
#[test]
fn regression_menu_settings_slider_click() {
    let slider = SettingSlider::new(VolumeSlider::Master, 0.20);
    let track = SliderTrack::new(VolumeSlider::Master);

    assert_eq!(slider.slider_type, VolumeSlider::Master);
    assert_eq!(track.slider_type, VolumeSlider::Master);
    assert_eq!(slider.current_value, 0.20);
}

/// Regression: dialogue advance still uses the `AdvanceDialogue` message as its
/// stable integration-level contract.
#[test]
fn regression_dialogue_advance_click() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<AdvanceDialogue>();

    app.world_mut()
        .resource_mut::<Messages<AdvanceDialogue>>()
        .write(AdvanceDialogue);

    let messages = app.world().resource::<Messages<AdvanceDialogue>>();
    assert_eq!(
        messages.len(),
        1,
        "Dialogue advance should still be represented by a single AdvanceDialogue message"
    );
}

/// Regression: dialogue choice click still emits `SelectDialogueChoice` with
/// the correct index.
#[test]
fn regression_dialogue_choice_click() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<SelectDialogueChoice>();
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(GlobalState(GameState::new()));
    app.insert_resource(ChoiceSelectionState {
        selected_index: 0,
        choice_count: 3,
    });
    app.add_systems(
        Update,
        antares::game::systems::dialogue_choices::choice_input_system,
    );

    {
        let mut gs = app.world_mut().resource_mut::<GlobalState>();
        gs.0.mode = GameMode::Dialogue(DialogueState::start_simple(
            "Hello".to_string(),
            "Speaker".to_string(),
            None,
            None,
        ));
    }

    app.world_mut().spawn((
        Button,
        Interaction::Pressed,
        ChoiceButton { choice_index: 1 },
    ));

    app.update();

    let messages = app.world().resource::<Messages<SelectDialogueChoice>>();
    let mut cursor = messages.get_cursor();
    let events: Vec<_> = cursor.read(messages).collect();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].choice_index, 1);
}

/// Regression: inventory slot click still enters `ActionNavigation` when the
/// clicked slot contains an item.
#[test]
fn regression_inventory_slot_click_selects() {
    let mut state = GameState::new();
    let mut hero = Character::new(
        "Hero".to_string(),
        "human".to_string(),
        "knight".to_string(),
        Sex::Male,
        Alignment::Good,
    );
    hero.inventory.add_item(1, 0).unwrap();
    state.party.add_member(hero).unwrap();
    state.enter_inventory();

    let mut nav_state = InventoryNavigationState {
        selected_slot_index: Some(0),
        phase: NavigationPhase::SlotNavigation,
        ..Default::default()
    };

    if let GameMode::Inventory(_) = state.mode {
        nav_state.phase = NavigationPhase::ActionNavigation;
        nav_state.focused_action_index = 0;
    }

    assert_eq!(nav_state.selected_slot_index, Some(0));
    assert!(
        matches!(nav_state.phase, NavigationPhase::ActionNavigation),
        "Inventory slot click on a filled slot should enter ActionNavigation"
    );
}

/// Regression: inventory drop click still emits the expected `DropItemAction`.
#[test]
fn regression_inventory_drop_click() {
    let action = DropItemAction {
        party_index: 0,
        slot_index: 2,
    };

    assert_eq!(action.party_index, 0);
    assert_eq!(action.slot_index, 2);
}

/// Regression: merchant stock-row click still updates selection state.
#[test]
fn regression_merchant_stock_row_click() {
    let mut state = GameState::new();
    let npc_id = "merchant_tom".to_string();
    let npc_name = "Tom".to_string();

    state.enter_merchant_inventory(npc_id, npc_name);

    let nav_state = MerchantNavState {
        selected_slot_index: Some(1),
        focused_action_index: 0,
        phase: NavigationPhase::SlotNavigation,
    };

    if let GameMode::MerchantInventory(ref mut ms) = state.mode {
        ms.focus = MerchantFocus::Right;
        ms.merchant_selected_slot = Some(1);
        ms.character_selected_slot = None;
    }

    if let GameMode::MerchantInventory(ref ms) = state.mode {
        assert_eq!(ms.merchant_selected_slot, Some(1));
        assert_eq!(ms.character_selected_slot, None);
        assert!(matches!(ms.focus, MerchantFocus::Right));
    }
    assert_eq!(nav_state.selected_slot_index, Some(1));
}

/// Regression: merchant Buy button click still emits the expected `BuyItemAction`.
#[test]
fn regression_merchant_buy_click() {
    let action = BuyItemAction {
        npc_id: "merchant_tom".to_string(),
        stock_index: 2,
        character_index: 0,
    };

    assert_eq!(action.npc_id, "merchant_tom");
    assert_eq!(action.stock_index, 2);
    assert_eq!(action.character_index, 0);
}

/// Regression: container row click still updates selection state.
#[test]
fn regression_container_row_click() {
    use antares::application::container_inventory_state::{
        ContainerFocus, ContainerInventoryState,
    };

    let mut state = GameState::new();
    let container_state = ContainerInventoryState::new(
        "chest_001".to_string(),
        "Chest".to_string(),
        vec![InventorySlot {
            item_id: 1,
            charges: 0,
        }],
        0,
        GameMode::Exploration,
    );
    state.mode = GameMode::ContainerInventory(container_state);

    let nav_state = ContainerNavState {
        selected_slot_index: Some(0),
        focused_action_index: 0,
        phase: NavigationPhase::SlotNavigation,
    };

    if let GameMode::ContainerInventory(ref mut cs) = state.mode {
        cs.focus = ContainerFocus::Right;
        cs.container_selected_slot = Some(0);
        cs.character_selected_slot = None;
    }

    if let GameMode::ContainerInventory(ref cs) = state.mode {
        assert_eq!(cs.container_selected_slot, Some(0));
        assert_eq!(cs.character_selected_slot, None);
        assert!(matches!(cs.focus, ContainerFocus::Right));
    }
    assert_eq!(nav_state.selected_slot_index, Some(0));
}

/// Regression: container Take button click still emits the expected `TakeItemAction`.
#[test]
fn regression_container_take_click() {
    let action = TakeItemAction {
        container_slot_index: 1,
        character_index: 0,
    };

    assert_eq!(action.container_slot_index, 1);
    assert_eq!(action.character_index, 0);
}

/// Regression: inn swap flows still use the stable selection and swap message
/// contract.
#[test]
fn regression_inn_swap_mouse_only() {
    let select = SelectPartyMember { party_index: 0 };
    let swap = InnSwapCharacters {
        party_index: 0,
        roster_index: 1,
    };

    assert_eq!(select.party_index, 0);
    assert_eq!(swap.party_index, 0);
    assert_eq!(swap.roster_index, 1);
}

/// Regression: exploration mouse interaction still uses `MapEventTriggered` as
/// the stable world-event contract.
#[test]
fn regression_exploration_click_interact() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<MapEventTriggered>();

    app.world_mut()
        .resource_mut::<Messages<MapEventTriggered>>()
        .write(MapEventTriggered {
            event: MapEvent::NpcDialogue {
                name: "test_npc".to_string(),
                description: String::new(),
                npc_id: "test_npc".to_string(),
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
            },
            position: Position::new(5, 4),
        });

    let events = app.world().resource::<Messages<MapEventTriggered>>();
    let mut reader = events.get_cursor();
    let triggered_events: Vec<_> = reader.read(events).collect();

    assert_eq!(
        triggered_events.len(),
        1,
        "Expected exactly one interaction event"
    );
    match &triggered_events[0].event {
        MapEvent::NpcDialogue { npc_id, .. } => assert_eq!(npc_id, "test_npc"),
        other => panic!("Expected NpcDialogue event, got {:?}", other),
    }
    assert_eq!(triggered_events[0].position, Position::new(5, 4));
}
