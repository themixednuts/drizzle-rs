# Plan 002: Execute relational-query suites in CI

> **Executor instructions**: Follow this plan step by step and run every
> verification. Stop on any listed STOP condition. Update the 002 row in
> `plans/README.md` when complete.
>
> **Drift check (run first)**: `git diff --stat 12495f36..HEAD -- test.just tests/sqlite/mod.rs tests/postgres/mod.rs tests/compile_fail.rs .github/workflows/ci.yml plans/README.md`
> Compare changed files with the excerpts below; semantic drift is a STOP.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: `plans/001-verify-compile-fail-diagnostics.md`
- **Category**: tests
- **Planned at**: commit `12495f36`, 2026-07-12

## Why this matters

Relational query tests exist but are gated behind `feature = "query"`, while
the feature matrices omit `query`. CI therefore does not execute a substantial
public API suite or its negative type tests. Add `query` only to the full
additive rows so bare-driver minimal-feature coverage remains intact.

## Current state

- `test.just:13`: `_sqlite-ext := "uuid,serde,arrayvec,compact-str,bytes,smallvec-types"`
- `test.just:15`: `_pg-ext := "uuid,serde,arrayvec,compact-str,bytes,smallvec-types,chrono,cidr,geo-types,bit-vec"`
- `.github/workflows/ci.yml` consumes JSON emitted by
  `just test::sqlite-matrix-json` and `just test::pg-matrix-json`.
- `tests/sqlite/mod.rs:23` and `tests/postgres/mod.rs:25` gate `mod query` on the
  `query` feature.
- `tests/compile_fail.rs` gates `query_api_sqlite_ui` on `rusqlite + query`.
- A verified local run at the planned commit passed 36 SQLite relational tests:
  `cargo test --features "rusqlite,uuid,serde,query" sqlite::query:: -- --test-threads=1`.
- Omitting `serde` does not compile the query fixtures because their JSON/custom
  types require deserialization; the full additive rows already include it.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Matrix output | `just test::sqlite-matrix-json` | full rusqlite/libsql rows include `query`; bare rows do not |
| SQLite query tests | `cargo test --features "rusqlite,uuid,serde,query" sqlite::query:: -- --test-threads=1` | 36 or more tests pass |
| PostgreSQL matrix | `just test::pg` | all PostgreSQL rows pass with Docker available |
| Full SQLite matrix | `just test::sqlite-matrix` | all rows pass |
| Format | `cargo +nightly fmt --all -- --check` | exit 0 |

## Scope

**In scope**:

- `test.just`
- `.github/workflows/ci.yml` only if a focused query job is required after the
  matrix change
- Query test fixtures only if they reveal a real test-only feature assumption
- `plans/README.md` (status row only)

**Out of scope**:

- Query implementation or public API changes
- Removing bare-driver matrix rows
- Turso CI behavior (explicitly deferred)
- Adding `query` to default crate features

## Git workflow

- Branch: `codex/002-query-tests-ci`
- Commit: `test: run relational query suites in CI`
- Do not push or open a PR unless asked.

## Steps

### Step 1: Add query to full additive matrices

Append `query` to `_sqlite-ext` and `_pg-ext` in `test.just`. Do not add it to
the bare rows. Avoid duplicating matrix definitions in the workflow.

**Verify**: run both `just test::sqlite-matrix-json` and
`just test::pg-matrix-json` → each full row contains `query`; each bare row is
unchanged.

### Step 2: Exercise the newly selected suites locally

Run the focused SQLite command, then `just test::sqlite-matrix`. Run
`just test::pg` with the full PostgreSQL feature string emitted by the matrix.
The existing `just` recipe starts and waits for PostgreSQL.

**Verify**: all commands exit 0 and the logs show `sqlite::query::` and
`postgres::query::` tests executed rather than filtered out.

### Step 3: Confirm CI wiring and formatting

Verify the workflow still derives its matrices from the `just` outputs. Do not
add a second source of feature truth unless GitHub's matrix cannot express the
needed row. Run format checks.

**Verify**: `rg -n "sqlite-matrix-json|pg-matrix-json" .github/workflows/ci.yml test.just`
shows the existing producer/consumer path; fmt exits 0.

## Test plan

- Keep bare driver rows as compatibility checks.
- Full rusqlite and libsql rows must include `query` and `serde`.
- Full postgres-sync and tokio-postgres rows must include `query` and `serde`.
- The compile-fail query API test from plan 001 must execute in the full
  rusqlite row.

## Done criteria

- [ ] Both additive feature constants contain `query` exactly once.
- [ ] Bare matrix rows remain unchanged.
- [ ] Focused SQLite relational tests pass (at least the current 36).
- [ ] Full SQLite and PostgreSQL matrix commands pass.
- [ ] No query implementation or Turso workflow code changed.
- [ ] `plans/README.md` marks 002 DONE.

## STOP conditions

- Plan 001 is not DONE.
- Adding `query` requires changing a public query API to make tests compile.
- A full row lacks `serde` or another documented prerequisite.
- PostgreSQL failures are unrelated infrastructure failures after two retries;
  report the logs instead of modifying the suite.

## Maintenance notes

Keep `query` in the shared additive constants so local `just` runs and GitHub
Actions cannot drift. If query later becomes a default feature, retain explicit
coverage showing both minimal and full feature configurations.
