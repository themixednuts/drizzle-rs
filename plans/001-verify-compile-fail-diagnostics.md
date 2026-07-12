# Plan 001: Verify intended compile-fail diagnostics

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If a STOP condition occurs, stop and report; do not improvise.
> When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 12495f36..HEAD -- tests/compile_fail.rs tests/ui Cargo.toml plans/README.md`
> If any in-scope file changed, compare the excerpts below with live code. A
> semantic mismatch is a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: LOW
- **Depends on**: none
- **Category**: tests
- **Planned at**: commit `12495f36`, 2026-07-12

## Why this matters

The current negative type tests only prove that a fixture fails somehow. A
syntax error, missing import, or unrelated trait error therefore satisfies the
test and can hide a regression in the intended type-safety rule. Use
trybuild's diagnostic snapshots so each fixture must fail for the reviewed
reason while retaining all existing pass fixtures and feature gates.

## Current state

- `tests/compile_fail.rs` owns all UI compile tests.
- `tests/ui/**/pass/*.rs` and `tests/ui/**/fail/*.rs` contain the fixtures.
- `Cargo.toml:130` already declares `trybuild = "1.0"`; no new dependency is
  required.
- There are currently no checked-in `.stderr` snapshots.

Current helper (`tests/compile_fail.rs:29`):

```rust
fn must_fail(glob_pattern: &str) {
    for file in collect_files(glob_pattern) {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let t = trybuild::TestCases::new();
            t.pass(&file);
        }));
        assert!(outcome.is_err(), "expected `{file}` to fail compilation");
    }
}
```

This catches any panic from asking trybuild to compile a negative fixture as a
passing fixture; it never compares the compiler output with an expected error.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Format | `cargo +nightly fmt --all -- --check` | exit 0 |
| SQLite UI tests | `cargo test --test compile_fail --features "rusqlite,uuid,serde,query" -- --test-threads=1` | all selected UI tests pass |
| PostgreSQL UI tests | `cargo test --test compile_fail --features "postgres-sync,uuid,serde,query" -- --test-threads=1` | all selected UI tests pass; no server is required |
| Lint | `cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings` | exit 0, no warnings |

## Scope

**In scope**:

- `tests/compile_fail.rs`
- `tests/ui/**/*.stderr` (create reviewed snapshots beside negative fixtures)
- Negative fixtures under `tests/ui/**/fail/*.rs` only when a minimal change is
  required to isolate the diagnostic the fixture is named for
- `plans/README.md` (status row only)

**Out of scope**:

- Production traits, macros, or query APIs
- Relaxing or removing any negative type-safety case
- Replacing trybuild with a custom compiler-output parser
- Updating toolchains or dependencies

## Git workflow

- Branch: `codex/001-compile-fail-diagnostics`
- Use conventional commits, e.g. `test: verify compile-fail diagnostics`
- Keep fixture/snapshot changes in the same logical commit.
- Do not push or open a PR unless the operator asks.

## Steps

### Step 1: Replace panic detection with trybuild compile-fail assertions

Remove `catch_unwind`, `AssertUnwindSafe`, `PathBuf`, `collect_files`, and the
per-file `t.pass()` loop. Implement `must_fail(glob: &str)` as a direct
`trybuild::TestCases::new().compile_fail(glob)` call. Keep `must_pass` and every
existing test function and `#[cfg]` unchanged so coverage does not shrink.

**Verify**: `cargo test --test compile_fail --features "rusqlite,uuid,serde,query" --no-run` → exit 0.

### Step 2: Generate and review snapshots

Run the SQLite command once with `TRYBUILD=overwrite` (PowerShell:
`$env:TRYBUILD='overwrite'; cargo test ...; Remove-Item Env:TRYBUILD`) and then
the PostgreSQL command the same way. Review every new `.stderr` file. Each must
contain the diagnostic implied by its directory and filename; delete incidental
warnings by fixing the fixture rather than accepting them blindly. Normalize
only through trybuild—do not hand-remove platform-specific placeholders.

**Verify**: `git status --short tests/ui` → only expected `.stderr` files and
any intentionally tightened negative fixtures are listed; no `wip/` directory
exists.

### Step 3: Prove snapshots detect the wrong failure

Choose one small negative fixture, temporarily introduce an unrelated syntax
error, and run its owning UI test. It must fail with a snapshot mismatch.
Restore the temporary edit immediately and rerun the test successfully. Do not
commit the temporary mutation.

**Verify**: `git diff --check` → exit 0, then both UI test commands in the table
pass without `TRYBUILD=overwrite`.

### Step 4: Run repository quality gates

Run formatting and all-feature clippy. Fix only test-harness issues within this
plan's scope.

**Verify**: the format and lint commands both exit 0.

## Test plan

- Preserve every existing positive fixture and feature gate.
- Add a `.stderr` snapshot for every negative fixture reached by the two broad
  feature sets.
- Confirm the `query_api_sqlite` negative cases are included by the `query`
  feature.
- Confirm a deliberately different compiler failure causes a mismatch.

## Done criteria

- [ ] `must_fail` uses `trybuild::TestCases::compile_fail`.
- [ ] Every exercised `fail/*.rs` has a reviewed sibling `.stderr` file.
- [ ] No `catch_unwind`, `AssertUnwindSafe`, or `.pass(&file)` remains in the
      negative-test path.
- [ ] Both UI test commands, fmt, and clippy exit 0.
- [ ] `rg --files tests | rg 'wip|\.stderr\.new'` returns no output.
- [ ] No production code changed.
- [ ] `plans/README.md` marks 001 DONE.

## STOP conditions

- The compiler output varies nondeterministically across two identical runs.
- A fixture fails only because a required feature/import is absent, and
  isolating its intended diagnostic would require production API changes.
- trybuild cannot normalize output on the repository's supported platforms.
- Any existing negative fixture unexpectedly compiles successfully.

## Maintenance notes

Reviewers should read snapshots for intent, not just their presence. Future
negative fixtures must land with `.stderr`; compiler/toolchain updates may
legitimately refresh snapshots, but each diff requires human review.
