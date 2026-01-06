// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests to validate map data files.
//!
//! Ensures every map under `data/maps` and `campaigns/tutorial/data/maps` has
//! non-empty `name` and `description` for all events and NPCs. This catches
//! missing metadata that can harm editor UX.

use std::fs;
use std::path::Path;

use antares::domain::world::{Map, MapEvent};

/// Scan maps to ensure every event and NPC has a non-empty name and description.
#[test]
fn test_maps_have_event_and_npc_names_and_descriptions() -> Result<(), Box<dyn std::error::Error>> {
    // Determine project root from crate manifest dir (sdk/campaign_builder)
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or("Unable to determine project root")?;

    let map_dirs = [
        project_root.join("data").join("maps"),
        project_root
            .join("campaigns")
            .join("tutorial")
            .join("data")
            .join("maps"),
    ];

    for dir in &map_dirs {
        let dir = dir.canonicalize().map_err(|e| {
            format!(
                "Failed to canonicalize map directory {}: {}",
                dir.display(),
                e
            )
        })?;

        if !dir.is_dir() {
            // Skip non-existent dirs (defensive)
            continue;
        }

        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ron") {
                continue;
            }

            let content = fs::read_to_string(&path)?;
            let map: Map =
                ron::de::from_str(&content).map_err(|e| -> Box<dyn std::error::Error> {
                    Box::from(format!("RON parse error in {}: {}", path.display(), e))
                })?;

            // Validate events
            for (pos, event) in &map.events {
                match event {
                    MapEvent::Encounter {
                        name, description, ..
                    } => {
                        assert!(
                            !name.trim().is_empty(),
                            "Encounter in map {} at {:?} has empty name",
                            path.display(),
                            pos
                        );
                        assert!(
                            !description.trim().is_empty(),
                            "Encounter in map {} at {:?} has empty description",
                            path.display(),
                            pos
                        );
                    }
                    MapEvent::Treasure {
                        name, description, ..
                    } => {
                        assert!(
                            !name.trim().is_empty(),
                            "Treasure in map {} at {:?} has empty name",
                            path.display(),
                            pos
                        );
                        assert!(
                            !description.trim().is_empty(),
                            "Treasure in map {} at {:?} has empty description",
                            path.display(),
                            pos
                        );
                    }
                    MapEvent::Teleport {
                        name, description, ..
                    } => {
                        assert!(
                            !name.trim().is_empty(),
                            "Teleport in map {} at {:?} has empty name",
                            path.display(),
                            pos
                        );
                        assert!(
                            !description.trim().is_empty(),
                            "Teleport in map {} at {:?} has empty description",
                            path.display(),
                            pos
                        );
                    }
                    MapEvent::Trap {
                        name, description, ..
                    } => {
                        assert!(
                            !name.trim().is_empty(),
                            "Trap in map {} at {:?} has empty name",
                            path.display(),
                            pos
                        );
                        assert!(
                            !description.trim().is_empty(),
                            "Trap in map {} at {:?} has empty description",
                            path.display(),
                            pos
                        );
                    }
                    MapEvent::Sign {
                        name,
                        description,
                        text: _,
                    } => {
                        assert!(
                            !name.trim().is_empty(),
                            "Sign in map {} at {:?} has empty name",
                            path.display(),
                            pos
                        );
                        assert!(
                            !description.trim().is_empty(),
                            "Sign in map {} at {:?} has empty description",
                            path.display(),
                            pos
                        );
                    }
                    MapEvent::NpcDialogue {
                        name,
                        description,
                        npc_id: _,
                    } => {
                        assert!(
                            !name.trim().is_empty(),
                            "NpcDialogue in map {} at {:?} has empty name",
                            path.display(),
                            pos
                        );
                        assert!(
                            !description.trim().is_empty(),
                            "NpcDialogue in map {} at {:?} has empty description",
                            path.display(),
                            pos
                        );
                    }
                    MapEvent::RecruitableCharacter {
                        name,
                        description,
                        character_id: _,
                    } => {
                        assert!(
                            !name.trim().is_empty(),
                            "RecruitableCharacter in map {} at {:?} has empty name",
                            path.display(),
                            pos
                        );
                        assert!(
                            !description.trim().is_empty(),
                            "RecruitableCharacter in map {} at {:?} has empty description",
                            path.display(),
                            pos
                        );
                    }
                    MapEvent::EnterInn {
                        name,
                        description,
                        inn_id: _,
                    } => {
                        assert!(
                            !name.trim().is_empty(),
                            "EnterInn in map {} at {:?} has empty name",
                            path.display(),
                            pos
                        );
                        assert!(
                            !description.trim().is_empty(),
                            "EnterInn in map {} at {:?} has empty description",
                            path.display(),
                            pos
                        );
                    }
                }
            }

            // Validate NPCs
            // Validate NPC Placements
            for placement in &map.npc_placements {
                assert!(
                    !placement.npc_id.trim().is_empty(),
                    "NPC placement in map {} has empty npc_id",
                    path.display()
                );
                // position sanity check
                let pos = placement.position;
                assert!(
                    pos.x >= 0 && pos.y >= 0,
                    "NPC placement in map {} id {} has invalid position {:?}",
                    path.display(),
                    placement.npc_id,
                    pos
                );
            }
        }
    }

    Ok(())
}
