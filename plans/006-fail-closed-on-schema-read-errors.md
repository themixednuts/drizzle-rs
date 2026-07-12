# Plan 006: Fail closed on metadata and introspection errors

> **Executor instructions**: Follow this plan step by step. Run each
> verification before continuing; stop and report on a STOP condition. Update
> the 006 row in `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat 12495f36..HEAD -- migrations/src/dir.rs src/builder/sqlite/rusqlite/mod.rs src/builder/sqlite/libsql/mod.rs src/builder/sqlite/turso/mod.rs cli/src/db/mod.rs cli/src/error.rs tests plans/README.md`
> Compare changed regions to the excerpts below. Semantic drift is a STOP.

## Status

- **Priority**: P1
- **Effort**: L
- **Risk**: MED
- **Depends on**: `plans/001-verify-compile-fail-diagnostics.md`
- **Category**: bug
- **Planned at**: commit `12495f36`, 2026-07-12

## Why this matters

Migration and introspection reads frequently convert database or decoding
errors into empty/partial collections. Callers can then treat applied
migrations as pending or diff a partial live schema into destructive changes.
Propagate errors with operation context, while distinguishing the one expected
case—an absent migration tracking table—from corruption, permission, transport,
and row-decoding failures.

## Current state

- `cli/src/db/mod.rs:1267-1302` treats any SQLite prepare error as “table might
  not exist” and drops row errors:

```rust
let Ok(mut stmt) = conn.prepare(&set.applied_names_sql()) else {
    return Ok(vec![]);
};
let names = stmt.query_map([], |row| row.get::<_, String>(0))?
    .filter_map(Result::ok)
    .collect();
```

- PostgreSQL migration reads use `unwrap_or_default` and `try_get(...).ok()` at
  `cli/src/db/mod.rs:1370-1373`, with similar sync/async copies around 1476,
  1633, 1682, 2576, and 2635.
- libsql and Turso applied-name/record readers return empty on any query error
  and stop their row loops on any `next()` error around lines 1927-1957 and
  2147-2177.
- Runtime rusqlite introspection drops row errors with
  `.filter_map(Result::ok)` and ignores prepare/query failures for indexes,
  foreign keys, and views (`src/builder/sqlite/rusqlite/mod.rs:460-583`).
- Runtime libsql/Turso introspection uses `while let Ok(Some(row))`,
  `unwrap_or_default`, and optional per-table queries
  (`src/builder/sqlite/libsql/mod.rs:485-615` and
  `src/builder/sqlite/turso/mod.rs:515-637`). Their public `introspect()` already
  returns `drizzle_core::error::Result`, so no public return-type change is
  needed.
- CLI introspection duplicates the same permissive patterns at
  `cli/src/db/mod.rs:3108-3237` (rusqlite) and `:3396-3518` (libsql/Turso path).
- `migrations/src/dir.rs:47-63` also skips unreadable directory entries and
  folders lacking `migration.sql`, despite its docs promising errors.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Migration discovery | `cargo test -p drizzle-migrations dir` | all pass |
| Rusqlite | `cargo test --features "rusqlite,uuid,serde" -- --test-threads=1` | all pass |
| libsql | `cargo test --features "libsql,uuid,serde" -- --test-threads=1` | all pass |
| Turso targeted | `cargo test --features "turso,uuid,serde" introspect_fail_closed -- --test-threads=1` | new targeted tests pass |
| PostgreSQL | `just test::pg "postgres-sync,tokio-postgres,uuid,serde"` | all pass |
| MSRV check | `cargo +1.95.0 check --workspace --all-targets --all-features` | exit 0 |
| Lint | `cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |

The Turso command is intentionally targeted. Do not turn unrelated experimental
Turso suite failures into CI enforcement; that was explicitly deferred.

## Scope

**In scope**:

- `migrations/src/dir.rs`
- `src/builder/sqlite/rusqlite/mod.rs`
- `src/builder/sqlite/libsql/mod.rs`
- `src/builder/sqlite/turso/mod.rs`
- `cli/src/db/mod.rs`
- `cli/src/error.rs` only for contextual error variants/messages
- Driver/discovery regression tests in existing test modules
- `plans/README.md` (status row only)

**Out of scope**:

- Schema diff rules or generated SQL
- Consolidating all driver adapters into a new architecture
- Changing public `introspect()` return types
- Turso feature-parity work or changing its CI `continue-on-error`
- Retry policies for network failures
- PostgreSQL TLS (plan 007)

## Git workflow

- Branch: `codex/006-fail-closed-schema-reads`
- Split commits by behavior: migration metadata/discovery, runtime
  introspection, then CLI introspection.
- Conventional example: `fix: propagate schema introspection errors`
- Do not push or open a PR unless asked.

## Steps

### Step 1: Make filesystem discovery truthful

Replace `filter_map(Result::ok)` in `MigrationDir::discover_v3` with an
explicit loop using `?`/`map_err`. If a directory looks like a migration folder
but lacks `migration.sql`, return the documented `MissingMigration`; do not
silently skip it. Ignore ordinary non-directory files in the output root.

Add tempfile tests for an unreadable/invalid entry where portable, a missing
SQL file, a normal migration, and an unrelated regular file.

**Verify**: migration discovery command passes.

### Step 2: Classify absent tracking tables separately from read failures

For each CLI/runtime migration reader, stop using “any query error means no
migrations.” Prefer ensuring the tracking table before querying where the flow
already supports it. Where a preflight read is required, recognize only the
driver's specific missing-table error code (`SQLITE_ERROR` with “no such table”
classification; PostgreSQL SQLSTATE `42P01`) as empty. Propagate syntax,
permission, transport, and decode errors with driver/operation context.

Replace every `filter_map(Result::ok)`, `try_get(...).ok()`,
`unwrap_or_default`, and error-terminating `while let` in applied migration
reads with explicit result collection/loops. Preserve record order.

**Verify**: rusqlite, libsql, and PostgreSQL commands pass.

### Step 3: Make runtime SQLite introspection all-or-error

Change rusqlite helper collection to `collect::<Result<Vec<_>, _>>()?` and make
views return `Result`. Convert optional index/FK/view prepares and queries into
propagated errors. For libsql and Turso, change tuple/vector helper return types
to `drizzle_core::error::Result<...>`, use `while let Some(row) = rows.next().await?`,
and apply `?`/`map_err` to required fields. Keep genuinely nullable catalog
fields as `Option`; do not replace NULL with an empty string.

Thread `?` through the existing public `introspect()` methods. Add context that
identifies the catalog operation and table/index without including credentials.

**Verify**: rusqlite, libsql, and targeted Turso commands pass.

### Step 4: Apply the same rules to CLI introspection

Refactor the corresponding helper blocks in `cli/src/db/mod.rs` to return
`Result` and propagate every catalog query, row iteration, and required-field
decode error. Reuse `CliError::MigrationError` or a small contextual
introspection error; do not stringify away the operation name.

Keep snapshot-building pure: it should only run after all catalog reads have
succeeded. A failure must return before diff/push SQL is generated or executed.

**Verify**: CLI-bearing rusqlite/libsql/PostgreSQL commands pass.

### Step 5: Add fail-closed regressions

Add tests that distinguish:

1. absent tracking table → empty applied set where absence is expected;
2. malformed tracking table/column type → error;
3. a row-decode failure after one good row → error, never a partial vector;
4. an introspection query/row failure → error, never a partial snapshot;
5. a valid schema with nullable catalog fields → successful snapshot preserving
   `Option` semantics.

Use local in-memory/file databases. For async Turso use its existing
`Builder::new_local(":memory:")` pattern. Do not require remote credentials.

**Verify**: run each driver command twice; results are deterministic and pass.

### Step 6: Search for remaining fail-open patterns and run gates

Review only the in-scope read paths. Legitimate best-effort cleanup (for
example rollback) may still ignore errors; catalog and metadata reads may not.

**Verify**:

```text
rg -n "filter_map\(Result::ok\)|unwrap_or_default\(\)|while let Ok\(Some|return Ok\(vec!\[\]\)" migrations/src/dir.rs src/builder/sqlite/rusqlite/mod.rs src/builder/sqlite/libsql/mod.rs src/builder/sqlite/turso/mod.rs cli/src/db/mod.rs
```

Every remaining match is unrelated to metadata/introspection and documented in
the PR. MSRV check and clippy both exit 0.

## Test plan

- Missing tracking table is the only empty-on-error case.
- Malformed metadata fails before migration selection.
- Partial catalog reads never produce a snapshot.
- Nullable database metadata remains nullable, not an error/default string.
- Runtime and CLI paths are covered for rusqlite/libsql; targeted local Turso
  coverage compiles and passes; both PostgreSQL clients pass existing tests.

## Done criteria

- [ ] Required directory, metadata, and catalog reads propagate errors.
- [ ] Missing-table handling uses driver error classification, not message-wide
      catch-all behavior.
- [ ] `introspect()` cannot return a partial snapshot after a read failure.
- [ ] No public return type or type-safety guarantee is weakened.
- [ ] All commands in the table pass, including targeted local Turso tests.
- [ ] Turso CI masking remains untouched.
- [ ] `plans/README.md` marks 006 DONE.

## STOP conditions

- Plan 001 is not DONE.
- A driver does not expose a stable missing-table/error classification; report
  the exact error type rather than matching arbitrary text.
- Correct propagation requires a breaking public return-type change.
- A test requires remote Turso credentials.
- Fixing a failure requires changing schema diff semantics or generated SQL.

## Maintenance notes

Reviewers should challenge every default/skip in catalog-reading code: nullable
data is normal, failed reads are not. Keep operation context free of URLs,
tokens, and passwords. Turso CI enforcement remains a separate deferred item.
