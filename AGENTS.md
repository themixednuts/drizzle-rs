# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Drizzle-RS is a type-safe SQL ORM/query builder for Rust inspired by Drizzle ORM. It uses procedural macros to generate type-safe schema definitions and query builders at compile time.

## Build Commands

```bash
# Build
cargo build --all-features

# Test SQLite
cargo test --features "rusqlite,uuid"

# Test PostgreSQL (requires Docker)
just test-pg              # starts container, runs tests, stops container
just test-pg-dev          # keeps container running for development

# Test with libsql (requires single-threaded execution)
cargo test --features "rusqlite,libsql,uuid" -- --test-threads=1

# Lint (requires nightly)
cargo +nightly fmt --all -- --check
cargo clippy --all-features -- -D warnings

# Generate docs
cargo doc --all-features --no-deps

# Run benchmarks
cargo bench --features "rusqlite,uuid"
```

## Project Architecture

### Workspace Crates

- **drizzle** (root) - Main library with driver implementations in `src/builder/` and re-exports
- **drizzle-core** - SQL generation engine, type-safe expression system (`expr/`), no_std compatible
- **drizzle-macros** - Procedural macros: `#[SQLiteTable]`, `#[PostgresTable]`, `#[SQLiteSchema]`, `#[PostgresSchema]`, `#[SQLiteEnum]`, `#[PostgresEnum]`, `#[SQLiteIndex]`, `#[PostgresIndex]`, `#[SQLiteFromRow]`, `#[PostgresFromRow]`
- **drizzle-sqlite** - SQLite query builder implementation
- **drizzle-postgres** - PostgreSQL query builder implementation
- **drizzle-mysql** - MySQL (WIP, minimal)
- **drizzle-migrations** - Migration infrastructure and DDL types
- **drizzle-types** - SQL type markers (`Int`, `Text`, `Bool`, etc.)
- **drizzle-cli** - CLI tool (`drizzle` binary) for migrations

### Key Architectural Patterns

**Type-Safe Expressions** (`core/src/expr/`): The `Expr<'a, V>` trait enforces compile-time type checking with associated types for `SQLType`, `Nullable` (NonNull/Null), and `Aggregate` (Scalar/Agg).

**Sealed Trait Pattern**: Internal traits use `private::Sealed` to prevent external implementations.

**Feature-Gated Drivers**: Each database driver is optional:
- SQLite: `rusqlite` (sync), `libsql` (async), `turso` (async)
- PostgreSQL: `postgres-sync`, `tokio-postgres`

**Macro Code Generation**: Procedural macros generate column accessors, Select/Insert/Update models, and schema metadata. Common code is in `procmacros/src/common/`.

### Test Organization

- `tests/sqlite/` - SQLite integration tests
- `tests/postgres/` - PostgreSQL integration tests
- `tests/compile_fail/` - Compile-time type safety verification via `trybuild`

## Configuration

- MSRV: Rust 1.95
- Edition: 2024
- Resolver: v3

## PostgreSQL Testing

PostgreSQL tests require a running container. The `docker-compose.yml` provides PostgreSQL 18-Alpine with database `drizzle_test` and credentials `postgres:postgres`.

```bash
just pg-up      # start container
just pg-down    # stop container
just pg-shell   # connect with psql
```

## Releases & Versioning (pre-1.0)

While the project is pre-1.0, **every release is a patch bump** (`0.1.x → 0.1.(x+1)`) by default. The trend is intentional — pinning to a 0.1.x version means "I accept anything between 0.1.x and 0.1.<latest>", and we want that contract to hold for the whole 0.1 series.

The release tooling enforces this:

- `release-plz.toml` sets `features_always_increment_minor = false`. In 0.x, `feat:` commits are treated as patch-level changes (release-plz only starts incrementing the minor on `feat:` once the workspace hits `1.x`).
- `semver_check = false` so `cargo-semver-checks` doesn't escalate the bump when it spots an API change. The developer decides when to roll the second number, not the tool.

### Commit type → bump effect

| Commit type | Bump | Use for |
|---|---|---|
| `fix:` | patch | bug fixes |
| `refactor:` | patch | internal restructuring, no public-API surface change |
| `chore:` | patch | repo housekeeping, deps, tooling |
| `docs:` | patch | documentation only |
| `perf:` | patch | performance only |
| `test:` | patch | test-only |
| `feat:` | **patch** (pre-1.0 policy) | new public API — tagged for changelog but doesn't escalate the version |
| `feat!:` / `BREAKING CHANGE:` footer | **minor** (in 0.x: `0.1.x → 0.2.0`) | intentional breaking API change |

### When to roll the second number

Until we hit 1.0, stay on `0.1.x` and use `feat!:` only when you genuinely want to declare a `0.2.0`. When ready for 1.0:

- Flip `features_always_increment_minor` to `true` in `release-plz.toml` so `feat:` starts incrementing minor again.
- Re-enable `semver_check = true` so accidental API breaks get caught.
- Bump `[workspace.package].version` to `1.0.0-rc.1` (or whatever prep version) and let release-plz take it from there.

### Manual override

If release-plz proposes a wrong version on a release PR, edit the workspace version in the PR before merging. release-plz publishes whatever's in `Cargo.toml` after the merge.

### CI must pass before publish

`publish.yml` runs a `gh pr checks` verification before invoking release-plz. If the release PR's head commit doesn't have all-green CI, the publish step refuses and the run fails. This protects against:

- Force-merged PRs that bypass branch protection
- PRs merged with skipped/stale CI runs
- PRs where CI was queued but never completed

For belt-and-suspenders, enable **branch protection** on `main`:

- Repo → Settings → Branches → Branch protection rules
- Add a rule for `main`
- Require status checks to pass before merging
- Required: `Lint`, `Documentation`, `MSRV Check`, at least one `test::sqlite` job, at least one `test::pg` job
- (Optional) Require pull request reviews

This gives you two layers of defense — the branch can't merge a red PR, and even if someone bypasses that, `publish.yml` re-verifies before pushing crates to crates.io.

> [!IMPORTANT]
> CI does *not* skip release PRs. The skip in `ci.yml` only fires on push events to `main` (the merge commit, after the PR-level CI already ran). Pull-request events always get the full lint/build/test/docs matrix — including release PRs opened by `github-actions[bot]`.

## Git: Splitting Commits by Intent

When a single working tree has changes spanning multiple features, use `git apply --cached` to stage individual hunks non-interactively (no `-i` / `-p` needed).

### Workflow

```bash
# 1. Unstage everything to start clean
git restore --staged .

# 2. For files that belong entirely to one commit, just git add them
git add path/to/new_file.rs path/to/single_intent_change.rs

# 3. For shared files (changes from multiple features in one file),
#    write a patch with only the hunks you want staged:
cat > ./tmp/feature_a.patch << 'PATCH'
diff --git a/shared/file.rs b/shared/file.rs
--- a/shared/file.rs
+++ b/shared/file.rs
@@ -10,6 +10,7 @@ existing context line
 more context
 another context line
+pub mod feature_a;
 trailing context
 more trailing context

PATCH

# 4. Stage just those hunks (--cached = index only, working tree untouched)
git apply --cached ./tmp/feature_a.patch

# 5. Commit, then repeat for the next intent
git commit -m "feat(a): description"
```

### Tips

- **Get the full diff first** for reference: `git diff path/to/shared_file.rs > ./tmp/full.patch`
- **Context lines matter** — include 3 lines of surrounding context so git can locate the hunk
- **Trailing newline required** — patches must end with a blank line after the last hunk
- **Line numbers are fuzzy** — git uses context matching, so `@@` line numbers don't need to be exact
- **New files** can't use `apply --cached` — just `git add` them directly
- **Verify before committing**: `git diff --cached --stat` to confirm only intended files/hunks are staged
