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
     - `BENCH_STEPS_OUT`
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
2. Harness fairness is enforced per database family — see §5a.

## 5a. Two-axis fairness

"Fair" means two different things and blurring them produces a table that looks
comparable and is not.

**Within a database family — enforced identical.** Every target declares
`fair.family`, `fair.workers`, `fair.pool` and `fair.tuning`. Two targets in the
same family that disagree on workers, pool or tuning **fail the run** with exit
`3`, before any load is generated:

```text
unfair sqlite comparison: target bun-sqlite declares fair.pool=1 but target
drizzle-rs-sqlite declares fair.pool=8. Targets in the same database family must
run an identical harness, otherwise the ranking compares configurations instead
of libraries. Change one of them, or move it to its own family.
```

This replaced a `warn` event plus a `manifest.fairness` entry. A warning buried
in an artifact is silent in every way that matters: it still let an unequal
library comparison ship, and a silently unequal comparison is worse than none.

**A family can span runs, so the specs are also checked as a set.** `postgres`
covers both `targets.postgres.v1.json` and `targets.postgres-rust-orms.v1.json`,
and outside the publish topology (§13.4) those execute as separate CI jobs
producing separate artifacts. A per-run check would pass on each shard
independently and the drift would surface only when a consumer merged them —
after publish, in someone else's UI. A unit test therefore runs the same
enforcement over the union of every checked-in `targets.*.json`, so a pool
changed in one file fails CI at source.

**Across families — declared and displayed.** Nothing is constrained. PostgreSQL
over TCP with a pool of 8 and an embedded SQLite with a pool of 1 *should* differ;
that difference is the stack comparison. `manifest.harness` records the verified
configuration per family so a reader cannot mistake a stack difference for a
library one.

### `fair.family` is a comparison group, not an engine

A family is **the set of targets claiming to be directly comparable**. It usually
maps one-to-one onto the database engine, but it splits when the harness cannot
honestly be equalised.

`sqlite-ts` is the worked example. `bun-sqlite` and `drizzle-orm-sqlite` run
`bun:sqlite` — a synchronous API on a single-threaded runtime — so a pool of 8
there is theatre. Raising their pool to match `targets.sqlite.v1.json` would
cripple them in the name of fairness, which is the opposite of what fairness is
for. drizzle-rs on rusqlite versus drizzle-orm on Bun differs in language,
runtime and concurrency model: that is a **stack** comparison, and stack
comparisons are the across-family axis. Inside `sqlite-ts`, `drizzle-orm-sqlite`
versus `bun-sqlite` is a real library comparison — same runtime, same pool of 1,
same pragmas — which is exactly what a family is for.

Two consequences:

1. **Delta scoping follows `fair.family`.** A target's within-family delta is
   against the drizzle target *in its own group*, so `bun-sqlite` reads "vs
   drizzle-orm on SQLite/Bun", not "vs drizzle-rs on SQLite/Rust".
2. **Presentation does not.** Both groups still appear in one global table with
   `SQLite` in the database column. Only enforcement and delta scoping follow
   `fair.family`; splitting a group does not hide a target.

Family is **declared, not inferred**. `db.profile` separates configurations
*inside* a group (prepared vs unprepared) and `fair.db` names the SQL dialect
several engines share, so neither identifies the bracket a target competes in.
It is also not taken from the spec file a target arrived in — publish-class runs
already execute three PostgreSQL spec files back to back inside one job (§13.4).
The vocabulary is a closed enum in `target.v1.schema.json`
(`sqlite`, `sqlite-ts`, `libsql`, `turso`, `postgres`, `spacetimedb`) and must be
extended in lockstep with the dashboard's family vocabulary.

Targets declaring `data_access: "in-process-cache"` are excluded from the
equality check — a replicated in-process cache has no connection pool to
equalise — and are listed in `harness[].exempt` rather than dropped. A family
whose members are all exempt reports `within_family_identical: false` with no
workers/pool/tuning, meaning "nothing to enforce", never "drift was tolerated".

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
8. each emitted point should carry `trial`, `stage`, `phase`, `vus`, and
   `requests`. Points without a `phase` are treated as steady state for
   backwards compatibility.
9. `load.cmd` may emit one aggregate per hold plateau to `BENCH_STEPS_OUT`: the
   same `Point` shape, one entry per step, ascending in time, each tagged with
   its `vus`. This is **required** for a workload declaring `saturation` — the
   per-step percentiles it carries exist only where the raw samples do, and the
   runner refuses to approximate them from the bucket series.
10. a series is persisted under `targets/<target_id>/raw/trial/<n>.series.json`,
    the aggregate point under `targets/<target_id>/raw/trial/<n>.point.json`,
    and the step list under `targets/<target_id>/raw/trial/<n>.steps.json`.

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
6. `saturation` is emitted only when the workload declares it — see §6c.
7. `latency` is emitted only when the workload declares the sustained-latency
   measurement — see §6d. `primary.latency` remains the whole-ramp aggregate;
   on a ramp that pushes a target past its ceiling it is queueing-dominated by
   construction, and the sustained reading is the figure that measures the
   target.
8. Stages marked `probe: true` are measured and charted but excluded from
   `summary.primary` — see §6d.

**Breaking artifact change.** `summary.saturation` previously always carried
`{knee_rps, knee_p95}`. Those keys are **removed**, not deprecated in place, and
runs recorded before this change no longer validate against `summary.v1`. The
heuristic ran on every workload including paced ones, where throughput is capped
by the load generator's own sleep timer — so its "knee" described the sleep timer
rather than the target — and it fell back to the highest-throughput bucket when
no knee appeared, reporting "no knee found" as a knee. The `did_not_saturate`
outcome is the honest expression of that situation. Keeping the old keys beside
the new block would leave two things called "saturation" in one artifact, so the
removal is deliberate: consumers discriminate on `saturation.outcome`, and an
archived run without it reads as "not measured".

## 6c. Saturation: peak throughput under an SLO

### Two suites, two headlines

| | paced suite | saturation suite |
| --- | --- | --- |
| `pacing.mode` | `drizzle-benchmark` | `none` |
| what it answers | latency under a fixed offered load | how much load the stack can carry |
| headline | **throughput at fixed load** | **peak throughput** (always quoted "at p99 < N ms") |
| comparable to | drizzle-benchmarks' published TS numbers | nothing outside this suite |

They are never averaged together. Each keeps its own number because they measure
different things.

**Why the paced suite cannot produce a capacity number.** Under
`pacing.mode=drizzle-benchmark` every virtual user sleeps `(iteration % 6) * 75ms`
after each request — a mean of 187.5 ms. Offered load is therefore capped near

```text
VUs / (mean think time + mean service time)
```

With think time two orders of magnitude larger than service time, that ceiling is
essentially `VUs / 0.1875 s` for *every* target. A tenfold difference in service
time moves the result by single-digit percent: every healthy target converges on
the same throughput because the number describes the sleep timer, not the
database. Capacity needs an unpaced measurement, which is why it lives in its own
suite and why the runner refuses `saturation` on a paced workload.

### Method

Unpaced closed-loop, stepped concurrency ramp. Each step ramps its VU count in
(`phase=ramp`, charted, not measured) and then holds it (`phase=hold`, measured).
With no think time, N virtual users are N requests in flight, so concurrency is
the x-axis and `rps ≈ N / service_time` until the stack runs out of capacity.

The steps are not listed separately in the spec: they *are* the hold stages of
`workload.stages`, so there is one definition of the ramp and no way for a
declared ladder to drift from the one that runs.

Each step's percentiles come from the merged raw samples of that plateau — the
load command emits them to `$BENCH_STEPS_OUT`, where the samples still exist.
A step aggregate is never derived from the bucket series: that would average
per-second percentiles together, which understates the tail and misplaces the
knee. A load command that does not write `$BENCH_STEPS_OUT` fails a saturation
run rather than getting an approximation.

Across trials each step is the **median** of the per-trial step values, matching
`summary.primary`.

### The headline and the three outcomes

A step **qualifies** when its `slo.metric` percentile is at or under `slo.ms`
*and* its error rate is within `limits.err`. The peak is the qualifying step with
the **highest throughput**; ties break toward the lower concurrency, since the
same throughput for fewer in-flight requests is strictly better. Exactly one of
three outcomes is recorded, and all three are first class:

Saturation is a property of **throughput**: a closed-loop target is at its
ceiling once more in-flight requests stop buying throughput. The objective is a
policy filter on which operating points are acceptable, not the definition of
the limit. The ceiling therefore counts as *found* when the curve **turned
over** — the maximum is interior to the ladder (bracketed by a rise into it and
a fall out of it) and measurably above the last qualifying step (a 2% margin,
so run-to-run wander cannot manufacture a peak out of an ordinary flat curve) —
or when the ramp's last step failed to qualify at all.

| `outcome` | when | what the artifact carries | how to say it |
| --- | --- | --- | --- |
| `saturated` | the curve turned over, or the last step failed to qualify | `peak` | "peak throughput N req/s at p99 < 25 ms" |
| `did_not_saturate` | the last step still qualified and the curve never turned over | `lower_bound_rps` (the best qualifying throughput), no `peak` | "at least N req/s — the curve never turned over" |
| `slo_never_met` | no step qualified | neither | "never met the p99 target" |

`did_not_saturate` is the **designed outcome for a flat curve**, and fast
targets produce flat curves normally: they reach their maximum at 4-16
in-flight requests and hold it, inside the objective, to the top of the ladder.
The value is a lower bound, never presented as a peak — and it is ranked as a
first-class figure at its own value, which places it at its minimum possible
position (see the plateau note below; extending the ladder cannot sharpen a
plateau). `slo_never_met` covers both "even the smallest step was too slow" and
"every step was disqualified by errors"; the per-step `disqualified` reason
distinguishes them, and there is no peak to report either way.

A step over `limits.err` is **disqualified**: it can never be the peak, it stays
in the curve, and it carries the reason string
(`"error rate 3.20% exceeds limit 1.00%"`). It is never silently skipped.

**Peak throughput means the most throughput, not the most concurrency.** A
closed-loop curve is often non-monotone: throughput dips once the pool saturates
and then flattens, so the *widest* step that held the SLO is frequently slower
than an earlier one that also held it. A measured drizzle-rs SQLite ramp does
31 457 rps at 16 VUs and 28 760 at 256, both inside a 25 ms p99 — reporting the
latter as "peak throughput at p99 < 25 ms" would understate the target by 9% and
point at a worse operating point on *both* axes. `peak.concurrency` is therefore
where the maximum occurred, not the last step to survive the SLO, and it can sit
mid-curve. Whether the ramp found the ceiling is a separate question, answered
by `outcome` through the turnover test above: an interior maximum that
measurably beats the last qualifying step is a found ceiling however early it
landed, and a maximum sitting within the 2% margin of the last step is a
plateau, which stays a lower bound.

### The curve

Every step records `concurrency`, `rps`, `latency` (p50/p90/p95/p99), `err`,
`cpu`, `slo_met` and `disqualified`. rps-vs-concurrency and
latency-vs-concurrency are the actual story; the headline is a single point read
off them, and publishing both is what makes it checkable.

### Spec rules the runner enforces

A workload carrying a `saturation` block is rejected unless:

1. `pacing.mode = none` — see the ceiling above.
2. `load.executor = ramping-vus` — a step measured through its own thundering
   herd is not a steady state.
3. `warmup_s` lands on a cumulative stage boundary — otherwise one step is
   measured over part of its plateau and is silently shorter than its neighbours.
4. at least **3** measured steps — two points cannot distinguish a knee from
   noise.
5. step concurrency strictly increases — the curve is read left to right.

### Why the shipped ramps look the way they do

`workload.saturation.v1.json` holds 20 s at each of 1, 2, 4, 8, 16, 32, 64 and
128 VUs, with a 5 s ramp into each and a 20 s warmup at the starting
concurrency. `workload.saturation-preview.v1.json` covers the same span with 4x
spacing (1, 4, 16, 64) and 8 s holds, for PR-sized runs.

An earlier ladder ran 4..1024. Measuring the tuned engines showed it was
sampling the wrong region: every target reaches its maximum throughput at 4-16
in-flight requests and then holds it while only latency climbs, so the steps at
256 and above bought no throughput information — under the breach-keyed outcome
rule of that era they existed *only* to make some step eventually breach the
objective. When the outcome moved to the turnover test, the ladder was re-cut
to where the information actually is. Do not "restore" the long ladder: rungs
above the plateau cannot produce an interior maximum, so under the current rule
they cannot convert a lower bound into a peak — they can only force an SLO
breach, which re-reports the already-known best step under a peak label while
adding ~70 minutes to the cross-family sequence and re-creating the design the
turnover change removed.

- **Geometric spacing, not linear.** The knee's location is not known in advance
  and moves by more than an order of magnitude between an in-process SQLite point
  lookup and a PostgreSQL round trip. Doubling brackets the knee within a factor
  of two anywhere in the span; linear spacing at the same cost would cover a
  single octave and miss most of them. Because rps flattens at the knee, coarse
  concurrency spacing costs little accuracy in the reported peak rps.
- **Starts at 1.** The turnover test requires the maximum to be *bracketed* — a
  rise into it and a fall out of it — so the ladder must start below every
  target's knee to see the rise. A ladder starting at 4 measured a target whose
  curve read 9.1k at 1 VU and 27.3k at 8 and never saw it climbing; its interior
  maximum only became demonstrable once the 1- and 2-VU rungs existed. These two
  rungs are load-bearing; removing them turns early maxima into unprovable
  lower bounds.
- **Ends at 128.** Every measured knee sits at concurrency 4-64 (run
  31773786939: all fifteen peaks landed at 8-64), so 128 gives the top of the
  ladder one rung of falling-or-flat evidence beyond the highest knee. Targets
  whose curve is still flat at 128 report `did_not_saturate` by design; that is
  a ranked lower bound, not a defect, and no finite ladder end changes it — a
  plateau has no interior maximum at any length.
- **20 s holds.** At 500 rps — a slow stack at low concurrency — that is 10 000
  samples per step, so the p99 tail has ~100 observations. The preview's 8 s holds
  are deliberately thinner; a preview answers "does this pipeline work and is the
  shape sane", not "publish this number".
- **`p99 < 25 ms`.** An ordinary web service level: the headline is a capacity
  claim only because a latency bound is attached to it. Under the turnover rule
  the SLO filters which operating points are acceptable rather than defining
  the ceiling, so nothing requires the ramp to run until the SLO breaks.
- **Single endpoint (`/customer-by-id`).** A point lookup is where library
  overhead is the largest share of service time, so the number is about the
  library rather than the query planner. A mixed p99 SLO would in practice be a
  threshold on whichever route is heaviest.

The whole 8-step ramp is 215 s per trial per target, under the 400 s of the
paced `workload.throughput.v1.json`. The preview is 48 s.

## 6d. Sustained latency: the paced ramp's honest latency figure

### Why the whole-ramp aggregate cannot be the latency headline

`summary.primary.latency` merges the raw samples of every counted hold
plateau. On a ramp that pushes targets past their throughput ceiling — the
paced 3000-VU ramp does this to every database-bound target on a 4-core
colocated runner — the samples above the ceiling are queueing delay, not
service time: past saturation a closed loop obeys `latency ≈ VUs /
throughput`, so each further stage adds a fixed increment of queue. A recorded
publish run made the failure mode concrete: every PostgreSQL target sat within
a few percent of 1.3k req/s (the shared two cores, not the libraries) while
the whole-ramp "p95" spread over 2.2–3.3 s and climbed *linearly with the
stage schedule* — 210 ms at 400 VUs, 650 ms at 1000, 2.2 s at 3000, at flat
throughput and 100% CPU throughout. Sorting targets by that number is sorting
by inverted throughput plus ramp overshoot.

`primary.latency` keeps its whole-ramp meaning anyway. The counted stages of
the paced ramp are a faithful transcription of
`drizzle-team/drizzle-benchmarks` (`bench.js`: same stage list, same
`sleep(0.075 * (i % 6))` pacing, and upstream's `prepare.ts` likewise
aggregates k6 `http_req_duration` across the run), and that comparability is
worth keeping under its established name. The service-latency figure gets a
new field instead of silently changing an old one.

### Probe stages: rungs below the historical floor

A stage may declare `"probe": true`. Probe stages are measured — they appear
in the timeseries (tagged `probe`) and as steps of the curve — but their
buckets are excluded from `summary.primary`, so the whole-ramp headline keeps
aggregating exactly the upstream stage list. `workload.throughput.v1.json`
prepends probe rungs at 25, 50 and 100 VUs ahead of the untouched 200→3000
ladder.

The rungs are derived from the recorded cohorts, not guessed. The slowest
measured target tops out near 770 req/s (spacetime-pgwire), and under the
187.5 ms mean pacing the rungs offer at most `N / 0.1875` — ≤133, ≤267 and
≤533 req/s respectively, i.e. at worst ~17%, ~35% and ~70% of that ceiling.
The 25→50 pair is the floor and its corroboration: at ≤35% utilization the
latency growth needed to fail the tolerance (~23 ms, see below) exceeds any
plausible queueing at that load by an order of magnitude, for any service
distribution, so every target — including ones slower than any yet recorded —
demonstrates scaling there. 100 VUs probes the region between the floor and
the old 200-VU start so slower targets read at the highest load they actually
held rather than at an overly conservative floor. Two consequences are
disclosed rather than hidden: the ladder now spends 60 s of light load before
the first counted stage (targets arrive at the 200-VU rung warmer than under
the bare upstream schedule), and timeseries `stage` indices shift by six.

### The measurement

A workload declares:

```json
"latency": {}
```

which emits `summary.latency`. The block is empty on purpose: the qualifying
rule is not a declarable objective but a property of the curve. The steps are
the hold stages of `workload.stages` (probes included), aggregated exactly
like the saturation curve (§6c: per-step percentiles from the plateau's merged
raw samples via `$BENCH_STEPS_OUT`, medianed across trials,
error-disqualification from `limits.err`).

**A step is *sustained* when it served the throughput the closed loop offered
it.** In a closed loop, an unsaturated target's per-VU throughput is constant
in N — `rps(N) = N / (think + latency)` with latency flat — so the floor
step's measured rate, scaled by concurrency, is exactly what a target that
kept its floor latency would serve at every higher rung. Each step publishes
that `offered_rps`, its `retention` (`rps / offered_rps`), and the verdict;
a step is sustained when retention stays within a tolerance of 1.0 and the
step's error rate is inside `limits.err`.

**The published figure is read at the ladder's second rung — the fixed
reference step — not at the last sustained rung.** The last-sustained reading
sits at the knee, the steepest part of the latency curve, so which rung it
lands on decides the figure. Replaying that rule over four measured
full-ladder curves under ±3% per-rung throughput noise (the observed
cross-trial bound) moved its published p95 by 51–99% on three of the four —
one target's figure swung between 4.7 ms and 73.2 ms on a one-rung flap —
while the fixed reference moved ~9%, which is nothing but the injected
latency noise itself: there is no rung selection left to perturb. The
last-sustained rule had a second, independent defect: it compared different
loads across targets — one row's p95 at 800 VUs against another's at 100 —
which is not an ordering. The reference step reads every target at the same
offered load, so the ranking column compares like with like. (Interpolating
latency at a fixed retention crossing was measured too: 38–51% movement,
because the crossing sits in the steep region; a derived-ceiling utilization
rule was bimodal, 9% or 52–114%, depending on where its boundary landed
relative to a rung — any rule that picks a rung near a data-dependent
boundary inherits the cliff.) Where a target *stopped* scaling remains fully
published: the curve carries one `sustained` flag per rung.

Why not a latency SLO? An SLO is a proxy for "this figure is service time,
not queue time", and it fails both ways: a tight ceiling denies slower targets
any figure at the ladder's floor, and a loose one lets an already-saturated
step "qualify" — laundering queueing as service time, which is worse than
reporting nothing. "Served what it was offered" is the direct test, needs no
threshold on the quantity being reported, and — because the recorded curves
collapse from ≥0.92 retention to ≤0.66 in a single geometric rung — separates
the two regimes with a wide margin.

The tolerance (0.10, recorded in the artifact) is derived, not liked: across
every recorded target and rung, below-knee retention never measured under
0.92 (median-of-trials wiggle ≤3% on the noisiest host) and first-rung-past-
the-knee retention never measured above 0.88. What it admits is the
instrument's resolution limit: a step can hide at most `tol/(1-tol) × (think +
latency)` ≈ 21 ms of added mean delay before the shortfall trips the
threshold — queueing below that scale is indistinguishable from service time
in paced closed-loop throughput data, and the per-rung retention discloses how
close each reading ran to the limit.

| `outcome` | when | `reference` | how to say it |
| --- | --- | --- | --- |
| `measured` | the reference step sustained | the ladder's second rung | "p95 6.5 ms at 50 VUs, serving 254 of 255 offered req/s; sustained through 200 VUs" |
| `floor_above_knee` | the reference step failed | absent | "the ladder needs lower rungs for this target" |
| `floor_disqualified` | the floor exceeded `limits.err` | absent | "erroring at the floor; no honest figure exists" |

The floor's retention is 1.0 by identity — it is its own yardstick — so it
cannot vouch for itself; the reference rung above it is what corroborates
that the floor sat below the knee, which is also why the reference is the
lowest rung with a non-vacuous verdict. `floor_above_knee` is therefore a
finding about the *ladder*, exactly like `did_not_saturate` is about the
ramp: with the shipped 25-VU floor and its 50-VU reference it requires a
target slower than ~250 req/s, three times slower than the slowest ever
recorded, and the fix is a lower rung, never a floor number that may already
be queue time. A local verification run measured the shipped probe rungs
directly on four targets (two of them from the slow cohort): every reference
retention rode at 0.974–0.997 against the 0.90 threshold, and knee positions
landed where the recorded ceilings predicted.

Unlike `saturation`, the block is legal on a **paced** workload — think time
caps offered load but does not distort the latency of the requests that are
sent, and the paced ramp is exactly where the whole-run aggregate misleads.
(Unpaced, the same criterion still reads correctly: retention reduces to
`floor latency / step latency`, a pure latency-growth knee test.) The runner
refuses the block when the ladder has fewer than two hold steps outside the
warmup window or the steps do not strictly climb, and a load command that does
not write `$BENCH_STEPS_OUT` fails the run rather than getting an
approximation (same rule as §6c).

**Which number to read.** "How fast is this library" is
`summary.latency.reference` quoted with its load ("p95 6.5 ms at 50 VUs") —
the same offered load for every target, so it orders. "How far does it keep
scaling" is the curve's per-rung `sustained` flags. "Throughput at the
upstream benchmark's fixed load" is `summary.primary.rps`.
`summary.primary.latency` is the whole-ramp, queue-inclusive aggregate and
should be labelled as such wherever it is shown.

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

### 13.1 One plan, grouped jobs

A `plan` job resolves `class`, `workload` and the paced trial count once from
the event and passes them to every benchmark job. The jobs are family
*groups*, not one job per family and not one job per OS: GitHub hard-cancels
any job at 360 minutes, and the paced ladder alone is ~6.9 measured minutes
per target-trial, so a single job running Linux's 27 targets at even one trial
is ~187 minutes of pure ramp — "everything in one job" cannot exist at this
ladder. The shipped grouping (three paced Linux groups, two paced desktop
groups per OS, one saturation job per OS) is the smallest set that fits a
padded time model the `plan` job re-checks on every run, failing in five
minutes rather than at minute 350.

Paced runs use 3 trials (was 5). Measured from the recorded five-trial
cohorts: median-of-3 moves `rps.avg` by at most 2.6% (median 0.1%) and the
whole-ramp p95 by at most 12.5% (median 1.1%); the sustained-latency reference
median is the loosest — per-trial jitter of 5–39% was measured on slower
targets — and `spread.trials` discloses the count either way. Class
resolution:

| `benchmark_size` | publish-class run | class | workload |
| --- | --- | --- | --- |
| `preview` | no | `small` | `workload.preview.v1.json` |
| `preview` | yes | `publish` | `workload.throughput.v1.json` |
| `full` | no | `full` | `workload.throughput.v1.json` |
| `full` | yes | `publish` | `workload.throughput.v1.json` |
| `single` | no | `full` | `workload.single-throughput.v1.json` |
| `single` | yes | `publish` | `workload.single-throughput.v1.json` |
| `saturation` | no | `small` | `workload.saturation-preview.v1.json` |
| `saturation` | yes | `publish` | `workload.saturation.v1.json` |

The saturation rows are the intended wiring for the capacity suite; the specs and
runner support exist, and `runners.yml` picks them up when the `saturation` size
is added to the dispatch input. They are a second suite, not a replacement: they
produce `summary.saturation` (peak throughput under an SLO, §6c) while the paced
rows produce `summary.primary` (throughput at fixed load). A family needs both to
be described completely, and their headlines are reported separately.

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

### 13.4 Topology and databases per OS

Rankings are only meaningful within one machine. The three PostgreSQL families
always run back to back inside one job per OS, against one database, under the
shared paced cohort id: on Linux a `postgres:18-alpine` service container
pinned to its own core (`--cpuset-cpus`), on Windows and macOS a natively
installed PostgreSQL 18 (service containers are Linux-only there), unpinned
like everything else on those platforms. Version parity is enforced by the
setup action — a platform gets the same major as the Linux image or no
PostgreSQL at all, never a mismatched engine in the same comparison. The
embedded families likewise share one job per OS, and the saturation jobs run
every hostable family on a single VM per OS.

Artifact names, output directories and baseline cache keys are per family in
every topology, so a consumer sees exactly one artifact per family either way
and reads the machine facts from `manifest.runner`.

Runs that share a `cohort_id` are the same logical comparison. Runs that share a
cohort id *and* host fields were measured on the same hardware. Only the second
supports a ranked table. A full CI run emits two cohorts — the paced one and
the `-cross` saturation one — sharing the `gh-<run>-…` prefix; a consumer may
join them per target (latency columns from the paced cohort, capacity from the
cross one) provided no number from one cohort is ever compared against a
number from the other.

### 13.5 Preview data is best effort

The dashboard preview job assembles whatever families succeeded. It downloads
every `runner-*` artifact, adds each run that carries a readable `run_id` to the
index, warns about the ones it skipped, and fails only when nothing at all could
be assembled. One flaky family degrades the preview instead of voiding it.
