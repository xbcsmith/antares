// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Example: Name Generator Usage
//!
//! Demonstrates how to use the name generator for creating NPCs and characters.
//!
//! Run with: cargo run --example name_generator_example

use antares::sdk::name_generator::{NameGenerator, NameTheme};

fn main() {
    println!("=== Antares Name Generator Examples ===\n");

    let generator = NameGenerator::new();

    // Example 1: Basic name generation
    println!("1. Basic Name Generation:");
    println!("   Fantasy: {}", generator.generate(NameTheme::Fantasy));
    println!("   Star: {}", generator.generate(NameTheme::Star));
    println!("   Antares: {}", generator.generate(NameTheme::Antares));
    println!("   Arcturus: {}", generator.generate(NameTheme::Arcturus));
    println!();

    // Example 2: Names with titles
    println!("2. Names with Titles (40% chance):");
    for _ in 0..5 {
        println!("   {}", generator.generate_with_title(NameTheme::Star));
    }
    println!();

    // Example 3: Names with lore
    println!("3. Names with Lore Descriptions:");
    let (name, lore) = generator.generate_with_lore(NameTheme::Antares);
    println!("   Name: {}", name);
    println!("   Lore: {}", lore);
    println!();

    // Example 4: Batch generation for a town
    println!("4. Generating Town NPCs (10 Fantasy names):");
    let townsfolk = generator.generate_multiple(10, NameTheme::Fantasy);
    for (i, name) in townsfolk.iter().enumerate() {
        println!("   {}. {}", i + 1, name);
    }
    println!();

    // Example 5: Generate guard patrol with lore
    println!("5. Guard Patrol (Arcturus theme with backstories):");
    let guards = generator.generate_multiple_with_lore(3, NameTheme::Arcturus);
    for (i, (name, backstory)) in guards.iter().enumerate() {
        println!("   Guard #{}: {}", i + 1, name);
        println!("   Background: {}", backstory);
        println!();
    }

    // Example 6: Themed character generation
    println!("6. Creating Theme-Based Characters:");

    println!("\n   Warrior Faction (Antares - aggressive, Mars-themed):");
    let warriors = generator.generate_multiple(5, NameTheme::Antares);
    for name in &warriors {
        println!("      - {}", name);
    }

    println!("\n   Guardian Order (Arcturus - protective, bear-themed):");
    let guardians = generator.generate_multiple(5, NameTheme::Arcturus);
    for name in &guardians {
        println!("      - {}", name);
    }

    println!("\n   Celestial Mages (Star - cosmic, astronomer-themed):");
    let mages = generator.generate_multiple(5, NameTheme::Star);
    for name in &mages {
        println!("      - {}", name);
    }

    // Example 7: Pre-generated character selection
    println!("\n7. Pre-Generated Character Selection:");
    println!("   Choose your hero:\n");
    let heroes = generator.generate_multiple_with_lore(4, NameTheme::Star);
    for (i, (name, backstory)) in heroes.iter().enumerate() {
        println!("   [{}] {}", i + 1, name);
        println!("       {}\n", backstory);
    }

    // Example 8: Race-specific naming (simulation)
    println!("8. Race-Specific Name Suggestions:");

    let race_themes = [
        ("Human", NameTheme::Fantasy),
        ("Elf", NameTheme::Star),
        ("Dwarf", NameTheme::Fantasy),
        ("Half-Orc", NameTheme::Antares),
    ];

    for (race, theme) in &race_themes {
        let names = generator.generate_multiple(3, *theme);
        println!("   {} names:", race);
        for name in names {
            println!("      - {}", name);
        }
    }
    println!();

    // Example 9: Large batch generation
    println!("9. Large Batch Generation (100 names for reference):");
    let large_batch = generator.generate_multiple(100, NameTheme::Fantasy);
    println!("   Generated {} unique names", large_batch.len());
    println!("   First 10: {:?}", &large_batch[0..10]);
    println!();

    // Example 10: All themes comparison
    println!("10. Theme Comparison (same structure, different vibes):");
    let themes = [
        ("Fantasy", NameTheme::Fantasy),
        ("Star", NameTheme::Star),
        ("Antares", NameTheme::Antares),
        ("Arcturus", NameTheme::Arcturus),
    ];

    for (theme_name, theme) in &themes {
        let name = generator.generate(*theme);
        println!("   {:12} => {}", theme_name, name);
    }
    println!();

    println!("=== Example Complete ===");
    println!("\nTry the CLI tool for interactive generation:");
    println!("  cargo run --bin antares-name-gen -- --help");
}
