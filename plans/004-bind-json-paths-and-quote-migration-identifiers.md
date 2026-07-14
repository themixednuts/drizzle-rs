# Plan 004: Remove unsafe implicit SQL interpolation

> **Executor instructions**: Follow this plan exactly, run every verification,
> and stop on any STOP condition. Update the 004 row in `plans/README.md` when
> done.
>
> **Drift check (run first)**: `git diff --stat 12495f36..HEAD -- sqlite/src/expr.rs migrations/src/migrator.rs types/src/migration.rs migrations/src tests plans/README.md`
> Compare any changed code with the excerpts below. Semantic drift is a STOP.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: `plans/001-verify-compile-fail-diagnostics.md`
- **Category**: security
- **Planned at**: commit `12495f36`, 2026-07-12

## Why this matters

SQLite JSON helper arguments and migration tracking names are caller-controlled
data but are currently inserted directly into SQL text. Quote-bearing values
can alter syntax, and malformed identifiers can break migration bookkeeping.
Render identifiers with dialect-correct escaping and bind JSON paths/fields as
parameters, preserving typed expression composition and parameter ordering.

## Current state

- `types/src/migration.rs:50-101` exposes `MigrationTracking::new`, `.table`,
  and `.schema` with arbitrary strings; validation cannot assume fixed names.
- `migrations/src/migrator.rs:317` builds quoted identifiers without escaping:

```rust
match (&self.dialect, &self.schema) {
    (Dialect::PostgreSQL, Some(schema)) => format!("\"{}\".\"{}\"", schema, self.table),
    (Dialect::MySQL, _) => format!("`{}`", self.table),
    _ => format!("\"{}\"", self.table),
}
```

- `create_schema_sql` similarly interpolates `"{schema}"`.
- `sqlite/src/expr.rs` embeds fields and paths in raw fragments, for example:

```rust
left.to_sql()
    .append(SQL::raw(format!("->>'{field}' = ")))
    .append(SQL::param(value.into()))
```

and:

```rust
SQL::raw("json_extract(")
    .append(left.to_sql())
    .append(SQL::raw(format!(", '{path}') = ")))
```

- The affected helpers are `json_eq`, `json_ne`, `json_contains`,
  `json_exists`, `json_not_exists`, `json_array_contains`,
  `json_object_contains_key`, `json_text_contains`, `json_gt`, `json_extract`,
  and `json_extract_text`.
- The established typed SQL mechanism is `SQL::param`; identifiers elsewhere
  use `SQL::ident`. `core/src/sql/chunk.rs:363` shows the required double-quote
  escaping rule: embedded `"` becomes `""`.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Migration tests | `cargo test -p drizzle-migrations` | all pass |
| SQLite expression tests | `cargo test -p drizzle-sqlite --features serde` | all pass, including doctests |
| SQLite integration | `cargo test --features "rusqlite,uuid,serde,query" -- --test-threads=1` | all pass |
| Format | `cargo +nightly fmt --all -- --check` | exit 0 |
| Lint | `cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |

## Scope

**In scope**:

- `migrations/src/migrator.rs`
- Migration identifier unit tests in `migrations/src/migrator.rs` or an
  existing sibling test module
- `sqlite/src/expr.rs`
- SQLite expression/integration tests under `sqlite/` or `tests/sqlite/`
- Documentation examples whose expected SQL/parameter order changes
- `plans/README.md` (status row only)

**Out of scope**:

- Restricting legal table/schema names in `MigrationTracking`
- Changing public helper names, return types, or type markers
- Replacing explicitly unsafe `SQL::raw` APIs
- Migration folder-name validation (plan 005)
- Broad SQL renderer refactoring

## Git workflow

- Branch: `codex/004-safe-dynamic-sql`
- Suggested commits: `fix: quote migration tracking identifiers`, then
  `fix: bind sqlite json paths`
- Do not push or open a PR unless asked.

## Steps

### Step 1: Add dialect-aware identifier escaping

In `migrations/src/migrator.rs`, add a small private renderer used by both
`table_ident` and `create_schema_sql`. For SQLite/PostgreSQL, surround with
double quotes and double embedded double quotes. For MySQL, surround with
backticks and double embedded backticks. Compose schema and table by quoting
each component separately. Do not reject valid punctuation or Unicode.

Add unit tests for default names, names containing the active quote character,
schema-qualified PostgreSQL, and MySQL backticks. Assert exact SQL strings for
`table_ident_sql`, `create_schema_sql`, `create_table_sql`, and one applied-name
query so all consumers are covered.

**Verify**: `cargo test -p drizzle-migrations migrator` → all matching tests pass.

### Step 2: Bind every JSON field/path

Replace raw quoted interpolation with `SQL::param(SQLiteValue::from(...))`.
Operators should be composed as raw operator tokens around a parameter, e.g.
`left ->> ? = ?`; functions should receive the path as their second bound
argument, e.g. `json_extract(left, ?) = ?`. Preserve the order in which callers'
existing expression parameters appear, then path/field, then comparison value.

For `json_object_contains_key`, construct the combined path as owned data and
bind that owned value. For `json_extract(path: impl AsRef<str>)`, copy into an
owned parameter if necessary to preserve its existing public signature and
lifetime flexibility. Do not convert a public helper to `&'a str` merely to
make borrowing easier.

**Verify**: `rg -n "format!\(.*(field|path|full_path)" sqlite/src/expr.rs` → no
matches in SQL construction.

### Step 3: Update examples and add execution regressions

Update doctest SQL strings to placeholders and assert parameter values/order
using the crate's existing parameter inspection pattern. Add a rusqlite
execution test with paths/fields containing quotes, dots, and bracket syntax.
The values must be treated strictly as JSON selectors, never SQL text. Cover a
helper with an existing left-hand expression parameter to prove ordering.

**Verify**: SQLite expression and integration commands both pass.

### Step 4: Run full quality gates

Run fmt and all-feature clippy. Confirm compile-fail tests from plan 001 still
pass for the broad SQLite feature set.

**Verify**: fmt, clippy, and
`cargo test --test compile_fail --features "rusqlite,uuid,serde,query" -- --test-threads=1`
all exit 0.

## Test plan

- Exact identifier rendering for ordinary and quote-bearing identifiers in all
  three dialect branches.
- Exact SQL and parameter ordering for each JSON helper.
- Runtime JSON lookup for quote/dot/bracket-bearing selectors.
- Existing typed expression and compile-fail suites remain unchanged and pass.

## Done criteria

- [ ] No migration tracking identifier is inserted without dialect escaping.
- [ ] No JSON field/path argument is embedded in raw SQL text.
- [ ] All affected doctests describe the parameterized SQL.
- [ ] New identifier, parameter-order, and runtime edge-case tests pass.
- [ ] Public helper names/signatures and type-safety tests are preserved.
- [ ] All commands in this plan exit 0.
- [ ] `plans/README.md` marks 004 DONE.

## STOP conditions

- Plan 001 is not DONE.
- SQLite does not accept a bound RHS for one of the JSON operators on the
  minimum supported SQLite library; report the exact version and error.
- Binding requires a breaking public lifetime/signature change.
- Identifier handling requires changing `MigrationTracking`'s public API.
- Parameter ordering cannot be asserted through existing SQL APIs.

## Maintenance notes

Any future helper accepting a runtime value must use `SQL::param`; only SQL
syntax belongs in `SQL::raw`. Reviewers should scrutinize both SQL text and
parameter order. Keep identifier escaping centralized in the migrator.
