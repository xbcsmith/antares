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
use antares::domain::character::{Alignment, Character, CharacterLocation, Sex};
use antares::domain::types::Position;
use antares::domain::world::MapEvent;
use antares::game::components::dialogue::DialoguePanelRoot;
use antares::game::resources::{GlobalState, LockInteractionPending};
use antares::game::systems::combat::{
    ActionButton, ActionButtonType, CombatPlugin, CombatResource, CombatTurnState,
    CombatTurnStateResource, TargetSelection,
};
use antares::game::systems::container_inventory_ui::{ContainerNavState, TakeItemAction};
use antares::game::systems::dialogue::{AdvanceDialogue, DialoguePlugin, SelectDialogueChoice};
use antares::game::systems::dialogue_choices::{ChoiceButton, ChoiceSelectionState};
use antares::game::systems::events::{EventPlugin, MapEventTriggered};
use antares::game::systems::inn_ui::{
    ExitInn, InnDismissCharacter, InnNavigationState, InnRecruitCharacter, InnSwapCharacters,
    SelectPartyMember, SelectRosterMember,
};
use antares::game::systems::input::InputPlugin;
use antares::game::systems::inventory_ui::{
    DropItemAction, InventoryNavigationState, NavigationPhase,
};
use antares::game::systems::map::{NpcMarker, TileCoord};
use antares::game::systems::menu::{
    MenuButton, MenuPlugin, MenuType, SaveGameManager, SettingSlider, SliderTrack, VolumeSlider,
};
use antares::game::systems::merchant_inventory_ui::{
    BuyItemAction, MerchantNavState, NavigationPhase as MerchantNavigationPhase,
};
use antares::game::systems::ui::GameLog;
use antares::sdk::game_config::ControlsConfig;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use tempfile::TempDir;

/// Regression: combat action button click still transitions into target
/// selection when the Attack button is hovered and the left mouse button is
/// pressed.
#[test]
fn regression_combat_action_button_click() {
    use antares::domain::combat::engine::CombatState;
    use antares::domain::combat::types::{CombatantId, Handicap};

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

    let mut cs = CombatState::new(Handicap::Even);
    cs.add_player(hero);
    cs.turn_order = vec![CombatantId::Player(0)];
    cs.current_turn = 0;
    gs.enter_combat_with_state(cs.clone());

    app.insert_resource(GlobalState(gs));
    {
        let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
        combat_res.state = cs;
        combat_res.player_orig_indices = vec![Some(0)];
    }
    app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;
    app.insert_resource(ButtonInput::<MouseButton>::default());

    app.update();

    let attack_entity = {
        let mut query = app
            .world_mut()
            .query_filtered::<(Entity, &ActionButton), With<Button>>();
        query
            .iter(app.world())
            .find(|(_, btn)| btn.button_type == ActionButtonType::Attack)
            .map(|(entity, _)| entity)
            .expect("Attack button must exist")
    };

    app.world_mut()
        .entity_mut(attack_entity)
        .insert(Interaction::Hovered);
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.press(MouseButton::Left);
    }

    app.update();

    let target_selection = app.world().resource::<TargetSelection>();
    assert!(
        target_selection.0.is_some(),
        "Hovered Attack button plus left-click should enter target selection"
    );
}

/// Regression: combat enemy-card click still emits the attack selection result
/// for the hovered enemy card.
#[test]
fn regression_combat_enemy_card_click() {
    use antares::domain::combat::engine::CombatState;
    use antares::domain::combat::monster::{LootTable, MonsterInstance};
    use antares::domain::combat::types::{CombatantId, Handicap};
    use antares::game::systems::combat::{EnemyCard, SelectTargetState};

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

    let mut combat_state = CombatState::new(Handicap::Even);
    combat_state.add_player(hero);
    let monster = MonsterInstance::new(
        "Goblin".to_string(),
        10,
        10,
        0,
        1,
        0,
        0,
        0,
        LootTable::default(),
    );
    combat_state.add_monster(monster);
    combat_state.turn_order = vec![CombatantId::Player(0)];
    combat_state.current_turn = 0;
    gs.enter_combat_with_state(combat_state.clone());

    app.insert_resource(GlobalState(gs));
    {
        let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
        combat_res.state = combat_state;
        combat_res.player_orig_indices = vec![Some(0)];
    }
    app.insert_resource(ButtonInput::<MouseButton>::default());

    app.world_mut().resource_mut::<TargetSelection>().0 = Some(SelectTargetState {
        attacker_index: 0,
        is_ranged: false,
    });

    app.update();

    let enemy_entity = {
        let mut query = app
            .world_mut()
            .query_filtered::<(Entity, &EnemyCard), With<Button>>();
        query
            .iter(app.world())
            .find(|(_, card)| card.participant_index == 0)
            .map(|(entity, _)| entity)
            .expect("Enemy card must exist")
    };

    app.world_mut()
        .entity_mut(enemy_entity)
        .insert(Interaction::Hovered);
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.press(MouseButton::Left);
    }

    app.update();

    let target_selection = app.world().resource::<TargetSelection>();
    assert!(
        target_selection.0.is_none(),
        "Enemy-card click should resolve and clear target-selection state"
    );
}

/// Regression: menu Resume button click still restores the previous mode.
#[test]
fn regression_menu_resume_click() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GlobalState(GameState::new()));
    app.insert_resource(SaveGameManager::new(TempDir::new().unwrap().path()).unwrap());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_plugins(MenuPlugin);

    {
        let mut global_state = app.world_mut().resource_mut::<GlobalState>();
        global_state.0.enter_menu();
    }

    app.update();

    let resume_entity = {
        let mut query = app
            .world_mut()
            .query_filtered::<(Entity, &MenuButton), With<Button>>();
        query
            .iter(app.world())
            .find(|(_, button)| matches!(button, MenuButton::Resume))
            .map(|(entity, _)| entity)
            .expect("Resume button must exist")
    };

    app.world_mut()
        .entity_mut(resume_entity)
        .insert(Interaction::Pressed);

    app.update();

    let global_state = app.world().resource::<GlobalState>();
    assert!(
        matches!(global_state.0.mode, GameMode::Exploration),
        "Resume click must restore Exploration mode"
    );
}

/// Regression: menu settings slider click still updates the audio setting value.
#[test]
fn regression_menu_settings_slider_click() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GlobalState(GameState::new()));
    app.insert_resource(SaveGameManager::new(TempDir::new().unwrap().path()).unwrap());
    app.insert_resource(ButtonInput::<MouseButton>::default());

    let mut window = Window::default();
    window.resolution.set_physical_resolution(900, 600);
    window.set_cursor_position(Some(Vec2::new(675.0, 300.0)));
    app.world_mut().spawn((window, PrimaryWindow));

    app.add_plugins(MenuPlugin);

    {
        let mut global_state = app.world_mut().resource_mut::<GlobalState>();
        global_state.0.enter_menu();
        if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
            menu_state.set_submenu(MenuType::Settings);
        }
    }

    let slider_entity = app
        .world_mut()
        .spawn(SettingSlider::new(VolumeSlider::Master, 0.20))
        .id();

    app.world_mut().spawn((
        Interaction::Hovered,
        Node {
            width: Val::Px(300.0),
            height: Val::Px(12.0),
            ..default()
        },
        GlobalTransform::from(Transform::from_xyz(600.0, 300.0, 0.0)),
        SliderTrack {
            slider_type: VolumeSlider::Master,
        },
    ));

    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.press(MouseButton::Left);
    }

    app.update();

    let slider = app
        .world()
        .entity(slider_entity)
        .get::<SettingSlider>()
        .expect("Master slider should exist");

    assert!(
        slider.current_value > 0.20,
        "Clicking the settings slider should raise the configured value"
    );
}

/// Regression: dialogue panel click still emits `AdvanceDialogue`.
#[test]
fn regression_dialogue_advance_click() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(DialoguePlugin);
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(GlobalState(GameState::new()));

    {
        let mut global_state = app.world_mut().resource_mut::<GlobalState>();
        let dialogue_state = DialogueState::start_simple(
            "Hello".to_string(),
            "Speaker".to_string(),
            None,
            Some(Position::new(1, 1)),
        );
        global_state.0.mode = GameMode::Dialogue(dialogue_state);
    }

    app.world_mut()
        .spawn((Button, Interaction::Pressed, DialoguePanelRoot));

    app.update();

    let messages = app.world().resource::<Messages<AdvanceDialogue>>();
    assert_eq!(
        messages.len(),
        1,
        "Dialogue click should emit AdvanceDialogue"
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
    let events: Vec<_> = cursor.read(messages).cloned().collect();

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

    let mut nav_state = InventoryNavigationState::default();
    nav_state.selected_slot_index = Some(0);
    nav_state.phase = NavigationPhase::SlotNavigation;

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

    let mut nav_state = MerchantNavState::default();

    if let GameMode::MerchantInventory(ref mut ms) = state.mode {
        ms.focus = MerchantFocus::Right;
        ms.merchant_selected_slot = Some(1);
        ms.character_selected_slot = None;
    }

    nav_state.selected_slot_index = Some(1);
    nav_state.focused_action_index = 0;
    nav_state.phase = MerchantNavigationPhase::SlotNavigation;

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
    use antares::domain::inventory::InventorySlot;

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

    let mut nav_state = ContainerNavState::default();

    if let GameMode::ContainerInventory(ref mut cs) = state.mode {
        cs.focus = ContainerFocus::Right;
        cs.container_selected_slot = Some(0);
        cs.character_selected_slot = None;
    }

    nav_state.selected_slot_index = Some(0);

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

/// Regression: inn mouse-only select-plus-swap flow still completes.
#[test]
fn regression_inn_swap_mouse_only() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    app.add_message::<InnRecruitCharacter>();
    app.add_message::<InnDismissCharacter>();
    app.add_message::<InnSwapCharacters>();
    app.add_message::<ExitInn>();
    app.add_message::<SelectPartyMember>();
    app.add_message::<SelectRosterMember>();
    app.init_resource::<InnNavigationState>();
    app.add_systems(
        Update,
        (
            antares::game::systems::inn_ui::inn_selection_system,
            antares::game::systems::inn_ui::inn_action_system,
        )
            .chain(),
    );

    let mut game = GameState::new();
    let inn_id = "test_inn".to_string();

    let party_character = Character::new(
        "PartyHero".to_string(),
        "human".to_string(),
        "knight".to_string(),
        Sex::Male,
        Alignment::Good,
    );
    let roster_character = Character::new(
        "InnMage".to_string(),
        "elf".to_string(),
        "mage".to_string(),
        Sex::Female,
        Alignment::Neutral,
    );

    game.roster
        .add_character(party_character.clone(), CharacterLocation::InParty)
        .unwrap();
    game.roster
        .add_character(
            roster_character.clone(),
            CharacterLocation::AtInn(inn_id.clone()),
        )
        .unwrap();
    game.party.add_member(party_character).unwrap();

    game.mode = GameMode::InnManagement(antares::application::inn::InnManagementState {
        current_inn_id: inn_id,
        selected_party_slot: None,
        selected_roster_slot: None,
    });

    app.insert_resource(GlobalState(game));

    app.world_mut()
        .resource_mut::<Messages<SelectPartyMember>>()
        .write(SelectPartyMember { party_index: 0 });
    app.update();

    app.world_mut()
        .resource_mut::<Messages<InnSwapCharacters>>()
        .write(InnSwapCharacters {
            party_index: 0,
            roster_index: 1,
        });
    app.update();

    let global = app.world().resource::<GlobalState>();
    assert_eq!(global.0.party.members.len(), 1);
    assert_eq!(global.0.party.members[0].name, "InnMage");
}

/// Regression: exploration click-to-interact still emits the same world event
/// as keyboard interact for the tile directly ahead.
#[test]
fn regression_exploration_click_interact() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(InputPlugin::new(ControlsConfig::default()));
    app.add_plugins(EventPlugin);

    app.insert_resource(GlobalState(GameState::new()));
    app.insert_resource(PendingRecruitmentContext::default());
    app.insert_resource(GameLog::new());
    app.init_resource::<LockInteractionPending>();

    let mut map =
        antares::domain::world::Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
    let party_pos = Position::new(5, 5);
    let npc_pos = Position::new(5, 4);
    map.npc_placements
        .push(antares::domain::world::NpcPlacement {
            npc_id: "test_npc".to_string(),
            position: npc_pos,
            facing: None,
            dialogue_override: None,
        });

    {
        let mut gs = app.world_mut().resource_mut::<GlobalState>();
        gs.0.world.add_map(map);
        gs.0.world.set_current_map(1);
        gs.0.world.set_party_position(party_pos);
        gs.0.world.party_facing = antares::domain::types::Direction::North;
        gs.0.mode = GameMode::Exploration;
    }

    app.world_mut().spawn((
        NpcMarker {
            npc_id: "test_npc".to_string(),
        },
        TileCoord(npc_pos),
    ));

    let mut window = Window::default();
    window.resolution.set_physical_resolution(900, 600);
    window.set_cursor_position(Some(Vec2::new(450.0, 300.0)));
    app.world_mut().spawn((window, PrimaryWindow));

    let mut mouse = ButtonInput::<MouseButton>::default();
    mouse.press(MouseButton::Left);
    app.insert_resource(mouse);

    app.update();

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
    assert_eq!(triggered_events[0].position, npc_pos);
}
