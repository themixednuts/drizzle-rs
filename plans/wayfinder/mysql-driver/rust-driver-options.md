# Rust MySQL driver options

Researched on 2026-08-25. This note covers the wire drivers that can sit under
the drizzle-rs MySQL dialect. It does not choose names or shape the public API.

## Recommendation

Use both current Blackbeam drivers:

```toml
mysql = { version = ">=28.0.0, <29", default-features = false, features = ["minimal-rust", "native-tls"] }
mysql_async = { version = ">=0.37.0, <0.38", default-features = false, features = ["minimal-rust", "native-tls-tls"] }
```

`mysql` is the practical blocking driver and `mysql_async` is the practical
Tokio driver. Their execution APIs differ, but both use the same
`mysql_common` value, parameter, row, and conversion types. The current
releases depend on compatible `mysql_common 0.37.x` ranges, so Cargo resolves
one conversion layer for both adapters. `mysql 28.0.0` requires
`mysql_common ^0.37.0`; `mysql_async 0.37.0` requires
`mysql_common ^0.37.1`. See the official package metadata for
[`mysql 28.0.0`](https://docs.rs/crate/mysql/28.0.0) and
[`mysql_async 0.37.0`](https://docs.rs/crate/mysql_async/0.37.0/source/Cargo.toml).

The native TLS choice is the least disruptive starting point for this
repository. The workspace already selects `native-tls >=0.2.18,<0.3` with its
`vendored` feature, and both drivers accept `native-tls 0.2`. The async driver
adds `tokio-native-tls`. This is an implementation dependency choice, not a
proposal for a public connection API. See the
[workspace manifest](../../../Cargo.toml) and the drivers' official feature
lists for [`mysql`](https://docs.rs/mysql/28.0.0/mysql/#crate-features) and
[`mysql_async`](https://docs.rs/mysql_async/0.37.0/mysql_async/#crate-features).

Do not expose native TLS and rustls as independent additive features without a
different build arrangement. Enabling both backends at once fails to compile
in `mysql 28.0.0`, which conflicts with drizzle-rs's normal
`--all-features` gate. A local Rust 1.95 check reproduced duplicate `Secure`,
`ClientIdentity`, `TlsError`, and `make_secure` definitions. The upstream
source selects the two implementations with independent Cargo feature gates:
[`mysql` TLS stream source](https://docs.rs/mysql/28.0.0/src/mysql/io/mod.rs.html)
and
[`mysql_async` TLS modules](https://docs.rs/mysql_async/0.37.0/src/mysql_async/io/tls/mod.rs.html).

## Repository baseline

The workspace currently has only the published `drizzle-mysql` stub. Neither
`mysql`, `mysql_async`, nor `mysql_common` appears in the workspace dependency
table or lockfile. The stub has no wire-driver dependency. See
[`mysql/Cargo.toml`](../../../mysql/Cargo.toml),
[`mysql/src/lib.rs`](../../../mysql/src/lib.rs), and
[`Cargo.lock`](../../../Cargo.lock).

The fit with the existing driver layout is straightforward. PostgreSQL has a
blocking adapter and a Tokio adapter behind separate features. SQLite does the
same for its supported clients. The MySQL pair can follow that internal split
without inventing a reduced query API. The existing adapter manifests are
[`postgres/Cargo.toml`](../../../postgres/Cargo.toml) and
[`sqlite/Cargo.toml`](../../../sqlite/Cargo.toml).

## Driver comparison

| Concern | `mysql 28.0.0` | `mysql_async 0.37.0` | drizzle-rs consequence |
|---|---|---|---|
| Execution | Blocking `Conn`, `PooledConn`, and `Transaction` implement `Queryable`. `exec_iter` accepts `Into<Params>`. [Official `Queryable` API](https://docs.rs/mysql/28.0.0/mysql/prelude/trait.Queryable.html) | Tokio-based `Conn`, pool, and transaction APIs. `exec_iter`, `prep`, and `close` return futures. [Official `Conn` API](https://docs.rs/mysql_async/0.37.0/mysql_async/struct.Conn.html) | Keep separate sync and async executors, as drizzle-rs already does for PostgreSQL. Share SQL generation and value conversion. |
| Parameters | `Params::Positional(Vec<Value>)` and `Into<Params>` are supported. [Official `Params` API](https://docs.rs/mysql/28.0.0/mysql/enum.Params.html) | The same three `Params` variants and conversions come from `mysql_common`. [Official `Params` API](https://docs.rs/mysql_async/0.37.0/mysql_async/enum.Params.html) | Convert the builder's ordered values to `Params::Positional`. Do not route through tuple conversions, which stop at arity 12. |
| Protocol | `exec*` uses prepared statements and the binary protocol. `query*` uses the text protocol, whose result values arrive as bytes. [Official protocol notes](https://docs.rs/mysql/28.0.0/mysql/#binary-protocol-and-prepared-statements) | The same distinction applies. Rust values can only be bound through prepared statements, with `?` placeholders. [Official protocol notes](https://docs.rs/mysql_async/0.37.0/mysql_async/#binary-protocol-and-prepared-statements) | Use `exec*` for rendered Drizzle queries, including `Params::Empty` when typed binary decoding matters. Never interpolate values into SQL. |
| Rows | `Row` offers non-panicking `get_opt` and `take_opt` by index or name. [Official `Row` API](https://docs.rs/mysql/28.0.0/mysql/struct.Row.html) | It re-exports the same `mysql_common::Row`. [Official crate exports](https://docs.rs/mysql_async/0.37.0/mysql_async/) | Implement drizzle-rs's offset-based row decoder on the shared row type. Use `get_opt` or `take_opt`, then map `FromValueError` into `DrizzleError`. Avoid the panicking `get`, `take`, and `from_row` paths. |
| Transactions | `start_transaction(TxOpts)` is synchronous. `TxOpts` covers isolation, access mode, and consistent snapshots. Drop performs an immediate rollback. [Official `TxOpts`](https://docs.rs/mysql/28.0.0/mysql/struct.TxOpts.html) and [drop source](https://docs.rs/mysql/28.0.0/src/mysql/conn/transaction.rs.html) | `start_transaction` and commit or rollback are async. Dropping schedules implicit rollback when the connection is next queried or returned to the pool, so rollback may be delayed. Nested transactions are rejected. [Official transaction docs](https://docs.rs/mysql_async/0.37.0/mysql_async/#transaction) | Give each driver its own transaction wrapper. Explicit commit and rollback are important for the async path. Drizzle savepoints should remain SQL inside the active transaction rather than calling nested `start_transaction`. |
| Pool behavior | The crate includes a thread-safe cloneable pool and blocking connection checkout. [Official crate docs](https://docs.rs/mysql/28.0.0/mysql/#pool) | `Pool` is lazy, `Send + Sync + 'static`, binds background tasks to the first runtime used, and needs explicit `disconnect` for graceful shutdown. [Official pool docs](https://docs.rs/mysql_async/0.37.0/mysql_async/struct.Pool.html) | An async adapter must not hide pool shutdown semantics or move one pool between short-lived runtimes. Accepting existing `Conn` or pool-backed connections should preserve upstream ownership rules. |
| Runtime | No async runtime. Network work blocks the calling thread. | Always pulls Tokio features for I/O, filesystem access, networking, time, runtime, and synchronization. [Official manifest](https://docs.rs/crate/mysql_async/0.37.0/source/Cargo.toml) | The current workspace Tokio range, `>=1.48,<2`, satisfies the driver's `tokio ^1.0` requirement. Keep the async dependency optional so blocking-only users do not pull Tokio networking. |
| Compression baseline | `minimal-rust` selects the Rust `flate2` backend. Defaults also add derive macros and a global buffer pool. [Official feature docs](https://docs.rs/mysql/28.0.0/mysql/#crate-features) | `minimal-rust` selects the Rust `flate2` backend. Defaults also add derive macros. [Official feature docs](https://docs.rs/mysql_async/0.37.0/mysql_async/#crate-features) | `minimal-rust` avoids a system zlib dependency. The Drizzle macros own model generation, so upstream derive macros are unnecessary. The sync buffer pool can be evaluated separately instead of arriving through defaults. |
| TLS | No TLS backend is enabled by the default feature set. The crate supports native TLS or one rustls provider. [Official SSL docs](https://docs.rs/mysql/28.0.0/mysql/#ssl-support) | No TLS backend is enabled by default. It supports native TLS or rustls with an explicit crypto provider. [Official TLS docs](https://docs.rs/mysql_async/0.37.0/mysql_async/#tlsssl-support) | Pick one backend across the workspace. Native TLS compiles with the repository's existing dependency. If the project later moves to rustls, replace the choice instead of adding a second normal feature that `--all-features` combines. |
| MSRV declaration | No `rust-version` is published. `cargo info mysql@28.0.0` reports `unknown`. [Published manifest](https://docs.rs/crate/mysql/28.0.0/source/Cargo.toml) | No `rust-version` is published. `cargo info mysql_async@0.37.0` reports `unknown`. [Published manifest](https://docs.rs/crate/mysql_async/0.37.0/source/Cargo.toml) | Metadata cannot guarantee Rust 1.95. Pin compatible release ranges and keep an MSRV job for every driver feature set. The current releases were tested locally under 1.95 as described below. |

## Value and row conversion

Both drivers expose `mysql_common::Value` with `NULL`, bytes, signed and
unsigned integers, float, double, date, and time variants. The same crate owns
`FromValue`, `FromRow`, `Params`, and `Row`, so the MySQL dialect needs one
conversion policy rather than separate sync and async policies. See the
[`Value` variants](https://docs.rs/mysql/28.0.0/mysql/enum.Value.html) and
[`FromRow` behavior](https://docs.rs/mysql_common/0.37.3/mysql_common/row/convert/trait.FromRow.html).

Practical rules for the adapter:

- Build `Params::Positional(Vec<Value>)` in SQL placeholder order. This avoids
  the tuple arity limit and matches Drizzle's existing ordered parameter list.
- Decode each selected column through the non-panicking row API. Whole-row
  `from_row` is a poor fit because it panics on mismatched nullability,
  signedness, width, or UTF-8 and supports tuples only through arity 12.
- Forward the shared upstream features for `chrono`, `time`, and
  `rust_decimal` when the matching drizzle-rs type feature is enabled. Both
  wire drivers proxy those features to `mysql_common`. JSON and UUID
  conversions already exist in `mysql_common`. See its
  [supported Rust types](https://docs.rs/mysql_common/0.37.3/mysql_common/#supported-rust-types).
- Test `DECIMAL` boundaries separately. Upstream documents that
  `rust_decimal::Decimal` does not cover the full MySQL `DECIMAL` range. The
  adapter must return a conversion error for out-of-range values rather than
  truncate them. [Official conversion table](https://docs.rs/mysql_common/0.37.3/mysql_common/#supported-rust-types)
- Keep MySQL's signed and unsigned integer variants distinct. This matters for
  `UNSIGNED` columns and avoids routing every integer through `i64`.

## MSRV checks performed

The following exact releases compiled together on Windows MSVC with
`cargo +1.95.0 check`:

```text
mysql = 28.0.0, default-features = false, features = ["minimal-rust"]
mysql_async = 0.37.0, default-features = false, features = ["minimal-rust"]
```

Cargo resolved one `mysql_common 0.37.3`. The check also proved the async
driver against a Rust-1.95-compatible `tokio 1.53.1` resolution.

The recommended native TLS combination also passed under Rust 1.95:

```text
mysql = 28.0.0, features = ["minimal-rust", "native-tls"]
mysql_async = 0.37.0, features = ["minimal-rust", "native-tls-tls"]
```

Cargo resolved `native-tls 0.2.18`, which matches the current workspace pin.
This proves the dependency set compiles at the repository's declared floor. It
does not replace live integration tests against a server or target checks for
Linux and macOS.

As a negative check, enabling native TLS and rustls together failed in
`mysql 28.0.0` with duplicate TLS types and methods. Treat a single selected
TLS backend as a build invariant.

## Test setup needed

The repository's Docker Compose file currently defines PostgreSQL only. Add a
MySQL service and wait for its health check before either driver suite runs.
See [`docker-compose.yml`](../../../docker-compose.yml).

Use MySQL 8 as the first compatibility target. The `mysql_async` maintainers
run their tests against `mysql:8.0` and document a default URL of
`mysql://root:password@127.0.0.1:3307/mysql`. Their server command also enables
large packets, local infile, and binary logging for the driver's full upstream
suite. Drizzle-rs does not need every one of those server flags for ordinary
query-builder tests, but using the same image and authentication generation is
a known-good connection baseline. See the
[official testing instructions](https://github.com/blackbeam/mysql_async#testing).

The drizzle-rs integration matrix should cover both wire drivers against the
same schema and assertions:

- connection, prepared parameter binding, and statement cache reuse;
- insert, select, update, delete, joins, aggregates, aliases, limits, and
  MySQL-specific conflict behavior;
- signed and unsigned integers, nulls, text, binary data, JSON, UUID, dates,
  times, decimals, enum values, and invalid conversion errors;
- explicit commit, explicit rollback, drop rollback, savepoints, affected row
  counts, and auto-increment IDs;
- a TLS connection for the chosen backend, plus an expected certificate or
  hostname failure;
- each driver feature alone, both drivers together, workspace
  `--all-features`, and Rust 1.95.

Add MariaDB only when the project explicitly promises MariaDB compatibility.
The two client crates understand both MySQL and MariaDB protocol details, but
that does not make every Drizzle SQL construct portable between the server
dialects.

## Bottom line

The Blackbeam pair matches drizzle-rs's existing feature-gated sync and async
architecture without changing the query-builder API. Their shared
`mysql_common` layer is the main advantage. It lets both adapters share value
conversion and row decoding while retaining separate execution and
transaction code. Start with `mysql 28.0.0`, `mysql_async 0.37.0`,
`minimal-rust`, and one TLS backend. Keep the exact two-driver and MSRV checks
in CI so a future upstream release cannot split the common types or raise the
toolchain floor unnoticed.
