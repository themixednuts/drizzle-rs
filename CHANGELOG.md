# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.13](https://github.com/themixednuts/drizzle-rs/compare/v0.1.12...v0.1.13) - 2026-07-14

### Fixed

- resolve audit CI regressions
- harden migrations and type safety
- restore all-features builds and README setup

## [0.1.12](https://github.com/themixednuts/drizzle-rs/compare/v0.1.11...v0.1.12) - 2026-07-03

### Added

- *(macros)* DDL macro surface, attr-const docs, and constraint fixes
- *(migrations)* DDL correctness, new features, and a single renderer per dialect

### Changed

- unify dialect query builders on shared state machine

### Documentation

- update AGENTS.md test recipe references to just test:: module

### Fixed

- *(drivers)* typed errors, statement caching, savepoint cleanup, cancellation
- *(ci)* correct CLI push-SQL assertion and PostgresPolicy doctest shim

### Performance

- *(core)* slim SQL chunks, capacity heuristics, and shared builder infra

## [0.1.11](https://github.com/themixednuts/drizzle-rs/compare/v0.1.10...v0.1.11) - 2026-06-29

### Added

- align query APIs with rc

## [0.1.10](https://github.com/themixednuts/drizzle-rs/compare/v0.1.9...v0.1.10) - 2026-06-25

### Fixed

- *(sqlite)* use re-exported backend paths in macros

## [0.1.9](https://github.com/themixednuts/drizzle-rs/compare/v0.1.8...v0.1.9) - 2026-06-20

### Added

- support custom column comparison values

## [0.1.8](https://github.com/themixednuts/drizzle-rs/compare/v0.1.7...v0.1.8) - 2026-06-02

### Added

- add ORM comparison examples

### Fixed

- compare byte buffers as scalar expressions

## [0.1.7](https://github.com/themixednuts/drizzle-rs/compare/v0.1.6...v0.1.7) - 2026-05-20

### Fixed

- *(ci)* publish.yml verification no longer self-references
- *(migrations)* preserve column marker metadata

## [0.1.6](https://github.com/themixednuts/drizzle-rs/compare/v0.1.5...v0.1.6) - 2026-05-16

### Added

- db.migrate returns MigrateOutcome across all drivers
- *(migrations)* build::Config::from_toml + getters + auto-watch
- *(drizzle)* re-export Casing at the crate root
- *(postgres,core)* Postgres COLLATE + Constraint enum + expr-level collate()
- *(sqlite)* column-level COLLATE attribute
- align postgres codecs with drizzle rc1
- *(postgres)* AWS Aurora Data API driver
- *(sqlite)* Durable Objects transactions + savepoints, doc refresh
- *(core)* add sqlcommenter-style .comment() and .comment_tags()
- *(sqlite)* scaffold Cloudflare Durable Objects SQL driver (WASM-only)
- *(sqlite)* scaffold Cloudflare D1 driver (WASM-only)
- *(builder)* typestate deduplication and compile-time GROUP BY validation
- *(types)* improve SQL type system correctness
- *(migrations)* align folder layout and tracking upgrades
- *(migrations)* redesign runtime and build migration APIs
- *(core)* support &ZST tables/columns across macros and traits
- add percent_rank/cume_dist window functions and improve expr docs
- many-to-many relations through junction tables
- add `.first()` sugar on RelationHandle
- handle BLOB and boolean columns in query API JSON serialization
- move enum value conversions from table macro to enum derive
- compile-time view query DSL with const SQL generation
- support foreign key references on views for relational queries
- add relational query API with nested relation loading
- *(builder)* expand transaction builders with group_by, set ops, CTEs, and select_distinct
- *(core)* add tag! macro for defining aliases in one line
- *(builder)* add .value() shorthand for single-row inserts
- *(builder)* expand DrizzleBuilder with IntoSelect, set operations, group_by, having, and PG joins
- *(prepared)* add debug_assert for parameter count validation
- *(core)* constrain PostgreSQL cast() with Compatible bound
- *(core)* enforce Textual bound on group_concat()
- *(core)* enforce BooleanLike constraint on WHERE, HAVING, and logical operators
- close type propagation gaps for SQLite and PostgreSQL value layers
- *(prepared)* use const-generic arrays for parameter bindings
- *(postgres)* add time crate date/time type support
- *(prepared)* validate parameter bindings and add ParamSet
- *(postgres)* add rust-decimal Numeric value support
- *(types)* add Assignable trait and nullable typed placeholders
- *(bind)* add typed placeholders and remove params macros
- *(expr)* add typed subqueries with CASE/window support
- *(types)* strengthen marker inference and prevent Any widening
- *(postgres)* add dedicated markers for extended postgres types
- *(expr)* enforce dialect-aware scalar typing and strict decode
- *(sqlite)* reject implicit Any fallback in type inference
- *(select)* block untyped raw expressions in strict decode
- *(cast)* enforce sqlite compatibility for typed cast targets
- *(expr)* add dialect-aware aggregate result policy
- *(sqlite)* enforce dialect cast targets and strict affinity guards
- *(expr)* infer coalesce_many nullability for strict decode
- *(expr)* accept dialect marker values in cast targets
- *(types)* add sqlite/postgres type alias modules
- *(select)* enforce typed row-shape matching for strict decode
- *(select)* enforce strict row-shape checks for strict decode
- *(dml)* add typed returning markers with scoped validation
- *(row)* enforce selector table scopes at compile time
- *(alias)* introduce Tag-based aliases for tables and CTEs
- *(types)* improve row type inference across builders and macros
- *(push)* scope and normalize live snapshot for idempotent push
- *(push)* execute statements directly instead of returning them
- *(postgres)* add introspect and push to Postgres drivers
- *(sqlite)* add introspect and push to all SQLite drivers
- *(core)* add asc and desc order-by helpers
- *(cli)* add JSON import/export to `drizzle new`
- *(cli)* add `drizzle new` interactive schema builder
- *(test)* capture SQL, params, and source expression in test failure reports
- add no_std/alloc support to sqlite, postgres, and mysql crates
- add executor traits for async SQLite prepared statements
- *(tests)* add prepared statement transaction tests
- add SQL::map_params() and ToSQL<BorrowedValue> for OwnedPreparedStatement
- *(tests)* add async edge case tests for cloned Drizzle across tasks
- make Drizzle cloneable for sharing across async tasks
- auto-derive Debug for schema types in macros
- store schema in Transaction structs and add schema() accessor
- *(bench)* add tokio-postgres benchmarks and split CI runs
- *(tests)* add postgres transaction and savepoint tests
- store schema in Drizzle struct and derive Copy for schema types
- *(ci)* extract structured failure reports from turso test output
- *(examples)* add turso async driver example
- generate reverse Joinable impls for auto-FK chained joins
- add JoinedTable associated type to JoinArg for chained joins
- *(tests)* add many-to-many join tests and FK references to junction tables
- *(joins)* auto-derive join conditions from FK metadata
- *(constraints)* compile-time FK, PK, and constraint type system
- *(seed)* add deterministic database seeder with type-safe config
- *(relations)* auto-derive relation metadata from FK attributes
- *(savepoints)* add nested transaction support via SQL savepoints
- *(tracing)* add feature-gated tracing integration alongside puffin
- *(types)* add smallserial parity and flexible text/blob wrappers
- add datetime, math, and PG sequence expressions
- add aggregate expressions (total, every, json_object_agg)
- add string expressions (concat_ws, char_length, translate, regexp)
- add comparison expressions and dialect-aware greatest/least
- *(core)* propagate aggregate markers and enforce type constraints
- *(core)* add AggOr combinator trait for aggregate propagation
- *(sqlite)* add serde_json::Value conversions
- *(bind)* expand ValueTypeForDialect for all supported Rust types
- *(update)* add Empty/NonEmpty typestate to prevent empty updates and improve FK/PK error spans
- compile-time SQL generation for PostgreSQL tables, indexes, and enums
- *(procmacros)* add PostgreSQL enum FromSql/ToSql and feature-gate cleanup
- *(types)* add distinct sqlite/postgres marker modules
- *(postgres)* align rc2 type normalization
- *(postgres)* add `Enum` variant to `OwnedPostgresValue`
- *(postgres)* fill AWS Data API row impls for chrono FixedOffset/Duration and time crate
- *(migrations)* add postgres introspection queries to migrations crate
- *(migrations)* add generate() for programmatic migration diffing
- *(cli)* add parity overrides and migration planning modes
- *(types)* lift EnvOr/EnvOrError into drizzle-types::migration
- *(cli)* add cargo-binstall metadata
- *(cli)* Cloudflare D1 HTTP driver for the CLI
- *(cli)* emit migrations.js bundle index on generate when configured
- *(cli)* add JSON Schema derives for config types

### Changed

- *(tx)* centralize savepoint logic in shared module
- *(builder,tx)* collapse DrizzleBuilder + TransactionBuilder into one
- *(tx)* collapse per-driver Postgres TransactionBuilder behind generic
- *(tx)* collapse per-driver SQLite TransactionBuilder behind generic
- *(tx)* macro-driven typestate for SQLite TransactionBuilder delete
- *(procmacros,tests)* migrate test macros to #[drizzle::test] attribute with DI
- *(core,procmacros)* pack ColumnRef metadata into ColumnFlags byte
- *(migrations)* identify migrations by name, not created_at
- simplify migration APIs
- *(sqlite)* remove unchecked decode feature path
- convert query extension traits to inherent methods, add LATERAL fix
- rename `.select_columns()` to `.columns()` on table ZSTs
- replace dyn trait objects with const Copy structs for schema metadata
- use file-based temp DBs for fully parallel SQLite tests
- schema-centric architecture with compile-time SQL and metadata traits
- add Clauses typestate to prevent duplicate query clauses
- deduplicate transaction query builder constructors with macros
- replace row() wrapper with direct tuple support in in_subquery
- restructure justfile with test module system
- *(tests)* replace manual result structs with SelectSimple and add DB validation
- *(types)* normalize impl ordering to canonical type order
- *(types)* replace core SQL type markers with dialect-resolved types
- *(alias)* remove runtime alias_named in favor of tag aliases
- *(postgres)* consolidate snapshot push API and deduplicate nextval parsing
- rename expressions() path helper and clean up prelude imports
- consolidate expressions module into expr
- *(test)* remove DDL from EXECUTED STATEMENTS
- *(test)* migrate postgres tests to drizzle_exec! => pattern
- *(test)* migrate sqlite tests to drizzle_exec! => pattern
- make prepared statement inner fields pub(crate)
- rename connection accessors to conn()/conn_mut() and inner()
- rename SQLSchema::sql() to ddl() for clarity
- *(tokio-postgres)* use collect for param_refs and restructure profiling scopes
- *(examples)* move imports inside main and add clone demo
- migrate transaction closures from Box::pin to AsyncFnOnce
- *(tests)* improve assertions and remove debug output
- *(tests)* fix feature-gate warnings and clean up imports
- *(tests)* consolidate feature-gated schema definitions
- *(tests)* clean up test infrastructure and remove dead code
- *(tests)* migrate compile-fail tests from trybuild to doc tests
- *(seed)* feature-gate backends and refine public API
- *(insert)* restructure ON CONFLICT with builder pattern
- *(core/row)* tighten row-decoding for sqlite + postgres
- *(core)* unify libsql + turso row decoding behind SqliteValueRow
- *(no_std)* full workspace audit, const fn, and prelude cleanup
- *(core)* macro-ify `ValueTypeForDialect` impls in bind.rs
- *(core)* share `checked_float_to_int` helper across dialects
- *(core)* add ParamStyle to decouple placeholder syntax from dialect
- *(types)* derive expr markers from resolved column types
- *(types)* move SQL marker system into drizzle-types
- *(alias)* switch typed aliases to generic Tagged wrapper
- *(shared)* consolidate CTE and JoinArg macro implementations
- *(core)* remove dead markers and unused migration helpers
- *(core)* replace manual tuple impls with recursive accumulator macros and add diagnostic error messages
- *(macros)* factor model-marker generation, slim view + insert codegen
- *(macros)* drop is_primary/is_unique bools from FieldInfo
- *(macros)* Constraint enum for SQLite primary/unique state
- *(macros)* unify Postgres DDL emission with SQLite pattern
- *(macros)* unify SQLite DDL emission into single fragment emitter
- *(procmacros)* dedupe view flag packing and test assertion macros
- use const SQL in SQLite schema create_statements for consistency
- clean up proc macro compile error messages
- deduplicate constraint generation between SQLite and PostgreSQL proc macros
- *(macros)* replace too-many-args with config structs
- *(migrations)* extract Snapshot<E> + canonical EntityCollection
- make migration version helpers const fn
- remove unused VersionLt trait
- *(types)* share casing and migration tracking config
- apply let-chains and match-style rewrites (MSRV 1.95)
- zero-copy seed crate by replacing String keys with &'static str
- *(cli)* centralize command harness boilerplate
- *(cli)* use canonical queries from migrations crate

### Documentation

- *(readme)* comprehensive cleanup + runnable smoke test
- *(readme)* regroup query features under single Querying H2, lift Migrations out of Getting Started, compress duplicated sections
- *(readme)* update group_by syntax and add condition examples
- *(readme)* clarify migration workflow
- tighten README flow
- *(doctest)* convert ignored fragments to runnable rust examples
- add query type alias example to README
- clean up README section ordering and remove internal details
- add trait import note for relational queries
- add relational query API to README
- add expressions, group by, and set operations to README
- clean up jargon, add missing doc comments, hide internal types
- overhaul README with generated models, CLI reference, and runtime migrations
- restructure README with getting started guide and CLI workflow
- add transaction, savepoint, and prepared statement examples to driver modules
- add transactions and savepoints section to README
- point install instructions to git repo instead of crates.io
- *(readme)* update join syntax and add order_by/limit/offset
- move project instructions to AGENTS.md and add git hunk tips
- *(readme)* update to 0.1.5 and align postgres schema example
- *(types)* add diagnostic::on_unimplemented to CountPolicy and ArithmeticOutput
- replace todo/ignore in prepared statement doctests with compilable scaffolds
- *(postgres)* add missing types module to doctest scaffolds
- *(migrations)* update crate-level docs with compilable examples

### Fixed

- *(docs)* repair pre-existing broken postgres doc tests
- address ci benchmark and lint failures
- *(transaction)* decouple query lifetimes from tx borrows
- *(tests)* drop unneeded wildcard fields covered by `..` in postgres/query
- *(ci)* drop libsql from Linux benchmarks too
- *(ci)* drop libsql from macOS benchmarks
- *(bench)* skip turso RETURNING benchmark
- *(ci)* use per-OS feature matrix for sqlite benchmarks
- *(test)* update sqlite returning doctest
- *(macros)* resolve optional-feature schema and helper compile errors
- satisfy pre-commit checks for ref trait impls
- update compile_fail stderr files for Deref type path changes
- make greatest/least PostgreSQL-only instead of mapping to SQLite MAX/MIN
- filter views query by schema during push to avoid concurrent DDL failures
- regenerate trybuild .stderr snapshots for current compiler
- query API param ordering, codegen, and error improvements
- increase prepared statement perf test tolerance to 5x
- feature-gate trybuild tests requiring uuid extension types
- use TransactionError and From for PG transaction error handling
- filter null elements from json_group_array and improve query API docs
- *(ci)* gate raw_sql_ui trybuild test on uuid feature
- resolve CI failures from missing ToSQL import and cross-driver catch_unwind
- use drizzle_catch_unwind! macro for cross-driver prepared statement tests, correct sign() return type
- *(builder)* enforce BooleanLike on ON CONFLICT WHERE clauses
- *(macros)* use specific driver features for postgres proc macro codegen
- *(postgres)* decode optional select models via NullProbeRow
- *(expr)* infer non-null coalesce/ifnull result nullability
- *(test)* enable foreign_keys pragma in SQLite test setup
- *(test)* capture Turso failures in structured reports
- *(test)* improve failure report readability
- *(tests)* wrap async prepared calls in drizzle_try!() for transactions
- *(tests)* update transaction tests for AsyncFnOnce closure syntax
- *(tokio-postgres)* remove Send bound from transaction future closures
- *(tests)* improve wrap_text and long test name rendering in failure reports
- *(examples)* feature-gate rusqlite example imports
- *(postgres)* convert doc tests from ignore to no_run with mod drizzle shim
- *(tests)* correct FK type mismatch in Post.author_id
- *(parity)* align sqlite metadata DDL and serde-gated test structs
- *(migrations)* track applied entries by created_at
- *(postgres)* improve alias ergonomics and add regressions
- *(sqlite)* decode named FromRow structs by column
- *(fromrow)* decode named postgres structs by column
- *(ci)* add workflow_dispatch trigger to release workflow
- *(core)* escape embedded double-quotes in SQL identifiers
- correct nullability in greatest/least and setval, fix sign doc
- remove extraneous Mo::Nullable bound in make_timestamp
- propagate nullability and aggregate kind in make_timestamp and date_bin
- rename duplicate 'Distinct Wrapper' section header in agg.rs
- ensure inner subquery exposes FK columns needed by nested relations
- emit compile error for invalid composite FK fields instead of silent fallback
- correct PG param ordering, UTF-8 safety, and query API robustness
- eliminate unnecessary string allocations in SQL generation
- *(core)* detect f32 overflow in SQLite FromRow conversions
- *(types)* complete tuple type propagation support
- use impl Iterator for create_statements to auto-derive Send
- *(macros)* resolve const_format path via proc-macro-crate
- expose postgres driver types in doctests
- *(ci)* align macro doctest feature wiring and time ISO formatting
- *(doctest)* wire feature-gated macro docs across drivers
- prevent duplicate `From` impls for `#[derive(PostgresEnum)]` fields
- use unwrapped base type for partial select model fields
- *(fromrow)* generate offset-aware row decoding in macros
- *(postgres)* prevent SERIAL columns from getting GENERATED AS IDENTITY
- *(test)* capture prepared statement SQL and params in reports
- *(fromrow)* fix turso JSON field temporary lifetime error
- *(ddl)* harden SQL generation and migration execution
- *(sqlite)* add shim to on_conflict doc test so it compiles with no_run
- *(postgres)* propagate scope markers in join_using methods
- *(postgres)* move schema() to SQLTableInfo impl in refresh test
- *(lint)* remove redundant .into_iter() in topological sort
- *(postgres)* fix PgTypeCategory misclassifying integer as serial
- *(postgres)* harden sequence and index introspection queries
- *(codegen)* map BOOLEAN SQL type to bool in SQLite introspection
- *(types)* always serialize Option fields in DDL snapshot structs
- *(seed)* split dialect imports so single-feature builds resolve
- *(cli)* bind view schema filter when introspecting postgres views
- remove spurious bind parameter from VIEWS_QUERY calls
- *(cli)* cfg-gate driver-dependent code and suppress warnings

### Other

- *(sqlite)* add raw prepared parity across drivers
- add postgres suite and puffin scope reporting

### Performance

- *(sqlite)* preallocate async prepared bind params
- *(drivers)* optimize decode paths and add fluent rows cursors
- *(core)* reduce SQL render allocations and bind overhead

## [0.1.5](https://github.com/themixednuts/drizzle-rs/compare/v0.1.4...v0.1.5) - 2026-02-11

### Added

- *(schema)* return Result from create_statements and detect duplicates
- *(values)* add UpdateValue types for SQLite and PostgreSQL
- *(sqlite)* align pragma API with SQLite docs
- *(postgres)* add FOR UPDATE/SHARE row locking
- *(postgres)* add array operators (@>, <@, &&)
- *(core)* implement Expr and ToSQL for Placeholder
- *(core)* add SQL::assignments_sql() for pre-built SQL fragments
- *(values)* support Box/Rc/Arc conversions
- *(postgres)* add materialized view refresh and builder enhancements
- *(macros)* add #[json] support to rusqlite FromRow and unify error types
- *(macros)* generate UpdateValue-based update models
- *(postgres)* add PgArray wrapper and Vec<T> array conversions
- *(migrations)* add SQLite view codegen and PostgreSQL view alterations

### Fixed

- *(ci)* use env var for commit message in skip check
- *(macros)* enforce deterministic schema ordering and clearer failures
- *(macros)* always generate FromSQLiteValue for enums
- *(macros)* quote column names and fix default value escaping in DDL
- *(sqlite)* use explicit import for SQLiteValue in tests
- *(migrations)* remove trailing comma in PostgreSQL schema table identifier
- *(schema)* restructure JSON schema for tombi compatibility

### Other

- include all sub-crate commits in main changelog
- apply cargo fmt across workspace
- rewrite README with cleaner flow and idiomatic schema.rs pattern
- add rusqlite JSON deserialization test and feature-gate schema modules
- clean up redundant dev-dependencies
- *(builder)* unify CTE conversion API and reduce state duplication
- *(core)* remove dead SQLComparable trait
- *(core)* clean up Placeholder and ParamBind APIs
- *(sqlite)* add placeholder update tests
- *(deps)* update Cargo.lock
- *(readme)* remove redundant sections
- remove re-exports and use explicit module paths
- *(readme)* use GitHub alert syntax for note
- *(readme)* expand examples and simplify install instructions
- *(postgres)* use boxed future for async transaction callbacks
- *(postgres)* use transaction builder for sync driver
- *(lib)* simplify module documentation
- remove section divider comments from builder and transaction modules
- make release-plz update manual only
- *(core)* add schema namespace to SQLTableInfo and unify view hierarchy
- *(core)* harden trait contracts and remove unsafe defaults
- *(core)* remove unused SQLExpr::as_sql()
- *(core)* add into_sql() to ToSQL trait and adopt across expr layer
- *(core)* optimize SQL internals and unify spacing logic
- *(core)* remove dead SQLChunk::Alias variant
- *(macros)* move has_json_attribute to shared helpers
- *(macros)* improve type checking and error handling in proc macros
- *(traits)* split DrizzleRow and add checked numeric conversions
- *(macros)* remove dead code and add targeted warning suppressions
- *(macros)* extract shared enum discriminant parsing into enum_utils
- *(macros)* unify ModelType usage across postgres and sqlite
- apply rustfmt and fix clippy warnings
- *(postgres)* optimize view SQL generation with write! macro

## [0.1.4](https://github.com/themixednuts/drizzle-rs/compare/v0.1.3...v0.1.4) - 2026-01-24

### Added

- add view support and CTE improvements
- *(view)* add macros, metadata, and markers
- *(builders)* implement Expr trait for query builders
- *(turso)* include SQL in error messages
- *(tests)* add structured failure reports with SQL capture
- *(cli)* add multi-database support and working migrations
- add standalone CLI crate with TOML configuration
- improve prepared statements, dialect system, and query builders
- *(postgres)* add PreparedStatement execute/all/get methods
- export ON_DELETE/ON_UPDATE and referential action markers in preludes
- *(tokio-postgres)* add client_mut method to Drizzle
- export attribute markers for IDE support
- *(schema)* add missing files for column builder implementation
- *(cli)* add drizzle CLI for migration management
- implement postgres-sync and tokio-postgres driver modules
- add tokio-postgres async test driver support
- add Docker Compose setup for local PostgreSQL testing
- *(migrations)* add compile-time embedded migrations
- *(drizzle)* add db.migrate() method to SQLite drivers
- *(macros)* generate table metadata for migrations
- add arrayvec support for SQLite and PostgreSQL

### Fixed

- *(cte)* relax builder bounds and update tests
- *(sqlite)* import prepared macro from prepared_common
- *(expr)* preserve type in null functions
- *(ci)* use postgres-sync feature (includes postgres dialect)
- *(procmacros)* remove dev-dependencies that leaked features
- *(tests)* gate turso module on turso feature only
- *(cli)* support multi-dialect schema files
- re-export drizzle_migrations for macro-generated code
- *(lifetimes)* change lifetimes to better support closure lifetimes
- *(tests)* update tests to handle iterator-based params()
- fixing postgres macro
- resolve clippy warnings with all features enabled
- resolve clippy warnings with nightly

### Other

- *(release-plz)* disable changelog for drizzle-types and drizzle-migrations
- consolidate tests into 2 jobs
- improve update and publish workflows
- use impl ToSQL for builder methods
- *(view)* cover defaults and existing views
- update README and examples/benches for refactor
- unify table pipeline + improve diagnostics/type mapping
- refactor root builders to shared driver/common modules
- add release workflow and scripts
- update README and add project config files
- update types and examples
- update and add compile-fail tests
- *(drivers)* update driver implementations
- add compile-fail tests for type safety
- improve README and fix sqlite doctests
- clean up module structure and remove dead code
- simplify and clean up GitHub Actions workflows
- *(parser)* modularize schema parser with nom combinators
- lint(fmt)
- *(turso)* parse test failures and save as artifacts
- *(tests)* use structured assertion macros
- fix imports in driver module examples
- lint(fmt)
- lint(clippy)
- add Zed editor settings
- update workspace dependencies
- update benchmarks and examples for refactored API
- *(cli)* update snapshot builder for new migrations API
- update tests for refactored API
- *(drizzle)* reorganize transaction and builder modules
- *(procmacros)* reorganize module structure
- *(migrations)* remove config module and update for new API
- *(postgres)* reorganize module structure
- *(sqlite)* reorganize module structure and add connection helper
- *(core)* reorganize module structure and clean up traits
- *(types)* update DDL types for Cow-based API
- *(procmacros)* add const validation for foreign key references
- *(procmacros)* use const blocks for index column name validation
- *(migrations)* update tests for new Cow-based API
- update Cargo.lock with dependency changes
- add missing core module imports to tests and benchmarks
- simplify documentation and clean up public API
- remove CLI module
- *(types)* add a shared types crate
- *(README)* update postgres json
- fix PostgreSQL example to use correct API
- *(postgres)* add module-level documentation with examples for PostgreSQL drivers
- *(sqlite)* add module-level documentation with examples for SQLite drivers
- remaining module updates
- miscellaneous updates and fixes
- *(postgres)* add PostgreSQL prepared statement tests
- remove columns module from dialect crates
- *(postgres)* Add PostgreSQL foreign keys tests
- Update Cargo.lock
- *(sqlite)* Update SQLite foreign keys tests
- Update main lib and libsql module
- *(postgres)* Add JSON/JSONB custom struct roundtrip tests
- add on_delete = cascade example to sqlite test schema
- update README with new API syntax
- remove sqlite/postgres preludes within base prelude
- migrate to unified column attribute syntax
- update feature flags and add type inference dependencies
- add postgres errors
- *(tests)* update tests to use driver-specific preludes
- wip add from row for postgres select models
- refactor to reduce trait conflicts
- update docs for FromRow; update macro paths
- adjust arrayvec tests/impl
- *(cli)* use workspace deps and fix deprecations
- format postgres values.rs and fix unused variable
- comprehensive README rewrite with validated API documentation
- update test common module and rusqlite example
- update SQLite tests for consistency
- convert PostgreSQL tests to end-to-end assertions
- *(postgres)* add arrayvec integration tests
- update postgres test expectation to use numbered placeholder
- *(postgres)* add comprehensive PostgreSQL test suite
- rename drizzle-schema to drizzle-migrations

