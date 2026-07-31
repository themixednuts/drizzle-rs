# Act module — run GitHub Actions locally via nektos/act
#
# Usage:
#   just act::criterion           — run all Criterion micro-benchmarks
#   just act::criterion-sqlite    — Criterion SQLite
#   just act::criterion-pg        — Criterion PostgreSQL
#   just act::runners             — run all runner throughput benchmarks
#   just act::runner-plan         — resolve class/workload only (fast smoke test)
#   just act::runner-sqlite       — runner SQLite
#   just act::runner-pg           — runner PostgreSQL (Rust)
#   just act::runner-pg-orms      — runner PostgreSQL (Rust ORMs)
#   just act::runner-pg-ts        — runner PostgreSQL (TS)
#   just act::runner-pg-all       — runner PostgreSQL cross-family (publish topology)
#   just act::runner-spacetimedb  — runner SpacetimeDB
#   just act::runner-turso        — runner Turso
#   just act::runner-dashboard    — assemble the dashboard preview store
#   just act::runners-list        — list runner jobs and their stages
#   just act::runners-dry         — dry-run a runner job, e.g. `just act::runners-dry turso`
#   just act::ci                  — run the CI workflow
#   just act::clean               — remove reused containers and artifacts
#
# Notes:
#   * Every runner job depends on the `plan` job, so act runs `plan` first even
#     when you select a single family with `-j`.
#   * The family jobs delegate to the local composite action
#     `.github/actions/bench-runner-run`; act resolves `./`-prefixed actions from
#     the bound working directory, so no extra flags are needed.
#   * `act -n` (dry run) panics on any job with a `services:` block — an act bug
#     in its dry-run service health check, not a workflow problem. Dry-run the
#     service-free jobs (plan, sqlite, turso, spacetimedb) and run the
#     PostgreSQL ones for real.

set windows-shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

# --- Criterion micro-benchmarks ---

# Run all Criterion benchmarks
criterion *ARGS:
    act -W .github/workflows/criterion.yml {{ARGS}}

# Run Criterion SQLite benchmarks
criterion-sqlite *ARGS:
    act -W .github/workflows/criterion.yml -j sqlite {{ARGS}}

# Run Criterion PostgreSQL benchmarks
criterion-pg *ARGS:
    act -W .github/workflows/criterion.yml -j postgres {{ARGS}}

# --- Runner throughput benchmarks ---

# Run all runner benchmarks
runners *ARGS:
    act -W .github/workflows/runners.yml {{ARGS}}

# Resolve the run class/workload only — the cheapest end-to-end smoke test
runner-plan *ARGS:
    act -W .github/workflows/runners.yml -j plan {{ARGS}}

# Run runner SQLite throughput benchmarks
runner-sqlite *ARGS:
    act -W .github/workflows/runners.yml -j sqlite {{ARGS}}

# Run runner PostgreSQL (Rust) throughput benchmarks
runner-pg *ARGS:
    act -W .github/workflows/runners.yml -j postgres {{ARGS}}

# Run runner PostgreSQL (Rust ORMs) throughput benchmarks
runner-pg-orms *ARGS:
    act -W .github/workflows/runners.yml -j postgres-rust-orms {{ARGS}}

# Run runner PostgreSQL (TS) throughput benchmarks
runner-pg-ts *ARGS:
    act -W .github/workflows/runners.yml -j postgres-ts {{ARGS}}

# Only selected on publish-class schedule/dispatch runs, so the event payload
# forces `ref=refs/heads/main` and `publish_to_r2=true`.
# Run the same-VM PostgreSQL cross-family job (publish topology)
runner-pg-all *ARGS:
    act workflow_dispatch -W .github/workflows/runners.yml -j postgres-all -e .github/act-events/publish-dispatch.json {{ARGS}}

# Run runner SpacetimeDB throughput benchmarks
runner-spacetimedb *ARGS:
    act -W .github/workflows/runners.yml -j spacetimedb {{ARGS}}

# Run runner Turso throughput benchmarks
runner-turso *ARGS:
    act -W .github/workflows/runners.yml -j turso {{ARGS}}

# Assemble the dashboard preview object store from uploaded artifacts
runner-dashboard *ARGS:
    act -W .github/workflows/runners.yml -j dashboard-preview {{ARGS}}

# List runner jobs, their names, and dependency stages
runners-list *ARGS:
    act -W .github/workflows/runners.yml -l {{ARGS}}

# Dry-run one runner job (service-free jobs only; see the note at the top)
runners-dry JOB *ARGS:
    act -n -W .github/workflows/runners.yml -j {{JOB}} {{ARGS}}

# --- Utilities ---

# Run the CI workflow
ci *ARGS:
    act -W .github/workflows/ci.yml {{ARGS}}

# Remove reused containers and act artifacts for a fresh start
clean:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Removing act containers..."
    docker ps -a --filter "label=com.github.nektos/act" -q | xargs -r docker rm -f
    echo "Removing act artifacts..."
    rm -rf .act-artifacts
    echo "Clean!"
