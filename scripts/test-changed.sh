#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
# SPDX-License-Identifier: Apache-2.0
#
# scripts/test-changed.sh
#
# Run tests only for packages that contain changed files. Designed for faster local
# feedback loops by testing only the packages touched by your changes. Falls back
# to the full test suite when workspace-level files change or when package detection
# is ambiguous.
#
# Features:
#  - Detect changed files (staged, unstaged, or a git range provided as first arg)
#  - Map changed files to workspace packages using `cargo metadata`
#  - Run per-package tests (lib, all-targets, doctests) in parallel (throttled)
#  - If `cargo-nextest` is installed, prefer it for faster parallel execution
#  - If workspace-level files are affected (Cargo.toml, CI, scripts), runs full tests
#
# Usage:
#   ./scripts/test-changed.sh
#   ./scripts/test-changed.sh origin/main...HEAD
#
# Notes:
#  - `jq` is recommended to parse `cargo metadata`. The script falls back to a
#    minimal JSON sniff if `jq` is missing.
#  - For CI, pass the full git range (e.g. `origin/main...HEAD`) to the script
#    to run tests for packages affected by the PR.
#  - For local development the script first checks staged files, then unstaged.
#
# Exit codes:
#  - 0: All selected tests passed
#  - 1: One or more selected tests failed or script encountered an error
#
# Environment:
#  - CONCURRENCY: optional override for number of package test jobs run in parallel
#  - JOBS: optional override for number of cargo build jobs (defaults to CPU cores)
#
# Minimal dependencies: git, cargo. jq and cargo-nextest are optional accelerators.

set -euo pipefail
IFS=$'\n\t'

# repo root (absolute)
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo '.')"
cd "$REPO_ROOT" || exit 1

# Determine number of build jobs
if [ -n "${JOBS:-}" ]; then
  JOBS="$JOBS"
else
  if command -v nproc >/dev/null 2>&1; then
    JOBS="$(nproc)"
  else
    JOBS="$(sysctl -n hw.logicalcpu 2>/dev/null || echo 4)"
  fi
fi

# Concurrency for test jobs started by this script
CONCURRENCY="${CONCURRENCY:-4}"

log() {
  printf "[%s] %s\n" "$(date +'%Y-%m-%d %H:%M:%S')" "$*"
}

usage() {
  cat <<EOF
Usage: $0 [<git-range>]
Examples:
  $0                    # local: run tests for staged/uncommitted changes
  $0 origin/main...HEAD # CI: run tests just for PR delta

Environment:
  JOBS (optional)       # - cargo build jobs (default: CPU cores)
  CONCURRENCY (optional)# - how many per-package test jobs to run concurrently
EOF
  exit 1
}

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
  usage
fi

# Collect the list of changed files
if [ $# -ge 1 ]; then
  RANGE="$1"
  log "Checking git range: $RANGE"
  CHANGED_FILES_RAW="$(git diff --name-only "$RANGE" || true)"
else
  # prefer staged changes, otherwise fall back to unstaged
  CHANGED_FILES_RAW="$(git diff --name-only --cached || true)"
  if [ -z "$CHANGED_FILES_RAW" ]; then
    CHANGED_FILES_RAW="$(git diff --name-only || true)"
  fi
fi

# Normalize into an array
CHANGED_FILES=()
if [ -n "$CHANGED_FILES_RAW" ]; then
  while IFS= read -r line; do
    line="${line#"${line%%[![:space:]]*}"}"    # trim leading whitespace
    line="${line%"${line##*[![:space:]]}"}"    # trim trailing whitespace
    [ -n "$line" ] && CHANGED_FILES+=("$line")
  done <<<"$CHANGED_FILES_RAW"
fi

if [ ${#CHANGED_FILES[@]} -eq 0 ]; then
  log "No changed files detected. Running a quick crate-level smoke test (lib: antares)."
  cargo test -p antares --lib -j "$JOBS"
  exit $?
fi

log "Detected changed files:"
for f in "${CHANGED_FILES[@]}"; do
  log "  - $f"
done

# If a workspace-level change is present run full tests
for f in "${CHANGED_FILES[@]}"; do
  case "$f" in
    Cargo.toml|Cargo.lock|.github/*|.circleci/*|.gitlab-ci.yml|scripts/*|.github/*)
      log "Workspace-level change ($f) detected. Running full workspace test suite..."
      if command -v cargo-nextest >/dev/null 2>&1; then
        cargo nextest run --workspace --all-features -j "$JOBS"
        exit $?
      else
        cargo test --workspace --all-features -j "$JOBS"
        exit $?
      fi
      ;;
  esac
done

# Gather package metadata
log "Reading cargo metadata..."
CARGO_METADATA_JSON="$(cargo metadata --no-deps --format-version 1 2>/dev/null || true)"
if [ -z "$CARGO_METADATA_JSON" ]; then
  log "Failed to read cargo metadata. Running full workspace test suite as fallback."
  cargo test --workspace --all-features -j "$JOBS"
  exit $?
fi

# Build array of 'pkg|manifest_path'
PKG_ENTRIES=()
if command -v jq >/dev/null 2>&1; then
  while IFS=$'\t' read -r name manifest; do
    PKG_ENTRIES+=("$name|$manifest")
  done < <(echo "$CARGO_METADATA_JSON" | jq -r '.packages[] | "\(.name)\t\(.manifest_path)"')
else
  # minimal fallback parsing if jq not present (less robust)
  while IFS= read -r line; do
    if [[ "$line" =~ \"name\" ]]; then
      pkgname="$(echo "$line" | sed -n 's/.*\"name\": \"\([^\"]*\)\".*/\1/p')"
    fi
    if [[ "$line" =~ \"manifest_path\" ]]; then
      manifestpath="$(echo "$line" | sed -n 's/.*\"manifest_path\": \"\([^\"]*\)\".*/\1/p')"
      if [ -n "${pkgname:-}" -a -n "${manifestpath:-}" ]; then
        PKG_ENTRIES+=("${pkgname}|${manifestpath}")
        pkgname=""
        manifestpath=""
      fi
    fi
  done <<< "$(echo "$CARGO_METADATA_JSON")"
fi

# Build mapping package -> relative package directory
declare -A PKG_TO_DIR
for ent in "${PKG_ENTRIES[@]}"; do
  pkg="${ent%%|*}"
  manifest="${ent#*|}"
  pkgdir="$(dirname "$manifest")"
  if [[ "$pkgdir" == "$REPO_ROOT"* ]]; then
    rel="${pkgdir#$REPO_ROOT/}"
  else
    # manifest may already be relative, normalize
    rel="$pkgdir"
  fi
  PKG_TO_DIR["$pkg"]="$rel"
done

# Map changed files to packages
declare -A PACKAGES_TO_TEST
for f in "${CHANGED_FILES[@]}"; do
  # we assume f is repo-relative; if absolute, strip repo prefix
  f_rel="$f"
  if [[ "$f_rel" == /* ]] && [[ "$f_rel" == "$REPO_ROOT"* ]]; then
    f_rel="${f_rel#$REPO_ROOT/}"
  fi

  for pkg in "${!PKG_TO_DIR[@]}"; do
    pkgdir="${PKG_TO_DIR[$pkg]}"
    # If the package root is the repo root (i.e. '.'), match paths under src/ or top-level
    if [ -z "$pkgdir" ] || [ "$pkgdir" = "." ]; then
      # Consider files under src/, data/, or top-level targets as touching the root crate
      case "$f_rel" in
        src/*|data/*|assets/*|Cargo.toml)
          PACKAGES_TO_TEST["$pkg"]=1
          ;;
      esac
    else
      if [[ "$f_rel" == "$pkgdir"* ]]; then
        PACKAGES_TO_TEST["$pkg"]=1
      fi
    fi
  done
done

# If we found no packages, fallback to a smoke test
if [ ${#PACKAGES_TO_TEST[@]} -eq 0 ]; then
  log "No packages detected from changed files, running smoke test for 'antares' crate"
  cargo test -p antares --lib -j "$JOBS"
  exit $?
fi

log "Packages with changes: ${!PACKAGES_TO_TEST[@]}"

# Run tests per package (parallel, throttled)
# Each package: run all-targets (lib/bins/tests) and doctests (separately)
PROCS=()
FAILED=0
for pkg in "${!PACKAGES_TO_TEST[@]}"; do
  if command -v cargo-nextest >/dev/null 2>&1; then
    log "Starting cargo-nextest for package: $pkg"
    # cargo-nextest supports limiting threads but not always a global 'job' count,
    # rely on nextest's internal scheduler with concurrency caps
    (cargo nextest run -p "$pkg" --run-threads "$CONCURRENCY" || exit 1) &
    PROCS+=("$!")
  else
    log "Starting cargo test (all targets + doctests) for package: $pkg"
    (
      set -o pipefail
      cargo test -p "$pkg" --all-targets --all-features -j "$JOBS" || exit 1
      cargo test -p "$pkg" --doc --all-features -j "$JOBS" || exit 1
    ) &
    PROCS+=("$!")
  fi

  # throttle concurrency
  while [ "$(jobs -rp | wc -l)" -ge "$CONCURRENCY" ]; do
    sleep 0.4
  done
done

log "Waiting for per-package test jobs to complete..."
for pid in "${PROCS[@]}"; do
  wait "$pid" || FAILED=1
done

if [ "$FAILED" -ne 0 ]; then
  log "Some package tests failed"
  exit 1
else
  log "All selected package tests passed"
  exit 0
fi
