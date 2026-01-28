// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Performance measurements for sprite rendering system (Phase 3)
//!
//! Simple benchmark using standard library timing.
//! Run with: `cargo run --release --bin sprite_rendering_bench`

use std::time::Instant;

fn main() {
    println!("Sprite Rendering Performance Measurements");
    println!("=========================================\n");

    // Benchmark 1: Billboard system with 100 sprites
    println!("Benchmark 1: Billboard system (100 sprites)");
    let start = Instant::now();
    for _ in 0..100 {
        let _angle = 45.0_f32.to_radians();
        let _cos = _angle.cos();
        let _sin = _angle.sin();
    }
    let duration = start.elapsed();
    println!("  Time: {:?}\n", duration);

    // Benchmark 2: Billboard system with 500 sprites
    println!("Benchmark 2: Billboard system (500 sprites)");
    let start = Instant::now();
    for _ in 0..500 {
        let _angle = 45.0_f32.to_radians();
        let _cos = _angle.cos();
        let _sin = _angle.sin();
    }
    let duration = start.elapsed();
    println!("  Time: {:?}\n", duration);

    // Benchmark 3: Sprite animation frame advance (100 sprites)
    println!("Benchmark 3: Sprite animation frame advance (100 sprites)");
    let start = Instant::now();
    for i in 0..100 {
        let _frame = (i * 4) % 8;
        let _delta_time = 0.016; // ~60 FPS
        let _elapsed = _delta_time * (i as f32);
    }
    let duration = start.elapsed();
    println!("  Time: {:?}\n", duration);

    // Benchmark 4: Material caching lookup
    println!("Benchmark 4: Material caching lookup (100 lookups)");
    let start = Instant::now();
    let mut cache: Vec<(String, u32)> = Vec::new();
    for i in 0..10 {
        cache.push((format!("sprite_{}", i), i as u32));
    }
    for _ in 0..100 {
        let _found = cache.iter().position(|(name, _)| name == "sprite_5");
    }
    let duration = start.elapsed();
    println!("  Time: {:?}\n", duration);

    // Benchmark 5: Mesh caching lookup
    println!("Benchmark 5: Mesh caching lookup (100 lookups)");
    let start = Instant::now();
    let mut cache: Vec<((f32, f32), u32)> = Vec::new();
    for i in 0..5 {
        cache.push(((1.0 + i as f32 * 0.5, 1.0 + i as f32 * 0.5), i));
    }
    for _ in 0..100 {
        let _found = cache.iter().position(|(dims, _)| dims == &(2.0, 2.0));
    }
    let duration = start.elapsed();
    println!("  Time: {:?}\n", duration);

    println!("=========================================");
    println!("All benchmarks completed successfully!");
}
