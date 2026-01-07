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
                        innkeeper_id,
                    } => {
                        // Basic presence checks
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

                        // Validate innkeeper_id: must be non-empty and must reference an NPC
                        let inn_id = innkeeper_id.trim();
                        assert!(
                            !inn_id.is_empty(),
                            "EnterInn in map {} at {:?} has empty innkeeper_id",
                            path.display(),
                            pos
                        );

                        // Look for an `npcs.ron` in the parent of the maps directory (e.g., campaigns/<campaign>/data/npcs.ron)
                        let mut found_and_valid = false;

                        if let Some(parent_dir) = dir.parent() {
                            let npcs_file = parent_dir.join("npcs.ron");
                            if npcs_file.exists() {
                                // Parse npcs.ron and verify the referenced NPC exists and is an innkeeper
                                let npcs_contents =
                                    fs::read_to_string(&npcs_file).map_err(|e| {
                                        format!(
                                            "Failed to read npcs.ron {}: {}",
                                            npcs_file.display(),
                                            e
                                        )
                                    })?;

                                let npcs: Vec<antares::domain::world::npc::NpcDefinition> =
                                    ron::de::from_str(&npcs_contents).map_err(
                                        |e| -> Box<dyn std::error::Error> {
                                            Box::from(format!(
                                                "RON parse error in {}: {}",
                                                npcs_file.display(),
                                                e
                                            ))
                                        },
                                    )?;

                                if let Some(npc) = npcs.iter().find(|n| n.id == inn_id) {
                                    assert!(
                                        npc.is_innkeeper,
                                        "EnterInn in map {} at {:?} references NPC '{}' which is not an innkeeper (is_innkeeper=false)",
                                        path.display(),
                                        pos,
                                        inn_id
                                    );
                                    found_and_valid = true;
                                } else {
                                    // Not found in npcs.ron; fall back to checking placements below
                                    found_and_valid = false;
                                }
                            }
                        }

                        // If an npcs.ron wasn't found or didn't contain the NPC, ensure the NPC is placed on the map
                        if !found_and_valid {
                            let placed = map.npc_placements.iter().any(|p| p.npc_id == inn_id);
                            assert!(
                                placed,
                                "EnterInn in map {} at {:?} references NPC '{}' not found in npcs.ron nor placed on map",
                                path.display(),
                                pos,
                                inn_id
                            );
                        }
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
