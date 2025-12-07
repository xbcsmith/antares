// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Data-driven validation tests for monster RON files
//!
//! These tests ensure `loot.experience` exists in the RON data files and that
//! there is no top-level `experience_value` field present. This mirrors the
//! in-code domain model which stores experience in the `LootTable` (`loot.experience`).

use ron::de::from_str;
use ron::value::Value as RonValue;
use std::env;
use std::fs;
use std::path::Path;

/// Helper for producing string-keyed `ron::Value` map keys.
fn str_key(s: &str) -> RonValue {
    RonValue::String(s.to_string())
}

/// Validates a single monsters.ron file: every monster must:
/// - be a RON map/object
/// - have a nested `loot` map
/// - have `loot.experience` defined
/// - NOT have a top-level `experience_value` entry (regression prevention)
fn validate_monsters_file(path: &Path) {
    let contents = fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("Failed to read monsters file {:?}: {}", path, e);
    });

    let monsters: Vec<RonValue> = from_str(&contents).unwrap_or_else(|e| {
        panic!("Failed to parse monsters RON at {:?}: {}", path, e);
    });

    for (idx, monster_value) in monsters.iter().enumerate() {
        // Monster must be a map/object
        let map = match monster_value {
            RonValue::Map(ref m) => m,
            other => panic!("Entry #{idx} in {:?} is not a map; found {:?}", path, other),
        };

        // Derive a friendly monster identifier (prefer name)
        let monster_ident = map
            .get(&str_key("name"))
            .and_then(|v| match v {
                RonValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_else(|| format!("[index {}]", idx));

        // Ensure top-level 'experience_value' is NOT present
        if map.get(&str_key("experience_value")).is_some() {
            panic!(
                "Found top-level unexpected `experience_value` field for monster '{}' in file {:?}. Use `loot.experience` instead.",
                monster_ident, path
            );
        }

        // Ensure 'loot' exists and is a map
        let loot_val = map.get(&str_key("loot")).unwrap_or_else(|| {
            panic!(
                "Monster '{}' in {:?} is missing `loot` block; expected `loot.experience`.",
                monster_ident, path
            );
        });

        let loot_map = match loot_val {
            RonValue::Map(ref lm) => lm,
            other => panic!(
                "Monster '{}' in {:?}: `loot` is not a map; found {:?}",
                monster_ident, path, other
            ),
        };

        // Ensure `loot.experience` exists
        if loot_map.get(&str_key("experience")).is_none() {
            panic!(
                "Monster '{}' in {:?}: `loot.experience` is missing. Define experience as `loot.experience = <number>`.",
                monster_ident, path
            );
        }
    }
}

#[test]
fn test_core_monsters_have_loot_experience() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let monsters_path = Path::new(&manifest_dir).join("data/monsters.ron");

    if !monsters_path.exists() {
        // If core monsters file is absent, fail loudly — this file should exist in repo.
        panic!("Core monsters file not found at {:?}", monsters_path);
    }

    validate_monsters_file(&monsters_path);
}

#[test]
fn test_campaigns_monsters_have_loot_experience_and_no_top_level_experience_value() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let campaigns_dir = Path::new(&manifest_dir).join("campaigns");

    // If there are no campaigns checked in (CI skip), exit quietly.
    if !campaigns_dir.exists() {
        println!(
            "No campaigns directory found at {:?}; skipping campaign monster checks.",
            campaigns_dir
        );
        return;
    }

    // Iterate campaign directories and validate any monsters.ron found in campaign/<id>/data/
    for entry in fs::read_dir(&campaigns_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let monsters_path = path.join("data").join("monsters.ron");
        if monsters_path.exists() {
            validate_monsters_file(&monsters_path);
        } else {
            // If campaign doesn't define monsters.ron (maybe intentionally), we just continue.
            println!("No monsters.ron found for campaign {:?}, skipping.", path);
        }
    }
}

#[test]
fn test_no_monsters_have_top_level_experience_value_anywhere() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    // Check the core file first
    let core_path = Path::new(&manifest_dir).join("data/monsters.ron");
    if core_path.exists() {
        // We can reuse `validate_monsters_file()` which already asserts no top-level field
        validate_monsters_file(&core_path);
    }

    // Walk campaign directories and validate if any monsters.ron exists
    let campaigns_dir = Path::new(&manifest_dir).join("campaigns");
    if campaigns_dir.exists() {
        for entry in fs::read_dir(&campaigns_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let monsters_path = path.join("data").join("monsters.ron");
            if monsters_path.exists() {
                validate_monsters_file(&monsters_path);
            }
        }
    }
}
