# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

Initial release will be automatically generated by semantic-release.