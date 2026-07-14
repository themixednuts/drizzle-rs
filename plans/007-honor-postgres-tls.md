# Plan 007: Honor PostgreSQL TLS with structured credentials

> **Executor instructions**: Follow every step and command. Stop and report on
> any STOP condition; do not weaken certificate verification to make tests
> pass. Update the 007 row in `plans/README.md` when complete.
>
> **Drift check (run first)**: `git diff --stat 12495f36..HEAD -- Cargo.toml Cargo.lock cli/Cargo.toml cli/src/config.rs cli/src/commands/overrides.rs cli/src/db cli/src/error.rs test.just docker-compose.yml plans/README.md`
> Compare any changed regions with the excerpts below. Semantic drift is a STOP.

## Status

- **Priority**: P1
- **Effort**: L
- **Risk**: MED
- **Depends on**: `plans/001-verify-compile-fail-diagnostics.md`
- **Category**: security
- **Planned at**: commit `12495f36`, 2026-07-12

## Why this matters

Host credentials retain an `ssl` flag, but all PostgreSQL operations build a
URL by hand and pass `NoTls`; TLS requests are therefore ignored or fail
misleadingly. Hand formatting also mishandles reserved characters in
credentials and database names. Parse/build `tokio_postgres::Config`, retain
the requested SSL mode, and route every sync/async operation through shared
connector helpers with explicit certificate policy.

## Current state

- `cli/src/config.rs:479-512` defines:

```rust
pub enum PostgresCreds {
    Url(Box<str>),
    Host { host: Box<str>, port: u16, user: Option<Box<str>>,
           password: Option<Box<str>>, database: Box<str>, ssl: bool },
}
```

and `connection_url()` concatenates `user:password@host/database` without URL
encoding while ignoring `ssl`.
- Config parsing (`cli/src/config.rs:634-651`) converts bool/string SSL values
  to only enabled/disabled. CLI overrides accept `require`, `allow`, `prefer`,
  `verify-full`, and `verify-ca` but collapse them to bool
  (`cli/src/commands/overrides.rs:228-245`).
- PostgreSQL call sites in `cli/src/db/mod.rs` repeatedly use
  `postgres::Client::connect(&url, postgres::NoTls)` or
  `tokio_postgres::connect(&url, tokio_postgres::NoTls)` (at least lines 1315,
  1356, 1452, 1512, 1598, 1653, 2562, 2607, 3663, and 3957, plus tests).
- Workspace versions are `postgres >=0.19.11,<0.20` and
  `tokio-postgres >=0.7.15,<0.8`.
- Official rust-postgres documentation confirms both clients accept an
  external `MakeTlsConnect`; `postgres-native-tls` is the project-provided
  connector for both sync and async clients. Read before implementation:
  <https://docs.rs/postgres/latest/postgres/#ssltls-support> and
  <https://docs.rs/postgres-native-tls/latest/postgres_native_tls/>.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Config unit tests | `cargo test -p drizzle-cli --all-features config` | all pass |
| Override unit tests | `cargo test -p drizzle-cli --all-features overrides` | all pass |
| PostgreSQL integration | `just test::pg "postgres-sync,tokio-postgres,uuid,serde"` | all pass |
| TLS integration | `just test::pg-tls` (add in this plan) | sync and async clients prove encrypted connections; negative verification case passes |
| MSRV | `cargo +1.95.0 check --workspace --all-targets --all-features` | exit 0 |
| Release targets | `cargo check -p drizzle-cli --all-features --release` | exit 0 |
| Lint | `cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |

## Scope

**In scope**:

- Root `Cargo.toml`/`Cargo.lock` and `cli/Cargo.toml` for TLS connector deps
- `cli/src/config.rs`
- `cli/src/commands/overrides.rs`
- `cli/src/db/mod.rs` and a new private `cli/src/db/postgres_connection.rs`
- `cli/src/error.rs` if structured connection/config errors need mapping
- PostgreSQL tests, `test.just`, and Docker test configuration needed for an
  ephemeral TLS-enabled PostgreSQL service
- `plans/README.md` (status row only)

**Out of scope**:

- TLS APIs in the public ORM driver crates
- Connection pooling, retries, or timeout policy
- Adding client-certificate authentication
- Persisting credentials or printing connection URLs
- Weak “accept any certificate” production defaults
- Changes to SQLite/Turso drivers

## Git workflow

- Branch: `codex/007-postgres-tls`
- Suggested commits: `refactor: structure postgres cli connections`,
  `fix: honor postgres tls settings`, and `test: cover postgres tls`
- Do not push or open a PR unless asked.

## Steps

### Step 1: Preserve SSL mode in the credential model

Introduce an internal, strongly typed `PostgresSslMode` covering `Disable`,
`Allow`, `Prefer`, `Require`, `VerifyCa`, and `VerifyFull`. Map config bools as
`false -> Disable`, `true -> Require`. Parse strings case-insensitively and
reject unknown values during config/override resolution instead of treating
every unknown string as enabled. Keep accepted spellings already documented by
the CLI.

Replace `Host.ssl: bool` with the enum. For URL credentials, parse SSL mode from
the URL through `tokio_postgres::Config`; do not write a second ad-hoc query
parser. If the library cannot preserve `verify-ca`/`verify-full` distinctions,
store the requested policy beside the parsed config.

**Verify**: config and override unit tests cover every mode, bool mapping, and
unknown input; both commands pass.

### Step 2: Replace URL construction with `tokio_postgres::Config`

Replace `connection_url()` with a fallible `connection_config()` (or equivalent)
returning structured config plus certificate policy. Parse the `Url` variant
using `FromStr`. Build the `Host` variant via `Config::host`, `port`, `user`,
`password`, and `dbname` setters so reserved characters are data, not URL
syntax. Preserve Unix-socket URL behavior if currently accepted.

Never include a full URL/password in `Debug`, `Display`, or errors. Add unit
tests using usernames/passwords/database names containing `@`, `:`, `/`, `%`,
and spaces, asserting the structured getters rather than reconnecting through a
rendered URL.

**Verify**: `rg -n "connection_url\(|format!\(\"postgres" cli/src` returns no
matches; config tests pass.

### Step 3: Centralize sync and async connector selection

Add a private `postgres_connection` module that accepts the structured config
and policy. Use the rust-postgres project's connector crate rather than a
home-grown TLS stream. Select:

- `Disable`: `NoTls` and config `SslMode::Disable`;
- `Allow`/`Prefer`: the library's optional/preferred TLS negotiation with
  certificate verification when TLS is selected;
- `Require`: TLS required, using system trust roots and hostname verification
  (a secure strengthening over libpq's historical no-verification meaning);
- `VerifyCa`: certificate chain verification with hostname relaxation only;
- `VerifyFull`: certificate chain and hostname verification.

For native TLS, `danger_accept_invalid_hostnames(true)` is permissible only in
the isolated `VerifyCa` branch; never enable `danger_accept_invalid_certs`.
Use a vendored/native-tls configuration that still builds the repository's
Linux musl release target, or STOP if that cannot be demonstrated. A rustls
connector is acceptable instead if it supports all policies without a custom
unsafe verifier.

Expose small sync/async helpers so call sites cannot choose `NoTls` directly.
For async connections, keep spawning/awaiting the connection task and preserve
its error reporting behavior.

**Verify**: new helper unit tests assert connector/config selection for each
mode; MSRV and release checks pass.

### Step 4: Migrate every PostgreSQL CLI operation

Replace every direct sync/async connection in migrations, push, introspection,
seed, and status/check flows with the shared helpers. Remove duplicated URL
creation. Preserve transaction and query behavior after connection creation.

**Verify**:

```text
rg -n "Client::connect|tokio_postgres::connect|postgres::NoTls|tokio_postgres::NoTls|connection_url" cli/src
```

Only the private connection module may contain connector construction; no
operation call site may remain. Existing PostgreSQL integration tests pass.

### Step 5: Add an ephemeral TLS integration recipe

Add `just test::pg-tls` and test-only Docker configuration that generates an
ephemeral CA/server certificate at runtime (do not commit private keys), starts
PostgreSQL with SSL enabled, and waits for readiness. Exercise both sync and
async helper paths. Query
`pg_stat_ssl` for the current backend and assert `ssl = true` for TLS modes.
Also assert:

- `Disable` connects without TLS when the server permits plaintext;
- `VerifyFull` succeeds with the generated CA and matching hostname;
- `VerifyFull` fails for an untrusted CA or mismatched hostname;
- password/database strings with reserved URL characters authenticate exactly.

If existing config lacks a CA-path field, add a narrowly scoped optional
`sslRootCert`/CLI equivalent compatible with drizzle-kit or load the standard
PostgreSQL root-certificate location. Do not silently trust the test cert.

**Verify**: `just test::pg-tls` exits 0 twice and leaves no certificate files in
the working tree (`git status --short`).

### Step 6: Run all gates

Run normal PostgreSQL tests, TLS tests, fmt, MSRV, release check, and clippy.
Inspect errors to ensure credentials are redacted.

**Verify**: every command in the table exits 0.

## Test plan

- Unit parsing for all SSL modes from config and CLI overrides.
- Structured credentials with URL-reserved characters.
- Central connector selection for sync and async paths.
- Actual encrypted connection proven via `pg_stat_ssl`.
- Positive CA/hostname verification and negative untrusted/mismatch cases.
- Plaintext disable mode remains available where explicitly requested.
- Existing migration, push, seed, and introspection PostgreSQL tests pass.

## Done criteria

- [ ] No PostgreSQL CLI operation builds a URL by string concatenation.
- [ ] Every PostgreSQL operation uses the shared structured connection helper.
- [ ] Requested TLS modes are retained and unknown modes fail configuration.
- [ ] Production verification never accepts an invalid certificate.
- [ ] Sync and async TLS connections are proven encrypted by integration tests.
- [ ] Credentials are absent from errors/logs.
- [ ] MSRV, release, PostgreSQL, TLS, fmt, and clippy commands pass.
- [ ] `plans/README.md` marks 007 DONE.

## STOP conditions

- Plan 001 is not DONE.
- The chosen connector does not support both locked postgres client families.
- `VerifyCa` would require disabling certificate-chain verification.
- TLS dependencies cannot build at MSRV or the supported musl release target.
- Supporting a URL form requires logging or re-rendering credentials.
- A real TLS integration test cannot distinguish encrypted from plaintext
  connections via `pg_stat_ssl`.

## Maintenance notes

Reviewers should focus on certificate semantics, redaction, and total call-site
coverage. Future PostgreSQL operations must use the shared helper. Keep TLS test
keys ephemeral and never weaken production verification for fixture convenience.
