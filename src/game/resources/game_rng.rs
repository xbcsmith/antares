// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Deterministic gameplay random-number generator resource.
//!
//! Antares' architecture calls for deterministic gameplay: given the same
//! starting seed and the same sequence of player inputs, a run must produce the
//! same outcomes so that save/load and replay are reproducible. To honour that,
//! all gameplay randomness flows from a single seeded generator rather than from
//! per-call [`rand::rng()`] (which is reseeded from OS entropy on every call and
//! is therefore non-reproducible).
//!
//! [`GameRng`] is the Bevy [`Resource`] that owns that generator. Systems pull
//! it in as `ResMut<GameRng>` and hand `game_rng.rng()` to the domain functions
//! that consume randomness. The seed is stored on
//! [`GameState`](crate::application::GameState) so it is persisted with the save
//! file; on load the resource is re-created from the persisted seed.

use bevy::prelude::Resource;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Seeded gameplay RNG resource.
///
/// Wraps a [`StdRng`] together with the seed it was created from. The seed is
/// retained so it can be persisted (via
/// [`GameState::rng_seed`](crate::application::GameState)) and surfaced for
/// debugging/replay.
///
/// # Examples
///
/// ```
/// use antares::game::resources::GameRng;
/// use rand::RngExt;
///
/// // Two generators created from the same seed produce the same stream.
/// let mut a = GameRng::from_seed(42);
/// let mut b = GameRng::from_seed(42);
/// let sa: Vec<u32> = (0..8).map(|_| a.rng().random_range(0..1000)).collect();
/// let sb: Vec<u32> = (0..8).map(|_| b.rng().random_range(0..1000)).collect();
/// assert_eq!(sa, sb);
/// assert_eq!(a.seed(), 42);
/// ```
#[derive(Resource, Debug)]
pub struct GameRng {
    seed: u64,
    rng: StdRng,
}

impl GameRng {
    /// Creates a generator from an explicit seed.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::GameRng;
    ///
    /// let rng = GameRng::from_seed(7);
    /// assert_eq!(rng.seed(), 7);
    /// ```
    pub fn from_seed(seed: u64) -> Self {
        Self {
            seed,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Creates a generator from a fresh OS-entropy seed.
    ///
    /// Use this only when no persisted seed is available (e.g. before a game
    /// has been created). The chosen seed is recorded and can be read via
    /// [`GameRng::seed`].
    pub fn from_entropy() -> Self {
        use rand::RngExt;
        let seed = rand::rng().random::<u64>();
        Self::from_seed(seed)
    }

    /// Returns the seed this generator was created from.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Re-seeds the generator in place, resetting its stream to the start of the
    /// sequence produced by `seed`.
    ///
    /// Used when loading a save so the live resource matches the persisted seed.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::GameRng;
    ///
    /// let mut rng = GameRng::from_seed(1);
    /// rng.reseed(99);
    /// assert_eq!(rng.seed(), 99);
    /// ```
    pub fn reseed(&mut self, seed: u64) {
        self.seed = seed;
        self.rng = StdRng::seed_from_u64(seed);
    }

    /// Returns a mutable reference to the underlying [`StdRng`] for use with the
    /// `rand` API and domain functions that take `&mut impl Rng`.
    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    /// Produces a standalone entropy-seeded [`StdRng`] for systems that may run
    /// without the [`GameRng`] resource inserted (e.g. minimal test apps).
    ///
    /// Production runs always insert the resource, so this fallback is only used
    /// in isolated tests where reproducibility is not required.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::GameRng;
    /// use rand::RngExt;
    ///
    /// let mut rng = GameRng::fallback_std_rng();
    /// let _ = rng.random_range(0..10);
    /// ```
    pub fn fallback_std_rng() -> StdRng {
        use rand::RngExt;
        StdRng::seed_from_u64(rand::rng().random::<u64>())
    }
}

impl Default for GameRng {
    /// Creates a generator from a fresh OS-entropy seed. Prefer
    /// [`GameRng::from_seed`] with the persisted [`GameState`](crate::application::GameState)
    /// seed for reproducible runs.
    fn default() -> Self {
        Self::from_entropy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngExt;

    #[test]
    fn test_same_seed_same_sequence() {
        let mut a = GameRng::from_seed(12345);
        let mut b = GameRng::from_seed(12345);
        for _ in 0..64 {
            assert_eq!(a.rng().random::<u64>(), b.rng().random::<u64>());
        }
    }

    #[test]
    fn test_different_seed_different_sequence() {
        let mut a = GameRng::from_seed(1);
        let mut b = GameRng::from_seed(2);
        let sa: Vec<u64> = (0..16).map(|_| a.rng().random::<u64>()).collect();
        let sb: Vec<u64> = (0..16).map(|_| b.rng().random::<u64>()).collect();
        assert_ne!(sa, sb);
    }

    #[test]
    fn test_reseed_resets_stream() {
        let mut rng = GameRng::from_seed(5);
        let first: Vec<u64> = (0..8).map(|_| rng.rng().random::<u64>()).collect();
        // Advance the stream, then reseed back to 5 and confirm we replay it.
        let _ = rng.rng().random::<u64>();
        rng.reseed(5);
        let replay: Vec<u64> = (0..8).map(|_| rng.rng().random::<u64>()).collect();
        assert_eq!(first, replay);
        assert_eq!(rng.seed(), 5);
    }

    #[test]
    fn test_seed_accessor_records_seed() {
        let rng = GameRng::from_seed(0xDEAD_BEEF);
        assert_eq!(rng.seed(), 0xDEAD_BEEF);
    }
}
