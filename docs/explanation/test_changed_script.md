antares/docs/explanation/test_changed_script.md#L1-200
# test-changed.sh — selective test-run helper

This document explains the `scripts/test-changed.sh` helper (introduced as an experiment)
used to accelerate local development by running tests only for the workspace packages
containing changes. Use it during development for quick feedback and still run a full
test pass (including heavy doctests and optional features) in CI or before merging.

Note: this script is intentionally conservative — it prefers safety and correctness:
if the script cannot map changed files reliably, or a workspace-level file changed,
it will fall back to running the full test suite.

---

## Goals

- Run tests only for packages containing changed files (fast feedback loop).
- Run doctests and normal tests for those packages while not running unrelated tests.
- Keep a safe fallback: if you changed a workspace-level file (Cargo.toml, CI), run the full suite.
- Prefer `cargo-nextest` for faster parallel tests if available; otherwise use `cargo test`.
- Provide a simple drop-in script for local development and CI workflows.

---

## Where to find it

- Script: `scripts/test-changed.sh`
- CLI doc: `docs/explanation/test_changed_script.md`
- Script behavior summary (high-level):
  - Detect changed files (staged or a git range).
  - Map changed files to package roots using `cargo metadata`.
  - For each affected package, run:
    - `cargo test -p <pkg> --all-targets --all-features`
    - `cargo test -p <pkg> --doc --all-features`
  - If `cargo nextest` is installed, it prefers `cargo nextest run -p <pkg>`
  - Run tests in parallel, throttled by `CONCURRENCY` (default 4).
  - Uses `JOBS` (default to number of logical CPU cores) for cargo job parallelism.

---

## Dependencies & recommended tools

- Required:
  - `git`
  - `cargo`
- Recommended:
  - `jq` (for robust parsing of `cargo metadata`)
  - `cargo-nextest` (`cargo nextest run`) to parallelize and schedule tests efficiently
  - `sccache` to speed subsequent rebuilds (recommended for local & CI)
- Optional:
  - `nproc` or `sysctl` for automatic detection of CPU cores
  - `realpath` (fallbacks in script but recommended for robust path normalization)

---

## Usage

### Default (local)
- Run against staged/uncommitted changes:
```/dev/null/commands.sh#L1-5
# Run tests for changed packages (staged first, then unstable)
./scripts/test-changed.sh
```

### CI (use a git range)
- Run for a range (example: PR delta from `origin/main`):
```/dev/null/commands.sh#L1-3
# CI-style: run tests only for files changed in the PR
./scripts/test-changed.sh origin/main...HEAD
```

### Environment knobs
- `JOBS` — number of parallel compilation jobs used by `cargo` (default: CPU cores).
- `CONCURRENCY` — number of package test jobs to run concurrently (default: 4).
- Example:
```/dev/null/commands.sh#L1-4
# Use a specific parallelism level
export JOBS=16
export CONCURRENCY=4
./scripts/test-changed.sh
```

---

## What it runs exactly

For each mapped package, the script runs (in a best-effort pattern):

- If `cargo nextest` is available:
  - `cargo nextest run -p <package> --run-threads <CONCURRENCY>`
- Otherwise:
  - `cargo test -p <package> --all-targets --all-features -j $JOBS`
  - `cargo test -p <package> --doc --all-features -j $JOBS`

This combination ensures:
- Unit & integration tests are run (`--all-targets`)
- Doctests for the package are run (`--doc`)
- Feature flags are included (`--all-features` as requested earlier)

If the script detects a workspace-level change (workspace-level `Cargo.toml`, CI config,
top-level `scripts/` or `.github/`), it will run full workspace tests with `--all-features`.

---

## Why this is useful

- Running `cargo test --all-features` on the full workspace can be slow:
  - Tests compile each package and doctests; multiple heavy dependencies (e.g., `bevy`) compile once.
  - If you only changed a single package, this script avoids re-running unrelated tests.
- The script enables quick local development feedback:
  - You change a file in `sdk/campaign_builder`, run the script, and handle failing tests with much shorter time spent.
- When combined with `sccache` and `cargo-nextest`, you get dramatic speedups.

---

## Limitations & edge cases

- Mapping to packages:
  - The script matches a changed file path to a package root via manifest path.
  - If your change is a global change (like a shared type), the script may not detect the full impact:
    - Example: you change `src/domain` types used across crates; the script may only test the package containing the changed file, not every consumer.
  - If in doubt, run full tests (`./scripts/test-full.sh`) or `cargo test --workspace --all-features`.
- Binary-level doctests:
  - Doctests are compiled per example; even if doctest is small, compile cost can still be significant.
  - The script still reduces test runs by limiting the set of compiled packages.
- `jq` missing:
  - The script attempts a minimal fallback without `jq`. If `jq` is not available, the heuristic parsing may be less reliable.
- Special file changes:
  - If you modify `data/` files that are referenced by multiple packages, those multiple packages may not be detected. The script attempts to map `data/` and `assets/` change heuristics to the root package; however full workspace-level change may be required.
- OS support:
  - The script uses `nproc` on Linux or `sysctl` on macOS to detect CPU cores; on other systems fallback default values are used.

---

## Recommended best practices

- For local dev:
  1. Use `sccache` to minimize repeated compile time.
  2. Use `cargo nextest` for best parallel scheduling:
```/dev/null/commands.sh#L1-4
cargo install cargo-nextest --locked
export RUSTC_WRAPPER=$(which sccache)
./scripts/test-changed.sh
```
- For full runs (pre-commit / CI), run the all-features full test suite:
```/dev/null/commands.sh#L1-2
# Pre-commit or CI gating
cargo nextest run --workspace --all-features
```
- If you modify `Cargo.toml`, `.github/*`, or other workspace-level files, the script will automatically run a full test suite.

---

## CI integration examples

Below is a recommended pattern for CI (e.g., GitHub Actions):

- Fast job for PRs:
  - Run `scripts/test-changed.sh origin/main...HEAD`
  - Use sccache and `nextest` to accelerate
- Full verification job (for main branch or merge):
  - Run `cargo nextest run --workspace --all-features` (or `cargo test --workspace --all-features`)
  - Cache cargo registry & `sccache` server artifacts between runs

Example (conceptual YAML snippet):
```/dev/null/commands.sh#L1-18
# Pseudocode steps for GitHub Actions job:
- name: Setup Rust
  uses: actions-rs/toolchain@v1
  with: stable

- name: Cache cargo & sccache
  uses: actions/cache@v3
  with: ...

- name: Install cargo-nextest & sccache
  run: |
    cargo install cargo-nextest --locked
    cargo install sccache --locked
    sccache --start-server

- name: Run changed tests
  run: |
    # CI uses origin/main...HEAD or a PR ref
    ./scripts/test-changed.sh origin/main...HEAD
```

For final merge checks, schedule a `full-test` job:
```/dev/null/commands.sh#L1-3
# Full verification (merge / protect main branch)
cargo nextest run --workspace --all-features -j $JOBS
```

---

## Adding script to your workflow (local pre-push & pre-commit)

- Pre-commit hook: minimal checks on staged files only
- Pre-push hook: runs `scripts/test-changed.sh` (recommended).
- Example `pre-push` hook:
```/dev/null/commands.sh#L1-6
#!/bin/sh
# .git/hooks/pre-push
set -e
./scripts/test-changed.sh || {
  echo "Some package tests failed, aborting push."
  exit 1
}
```

Note: Avoid blocking `git commit` with long tests — prefer `pre-push`.

---

## Troubleshooting

- If `cargo metadata` fails:
  - Ensure you are in the repository root.
  - Confirm `cargo` toolchain installed and `CARGO_HOME` isn't misconfigured.
- If `jq` is missing:
  - Install `jq` for better parsing:
    - macOS: `brew install jq`
    - Linux: `sudo apt install jq`
- If `sccache` is not present:
  - It’s optional, but running without sccache can mean slower builds.
  - Install and configure `sccache` to wrap `rustc`:
```/dev/null/commands.sh#L1-3
sccache --start-server
export RUSTC_WRAPPER=$(which sccache)
```
- If tests fail unexpectedly:
  - Re-run with `--verbose` and/or run the failing tests in isolation to gather logs:
```/dev/null/commands.sh#L1-2
cargo test -p my_pkg::some_test -- --nocapture
RUST_BACKTRACE=1 cargo test -p my_pkg -- --test-threads 1
```

---

## Advanced usage & tips

- Use `./scripts/test-changed.sh origin/main...HEAD --debug` to display more logs:
  - (The script prints which packages are mapped and commands invoked by the script).
- If you are working on a `core` crate that many other crates consume (e.g., `domain`), the change will likely require running multiple packages’ tests; the script will try to detect impacted packages but in complex cases do a full run:
  - `JOBS` set to a small value to avoid dev workstation meltdown.
- Consider creating `scripts/test-full.sh` which:
  - Bootstraps sccache and `cargo nextest`
  - Runs a full `nextest run` across the workspace
  - Use as a final verification step in CI or before merging.

---

## Implementation notes & decisions (brief)

- The script maps changed file paths to package directories derived from `cargo metadata`.
- It treats certain high-level files (top-level `Cargo.toml`, CI scripts, `scripts/`) as workspace-level changes and runs the full test suite in those cases.
- `jq` is preferred for parsing, but a fallback is available if `jq` is missing.
- The script runs `--lib`, `--all-targets`, and `--doc` to be conservative about running relevant tests; it tries to avoid cross-package runs unless necessary.

---

## Future improvements (ideas)

- Add a `--mode fast|full|changed` flag:
  - `fast`: no doctests, fast compile-only tests
  - `changed`: default behaviour described here
  - `full`: same as `cargo nextest run --workspace --all-features`
- Add a `--packages` option to specify test jobs explicitly (manual override).
- Add a `--preview` mode to list packages to be tested without running them.
- Add an "impact analysis" which looks up package dependency graph and tests packages that depend on changed packages (conservative but increases accuracy).
- Add "smart detection" for compile-time-only changes (header type modifications) that may require running tests beyond only the package with direct changes.

---

## Contact / maintenance

- A suggested owner for the script: the SDK / local test tooling maintainers.
- If you'd like, expand the script to:
  - Be more strict about changes in domain libs and run dependent packages too.
  - Add a faster 'check only' path (`cargo check` instead of `cargo test`).
- Remember that it’s intentional the script is conservative — when unsure it runs the full suite.

---

## Quick summary (one-liners)

- Run quick package tests for changed files:
```/dev/null/commands.sh#L1-2
./scripts/test-changed.sh
```

- Run for a range (CI):
```/dev/null/commands.sh#L1-2
./scripts/test-changed.sh origin/main...HEAD
```

- Run full verification (use in CI gate or pre-merge job):
```/dev/null/commands.sh#L1-2
cargo nextest run --workspace --all-features
```

---

Thanks for trying this approach. The script provides fast local feedback and a path to keep the `cargo test --all-features` gating for merges/CI. If you want, I can add:
- a `scripts/test-full.sh` wrapper that configures `sccache` + `cargo nextest`,
- a `pre-push` hook example and a `Makefile` target for local dev,
- or integrate the script into a recommended GitHub Actions workflow snippet.
