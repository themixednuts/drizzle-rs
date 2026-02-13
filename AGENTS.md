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

- MSRV: Rust 1.91
- Edition: 2024
- Resolver: v3

## PostgreSQL Testing

PostgreSQL tests require a running container. The `docker-compose.yml` provides PostgreSQL 18-Alpine with database `drizzle_test` and credentials `postgres:postgres`.

```bash
just pg-up      # start container
just pg-down    # stop container
just pg-shell   # connect with psql
```

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
