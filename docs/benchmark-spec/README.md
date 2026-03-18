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
