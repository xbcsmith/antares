// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Generate tiling 512×512 RGBA8 foliage detail textures for tree species.
//!
//! Produces six PNG files (Oak, Pine, Birch, Willow, Palm, Shrub) and writes
//! them to both:
//! - `assets/textures/trees/foliage_{species}.png`
//! - `campaigns/tutorial/assets/textures/trees/foliage_{species}.png`

use image::{ImageBuffer, Rgba};
use std::f32::consts::PI;
use std::path::Path;
use thiserror::Error;

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors that can occur while generating or saving foliage textures.
#[derive(Error, Debug)]
pub enum GeneratorError {
    /// An I/O error occurred (e.g. directory creation or file copy).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The `image` crate failed to encode or write the PNG.
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
}

// ── PRNG: SplitMix64 ─────────────────────────────────────────────────────────

/// A fast, high-quality non-cryptographic pseudo-random number generator
/// based on the SplitMix64 algorithm.
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Creates a new `SplitMix64` seeded with the given value.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Advances the state and returns the next pseudo-random `u64`.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Returns a pseudo-random `f32` in the range `[0, 1)`.
    pub fn f32(&mut self) -> f32 {
        // Use upper 24 bits for a value in [0, 1)
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Returns a pseudo-random `f32` in the range `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.f32() * (hi - lo)
    }
}

// ── Drawing primitive ─────────────────────────────────────────────────────────

/// Draws a rotated, optionally ragged ellipse onto the image with wrap-around.
///
/// # Arguments
/// * `img`     – Target 512×512 RGBA image buffer.
/// * `rng`     – PRNG used for edge raggedness and per-pixel jitter.
/// * `cx`      – Centre X coordinate (may be negative; wraps modulo 512).
/// * `cy`      – Centre Y coordinate (may be negative; wraps modulo 512).
/// * `semi_a`  – Semi-axis along the local X direction (px).
/// * `semi_b`  – Semi-axis along the local Y direction (px).
/// * `angle`   – Rotation of the ellipse (radians).
/// * `base`    – Base sRGB colour `[r, g, b]` in `[0.0, 1.0]`.
#[allow(clippy::too_many_arguments)]
pub fn draw_ellipse(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    rng: &mut SplitMix64,
    cx: i32,
    cy: i32,
    semi_a: f32,
    semi_b: f32,
    angle: f32,
    base: [f32; 3],
) {
    let (sin_a, cos_a) = angle.sin_cos();
    // A rotated ellipse can extend to max(semi_a, semi_b) in both axes.
    let bound = semi_a.max(semi_b).ceil() as i32 + 1;
    let threshold = 1.0 + rng.range(-0.12, 0.04);

    for dy in -bound..=bound {
        for dx in -bound..=bound {
            let fdx = dx as f32;
            let fdy = dy as f32;
            let rx = fdx * cos_a + fdy * sin_a;
            let ry = -fdx * sin_a + fdy * cos_a;
            let d2 = (rx / semi_a).powi(2) + (ry / semi_b).powi(2);
            if d2 <= threshold {
                let jitter = rng.range(-0.05, 0.05);
                let d = d2.sqrt().min(1.0);
                let edge = if d > 0.85 {
                    1.0 - (d - 0.85) * 1.33
                } else {
                    1.0_f32
                };

                let wx = (cx + dx).rem_euclid(512) as u32;
                let wy = (cy + dy).rem_euclid(512) as u32;

                let r = ((base[0] * (1.0 + jitter) * edge.max(0.1)).clamp(0.0, 1.0) * 255.0) as u8;
                let g = ((base[1] * (1.0 + jitter) * edge.max(0.1)).clamp(0.0, 1.0) * 255.0) as u8;
                let b = ((base[2] * (1.0 + jitter) * edge.max(0.1)).clamp(0.0, 1.0) * 255.0) as u8;

                img.put_pixel(wx, wy, Rgba([r, g, b, 255]));
            }
        }
    }
}

// ── Per-species stamp functions ───────────────────────────────────────────────

/// Stamps a 5-lobe oak leaf centred at `(cx, cy)` with the given `size`.
///
/// Draws a rounded centre plus five lobes radiating outward.
pub fn stamp_oak(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    rng: &mut SplitMix64,
    cx: i32,
    cy: i32,
    size: f32,
    base: [f32; 3],
) {
    let half = size * 0.5;
    let angle = rng.range(0.0, 2.0 * PI);
    // Centre body
    draw_ellipse(img, rng, cx, cy, half * 0.40, half * 0.32, angle, base);
    // Five lobes
    for i in 0..5 {
        let lobe_angle = angle + i as f32 * (2.0 * PI / 5.0);
        let lx = cx + (lobe_angle.cos() * half * 0.40) as i32;
        let ly = cy + (lobe_angle.sin() * half * 0.40) as i32;
        draw_ellipse(img, rng, lx, ly, half * 0.28, half * 0.22, lobe_angle, base);
    }
}

/// Stamps a thin pine needle stroke centred at `(cx, cy)` with the given `size`.
///
/// Draws a single narrow ellipse (3 px wide) to simulate a needle.
pub fn stamp_pine(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    rng: &mut SplitMix64,
    cx: i32,
    cy: i32,
    size: f32,
    base: [f32; 3],
) {
    let angle = rng.range(0.0, 2.0 * PI);
    draw_ellipse(img, rng, cx, cy, size * 0.5, 1.5, angle, base);
}

/// Stamps an ovate birch leaf centred at `(cx, cy)` with the given `size`.
///
/// Uses an elongated ellipse to represent the ovate leaf shape.
pub fn stamp_birch(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    rng: &mut SplitMix64,
    cx: i32,
    cy: i32,
    size: f32,
    base: [f32; 3],
) {
    let angle = rng.range(0.0, 2.0 * PI);
    draw_ellipse(img, rng, cx, cy, size * 0.5, size * 0.40, angle, base);
}

/// Stamps a narrow lanceolate willow strip centred at `(cx, cy)`.
///
/// The strip length is given by `length`; width is randomly chosen in 3–5 px.
pub fn stamp_willow(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    rng: &mut SplitMix64,
    cx: i32,
    cy: i32,
    length: f32,
    base: [f32; 3],
) {
    let width = rng.range(3.0, 5.0);
    let angle = rng.range(0.0, 2.0 * PI);
    draw_ellipse(img, rng, cx, cy, length * 0.5, width, angle, base);
}

/// Stamps a palm frond centred at `(cx, cy)` with the given `length`.
///
/// Draws a central rib plus 6 pairs of leaflets spread along the rib.
pub fn stamp_palm(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    rng: &mut SplitMix64,
    cx: i32,
    cy: i32,
    length: f32,
    base: [f32; 3],
) {
    let angle = rng.range(0.0, 2.0 * PI);
    let (sin_a, cos_a) = angle.sin_cos();
    // Central rib
    draw_ellipse(img, rng, cx, cy, length * 0.5, 1.5, angle, base);

    // 6 pairs of leaflets
    for i in 0..6 {
        let t = (i as f32 / 6.0 - 0.5) * length;
        let lx = cx + (t * cos_a) as i32;
        let ly = cy + (t * sin_a) as i32;

        // Left leaflet
        let la = angle + PI * 0.389;
        let llx = lx + (length * 0.18 * 0.5 * la.cos()) as i32;
        let lly = ly + (length * 0.18 * 0.5 * la.sin()) as i32;
        draw_ellipse(img, rng, llx, lly, length * 0.18, 4.0, la, base);

        // Right leaflet
        let ra = angle - PI * 0.389;
        let rlx = lx + (length * 0.18 * 0.5 * ra.cos()) as i32;
        let rly = ly + (length * 0.18 * 0.5 * ra.sin()) as i32;
        draw_ellipse(img, rng, rlx, rly, length * 0.18, 4.0, ra, base);
    }
}

/// Stamps a near-circular shrub leaf centred at `(cx, cy)` with the given `size`.
pub fn stamp_shrub(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    rng: &mut SplitMix64,
    cx: i32,
    cy: i32,
    size: f32,
    base: [f32; 3],
) {
    let angle = rng.range(0.0, 2.0 * PI);
    draw_ellipse(img, rng, cx, cy, size * 0.5, size * 0.48, angle, base);
}

// ── Generation helpers ────────────────────────────────────────────────────────

/// Generates a 512×512 RGBA8 foliage texture for one species.
///
/// # Arguments
/// * `seed`      – PRNG seed (unique per species).
/// * `count`     – Number of leaf/needle stamps to draw.
/// * `min_size`  – Minimum stamp size in pixels.
/// * `max_size`  – Maximum stamp size in pixels.
/// * `base`      – Base sRGB colour `[r, g, b]` in `[0.0, 1.0]`.
/// * `stamp`     – Species-specific stamp closure.
///
/// # Returns
/// A fully populated `ImageBuffer<Rgba<u8>, Vec<u8>>`.
pub fn generate_species(
    seed: u64,
    count: usize,
    min_size: f32,
    max_size: f32,
    base: [f32; 3],
    stamp: impl Fn(&mut ImageBuffer<Rgba<u8>, Vec<u8>>, &mut SplitMix64, i32, i32, f32, [f32; 3]),
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(512, 512, Rgba([0u8, 0, 0, 0]));
    let mut rng = SplitMix64::new(seed);

    for _ in 0..count {
        let cx = rng.range(0.0, 512.0) as i32;
        let cy = rng.range(0.0, 512.0) as i32;
        let size = rng.range(min_size, max_size);
        stamp(&mut img, &mut rng, cx, cy, size, base);
    }

    img
}

/// Validates that a generated image meets the output specification.
///
/// Panics with a descriptive message if any assertion fails.
///
/// # Assertions
/// - Width and height are both 512.
/// - Opaque pixel coverage (alpha ≥ 128) is between 40 % and 85 %.
/// - Mean luminance of opaque pixels is between 0.65 and 0.90.
pub fn assert_spec(img: &ImageBuffer<Rgba<u8>, Vec<u8>>, species: &str) {
    assert_eq!(img.width(), 512, "{species}: width must be 512");
    assert_eq!(img.height(), 512, "{species}: height must be 512");

    let total = (512u64 * 512) as f64;
    let mut opaque_count = 0u64;
    let mut lum_sum = 0.0f64;

    for pixel in img.pixels() {
        let Rgba([r, g, b, a]) = *pixel;
        if a >= 128 {
            opaque_count += 1;
            lum_sum += (r as f64 / 255.0 + g as f64 / 255.0 + b as f64 / 255.0) / 3.0;
        }
    }

    let coverage = opaque_count as f64 / total;
    assert!(
        (0.40..=0.85).contains(&coverage),
        "{species}: opaque coverage {coverage:.3} is outside [0.40, 0.85]"
    );

    let mean_lum = if opaque_count > 0 {
        lum_sum / opaque_count as f64
    } else {
        0.0
    };
    assert!(
        (0.65..=0.90).contains(&mean_lum),
        "{species}: mean luminance {mean_lum:.3} of opaque pixels is outside [0.65, 0.90]"
    );
}

/// Saves a generated image to `path`, creating parent directories as needed.
///
/// # Errors
/// Returns `GeneratorError::Io` if directory creation fails, or
/// `GeneratorError::Image` if the PNG cannot be written.
pub fn save_image(img: &ImageBuffer<Rgba<u8>, Vec<u8>>, path: &Path) -> Result<(), GeneratorError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    img.save(path)?;
    Ok(())
}

// ── Species descriptor ────────────────────────────────────────────────────────

/// Parameters that fully describe one foliage species for generation.
struct SpeciesParams {
    /// Short lowercase species name used in the output filename.
    name: &'static str,
    /// PRNG seed (unique per species).
    seed: u64,
    /// Number of leaf/needle stamps to draw.
    count: usize,
    /// Minimum stamp size in pixels.
    min_size: f32,
    /// Maximum stamp size in pixels.
    max_size: f32,
    /// Base sRGB colour `[r, g, b]` in `[0.0, 1.0]`.
    base: [f32; 3],
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() -> Result<(), GeneratorError> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let species = [
        SpeciesParams {
            name: "oak",
            seed: 101,
            count: 220,
            min_size: 40.0,
            max_size: 70.0,
            base: [0.80, 0.84, 0.72],
        },
        SpeciesParams {
            name: "pine",
            seed: 102,
            count: 1400,
            min_size: 30.0,
            max_size: 55.0,
            base: [0.74, 0.80, 0.74],
        },
        SpeciesParams {
            name: "birch",
            seed: 103,
            count: 300,
            min_size: 28.0,
            max_size: 48.0,
            base: [0.84, 0.87, 0.74],
        },
        SpeciesParams {
            name: "willow",
            seed: 104,
            count: 500,
            min_size: 50.0,
            max_size: 90.0,
            base: [0.80, 0.84, 0.70],
        },
        SpeciesParams {
            name: "palm",
            seed: 105,
            count: 90,
            min_size: 90.0,
            max_size: 140.0,
            base: [0.78, 0.84, 0.70],
        },
        SpeciesParams {
            name: "shrub",
            seed: 106,
            count: 420,
            min_size: 18.0,
            max_size: 34.0,
            base: [0.80, 0.83, 0.72],
        },
    ];

    for sp in &species {
        let SpeciesParams {
            name,
            seed,
            count,
            min_size,
            max_size,
            base,
        } = *sp;
        let img = match name {
            "oak" => generate_species(seed, count, min_size, max_size, base, stamp_oak),
            "pine" => generate_species(seed, count, min_size, max_size, base, stamp_pine),
            "birch" => generate_species(seed, count, min_size, max_size, base, stamp_birch),
            "willow" => generate_species(seed, count, min_size, max_size, base, stamp_willow),
            "palm" => generate_species(seed, count, min_size, max_size, base, stamp_palm),
            "shrub" => generate_species(seed, count, min_size, max_size, base, stamp_shrub),
            _ => unreachable!("unknown species: {name}"),
        };

        assert_spec(&img, name);

        let filename = format!("foliage_{name}.png");

        let assets_path = Path::new(manifest_dir)
            .join("assets/textures/trees")
            .join(&filename);
        save_image(&img, &assets_path)?;
        println!("Generated: {}", assets_path.display());

        let campaign_path = Path::new(manifest_dir)
            .join("campaigns/tutorial/assets/textures/trees")
            .join(&filename);
        if let Some(parent) = campaign_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&assets_path, &campaign_path)?;
        println!("Generated: {}", campaign_path.display());
    }

    Ok(())
}
