# Runner Contract v1

## 1. Scope

This contract defines runner input, output, events, and exit codes for benchmark execution.

It is language-agnostic and is designed so Rust execution and TS/SvelteKit presentation share one artifact contract.

Non-goal:

1. `criterion` is not part of the cross-runtime publish harness.
2. Rust microbench can continue to use `criterion` locally, but publish-grade benchmark runs must go through the runner target contract.

## 2. Command

Required command:

```text
bench-runner run \
  --suite <suite> \
  --workload <path> \
  --targets <path> \
  --requests <path> \
  --out <dir>
```

External PostgreSQL target setup command:

```text
bench-runner seed-postgres --seed <u64>
```

Optional flags:

```text
--class <small|full|publish>
--trials <n>
--baseline <run_id|auto>
--cohort-id <id>
--publish
--seed <u64>
--timeout-s <n>
--json
```

Rules:

1. `--suite` must match `workload.suite`. `throughput-http` is the only suite
   the runner implements.
2. `--trials` defaults by class:
   - `small`: `3` — local and PR smoke runs. Accepts `--seed`.
   - `full`: `5` — full matrix, not published. Ignores `--seed`.
   - `publish`: `5` — publish-grade. Ignores `--seed`, and gate failures are
     fatal (exit `9`).
3. `--seed` overrides the workload seed for `small` runs only. `full` and
   `publish` always use `workload.data.seed`, and the runner emits an event
   noting the override was ignored.
4. `--json` enables JSONL events on stdout.
5. `--baseline` enables regression comparison against `out/runs/<run_id>`.
   `auto` selects the newest successful run with the **same suite, the same
   workload sha256, and the same class**. A baseline that differs on any of
   those is refused with exit `10`: comparing a 3000-VU publish run against a
   128-VU preview run and calling the difference a regression is worse than
   having no baseline.
6. `--publish` appends the completed run to `<out>/index.json`, the same index
   the standalone `publish` subcommand maintains.
7. `--timeout-s` is the default wall-clock budget for each target
   `parity`/`warmup`/`load` invocation (default `1800`). A per-target
   `timeout_s` in the spec overrides it. Children run in their own process
   group and are killed as a group on timeout, so wrapper commands cannot leave
   an orphaned server holding the port.

## 3. Input Files

Required input:

1. `workload` file:
   - schema: `docs/benchmark-spec/jsonschema/workload.v1.schema.json`
   - `pacing.mode=drizzle-benchmark` applies the upstream k6 sleep cadence
     (`(iteration % 6) * 75ms`, per VU). This caps offered load at roughly
     `VUs / (mean think time + mean service time)` — a paced run measures
     latency under a bounded arrival rate, not peak throughput.
   - `pacing.mode=none` is for closed-loop saturation workloads such as single-endpoint throughput.
   - `load.executor` must be `constant-vus` or `ramping-vus`. The arrival-rate
     names are reserved in the schema but rejected by the runner: there is no
     open-loop generator, and accepting them would run a closed-loop VU test
     under an open-model label.
   - `warmup_s` (default `10`) marks the leading seconds of the run as
     `phase=warmup`; those buckets appear in the timeseries but are excluded
     from `summary.primary`.
2. `targets` file:
   - array of target descriptors (each matches `target.v1`).
   - optional execution hooks per target:
     - `parity.cmd` (array of tokens)
     - `warmup.cmd`
     - `load.cmd`
3. `requests` file:
   - deterministic request list generated from seed pipeline.

Optional input:

1. `baseline` file or `run_id` resolver.
2. `env` file for run-local overrides.

## 4. Seed Policy

1. Seed generation should be owned by the Rust seed crate.
2. Runner consumes generated fixtures, it does not reimplement RNG logic in TS.
3. Published runs must use fixture hash + seed recorded in manifest.
4. If seed crate changes fixture semantics, bump fixture schema version.

Implementation note:

1. request fixture generation uses `drizzle-seed` deterministic generators.

## 5. Execution Flow

Runner steps:

1. validate CLI and input schema.
2. resolve targets and health check.
3. run parity gate.
4. execute warmup.
5. execute benchmark trials.
   - for each target `load.cmd`, runner exports:
     - `BENCH_RUN_DIR`
     - `BENCH_SUITE`
     - `BENCH_TARGET_ID`
     - `BENCH_TRIAL` (zero-based; the load command must use it to decorrelate
       the request order, otherwise every trial replays the same sequence)
     - `BENCH_SEED`
     - `BENCH_WORKLOAD_FILE`
     - `BENCH_REQUESTS_FILE`
     - `BENCH_POINT_OUT`
     - `BENCH_TIMESERIES_OUT`
     - `BENCH_POOL_SIZE` (from `target.pool.max`)
     - `BENCH_WORKERS` (from `target.proc.workers`)
6. aggregate results.
7. evaluate gates (headroom/regression/limits).
8. write artifacts.
9. publish (if enabled).

Command token resolution:

1. `argv[0] == "$BENCH_RUNNER_BIN"` in any `parity`/`warmup`/`load`/`server`
   command resolves to the running executable. Specs use this instead of
   `cargo run`, which would otherwise perform freshness checks and take the
   build lock in the middle of a measured trial.
2. Any other `$NAME` argument expands from the environment.

Runtime parity:

1. `BENCH_WORKERS` is the declared worker count, and the `serve` subcommand
   sizes its tokio runtime to exactly that (default `1`). Without it a Rust
   target takes every core while the Bun targets it is ranked against take one,
   and `fair.workers: 1` is false for half the table.
2. Targets ranked in the same run should agree on `fair.pool` and
   `fair.workers`. Disagreements are recorded in `manifest.fairness` and emitted
   as `warn` events rather than failing the run; targets declaring
   `data_access: "in-process-cache"` are exempt, since they have no pool.

Target lifecycle:

1. External targets are spawned in their own process group and torn down as a
   group.
2. Both child pipes are drained for the entire process lifetime. Stopping at the
   `LISTENING` line lets the stdout buffer fill, and the target then dies on
   `EPIPE` at its next log line, mid-benchmark.
3. After `LISTENING`, the runner polls `GET /stats` until it answers `200`
   before measurement begins. `LISTENING` only proves the socket is bound;
   pools may still be filling and JIT tiers may still be cold.

Pre-run fixture stage:

1. runner canonicalizes request entries.
2. if request list is empty, runner generates deterministic requests using seed-driven generators.
3. runner removes requests whose path starts with any `workload.requests.skip` prefix.
4. runner writes `requests.generated.json` under run root and uses it as effective request set.

Load output rules:

1. `load.cmd` may emit a single trial point to `BENCH_POINT_OUT`.
2. `load.cmd` may instead emit a per-trial point series to `BENCH_TIMESERIES_OUT`.
3. each timeseries point may include `queries[]`, a per-route breakdown with `method`, `path`, `rps`, `err`, and response latency percentiles for that bucket.
4. if a route is absent from a bucket that otherwise has query metrics, it had zero measured requests in that bucket.
5. process metrics such as CPU and memory are target-level measurements; they are not attributed to individual query routes unless a future instrumented target contract provides that data.
6. at least one of `BENCH_POINT_OUT` or `BENCH_TIMESERIES_OUT` must be written.
7. **emitting both is preferred.** Exact hold-phase percentiles can only be
   computed where the raw per-request samples live, which is inside the load
   command. When both artifacts are present the runner takes the series for
   charts and the point as the trial aggregate. When only a series is present
   the runner derives an aggregate from it: request-count-weighted `rps`/`err`
   over `phase=hold` buckets, with percentiles approximated as the median of the
   per-bucket percentiles.
8. each emitted point should carry `trial`, `stage`, `phase`, and `requests`.
   Points without a `phase` are treated as steady state for backwards
   compatibility.
9. a series is persisted under `targets/<target_id>/raw/trial/<n>.series.json`,
   and the aggregate point under `targets/<target_id>/raw/trial/<n>.point.json`.

Implementation note:

1. a target `load.cmd` is expected to exercise the real target path, not synthetic fixture generation.
2. the current Rust HTTP adapter uses `axum 0.8` and serves the benchmark route contract on an ephemeral local port during the trial, with `TCP_NODELAY` set on every accepted connection.
3. HTTP/1.1 is the default benchmark transport.
4. HTTP/2, when supported, should be published as a separate labeled profile rather than replacing the default transport silently.
5. external targets may define `server.cmd`; the runner forwards it to both parity and load via `BENCH_SERVER_CMD`, so correctness and measurement exercise the same adapter process.
6. external PostgreSQL targets should call `seed-postgres` before printing `LISTENING`; this keeps schema and deterministic row generation shared with built-in PostgreSQL targets while excluding setup time from measured load.

## 6. Output Layout

Output root:

```text
<out>/runs/<run_id>/
  manifest.json
  env.json
  requests.generated.json
  events.jsonl
  targets/<target_id>/raw/k6.csv
  targets/<target_id>/raw/cpu.csv
  targets/<target_id>/raw/k6.parquet
  targets/<target_id>/raw/trial/<n>.series.json
  targets/<target_id>/raw/trial/<n>.point.json
  targets/<target_id>/timeseries.json
  targets/<target_id>/summary.json
  reports/compare.md
  result.json
```

`result.json` minimum:

```json
{
  "version": "v1",
  "run_id": "20260305T180000Z_abc1234_throughput-http",
  "status": "success",
  "suite": "throughput-http",
  "class": "publish",
  "trials": 5,
  "gates": {
    "parity": "pass",
    "headroom": "skip",
    "regression": "pass",
    "limits": "pass"
  }
}
```

## 6a. Aggregation

1. Only `phase == "hold"` buckets contribute to `summary.primary`. Warmup and
   ramp buckets stay in the timeseries for charts.
2. Per trial: `rps = sum(requests) / sum(wall)`, `err = sum(errors) / sum(requests)`.
   Latency percentiles come from the merged raw hold-phase samples of that
   trial, sorted exactly — not from a median of per-bucket percentiles.
3. Across trials: the **median** of the per-trial aggregates. Peaks are the
   maximum over hold-phase buckets.
4. `latency.p50` and `latency.p90` are measured. The previous release derived
   `p90` as `avg + (p95 - avg) * 0.9/0.95`, which treats the mean as a 0th
   percentile; that number was invented and is gone.
5. `spread.ci95` has been removed. A 512-resample bootstrap over 3-5 trials
   describes the resampling, not the target. Use `spread.rps`, `spread.p95`,
   `spread.variance`, and `spread.boxplot`.
6. `saturation` is computed per trial and medianed. A bucket is degraded once
   its p95 exceeds twice the median p95 of that trial's lowest-VU hold plateau;
   the knee is the third consecutive degraded bucket, or the highest-throughput
   hold bucket if none qualifies. It is never computed across a trial boundary.

## 6b. Host and Topology

`manifest.runner` records what the numbers were measured on:

1. `cpu` is the host CPU brand string, `cores` the logical count, and
   `cores_physical` the physical count when the OS reports it.
2. `topology.loadgen_colocated` is always `true`: the runner spawns every target
   as its own child, so the load generator shares the host with the system under
   test. `topology.db_colocated` is `true` unless `DATABASE_URL` points at a
   non-loopback host.
3. `topology.cpu_pinning` records the applied cpuset. On Linux, setting
   `BENCH_CPUSET_LOAD` and/or `BENCH_CPUSET_SERVER` (for example `0-1` and
   `2-3`) pins the load process and the spawned target processes respectively.
   No-op on other platforms. CI sets both on every Linux job — see §13.
4. `headroom.cpu_peak` is the peak single-core utilisation;
   `headroom.cpu_mean_peak` is the peak of the mean across cores and is what the
   publish gate compares.

## 7. Stdout/Stderr

When `--json` is set, stdout emits JSONL events:

```json
{"time":"2026-03-05T18:00:00Z","level":"info","step":"validate","msg":"start"}
{"time":"2026-03-05T18:00:08Z","level":"info","step":"parity","msg":"pass"}
{"time":"2026-03-05T18:22:59Z","level":"info","step":"aggregate","msg":"done"}
```

Rules:

1. human-readable logs may be emitted without `--json`.
2. stderr is reserved for errors and diagnostics.
3. final line on success should include `run_id`.

## 8. Exit Codes

`0` success

`2` invalid_cli
- missing/invalid arguments.

`3` invalid_input
- schema or file validation failed.

`4` parity_fail
- target correctness parity failed.

`5` target_fail
- target health check or startup failed.

`6` run_fail
- workload execution failed or timed out.

`7` aggregate_fail
- merge/summarize step failed.

`8` publish_fail
- artifact upload/index write failed.

`9` gate_fail
- regression or headroom gate failed.

`10` no_baseline
- regression gate requested but baseline missing.

`11` canceled
- interrupted by user or CI cancel signal.

## 9. TS/SvelteKit Integration

1. Runner writes immutable artifacts only.
2. SvelteKit reads artifacts through static files or API.
3. Worker/API layer may filter and paginate; it must not execute benchmarks.
4. UI and API types should be generated from JSON Schema/OpenAPI contracts.

## 10. Baseline Compatibility

Compare compatibility is `suite + workload + class + target_id`. All four are
already present as top-level manifest fields (`suite`, `workload`,
`runner.class`, `targets[]`), and the resolver reads them directly — both for an
explicit `--baseline <run_id>` and for `--baseline auto`. No separate `compat`
hint object is written.

## 11. Compatibility

Breaking examples requiring `runner.v2`:

1. changing required CLI args.
2. changing artifact required files.
3. changing exit code semantics.
4. changing `result.json` required fields.

## 12. Gates

Headroom gate:

1. informational by default (`skip`): the closed-loop ramp saturates a
   colocated host on purpose (knee detection needs to reach saturation), so a
   saturated mean is a property of the methodology, not a defect. The measured
   `cpu_mean_peak` is still recorded in the manifest and events.
2. enforced only when the workload sets `limits.cpu_mean_peak` (a hard ceiling
   in percent). Set it on topologies where genuine headroom is expected — e.g.
   dedicated load and SUT hosts — and publish-class runs then hard-fail above
   the ceiling.
3. the number deliberately uses the mean across cores. The single-core peak
   hits 100% on any multi-core host the moment one thread is busy.
4. the measurement reads the whole host, not the load generator's cpuset. Under
   CI pinning (§13) a saturated target still shows as roughly half the host —
   the number describes the machine, not the system under test.

Regression gate, when a baseline is provided:

1. compare per target on `rps.avg` and `latency.p95` (both cross-trial medians).
2. fail if either:
   - `rps.avg` drops by more than `50` and more than `10%`.
   - `latency.p95` rises by more than `5` and more than `10%`.
3. baselines that do not match on suite, workload sha256, and class are refused
   with exit `10` rather than compared.

Limits gate:

1. fails when `summary.primary.err` exceeds `workload.limits.err`, or
   `latency.p95` exceeds `workload.limits.p95` when set.
2. `err` here is the weighted hold-phase error rate, so a single failing bucket
   during ramp no longer moves it.

For publish class, any gate failure exits with code `9`.

## 13. CI Execution

`.github/workflows/runners.yml` is the reference execution of this contract.
The measurement-relevant parts of it are contract, not incidental CI plumbing.

### 13.1 One plan, many families

A `plan` job resolves `class` and `workload` once from the event and passes them
to every family job. Class resolution:

| `benchmark_size` | publish-class run | class | workload |
| --- | --- | --- | --- |
| `preview` | no | `small` | `workload.preview.v1.json` |
| `preview` | yes | `publish` | `workload.throughput.v1.json` |
| `full` | no | `full` | `workload.throughput.v1.json` |
| `full` | yes | `publish` | `workload.throughput.v1.json` |
| `single` | no | `full` | `workload.single-throughput.v1.json` |
| `single` | yes | `publish` | `workload.single-throughput.v1.json` |

A run is publish-class on pushes to `main` and tags, on the weekly schedule, and
on a manual dispatch from `main` with `publish_to_r2` set.

### 13.2 Nothing compiles during measurement

Each job builds `bench-runner` with `--release` once, exports the resulting path
as `BENCH_RUNNER_BIN`, and drives the run through that binary rather than
`cargo run`. Families whose specs spawn external Rust targets
(`bench/targets/rust-pg-orms`, `bench/targets/toasty`,
`bench/targets/spacetime-native-rs`) build those manifests with `--release` in
the same step.

This matters because `server.cmd` runs once per target per trial. A `cargo run`
there performs a freshness check and takes the build lock inside the measurement
window, and a debug build of a built-in target would not be comparable to a
release build of its neighbour in the same table.

### 13.3 CPU separation

Linux jobs derive a disjoint pair of cpusets from `nproc` and export them:
the lower half as `BENCH_CPUSET_LOAD`, the upper half as `BENCH_CPUSET_SERVER`.
On the 4 vCPU GitHub-hosted Ubuntu runner that is `0-1` for the load generator
and `2-3` for the target and any database it spawns.

This is best-effort separation on a shared VM, not isolation. Memory bandwidth,
last-level cache, the kernel network stack, and (for the PostgreSQL families)
the service container are all still shared. What it buys is that the load
generator and the system under test stop competing for the same cores, which is
the largest single source of run-to-run noise in a colocated setup. macOS and
Windows jobs run unpinned — the runner ignores both variables there — so
`topology.cpu_pinning` is `null` for those runs and they are not comparable to
Linux runs of the same family.

Pinning changes absolute numbers. A baseline recorded before pinning is not a
valid comparison point for a pinned run even though suite, workload sha and
class all still match.

### 13.4 Publish topology

Rankings are only meaningful within one machine. On publish-class schedule and
manual-dispatch runs the three PostgreSQL families
(`targets.postgres.v1.json`, `targets.postgres-rust-orms.v1.json`,
`targets.postgres-ts.v1.json`) run back to back inside a single job, against a
single PostgreSQL service, under a single cohort id, and the per-family
PostgreSQL jobs are skipped for that run. Every other run keeps the families
parallel on separate VMs, which is faster but only comparable within a family.

Artifact names, output directories and baseline cache keys are per family in
both topologies, so a consumer sees exactly one artifact per family either way
and cannot tell the two apart except through `manifest.runner` host fields.

Runs that share a `cohort_id` are the same logical comparison. Runs that share a
cohort id *and* host fields were measured on the same hardware. Only the second
supports a ranked table.

### 13.5 Preview data is best effort

The dashboard preview job assembles whatever families succeeded. It downloads
every `runner-*` artifact, adds each run that carries a readable `run_id` to the
index, warns about the ones it skipped, and fails only when nothing at all could
be assembled. One flaky family degrades the preview instead of voiding it.
