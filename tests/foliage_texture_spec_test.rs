// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Verifies that the committed foliage detail textures meet the output
//! specification defined in the vegetation visual improvement plan.
//!
//! Each of the six PNGs in `assets/textures/trees/` must be:
//! - 512 × 512 RGBA8
//! - Opaque coverage (alpha ≥ 128): 40 – 85 % of all pixels
//! - Mean luminance of opaque pixels (simple RGB mean): 0.65 – 0.90
//! - Seamlessly tileable (edge strips carry non-zero opaque pixels,
//!   confirming that stamps wrapped across the canvas boundaries)

use image::GenericImageView;
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct SpecResult {
    width: u32,
    height: u32,
    coverage: f64,
    mean_lum: f64,
    edge_opaque: bool,
}

fn inspect(path: &PathBuf) -> SpecResult {
    let img = image::open(path).unwrap_or_else(|e| panic!("cannot open {}: {e}", path.display()));
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();

    let total = (w * h) as f64;
    let mut opaque = 0u64;
    let mut lum_sum = 0.0f64;

    // Opaque pixels in a 4-px-wide strip along each edge (tests wrap-around coverage)
    let strip = 4u32;
    let mut edge_opaque = false;

    for (x, y, px) in rgba.enumerate_pixels() {
        let [r, g, b, a] = px.0;
        if a >= 128 {
            opaque += 1;
            lum_sum += (r as f64 / 255.0 + g as f64 / 255.0 + b as f64 / 255.0) / 3.0;
            if x < strip
                || x >= w.saturating_sub(strip)
                || y < strip
                || y >= h.saturating_sub(strip)
            {
                edge_opaque = true;
            }
        }
    }

    SpecResult {
        width: w,
        height: h,
        coverage: opaque as f64 / total,
        mean_lum: if opaque > 0 {
            lum_sum / opaque as f64
        } else {
            0.0
        },
        edge_opaque,
    }
}

fn check_species(name: &str) {
    let path = manifest_dir()
        .join("assets/textures/trees")
        .join(format!("foliage_{name}.png"));

    assert!(
        path.exists(),
        "foliage_{name}.png not found at {}",
        path.display()
    );

    let r = inspect(&path);

    assert_eq!(
        r.width, 512,
        "foliage_{name}: expected width 512, got {}",
        r.width
    );
    assert_eq!(
        r.height, 512,
        "foliage_{name}: expected height 512, got {}",
        r.height
    );

    assert!(
        (0.40..=0.85).contains(&r.coverage),
        "foliage_{name}: opaque coverage {:.3} outside [0.40, 0.85]",
        r.coverage
    );

    assert!(
        (0.65..=0.90).contains(&r.mean_lum),
        "foliage_{name}: mean luminance {:.3} outside [0.65, 0.90]",
        r.mean_lum
    );

    assert!(
        r.edge_opaque,
        "foliage_{name}: no opaque pixels in the 4-px edge strip — stamps may not be wrapping"
    );
}

#[test]
fn foliage_spec_oak() {
    check_species("oak");
}

#[test]
fn foliage_spec_pine() {
    check_species("pine");
}

#[test]
fn foliage_spec_birch() {
    check_species("birch");
}

#[test]
fn foliage_spec_willow() {
    check_species("willow");
}

#[test]
fn foliage_spec_palm() {
    check_species("palm");
}

#[test]
fn foliage_spec_shrub() {
    check_species("shrub");
}

#[test]
fn foliage_spec_campaign_tutorial_matches_assets() {
    // The two copies (assets/ and campaigns/tutorial/) must be byte-identical.
    for name in ["oak", "pine", "birch", "willow", "palm", "shrub"] {
        let src = manifest_dir()
            .join("assets/textures/trees")
            .join(format!("foliage_{name}.png"));
        let dst = manifest_dir()
            .join("campaigns/tutorial/assets/textures/trees")
            .join(format!("foliage_{name}.png"));

        assert!(
            src.exists(),
            "foliage_{name}.png missing from assets/textures/trees/"
        );
        assert!(
            dst.exists(),
            "foliage_{name}.png missing from campaigns/tutorial/assets/textures/trees/"
        );

        let src_bytes =
            std::fs::read(&src).unwrap_or_else(|e| panic!("cannot read {}: {e}", src.display()));
        let dst_bytes =
            std::fs::read(&dst).unwrap_or_else(|e| panic!("cannot read {}: {e}", dst.display()));

        assert_eq!(
            src_bytes, dst_bytes,
            "foliage_{name}.png: assets/ and campaigns/tutorial/ copies differ"
        );
    }
}
