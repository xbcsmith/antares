// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Binary that generates placeholder terrain and grass textures for the Antares game.
//!
//! # Terrain textures
//!
//! Each terrain texture is a 64×64 RGBA PNG with a solid base colour and a
//! deterministic per-pixel noise overlay of ±10 per channel.  The noise seed
//! is fixed so that the output is identical across runs (reproducible builds).
//!
//! # Grass blade texture
//!
//! A single 32×128 RGBA PNG representing one vertical grass blade with a
//! transparent background.  The blade occupies the centre 16 pixels and fades
//! from fully opaque (alpha = 255) at the base to semi-transparent (alpha ≈ 64)
//! at the tip.  Base colour: RGBA (60, 130, 50, 255).
//!
//! # Usage
//!
//! ```text
//! cargo run --bin generate_terrain_textures
//! ```
//!
//! Output files are written to:
//! - `assets/textures/terrain/` for terrain PNGs
//! - `assets/textures/grass/`   for grass PNGs
//!
//! Both directories are relative to `CARGO_MANIFEST_DIR`.

use image::{ImageBuffer, Rgba};
use std::path::PathBuf;

// ──────────────────────────────────────────────────────────────────────────────
// Grass blade texture constants
// ──────────────────────────────────────────────────────────────────────────────

/// Width of the grass blade texture in pixels.
const GRASS_BLADE_WIDTH: u32 = 32;
/// Height of the grass blade texture in pixels.
const GRASS_BLADE_HEIGHT: u32 = 128;
/// Width of the visible blade strip in pixels (centred in the image).
const BLADE_STRIP_WIDTH: u32 = 16;
/// Base red channel for the grass blade colour.
const GRASS_BLADE_R: u8 = 60;
/// Base green channel for the grass blade colour.
const GRASS_BLADE_G: u8 = 130;
/// Base blue channel for the grass blade colour.
const GRASS_BLADE_B: u8 = 50;
/// Alpha at the very base of the blade (fully opaque).
const GRASS_BLADE_ALPHA_BASE: u8 = 255;
/// Alpha at the very tip of the blade (semi-transparent).
const GRASS_BLADE_ALPHA_TIP: u8 = 64;
/// Deterministic noise seed for the grass blade texture.
const GRASS_BLADE_SEED: u64 = 0xA1B2_C3D4_E5F6_0718;

// ──────────────────────────────────────────────────────────────────────────────
// Tree texture constants
// ──────────────────────────────────────────────────────────────────────────────

/// Width of bark texture in pixels.
const BARK_WIDTH: u32 = 64;
/// Height of bark texture in pixels.
const BARK_HEIGHT: u32 = 128;
/// Bark base brown colour — red channel.
const BARK_R: u8 = 90;
/// Bark base brown colour — green channel.
const BARK_G: u8 = 60;
/// Bark base brown colour — blue channel.
const BARK_B: u8 = 35;
/// Bark noise seed.
const BARK_SEED: u64 = 0xB1C2_D3E4_F5A6_0718;

/// Width of foliage textures in pixels.
const FOLIAGE_WIDTH: u32 = 128;
/// Height of foliage textures in pixels.
const FOLIAGE_HEIGHT: u32 = 128;
/// Width/height of shrub foliage texture (smaller).
const SHRUB_FOLIAGE_SIZE: u32 = 64;
/// Width of the pine foliage texture.
const PINE_FOLIAGE_WIDTH: u32 = 64;
/// Height of the pine foliage texture.
const PINE_FOLIAGE_HEIGHT: u32 = 128;
/// Alpha cutoff threshold — pixels with radius fraction above this are transparent.
const FOLIAGE_ALPHA_OUTER: u8 = 0;
/// Alpha for fully opaque foliage pixels.
const FOLIAGE_ALPHA_INNER: u8 = 240;

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

/// Generates a 64×128 RGBA bark texture for tree trunks.
///
/// The texture is fully opaque (alpha = 255) with a brown base colour and
/// deterministic vertical-grain noise applied per pixel.
pub fn generate_bark_texture() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(BARK_WIDTH, BARK_HEIGHT);
    let mut state = BARK_SEED;

    for y in 0..BARK_HEIGHT {
        // Vertical grain: modulate brightness slightly per row
        let grain = ((y % 5) as i32) - 2; // -2..+2 grain variation
        for x in 0..BARK_WIDTH {
            let r_base = clamp_channel(BARK_R as i32 + grain);
            let g_base = clamp_channel(BARK_G as i32 + grain);
            let (r, s1) = apply_noise(r_base, state);
            let (g, s2) = apply_noise(g_base, s1);
            let (b, s3) = apply_noise(BARK_B, s2);
            state = s3;
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
    img
}

/// Generates an RGBA foliage texture with a circular alpha mask.
///
/// Pixels within the inscribed circle of the image are opaque, pixels outside
/// are transparent. A small amount of noise is applied to the colour channels
/// inside the circle for a natural look.
///
/// # Arguments
///
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `base_r` - Base red channel for foliage colour
/// * `base_g` - Base green channel for foliage colour
/// * `base_b` - Base blue channel for foliage colour
/// * `seed` - Deterministic noise seed
pub fn generate_foliage_texture(
    width: u32,
    height: u32,
    base_r: u8,
    base_g: u8,
    base_b: u8,
    seed: u64,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(width, height);
    let mut state = seed;

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let radius = (cx.min(cy)) * 0.85; // Slightly inset for soft edge

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= radius {
                // Inside circle: apply colour with noise
                let (r, s1) = apply_noise(base_r, state);
                let (g, s2) = apply_noise(base_g, s1);
                let (b, s3) = apply_noise(base_b, s2);
                state = s3;
                // Soft falloff near edge
                let alpha = if dist > radius * 0.8 {
                    let t = (dist - radius * 0.8) / (radius * 0.2);
                    (FOLIAGE_ALPHA_INNER as f32 * (1.0 - t)) as u8
                } else {
                    FOLIAGE_ALPHA_INNER
                };
                img.put_pixel(x, y, Rgba([r, g, b, alpha]));
            } else {
                // Advance state for determinism even for transparent pixels
                state = xorshift64(state);
                img.put_pixel(x, y, Rgba([0, 0, 0, FOLIAGE_ALPHA_OUTER]));
            }
        }
    }
    img
}

/// Generates all tree texture assets and writes them to `assets/textures/trees/`.
///
/// # Textures generated
///
/// | File                | Dimensions | Content                          |
/// |---------------------|------------|----------------------------------|
/// | `bark.png`          | 64×128     | Brown vertical-grain bark        |
/// | `foliage_oak.png`   | 128×128    | Rounded leaf cluster             |
/// | `foliage_pine.png`  | 64×128     | Vertical needle cluster          |
/// | `foliage_birch.png` | 128×128    | Small round leaves               |
/// | `foliage_willow.png`| 128×128    | Drooping curtain of leaves       |
/// | `foliage_palm.png`  | 128×128    | Fan-shaped palm fronds           |
/// | `foliage_shrub.png` | 64×64      | Dense bush silhouette            |
pub fn generate_tree_textures(trees_dir: &std::path::Path) {
    std::fs::create_dir_all(trees_dir).unwrap_or_else(|e| {
        eprintln!(
            "ERROR: Could not create directory '{}': {e}",
            trees_dir.display()
        );
        std::process::exit(1);
    });

    // Bark texture (fully opaque)
    let bark = generate_bark_texture();
    save_texture(trees_dir, "bark.png", &bark);

    // Oak foliage — medium green, 128×128 circular
    let oak = generate_foliage_texture(
        FOLIAGE_WIDTH,
        FOLIAGE_HEIGHT,
        45,
        110,
        35,
        0xC1D2_E3F4_A506_1728,
    );
    save_texture(trees_dir, "foliage_oak.png", &oak);

    // Pine foliage — dark green, 64×128 (tall narrow)
    let pine = generate_foliage_texture(
        PINE_FOLIAGE_WIDTH,
        PINE_FOLIAGE_HEIGHT,
        20,
        75,
        25,
        0xD2E3_F4A5_0617_2839,
    );
    save_texture(trees_dir, "foliage_pine.png", &pine);

    // Birch foliage — light green, 128×128 circular
    let birch = generate_foliage_texture(
        FOLIAGE_WIDTH,
        FOLIAGE_HEIGHT,
        65,
        130,
        55,
        0xE3F4_A506_1728_394A,
    );
    save_texture(trees_dir, "foliage_birch.png", &birch);

    // Willow foliage — medium-dark green, 128×128 circular
    let willow = generate_foliage_texture(
        FOLIAGE_WIDTH,
        FOLIAGE_HEIGHT,
        50,
        100,
        40,
        0xF4A5_0617_2839_4A5B,
    );
    save_texture(trees_dir, "foliage_willow.png", &willow);

    // Palm foliage — tropical green, 128×128 circular
    let palm = generate_foliage_texture(
        FOLIAGE_WIDTH,
        FOLIAGE_HEIGHT,
        55,
        125,
        30,
        0xA506_1728_394A_5B6C,
    );
    save_texture(trees_dir, "foliage_palm.png", &palm);

    // Shrub foliage — dark bush green, 64×64
    let shrub = generate_foliage_texture(
        SHRUB_FOLIAGE_SIZE,
        SHRUB_FOLIAGE_SIZE,
        40,
        90,
        30,
        0x0617_2839_4A5B_6C7D,
    );
    save_texture(trees_dir, "foliage_shrub.png", &shrub);

    println!("Done. 7 tree textures written to: {}", trees_dir.display());
}

/// Saves an image buffer to a file, exiting on error.
fn save_texture(dir: &std::path::Path, filename: &str, img: &ImageBuffer<Rgba<u8>, Vec<u8>>) {
    let path = dir.join(filename);
    match img.save(&path) {
        Ok(()) => println!("  ✓  {filename}"),
        Err(e) => {
            eprintln!("ERROR: Failed to write '{filename}': {e}");
            std::process::exit(1);
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Entry point
// ──────────────────────────────────────────────────────────────────────────────

// ──────────────────────────────────────────────────────────────────────────────
// Grass blade texture generation
// ──────────────────────────────────────────────────────────────────────────────

/// Generates a 32×128 RGBA grass blade texture with a transparent background.
///
/// The blade occupies the centre [`BLADE_STRIP_WIDTH`] columns and fades from
/// [`GRASS_BLADE_ALPHA_BASE`] at the bottom row to [`GRASS_BLADE_ALPHA_TIP`]
/// at the top row.  A small amount of per-pixel noise is applied to the RGB
/// channels inside the blade strip so the result has a natural look.
pub fn generate_grass_blade_texture() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(GRASS_BLADE_WIDTH, GRASS_BLADE_HEIGHT);
    let mut state = GRASS_BLADE_SEED;

    let blade_left = (GRASS_BLADE_WIDTH - BLADE_STRIP_WIDTH) / 2;
    let blade_right = blade_left + BLADE_STRIP_WIDTH;

    for y in 0..GRASS_BLADE_HEIGHT {
        // Normalised position from bottom (0.0) to top (1.0).
        let t = y as f32 / (GRASS_BLADE_HEIGHT - 1) as f32;
        // Alpha lerps from base (bottom) to tip (top).
        let alpha_f = GRASS_BLADE_ALPHA_BASE as f32
            + t * (GRASS_BLADE_ALPHA_TIP as f32 - GRASS_BLADE_ALPHA_BASE as f32);
        let alpha = alpha_f.round() as u8;

        for x in 0..GRASS_BLADE_WIDTH {
            if x >= blade_left && x < blade_right {
                // Inside the blade strip: apply noise to RGB.
                let (r, s1) = apply_noise(GRASS_BLADE_R, state);
                let (g, s2) = apply_noise(GRASS_BLADE_G, s1);
                let (b, s3) = apply_noise(GRASS_BLADE_B, s2);
                state = s3;
                img.put_pixel(x, y, Rgba([r, g, b, alpha]));
            } else {
                // Outside the blade strip: fully transparent.
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }

    img
}

// ──────────────────────────────────────────────────────────────────────────────
// Entry point
// ──────────────────────────────────────────────────────────────────────────────

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    // ── Terrain textures ──────────────────────────────────────────────────────
    let terrain_dir = PathBuf::from(manifest_dir).join("assets/textures/terrain");

    std::fs::create_dir_all(&terrain_dir).unwrap_or_else(|e| {
        eprintln!(
            "ERROR: Could not create directory '{}': {e}",
            terrain_dir.display()
        );
        std::process::exit(1);
    });

    println!("Writing terrain textures to: {}", terrain_dir.display());

    for spec in TERRAIN_SPECS {
        let path = terrain_dir.join(spec.filename);
        let img = generate_texture(spec);

        match img.save(&path) {
            Ok(()) => println!("  ✓  {}", spec.filename),
            Err(e) => {
                eprintln!("ERROR: Failed to write '{}': {e}", path.display());
                std::process::exit(1);
            }
        }
    }

    println!("Done. {} terrain textures written.", TERRAIN_SPECS.len());

    // ── Grass blade texture ───────────────────────────────────────────────────
    let grass_dir = PathBuf::from(manifest_dir).join("assets/textures/grass");

    std::fs::create_dir_all(&grass_dir).unwrap_or_else(|e| {
        eprintln!(
            "ERROR: Could not create directory '{}': {e}",
            grass_dir.display()
        );
        std::process::exit(1);
    });

    println!("Writing grass textures to: {}", grass_dir.display());

    let blade_path = grass_dir.join("grass_blade.png");
    let blade_img = generate_grass_blade_texture();

    match blade_img.save(&blade_path) {
        Ok(()) => println!("  ✓  grass_blade.png"),
        Err(e) => {
            eprintln!("ERROR: Failed to write 'grass_blade.png': {e}");
            std::process::exit(1);
        }
    }

    println!("Done. 1 grass texture written.");

    // ── Tree textures ─────────────────────────────────────────────────────────
    let trees_dir = PathBuf::from(manifest_dir).join("assets/textures/trees");
    println!("Writing tree textures to: {}", trees_dir.display());
    generate_tree_textures(&trees_dir);
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

    // ── Grass blade texture tests ─────────────────────────────────────────────

    /// `generate_grass_blade_texture` must produce an image with the correct
    /// dimensions (32×128).
    #[test]
    fn test_generate_grass_blade_texture_dimensions() {
        let img = generate_grass_blade_texture();
        assert_eq!(img.width(), GRASS_BLADE_WIDTH);
        assert_eq!(img.height(), GRASS_BLADE_HEIGHT);
    }

    /// Pixels outside the blade strip must be fully transparent (alpha == 0).
    #[test]
    fn test_generate_grass_blade_texture_outside_strip_transparent() {
        let img = generate_grass_blade_texture();
        let blade_left = (GRASS_BLADE_WIDTH - BLADE_STRIP_WIDTH) / 2;
        let blade_right = blade_left + BLADE_STRIP_WIDTH;

        for y in 0..GRASS_BLADE_HEIGHT {
            for x in 0..GRASS_BLADE_WIDTH {
                if x < blade_left || x >= blade_right {
                    let pixel = img.get_pixel(x, y);
                    assert_eq!(
                        pixel[3], 0,
                        "Expected alpha=0 outside blade strip at ({x},{y}), got {}",
                        pixel[3]
                    );
                }
            }
        }
    }

    /// Pixels inside the blade strip must have alpha within the expected range
    /// [`GRASS_BLADE_ALPHA_TIP`, `GRASS_BLADE_ALPHA_BASE`].
    #[test]
    fn test_generate_grass_blade_texture_inside_strip_alpha_range() {
        let img = generate_grass_blade_texture();
        let blade_left = (GRASS_BLADE_WIDTH - BLADE_STRIP_WIDTH) / 2;
        let blade_right = blade_left + BLADE_STRIP_WIDTH;

        for y in 0..GRASS_BLADE_HEIGHT {
            for x in blade_left..blade_right {
                let pixel = img.get_pixel(x, y);
                let alpha = pixel[3];
                assert!(
                    alpha >= GRASS_BLADE_ALPHA_TIP,
                    "Alpha {alpha} at ({x},{y}) is below minimum [{GRASS_BLADE_ALPHA_TIP}]"
                );
            }
        }
    }

    /// Verifies that the alpha gradient runs from fully-opaque at the **top**
    /// of the image (`y = 0`) to semi-transparent at the **bottom**
    /// (`y = HEIGHT - 1`).
    ///
    /// The UV generation in `create_grass_blade_mesh` maps `v = 0` (base of
    /// blade) to the top of the texture and `v = 1` (tip of blade) to the
    /// bottom, so:
    ///
    /// - `y = 0`          → `t = 0` → alpha = `GRASS_BLADE_ALPHA_BASE` (255, fully opaque)
    /// - `y = HEIGHT - 1` → `t = 1` → alpha = `GRASS_BLADE_ALPHA_TIP`  (64, semi-transparent)
    #[test]
    fn test_generate_grass_blade_texture_alpha_gradient_direction() {
        let img = generate_grass_blade_texture();
        let blade_left = (GRASS_BLADE_WIDTH - BLADE_STRIP_WIDTH) / 2;

        // y=0 is the top row of the image → blade base → must be near ALPHA_BASE (255).
        let top_row_alpha = img.get_pixel(blade_left, 0)[3];
        // y=HEIGHT-1 is the bottom row of the image → blade tip → must be near ALPHA_TIP (64).
        let bottom_row_alpha = img.get_pixel(blade_left, GRASS_BLADE_HEIGHT - 1)[3];

        assert!(
            top_row_alpha > bottom_row_alpha,
            "Top-row alpha ({top_row_alpha}) should be greater than bottom-row alpha \
             ({bottom_row_alpha}): base of blade (v=0) must be more opaque than tip (v=1)"
        );
    }

    /// `generate_grass_blade_texture` must be deterministic.
    #[test]
    fn test_generate_grass_blade_texture_is_deterministic() {
        let img1 = generate_grass_blade_texture();
        let img2 = generate_grass_blade_texture();
        assert_eq!(
            img1.as_raw(),
            img2.as_raw(),
            "generate_grass_blade_texture must be deterministic"
        );
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

    // ==================== Tree Texture Tests ====================

    /// Tests that generate_bark_texture returns correct dimensions
    #[test]
    fn test_generate_bark_texture_dimensions() {
        let img = generate_bark_texture();
        assert_eq!(img.width(), BARK_WIDTH, "Bark texture width mismatch");
        assert_eq!(img.height(), BARK_HEIGHT, "Bark texture height mismatch");
    }

    /// Tests that bark texture pixels are fully opaque
    #[test]
    fn test_generate_bark_texture_fully_opaque() {
        let img = generate_bark_texture();
        for (_x, _y, pixel) in img.enumerate_pixels() {
            assert_eq!(pixel[3], 255, "All bark pixels must be fully opaque");
        }
    }

    /// Tests that generate_bark_texture is deterministic
    #[test]
    fn test_generate_bark_texture_is_deterministic() {
        let img1 = generate_bark_texture();
        let img2 = generate_bark_texture();
        assert_eq!(
            img1.into_raw(),
            img2.into_raw(),
            "Bark texture must be deterministic"
        );
    }

    /// Tests that generate_foliage_texture returns correct dimensions
    #[test]
    fn test_generate_foliage_texture_dimensions() {
        let img = generate_foliage_texture(
            FOLIAGE_WIDTH,
            FOLIAGE_HEIGHT,
            45,
            110,
            35,
            0xABCD_1234_5678_9EF0,
        );
        assert_eq!(img.width(), FOLIAGE_WIDTH);
        assert_eq!(img.height(), FOLIAGE_HEIGHT);
    }

    /// Tests that foliage texture centre pixel is opaque
    #[test]
    fn test_generate_foliage_texture_centre_is_opaque() {
        let img = generate_foliage_texture(
            FOLIAGE_WIDTH,
            FOLIAGE_HEIGHT,
            45,
            110,
            35,
            0xABCD_1234_5678_9EF0,
        );
        let cx = FOLIAGE_WIDTH / 2;
        let cy = FOLIAGE_HEIGHT / 2;
        let pixel = img.get_pixel(cx, cy);
        assert!(
            pixel[3] > 0,
            "Centre pixel of foliage texture should be opaque"
        );
    }

    /// Tests that foliage texture corners are transparent
    #[test]
    fn test_generate_foliage_texture_corners_transparent() {
        let img = generate_foliage_texture(
            FOLIAGE_WIDTH,
            FOLIAGE_HEIGHT,
            45,
            110,
            35,
            0xABCD_1234_5678_9EF0,
        );
        // Corners should be outside the circular mask
        let corners = [
            (0, 0),
            (FOLIAGE_WIDTH - 1, 0),
            (0, FOLIAGE_HEIGHT - 1),
            (FOLIAGE_WIDTH - 1, FOLIAGE_HEIGHT - 1),
        ];
        for (x, y) in corners {
            let pixel = img.get_pixel(x, y);
            assert_eq!(
                pixel[3], FOLIAGE_ALPHA_OUTER,
                "Corner pixel ({x},{y}) should be transparent"
            );
        }
    }

    /// Tests that generate_foliage_texture is deterministic
    #[test]
    fn test_generate_foliage_texture_is_deterministic() {
        let seed = 0xABCD_1234_5678_9EF0_u64;
        let img1 = generate_foliage_texture(FOLIAGE_WIDTH, FOLIAGE_HEIGHT, 45, 110, 35, seed);
        let img2 = generate_foliage_texture(FOLIAGE_WIDTH, FOLIAGE_HEIGHT, 45, 110, 35, seed);
        assert_eq!(
            img1.into_raw(),
            img2.into_raw(),
            "Foliage texture must be deterministic"
        );
    }
}
