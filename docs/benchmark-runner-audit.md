# Benchmark Runner Audit

Last updated: 2026-05-04.

## Execution Surfaces

| Surface | Workflow / command | Runtime location | Publishes dashboard artifacts |
| --- | --- | --- | --- |
| Contract runner: SQLite | `.github/workflows/runners.yml` `sqlite` | GitHub-hosted Ubuntu, Windows, macOS runners | Yes, to Cloudflare R2 on `main` when Cloudflare secrets are present |
| Contract runner: PostgreSQL | `.github/workflows/runners.yml` `postgres` | GitHub-hosted Ubuntu runner plus `postgres:18-alpine` service | Yes, to Cloudflare R2 on `main` |
| Contract runner: PostgreSQL Rust ORMs | `.github/workflows/runners.yml` `postgres-rust-orms` | GitHub-hosted Ubuntu runner plus `postgres:18-alpine` service | Yes, to Cloudflare R2 on `main` |
| Contract runner: PostgreSQL TS | `.github/workflows/runners.yml` `postgres-ts` | GitHub-hosted Ubuntu runner plus Bun and `postgres:18-alpine` service | Yes, to Cloudflare R2 on `main` |
| Contract runner: SpacetimeDB | `.github/workflows/runners.yml` `spacetimedb` | GitHub-hosted Ubuntu runner plus local SpacetimeDB service | Yes, to Cloudflare R2 on `main` |
| Contract runner: Turso | `.github/workflows/runners.yml` `turso` | GitHub-hosted Ubuntu runner | Yes, to Cloudflare R2 on `main` |
| Criterion microbench | `.github/workflows/criterion.yml` | GitHub-hosted runners | No, uploads GitHub Actions artifacts only |
| Dashboard | `bench/dashboard` Cloudflare Worker | Cloudflare Workers, reading R2 | Reads and renders published artifacts; does not run benchmarks |

## Coverage

| Target family | Contract target file | Targets |
| --- | --- | --- |
| SQLite/rusqlite | `bench/spec/targets.sqlite.v1.json` | `drizzle-rs-sqlite`, `rusqlite-sqlite-prepared`, `rusqlite-sqlite-unprepared` |
| Drizzle-RS/Turso SQLite | `bench/spec/targets.turso.v1.json` | `drizzle-rs-turso`, `turso-sqlite-prepared`, `turso-sqlite-unprepared` |
| PostgreSQL driver baselines | `bench/spec/targets.postgres.v1.json` | `tokio-postgres-prepared`, `tokio-postgres-unprepared` |
| Rust PostgreSQL ORMs | `bench/spec/targets.postgres-rust-orms.v1.json` | `sqlx-pg`, `diesel-pg`, `seaorm-pg` |
| TS PostgreSQL comparators | `bench/spec/targets.postgres-ts.v1.json` | `bun-sql-pg`, `drizzle-ts-pg`, `prisma-pg` |
| SpacetimeDB | `bench/spec/targets.spacetimedb.v1.json` | `spacetime-pgwire-rs` |

## Data Contract

PostgreSQL targets use the runner-owned Northwind micro schema and deterministic `drizzle_seed::SeedConfig::postgres` seed path. External PostgreSQL targets seed by invoking `bench-runner seed-postgres` before printing `LISTENING`, so setup stays outside measured load and parity/load exercise the same table layout and rows. The shared seed path now binds PostgreSQL date/time values as typed parameters instead of text, which keeps the schema identical across Drizzle-RS, SQLx, Diesel, SeaORM, Bun SQL, Drizzle TS, and Prisma.

PostgreSQL setup caches generated seed data per seed/version in a private `bench_seed_*` schema. The first setup for a seed builds that cache with the normal constrained seeder; later target resets replay from the cache into `public`, reset serial sequences, and recreate the same indexes. External targets receive `BENCH_RUNNER_BIN` from the parent runner and call that binary directly for seeding, avoiding a nested `cargo run` per target.

PostgreSQL concurrency is explicit in the target specs. tokio-postgres, SQLx, Diesel, SeaORM, Bun SQL, Drizzle TS, and Prisma all advertise and use pool size `8`. Diesel uses a round-robin pool of synchronous libpq connections and runs blocking query work on Tokio's blocking pool instead of serializing all requests behind one connection. The Diesel target bundles libpq 18.3 through `pq-sys`/`pq-src` so CI and local Windows runs do not depend on a system `libpq` import library.

The Drizzle TS comparator is pinned to `drizzle-orm@1.0.0-rc.1`, matching the requested v1 RC feature surface Drizzle-RS is benchmarking against.

The throughput workload mirrors the upstream drizzle-benchmarks ramp shape: 200 to 3000 VUs in alternating 5s ramp / 15s hold stages, then 55s at 3000 VUs. The async in-process load generator uses the same per-iteration pacing as upstream k6 (`sleep(0.1 * (iteration % 6))`) and excludes `/search*` requests for throughput runs, while parity still checks search routes.

Benchmark runs must invoke the runner in release mode. Several built-in targets are launched through `$BENCH_RUNNER_BIN serve`; if the parent command is `cargo run -p bench-runner -- run`, those target servers are debug binaries and throughput numbers are not comparable. CI uses `cargo run --release -p bench-runner -- run` for every measured job.

SQLite targets use the same in-memory SQLite connection model and report pool size `1` in fairness metadata.

SpacetimeDB currently runs through the PGWire target against the same Northwind contract as the other database targets. The older native Rust/TypeScript Spacetime wrappers targeted a previous `bench_users`/`bench_posts` module shape and are not part of the active runner spec until they are rebuilt against the Northwind module.

## Hosting Notes

Benchmarks currently run in GitHub Actions, not Cloudflare Workers. Cloudflare is used for R2 artifact storage and dashboard/API hosting. There is no AWS benchmark runner workflow today; AWS appears only as optional library support in the main crate feature set.

If dedicated hardware is needed for publish-grade numbers, add a self-hosted GitHub runner label or a separate dispatch workflow. Keep artifact output identical and continue publishing immutable run directories plus `index.json` to R2.
