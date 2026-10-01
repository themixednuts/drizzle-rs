# Benchmark Runner Audit

Last updated: 2026-10-01.

## Execution Surfaces

| Surface | Workflow / command | Runtime location | Publishes dashboard artifacts |
| --- | --- | --- | --- |
| Run plan | `.github/workflows/runners.yml` `plan` | GitHub-hosted Ubuntu runner | No, resolves class/workload/topology for every other job |
| Contract runner: SQLite | `.github/workflows/runners.yml` `sqlite` | GitHub-hosted Ubuntu, Windows, macOS runners | Yes, for releases and opted-in manual runs on main or a version tag |
| Contract runner: PostgreSQL | `.github/workflows/runners.yml` `postgres` | GitHub-hosted Ubuntu runner plus `postgres:18-alpine` service | Yes, for releases and opted-in manual runs on main or a version tag |
| Contract runner: PostgreSQL Rust ORMs | `.github/workflows/runners.yml` `postgres-rust-orms` | GitHub-hosted Ubuntu runner plus `postgres:18-alpine` service | Yes, for releases and opted-in manual runs on main or a version tag |
| Contract runner: PostgreSQL TS | `.github/workflows/runners.yml` `postgres-ts` | GitHub-hosted Ubuntu runner plus Bun and `postgres:18-alpine` service | Yes, for releases and opted-in manual runs on main or a version tag |
| Cross-family ranking: linux | `.github/workflows/runners.yml` `linux-all` | One GitHub-hosted Ubuntu runner plus Bun, a cpuset-pinned `postgres:18-alpine` service and a local SpacetimeDB, running all eight families sequentially | Yes, for releases and opted-in manual runs on main or a version tag |
| Cross-family ranking: macOS / Windows | `.github/workflows/runners.yml` `desktop-all` | One GitHub-hosted macOS runner and one Windows runner plus Bun, running the three in-process families sequentially | Yes, for releases and opted-in manual runs on main or a version tag |
| Contract runner: SpacetimeDB | `.github/workflows/runners.yml` `spacetimedb` | GitHub-hosted Ubuntu runner plus local SpacetimeDB service | Yes, for releases and opted-in manual runs on main or a version tag |
| Contract runner: Turso | `.github/workflows/runners.yml` `turso` | GitHub-hosted Ubuntu runner | Yes, for releases and opted-in manual runs on main or a version tag |
| Criterion microbench | `.github/workflows/criterion.yml` | GitHub-hosted runners | No, uploads GitHub Actions artifacts only |
| Dashboard | `bench/dashboard` Cloudflare Worker | Cloudflare Workers, reading R2 | Reads and renders published artifacts; does not run benchmarks |

Every family job is a thin caller of the composite action
`.github/actions/bench-runner-run`, which owns baseline restore, the release
prebuild, the run, artifact validation, upload, and baseline save. The two other
composite actions are `.github/actions/resolve-postgres-url` (service container
address, including the act fallback) and `.github/actions/install-ts-targets`
(Bun installs plus the load-bearing `prisma generate`).

## Coverage

| Target family | Contract target file | Targets |
| --- | --- | --- |
| SQLite/rusqlite | `bench/spec/targets.sqlite.v1.json` | `drizzle-rs-sqlite`, `drizzle-rs-sqlite-query`, `rusqlite-sqlite-prepared`, `rusqlite-sqlite-unprepared` |
| TS SQLite comparators | `bench/spec/targets.sqlite-ts.v1.json` | `bun-sqlite`, `drizzle-orm-sqlite` |
| Drizzle-RS/Turso SQLite | `bench/spec/targets.turso.v1.json` | `drizzle-rs-turso`, `turso-sqlite-prepared`, `turso-sqlite-unprepared`, `toasty-turso` |
| Drizzle-RS/libSQL SQLite | `bench/spec/targets.libsql.v1.json` | `drizzle-rs-libsql`, `libsql-sqlite-prepared`, `libsql-sqlite-unprepared` |
| Drizzle-RS PostgreSQL + driver baselines | `bench/spec/targets.postgres.v1.json` | `drizzle-rs-pg`, `drizzle-rs-pg-query`, `drizzle-rs-pg-sync`, `tokio-postgres-prepared`, `tokio-postgres-unprepared` |
| Rust PostgreSQL ORMs | `bench/spec/targets.postgres-rust-orms.v1.json` | `sqlx-pg`, `diesel-pg`, `seaorm-pg`, `toasty-pg` |
| TS PostgreSQL comparators | `bench/spec/targets.postgres-ts.v1.json` | `bun-sql-pg`, `drizzle-ts-pg`, `prisma-pg` |
| SpacetimeDB | `bench/spec/targets.spacetimedb.v1.json` | `spacetime-pgwire-rs`, `spacetime-sdk-rs`, `spacetime-module-rs` |

The libSQL family is the one exception to "every family runs on every platform
its driver supports": it is behind the `libsql` cargo feature, off by default,
and its CI job is Linux-only, because libsql has a history of crashing the
benchmark process on Windows and macOS. A default `bench-runner` build does not
link it, so the Windows and macOS SQLite jobs are unaffected.

## Data Contract

PostgreSQL targets use the runner-owned Northwind micro schema and deterministic `drizzle_seed::SeedConfig::postgres` seed path. External PostgreSQL targets seed by invoking `bench-runner seed-postgres` before printing `LISTENING`, so setup stays outside measured load and parity/load exercise the same table layout and rows. The shared seed path now binds PostgreSQL date/time values as typed parameters instead of text, which keeps the schema identical across Drizzle-RS, SQLx, Diesel, SeaORM, Bun SQL, Drizzle TS, and Prisma.

PostgreSQL setup caches generated seed data per seed/version in a private `bench_seed_*` schema. The first setup for a seed builds that cache with the normal constrained seeder — in `public`, because the seeder's INSERTs name `"public".<table>` explicitly and `search_path` cannot redirect them, after which the finished tables are moved into the cache schema in the same transaction; later target resets replay from the cache into `public`, reset serial sequences, and recreate the same indexes. External targets receive `BENCH_RUNNER_BIN` from the parent runner and call that binary directly for seeding, avoiding a nested `cargo run` per target.

PostgreSQL concurrency is explicit in the target specs. tokio-postgres, SQLx, Diesel, SeaORM, Bun SQL, Drizzle TS, and Prisma all advertise and use pool size `8`. Diesel uses a round-robin pool of synchronous libpq connections and runs blocking query work on Tokio's blocking pool instead of serializing all requests behind one connection. The Diesel target bundles libpq 18.3 through `pq-sys`/`pq-src` so CI and local Windows runs do not depend on a system `libpq` import library.

The Drizzle TS comparator is pinned to `drizzle-orm@1.0.0-rc.1`, matching the requested v1 RC feature surface Drizzle-RS is benchmarking against.

The throughput workload mirrors the upstream drizzle-benchmarks ramp shape: 200 to 3000 VUs in alternating 5s ramp / 15s hold stages, then 55s at 3000 VUs, preceded by probe rungs at 25, 50 and 100 VUs that give the sustained-latency curve a floor below every target's knee and are excluded from `primary.*`. The async in-process load generator uses the same per-iteration pacing as upstream k6 (`sleep(0.075 * (iteration % 6))`, a mean 187.5 ms) and excludes `/search*` requests for throughput runs, while parity still checks search routes.

Benchmark runs must invoke the runner in release mode. Several built-in targets are launched through `$BENCH_RUNNER_BIN serve`; if the parent command is `cargo run -p bench-runner -- run`, those target servers are debug binaries and throughput numbers are not comparable. CI builds `bench-runner` once with `cargo build --release -p bench-runner`, exports the binary path as `BENCH_RUNNER_BIN`, and invokes that binary directly for both `run` and `validate`. Families with external Rust targets (`rust-pg-orms`, `toasty`, `spacetime-native-rs`) build those manifests with `--release` in the same step, so the `cargo run` in their `server.cmd` is a freshness check rather than a compile inside the measurement window.

## CI Topology

Linux jobs cut the machine in half: `BENCH_CPUSET_LOAD` gets the lower half, and the upper half is the system under test. How the SUT half is subdivided follows the architecture being measured, not the family. For an in-process engine the whole half is the server, because the process that serves HTTP is the process that executes the query. For an out-of-process engine the caller pins the database to a top slice of that half — `--cpuset-cpus` on the PostgreSQL service container, `taskset` on the SpacetimeDB daemon — and passes the same literal as the composite action's `db-cpuset`, which carves it out of `BENCH_CPUSET_SERVER` and hard-fails if the literal disagrees with `nproc`. Either way the SUT owns the same cores (`0-1` load, `2-3` SUT on the 4 vCPU hosted runner), which is what makes a cross-family ranking on one box mean anything.

It remains best-effort separation on a shared VM — caches, memory bandwidth and the network stack stay shared. It is a no-op on macOS and Windows: the runner's affinity call is Linux-only, and Darwin exposes no usable CPU-affinity API, so `topology.cpu_pinning` is null there and the dashboard reports the absence rather than implying isolation. Numbers recorded before pinning are not comparable to pinned numbers.

## Per-OS cross-family ranking

The leaderboard puts every family in one table, so a rank only means something if the rows came off one machine. Release and manual-dispatch runs therefore add `linux-all` and `desktop-all`, which run every family their OS can host back to back on one VM under one cohort id (`<cohort>-cross-<os>`).

Which families an OS can host is a platform fact. GitHub runs service containers on Linux only, so PostgreSQL and SpacetimeDB exist on linux alone, and libsql has a history of segfaulting the benchmark process on macOS and Windows. That is why the ranking is scoped per OS on the dashboard rather than merged into one table.

These jobs run the saturation ramp regardless of the class's resolved workload, because a cross-family ranking needs a number that describes the target and the paced suites cap every target at `VUs / think_time`. They run *alongside* the per-family jobs rather than replacing them: the paced latency reading still comes from those, in parallel, on every event. `slug-suffix: cross` keeps the two sets of artifacts, output directories and baseline cache keys apart.

`linux-all` is a serial sequence and is the one job here that can walk into GitHub's hard 360-minute cancellation, so `plan` estimates it from the specs themselves — target count × trials × ramp length, plus a build allowance — and fails the run in five minutes if the estimate exceeds the job timeout. Adding a target to any family spec lengthens that sequence, and this is what makes it say so.

The dashboard preview job assembles whatever families succeeded and fails only when no run at all could be assembled.

SQLite targets serve a seeded, file-backed SQLite database in a per-run temp directory (WAL journal). The pool size is per family and declared in `fair.pool`: `8` for the rusqlite family and libSQL, `4` for Turso, and `1` for the Bun-based `sqlite-ts` family, whose synchronous single-threaded runtime cannot use more.

SpacetimeDB runs three targets against the same Northwind contract and module (`bench/targets/spacetime-module`):

- `spacetime-pgwire-rs` sends SQL over PGWire from a `bench-runner serve` client pool (pool 4); shapes PGWire cannot express are finished in the client process.
- `spacetime-sdk-rs` subscribes to every table and answers from the SDK's client-side cache (`data_access: "in-process-cache"`, exempt from the pool/worker equality check because it does no per-request database work).
- `spacetime-module-rs` is SpacetimeDB's own design point: every route's lookups, joins, aggregates and JSON serialization run inside the database as one `route_*` procedure (`bench/targets/spacetime-module/src/routes.rs`). The HTTP process (`spacetime-native-rs --module-procedures`) only forwards each request as one `POST /v1/database/<db>/call/<procedure>` and copies the returned body. It declares `data_access: "in-database"`, which is *not* exempt: it does real database work per request, so it must match the family's `fair.workers`/`fair.pool` (it caps in-flight calls at `pool.max` = 4, like the PGWire pool). Its per-request cost includes an HTTP/1.1 call with bearer-token auth to the database and a procedure-scoped transaction (2.8 procedures only expose `with_tx`, a mutable transaction).

## Hosting Notes

Benchmarks currently run in GitHub Actions, not Cloudflare Workers. Cloudflare is used for R2 artifact storage and dashboard/API hosting. There is no AWS benchmark runner workflow today; AWS appears only as optional library support in the main crate feature set.

If dedicated hardware is needed for publish-grade numbers, add a self-hosted GitHub runner label or a separate dispatch workflow. Keep artifact output identical and continue publishing immutable run directories plus `index.json` to R2.
