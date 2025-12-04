// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for ContentDatabase loading and validation

use antares::domain::dialogue::DialogueTree;
use antares::domain::quest::Quest;
use antares::sdk::database::ContentDatabase;
use std::fs;
use std::path::Path;

#[test]
fn test_load_full_campaign() {
    // This test attempts to load the actual tutorial campaign
    // It serves as an integration test for all content types

    // Use absolute path relative to cargo manifest dir to ensure we find the campaign
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let campaign_dir = Path::new(&manifest_dir).join("campaigns/tutorial");

    if !campaign_dir.exists() {
        // Skip test if tutorial campaign doesn't exist (e.g. in CI without assets)
        println!(
            "Skipping test: Tutorial campaign not found at {:?}",
            campaign_dir
        );
        return;
    }

    let db =
        ContentDatabase::load_campaign(&campaign_dir).expect("Failed to load tutorial campaign");

    // Verify we loaded content
    let stats = db.stats();
    println!("Loaded stats: {:?}", stats);

    // We expect some content in all categories
    assert!(stats.class_count > 0, "Should have classes");
    assert!(stats.item_count > 0, "Should have items");
    assert!(stats.monster_count > 0, "Should have monsters");
    assert!(stats.spell_count > 0, "Should have spells");
    assert!(stats.quest_count > 0, "Should have quests");
    assert!(stats.dialogue_count > 0, "Should have dialogues");

    // Verify specific counts based on actual data files in the tutorial campaign.
    // Instead of asserting hard-coded values, parse the RON data files and
    // assert the database counts match the file counts. This makes the test
    // robust against content updates in the tutorial campaign.
    let data_dir = campaign_dir.join("data");

    // Quests: compare DB count to file count if quests.ron exists
    let quests_path = data_dir.join("quests.ron");
    if quests_path.exists() {
        let quests_contents = fs::read_to_string(&quests_path).expect("Failed to read quests.ron");
        let quests_file: Vec<Quest> =
            ron::from_str(&quests_contents).expect("Failed to parse quests.ron");
        assert_eq!(
            stats.quest_count,
            quests_file.len(),
            "Database quest count ({}) should match quests.ron file count ({})",
            stats.quest_count,
            quests_file.len()
        );
    } else {
        // If the file is missing, we expect the DB to be empty for that type
        assert_eq!(
            stats.quest_count, 0,
            "No quests.ron file found and DB should have no quests"
        );
    }

    // Dialogues: compare DB count to file count if dialogues.ron exists
    let dialogues_path = data_dir.join("dialogues.ron");
    if dialogues_path.exists() {
        let dialogues_contents =
            fs::read_to_string(&dialogues_path).expect("Failed to read dialogues.ron");
        let dialogues_file: Vec<DialogueTree> =
            ron::from_str(&dialogues_contents).expect("Failed to parse dialogues.ron");
        assert_eq!(
            stats.dialogue_count,
            dialogues_file.len(),
            "Database dialogue count ({}) should match dialogues.ron file count ({})",
            stats.dialogue_count,
            dialogues_file.len()
        );
    } else {
        assert_eq!(
            stats.dialogue_count, 0,
            "No dialogues.ron file found and DB should have no dialogues"
        );
    }

    // Validate the loaded content
    // Note: We expect validation to pass for the tutorial campaign
    // If it fails, it means the tutorial data is inconsistent or our validation is too strict
    if let Err(e) = db.validate() {
        panic!("Tutorial campaign validation failed: {}", e);
    }
}
