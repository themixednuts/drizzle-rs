# Act module — run GitHub Actions locally via nektos/act
#
# Usage:
#   just act::criterion           — run all Criterion micro-benchmarks
#   just act::criterion-sqlite    — Criterion SQLite
#   just act::criterion-pg        — Criterion PostgreSQL
#   just act::runners             — run all runner throughput benchmarks
#   just act::runner-sqlite       — runner SQLite
#   just act::runner-pg           — runner PostgreSQL (Rust)
#   just act::runner-pg-ts        — runner PostgreSQL (TS)
#   just act::runner-spacetimedb  — runner SpacetimeDB
#   just act::runner-turso        — runner Turso
#   just act::ci                  — run the CI workflow
#   just act::clean               — remove reused containers and artifacts

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

# Run runner SQLite throughput benchmarks
runner-sqlite *ARGS:
    act -W .github/workflows/runners.yml -j sqlite {{ARGS}}

# Run runner PostgreSQL (Rust) throughput benchmarks
runner-pg *ARGS:
    act -W .github/workflows/runners.yml -j postgres {{ARGS}}

# Run runner PostgreSQL (TS) throughput benchmarks
runner-pg-ts *ARGS:
    act -W .github/workflows/runners.yml -j postgres-ts {{ARGS}}

# Run runner SpacetimeDB throughput benchmarks
runner-spacetimedb *ARGS:
    act -W .github/workflows/runners.yml -j spacetimedb {{ARGS}}

# Run runner Turso throughput benchmarks
runner-turso *ARGS:
    act -W .github/workflows/runners.yml -j turso {{ARGS}}

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
