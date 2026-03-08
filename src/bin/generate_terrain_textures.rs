// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Binary that generates placeholder terrain textures for the Antares game.
//!
//! Each texture is a 64×64 RGBA PNG with a solid base colour and a deterministic
//! per-pixel noise overlay of ±10 per channel. The noise seed is fixed so that
//! the output is identical across runs (reproducible builds).
//!
//! # Usage
//!
//! ```text
//! cargo run --bin generate_terrain_textures
//! ```
//!
//! Output files are written to `assets/textures/terrain/` relative to
//! `CARGO_MANIFEST_DIR`.

use image::{ImageBuffer, Rgba};
use std::path::PathBuf;

// ──────────────────────────────────────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────────────────────────────────────

/// Output image width in pixels.
const IMAGE_WIDTH: u32 = 64;
/// Output image height in pixels.
const IMAGE_HEIGHT: u32 = 64;
/// Maximum per-channel noise magnitude (±NOISE_RANGE).
const NOISE_RANGE: i32 = 10;

/// Describes one terrain texture to generate.
struct TerrainTextureSpec {
    /// Output filename (placed in `assets/textures/terrain/`).
    filename: &'static str,
    /// Base red channel value (0–255).
    r: u8,
    /// Base green channel value (0–255).
    g: u8,
    /// Base blue channel value (0–255).
    b: u8,
    /// Alpha channel value (always 255 for terrain tiles).
    a: u8,
    /// Deterministic noise seed – unique per texture so each looks different.
    seed: u64,
}

/// All nine terrain textures defined in the implementation plan (Section 1.1).
const TERRAIN_SPECS: &[TerrainTextureSpec] = &[
    TerrainTextureSpec {
        filename: "ground.png",
        r: 100,
        g: 95,
        b: 85,
        a: 255,
        seed: 0x1A2B_3C4D_5E6F_7081,
    },
    TerrainTextureSpec {
        filename: "grass.png",
        r: 65,
        g: 120,
        b: 50,
        a: 255,
        seed: 0x2B3C_4D5E_6F70_8192,
    },
    TerrainTextureSpec {
        filename: "stone.png",
        r: 130,
        g: 130,
        b: 135,
        a: 255,
        seed: 0x3C4D_5E6F_7081_92A3,
    },
    TerrainTextureSpec {
        filename: "mountain.png",
        r: 90,
        g: 88,
        b: 90,
        a: 255,
        seed: 0x4D5E_6F70_8192_A3B4,
    },
    TerrainTextureSpec {
        filename: "dirt.png",
        r: 110,
        g: 80,
        b: 55,
        a: 255,
        seed: 0x5E6F_7081_92A3_B4C5,
    },
    TerrainTextureSpec {
        filename: "water.png",
        r: 55,
        g: 105,
        b: 200,
        a: 255,
        seed: 0x6F70_8192_A3B4_C5D6,
    },
    TerrainTextureSpec {
        filename: "lava.png",
        r: 210,
        g: 75,
        b: 50,
        a: 255,
        seed: 0x7081_92A3_B4C5_D6E7,
    },
    TerrainTextureSpec {
        filename: "swamp.png",
        r: 88,
        g: 100,
        b: 55,
        a: 255,
        seed: 0x8192_A3B4_C5D6_E7F8,
    },
    TerrainTextureSpec {
        filename: "forest_floor.png",
        r: 50,
        g: 95,
        b: 40,
        a: 255,
        seed: 0x92A3_B4C5_D6E7_F809,
    },
];

// ──────────────────────────────────────────────────────────────────────────────
// Noise helper
// ──────────────────────────────────────────────────────────────────────────────

/// Minimal xorshift64 PRNG – deterministic, no external crate needed.
///
/// Returns the next pseudo-random `u64` state value.
fn xorshift64(state: u64) -> u64 {
    let mut x = state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

/// Clamps `value` into the `[0, 255]` range and returns a `u8`.
fn clamp_channel(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

/// Applies ±`NOISE_RANGE` noise to a base channel value using the provided
/// PRNG state, returning `(noisy_value, next_state)`.
fn apply_noise(base: u8, state: u64) -> (u8, u64) {
    let next = xorshift64(state);
    // Use unsigned modulo to get a value in [0, NOISE_RANGE * 2] then shift to
    // [-NOISE_RANGE, NOISE_RANGE].  Unsigned modulo never produces a negative
    // remainder, so the offset is always within bounds.
    let range = (NOISE_RANGE * 2 + 1) as u64;
    let offset = (next % range) as i32 - NOISE_RANGE;
    (clamp_channel(base as i32 + offset), next)
}

// ──────────────────────────────────────────────────────────────────────────────
// Texture generation
// ──────────────────────────────────────────────────────────────────────────────

/// Generates a single 64×64 RGBA texture for the given [`TerrainTextureSpec`].
///
/// Each pixel's R, G, and B channels receive independent noise. Alpha is always
/// set to the spec's `a` value (255 for all terrain tiles).
fn generate_texture(spec: &TerrainTextureSpec) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(IMAGE_WIDTH, IMAGE_HEIGHT);
    let mut state = spec.seed;

    for y in 0..IMAGE_HEIGHT {
        for x in 0..IMAGE_WIDTH {
            let (r, s1) = apply_noise(spec.r, state);
            let (g, s2) = apply_noise(spec.g, s1);
            let (b, s3) = apply_noise(spec.b, s2);
            state = s3;
            img.put_pixel(x, y, Rgba([r, g, b, spec.a]));
        }
    }

    img
}

// ──────────────────────────────────────────────────────────────────────────────
// Entry point
// ──────────────────────────────────────────────────────────────────────────────

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let output_dir = PathBuf::from(manifest_dir).join("assets/textures/terrain");

    std::fs::create_dir_all(&output_dir).unwrap_or_else(|e| {
        eprintln!(
            "ERROR: Could not create output directory '{}': {e}",
            output_dir.display()
        );
        std::process::exit(1);
    });

    println!("Writing terrain textures to: {}", output_dir.display());

    for spec in TERRAIN_SPECS {
        let path = output_dir.join(spec.filename);
        let img = generate_texture(spec);

        match img.save(&path) {
            Ok(()) => println!("  ✓  {}", spec.filename),
            Err(e) => {
                eprintln!("ERROR: Failed to write '{}': {e}", path.display());
                std::process::exit(1);
            }
        }
    }

    println!("Done. {} textures written.", TERRAIN_SPECS.len());
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// xorshift64 must not return the same value it was given (it must advance).
    #[test]
    fn test_xorshift64_advances_state() {
        let seed = 0xDEAD_BEEF_CAFE_1234_u64;
        let next = xorshift64(seed);
        assert_ne!(next, seed, "xorshift64 must change the state");
    }

    /// xorshift64 is deterministic: same input → same output.
    #[test]
    fn test_xorshift64_is_deterministic() {
        let seed = 0x1122_3344_5566_7788_u64;
        assert_eq!(xorshift64(seed), xorshift64(seed));
    }

    /// `clamp_channel` must saturate at 0.
    #[test]
    fn test_clamp_channel_low() {
        assert_eq!(clamp_channel(-1), 0);
        assert_eq!(clamp_channel(i32::MIN), 0);
    }

    /// `clamp_channel` must saturate at 255.
    #[test]
    fn test_clamp_channel_high() {
        assert_eq!(clamp_channel(256), 255);
        assert_eq!(clamp_channel(i32::MAX), 255);
    }

    /// `clamp_channel` must pass through valid values unchanged.
    #[test]
    fn test_clamp_channel_valid_range() {
        for v in 0..=255_i32 {
            assert_eq!(clamp_channel(v), v as u8);
        }
    }

    /// `apply_noise` must keep the noisy channel within the clamped `[0, 255]`
    /// range and no further than `NOISE_RANGE` away from the base value
    /// (modulo saturation at the boundaries).
    #[test]
    fn test_apply_noise_stays_in_bounds() {
        let state = 0xABCD_EF01_2345_6789_u64;
        for base in [0_u8, 10, 128, 245, 255] {
            let (noisy, _) = apply_noise(base, state);
            // The noisy value must stay within NOISE_RANGE of the base,
            // clamped to [0, 255].
            let lo = (base as i32 - NOISE_RANGE).max(0) as u8;
            let hi = (base as i32 + NOISE_RANGE).min(255) as u8;
            assert!(
                noisy >= lo && noisy <= hi,
                "apply_noise({base}) = {noisy} is outside [{lo}, {hi}]"
            );
        }
    }

    /// `apply_noise` must return a different PRNG state each call.
    #[test]
    fn test_apply_noise_advances_state() {
        let state = 0x1111_2222_3333_4444_u64;
        let (_, next) = apply_noise(128, state);
        assert_ne!(next, state);
    }

    /// Every entry in `TERRAIN_SPECS` must have a unique seed.
    #[test]
    fn test_terrain_specs_unique_seeds() {
        let seeds: Vec<u64> = TERRAIN_SPECS.iter().map(|s| s.seed).collect();
        let mut sorted = seeds.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            seeds.len(),
            "Duplicate seeds found in TERRAIN_SPECS"
        );
    }

    /// Every entry in `TERRAIN_SPECS` must have a unique filename.
    #[test]
    fn test_terrain_specs_unique_filenames() {
        let mut names: Vec<&str> = TERRAIN_SPECS.iter().map(|s| s.filename).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), TERRAIN_SPECS.len());
    }

    /// `TERRAIN_SPECS` must contain exactly nine entries (one per terrain type).
    #[test]
    fn test_terrain_specs_count() {
        assert_eq!(TERRAIN_SPECS.len(), 9, "Expected exactly 9 terrain specs");
    }

    /// All specs must have alpha == 255.
    #[test]
    fn test_terrain_specs_all_opaque() {
        for spec in TERRAIN_SPECS {
            assert_eq!(spec.a, 255, "Spec '{}' must be fully opaque", spec.filename);
        }
    }

    /// `generate_texture` must produce an image with the correct dimensions.
    #[test]
    fn test_generate_texture_dimensions() {
        let spec = &TERRAIN_SPECS[0]; // ground
        let img = generate_texture(spec);
        assert_eq!(img.width(), IMAGE_WIDTH);
        assert_eq!(img.height(), IMAGE_HEIGHT);
    }

    /// `generate_texture` must produce an image whose pixels are within bounds.
    #[test]
    fn test_generate_texture_pixels_in_bounds() {
        for spec in TERRAIN_SPECS {
            let img = generate_texture(spec);
            for pixel in img.pixels() {
                let Rgba([r, g, b, a]) = *pixel;
                // Alpha is always the spec value
                assert_eq!(a, spec.a, "Alpha mismatch in '{}'", spec.filename);
                // Channels must be within NOISE_RANGE of the base
                let r_diff = (r as i32 - spec.r as i32).abs();
                let g_diff = (g as i32 - spec.g as i32).abs();
                let b_diff = (b as i32 - spec.b as i32).abs();
                assert!(
                    r_diff <= NOISE_RANGE,
                    "Red channel noise too large in '{}': diff={r_diff}",
                    spec.filename
                );
                assert!(
                    g_diff <= NOISE_RANGE,
                    "Green channel noise too large in '{}': diff={g_diff}",
                    spec.filename
                );
                assert!(
                    b_diff <= NOISE_RANGE,
                    "Blue channel noise too large in '{}': diff={b_diff}",
                    spec.filename
                );
            }
        }
    }

    /// `generate_texture` must be deterministic: two calls with the same spec
    /// must produce identical pixel data.
    #[test]
    fn test_generate_texture_is_deterministic() {
        for spec in TERRAIN_SPECS {
            let img1 = generate_texture(spec);
            let img2 = generate_texture(spec);
            assert_eq!(
                img1.as_raw(),
                img2.as_raw(),
                "generate_texture is not deterministic for '{}'",
                spec.filename
            );
        }
    }

    /// Two specs with different seeds must produce different pixel data.
    #[test]
    fn test_generate_texture_different_seeds_differ() {
        let img1 = generate_texture(&TERRAIN_SPECS[0]); // ground
        let img2 = generate_texture(&TERRAIN_SPECS[1]); // grass
                                                        // At least one pixel must differ (almost certainly many will)
        assert_ne!(
            img1.as_raw(),
            img2.as_raw(),
            "Ground and grass textures should not be identical"
        );
    }
}
