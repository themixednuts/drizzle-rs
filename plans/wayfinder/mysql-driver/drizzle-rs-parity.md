# drizzle-rs MySQL parity inventory

Scope: `E:\Projects\drizzle-rs` only. This note compares the current MySQL stub with the SQLite and PostgreSQL implementations. It does not choose a Rust MySQL client or describe upstream Drizzle ORM behavior.

## Current state

`drizzle-mysql` is a published placeholder. Its library has five lines, exports no modules or types, and says the builder and driver integrations are future work. Its manifest depends only on `drizzle-core`, `serde`, `serde_json`, and `uuid`; there is no MySQL client dependency, `drizzle-types` dependency, error type, macro dev-dependency, or test support. Sources: `mysql/src/lib.rs:1-5`, `mysql/Cargo.toml:11-30`.

The umbrella crate exposes a `mysql` feature, but the public module is empty. In contrast, the SQLite and PostgreSQL modules re-export macros, dialect crates, driver-specific `Drizzle` types, and matching preludes. Sources: `Cargo.toml:321-324`, `src/lib.rs:228-315`, `src/lib.rs:318-426`, `src/lib.rs:429-432`.

This is not a partially working driver. It is a name reserved across the workspace.

## The public API MySQL should copy

MySQL should use the same API grammar as the other dialects. Dialect differences belong behind the grammar, not in new top-level verbs.

| Concern | Existing convention | Evidence |
|---|---|---|
| Connect | `Drizzle::new(connection, Schema::new()) -> (Drizzle<Schema>, Schema)` | `src/builder/sqlite/common.rs:168`, `src/builder/postgres/postgres_sync/mod.rs:200`, `src/builder/postgres/tokio_postgres/mod.rs:246-250` |
| Start a query | `select`, `select_distinct`, `insert`, `update`, `delete` | `src/builder/sqlite/common.rs:205-310`, `src/builder/postgres/mod.rs:3-96` |
| Select all table columns | `db.select(()).from(table)` | `README.md:276-293`, `tests/postgres/select.rs:179-194` |
| Select explicit columns | `db.select((table.id, table.name)).from(table)` | `README.md:289-293`, `sqlite/src/builder/mod.rs:478-493` |
| Continue a select | `from`, joins, `where`, `group_by`, `having`, `order_by`, `limit`, `offset`, set operations, `into_cte` | `sqlite/src/builder/select.rs:278-303`, `sqlite/src/builder/select.rs:363-723`; PostgreSQL has the same names at `postgres/src/builder/select.rs:175-525` |
| Execute | terminal `.execute()`, `.all()`, `.get()` on a built query | `src/builder/sqlite/rusqlite/mod.rs:1353-1432`, `src/builder/postgres/postgres_sync/mod.rs:1228-1316` |
| Driver-wide operations | `query`, `transaction`, `create`, `migrate`, `push` where supported | `src/builder/postgres/postgres_sync/mod.rs:381-478`, `src/builder/postgres/postgres_sync/mod.rs:511-533`, `src/builder/postgres/postgres_sync/mod.rs:1185` |
| Prepared queries | `.prepare()` on the same typed builder | `src/builder/mod.rs:9-37` |

Do not add `select_all`, `find_all`, or another MySQL-only shortcut. `select(())` already maps to the `SelectStar` type marker, infers the generated table select model, and expands to explicit table columns. Sources: `core/src/row/mod.rs:1609-1634`, `core/src/row/mod.rs:1366-1368`, `tests/postgres/select.rs:187-194`. `.all()` already means "execute this query and return every row", so a second "all" API would blur projection and execution.

The dialect crate should match the existing module layout: `attrs`, `builder`, `common`, `expr`, `helpers`, `traits`, `types`, and `values`. The root module should re-export those, expose driver namespaces containing `Drizzle` and transaction types, and provide `drizzle::mysql::prelude::*`. Sources: `sqlite/src/lib.rs:37-50`, `postgres/src/lib.rs:35-46`, `src/lib.rs:231-315`, `src/lib.rs:321-426`.

The builder should reuse the shared typestate and helper vocabulary. SQLite re-exports common `select`, `from`, set-operation, pagination, and mutation helpers, while PostgreSQL adds only PostgreSQL-specific pieces such as `select_distinct_on`, `join_using`, and row-locking methods. Sources: `sqlite/src/helpers.rs:10-43`, `postgres/src/helpers.rs:7-55`, `postgres/src/builder/select.rs:607-715`. MySQL-specific syntax should follow that pattern. It should not fork the whole API.

## Exact parity gaps

### 1. SQL types and compile-time dialect plumbing

`drizzle-types` has SQLite and PostgreSQL modules but no MySQL module. Its prelude exports only those two type systems. Sources: `types/src/lib.rs:51-61`, `types/src/lib.rs:71-77`.

`drizzle-core` has only `SQLiteDialect` and `PostgresDialect`. `DialectTypes`, `ValueTypeForDialect`, and `SQLTypeToRust` have no MySQL implementation. A MySQL value type therefore cannot satisfy the core bind and row-inference contracts. Sources: `core/src/dialect.rs:10-23`, `core/src/dialect.rs:31-90`, `core/src/bind.rs:1-10`, `core/src/bind.rs:77-219`, `core/src/row/mod.rs:1030-1098`.

Required parity:

- Add MySQL SQL marker and DDL types in `drizzle-types`.
- Add `MySQLDialect: DialectTypes`, MySQL `ValueTypeForDialect` mappings, and MySQL `SQLTypeToRust` mappings.
- Add borrowed and owned MySQL parameter values, insert/update wrappers, conversions, row traits, table/column traits, and `SQLParam` with `Dialect::MySQL`. SQLite and PostgreSQL show the required shape at `sqlite/src/values/mod.rs:20-31`, `sqlite/src/values/mod.rs:228-230`, `postgres/src/values/mod.rs:780-782`, `sqlite/src/traits/mod.rs:1-9`, and `postgres/src/traits.rs:1-13`.

### 2. Identifier rendering must become dialect-aware

The shared SQL renderer always surrounds identifiers, tables, and columns with double quotes. The function explicitly says it is correct for PostgreSQL and SQLite. Sources: `core/src/sql/chunk.rs:363-376`, `core/src/sql/chunk.rs:520-543`.

MySQL cannot be correct by merely setting `SQLParam::DIALECT = Dialect::MySQL`. Placeholder rendering is dialect-aware, but identifier rendering is not. Sources: `core/src/dialect.rs:93-121`, `core/src/traits/param.rs:11-20`.

Required parity: make identifier escaping/rendering dispatch on the value dialect and preserve the current output for SQLite and PostgreSQL. Cover embedded quote characters for each quote style. Do this in the shared renderer so every builder, generated schema, prepared statement, CTE, alias, and relation uses the same rule.

The relational query SQL path has a second dialect-blind quoting implementation. It writes literal double quotes around qualified columns and junction joins. Sources: `core/src/query/sql.rs:739-773`. That path also groups MySQL with SQLite for `json_group_array`, rather than owning an explicit MySQL branch. Sources: `core/src/query/sql.rs:813-837`. MySQL query support needs its own identifier and JSON rendering branch before the root `query` feature can honestly forward to it.

### 3. Dialect crate and builders

`mysql/src/lib.rs` exports none of the modules that define the other dialects. There is no MySQL schema object enum, number/value representation, query builder, CTE support, mutation builder, prepared statement, expression module, or dialect helpers. Compare `mysql/src/lib.rs:1-5` with `sqlite/src/lib.rs:37-50` and `postgres/src/lib.rs:35-46`.

Required parity:

- Implement the common query chain using the existing names and typestate rules.
- Gate dialect-only clauses. Do not copy PostgreSQL-only `DISTINCT ON`, named-constraint conflict targets, materialized view refresh, or PostgreSQL lock variants merely for symmetry. Their locations make the intended separation clear: `postgres/src/helpers.rs:42-55`, `postgres/src/builder/insert.rs:326-340`, `postgres/src/builder/refresh.rs:65-154`, `postgres/src/builder/select.rs:607-715`.
- Implement MySQL's insert-conflict and mutation syntax behind MySQL-specific builder states while keeping the entry point `insert(table).values(...)`. Existing insert entry points and model setters are at `sqlite/src/builder/insert.rs:121-157` and `postgres/src/builder/insert.rs:113-142`.
- Keep generated select/insert/update models and trait names parallel to the other dialects. The existing macro contract describes those models at `procmacros/src/lib.rs:2932-2941`.

### 4. Procedural macros

The macro crate declares `mysql = []`; it has optional SQLite and PostgreSQL dependencies but no optional `drizzle-mysql` dependency. Its source conditionally declares only `sqlite` and `postgres` modules. Sources: `procmacros/Cargo.toml:29-39`, `procmacros/Cargo.toml:50-59`, `procmacros/src/lib.rs:121-125`.

There are no `MySQLTable`, `MySQLSchema`, `MySQLIndex`, `MySQLFromRow`, `MySQLEnum`, or view macros. The baseline macro set is visible in `procmacros/src/lib.rs:316-318`, `procmacros/src/lib.rs:785-808`, `procmacros/src/lib.rs:1055-1056`, `procmacros/src/lib.rs:1557-1565`, `procmacros/src/lib.rs:1899-1917`, and the PostgreSQL equivalents at `procmacros/src/lib.rs:2611-2613`, `procmacros/src/lib.rs:2942-2979`, `procmacros/src/lib.rs:3147-3157`.

The names are already fixed elsewhere as `MySQLTable`, `MySQLIndex`, `MySQLSchema`, and `MySQLEnum`. Sources: `types/src/dialect.rs:92-121`, `migrations/src/parser/mod.rs:167-182`, `migrations/src/parser/mod.rs:422`.

The parser currently recognizes MySQL table and index attributes but parses MySQL fields using SQLite rules into a scratch diagnostics buffer because no snapshot is produced. That is a temporary parser shim, not macro support. Source: `migrations/src/parser/mod.rs:167-215`.

The shared `#[drizzle::test]` macro also knows only SQLite and PostgreSQL. It accepts only those dialect names and emits rusqlite/libsql/turso or postgres-sync/tokio-postgres cases. Sources: `procmacros/src/drizzle_test.rs:44-72`, `procmacros/src/drizzle_test.rs:102-108`, `procmacros/src/drizzle_test.rs:293-332`.

### 5. Root crate, drivers, and transactions

The root builder and transaction trees compile only SQLite and PostgreSQL modules. Sources: `src/builder/mod.rs:1-7`, `src/transaction/mod.rs:1-10`.

Required parity:

- Add a root MySQL builder adapter for each supported Rust client, plus driver-specific prepared execution and row decoding.
- Add transaction and savepoint adapters if the chosen client exposes them.
- Match the sync or async signatures of the client without changing query construction. Existing async drivers keep the same verbs and only make terminals async. Compare `src/builder/sqlite/rusqlite/mod.rs:168-283` with `src/builder/sqlite/libsql/mod.rs:172-316` and `src/builder/postgres/tokio_postgres/mod.rs:308-454`.
- Re-export driver adapters under `drizzle::mysql::<driver>::Drizzle`, not from an unrelated helper namespace.

### 6. Feature forwarding

The umbrella `mysql` feature currently enables an empty macro feature and the stub crate. `std` and `alloc` deliberately omit `drizzle-mysql`, but their comments are stale because `drizzle-mysql` is now published with both feature names. Sources: `Cargo.toml:174-195`, `Cargo.toml:321-324`, `mysql/Cargo.toml:22-30`.

`serde` and `uuid` forward to `drizzle-mysql`, but `query`, date/time, decimal, collection, bytes, and other common optional types do not. Sources: `Cargo.toml:253-299`, `Cargo.toml:313-324`. The macro crate likewise has no MySQL forwards, and `drizzle-core` has no MySQL backend feature or client dependency. Sources: `procmacros/Cargo.toml:50-114`, `core/Cargo.toml:98-104`.

`drizzle-seed` has only SQLite and PostgreSQL dialect features. The umbrella `mysql` feature does not activate seed support, unlike `sqlite` and `postgres`. Sources: `seed/Cargo.toml:11-30`, `Cargo.toml:321-324`.

Feature parity means each optional Rust value type must forward weakly through core, macros, dialect crate, driver crate, and seed where supported. Driver selection must be a separate feature from the `mysql` dialect feature, matching `postgres` versus `postgres-sync`/`tokio-postgres` at `Cargo.toml:321-342`.

### 7. Migrations and CLI

Migration tracking already contains MySQL-specific identifier quoting and tracking SQL. That part can be reused. Sources: `migrations/src/migrator.rs:611-619`, `migrations/src/migrator.rs:623-641`, `migrations/src/migrator.rs:690-719`.

Schema migrations are otherwise placeholders:

- The migration crate exports `sqlite` and `postgres` modules, but no `mysql` module. Source: `migrations/src/lib.rs:118-137`.
- `Snapshot` has only SQLite and PostgreSQL variants; loading, creating, and building a MySQL snapshot errors or panics. Sources: `migrations/src/schema.rs:10-29`, `migrations/src/schema.rs:45-77`, `migrations/src/snapshot_builder.rs:52-71`.
- The `Mysql` dialect trait uses `()` for snapshot, DDL, entity, and generator types, then panics in `diff_and_generate`. Source: `migrations/src/traits.rs:533-562`.
- The build API rejects any dialect except SQLite and PostgreSQL. Source: `migrations/src/build.rs:594-600`.
- The version model contradicts itself. `MYSQL_SNAPSHOT_VERSION` is 6 and the generic version helpers support 5 through 6, while the `Mysql` trait says its latest version is 5. Sources: `migrations/src/version.rs:21-22`, `migrations/src/version.rs:64-79`, `migrations/src/traits.rs:537-548`.

The CLI's own dialect enum accepts only `sqlite`, `postgresql`, and `turso`; its driver enum has no MySQL client. Sources: `cli/src/config.rs:213-268`, `cli/src/config.rs:283-339`. MySQL therefore needs config parsing, credentials, driver validation, generate/push/pull/introspection routing, and upgrade handling after snapshot support exists.

### 8. Tests, examples, docs, and CI

There is no MySQL test module or shared MySQL schema. Sources: `tests/lib.rs:1-7`, `tests/common/schema/mod.rs:1-4`, `tests/common/mod.rs:1-9`. There are no MySQL cases under `tests/ui`, and `tests/compile_fail.rs` gates cases only on SQLite and PostgreSQL features, for example `tests/compile_fail.rs:13-31`.

The local test matrix has only SQLite and PostgreSQL recipes. CI generates and runs only those two matrices. Sources: `test.just:12-18`, `test.just:24-36`, `test.just:43-85`, `.github/workflows/ci.yml:40-83`, `.github/workflows/ci.yml:108-132`. Docker Compose defines only PostgreSQL. Source: `docker-compose.yml:1-15`.

The README lists only SQLite and PostgreSQL drivers, and there is no first-party MySQL example. Sources: `README.md:48-53`, `README.md:702-727`, `examples/rusqlite.rs:1-35`.

Minimum coverage should mirror the existing dialects:

- Unit SQL rendering tests for every builder state and MySQL-only clause.
- Shared integration tests executed through every supported MySQL client by `#[drizzle::test(mysql)]`.
- Type and macro compile-pass/compile-fail cases for selection, joins, casts, enums, nullability, feature gates, and invalid attributes.
- Migration DDL, diff, serializer, snapshot, introspection, and push round-trip tests once migrations enter scope. The current SQLite/PostgreSQL test families are listed in `migrations/tests/sqlite_ddl_generation.rs:1` and `migrations/tests/postgres_ddl_generation.rs:1`.
- A MySQL service and bare/full feature matrix in `test.just` and CI, plus one runnable example.

## Recommended parity boundary for the first implementation

The first usable slice should be complete vertically, not a broad collection of stubs:

1. MySQL types, dialect marker, values, identifier rendering, and SQL-only builders.
2. `MySQLTable`, `MySQLSchema`, generated models, `MySQLFromRow`, and the root prelude.
3. One production driver with `Drizzle::new`, `select`/`insert`/`update`/`delete`, `.execute()`/`.all()`/`.get()`, prepared queries, transactions, and savepoints where supported.
4. Integration and compile-fail coverage plus a CI service and feature matrix.
5. Relational `query` only after its identifier and JSON SQL paths have real MySQL branches.
6. Migration snapshots, DDL, diffing, introspection, and CLI workflows as a separate complete slice.

That boundary preserves the crate's existing developer experience. A user switches dialect imports, schema macros, and driver namespace. They do not relearn the query API.
