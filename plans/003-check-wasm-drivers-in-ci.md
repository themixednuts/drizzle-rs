# Plan 003: Compile D1 and Durable drivers for WASM in CI

> **Executor instructions**: Follow every step and verification. Stop and
> report on a STOP condition. Update the 003 row in `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat 12495f36..HEAD -- .github/workflows/ci.yml src/lib.rs Cargo.toml procmacros/src plans/README.md`
> Semantic drift from the current-state excerpts is a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: MED
- **Depends on**: none
- **Category**: dx
- **Planned at**: commit `12495f36`, 2026-07-12

## Why this matters

The public D1 and Durable drivers only compile on `wasm32`, so host
`--all-features` jobs cannot validate them. Both target checks pass at the
planned commit, but neither is enforced in CI. Add an explicit WASM job without
changing the drivers or pretending runtime integration is covered.

## Current state

- `src/lib.rs:273-288` exports `builder::sqlite::d1` and
  `builder::sqlite::durable` only under `target_arch = "wasm32"` plus their
  respective features.
- `.github/workflows/ci.yml:263` runs
  `cargo build --workspace --all-features` on host targets.
- The following exact MSRV commands were verified at commit `12495f36`:

```text
cargo +1.95.0 check -p drizzle --target wasm32-unknown-unknown --no-default-features --features d1
cargo +1.95.0 check -p drizzle --target wasm32-unknown-unknown --no-default-features --features durable
```

Both exit 0. Each currently emits 13 feature-specific unused-code warnings
from `drizzle-macros`; that warning cleanup is not part of this plan.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| D1 target check | command above with `--features d1` | exit 0 |
| Durable target check | command above with `--features durable` | exit 0 |
| Workflow lint proxy | `cargo +nightly fmt --all -- --check` | exit 0 |
| Host regression | `cargo +1.95.0 check --workspace --all-targets --all-features` | exit 0 |

## Scope

**In scope**:

- `.github/workflows/ci.yml`
- `plans/README.md` (status row only)

**Out of scope**:

- D1/Durable implementation changes
- Browser/Workers runtime integration tests
- Fixing macro warnings exposed by minimal WASM features
- Turso CI behavior
- Changing the workspace MSRV or default features

## Git workflow

- Branch: `codex/003-wasm-driver-ci`
- Commit: `ci: check wasm-only sqlite drivers`
- Do not push or open a PR unless asked.

## Steps

### Step 1: Add a dedicated WASM compile job

Add a GitHub Actions job after lint that installs Rust `1.95.0` (prefer reading
the workspace `rust-version` exactly as the existing MSRV job does) with target
`wasm32-unknown-unknown`, restores the Rust cache with a distinct `wasm` key,
and runs the two verified commands separately. Give steps descriptive names so
the failing driver is obvious.

Do not use `--all-features`: the two isolated checks prove each target-only
feature compiles without accidental feature unification.

**Verify**: `rg -n "wasm32-unknown-unknown|--features d1|--features durable" .github/workflows/ci.yml`
shows one target installation and both isolated checks.

### Step 2: Run the commands and host regression locally

Install the target if needed with
`rustup target add wasm32-unknown-unknown --toolchain 1.95.0`, then run both
target checks and the host MSRV regression.

**Verify**: all three cargo commands exit 0. Warnings are allowed for the two
WASM checks; errors are not.

### Step 3: Keep CI dependency semantics simple

The new job may depend on `lint`, matching other validation jobs. Do not make
unrelated test jobs depend on it or edit release/publish workflows in this
plan.

**Verify**: inspect `git diff -- .github/workflows/ci.yml` → only the new job is
present; YAML indentation matches adjacent jobs.

## Test plan

- Compile D1 alone at MSRV for `wasm32-unknown-unknown`.
- Compile Durable alone at MSRV for the same target.
- Re-run host all-target/all-feature MSRV check.
- The CI job must fail on either command's nonzero exit (no `continue-on-error`
  or shell masking).

## Done criteria

- [ ] CI installs `wasm32-unknown-unknown` for the workspace MSRV.
- [ ] D1 and Durable are checked in separate commands without default features.
- [ ] Neither command is masked or allowed to fail.
- [ ] All local verification commands exit 0.
- [ ] Only `.github/workflows/ci.yml` and the plan status row changed.
- [ ] `plans/README.md` marks 003 DONE.

## STOP conditions

- Either verified command no longer compiles before workflow edits.
- The target requires credentials, network services, or a Workers runtime just
  to type-check.
- The job requires a source change to a D1/Durable implementation.
- GitHub's runner lacks the MSRV target; report the toolchain error.

## Maintenance notes

This is compile coverage, not behavioral coverage. Reviewers should ensure
future WASM-only features get isolated checks and that warnings are not
silently upgraded to errors without first cleaning the minimal feature builds.
