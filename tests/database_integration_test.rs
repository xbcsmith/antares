// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for ContentDatabase loading and validation

use antares::sdk::database::ContentDatabase;
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

    // Verify specific counts based on known tutorial content
    // Note: These numbers might change if tutorial content is updated,
    // but they should match what we saw in the files (4 quests, 1 dialogue)
    assert_eq!(stats.quest_count, 4, "Should have 4 quests");
    assert_eq!(stats.dialogue_count, 1, "Should have 1 dialogue");

    // Validate the loaded content
    // Note: We expect validation to pass for the tutorial campaign
    // If it fails, it means the tutorial data is inconsistent or our validation is too strict
    if let Err(e) = db.validate() {
        panic!("Tutorial campaign validation failed: {}", e);
    }
}
