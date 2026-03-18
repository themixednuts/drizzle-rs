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

Optional flags:

```text
--class <small|publish>
--trials <n>
--baseline <run_id>
--publish
--seed <u64>
--timeout_s <n>
--json
```

Rules:

1. `--suite` must match `workload.suite`.
2. `--trials` defaults by class:
   - `small`: `1`
   - `publish`: `5`
3. `--seed` overrides workload seed for non-publish local runs only.
4. `--json` enables JSONL events on stdout.
5. `--baseline` enables regression comparison against `out/runs/<run_id>`.

## 3. Input Files

Required input:

1. `workload` file:
   - schema: `docs/benchmark-spec/jsonschema/workload.v1.schema.json`
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
     - `BENCH_TRIAL`
     - `BENCH_SEED`
     - `BENCH_WORKLOAD_FILE`
     - `BENCH_REQUESTS_FILE`
     - `BENCH_POINT_OUT`
     - `BENCH_TIMESERIES_OUT`
6. aggregate results.
7. evaluate gates (headroom/regression).
8. write artifacts.
9. publish (if enabled).

Pre-run fixture stage:

1. runner canonicalizes request entries.
2. if request list is empty, runner generates deterministic requests using seed-driven generators.
3. runner writes `requests.generated.json` under run root and uses it as effective request set.

Load output rules:

1. `load.cmd` may emit a single trial point to `BENCH_POINT_OUT`.
2. `load.cmd` may instead emit a per-trial point series to `BENCH_TIMESERIES_OUT`.
3. if a series is emitted, runner persists it under `targets/<target_id>/raw/trial/` and derives the trial point from that series.
4. at least one of `BENCH_POINT_OUT` or `BENCH_TIMESERIES_OUT` must be written.

Implementation note:

1. a target `load.cmd` is expected to exercise the real target path, not synthetic fixture generation.
2. the current Rust HTTP adapter uses `axum 0.6.20` and serves the benchmark route contract on an ephemeral local port during the trial.
3. HTTP/1.1 is the default benchmark transport.
4. HTTP/2, when supported, should be published as a separate labeled profile rather than replacing the default transport silently.

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
    "headroom": "pass",
    "regression": "pass"
  }
}
```

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

## 10. Baseline Compatibility Hint

`manifest.json` may include an additive `compat` object:

```json
{
  "compat": {
    "workload": "sha256:...",
    "class": "publish",
    "targets": ["drizzle-rs-pg-sync"]
  }
}
```

Rules:

1. this is a selection hint for future baseline resolvers.
2. exact compare compatibility is `suite + workload + class + target_id`.
3. missing `compat` is allowed for older `v1` runs.

## 11. Compatibility

Breaking examples requiring `runner.v2`:

1. changing required CLI args.
2. changing artifact required files.
3. changing exit code semantics.
4. changing `result.json` required fields.

## 12. Regression Gate

When baseline is provided:

1. compare per target on `rps.avg` and `latency.p95`.
2. fail if either:
   - `rps.avg` drops by more than `50` and more than `10%`.
   - `latency.p95` rises by more than `5` and more than `10%`.
3. for publish class, regression failure exits with code `9`.
