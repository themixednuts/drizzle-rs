# Benchmark Design and Port Plan

## 1. Scope

This document defines the benchmark system design for `drizzle-rs`, based on the public Drizzle benchmark approach, with support for cross-language comparisons (TS/Go/Rust) under a single workload contract.

The priority is specification quality and reproducibility.

## 2. Core Principles

1. One workload contract, many implementations.
2. Benchmark execution is done on dedicated compute (CI runners or self-hosted machines), not edge/serverless request handlers.
3. Dashboard rendering can be static; query/filter APIs can be dynamic.
4. Every published number must be traceable to immutable run artifacts.
5. `criterion` is for local Rust microbench only; it is not part of the cross-runtime benchmark clone.

## 3. Execution Model

### 3.1 Throughput Suite (HTTP)

- Generate scenario request stream.
- Run target server(s) implementing shared endpoint contract.
- Execute load with `k6` using explicit profile types:
  - open-loop (`constant-arrival-rate`) for primary throughput comparability
  - optional closed-loop VU ramps for supplemental analysis
  - single-endpoint profiles for regression isolation
  - mixed-traffic profiles for production-like behavior
- Sample CPU usage periodically from `/stats`.
- Merge load + CPU telemetry into normalized time-series output.

Current Rust adapter note:

1. the in-repo Rust implementation uses an `axum` HTTP adapter for the benchmark route contract.
2. the adapter is spawned by the target `load.cmd` on an ephemeral local port for each trial.
3. this keeps the publish path on the HTTP contract while still allowing later extraction into standalone target services.
4. the current runner uses an internal HTTP loop for Rust targets; external `k6` remains the target design for the broader cross-runtime system.

### 3.2 MVCC/Contention Suite (separate)

- Concurrency-focused read/write contention tests.
- Different metrics and pass/fail rules from throughput suite.
- Results are stored separately to avoid metric confusion.

### 3.3 MVCC Profile Contract

Each MVCC profile must declare:

1. `read_ratio` and `write_ratio` (sum must be `1.0`).
2. transaction size (`ops_per_tx`).
3. isolation mode (`read_committed`, `repeatable_read`, `serializable`).
4. retry policy (`max_retry`, backoff mode, backoff bounds).
5. conflict policy (`count`, `retry`, `abort` accounting).
6. lock timeout/deadline policy.

## 4. System Components

1. `scenario-generator`: creates deterministic request datasets.
2. `targets`: adapter services for each ORM/driver/language.
3. `http-adapter`: thin per-target HTTP surface for the shared route contract.
4. `runner`: orchestrates warmup, load, and telemetry collection.
5. `aggregator`: computes merged metrics and summaries.
6. `publisher`: uploads artifacts and updates benchmark index/manifest.
7. `dashboard`: visualizes summary + time-series.

## 5. Shared Workload Spec

Recommended file: `perf/spec/workload.v1.json`

```json
{
  "version": "v1",
  "suite": "throughput-http",
  "load": {
    "kind": "open",
    "executor": "constant-arrival-rate",
    "unit": "1s"
  },
  "data": {
    "name": "ecommerce-micro",
    "seed": 424242,
    "schema": "2026-03-05"
  },
  "shape": {
    "mode": "mixed",
    "endpoint": null
  },
  "stages": [
    { "sec": 5, "vus": 200 },
    { "sec": 15, "vus": 200 },
    { "sec": 5, "vus": 400 }
  ],
  "requests": {
    "source": "generated",
    "file": "requests.json",
    "skip": ["/search"]
  },
  "sampling": {
    "cpu_ms": 200,
    "bucket_s": 1
  },
  "limits": {
    "err": 0.01
  }
}
```

### 5.1 Throughput Workload Freeze

Publish-grade throughput runs must freeze:

1. endpoint set (exact route list).
2. endpoint weights (must sum to `100`).
3. payload shape and size bounds per endpoint.
4. query parameter domains per endpoint.
5. dataset seed and row counts per table.
6. request ordering mode (`fixed` or `seeded-random`).

## 6. Target Adapter Contract

All throughput targets must expose equivalent endpoints and semantics:

1. `GET /stats` -> per-core CPU percentages array.
2. Query endpoints (`/customers`, `/customer-by-id`, `/orders-with-details`, etc.) with identical query-parameter behavior.
3. Consistent response shape rules for nullable fields, numeric precision, and ordering.

Rust implementation choice:

1. use `axum` for the HTTP adapter.
2. pin to the version already present in the repo dependency graph unless there is a deliberate upgrade plan.
3. keep the adapter thin: request decode, target call, response encode, stats route.
4. keep HTTP/1.1 as the default benchmark transport.
5. if HTTP/2 is tested, treat it as a separate profile and label, not a transparent default upgrade.

Each target must declare:

- language/runtime/toolchain versions
- ORM/driver versions
- worker/process model
- pool/connection settings
- serialization settings
- fairness declaration
  - workers
  - pool
  - DB settings profile
  - schema/index profile hash
  - response contract version

Correctness gate before any perf run:

1. Execute parity test corpus against all enabled targets.
2. Validate response equivalence rules (ordering, nullability, numeric tolerance, shape).
3. Block benchmark publication if correctness parity fails.

## 7. Normalization Rules

1. Fixed data seed for published runs.
2. Fixed hardware profile (for publish-grade runs).
3. Fixed process count policy across targets unless explicitly labeled.
4. Fixed DB settings per suite (autovacuum, WAL/pragma, pool size).
5. Fixed timing windows:
   - warmup: `60s` (excluded)
   - sample: `300s` (included)
   - cooldown: `30s` (excluded)
6. Load-generator headroom must be verified and recorded (CPU/network not saturated on driver host).
7. Publish run only when fairness declaration is complete for all compared targets.

## 8. Artifact Spec

Immutable artifact layout:

```text
runs/<run_id>/
  manifest.json
  env.json
  targets/<target_id>/raw/k6.csv
  targets/<target_id>/raw/cpu.csv
  targets/<target_id>/raw/k6.parquet
  targets/<target_id>/timeseries.json
  targets/<target_id>/summary.json
  reports/compare.md
```

`run_id` format:

- `YYYYMMDDTHHMMSSZ_<git>_<suite>`

`manifest.json` minimum fields:

```json
{
  "run_id": "20260305T180000Z_abc1234_throughput-http",
  "suite": "throughput-http",
  "git": "abc1234...",
  "workload": "sha256:...",
  "targets": ["drizzle-ts-node24", "prisma-ts-node24", "drizzle-rs-tokio"],
  "start": "2026-03-05T18:00:00Z",
  "end": "2026-03-05T18:23:00Z",
  "status": "success"
}
```

## 9. Summary Metric Spec

Per target summary:

- `rps.avg`
- `rps.peak`
- `latency.avg`
- `latency.p90`
- `latency.p95`
- `latency.p99`
- `latency.p999`
- `cpu.avg`
- `cpu.peak`
- `err`
- `saturation.knee_rps`
- `saturation.knee_p95`

Metric definitions and units:

1. `rps.*`: requests per second, computed from `bucket_s` windows, excluding warmup/cooldown.
2. `latency.*`: milliseconds.
3. `cpu.*`: percent of one host (0-100), summarized from per-core samples.
4. `err`: failed requests / total requests in sample window (range `0..1`).
5. `saturation.knee_*`: first sustained point where latency slope breaches threshold.

Trial policy:

1. Run `N` repeated trials (default `N=5`) per target/profile.
2. Publish median as primary value.
3. Publish variability (`min`, `max`, and confidence interval when available).

Time-series record:

```json
{
  "time": "2026-03-05T18:00:11Z",
  "rps": 5120,
  "err": 0,
  "latency": { "avg": 78.1, "p95": 114.2, "p99": 142.4 },
  "cpu": [74.1, 69.8, 72.5, 70.4]
}
```

## 10. API Spec for Dashboard

Read-only API contract (dynamic querying of static artifacts):

1. `GET /api/runs/latest?suite=throughput-http`
2. `GET /api/runs/:run_id/manifest`
3. `GET /api/runs/:run_id/summary`
4. `GET /api/runs/:run_id/timeseries?targets=a,b`
5. `GET /api/compare?base=<run_id>&head=<run_id>&metric=latency.p95`

The API layer should not execute benchmarks; it only reads artifacts/manifests.

Contract surfaces:

1. Internal UI contract:
  - SvelteKit remote functions (typed at compile time).
2. External/public contract:
  - versioned HTTP API described with OpenAPI.
  - intended for third-party consumers, external tooling, and integrations.

Schema-first policy:

1. JSON Schema is source of truth for persisted artifacts (`workload`, `target`, `manifest`, `summary`, `timeseries`).
2. OpenAPI references those artifact shapes (directly or mirrored via generated types).
3. Contract changes require:
  - version bump (`v1` -> `v2`) for breaking changes
  - migration note and compatibility window
  - changelog entry in benchmark docs

Consumer compatibility policy:

1. additive fields are allowed in `v1` and must be optional.
2. field rename/removal/type change requires `v2`.
3. metric semantic change (same key, different meaning) requires `v2`.
4. API endpoints must advertise supported contract versions.

Initial contract files:

1. `docs/benchmark-spec/jsonschema/workload.v1.schema.json`
2. `docs/benchmark-spec/jsonschema/target.v1.schema.json`
3. `docs/benchmark-spec/jsonschema/run-manifest.v1.schema.json`
4. `docs/benchmark-spec/jsonschema/summary.v1.schema.json`
5. `docs/benchmark-spec/jsonschema/timeseries.v1.schema.json`
6. `docs/benchmark-spec/openapi/bench-api.v1.yaml`

Two serving modes are supported:

1. Client-side filtering mode:
  - UI downloads precomputed summary/time-series bundles and filters in SPA.
  - Best for smaller run history.
2. Worker-filtered mode:
  - UI requests filtered subsets from API.
  - Best for large historical datasets and lower browser memory use.

## 11. CI and Run Policy

### 11.1 Scheduled/Manual Benchmark (publish-grade)

- Full workload profile.
- Dedicated runner class.
- Publish artifacts + summary index.
- Optional gating for severe regressions.
- Multi-trial required (`N>=3`, default `N=5`) for publish-grade reports.

### 11.2 Baseline Policy

- Maintain per-suite rolling baseline on `main`.
- Compare each new run against latest compatible baseline.
- Compatibility key: suite + workload hash + runner class + target id.
- Baseline selection picks newest successful run with exact compatibility key.
- If no exact baseline exists, mark compare status as `no_baseline` and skip regression gate.

### 11.3 Run Classes

1. `small`:
   - fast confidence checks
   - `N=1`
   - non-blocking for publication
2. `publish`:
   - full profile
   - `N>=3` (default `5`)
   - required for dashboard publication
   - headroom must pass

### 11.4 Trial and Regression Rules

1. primary value is median across trials.
2. variability reports min/max and optional `ci95`.
3. regression decision requires:
   - absolute delta threshold and
   - relative delta threshold and
   - consistency check (`>= 2/3` of trials in same direction).

### 11.5 Publication Gates

A publish run is only accepted when all are true:

1. parity gate passed for compared targets.
2. fairness lock present for each target.
3. headroom gate passed:
   - load host CPU peak `< 85`
   - load host net peak `< 80`
4. trial count satisfies run class policy.
5. artifact schema validation passed.

## 12. Reference Deployment Profile

Assumption for initial implementation: Cloudflare + SvelteKit.

This is a deployment profile, not a hard protocol dependency.

1. UI: SvelteKit static output.
2. Artifact store: object storage (`R2` in reference profile).
3. API (optional): edge/service worker that reads manifests and artifacts.

Clarification:

- Dynamic testing is not done in Workers.
- Benchmark execution remains on dedicated compute.
- Workers/API layer is for dynamic result retrieval, filtering, and compare views.

## 13. Hosting Notes

1. Pages is valid for static benchmark UI hosting.
2. Workers static assets is also valid if we want a single Worker-centric deployment model.
3. Keep dashboard and API contracts deployment-agnostic so switching between Pages and Workers static assets is low risk.
4. If all data is precomputed and bounded in size, Pages + static SvelteKit can serve everything without Worker APIs.
5. If run history grows large, prefer Worker-filtered APIs to avoid sending full historical payloads to every client.

## 14. Security and Integrity

1. CI publish credentials scoped to artifact upload path.
2. Artifact checksum recorded in manifest.
3. Signed manifest optional for public verification.
4. Run metadata includes tool versions and host fingerprint for auditability.

## 15. Implementation Plan

### Phase 0: Contracts

1. Create JSON Schemas for core artifacts.
2. Create OpenAPI `v1` for read-only public API.
3. Add schema validation in CI for published artifacts.
4. Add compatibility/versioning policy document.

### Phase 1: Spec + Harness

1. Create workload spec files and schema validator.
2. Implement runner + aggregator with deterministic output.
3. Port shared endpoint contract tests.
4. Add open-loop and mixed/single profile definitions.
5. keep `criterion` separate from the publish harness.

### Phase 2: Targets

1. Add TS/Go reference adapters.
2. Add Rust adapters:
   - `drizzle-rs` `postgres-sync`
   - `drizzle-rs` `tokio-postgres`
   - `drizzle-rs` `rusqlite`
   - `drizzle-rs` `turso`
3. implement each Rust target behind the shared `axum` adapter surface.
4. Add fairness declaration per target.
5. Add correctness parity gate before performance runs.

### Phase 3: CI + Publish

1. Add scheduled/manual publish-grade job.
2. Add baseline compare step against latest compatible run.
3. Run multi-trial execution and aggregate median + variability.
4. Publish artifacts + manifest index.
5. Add load-generator bottleneck checks to run metadata.

### Phase 4: Dashboard + API

1. Implement SvelteKit dashboard against API contract.
2. Implement client-side filtering mode first.
3. Implement read-only Worker API for run discovery/compare as scale path.
4. Add regression, saturation, and trend visualizations.

### Phase 5: MVCC Suite

1. Add contention workload spec.
2. Add suite-specific metrics + summary rules.
3. Integrate into publish flow as separate track.

## 16. Success Criteria

1. Same workload spec drives all targets.
2. Published numbers are reproducible and traceable.
3. Dashboard can compare TS/Go/Rust implementations in one view.
4. Throughput and MVCC suites are clearly separated and interpretable.

## 17. v1 Readiness Checklist

Before implementation starts, these must be complete:

1. metric dictionary finalized (units + formulas + windows).
2. throughput profile frozen (routes, weights, payloads, params, seed).
3. MVCC profile frozen (ratios, tx shape, isolation, retry/conflict policy).
4. run classes finalized (`small`, `publish`) with hard thresholds.
5. regression rule finalized (absolute + relative + consistency).
6. baseline algorithm finalized and documented.
7. fairness lock format finalized and required in publish runs.
8. publication gates wired into CI policy.
9. schema compatibility/version policy finalized for external consumers.
10. acceptance tests defined for parity, headroom, schema validation, and compare outputs.

## 18. Runner Contract

The runner contract is defined in:

1. `docs/benchmark-spec/runner.v1.md`

Rules:

1. `runner.v1` is source of truth for CLI input, file input, output layout, events, and exit codes.
2. Any breaking change to runner CLI/file contract requires `runner.v2`.
