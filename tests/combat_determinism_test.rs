// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Determinism tests for the seeded [`GameRng`] resource.
//!
//! Phase 4 of the codebase-cleanup plan replaced ad-hoc `rand::rng()` calls at
//! the gameplay boundary with a single shared, seedable RNG. These tests assert
//! the core determinism guarantee that makes save/load and reproducible combat
//! possible: **the same seed driving the same sequence of combat-relevant rolls
//! produces byte-for-byte identical outcomes**, while a different seed diverges.

use antares::domain::magic::fizzle::roll_fizzle;
use antares::domain::progression::roll_hp_gain;
use antares::domain::types::DiceRoll;
use antares::game::resources::GameRng;

/// Drives a fixed, combat-representative sequence of rolls against the supplied
/// RNG and records every observable outcome.
///
/// The sequence intentionally mixes the roll shapes the combat and progression
/// systems actually use — attack/damage dice, per-class HP dice, fizzle checks,
/// and raw range rolls — so the recorded trace is a faithful proxy for a real
/// combat encounter's RNG consumption.
fn run_roll_sequence(game_rng: &mut GameRng) -> Vec<i64> {
    use rand::RngExt as _;

    let rng = game_rng.rng();
    let mut trace: Vec<i64> = Vec::new();

    // 1d20 "attack rolls", 2d6+1 "damage", per-class HP dice, fizzle checks, and
    // a raw range roll — repeated to build a long, sensitive trace.
    let classes = ["knight", "sorcerer", "cleric", "robber"];
    for round in 0..64_u32 {
        let attack = DiceRoll::new(1, 20, 0).roll(rng);
        trace.push(attack as i64);

        let damage = DiceRoll::new(2, 6, 1).roll(rng);
        trace.push(damage as i64);

        let class_id = classes[(round as usize) % classes.len()];
        let hp = roll_hp_gain(class_id, rng);
        trace.push(hp as i64);

        let fizzled = roll_fizzle(35, rng);
        trace.push(i64::from(fizzled));

        let raw = rng.random_range(0..1_000_000_u32);
        trace.push(raw as i64);
    }

    trace
}

/// Same seed + same roll sequence must yield identical traces.
#[test]
fn test_same_seed_produces_identical_combat_trace() {
    const SEED: u64 = 0xA17A_2E50_u64;

    let mut rng_a = GameRng::from_seed(SEED);
    let mut rng_b = GameRng::from_seed(SEED);

    let trace_a = run_roll_sequence(&mut rng_a);
    let trace_b = run_roll_sequence(&mut rng_b);

    assert_eq!(
        trace_a, trace_b,
        "identical seeds must produce identical combat RNG traces"
    );
    assert!(
        !trace_a.is_empty(),
        "the roll sequence must actually consume the RNG"
    );
}

/// Different seeds must (with overwhelming probability) diverge, proving the
/// trace is genuinely seed-driven rather than constant.
#[test]
fn test_different_seed_produces_different_combat_trace() {
    let mut rng_a = GameRng::from_seed(1);
    let mut rng_b = GameRng::from_seed(2);

    let trace_a = run_roll_sequence(&mut rng_a);
    let trace_b = run_roll_sequence(&mut rng_b);

    assert_ne!(
        trace_a, trace_b,
        "distinct seeds must produce distinct combat RNG traces"
    );
}

/// Reseeding to the original seed rewinds the stream: a fresh run after
/// `reseed` reproduces the very first trace. This is exactly the guarantee the
/// save/load path relies on when it restores `GameState::rng_seed`.
#[test]
fn test_reseed_restores_deterministic_stream() {
    const SEED: u64 = 987_654_321_u64;

    let mut game_rng = GameRng::from_seed(SEED);
    let first = run_roll_sequence(&mut game_rng);

    // Advance the stream further so internal state is definitely different.
    let _ = run_roll_sequence(&mut game_rng);

    // Reseeding must rewind to the initial deterministic stream.
    game_rng.reseed(SEED);
    assert_eq!(game_rng.seed(), SEED);

    let after_reseed = run_roll_sequence(&mut game_rng);
    assert_eq!(
        first, after_reseed,
        "reseeding to the original seed must reproduce the original trace"
    );
}
