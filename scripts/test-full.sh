#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
# SPDX-License-Identifier: Apache-2.0
#
# scripts/test-full.sh
#
# Run the full test suite for the repository with optional compilation caching and
# a faster test harness. The script:
#
#  - Optionally enables a compilation cache (sccache) if available
#  - Prefers `cargo-nextest` if installed, otherwise runs `cargo test`
#  - Honours the number of parallel jobs (-j / --jobs)
#  - Provides simple toggles to skip cache or nextest
#
# Usage:
#   ./scripts/test-full.sh               # fastest full run using nextest & sccache (if available)
#   ./scripts/test-full.sh -j 8          # run with 8 build jobs
#   ./scripts/test-full.sh --no-cache    # run without compile cache
#   ./scripts/test-full.sh --no-nextest  # force cargo test fallback
#   ./scripts/test-full.sh --release     # run release build tests (slower compile, faster run)
#   ./scripts/test-full.sh --check       # skip tests, run cargo check only (fast)
#
# Environment:
#  - JOBS: override detected number-of-cores per default
#  - CONCURRENCY: override concurrency for nextest (when used)
#  - RUSTC_WRAPPER: will be set to sccache automatically if available (unless --no-cache)
#
# Exit codes:
#  - 0: all tests/checks passed
#  - non-zero: one or more tests failed or script encountered an error
#

set -euo pipefail
IFS=$'\n\t'

# Helpers
log() {
    printf '[%s] %s\n' "$(date +"%Y-%m-%d %H:%M:%S")" "$*"
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [options]

Options:
  -j, --jobs <N>       Set number of build jobs (default: CPU cores)
  --no-cache           Don't start or use sccache (or rustc wrapper)
  --no-nextest         Don't try cargo-nextest; use cargo test
  --release            Run tests in release profile (cargo test --release)
  --check              Run cargo check for all targets instead of tests
  -q, --quiet          Reduce output verbosity (pass --quiet to cargo if supported)
  -h, --help           Show this help text

Examples:
  ./scripts/test-full.sh -j 8 --no-cache
  ./scripts/test-full.sh --release
  ./scripts/test-full.sh origin/main...HEAD    # note: this script ignores ranges; use test-changed.sh for ranges
EOF
    exit 1
}

# Default behaviour / auto-detect parallelism
if command -v nproc >/dev/null 2>&1; then
    DEFAULT_JOBS=$(nproc)
else
    DEFAULT_JOBS=$(sysctl -n hw.logicalcpu 2>/dev/null || echo 4)
fi

JOBS="$DEFAULT_JOBS"
NO_CACHE=0
NO_NEXTEST=0
RELEASE=0
CHECK_ONLY=0
QUIET=0

# Parse args
# Support both short and long options
while [ $# -gt 0 ]; do
    case "$1" in
        -j|--jobs)
            if [ -n "${2:-}" ] && [[ "$2" =~ ^[0-9]+$ ]]; then
                JOBS="$2"
                shift 2
            else
                echo "Error: --jobs requires an integer argument"
                exit 2
            fi
            ;;
        --no-cache)
            NO_CACHE=1
            shift
            ;;
        --no-nextest)
            NO_NEXTEST=1
            shift
            ;;
        --release)
            RELEASE=1
            shift
            ;;
        --check)
            CHECK_ONLY=1
            shift
            ;;
        -q|--quiet)
            QUIET=1
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# Repo root
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo '.')"
cd "$REPO_ROOT" || exit 1

# Print effective configuration
log "Full test runner"
log " - repo root: $REPO_ROOT"
log " - jobs: $JOBS"
log " - concurrency (nextest): ${CONCURRENCY:-4}"
log " - release: $RELEASE"
log " - check_only: $CHECK_ONLY"
log " - use cache: $([ $NO_CACHE -eq 1 ] && echo \"no\" || echo \"yes\")"
log " - prefer nextest: $([ $NO_NEXTEST -eq 1 ] && echo \"no\" || echo \"yes\")"
log " - quiet: $([ $QUIET -eq 1 ] && echo \"yes\" || echo \"no\")"

# Setup optional sccache
SCCACHE_WRAPPER=""
if [ "$NO_CACHE" -eq 0 ]; then
    if command -v sccache >/dev/null 2>&1; then
        SCCACHE_WRAPPER="$(command -v sccache)"
        export RUSTC_WRAPPER="$SCCACHE_WRAPPER"
        # Try to start server (no-op if already running)
        sccache --start-server >/dev/null 2>&1 || true
        log "Using sccache at $SCCACHE_WRAPPER (RUSTC_WRAPPER)"
        # Print sccache stats if available at the end
    else
        log "sccache not found; proceeding without compile cache (recommended: install sccache)"
        SCCACHE_WRAPPER=""
    fi
else
    log "Skipping sccache (explicit --no-cache)"
fi

# Prepare common command bits
NEXTTEST_AVAILABLE=0
if [ "$NO_NEXTEST" -eq 0 ] && command -v cargo-nextest >/dev/null 2>&1; then
    NEXTTEST_AVAILABLE=1
fi

# Prefer cargo-nextest (if installed and not suppressed)
if [ "$NEXTTEST_AVAILABLE" -eq 1 ]; then
    log "cargo-nextest found: will use cargo nextest for faster test runs"
fi

# Build base command depending on presence of 'release' flag
CARGO_CMD_COMMON=(cargo)
if [ "$RELEASE" -eq 1 ]; then
    CARGO_FLAGS+=(--release)
fi

# Simple check only option
if [ "$CHECK_ONLY" -eq 1 ]; then
    log "Running a full 'cargo check' for the workspace with all features..."
    if [ "$NEXTTEST_AVAILABLE" -eq 1 ]; then
        # nextest doesn't have a check alias; fallback to cargo check
        cargo check --all-targets --all-features -j "$JOBS"
    else
        cargo check --all-targets --all-features -j "$JOBS"
    fi
    log "cargo check completed"
    exit $?
fi

# Run the full test suite (prefer nextest)
if [ "$NEXTTEST_AVAILABLE" -eq 1 ]; then
    # Compose nextest options
    NEXTTEST_CMD=(cargo nextest run --workspace --all-features)
    # Respect release
    if [ "$RELEASE" -eq 1 ]; then
        NEXTTEST_CMD+=(--release)
    fi
    # Add parallelism
    NEXTTEST_CMD+=(-j "$JOBS")
    # Optionally quiet
    if [ "$QUIET" -eq 1 ]; then
        NEXTTEST_CMD+=(--quiet)
    fi

    log "Running full test suite with cargo nextest..."
    printf 'Running: %s\n' "${NEXTTEST_CMD[*]}"
    if "${NEXTTEST_CMD[@]}"; then
        log "nextest completed successfully"
    else
        log "nextest failed; falling back to cargo test to ensure full diagnostics"
        if cargo test --workspace --all-features -j "$JOBS"; then
            log "cargo test fallback succeeded"
        else
            log "Both nextest and cargo test failed. Exiting with non-zero status."
            exit 1
        fi
    fi
else
    # Run cargo test directly
    TEST_CMD=(cargo test --workspace --all-features -j "$JOBS")
    if [ "$RELEASE" -eq 1 ]; then
        TEST_CMD+=(--release)
    fi
    if [ "$QUIET" -eq 1 ]; then
        TEST_CMD+=(--quiet)
    fi
    log "Running full test suite with cargo test..."
    printf 'Running: %s\n' "${TEST_CMD[*]}"
    if "${TEST_CMD[@]}"; then
        log "cargo test succeeded"
    else
        log "cargo test failed"
        exit 1
    fi
fi

# Optionally print sccache summary to help tune environment (if we used it)
if [ -n "$SCCACHE_WRAPPER" ] && command -v sccache >/dev/null 2>&1; then
    log "sccache stats (top-level):"
    sccache --show-stats || true
fi

log "Test run completed successfully"
exit 0
