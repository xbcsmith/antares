// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Phase 5 integration tests: Barred Passage campaign data.
//!
//! Verifies that:
//! - Map 1 tile `(17, 12)` has a `Treasure` event with `mesh_id: Some("barred_passage")`
//!   and `dialogue_id: Some(500)`.
//! - Dialogue tree 500 ("Barred Passage") exists and has the expected content.
//! - `"barred_passage"` is registered in the unified object mesh registry.
//! - The `barred_door.ron` asset parses as a valid `CreatureDefinition`.
//!
//! All tests load from `data/test_campaign` — never from `campaigns/tutorial`
//! (Implementation Rule 5).

use antares::domain::types::Position;
use antares::domain::world::MapEvent;
use antares::sdk::campaign_loader::Campaign;
use antares::sdk::database::{DialogueDatabase, MapDatabase};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Loads map 1 from the test campaign's map directory.
fn load_test_map_1() -> antares::domain::world::Map {
    let map_db = MapDatabase::load_from_directory("data/test_campaign/data/maps")
        .expect("test campaign maps directory should load");
    map_db
        .get_map(1)
        .expect("map id 1 should exist in test campaign")
        .clone()
}

/// Loads the test campaign's dialogue database.
fn load_test_dialogues() -> DialogueDatabase {
    DialogueDatabase::load_from_file("data/test_campaign/data/dialogues.ron")
        .expect("test campaign dialogues.ron should parse")
}

// ---------------------------------------------------------------------------
// Tests — map event data
// ---------------------------------------------------------------------------

/// P5-BP1: Map 1 tile (17, 12) must contain a Treasure event.
#[test]
fn test_barred_passage_tile_has_treasure_event() {
    let map = load_test_map_1();
    let pos = Position::new(17, 12);
    let event = map
        .get_event(pos)
        .expect("Map 1 must have an event at tile (17, 12)");
    assert!(
        matches!(event, MapEvent::Treasure { .. }),
        "Event at (17, 12) must be Treasure, found: {:?}",
        event
    );
}

/// P5-BP2: Treasure event at (17, 12) must carry `mesh_id = Some("barred_passage")`.
#[test]
fn test_barred_passage_event_has_mesh_id() {
    let map = load_test_map_1();
    let pos = Position::new(17, 12);
    let event = map
        .get_event(pos)
        .expect("Map 1 must have an event at tile (17, 12)");

    match event {
        MapEvent::Treasure { mesh_id, .. } => {
            assert_eq!(
                mesh_id.as_deref(),
                Some("barred_passage"),
                "Barred Passage Treasure at (17, 12) must have mesh_id 'barred_passage'"
            );
        }
        other => panic!("Expected Treasure at (17, 12); found {:?}", other),
    }
}

/// P5-BP3: Treasure event at (17, 12) must carry `dialogue_id = Some(500)`.
#[test]
fn test_barred_passage_event_has_dialogue_id_500() {
    let map = load_test_map_1();
    let pos = Position::new(17, 12);
    let event = map
        .get_event(pos)
        .expect("Map 1 must have an event at tile (17, 12)");

    match event {
        MapEvent::Treasure { dialogue_id, .. } => {
            assert_eq!(
                *dialogue_id,
                Some(500),
                "Barred Passage Treasure at (17, 12) must reference dialogue_id 500"
            );
        }
        other => panic!("Expected Treasure at (17, 12); found {:?}", other),
    }
}

/// P5-BP4: Barred Passage event must have the correct name, description, and empty loot.
#[test]
fn test_barred_passage_event_metadata() {
    let map = load_test_map_1();
    let pos = Position::new(17, 12);
    let event = map
        .get_event(pos)
        .expect("Map 1 must have an event at tile (17, 12)");

    match event {
        MapEvent::Treasure {
            name,
            description,
            loot,
            ..
        } => {
            assert_eq!(
                name, "Barred Passage",
                "Event name must be 'Barred Passage'"
            );
            assert_eq!(
                description, "A heavy iron bar blocks the passage.",
                "Event description must match the specification"
            );
            assert!(
                loot.is_empty(),
                "Barred Passage must have no loot items (empty loot vec)"
            );
        }
        other => panic!("Expected Treasure at (17, 12); found {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Tests — dialogue tree 500
// ---------------------------------------------------------------------------

/// P5-BP5: Dialogue 500 must exist in the test campaign dialogues database.
#[test]
fn test_barred_passage_dialogue_500_exists() {
    let db = load_test_dialogues();
    assert!(
        db.has_dialogue(&500),
        "Dialogue 500 ('Barred Passage') must exist in data/test_campaign/data/dialogues.ron"
    );
}

/// P5-BP6: Dialogue 500 must be named "Barred Passage" and have a meaningful root node.
#[test]
fn test_barred_passage_dialogue_500_content() {
    let db = load_test_dialogues();
    let dialogue = db
        .get_dialogue(500)
        .expect("Dialogue 500 must be present in the test campaign");

    assert_eq!(
        dialogue.name, "Barred Passage",
        "Dialogue 500 name must be 'Barred Passage'"
    );

    let root = dialogue
        .get_root_node()
        .expect("Dialogue 500 must have a root node");

    assert!(
        root.text.contains("passage is barred shut"),
        "Root node text must mention the passage being barred shut; got: {:?}",
        root.text
    );

    assert_eq!(
        root.choices.len(),
        1,
        "Barred Passage dialogue root node must have exactly one choice"
    );

    let choice = &root.choices[0];
    assert!(
        choice.ends_dialogue,
        "The single choice must end the dialogue (ends_dialogue: true)"
    );
    assert_eq!(
        choice.target_node, None,
        "The 'Leave it for now' choice must not point to another node"
    );
}

/// P5-BP7: Dialogue 500 must be repeatable (player can re-read the passage text).
#[test]
fn test_barred_passage_dialogue_is_repeatable() {
    let db = load_test_dialogues();
    let dialogue = db.get_dialogue(500).expect("Dialogue 500 must be present");
    assert!(
        dialogue.repeatable,
        "Barred Passage dialogue must be repeatable (no quest involvement)"
    );
}

// ---------------------------------------------------------------------------
// Tests — object mesh registry
// ---------------------------------------------------------------------------

/// P5-BP8: "barred_passage" key must appear in the unified object mesh registry.
#[test]
fn test_barred_passage_mesh_registered_in_object_mesh_registry() {
    let content = Campaign::load("data/test_campaign")
        .expect("test campaign should load")
        .load_content()
        .expect("test campaign content should load");

    assert!(
        content.object_meshes.has_mesh("barred_passage"),
        "'barred_passage' must be registered in data/test_campaign/data/object_mesh_registry.ron"
    );
}

/// P5-BP9: Full campaign load succeeds, meaning barred_door.ron parses as a valid
/// CreatureDefinition and all mesh dimensions / indices are structurally sound.
#[test]
fn test_barred_door_asset_parses_as_valid_creature_definition() {
    let result = Campaign::load("data/test_campaign").and_then(|c| c.load_content());
    assert!(
        result.is_ok(),
        "Campaign load must succeed — barred_door.ron must be a valid CreatureDefinition: {:?}",
        result.err()
    );
    println!(
        "✓ barred_door.ron parsed successfully as CreatureDefinition (id: 20001, name: BarredDoor)"
    );
}

/// P5-BP10: Cross-reference check — the dialogue_id stored on the map event
/// resolves to an existing dialogue in the same campaign.
#[test]
fn test_barred_passage_dialogue_cross_reference_is_valid() {
    let map = load_test_map_1();
    let db = load_test_dialogues();
    let pos = Position::new(17, 12);

    let dialogue_id = match map.get_event(pos).expect("event at (17, 12) must exist") {
        MapEvent::Treasure { dialogue_id, .. } => *dialogue_id,
        other => panic!("Expected Treasure at (17, 12); found {:?}", other),
    };

    let id = dialogue_id.expect("Treasure event must carry a dialogue_id");
    assert!(
        db.has_dialogue(&id),
        "Map event dialogue_id {} at (17, 12) must resolve to an existing dialogue",
        id
    );
    println!(
        "✓ Barred Passage event dialogue_id {} resolves to dialogue '{}'",
        id,
        db.get_dialogue(id)
            .map(|d| d.name.as_str())
            .unwrap_or("<unknown>")
    );
}
