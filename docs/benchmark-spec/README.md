# Benchmark Contracts

This directory contains the public and persisted benchmark contracts.

## Structure

- `jsonschema/`
  - `workload.v1.schema.json`
  - `target.v1.schema.json`
  - `run-manifest.v1.schema.json`
  - `summary.v1.schema.json`
  - `timeseries.v1.schema.json`
- `openapi/`
  - `bench-api.v1.yaml`
- `runner.v1.md`

## Contract Policy

1. JSON Schema is source of truth for persisted artifacts.
2. OpenAPI is source of truth for external HTTP API.
3. Breaking change requires new major contract version (`v1` -> `v2`).
4. Additive/non-breaking fields can ship as minor updates within same version.
5. Field rename/removal/type change is breaking.
6. Metric semantic change under same key is breaking.

## Validation

Artifact producers should validate against these schemas before publish.

Minimum required validations:

1. `workload` spec file used by run.
2. per-target `summary` output.
3. per-target `timeseries` output.
4. top-level `run-manifest`.

## Measurement Semantics

These are properties of the artifacts, not of the runner implementation, so
they belong with the contract:

1. **Phases.** Every timeseries point carries `phase`. `warmup` covers the first
   `workload.warmup_s` seconds (default 10). `ramp` covers stages whose VU count
   is still interpolating toward a target. `hold` is steady state. **Only `hold`
   buckets feed `summary.primary`.** A run shorter than its own warmup has no
   hold buckets, and the aggregate falls back to counting everything.
2. **Weighting.** `rps` is total hold-phase requests over total hold-phase wall
   seconds; `err` is total errors over total requests. Neither is a mean of
   per-bucket ratios, which would let a 10-request bucket outvote a
   10 000-request one.
3. **Percentiles.** Trial percentiles are computed from the merged raw samples of
   that trial's hold buckets. A median of per-second p95s is not the trial p95 —
   it hides the tail. `p50` and `p90` are measured; earlier runs interpolated
   `p90` from `avg` and `p95`, which was a fabricated number.
4. **Cross-trial aggregation is the median**, including for the fields named
   `*.avg`. Those key names are retained for artifact compatibility;
   `trials.aggregate` and `spread.aggregate` state the real operation.
5. **Errors are counted once.** A non-2xx response is one error; the connection
   is kept and the request is not retried. Only transport failures invalidate a
   connection, and the failed request is still not re-sent.
6. **CPU is host-wide.** `runner.metrics.cpu_scope` is `host`, and
   `runner.topology` records that the load generator (and usually the database)
   shares the machine with the target. `headroom.cpu_peak` is the peak single
   core; `headroom.cpu_mean_peak` is the mean across cores and is what the
   publish gate compares. On Linux CI jobs the load generator and the target are
   pinned to disjoint halves of the runner's cores
   (`topology.cpu_pinning`); that is a partition of a shared VM, not isolation,
   and unpinned runs (macOS, Windows, local) are not comparable to pinned ones.
7. **Pacing caps offered load.** `pacing.mode=drizzle-benchmark` sleeps
   `(iteration % 6) * 75ms` per VU, so throughput is bounded by roughly
   `VUs / (mean think time + mean service time)`. A paced run measures latency
   under bounded arrival, not peak throughput.
8. **Timeseries are concatenated across trials.** `points` is every trial's
   buckets end to end; segment on `point.trial` rather than assuming one
   continuous timeline.
9. **A shared `cohort_id` is not a shared machine.** A cohort groups the runs
   that belong to one logical comparison; when the families ran on separate CI
   VMs the host fields in `manifest.runner` differ and the numbers are only
   comparable within a family. Publish-class schedule and dispatch runs put the
   three PostgreSQL families on one VM specifically so cross-family ranking is
   defensible — see `runner.v1.md` §13.4.
