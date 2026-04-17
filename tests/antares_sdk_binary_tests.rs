// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the `antares-sdk` binary.
//!
//! These tests invoke the compiled `antares-sdk` executable directly via
//! [`std::process::Command`] and verify that:
//!
//! 1. `antares-sdk --help` exits with status 0 and lists all expected
//!    subcommands in its output.
//! 2. Every subcommand responds to `--help` with status 0 and does not
//!    print anything to stderr.
//!
//! # Prerequisites
//!
//! Cargo builds the `antares-sdk` binary before running integration tests,
//! so these tests do not require a separate `cargo build` step. The binary
//! path is resolved via the `CARGO_BIN_EXE_antares_sdk` environment variable
//! that Cargo injects automatically.
//!
//! # Running
//!
//! ```bash
//! cargo nextest run --test antares_sdk_binary_tests
//! # or
//! cargo test --test antares_sdk_binary_tests
//! ```

use std::process::Command;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Returns a [`Command`] pre-configured to run the `antares-sdk` binary.
///
/// Resolution order:
///
/// 1. `CARGO_BIN_EXE_antares_sdk` — set by Cargo at compile time when
///    building integration tests alongside binaries (`cargo test` /
///    `cargo nextest run`).
/// 2. `<CARGO_MANIFEST_DIR>/target/debug/antares-sdk` — derived from the
///    always-available compile-time `CARGO_MANIFEST_DIR` macro.  This path
///    exists whenever `cargo nextest run` (or `cargo test`) is used, because
///    Cargo builds all binary targets before executing tests.
///
/// Using `option_env!` (compile-time optional lookup) rather than `env!`
/// avoids a hard compile error during `cargo check --all-targets`, where
/// Cargo does not build binaries and therefore does not inject
/// `CARGO_BIN_EXE_*` variables.
fn sdk_cmd() -> Command {
    let bin = option_env!("CARGO_BIN_EXE_antares_sdk")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            // Fallback: construct the path from the package root.
            // `cargo nextest run` and `cargo test` always build all binary
            // targets before running tests, so this path is guaranteed to
            // exist at test-execution time.
            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            let bin_name = if cfg!(windows) {
                "antares-sdk.exe"
            } else {
                "antares-sdk"
            };
            std::path::Path::new(manifest_dir)
                .join("target")
                .join("debug")
                .join(bin_name)
        });
    Command::new(&bin)
}

// ─────────────────────────────────────────────────────────────────────────────
// Top-level --help
// ─────────────────────────────────────────────────────────────────────────────

/// `antares-sdk --help` must exit with status 0 and list every registered
/// subcommand in its stdout output.
#[test]
fn test_antares_sdk_help_exits_zero() {
    let output = sdk_cmd()
        .arg("--help")
        .output()
        .expect("failed to spawn antares-sdk --help");

    assert!(
        output.status.success(),
        "antares-sdk --help must exit with status 0; got: {}",
        output.status
    );
}

/// `antares-sdk --help` output must contain the word "names".
#[test]
fn test_antares_sdk_help_lists_names_subcommand() {
    let output = sdk_cmd()
        .arg("--help")
        .output()
        .expect("failed to spawn antares-sdk --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("names"),
        "help output must mention 'names' subcommand; got:\n{stdout}"
    );
}

/// `antares-sdk --help` output must contain the word "campaign".
#[test]
fn test_antares_sdk_help_lists_campaign_subcommand() {
    let output = sdk_cmd()
        .arg("--help")
        .output()
        .expect("failed to spawn antares-sdk --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("campaign"),
        "help output must mention 'campaign' subcommand; got:\n{stdout}"
    );
}

/// `antares-sdk --help` output must contain the word "class".
#[test]
fn test_antares_sdk_help_lists_class_subcommand() {
    let output = sdk_cmd()
        .arg("--help")
        .output()
        .expect("failed to spawn antares-sdk --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("class"),
        "help output must mention 'class' subcommand; got:\n{stdout}"
    );
}

/// `antares-sdk --help` output must contain the word "item".
#[test]
fn test_antares_sdk_help_lists_item_subcommand() {
    let output = sdk_cmd()
        .arg("--help")
        .output()
        .expect("failed to spawn antares-sdk --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("item"),
        "help output must mention 'item' subcommand; got:\n{stdout}"
    );
}

/// `antares-sdk --help` output must contain the word "map".
#[test]
fn test_antares_sdk_help_lists_map_subcommand() {
    let output = sdk_cmd()
        .arg("--help")
        .output()
        .expect("failed to spawn antares-sdk --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("map"),
        "help output must mention 'map' subcommand; got:\n{stdout}"
    );
}

/// `antares-sdk --help` output must contain the word "race".
#[test]
fn test_antares_sdk_help_lists_race_subcommand() {
    let output = sdk_cmd()
        .arg("--help")
        .output()
        .expect("failed to spawn antares-sdk --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("race"),
        "help output must mention 'race' subcommand; got:\n{stdout}"
    );
}

/// `antares-sdk --help` output must contain the word "textures".
#[test]
fn test_antares_sdk_help_lists_textures_subcommand() {
    let output = sdk_cmd()
        .arg("--help")
        .output()
        .expect("failed to spawn antares-sdk --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("textures"),
        "help output must mention 'textures' subcommand; got:\n{stdout}"
    );
}

/// `antares-sdk --help` must produce no output on stderr.
#[test]
fn test_antares_sdk_help_produces_no_stderr() {
    let output = sdk_cmd()
        .arg("--help")
        .output()
        .expect("failed to spawn antares-sdk --help");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "antares-sdk --help must not write to stderr; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-subcommand --help (4.5 requirement: each subcommand with --help must
// exit without error)
// ─────────────────────────────────────────────────────────────────────────────

/// `antares-sdk names --help` must exit with status 0.
#[test]
fn test_names_subcommand_help_exits_zero() {
    let output = sdk_cmd()
        .args(["names", "--help"])
        .output()
        .expect("failed to spawn antares-sdk names --help");

    assert!(
        output.status.success(),
        "antares-sdk names --help must exit 0; got: {}",
        output.status
    );
}

/// `antares-sdk names --help` must not write to stderr.
#[test]
fn test_names_subcommand_help_no_stderr() {
    let output = sdk_cmd()
        .args(["names", "--help"])
        .output()
        .expect("failed to spawn antares-sdk names --help");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "names --help must not write to stderr; got:\n{stderr}"
    );
}

/// `antares-sdk campaign --help` must exit with status 0.
#[test]
fn test_campaign_subcommand_help_exits_zero() {
    let output = sdk_cmd()
        .args(["campaign", "--help"])
        .output()
        .expect("failed to spawn antares-sdk campaign --help");

    assert!(
        output.status.success(),
        "antares-sdk campaign --help must exit 0; got: {}",
        output.status
    );
}

/// `antares-sdk campaign validate --help` must exit with status 0.
#[test]
fn test_campaign_validate_help_exits_zero() {
    let output = sdk_cmd()
        .args(["campaign", "validate", "--help"])
        .output()
        .expect("failed to spawn antares-sdk campaign validate --help");

    assert!(
        output.status.success(),
        "antares-sdk campaign validate --help must exit 0; got: {}",
        output.status
    );
}

/// `antares-sdk class --help` must exit with status 0.
#[test]
fn test_class_subcommand_help_exits_zero() {
    let output = sdk_cmd()
        .args(["class", "--help"])
        .output()
        .expect("failed to spawn antares-sdk class --help");

    assert!(
        output.status.success(),
        "antares-sdk class --help must exit 0; got: {}",
        output.status
    );
}

/// `antares-sdk class --help` output must mention the `--campaign` flag.
#[test]
fn test_class_subcommand_help_mentions_campaign_flag() {
    let output = sdk_cmd()
        .args(["class", "--help"])
        .output()
        .expect("failed to spawn antares-sdk class --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--campaign"),
        "class --help must document --campaign flag; got:\n{stdout}"
    );
}

/// `antares-sdk race --help` must exit with status 0.
#[test]
fn test_race_subcommand_help_exits_zero() {
    let output = sdk_cmd()
        .args(["race", "--help"])
        .output()
        .expect("failed to spawn antares-sdk race --help");

    assert!(
        output.status.success(),
        "antares-sdk race --help must exit 0; got: {}",
        output.status
    );
}

/// `antares-sdk race --help` output must mention the `--campaign` flag.
#[test]
fn test_race_subcommand_help_mentions_campaign_flag() {
    let output = sdk_cmd()
        .args(["race", "--help"])
        .output()
        .expect("failed to spawn antares-sdk race --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--campaign"),
        "race --help must document --campaign flag; got:\n{stdout}"
    );
}

/// `antares-sdk item --help` must exit with status 0.
#[test]
fn test_item_subcommand_help_exits_zero() {
    let output = sdk_cmd()
        .args(["item", "--help"])
        .output()
        .expect("failed to spawn antares-sdk item --help");

    assert!(
        output.status.success(),
        "antares-sdk item --help must exit 0; got: {}",
        output.status
    );
}

/// `antares-sdk item --help` output must mention the `--campaign` flag.
#[test]
fn test_item_subcommand_help_mentions_campaign_flag() {
    let output = sdk_cmd()
        .args(["item", "--help"])
        .output()
        .expect("failed to spawn antares-sdk item --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--campaign"),
        "item --help must document --campaign flag; got:\n{stdout}"
    );
}

/// `antares-sdk map --help` must exit with status 0.
#[test]
fn test_map_subcommand_help_exits_zero() {
    let output = sdk_cmd()
        .args(["map", "--help"])
        .output()
        .expect("failed to spawn antares-sdk map --help");

    assert!(
        output.status.success(),
        "antares-sdk map --help must exit 0; got: {}",
        output.status
    );
}

/// `antares-sdk map validate --help` must exit with status 0.
#[test]
fn test_map_validate_help_exits_zero() {
    let output = sdk_cmd()
        .args(["map", "validate", "--help"])
        .output()
        .expect("failed to spawn antares-sdk map validate --help");

    assert!(
        output.status.success(),
        "antares-sdk map validate --help must exit 0; got: {}",
        output.status
    );
}

/// `antares-sdk map build --help` must exit with status 0.
#[test]
fn test_map_build_help_exits_zero() {
    let output = sdk_cmd()
        .args(["map", "build", "--help"])
        .output()
        .expect("failed to spawn antares-sdk map build --help");

    assert!(
        output.status.success(),
        "antares-sdk map build --help must exit 0; got: {}",
        output.status
    );
}

/// `antares-sdk textures --help` must exit with status 0.
#[test]
fn test_textures_subcommand_help_exits_zero() {
    let output = sdk_cmd()
        .args(["textures", "--help"])
        .output()
        .expect("failed to spawn antares-sdk textures --help");

    assert!(
        output.status.success(),
        "antares-sdk textures --help must exit 0; got: {}",
        output.status
    );
}

/// `antares-sdk textures generate --help` must exit with status 0.
#[test]
fn test_textures_generate_help_exits_zero() {
    let output = sdk_cmd()
        .args(["textures", "generate", "--help"])
        .output()
        .expect("failed to spawn antares-sdk textures generate --help");

    assert!(
        output.status.success(),
        "antares-sdk textures generate --help must exit 0; got: {}",
        output.status
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Top-level flag smoke tests
// ─────────────────────────────────────────────────────────────────────────────

/// `antares-sdk --verbose --help` must exit with status 0.
///
/// Verifies that the `--verbose` flag is accepted by the top-level parser
/// even when combined with `--help`.
#[test]
fn test_verbose_flag_with_help_exits_zero() {
    let output = sdk_cmd()
        .args(["--verbose", "--help"])
        .output()
        .expect("failed to spawn antares-sdk --verbose --help");

    assert!(
        output.status.success(),
        "antares-sdk --verbose --help must exit 0; got: {}",
        output.status
    );
}

/// `antares-sdk --quiet --help` must exit with status 0.
///
/// Verifies that the `--quiet` flag is accepted by the top-level parser
/// even when combined with `--help`.
#[test]
fn test_quiet_flag_with_help_exits_zero() {
    let output = sdk_cmd()
        .args(["--quiet", "--help"])
        .output()
        .expect("failed to spawn antares-sdk --quiet --help");

    assert!(
        output.status.success(),
        "antares-sdk --quiet --help must exit 0; got: {}",
        output.status
    );
}

/// `antares-sdk --help` output must mention `--verbose`.
#[test]
fn test_antares_sdk_help_mentions_verbose_flag() {
    let output = sdk_cmd()
        .arg("--help")
        .output()
        .expect("failed to spawn antares-sdk --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--verbose"),
        "top-level help must document --verbose; got:\n{stdout}"
    );
}

/// `antares-sdk --help` output must mention `--quiet`.
#[test]
fn test_antares_sdk_help_mentions_quiet_flag() {
    let output = sdk_cmd()
        .arg("--help")
        .output()
        .expect("failed to spawn antares-sdk --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--quiet"),
        "top-level help must document --quiet; got:\n{stdout}"
    );
}
